use super::*;
use crate::k1_natural_scheduler::authority::append_and_persist;
use crate::k1_natural_scheduler::journal::restore_anchored_scheduler_for;
use crate::k1_natural_scheduler::projection::exact_attempt_index_for;

fn acquisition_fail(
    freeze: &K1NaturalCandidateFreezeV1,
    diagnostic: Option<&TerminalDiagnosticV1>,
) -> K1GenerationTerminalVerdictV1 {
    let mut evidence = vec![freeze.freeze_root_sha256.clone()];
    if let Some(diagnostic) = diagnostic {
        evidence.push(diagnostic.terminal_diagnostic_root_sha256.clone());
    }
    K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        evidence,
        K1GenerationVerdictClassV1::AcquisitionFail,
        "motif_program_candidates_empty".to_owned(),
        1_700_000_100,
        None,
    )
    .expect("terminal verdict")
}

#[test]
fn legacy_terminal_is_counted_but_never_becomes_an_exact_attempt() {
    let freeze = candidate_freeze();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()))
        .expect("freeze");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            K1GenerationTerminalVerdictV1::seal(
                freeze.freeze_root_sha256,
                None,
                Vec::new(),
                vec![root(16_000)],
                K1GenerationVerdictClassV1::AcquisitionFail,
                "bounded_acquisition_failed".to_owned(),
                1_700_000_100,
                None,
            )
            .expect("legacy terminal"),
        )))
        .expect("append terminal");

    let index = exact_attempt_index_for(&ledger).expect("index");
    assert!(index.deterministic_attempts.is_empty());
    assert_eq!(index.legacy_unbound_terminals, 1);
}

#[test]
fn diagnostic_without_terminal_verdict_is_not_an_attempt() {
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze))
        .expect("freeze");
    ledger
        .append(K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(
            Box::new(diagnostic),
        ))
        .expect("diagnostic");

    let index = exact_attempt_index_for(&ledger).expect("index");
    assert!(index.deterministic_attempts.is_empty());
    assert_eq!(index.legacy_unbound_terminals, 0);
}

#[test]
fn matching_diagnostic_and_verdict_create_one_exact_attempt() {
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let verdict = acquisition_fail(&freeze, Some(&diagnostic));
    let opportunity = diagnostic.opportunity_root_sha256.clone();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze),
        K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic)),
        K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
    ] {
        ledger.append(payload).expect("append exact event");
    }

    let index = exact_attempt_index_for(&ledger).expect("index");
    assert_eq!(index.deterministic_attempts.len(), 1);
    assert!(index.contains(&opportunity));
}

#[test]
fn forged_or_unbound_terminal_verdict_is_rejected() {
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()))
        .expect("freeze");
    ledger
        .append(K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(
            Box::new(diagnostic),
        ))
        .expect("diagnostic");

    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            acquisition_fail(&freeze, None),
        ))),
        Err("k1_scheduler_exact_terminal_verdict_mismatch")
    );
}

#[test]
fn persisted_exact_attempt_index_is_byte_identical_after_restart() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let verdict = acquisition_fail(&freeze, Some(&diagnostic));
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze),
        K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic)),
        K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
    ] {
        append_and_persist(
            &config,
            K1SchedulerLaneV1::Epistemic,
            &signing_key,
            &mut ledger,
            payload,
        )
        .expect("persist exact event");
    }
    let before = serde_json::to_vec(&exact_attempt_index_for(&ledger).expect("before index"))
        .expect("before bytes");
    let restored =
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic).expect("restore");
    let after = serde_json::to_vec(&exact_attempt_index_for(&restored).expect("after index"))
        .expect("after bytes");

    assert_eq!(after, before);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn mechanism_lane_cannot_change_the_epistemic_exact_index() {
    let (root_dir, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis");
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let verdict = acquisition_fail(&freeze, Some(&diagnostic));
    let mut mechanism = K1SchedulerLedgerV1::empty().expect("mechanism ledger");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(freeze),
        K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(Box::new(diagnostic)),
        K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(verdict)),
    ] {
        append_and_persist(
            &config,
            K1SchedulerLaneV1::Mechanism,
            &signing_key,
            &mut mechanism,
            payload,
        )
        .expect("persist mechanism event");
    }
    let epistemic = K1SchedulerLedgerV1::empty().expect("uncreated epistemic lane");

    assert_eq!(
        exact_attempt_index_for(&mechanism)
            .expect("mechanism index")
            .deterministic_attempts
            .len(),
        1
    );
    assert!(
        exact_attempt_index_for(&epistemic)
            .expect("epistemic index")
            .deterministic_attempts
            .is_empty()
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}
