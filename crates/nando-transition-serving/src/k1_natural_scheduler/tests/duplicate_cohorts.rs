use nando_operator_learning::multi_source::{
    K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1,
};

use super::*;
use crate::k1_natural_scheduler::duplicate_cohorts::duplicate_candidate_exclusions;

fn ledger_with_terminal(
    blocker: &str,
    evidence_roots_sha256: Vec<String>,
) -> (K1SchedulerLedgerV1, K1NaturalCandidateFreezeV1) {
    ledger_with_terminal_for_basis(
        blocker,
        evidence_roots_sha256,
        natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    )
}

fn ledger_with_terminal_for_basis(
    blocker: &str,
    evidence_roots_sha256: Vec<String>,
    discovery_basis_root_sha256: String,
) -> (K1SchedulerLedgerV1, K1NaturalCandidateFreezeV1) {
    let freeze = candidate_freeze_with_basis(discovery_basis_root_sha256);
    let terminal = K1GenerationTerminalVerdictV1::seal(
        freeze.freeze_root_sha256.clone(),
        None,
        Vec::new(),
        evidence_roots_sha256,
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
                root(199),
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
        nando_operator_learning::multi_source::MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V3.to_owned(),
    )
    .expect("catalog")
}

#[test]
fn completed_duplicate_cohort_stays_excluded_when_evidence_manifest_grows() {
    let active_set_root = root(900);
    let (ledger, freeze) = ledger_with_terminal(
        "all_supported_t1_protocol_modes_already_active",
        vec![active_set_root.clone()],
    );
    let catalog = catalog_with_additional_evidence();
    let candidate = &catalog.candidates[0];

    assert_ne!(
        candidate.candidate_root_sha256,
        freeze.candidate_root_sha256
    );
    let excluded = duplicate_candidate_exclusions(
        &ledger,
        &catalog,
        &active_set_root,
        &freeze.discovery_basis_root_sha256,
    )
    .expect("exclusions");
    assert_eq!(
        excluded,
        BTreeSet::from([candidate.candidate_root_sha256.clone()])
    );
}

#[test]
fn active_protocol_mode_set_change_reopens_duplicate_cohort() {
    let (ledger, _) = ledger_with_terminal(
        "all_supported_t1_protocol_modes_already_active",
        vec![root(900)],
    );
    let catalog = catalog_with_additional_evidence();

    let excluded = duplicate_candidate_exclusions(
        &ledger,
        &catalog,
        &root(999),
        &natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    )
    .expect("exclusions");
    assert!(excluded.is_empty());
}

#[test]
fn unrelated_epistemic_registry_change_does_not_reopen_duplicate_cohort() {
    let active_set_root = root(900);
    let (ledger, freeze) = ledger_with_terminal(
        "all_supported_t1_protocol_modes_already_active",
        vec![active_set_root.clone()],
    );
    assert_ne!(freeze.epistemic_registry_root_sha256, root(999));
    let excluded = duplicate_candidate_exclusions(
        &ledger,
        &catalog_with_additional_evidence(),
        &active_set_root,
        &freeze.discovery_basis_root_sha256,
    )
    .expect("exclusions");
    assert_eq!(excluded.len(), 1);
}

#[test]
fn legacy_duplicate_terminal_without_active_set_root_is_re_evaluated() {
    let (ledger, _) = ledger_with_terminal(
        "all_supported_t1_protocol_modes_already_active",
        vec![root(901)],
    );
    let excluded = duplicate_candidate_exclusions(
        &ledger,
        &catalog_with_additional_evidence(),
        &root(900),
        &natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    )
    .expect("exclusions");
    assert!(excluded.is_empty());
}

#[test]
fn repairable_acquisition_failure_does_not_suppress_cohort() {
    let (ledger, _) = ledger_with_terminal("source_neutral_self_replay_failed", vec![root(900)]);
    let catalog = catalog_with_additional_evidence();

    let excluded = duplicate_candidate_exclusions(
        &ledger,
        &catalog,
        &root(900),
        &natural_t1_discovery_basis_root_v3().expect("discovery basis"),
    )
    .expect("exclusions");
    assert!(excluded.is_empty());
}

#[test]
fn discovery_basis_change_reopens_once_then_new_terminal_excludes() {
    let active_set_root = root(900);
    let old_basis = root(950);
    let new_basis = root(951);
    let catalog = catalog_with_additional_evidence();
    let (old_ledger, _) = ledger_with_terminal_for_basis(
        "all_supported_t1_protocol_modes_already_active",
        vec![active_set_root.clone()],
        old_basis,
    );

    assert!(
        duplicate_candidate_exclusions(&old_ledger, &catalog, &active_set_root, &new_basis,)
            .expect("reopened exclusions")
            .is_empty()
    );

    let (new_ledger, _) = ledger_with_terminal_for_basis(
        "all_supported_t1_protocol_modes_already_active",
        vec![active_set_root.clone()],
        new_basis.clone(),
    );
    assert_eq!(
        duplicate_candidate_exclusions(&new_ledger, &catalog, &active_set_root, &new_basis,)
            .expect("new basis exclusions"),
        BTreeSet::from([catalog.candidates[0].candidate_root_sha256.clone()])
    );
}
