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
    let repeated = candidate_freeze(2);
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
