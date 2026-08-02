use super::*;

#[test]
fn probe_version_space_changes_only_through_append_only_monotonic_receipts() {
    let (candidate, identification, classes) = frozen_generation();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate.clone(),
        ))
        .expect("candidate event");
    ledger
        .append(K1SchedulerEventPayloadV1::IdentificationFreeze(
            identification.clone(),
        ))
        .expect("identification event");

    let probe_one = root(800);
    let difference_one = root(801);
    let (predictions_root_one, predictions_one) = prediction_contract(
        &probe_one,
        &difference_one,
        vec![
            (classes[0].clone(), root(810)),
            (classes[1].clone(), root(810)),
            (classes[2].clone(), root(811)),
        ],
    );
    let pending = K1ProbeRoundReceiptV1::seal_pending(
        identification.freeze_root_sha256.clone(),
        1,
        classes.clone(),
        probe_one,
        difference_one,
        predictions_root_one,
        predictions_one,
        1_001,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 3,
            probe_cost_units: 90,
        },
    )
    .expect("pending");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(pending.clone()))
        .expect("pending event");
    let outcome = K1ProbeRoundReceiptV1::seal_outcome(
        &pending,
        root(802),
        root(803),
        classes[..2].to_vec(),
        false,
    )
    .expect("outcome");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(outcome.clone()))
        .expect("outcome event");

    let probe_two = root(804);
    let difference_two = root(805);
    let (predictions_root_two, predictions_two) = prediction_contract(
        &probe_two,
        &difference_two,
        vec![
            (classes[0].clone(), root(812)),
            (classes[1].clone(), root(813)),
        ],
    );
    let pending_two = K1ProbeRoundReceiptV1::seal_pending(
        identification.freeze_root_sha256.clone(),
        2,
        outcome.next_semantic_class_roots_sha256.clone(),
        probe_two,
        difference_two,
        predictions_root_two,
        predictions_two,
        1_002,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 2,
            probe_cost_units: 80,
        },
    )
    .expect("second pending");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(pending_two.clone()))
        .expect("second pending event");
    assert_eq!(
        K1ProbeRoundReceiptV1::seal_outcome(&pending_two, root(806), root(807), classes, false,),
        Err("k1_probe_version_space_not_monotonic")
    );
    let final_outcome = K1ProbeRoundReceiptV1::seal_outcome(
        &pending_two,
        root(808),
        root(809),
        vec![root(700)],
        false,
    )
    .expect("final outcome");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(final_outcome.clone()))
        .expect("final outcome event");
    let verdict = K1GenerationTerminalVerdictV1::seal(
        candidate.freeze_root_sha256,
        Some(identification.freeze_root_sha256),
        final_outcome.next_semantic_class_roots_sha256,
        vec![root(808), root(809)],
        K1GenerationVerdictClassV1::Abstain,
        "test_terminal_without_transfer_artifact".to_owned(),
        1_100,
        None,
    )
    .expect("verdict");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            verdict,
        )))
        .expect("terminal event");
    ledger.validate().expect("replay validates");
    assert!(ledger.active_candidate_freeze().is_none());
}

#[test]
fn censored_probe_preserves_version_space_and_budget_cannot_expand() {
    let (candidate, identification, classes) = frozen_generation();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(candidate))
        .expect("candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::IdentificationFreeze(
            identification.clone(),
        ))
        .expect("identification");
    let probe = root(900);
    let difference = root(901);
    let (predictions_root, predictions) = prediction_contract(
        &probe,
        &difference,
        vec![
            (classes[0].clone(), root(910)),
            (classes[1].clone(), root(910)),
            (classes[2].clone(), root(911)),
        ],
    );
    let pending = K1ProbeRoundReceiptV1::seal_pending(
        identification.freeze_root_sha256,
        1,
        classes.clone(),
        probe,
        difference,
        predictions_root,
        predictions,
        1_001,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 3,
            probe_cost_units: 90,
        },
    )
    .expect("pending");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(pending.clone()))
        .expect("pending event");
    let censored =
        K1ProbeRoundReceiptV1::seal_outcome(&pending, root(902), root(903), classes.clone(), true)
            .expect("censored");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(censored))
        .expect("censored event");
    let probe_two = root(904);
    let difference_two = root(905);
    let (predictions_root_two, predictions_two) = prediction_contract(
        &probe_two,
        &difference_two,
        vec![
            (classes[0].clone(), root(912)),
            (classes[1].clone(), root(912)),
            (classes[2].clone(), root(913)),
        ],
    );
    let expanded_budget = K1ProbeRoundReceiptV1::seal_pending(
        pending.identification_freeze_root_sha256,
        2,
        classes,
        probe_two,
        difference_two,
        predictions_root_two,
        predictions_two,
        1_002,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 4,
            probe_cost_units: 100,
        },
    )
    .expect("structurally valid pending");
    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::ProbeRound(expanded_budget)),
        Err("k1_scheduler_probe_budget_or_version_mismatch")
    );
}

#[test]
fn applied_probe_outcome_must_equal_one_precommitted_partition() {
    let (_, identification, classes) = frozen_generation();
    let probe = root(950);
    let difference = root(951);
    let (predictions_root, predictions) = prediction_contract(
        &probe,
        &difference,
        vec![
            (classes[0].clone(), root(960)),
            (classes[1].clone(), root(960)),
            (classes[2].clone(), root(961)),
        ],
    );
    let pending = K1ProbeRoundReceiptV1::seal_pending(
        identification.freeze_root_sha256,
        1,
        classes.clone(),
        probe,
        difference,
        predictions_root,
        predictions,
        1_001,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 3,
            probe_cost_units: 90,
        },
    )
    .expect("pending");
    assert_eq!(
        K1ProbeRoundReceiptV1::seal_outcome(
            &pending,
            root(952),
            root(953),
            vec![classes[0].clone(), classes[2].clone()],
            false,
        ),
        Err("k1_probe_outcome_not_precommitted_partition")
    );
}

#[test]
fn deadline_can_close_a_durably_pending_probe_without_forging_an_outcome() {
    let (candidate, identification, classes) = frozen_generation();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate.clone(),
        ))
        .expect("candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::IdentificationFreeze(
            identification.clone(),
        ))
        .expect("identification");
    let probe = root(970);
    let difference = root(971);
    let (predictions_root, predictions) = prediction_contract(
        &probe,
        &difference,
        vec![
            (classes[0].clone(), root(972)),
            (classes[1].clone(), root(972)),
            (classes[2].clone(), root(973)),
        ],
    );
    let pending = K1ProbeRoundReceiptV1::seal_pending(
        identification.freeze_root_sha256.clone(),
        1,
        classes.clone(),
        probe,
        difference,
        predictions_root,
        predictions,
        1_001,
        K1ProbeBudgetRemainingV1 {
            probe_rounds: 3,
            probe_cost_units: 90,
        },
    )
    .expect("pending");
    ledger
        .append(K1SchedulerEventPayloadV1::ProbeRound(pending.clone()))
        .expect("pending event");
    let verdict = K1GenerationTerminalVerdictV1::seal(
        candidate.freeze_root_sha256,
        Some(identification.freeze_root_sha256),
        classes,
        vec![pending.receipt_root_sha256, root(974)],
        K1GenerationVerdictClassV1::ProbeExhausted,
        "generation_deadline_exhausted".to_owned(),
        1_100,
        None,
    )
    .expect("deadline verdict");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            verdict,
        )))
        .expect("pending probe closes on deadline");
}
