fn direct_artifacts_v1(fixture: &InquiryFixtureV1) -> InquiryArtifactsV1 {
    let selector_request = K2InquirySelectorRequestV1::seal(
        root_v1(&"development-selector-executable"),
        fixture.public_case.clone(),
    )
    .expect("development selector request");
    let precommit = select_model_guided_probe_v1(&selector_request).expect("development selection");
    let baseline_request = K2InquiryBaselineRequestV1::seal(
        root_v1(&"development-baseline-executable"),
        fixture.public_case.clone(),
    )
    .expect("development baseline request");
    let baselines =
        evaluate_inquiry_baselines_v1(&baseline_request).expect("development baselines");
    let verifier_executable_sha256 = root_v1(&"development-verifier-executable");
    let selection_verification = verify_inquiry_selection_v1(
        verifier_executable_sha256.clone(),
        &selector_request,
        &precommit,
    )
    .expect("development selection verification");
    let selected_evaluation = evaluation_v1(&precommit, &precommit.selected_probe_root_sha256);
    let true_prediction = selected_evaluation
        .predictions
        .iter()
        .find(|prediction| prediction.model_root_sha256 == fixture.true_model_root_sha256)
        .expect("true-model prediction");
    let observer_request = K2InquiryObserverRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        precommit.selected_probe_root_sha256.clone(),
        root_v1(&"development-observer-executable"),
    )
    .expect("development observer request");
    let mut observation = K2InquiryObservationReceiptV1 {
        schema: K2_INQUIRY_OBSERVATION_SCHEMA_V1.to_owned(),
        observer_request_root_sha256: observer_request.request_root_sha256,
        observer_executable_sha256: observer_request.observer_executable_sha256,
        selected_probe_root_sha256: precommit.selected_probe_root_sha256.clone(),
        post_manifest: true_prediction.predicted_post_manifest.clone(),
        observable_outcome_root_sha256: String::new(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    observation.reseal().expect("development observation");
    let outcome_request = K2InquiryOutcomeVerificationRequestV1::seal(
        verifier_executable_sha256,
        selector_request.clone(),
        precommit.clone(),
        selection_verification.clone(),
        baseline_request.clone(),
        baselines.clone(),
        observation.clone(),
        fixture.true_model_root_sha256.clone(),
    )
    .expect("development outcome request");
    let outcome_receipt =
        verify_inquiry_outcome_v1(&outcome_request).expect("development outcome verification");
    InquiryArtifactsV1 {
        selector_request,
        precommit,
        baseline_request,
        baselines,
        selection_verification,
        observation,
        outcome_request,
        outcome_receipt,
    }
}

fn assert_core_contract_v1(fixture: &InquiryFixtureV1, artifacts: &InquiryArtifactsV1) {
    assert_eq!(fixture.public_case.models.len(), 4);
    assert_eq!(fixture.public_case.probes.len(), 8);
    assert_eq!(artifacts.precommit.evaluations.len(), 8);
    assert!(
        artifacts
            .precommit
            .evaluations
            .iter()
            .all(|evaluation| evaluation.predictions.len() == 4)
    );
    assert_eq!(
        artifacts.precommit.selected_probe_root_sha256,
        fixture.roles.optimal
    );
    let selected = evaluation_v1(&artifacts.precommit, &fixture.roles.optimal);
    assert_eq!(selected.partition_sizes, vec![1, 1, 1, 1]);
    assert_eq!(
        (selected.minimax_eliminated, selected.pair_separation),
        (3, 12)
    );
    assert_eq!(artifacts.precommit.exact_best_ties, 1);
    assert_eq!(
        artifacts
            .precommit
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.eligibility.eligible)
            .count(),
        4
    );
    let decisions = artifacts
        .baselines
        .decisions
        .iter()
        .map(|decision| (decision.kind, decision.selected_probe_root_sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(decisions[&K2InquiryBaselineKindV1::Passive], None);
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::StableHash],
        Some(fixture.roles.stable.clone())
    );
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::CheapestFirst],
        Some(fixture.roles.cheapest.clone())
    );
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::ExplicitHeuristic],
        Some(fixture.roles.heuristic.clone())
    );
    assert_eq!(
        artifacts.outcome_receipt.surviving_model_roots_sha256.len(),
        1
    );
    assert!(artifacts.outcome_receipt.selector_matches_oracle);
    assert_eq!(artifacts.outcome_receipt.oracle_survivors, 1);
    assert_eq!(artifacts.outcome_receipt.complete_prediction_count, 32);
    artifacts
        .outcome_receipt
        .authority
        .validate()
        .expect("authority denied");
}

