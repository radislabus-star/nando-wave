use nando_operator_learning::multi_source::{
    K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1,
};

use super::*;
use crate::k1_natural_scheduler::duplicate_cohorts::duplicate_candidate_exclusions;

fn ledger_with_terminal(blocker: &str) -> (K1SchedulerLedgerV1, K1NaturalCandidateFreezeV1) {
    let freeze = candidate_freeze();
    let terminal = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        vec![root(900)],
        K1GenerationVerdictClassV1::AcquisitionFail,
        blocker.to_owned(),
        1_700_000_001,
        None,
    )
    .expect("terminal");
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    ledger
        .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()))
        .expect("candidate");
    ledger
        .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
            terminal,
        )))
        .expect("terminal");
    (ledger, freeze)
}

fn catalog_with_additional_evidence() -> K1NaturalCohortCatalogV1 {
    let rows = (1..=9)
        .map(|index| {
            K1NaturalEvidenceRowV1::seal(
                root(1_000 + index),
                root(200),
                root(201),
                root(202),
                root(if index <= 4 { 300 } else { 301 }),
                K1ConsequenceTypeV1::Scalar,
                K1NaturalEvidenceClassV1::NaturalLive,
                index,
                100,
                1_000,
                true,
                index <= 2,
                false,
            )
            .expect("evidence")
        })
        .collect::<Vec<_>>();
    build_k1_natural_cohort_catalog_v1(
        &rows,
        root(910),
        root(401),
        "nando.operator-blind-version-space-generator.v1".to_owned(),
    )
    .expect("catalog")
}

#[test]
fn completed_duplicate_cohort_stays_excluded_when_evidence_manifest_grows() {
    let (ledger, freeze) = ledger_with_terminal("all_supported_t1_protocol_modes_already_active");
    let catalog = catalog_with_additional_evidence();
    let candidate = &catalog.candidates[0];

    assert_ne!(
        candidate.candidate_root_sha256,
        freeze.candidate_root_sha256
    );
    let excluded =
        duplicate_candidate_exclusions(&ledger, &catalog, &freeze.epistemic_registry_root_sha256)
            .expect("exclusions");
    assert_eq!(
        excluded,
        BTreeSet::from([candidate.candidate_root_sha256.clone()])
    );
}

#[test]
fn registry_change_reopens_duplicate_cohort() {
    let (ledger, freeze) = ledger_with_terminal("all_supported_t1_protocol_modes_already_active");
    let catalog = catalog_with_additional_evidence();

    let excluded =
        duplicate_candidate_exclusions(&ledger, &catalog, &root(999)).expect("exclusions");
    assert_ne!(root(999), freeze.epistemic_registry_root_sha256);
    assert!(excluded.is_empty());
}

#[test]
fn repairable_acquisition_failure_does_not_suppress_cohort() {
    let (ledger, freeze) = ledger_with_terminal("source_neutral_self_replay_failed");
    let catalog = catalog_with_additional_evidence();

    let excluded =
        duplicate_candidate_exclusions(&ledger, &catalog, &freeze.epistemic_registry_root_sha256)
            .expect("exclusions");
    assert!(excluded.is_empty());
}
