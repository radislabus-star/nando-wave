use nando_operator_admission::{
    ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1, LawCertificateV1,
    MechanismCertificateStatusV1, MechanismCertificateV1, OperatorCertificationEntryV1,
    OperatorCertificationLedgerV1, OperatorMechanismClassV1,
};

use super::*;
use crate::k1_natural_scheduler::authority::{
    certification_authorizes_settlement, validate_active_protocol_mode_cas,
    validate_discovery_basis_cas, validate_registry_cas,
};

#[test]
fn authority_rejects_a_valid_freeze_bound_to_an_uninstalled_discovery_basis() {
    validate_discovery_basis_cas(&candidate_freeze()).expect("installed discovery basis");
    assert_eq!(
        validate_discovery_basis_cas(&candidate_freeze_with_basis(root(999))),
        Err("k1_candidate_freeze_discovery_basis_cas_failed".to_owned())
    );
}

#[test]
fn authority_rejects_stale_known_epistemic_protocol_mode_set_root() {
    let (root_dir, config, _) = test_context();
    std::fs::write(
        &config.response_registry_path,
        serde_json::to_vec(&nando_response_actor::ResponseRegistry {
            schema: "nando.response-registry.v6".to_owned(),
            revision: 0,
            packages: Vec::new(),
        })
        .expect("registry encode"),
    )
    .expect("registry write");
    let current =
        super::super::duplicate_cohorts::known_epistemic_protocol_mode_set_root(&BTreeSet::new())
            .expect("known set root");
    validate_active_protocol_mode_cas(&config, &current).expect("current root");
    assert_eq!(
        validate_active_protocol_mode_cas(&config, &root(999)),
        Err("k1_candidate_freeze_active_protocol_mode_cas_failed".to_owned())
    );
    std::fs::remove_dir_all(root_dir).expect("cleanup");
}
use crate::k1_natural_scheduler::selection_authority::validate_queue_derivation;

#[test]
fn stale_registry_snapshot_cannot_freeze_a_generation() {
    let ledger = OperatorCertificationLedgerV1::empty().expect("empty registry");
    let stale = K1DeficitSnapshotV1::seal(
        ledger.revision.saturating_add(1),
        root(600),
        root(601),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("valid stale snapshot");

    assert_eq!(
        validate_registry_cas(&ledger, &stale),
        Err("k1_candidate_freeze_registry_cas_failed".to_owned())
    );
}

#[test]
fn transfer_settlement_cannot_precede_law_certificate_pass() {
    let bundle = root(610);
    let package = "package-one";
    let package_candidate = root(611);
    let terminal = root(612);
    let identification = root(613);
    let entry = OperatorCertificationEntryV1::seal(
        &bundle,
        package,
        &root(614),
        &root(615),
        ExecutionCertificateV1::seal(
            &bundle,
            package,
            ExecutionCertificateStatusV1::Pass,
            vec![root(616)],
            "",
        )
        .expect("execution"),
        LawCertificateV1::seal(
            &bundle,
            package,
            LawCertificateStatusV1::Partial,
            vec![
                package_candidate.clone(),
                terminal.clone(),
                identification.clone(),
            ],
            None,
            "cleanup_receipt_pending",
        )
        .expect("partial law"),
        MechanismCertificateV1::seal(
            &bundle,
            package,
            MechanismCertificateStatusV1::Collecting,
            OperatorMechanismClassV1::Unresolved,
            vec![root(617)],
            "exact_wave_collecting",
        )
        .expect("mechanism"),
        0,
    )
    .expect("entry");
    let settlement = K1TransferSettlementV1 {
        schema: "test".to_owned(),
        settlement_root_sha256: root(618),
        terminal_verdict_root_sha256: terminal,
        candidate_freeze_root_sha256: root(619),
        identification_report_root_sha256: identification,
        package_id: package.to_owned(),
        package_candidate_root_sha256: package_candidate,
        certification_entry_root_sha256: entry.entry_root_sha256.clone(),
        certification_ledger_root_sha256: root(620),
        law_certificate_root_sha256: entry.law.certificate_root_sha256.clone(),
        settled_at_unix: 1_700_000_000,
        authority_ready: false,
        phase_mutation_allowed: false,
    };

    assert!(!certification_authorizes_settlement(&entry, &settlement));
}

#[test]
fn authority_rebuilds_queue_and_rejects_a_valid_omission() {
    let rows = (1..=8)
        .map(|index| {
            K1NaturalEvidenceRowV1::seal(
                root(700 + index),
                root(799),
                root(800),
                root(801),
                root(802),
                root(if index <= 4 { 803 } else { 804 }),
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
    let catalog = build_k1_natural_cohort_catalog_v1(
        &rows,
        root(805),
        root(806),
        "nando.operator-blind-version-space-generator.v1".to_owned(),
    )
    .expect("catalog");
    let deficit = K1DeficitSnapshotV1::seal(
        0,
        root(807),
        root(808),
        0,
        0,
        0,
        0,
        0,
        3,
        3,
        2,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect("deficit");
    let contract_watermark = catalog
        .candidates
        .iter()
        .map(|candidate| candidate.last_capture_sequence)
        .max()
        .expect("candidate watermark");
    let proposed =
        build_k1_natural_candidate_queue_v1(&catalog, &deficit, contract_watermark).expect("queue");
    let completed = BTreeSet::new();
    validate_queue_derivation(
        &catalog,
        &deficit,
        &completed,
        contract_watermark,
        &proposed,
    )
    .expect("authoritative derivation");

    let omitted = proposed.rows[0].candidate_root_sha256.clone();
    let tampered =
        nando_operator_learning::multi_source::build_k1_natural_candidate_queue_with_exclusions_v1(
            &catalog,
            &deficit,
            &BTreeSet::from([omitted]),
            contract_watermark,
        )
        .expect("internally valid omitted queue");
    assert_eq!(
        validate_queue_derivation(
            &catalog,
            &deficit,
            &completed,
            contract_watermark,
            &tampered,
        ),
        Err("k1_candidate_queue_derivation_mismatch".to_owned())
    );
}