fn assert_negative_controls_v1(
    fixture: &InquiryFixtureV1,
    artifacts: &InquiryArtifactsV1,
    environment: &TestEnvironmentV1,
) -> BTreeSet<String> {
    let mut passed = BTreeSet::new();
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.unsafe_high_information,
        K2InquiryEligibilityReasonV1::NonReversible,
    );
    passed.insert("01-unsafe-high-information-veto".to_owned());
    assert_ne!(
        artifacts.precommit.selected_probe_root_sha256,
        fixture.roles.cheapest
    );
    passed.insert("02-cheapest-useless-not-selected".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.ambiguous,
        K2InquiryEligibilityReasonV1::AmbiguousObservation,
    );
    passed.insert("03-ambiguous-observation-veto".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.delayed,
        K2InquiryEligibilityReasonV1::DelayedObservation,
    );
    passed.insert("04-delayed-observation-veto".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.unknown,
        K2InquiryEligibilityReasonV1::UnknownAction,
    );
    passed.insert("05-unknown-action-veto".to_owned());

    assert_choice_invariant_public_inputs_v1(
        fixture,
        &artifacts.selector_request,
        &artifacts.baseline_request,
    );
    let mut selector_value =
        serde_json::to_value(&artifacts.selector_request).expect("selector value");
    selector_value["private_true_model_root_sha256"] =
        serde_json::Value::String(fixture.true_model_root_sha256.clone());
    let selector_bytes = composition_bytes_v1(&selector_value).expect("injected selector bytes");
    assert!(composition_decode_v1::<K2InquirySelectorRequestV1>(&selector_bytes).is_err());
    passed.insert("06-private-true-choice-excluded".to_owned());

    let mut outcome_value =
        serde_json::to_value(&artifacts.selector_request).expect("outcome value");
    outcome_value["post_outcome_root_sha256"] =
        serde_json::Value::String(artifacts.observation.observable_outcome_root_sha256.clone());
    let outcome_bytes = composition_bytes_v1(&outcome_value).expect("injected outcome bytes");
    assert!(composition_decode_v1::<K2InquirySelectorRequestV1>(&outcome_bytes).is_err());
    passed.insert("07-post-outcome-input-rejected".to_owned());

    let mut tampered_prediction = artifacts.precommit.clone();
    let selected_probe_root_sha256 = tampered_prediction.selected_probe_root_sha256.clone();
    let selected = tampered_prediction
        .evaluations
        .iter_mut()
        .find(|evaluation| evaluation.probe_root_sha256 == selected_probe_root_sha256)
        .expect("selected evaluation mutable");
    let old_prediction = selected.predictions[0].clone();
    selected.predictions[0] = K2InquiryPredictionV1::seal(
        old_prediction.model_root_sha256,
        old_prediction.probe_root_sha256,
        false,
        "tampered".to_owned(),
        fixture.public_case.probes[0].initial_manifest.clone(),
        K2InquiryObservationModeV1::ExactImmediate,
    )
    .expect("tampered prediction");
    selected.reseal().expect("reseal tampered evaluation");
    tampered_prediction
        .reseal()
        .expect("reseal tampered precommit");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &tampered_prediction,
        )
        .is_err()
    );
    passed.insert("08-tampered-prediction-rejected".to_owned());

    let mut tampered_selected = artifacts.precommit.clone();
    tampered_selected.selected_probe_root_sha256 = fixture.roles.cheapest.clone();
    tampered_selected.reseal().expect("reseal selected tamper");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &tampered_selected,
        )
        .is_err()
    );
    passed.insert("09-tampered-selected-root-rejected".to_owned());

    let mut tampered_observation = artifacts.observation.clone();
    tampered_observation.post_manifest = fixture.public_case.probes[0].initial_manifest.clone();
    let tampered_observation_request = K2InquiryOutcomeVerificationRequestV1::seal(
        artifacts
            .selection_verification
            .verifier_executable_sha256
            .clone(),
        artifacts.selector_request.clone(),
        artifacts.precommit.clone(),
        artifacts.selection_verification.clone(),
        artifacts.baseline_request.clone(),
        artifacts.baselines.clone(),
        tampered_observation,
        fixture.true_model_root_sha256.clone(),
    )
    .expect("tampered observation request");
    assert!(verify_inquiry_outcome_v1(&tampered_observation_request).is_err());
    passed.insert("10-tampered-observer-manifest-rejected".to_owned());

    let action_permuted = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            action_permutation: 1,
            ..CaseVariantV1::default()
        },
    );
    let action_permuted_artifacts = direct_artifacts_v1(&action_permuted);
    assert_eq!(
        action_permuted_artifacts
            .precommit
            .selected_probe_root_sha256,
        action_permuted.roles.optimal
    );
    assert_selected_partition_v1(
        &action_permuted_artifacts.precommit,
        &action_permuted.roles.optimal,
    );
    passed.insert("11-action-id-permutation-invariant".to_owned());

    let path_bijected = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            path_bijection: 1,
            ..CaseVariantV1::default()
        },
    );
    let path_bijected_artifacts = direct_artifacts_v1(&path_bijected);
    assert_eq!(
        path_bijected_artifacts.precommit.selected_probe_root_sha256,
        path_bijected.roles.optimal
    );
    assert_selected_partition_v1(
        &path_bijected_artifacts.precommit,
        &path_bijected.roles.optimal,
    );
    passed.insert("12-path-bijection-invariant".to_owned());

    let reversed = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            reverse_candidates: true,
            ..CaseVariantV1::default()
        },
    );
    let reversed_artifacts = direct_artifacts_v1(&reversed);
    assert_eq!(
        reversed_artifacts.precommit.selected_probe_root_sha256,
        artifacts.precommit.selected_probe_root_sha256
    );
    passed.insert("13-candidate-order-shuffle-invariant".to_owned());

    let collapsed = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            collapse_optimal_predictions: true,
            ..CaseVariantV1::default()
        },
    );
    let collapsed_artifacts = direct_artifacts_v1(&collapsed);
    let collapsed_selected = evaluation_v1(
        &collapsed_artifacts.precommit,
        &collapsed_artifacts.precommit.selected_probe_root_sha256,
    );
    assert!(collapsed_selected.minimax_eliminated < 3);
    assert!(
        collapsed_artifacts
            .outcome_receipt
            .surviving_model_roots_sha256
            .len()
            > 1
    );
    passed.insert("14-collapsed-predictions-destroy-unique-id".to_owned());

    let rotated = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            rotate_model_effect_bindings: true,
            ..CaseVariantV1::default()
        },
    );
    let rotated_artifacts = direct_artifacts_v1(&rotated);
    let original_outcome = predicted_outcome_for_model_id_v1(
        fixture,
        &artifacts.precommit,
        &fixture.true_model_id_sha256,
    );
    let rotated_outcome = predicted_outcome_for_model_id_v1(
        &rotated,
        &rotated_artifacts.precommit,
        &rotated.true_model_id_sha256,
    );
    assert_ne!(original_outcome, rotated_outcome);
    passed.insert("15-shuffled-model-effect-binding-changes-result".to_owned());

    assert_same_identity_redispatch_rejected_v1(environment);
    passed.insert("16-same-identity-redispatch-rejected".to_owned());
    assert_journal_restart_and_fault_parity_v1(environment);
    passed.insert("17-journal-prefix-restart-parity".to_owned());
    assert_authority_promotion_rejected_v1(fixture, artifacts);
    passed.insert("18-authority-promotion-rejected".to_owned());
    passed
}

