use serde_json::Value;

use super::*;
use crate::k1_natural_scheduler::journal::{
    persist_scheduler_anchor, persist_scheduler_event, restore_anchored_scheduler,
    restore_anchored_scheduler_for, scheduler_anchor_path, scheduler_anchor_path_for,
    scheduler_genesis_root, scheduler_journal_path,
};

#[test]
fn fresh_authority_recovery_initializes_both_lane_anchors_without_events() {
    let (root, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("fresh recovery");

    for lane in [K1SchedulerLaneV1::Mechanism, K1SchedulerLaneV1::Epistemic] {
        let ledger = restore_anchored_scheduler_for(&config, lane).expect("anchored empty lane");
        assert_eq!(ledger.revision, 0);
        assert!(ledger.latest_event().is_none());
        assert!(
            scheduler_anchor_path_for(&config, lane)
                .expect("anchor path")
                .is_file()
        );
    }

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn genesis_anchor_detects_unsigned_first_event_rollback() {
    let (root, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis anchor");
    let anchored = restore_anchored_scheduler(&config).expect("anchored empty");
    assert_eq!(anchored.revision, 0);

    fs::remove_file(scheduler_anchor_path(&config).expect("anchor path")).expect("remove anchor");
    assert!(restore_anchored_scheduler(&config).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn signed_crash_tail_advances_only_after_prefix_anchor_matches() {
    let (root, config, signing_key) = test_context();
    recover_authority(&config, &signing_key).expect("genesis anchor");
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    let event = ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate_freeze(),
        ))
        .expect("candidate event")
        .clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), &signing_key)
            .expect("signed event");
    persist_scheduler_event(&config, &signed).expect("crash tail");

    assert_eq!(
        restore_anchored_scheduler(&config),
        Err("k1_scheduler_rollback_detected".to_owned())
    );
    recover_authority(&config, &signing_key).expect("recover signed tail");
    assert_eq!(
        restore_anchored_scheduler(&config)
            .expect("recovered")
            .revision,
        1
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stale_crash_temp_is_replaced_before_atomic_journal_publish() {
    let (root, config, signing_key) = test_context();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    let event = ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate_freeze(),
        ))
        .expect("candidate event")
        .clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), &signing_key)
            .expect("signed event");
    let directory = scheduler_journal_path(&config);
    fs::create_dir_all(&directory).expect("journal directory");
    let temporary = directory.join(format!(
        ".{:020}-{}.tmp",
        signed.event.sequence, signed.event.event_root_sha256
    ));
    fs::write(&temporary, b"truncated-crash-tail").expect("orphan temp");

    persist_scheduler_event(&config, &signed).expect("atomic retry");

    assert!(!temporary.exists());
    let final_path = directory.join("00000000000000000001.json");
    let restored: SignedSchedulerEventV1 =
        serde_json::from_slice(&fs::read(final_path).expect("published event"))
            .expect("complete signed event");
    assert_eq!(restored, signed);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn existing_final_journal_event_is_never_replaced() {
    let (root, config, signing_key) = test_context();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    let event = ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate_freeze(),
        ))
        .expect("candidate event")
        .clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), &signing_key)
            .expect("signed event");
    persist_scheduler_event(&config, &signed).expect("first publish");
    assert_eq!(
        persist_scheduler_event(&config, &signed),
        Err("k1_scheduler_journal_replacement_forbidden".to_owned())
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn journal_tamper_is_rejected_even_when_anchor_still_exists() {
    let (root, config, signing_key) = test_context();
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    let event = ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(
            candidate_freeze(),
        ))
        .expect("candidate event")
        .clone();
    let signed =
        SignedSchedulerEventV1::seal(event, ledger.ledger_root_sha256.clone(), &signing_key)
            .expect("signed event");
    persist_scheduler_event(&config, &signed).expect("event");
    persist_scheduler_anchor(
        &config,
        &signing_key,
        &ledger,
        &signed.event.event_root_sha256,
    )
    .expect("anchor");

    let path = scheduler_journal_path(&config).join("00000000000000000001.json");
    let mut value: Value =
        serde_json::from_slice(&fs::read(&path).expect("journal read")).expect("journal json");
    value["resulting_ledger_root_sha256"] = Value::String(root_sha256(999));
    fs::write(&path, serde_json::to_vec(&value).expect("tamper encode")).expect("tamper write");
    assert!(restore_anchored_scheduler(&config).is_err());
    fs::remove_dir_all(root).expect("cleanup");
}

fn root_sha256(value: u64) -> String {
    format!("{value:064x}")
}

#[test]
fn genesis_root_is_nonzero_and_stable() {
    assert_eq!(scheduler_genesis_root(), scheduler_genesis_root());
    assert_ne!(scheduler_genesis_root(), root_sha256(0));
}
