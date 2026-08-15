use std::path::PathBuf;

use nando_operator_learning::{
    K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2, K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2,
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2UncertaintyBatchPrecommitV2,
    K2UncertaintyCaseJournalFaultV2, K2UncertaintyCaseJournalPhaseV2, K2UncertaintyCaseJournalV2,
    K2UncertaintyCasePrecommitEntryV2, K2UncertaintyCasePreverificationV2,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlanV1,
    K2UncertaintyClosurePlannerRequestV1, K2UncertaintyClosureVerificationRequestV1,
    composition_root_v1, composition_sha256_file_v1, decode_self_formed_closure_planner_request_v1,
    plan_self_formed_uncertainty_closure_v1, uncertainty_bytes_v1,
    verify_self_formed_closure_independently_v1,
};

use super::closure::{TwoProbeHarness, build_two_probe_harness};
use super::fixture::{R7Fixture, root_hash};
use super::ledger::V4ControlLedger;

pub fn run(fixture: &R7Fixture, ledger: &mut V4ControlLedger) {
    let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-self-formed-closure-verifier"));
    let verifier_sha256 = composition_sha256_file_v1(&verifier).expect("closure verifier sha");
    let harness = build_two_probe_harness(fixture, &verifier_sha256);

    reject_private_and_post_outcome_inputs(&harness, ledger);
    reject_census_and_ranking_tampering(&harness, ledger);
    reject_changed_first_winner(fixture, &harness, &verifier_sha256, ledger);
    reject_dispatch_observation_and_crash_tampering(fixture, &harness, ledger);
    reject_all_case_omission(fixture, &harness, ledger);
    reject_joint_partition_drift(&harness, ledger);
}

fn reject_private_and_post_outcome_inputs(harness: &TwoProbeHarness, ledger: &mut V4ControlLedger) {
    let canonical =
        uncertainty_bytes_v1(&harness.planner_request).expect("canonical planner bytes");
    assert_eq!(
        decode_self_formed_closure_planner_request_v1(&canonical)
            .expect("canonical planner request"),
        harness.planner_request
    );
    let private = injected_request_bytes(&harness.planner_request, "mapping");
    assert_error(
        decode_self_formed_closure_planner_request_v1(&private),
        "self_formed_closure_private_input_forbidden",
    );
    ledger.pass("J1", "private_completion_input_rejected");

    let outcome = injected_request_bytes(&harness.planner_request, "observed_outcome");
    assert_error(
        decode_self_formed_closure_planner_request_v1(&outcome),
        "self_formed_closure_post_outcome_input_forbidden",
    );
    ledger.pass("J2", "post_outcome_completion_input_rejected");
}

