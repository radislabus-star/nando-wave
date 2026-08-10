use super::*;

#[test]
fn candidate_cannot_be_replaced_before_terminal_verdict() {
    let (candidate, _, _) = frozen_generation();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate.clone(),
        ))
        .expect("candidate");
    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::CandidateFreeze(candidate)),
        Err("k1_scheduler_candidate_replacement_forbidden")
    );
}

#[test]
fn completed_candidate_cannot_reopen_in_a_later_generation() {
    let candidate = candidate_freeze(1);
    let repeated = candidate_freeze_for_basis(2, root(707));
    assert_eq!(
        candidate.candidate_root_sha256,
        repeated.candidate_root_sha256
    );
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate.clone(),
        ))
        .expect("candidate");
    let verdict = K1GenerationTerminalVerdictV1::seal(
        candidate.freeze_root_sha256,
        None,
        Vec::new(),
        vec![root(980)],
        K1GenerationVerdictClassV1::AcquisitionFail,
        "bounded_acquisition_failed".to_owned(),
        1_100,
        None,
    )
    .expect("verdict");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            verdict,
        )))
        .expect("terminal");
    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::CandidateFreeze(repeated)),
        Err("k1_scheduler_candidate_replacement_forbidden")
    );
}

#[test]
fn old_basis_duplicate_candidate_reopens_once_under_v5() {
    let mut old = candidate_freeze_for_basis(1, root(705));
    old.schema = K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4.to_owned();
    old.freeze_root_sha256 = old.expected_root().expect("old freeze root");
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(old.clone()))
        .expect("old candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            K1GenerationTerminalVerdictV1::seal(
                old.freeze_root_sha256,
                None,
                Vec::new(),
                vec![root(980)],
                K1GenerationVerdictClassV1::AcquisitionFail,
                K1_DUPLICATE_PROTOCOL_BLOCKER_V1.to_owned(),
                1_100,
                None,
            )
            .expect("old terminal"),
        )))
        .expect("old terminal append");

    let reopened = candidate_freeze_for_basis(2, root(706));
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(reopened.clone()))
        .expect("new basis candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            K1GenerationTerminalVerdictV1::seal(
                reopened.freeze_root_sha256,
                None,
                Vec::new(),
                vec![root(981)],
                K1GenerationVerdictClassV1::AcquisitionFail,
                K1_DUPLICATE_PROTOCOL_BLOCKER_V1.to_owned(),
                1_200,
                None,
            )
            .expect("new terminal"),
        )))
        .expect("new terminal append");

    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate_freeze_for_basis(3, root(706)),
        )),
        Err("k1_scheduler_candidate_replacement_forbidden")
    );
    ledger.validate().expect("basis-aware replay");
}
