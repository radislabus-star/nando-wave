use super::*;
use crate::k1_natural_scheduler::authority::append_and_persist;
use crate::k1_natural_scheduler::journal::{encode_hex, restore_anchored_scheduler_for};
use crate::k1_natural_scheduler::projection::exact_attempt_index_for;
use sha2::{Digest, Sha256};

const P6_PRODUCTION_LEDGER_SHA256: &str =
    "00e44ee2c9127c71231bb2b413500fbe1a4693e1c834c5cf061f60c8df8cd362";
const P6_PRODUCTION_LEDGER_BYTES: usize = 2_246_130;

fn acquisition_fail(
    freeze: &K1NaturalCandidateFreezeV1,
    diagnostic: Option<&TerminalDiagnosticV1>,
) -> K1GenerationTerminalVerdictV1 {
    let mut evidence = vec![freeze.freeze_root_sha256.clone()];
    if let Some(diagnostic) = diagnostic {
        evidence.push(diagnostic.terminal_diagnostic_root_sha256.clone());
        evidence.push(diagnostic.identifier_report_root_sha256.clone());
        evidence.push(diagnostic.identifier_result_root_sha256.clone());
    }
    K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        evidence,
        K1GenerationVerdictClassV1::AcquisitionFail,
        "motif_program_candidates_empty".to_owned(),
        diagnostic.map_or(1_700_000_100, |value| value.terminal_at_unix),
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
fn frozen_production_copy_is_byte_exact_and_preserves_active_legacy_generation() {
    let Some(path) = std::env::var_os("NANDO_K1_P6_PRODUCTION_LEDGER_COPY") else {
        return;
    };
    let bytes = fs::read(path).expect("frozen production ledger copy");
    assert_eq!(bytes.len(), P6_PRODUCTION_LEDGER_BYTES);
    assert_eq!(
        encode_hex(&Sha256::digest(&bytes)),
        P6_PRODUCTION_LEDGER_SHA256
    );

    let ledger: K1SchedulerLedgerV1 =
        serde_json::from_slice(&bytes).expect("decode production ledger copy");
    ledger.validate().expect("validate production ledger copy");
    assert_eq!(
        serde_json::to_vec(&ledger).expect("re-encode ledger"),
        bytes
    );
    assert_eq!(ledger.revision, 1_174);

    let freeze_counts = ledger
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze) => Some(freeze.schema.as_str()),
            _ => None,
        })
        .fold(std::collections::BTreeMap::new(), |mut counts, schema| {
            *counts.entry(schema.to_owned()).or_insert(0_u64) += 1;
            counts
        });
    assert_eq!(
        freeze_counts,
        std::collections::BTreeMap::from([
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1.to_owned(), 40),
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2.to_owned(), 37),
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V3.to_owned(), 8),
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4.to_owned(), 37),
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5.to_owned(), 32),
            (K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6.to_owned(), 432),
        ])
    );
    let active = ledger
        .active_candidate_freeze()
        .expect("active legacy generation");
    assert_eq!(active.schema, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V6);
    assert_eq!(active.generation_sequence, 586);
    assert_eq!(
        active.freeze_root_sha256,
        "685ead18cde7fa40330a743e474758b5ee0436115730418903d7c6fde94afadd"
    );
    let index = exact_attempt_index_for(&ledger).expect("legacy exact index");
    assert!(index.deterministic_attempts.is_empty());
    assert_eq!(index.legacy_unbound_terminals, 585);

    let prefix = ledger.events.clone();
    let mut completed = ledger;
    let active = completed
        .active_candidate_freeze()
        .expect("active generation")
        .clone();
    completed
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            K1GenerationTerminalVerdictV1::seal(
                active.freeze_root_sha256.clone(),
                None,
                Vec::new(),
                vec![active.freeze_root_sha256],
                K1GenerationVerdictClassV1::AcquisitionFail,
                "bounded_acquisition_failed".to_owned(),
                active.selected_at_unix.saturating_add(1),
                None,
            )
            .expect("legacy terminal"),
        )))
        .expect("complete active legacy generation");
    assert_eq!(&completed.events[..prefix.len()], prefix.as_slice());
    assert!(completed.active_candidate_freeze().is_none());
    assert_eq!(
        exact_attempt_index_for(&completed)
            .expect("completed legacy index")
            .legacy_unbound_terminals,
        586
    );
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
fn exact_verdict_missing_identifier_result_root_is_rejected() {
    let freeze = exact_candidate_freeze(1);
    let diagnostic = exact_terminal_diagnostic(&freeze);
    let incomplete = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        vec![
            freeze.freeze_root_sha256.clone(),
            diagnostic.terminal_diagnostic_root_sha256.clone(),
            diagnostic.identifier_report_root_sha256.clone(),
        ],
        K1GenerationVerdictClassV1::AcquisitionFail,
        diagnostic.exact_result_blocker.clone(),
        diagnostic.terminal_at_unix,
        None,
    )
    .expect("incomplete terminal verdict");
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze))
        .expect("freeze");
    ledger
        .append(K1SchedulerEventPayloadV1::ExactTerminalDiagnostic(
            Box::new(diagnostic),
        ))
        .expect("diagnostic");

    assert_eq!(
        ledger.append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            incomplete,
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
