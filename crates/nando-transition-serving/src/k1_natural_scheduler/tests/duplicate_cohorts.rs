use nando_operator_learning::multi_source::{
    K1_DUPLICATE_PROTOCOL_BLOCKER_V1, K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1,
};

use super::*;
use crate::k1_natural_scheduler::duplicate_cohorts::{
    duplicate_candidate_exclusions, effective_candidate_exclusions,
};

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

#[test]
fn effective_exclusions_reopen_only_old_basis_duplicate_terminal() {
    let active_set_root = root(900);
    let old_basis = root(950);
    let new_basis = root(951);
    let catalog = catalog_with_additional_evidence();
    let (old_duplicate, old_freeze) = ledger_with_terminal_for_basis(
        K1_DUPLICATE_PROTOCOL_BLOCKER_V1,
        vec![active_set_root.clone()],
        old_basis,
    );
    let reopened = effective_candidate_exclusions(
        &old_duplicate,
        &catalog,
        &active_set_root,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5,
        &new_basis,
    )
    .expect("old basis exclusions");
    assert!(!reopened.contains(&old_freeze.candidate_root_sha256));

    let same_basis = effective_candidate_exclusions(
        &old_duplicate,
        &catalog,
        &active_set_root,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5,
        &old_freeze.discovery_basis_root_sha256,
    )
    .expect("same basis exclusions");
    assert!(same_basis.contains(&old_freeze.candidate_root_sha256));

    let (non_duplicate, non_duplicate_freeze) = ledger_with_terminal_for_basis(
        "selected_role_witness_missing",
        vec![active_set_root.clone()],
        root(952),
    );
    let non_duplicate_exclusions = effective_candidate_exclusions(
        &non_duplicate,
        &catalog,
        &active_set_root,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V5,
        &new_basis,
    )
    .expect("non-duplicate exclusions");
    assert!(non_duplicate_exclusions.contains(&non_duplicate_freeze.candidate_root_sha256));

    let legacy_schema_exclusions = effective_candidate_exclusions(
        &old_duplicate,
        &catalog,
        &active_set_root,
        K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V4,
        &new_basis,
    )
    .expect("legacy schema exclusions");
    assert!(legacy_schema_exclusions.contains(&old_freeze.candidate_root_sha256));
}

#[test]
fn projection_preserves_two_basis_attempts_for_the_same_candidate_bytes() {
    let old = candidate_freeze_for_generation_and_basis(1, root(950));
    let current = candidate_freeze_for_generation_and_basis(2, root(951));
    assert_eq!(old.candidate_root_sha256, current.candidate_root_sha256);
    let mut ledger = K1SchedulerLedgerV1::empty().expect("ledger");
    for freeze in [old, current] {
        ledger
            .append(K1SchedulerEventPayloadV1::CandidateFreeze(freeze.clone()))
            .expect("basis candidate");
        ledger
            .append(K1SchedulerEventPayloadV1::TerminalVerdict(Box::new(
                K1GenerationTerminalVerdictV1::seal(
                    freeze.freeze_root_sha256,
                    None,
                    Vec::new(),
                    vec![root(900)],
                    K1GenerationVerdictClassV1::AcquisitionFail,
                    K1_DUPLICATE_PROTOCOL_BLOCKER_V1.to_owned(),
                    1_700_000_000 + freeze.generation_sequence,
                    None,
                )
                .expect("terminal"),
            )))
            .expect("terminal append");
    }

    let projection = super::super::projection::projection_for(&ledger).expect("projection");
    assert_eq!(projection.completed_generations, 2);
    assert_eq!(projection.completed_candidate_roots_sha256.len(), 2);
    assert_eq!(
        projection.completed_candidate_roots_sha256[0],
        projection.completed_candidate_roots_sha256[1]
    );
}
