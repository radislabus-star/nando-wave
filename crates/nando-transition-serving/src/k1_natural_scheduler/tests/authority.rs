use nando_operator_admission::{
    ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1, LawCertificateV1,
    MechanismCertificateStatusV1, MechanismCertificateV1, OperatorCertificationEntryV1,
    OperatorCertificationLedgerV1, OperatorMechanismClassV1,
};

use super::*;
use crate::k1_natural_scheduler::authority::{
    certification_authorizes_settlement, validate_registry_cas,
};

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