fn assert_generated_provenance_control_v1(fixture: &InquiryFixtureV1) {
    let original = fixture
        .public_case
        .probe(&fixture.roles.optimal)
        .expect("optimal probe");
    let foreign = K2InquiryProbeV1::seal(
        original.experiment_id_sha256.clone(),
        original.probe_id_sha256.clone(),
        original.action_id_sha256.clone(),
        original.initial_manifest.clone(),
        original.reversible,
        original.observation_mode,
        original.risk_units,
        original.cost_units,
        original.applicability_hint,
        original.dependency_hint,
        original.cleanup_hint,
        root_v1(&"foreign-generated-provenance"),
    )
    .expect("foreign-provenance probe");
    let mut probes = fixture.public_case.probes.clone();
    probes.retain(|probe| probe.probe_root_sha256 != fixture.roles.optimal);
    let foreign_root = foreign.probe_root_sha256.clone();
    probes.push(foreign);
    let case = K2InquiryPublicCaseV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        fixture.public_case.generator_schema_root_sha256.clone(),
        fixture.public_case.split_commitment_root_sha256.clone(),
        fixture.public_case.models.clone(),
        probes,
    )
    .expect("foreign-provenance case");
    let request = K2InquirySelectorRequestV1::seal(root_v1(&"provenance-selector"), case)
        .expect("provenance selector request");
    let precommit = select_model_guided_probe_v1(&request).expect("provenance selection");
    assert_reason_v1(
        &precommit,
        &foreign_root,
        K2InquiryEligibilityReasonV1::NonGeneratedProvenance,
    );
}

