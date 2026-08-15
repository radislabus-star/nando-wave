#[test]
fn development_active_inquiry_contract_is_complete_and_fail_closed() {
    let fixture = build_fixture_v1(0, DEVELOPMENT_COMMITMENT_V1, CaseVariantV1::default());
    let artifacts = direct_artifacts_v1(&fixture);
    assert_core_contract_v1(&fixture, &artifacts);

    let environment = TestEnvironmentV1::new("development-controls");
    let controls = assert_negative_controls_v1(&fixture, &artifacts, &environment);
    assert_eq!(controls.len(), 18);
    assert_generated_provenance_control_v1(&fixture);
    assert!(directory_is_empty_v1(&environment.journal_store));
}

#[test]
#[ignore = "exactly one sealed run; requires Linux bwrap and five isolated inquiry binaries"]
fn sealed_model_guided_active_inquiry_uses_one_safe_probe_per_case() {
    let binaries = ProcessBinariesV1::from_cargo();
    binaries.assert_pairwise_distinct();
    let environment = TestEnvironmentV1::new("sealed-confirm");
    let fixtures = (0..K2_INQUIRY_CONFIRM_CASES_V1)
        .map(|case_index| {
            build_fixture_v1(case_index, CONFIRM_COMMITMENT_V1, CaseVariantV1::default())
        })
        .collect::<Vec<_>>();
    assert_confirm_disjointness_v1(&fixtures);

    let experiment_id_sha256 = root_v1(&(
        "sealed-experiment",
        fixtures
            .iter()
            .map(|fixture| &fixture.public_case.case_root_sha256)
            .collect::<Vec<_>>(),
    ));
    let mut journal =
        K2InquiryJournalV1::create(&environment.journal_store, experiment_id_sha256.clone())
            .expect("create sealed inquiry journal");
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ExperimentFrozen,
        root_set_v1(
            "sealed-public-cases",
            fixtures
                .iter()
                .map(|fixture| fixture.public_case.case_root_sha256.clone()),
        ),
    );

    let baseline_requests = fixtures
        .iter()
        .map(|fixture| {
            K2InquiryBaselineRequestV1::seal(
                binaries.baseline_sha256.clone(),
                fixture.public_case.clone(),
            )
            .expect("seal baseline request")
        })
        .collect::<Vec<_>>();
    let baselines = baseline_requests
        .iter()
        .map(|request| run_isolated_protocol_v1(&binaries.baseline, request))
        .collect::<Vec<K2InquiryBaselinesV1>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::BaselinesFrozen,
        root_set_v1(
            "sealed-baselines",
            baselines
                .iter()
                .map(|value| value.baselines_root_sha256.clone()),
        ),
    );

    let selector_requests = fixtures
        .iter()
        .map(|fixture| {
            K2InquirySelectorRequestV1::seal(
                binaries.selector_sha256.clone(),
                fixture.public_case.clone(),
            )
            .expect("seal selector request")
        })
        .collect::<Vec<_>>();
    assert_choice_invariant_public_inputs_v1(
        &fixtures[0],
        &selector_requests[0],
        &baseline_requests[0],
    );
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionDispatched,
        root_set_v1(
            "sealed-selector-dispatches",
            selector_requests
                .iter()
                .map(|request| request.request_root_sha256.clone()),
        ),
    );
    let precommits = selector_requests
        .iter()
        .map(|request| run_isolated_protocol_v1(&binaries.selector, request))
        .collect::<Vec<K2InquirySelectionPrecommitV1>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionPrecommitted,
        root_set_v1(
            "sealed-selection-precommits",
            precommits
                .iter()
                .map(|value| value.precommit_root_sha256.clone()),
        ),
    );

    let selection_verifications = selector_requests
        .iter()
        .zip(&precommits)
        .map(|(selector_request, precommit)| {
            let command = K2InquiryVerifierCommandV1::VerifySelection {
                verifier_executable_sha256: binaries.verifier_sha256.clone(),
                selector_request: Box::new(selector_request.clone()),
                precommit: Box::new(precommit.clone()),
            };
            match run_isolated_protocol_v1(&binaries.verifier, &command) {
                K2InquiryVerifierReceiptV1::Selection { value } => value,
                K2InquiryVerifierReceiptV1::Outcome { .. } => {
                    panic!("selection verifier returned outcome receipt")
                }
            }
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionVerified,
        root_set_v1(
            "sealed-selection-verifications",
            selection_verifications
                .iter()
                .map(|value| value.receipt_root_sha256.clone()),
        ),
    );

    let worker_requests = fixtures
        .iter()
        .zip(&precommits)
        .zip(&selection_verifications)
        .map(|((fixture, precommit), selection)| {
            let probe = fixture
                .public_case
                .probe(&precommit.selected_probe_root_sha256)
                .expect("selected probe exists");
            let true_model = fixture
                .public_case
                .model(&fixture.true_model_root_sha256)
                .expect("true model exists");
            let resolved_effect = true_model
                .effect(&probe.action_id_sha256)
                .expect("selected action known")
                .clone();
            K2InquiryWorkerRequestV1::seal(
                fixture.public_case.experiment_id_sha256.clone(),
                selection.receipt_root_sha256.clone(),
                probe.probe_root_sha256.clone(),
                probe.action_id_sha256.clone(),
                binaries.worker_sha256.clone(),
                probe.initial_manifest.clone(),
                resolved_effect,
            )
            .expect("seal worker request")
        })
        .collect::<Vec<_>>();
    let observer_requests = fixtures
        .iter()
        .zip(&precommits)
        .map(|(fixture, precommit)| {
            K2InquiryObserverRequestV1::seal(
                fixture.public_case.experiment_id_sha256.clone(),
                precommit.selected_probe_root_sha256.clone(),
                binaries.observer_sha256.clone(),
            )
            .expect("seal observer request")
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ProbeDispatched,
        root_set_v1(
            "sealed-probe-dispatches",
            worker_requests
                .iter()
                .map(|request| request.request_root_sha256.clone()),
        ),
    );

    let sandbox = K2InquirySandboxAdapterV1::new(
        binaries.worker.clone(),
        binaries.worker_sha256.clone(),
        binaries.observer.clone(),
        binaries.observer_sha256.clone(),
        environment.workspace_store.clone(),
    )
    .expect("create inquiry sandbox");
    let executions = worker_requests
        .iter()
        .zip(&observer_requests)
        .zip(&fixtures)
        .map(|((worker_request, observer_request), fixture)| {
            sandbox
                .execute(worker_request, observer_request, &fixture.initial_files)
                .expect("execute one isolated inquiry probe")
        })
        .collect::<Vec<_>>();
    assert!(
        executions.iter().all(|execution| {
            execution.source_integrity_preserved && execution.workspace_removed
        })
    );
    assert!(directory_is_empty_v1(&environment.workspace_store));
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ProbeObserved,
        root_set_v1(
            "sealed-observations",
            executions
                .iter()
                .map(|execution| execution.observation.receipt_root_sha256.clone()),
        ),
    );

    let outcome_requests = (0..fixtures.len())
        .map(|index| {
            K2InquiryOutcomeVerificationRequestV1::seal(
                binaries.verifier_sha256.clone(),
                selector_requests[index].clone(),
                precommits[index].clone(),
                selection_verifications[index].clone(),
                baseline_requests[index].clone(),
                baselines[index].clone(),
                executions[index].observation.clone(),
                fixtures[index].true_model_root_sha256.clone(),
            )
            .expect("seal outcome verification request")
        })
        .collect::<Vec<_>>();
    let outcomes = outcome_requests
        .iter()
        .map(|request| {
            let command = K2InquiryVerifierCommandV1::VerifyOutcome {
                request: Box::new(request.clone()),
            };
            match run_isolated_protocol_v1(&binaries.verifier, &command) {
                K2InquiryVerifierReceiptV1::Outcome { value } => value,
                K2InquiryVerifierReceiptV1::Selection { .. } => {
                    panic!("outcome verifier returned selection receipt")
                }
            }
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ModelsUpdated,
        root_set_v1(
            "sealed-model-updates",
            outcomes
                .iter()
                .map(|outcome| outcome.receipt_root_sha256.clone()),
        ),
    );

    let first_artifacts = InquiryArtifactsV1 {
        selector_request: selector_requests[0].clone(),
        precommit: precommits[0].clone(),
        baseline_request: baseline_requests[0].clone(),
        baselines: baselines[0].clone(),
        selection_verification: selection_verifications[0].clone(),
        observation: executions[0].observation.clone(),
        outcome_request: outcome_requests[0].clone(),
        outcome_receipt: outcomes[0].clone(),
    };
    let controls = assert_negative_controls_v1(&fixtures[0], &first_artifacts, &environment);
    assert_eq!(controls.len(), 18);
    assert_generated_provenance_control_v1(&fixtures[0]);
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ControlsFrozen,
        root_set_v1("sealed-controls", controls.iter().cloned()),
    );

    let totals = InquiryTotalsV1::from_results(&precommits, &outcomes);
    totals.assert_pass();
    let terminal_payload_root_sha256 =
        composition_root_v1(&(K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS_V1, &totals, &controls))
            .expect("terminal payload root");
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::TerminalFrozen,
        terminal_payload_root_sha256,
    );
    let projection = journal.projection();
    assert!(projection.terminal);
    let terminal_event_root_sha256 = projection
        .last_event_root_sha256
        .clone()
        .expect("terminal event root");

    let mut receipt = SealedInquiryReceiptV1 {
        schema: "nando.k2-self-chosen-safe-inquiry-receipt.v1".to_owned(),
        disposition: K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS_V1.to_owned(),
        confirm_commitment_sha256: CONFIRM_COMMITMENT_V1.to_owned(),
        generator_schema_sha256: GENERATOR_SCHEMA_ROOT_V1.to_owned(),
        cases: fixtures.len() as u64,
        totals,
        negative_controls_passed: controls.len() as u64,
        forbidden_probe_executions: 0,
        terminal_event_root_sha256,
        selector_executable_sha256: binaries.selector_sha256.clone(),
        baseline_executable_sha256: binaries.baseline_sha256.clone(),
        verifier_executable_sha256: binaries.verifier_sha256.clone(),
        worker_executable_sha256: binaries.worker_sha256.clone(),
        observer_executable_sha256: binaries.observer_sha256.clone(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 = composition_root_v1(&(
        &receipt.schema,
        &receipt.disposition,
        &receipt.confirm_commitment_sha256,
        &receipt.generator_schema_sha256,
        receipt.cases,
        &receipt.totals,
        receipt.negative_controls_passed,
        receipt.forbidden_probe_executions,
        &receipt.terminal_event_root_sha256,
        &receipt.selector_executable_sha256,
        &receipt.baseline_executable_sha256,
        &receipt.verifier_executable_sha256,
        &receipt.worker_executable_sha256,
        &receipt.observer_executable_sha256,
        &receipt.authority,
    ))
    .expect("receipt root");

    journal.cleanup().expect("cleanup sealed journal");
    assert!(directory_is_empty_v1(&environment.journal_store));
    assert!(directory_is_empty_v1(&environment.workspace_store));
    println!(
        "NANDO_K2_INQUIRY_SEALED_RESULT={}",
        String::from_utf8(composition_bytes_v1(&receipt).expect("receipt bytes"))
            .expect("receipt utf8")
    );
}