fn reject_census_and_ranking_tampering(harness: &TwoProbeHarness, ledger: &mut V4ControlLedger) {
    let mut omitted = harness.census.clone();
    omitted.candidates.pop().expect("completion candidate");
    assert_error(omitted.reseal(), "self_formed_closure_census_invalid");
    ledger.pass("J3", "omitted_completion_candidate_rejected");

    let mut duplicate = harness.census.clone();
    duplicate.candidates.push(duplicate.candidates[0].clone());
    duplicate.candidates.sort_by(|left, right| {
        left.second_probe_root_sha256
            .cmp(&right.second_probe_root_sha256)
    });
    assert_error(
        duplicate.reseal(),
        "self_formed_closure_candidates_not_canonical",
    );
    ledger.pass("J4", "duplicate_or_foreign_second_probe_rejected");

    let mut wrong_and = harness.census.candidates[0].clone();
    wrong_and.joint_pairwise_outcome_equal = [true, true, false, false, false, false];
    assert_error(
        wrong_and.reseal(),
        "self_formed_completion_equality_nontransitive",
    );
    ledger.pass("J5", "wrong_joint_and_matrix_rejected");

    let mut wrong_count = harness.census.clone();
    wrong_count.candidate_count = wrong_count.candidate_count.saturating_add(1);
    assert_error(wrong_count.reseal(), "self_formed_closure_census_invalid");
    let mut wrong_membership = harness.census.clone();
    wrong_membership.membership_root_sha256 = root_hash("j12-wrong-membership");
    assert_error(
        wrong_membership.reseal(),
        "self_formed_closure_census_invalid",
    );
    ledger.pass("J12", "completion_count_or_membership_rejected");

    let selected = harness
        .census
        .selected_second_probe_root_sha256
        .as_ref()
        .expect("selected second probe");
    let alternative = harness
        .census
        .second_probe_candidate_roots_sha256
        .iter()
        .find(|root| *root != selected)
        .expect("non-global second probe")
        .clone();
    let mut wrong_winner = harness.census.clone();
    wrong_winner.selected_second_probe_root_sha256 = Some(alternative);
    assert_error(wrong_winner.reseal(), "self_formed_closure_census_invalid");
    ledger.pass("J13", "non_global_second_winner_rejected");

    let mut risk = harness.census.candidates[0].clone();
    risk.cumulative_risk_units = 21;
    assert_error(risk.reseal(), "self_formed_completion_candidate_invalid");
    let mut cost = harness.census.candidates[0].clone();
    cost.cumulative_cost_units = 21;
    assert_error(cost.reseal(), "self_formed_completion_candidate_invalid");
    ledger.pass("J14", "cumulative_budget_excess_rejected");
}

fn reject_changed_first_winner(
    fixture: &R7Fixture,
    harness: &TwoProbeHarness,
    verifier_sha256: &str,
    ledger: &mut V4ControlLedger,
) {
    let changed_first = harness
        .planner_request
        .representatives
        .iter()
        .map(|value| &value.probe.probe_root_sha256)
        .find(|root| *root != &harness.planner_request.first_probe_root_sha256)
        .expect("alternative first probe")
        .clone();
    let planner = K2UncertaintyClosurePlannerRequestV1::seal(
        harness.planner_request.case_id_sha256.clone(),
        harness.planner_request.frontier_root_sha256.clone(),
        harness.planner_request.first_tournament_root_sha256.clone(),
        changed_first,
        harness.planner_request.representatives.clone(),
        harness.planner_request.planner_executable_sha256.clone(),
    )
    .expect("changed-first planner request");
    let census = plan_self_formed_uncertainty_closure_v1(&planner).expect("changed-first census");
    let verification_request = K2UncertaintyClosureVerificationRequestV1::seal(
        verifier_sha256.to_owned(),
        planner.clone(),
        census.clone(),
    )
    .expect("changed-first verification request");
    let verification_receipt = verify_self_formed_closure_independently_v1(&verification_request)
        .expect("changed-first independent verification");
    let plan = match census.disposition {
        K2UncertaintyClosureDispositionV1::ClosureUnavailable => None,
        _ => Some(
            K2UncertaintyClosurePlanV1::seal(&planner, &census, &verification_receipt)
                .expect("changed-first plan"),
        ),
    };
    assert_error(
        K2UncertaintyCasePreverificationV2::seal(
            fixture.preverification.clone(),
            verification_request,
            verification_receipt,
            plan,
        ),
        "self_formed_case_preverification_v2_invalid",
    );
    ledger.pass("J6", "changed_predecessor_winner_rejected");
}