fn assert_choice_invariant_public_inputs_v1(
    fixture: &InquiryFixtureV1,
    selector_request: &K2InquirySelectorRequestV1,
    baseline_request: &K2InquiryBaselineRequestV1,
) {
    let selector_bytes = composition_bytes_v1(selector_request).expect("selector bytes");
    let baseline_bytes = composition_bytes_v1(baseline_request).expect("baseline bytes");
    let selector_variants = fixture
        .public_case
        .models
        .iter()
        .map(|_private_choice| selector_bytes.clone())
        .collect::<BTreeSet<_>>();
    let baseline_variants = fixture
        .public_case
        .models
        .iter()
        .map(|_private_choice| baseline_bytes.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(selector_variants.len(), 1);
    assert_eq!(baseline_variants.len(), 1);
    let selector_value = serde_json::to_value(selector_request).expect("selector value");
    let baseline_value = serde_json::to_value(baseline_request).expect("baseline value");
    assert!(
        selector_value
            .get("private_true_model_root_sha256")
            .is_none()
    );
    assert!(
        baseline_value
            .get("private_true_model_root_sha256")
            .is_none()
    );
    assert!(selector_value.get("observation").is_none());
    assert!(baseline_value.get("observation").is_none());
}

fn assert_authority_promotion_rejected_v1(
    fixture: &InquiryFixtureV1,
    artifacts: &InquiryArtifactsV1,
) {
    let mut selector = artifacts.selector_request.clone();
    selector.authority.natural_k2_authority = true;
    assert!(selector.validate().is_err());
    let mut baseline = artifacts.baseline_request.clone();
    baseline.authority.product_authority = true;
    assert!(baseline.validate().is_err());
    let selected_probe = fixture
        .public_case
        .probe(&artifacts.precommit.selected_probe_root_sha256)
        .expect("selected probe");
    let true_model = fixture
        .public_case
        .model(&fixture.true_model_root_sha256)
        .expect("true model");
    let mut worker = K2InquiryWorkerRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        artifacts.selection_verification.receipt_root_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        selected_probe.action_id_sha256.clone(),
        root_v1(&"authority-worker"),
        selected_probe.initial_manifest.clone(),
        true_model
            .effect(&selected_probe.action_id_sha256)
            .expect("true effect")
            .clone(),
    )
    .expect("authority worker request");
    worker.authority.package_activated = true;
    assert!(worker.validate().is_err());
    let mut observer = K2InquiryObserverRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        root_v1(&"authority-observer"),
    )
    .expect("authority observer request");
    observer.authority.phase_memory_mutated = true;
    assert!(observer.validate().is_err());
    let mut outcome_request = artifacts.outcome_request.clone();
    outcome_request.authority.law_certificate_issued = true;
    assert!(verify_inquiry_outcome_v1(&outcome_request).is_err());
    let mut precommit = artifacts.precommit.clone();
    precommit.authority.k1_registry_mutated = true;
    precommit.reseal().expect("reseal promoted precommit");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &precommit,
        )
        .is_err()
    );
    let mut observation = artifacts.observation.clone();
    observation.authority.deployment_authority = true;
    observation.reseal().expect("reseal promoted observation");
    let request = K2InquiryOutcomeVerificationRequestV1::seal(
        artifacts
            .selection_verification
            .verifier_executable_sha256
            .clone(),
        artifacts.selector_request.clone(),
        artifacts.precommit.clone(),
        artifacts.selection_verification.clone(),
        artifacts.baseline_request.clone(),
        artifacts.baselines.clone(),
        observation,
        fixture.true_model_root_sha256.clone(),
    )
    .expect("promoted observation request");
    assert!(verify_inquiry_outcome_v1(&request).is_err());
    let mut terminal = artifacts.outcome_receipt.clone();
    terminal.authority.product_authority = true;
    assert!(terminal.authority.validate().is_err());
}

