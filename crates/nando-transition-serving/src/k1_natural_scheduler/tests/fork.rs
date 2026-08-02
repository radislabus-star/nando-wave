use super::*;
use crate::k1_natural_scheduler::authority::append_and_persist;
use crate::k1_natural_scheduler::fork::{ensure_epistemic_lane, epistemic_exclusions};
use crate::k1_natural_scheduler::journal::{
    restore_anchored_scheduler_for, scheduler_anchor_path_for, scheduler_journal_path_for,
};

fn mechanism_generation() -> (K1NaturalCandidateFreezeV1, K1IdentificationFreezeV1) {
    let candidate = candidate_freeze();
    let identification = K1IdentificationFreezeV1::seal(
        &candidate,
        root(1_300),
        "nando.operator-blind-version-space-generator.v1".to_owned(),
        vec![root(1_301)],
        root(1_302),
        root(1_303),
        "nando.multi-source-t1-passive-outcome-partition.v2".to_owned(),
    )
    .expect("identification");
    (candidate, identification)
}

fn append_mechanism_generation(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    candidate: &K1NaturalCandidateFreezeV1,
    identification: &K1IdentificationFreezeV1,
) {
    recover_authority(config, signing_key).expect("genesis");
    let mut ledger = restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Mechanism)
        .expect("mechanism ledger");
    append_and_persist(
        config,
        K1SchedulerLaneV1::Mechanism,
        signing_key,
        &mut ledger,
        K1SchedulerEventPayloadV1::CandidateFreeze(candidate.clone()),
    )
    .expect("candidate");
    append_and_persist(
        config,
        K1SchedulerLaneV1::Mechanism,
        signing_key,
        &mut ledger,
        K1SchedulerEventPayloadV1::IdentificationFreeze(identification.clone()),
    )
    .expect("identification");
}

#[test]
fn signed_fork_preserves_mechanism_bytes_and_excludes_watched_candidate() {
    let (root_dir, config, signing_key) = test_context();
    let (candidate, identification) = mechanism_generation();
    append_mechanism_generation(&config, &signing_key, &candidate, &identification);
    let journal = scheduler_journal_path_for(&config, K1SchedulerLaneV1::Mechanism);
    let anchor =
        scheduler_anchor_path_for(&config, K1SchedulerLaneV1::Mechanism).expect("anchor path");
    let before_events = [1, 2].map(|sequence| {
        fs::read(journal.join(format!("{sequence:020}.json"))).expect("mechanism event")
    });
    let before_anchor = fs::read(&anchor).expect("mechanism anchor");

    ensure_epistemic_lane(&config, &signing_key).expect("signed fork");

    let after_events = [1, 2].map(|sequence| {
        fs::read(journal.join(format!("{sequence:020}.json"))).expect("mechanism event")
    });
    assert_eq!(before_events, after_events);
    assert_eq!(before_anchor, fs::read(anchor).expect("mechanism anchor"));
    assert_eq!(
        epistemic_exclusions(&config).expect("exclusions"),
        BTreeSet::from([candidate.candidate_root_sha256])
    );
    let epistemic = restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic)
        .expect("epistemic prefix");
    assert_eq!(epistemic.revision, 0);
    fs::remove_dir_all(root_dir).expect("cleanup");
}

#[test]
fn epistemic_anchor_detects_rollback_independently() {
    let (root_dir, config, signing_key) = test_context();
    let (candidate, identification) = mechanism_generation();
    append_mechanism_generation(&config, &signing_key, &candidate, &identification);
    ensure_epistemic_lane(&config, &signing_key).expect("fork");
    let mut epistemic =
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic).expect("prefix");
    append_and_persist(
        &config,
        K1SchedulerLaneV1::Epistemic,
        &signing_key,
        &mut epistemic,
        K1SchedulerEventPayloadV1::CandidateFreeze(candidate),
    )
    .expect("epistemic event");
    fs::remove_file(
        scheduler_journal_path_for(&config, K1SchedulerLaneV1::Epistemic)
            .join("00000000000000000001.json"),
    )
    .expect("remove event");
    assert_eq!(
        restore_anchored_scheduler_for(&config, K1SchedulerLaneV1::Epistemic),
        Err("k1_scheduler_rollback_detected".to_owned())
    );
    fs::remove_dir_all(root_dir).expect("cleanup");
}