fn reject_dispatch_observation_and_crash_tampering(
    fixture: &R7Fixture,
    harness: &TwoProbeHarness,
    ledger: &mut V4ControlLedger,
) {
    let mut swapped = harness.dispatch.clone();
    swapped.items.swap(0, 1);
    assert_error(
        swapped.reseal(),
        "self_formed_plan_dispatch_item_v2_invalid",
    );
    ledger.pass("J7", "swapped_probe_ordinal_rejected");

    let mut shared = harness.dispatch.clone();
    shared.items[1].workspace_identity = shared.items[0].workspace_identity.clone();
    assert_error(
        shared.items[1].reseal(),
        "self_formed_probe_dispatch_item_v2_invalid",
    );
    ledger.pass("J8", "shared_workspace_binding_rejected");

    let journal_root = fixture.root.join("j9-j10-case-journal");
    let mut journal = K2UncertaintyCaseJournalV2::create(&journal_root, harness.dispatch.clone())
        .expect("J9 case journal");
    journal
        .record_plan_dispatch(root_hash("j9-owner"), K2UncertaintyCaseJournalFaultV2::None)
        .expect("J9 plan dispatch");
    let first = journal
        .begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::None)
        .expect("J9 first execution");
    journal
        .record_probe_observation(
            first,
            root_hash("j9-first-observation"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("J9 first observation");
    assert_error(
        journal.freeze_observation_vector(
            root_hash("j9-vector-owner"),
            root_hash("j9-vector-request"),
            root_hash("j9-vector"),
            K2UncertaintyCaseJournalFaultV2::None,
        ),
        "self_formed_case_journal_observation_vector_order_v2_invalid",
    );
    ledger.pass("J9", "missing_second_observation_rejected");

    let second = journal
        .begin_probe_execution(1, K2UncertaintyCaseJournalFaultV2::None)
        .expect("J10 second execution");
    journal
        .record_probe_observation(
            second,
            root_hash("j10-second-observation"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("J10 second observation");
    journal
        .freeze_observation_vector(
            root_hash("j10-vector-owner"),
            root_hash("j10-vector-request"),
            root_hash("j10-vector"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("J10 vector freeze");
    assert_error(
        journal.freeze_cleanup(
            root_hash("j10-cleanup-owner"),
            root_hash("j10-cleanup-request"),
            root_hash("j10-cleanup-receipt"),
            K2UncertaintyCaseJournalFaultV2::None,
        ),
        "self_formed_case_journal_cleanup_order_v2_invalid",
    );
    ledger.pass("J10", "cleanup_before_terminal_rejected");

    let crash_root = fixture.root.join("j15-crash-journal");
    let mut crash = K2UncertaintyCaseJournalV2::create(&crash_root, harness.dispatch.clone())
        .expect("J15 crash journal");
    assert_error(
        crash.begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::None),
        "self_formed_case_journal_probe_redispatch_v2",
    );
    crash
        .record_plan_dispatch(
            root_hash("j15-dispatch-owner"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("J15 plan dispatch");
    assert_error(
        crash.begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::AfterRename),
        "self_formed_case_journal_v2_fault_after_rename",
    );
    drop(crash);
    let mut reopened = K2UncertaintyCaseJournalV2::reopen(&crash_root).expect("J15 reopen");
    assert_eq!(
        reopened.projection().expect("J15 projection").phase,
        K2UncertaintyCaseJournalPhaseV2::IndeterminateExecution { probe_ordinal: 0 }
    );
    assert_error(
        reopened.begin_probe_execution(0, K2UncertaintyCaseJournalFaultV2::None),
        "self_formed_case_journal_probe_redispatch_v2",
    );
    reopened
        .freeze_indeterminate_execution(
            root_hash("j15-terminal-owner"),
            root_hash("j15-terminal-receipt"),
            K2UncertaintyCaseJournalFaultV2::None,
        )
        .expect("J15 terminal freeze");
    ledger.pass("J15", "invalid_crash_prefix_or_redispatch_rejected");
}

fn reject_all_case_omission(
    fixture: &R7Fixture,
    harness: &TwoProbeHarness,
    ledger: &mut V4ControlLedger,
) {
    let mut unavailable = K2UncertaintyCasePrecommitEntryV2::seal(&harness.case_preverification)
        .expect("J11 source entry");
    unavailable.closure_disposition = K2UncertaintyClosureDispositionV1::ClosureUnavailable;
    unavailable.closure_plan_root_sha256 = None;
    unavailable.dispatchable = false;
    unavailable.entry_root_sha256 = composition_root_v1(&(
        K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2,
        &unavailable.case_id_sha256,
        &unavailable.case_preverification_root_sha256,
        &unavailable.selection_preverification_root_sha256,
        &unavailable.closure_planner_request_root_sha256,
        &unavailable.closure_census_root_sha256,
        unavailable.closure_disposition,
        &unavailable.closure_verification_receipt_root_sha256,
        &unavailable.closure_plan_root_sha256,
        unavailable.dispatchable,
    ))
    .expect("J11 unavailable entry root");
    unavailable.validate().expect("J11 unavailable entry");

    let valid = K2UncertaintyCasePrecommitEntryV2::seal(&harness.case_preverification)
        .expect("J11 valid entry");
    let batch = K2UncertaintyBatchPrecommitV2 {
        schema: K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2.to_owned(),
        experiment_id_sha256: fixture.generated.public.experiment_id_sha256.clone(),
        private_expected_denominator_commitment_sha256: fixture
            .generated
            .private
            .expected_denominator_commitment_sha256
            .clone(),
        cases: vec![valid; 15],
        execution_order_case_roots_sha256: vec![unavailable.case_id_sha256; 16],
        closure_census_denominator_root_sha256: root_hash("j11-census-denominator"),
        closure_plan_denominator_root_sha256: root_hash("j11-plan-denominator"),
        dispatch_permitted: true,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        batch_root_sha256: root_hash("j11-batch"),
    };
    assert_error(
        batch.validate(),
        "self_formed_batch_precommit_v2_case_count_invalid",
    );
    ledger.pass("J11", "unavailable_case_omission_rejected");
}

fn reject_joint_partition_drift(harness: &TwoProbeHarness, ledger: &mut V4ControlLedger) {
    let selected = harness
        .census
        .selected_second_probe_root_sha256
        .as_ref()
        .expect("J16 selected second");
    let mut tampered = harness.census.clone();
    let candidate = tampered
        .candidates
        .iter_mut()
        .find(|value| &value.second_probe_root_sha256 != selected)
        .expect("J16 non-selected candidate");
    candidate.joint_pairwise_outcome_equal = [true, false, false, false, false, false];
    candidate.joint_partition_sizes = vec![2, 1, 1];
    candidate.joint_minimax_eliminated = 2;
    candidate.joint_pair_separation = 10;
    candidate.reseal().expect("J16 self-consistent candidate");
    let candidate_roots = tampered
        .candidates
        .iter()
        .map(|value| value.candidate_root_sha256.clone())
        .collect::<Vec<_>>();
    tampered.candidate_denominator_root_sha256 = composition_root_v1(&(
        "nando.k2-self-formed-completion-denominator.v1",
        &candidate_roots,
    ))
    .expect("J16 denominator root");
    tampered.reseal().expect("J16 internally valid census");
    let request = K2UncertaintyClosureVerificationRequestV1::seal(
        harness
            .verification_request
            .verifier_executable_sha256
            .clone(),
        harness.planner_request.clone(),
        tampered,
    )
    .expect("J16 verification request");
    assert_error(
        verify_self_formed_closure_independently_v1(&request),
        "self_formed_closure_verification_census_mismatch",
    );
    ledger.pass("J16", "stored_joint_partition_drift_rejected");
}

fn injected_request_bytes(request: &K2UncertaintyClosurePlannerRequestV1, key: &str) -> Vec<u8> {
    let mut value = serde_json::to_value(request).expect("planner request JSON");
    value
        .as_object_mut()
        .expect("planner request object")
        .insert(key.to_owned(), serde_json::json!({"forbidden": true}));
    serde_json::to_vec(&value).expect("injected planner request bytes")
}

fn assert_error<T>(result: Result<T, K2CompositionErrorV1>, code: &str) {
    let error = result.err().expect("V4 control accepted");
    assert!(error.to_string().contains(code), "wrong error: {error}");
}