fn assert_same_identity_redispatch_rejected_v1(environment: &TestEnvironmentV1) {
    let experiment = root_v1(&(
        "redispatch-control",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut journal = K2InquiryJournalV1::create(&environment.journal_store, experiment.clone())
        .expect("redispatch journal");
    for (index, kind) in journal_kinds_v1().into_iter().take(6).enumerate() {
        journal
            .append(kind, root_v1(&(experiment.as_str(), index)))
            .expect("append redispatch prefix");
    }
    assert!(
        journal
            .append(
                K2InquiryJournalEventKindV1::ProbeDispatched,
                root_v1(&"duplicate-probe-dispatch"),
            )
            .is_err()
    );
    let reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, experiment)
        .expect("reopen redispatch journal");
    assert_eq!(reopened.projection().event_count, 6);
    assert!(reopened.projection().indeterminate_probe_dispatch);
    reopened.cleanup().expect("cleanup redispatch journal");
}

fn assert_journal_restart_and_fault_parity_v1(environment: &TestEnvironmentV1) {
    for prefix in 0..=10 {
        let experiment = root_v1(&(
            "journal-prefix-control",
            prefix,
            TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
        ));
        let mut journal =
            K2InquiryJournalV1::create(&environment.journal_store, experiment.clone())
                .expect("prefix journal");
        for (index, kind) in journal_kinds_v1().into_iter().take(prefix).enumerate() {
            journal
                .append(kind, root_v1(&(experiment.as_str(), index)))
                .expect("append prefix event");
            let reopened =
                K2InquiryJournalV1::open_existing(&environment.journal_store, experiment.clone())
                    .expect("reopen legal prefix");
            assert_eq!(reopened.projection(), journal.projection());
        }
        let reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, experiment)
            .expect("reopen final prefix");
        assert_eq!(reopened.projection(), journal.projection());
        reopened.cleanup().expect("cleanup prefix journal");
    }

    let before_id = root_v1(&(
        "journal-before-rename",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut before = K2InquiryJournalV1::create(&environment.journal_store, before_id.clone())
        .expect("before-rename journal");
    assert!(
        before
            .append_with_fault(
                K2InquiryJournalEventKindV1::ExperimentFrozen,
                root_v1(&"before-rename-payload"),
                K2InquiryJournalFaultV1::BeforeRename,
            )
            .is_err()
    );
    let before_reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, before_id)
        .expect("reopen before-rename journal");
    assert_eq!(before_reopened.projection().event_count, 0);
    before_reopened
        .cleanup()
        .expect("cleanup before-rename journal");

    let after_id = root_v1(&(
        "journal-after-rename",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut after = K2InquiryJournalV1::create(&environment.journal_store, after_id.clone())
        .expect("after-rename journal");
    assert!(
        after
            .append_with_fault(
                K2InquiryJournalEventKindV1::ExperimentFrozen,
                root_v1(&"after-rename-payload"),
                K2InquiryJournalFaultV1::AfterRename,
            )
            .is_err()
    );
    let after_reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, after_id)
        .expect("reopen after-rename journal");
    assert_eq!(after_reopened.projection().event_count, 1);
    after_reopened
        .cleanup()
        .expect("cleanup after-rename journal");
}

fn journal_kinds_v1() -> [K2InquiryJournalEventKindV1; 10] {
    [
        K2InquiryJournalEventKindV1::ExperimentFrozen,
        K2InquiryJournalEventKindV1::BaselinesFrozen,
        K2InquiryJournalEventKindV1::SelectionDispatched,
        K2InquiryJournalEventKindV1::SelectionPrecommitted,
        K2InquiryJournalEventKindV1::SelectionVerified,
        K2InquiryJournalEventKindV1::ProbeDispatched,
        K2InquiryJournalEventKindV1::ProbeObserved,
        K2InquiryJournalEventKindV1::ModelsUpdated,
        K2InquiryJournalEventKindV1::ControlsFrozen,
        K2InquiryJournalEventKindV1::TerminalFrozen,
    ]
}

fn assert_confirm_disjointness_v1(fixtures: &[InquiryFixtureV1]) {
    assert_eq!(fixtures.len(), 8);
    let experiment_roots = fixtures
        .iter()
        .map(|fixture| &fixture.public_case.experiment_id_sha256)
        .collect::<BTreeSet<_>>();
    let case_roots = fixtures
        .iter()
        .map(|fixture| &fixture.public_case.case_root_sha256)
        .collect::<BTreeSet<_>>();
    let model_roots = fixtures
        .iter()
        .flat_map(|fixture| {
            fixture
                .public_case
                .models
                .iter()
                .map(|model| &model.model_root_sha256)
        })
        .collect::<BTreeSet<_>>();
    let probe_roots = fixtures
        .iter()
        .flat_map(|fixture| {
            fixture
                .public_case
                .probes
                .iter()
                .map(|probe| &probe.probe_root_sha256)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(experiment_roots.len(), 8);
    assert_eq!(case_roots.len(), 8);
    assert_eq!(model_roots.len(), 8 * 4);
    assert_eq!(probe_roots.len(), 8 * 8);
    assert!(fixtures.iter().all(|fixture| {
        fixture.public_case.split_commitment_root_sha256 == CONFIRM_COMMITMENT_V1
            && fixture.public_case.generator_schema_root_sha256 == GENERATOR_SCHEMA_ROOT_V1
    }));
}

fn append_and_reopen_v1(
    journal: &mut K2InquiryJournalV1,
    store: &Path,
    experiment_id_sha256: &str,
    kind: K2InquiryJournalEventKindV1,
    payload_root_sha256: String,
) {
    journal
        .append(kind, payload_root_sha256)
        .expect("append inquiry journal event");
    let reopened = K2InquiryJournalV1::open_existing(store, experiment_id_sha256.to_owned())
        .expect("reopen inquiry journal");
    assert_eq!(reopened.projection(), journal.projection());
}

fn evaluation_v1<'a>(
    precommit: &'a K2InquirySelectionPrecommitV1,
    probe_root: &str,
) -> &'a K2InquiryProbeEvaluationV1 {
    precommit
        .evaluations
        .iter()
        .find(|evaluation| evaluation.probe_root_sha256 == probe_root)
        .expect("probe evaluation")
}

fn assert_reason_v1(
    precommit: &K2InquirySelectionPrecommitV1,
    probe_root: &str,
    reason: K2InquiryEligibilityReasonV1,
) {
    let evaluation = evaluation_v1(precommit, probe_root);
    assert!(!evaluation.eligibility.eligible);
    assert_eq!(evaluation.eligibility.reason, reason);
}

fn assert_selected_partition_v1(precommit: &K2InquirySelectionPrecommitV1, optimal_root: &str) {
    let evaluation = evaluation_v1(precommit, optimal_root);
    assert_eq!(evaluation.partition_sizes, vec![1, 1, 1, 1]);
    assert_eq!(
        (evaluation.minimax_eliminated, evaluation.pair_separation),
        (3, 12)
    );
}

fn predicted_outcome_for_model_id_v1(
    fixture: &InquiryFixtureV1,
    precommit: &K2InquirySelectionPrecommitV1,
    model_id_sha256: &str,
) -> String {
    let model_root = &fixture
        .public_case
        .models
        .iter()
        .find(|model| model.model_id_sha256 == model_id_sha256)
        .expect("model by opaque id")
        .model_root_sha256;
    evaluation_v1(precommit, &fixture.roles.optimal)
        .predictions
        .iter()
        .find(|prediction| &prediction.model_root_sha256 == model_root)
        .expect("model prediction")
        .observable_outcome_root_sha256
        .clone()
}

fn copy_v1(source: &str, target: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::CopyFile {
        source_path: source.to_owned(),
        target_path: target.to_owned(),
    }
}

fn remove_v1(path: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::RemoveFile {
        path: path.to_owned(),
    }
}

fn root_v1<T: Serialize>(value: &T) -> String {
    composition_root_v1(&("nando.k2-inquiry-test-root.v1", value)).expect("test root")
}

fn root_set_v1(label: &str, roots: impl IntoIterator<Item = String>) -> String {
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort();
    composition_root_v1(&(label, roots)).expect("root set")
}

fn directory_is_empty_v1(path: &Path) -> bool {
    fs::read_dir(path)
        .expect("read generated directory")
        .next()
        .is_none()
}

