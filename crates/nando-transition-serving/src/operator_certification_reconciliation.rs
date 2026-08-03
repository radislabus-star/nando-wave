use nando_operator_admission::{
    ExecutionCertificateStatusV1, LawCertificateStatusV1, MechanismCertificateStatusV1,
    OperatorCertificationEntryV1, OperatorCertificationLedgerV1,
};

pub(crate) fn preserve_monotonic_certificates(
    ledger: &mut OperatorCertificationLedgerV1,
    requested: OperatorCertificationEntryV1,
) -> Result<Option<(bool, OperatorCertificationEntryV1)>, String> {
    let Some(previous) = ledger
        .latest_entries()
        .into_iter()
        .find(|entry| entry.package_id == requested.package_id)
        .cloned()
    else {
        return Ok(None);
    };
    if previous.role_topology_id_sha256 != requested.role_topology_id_sha256 {
        return Ok(None);
    }
    let execution = if matches!(
        (previous.execution.status, requested.execution.status),
        (
            ExecutionCertificateStatusV1::Pass,
            ExecutionCertificateStatusV1::Pending
        ) | (
            ExecutionCertificateStatusV1::Revoked,
            ExecutionCertificateStatusV1::Pending | ExecutionCertificateStatusV1::Pass
        )
    ) {
        previous.execution.clone()
    } else {
        requested.execution
    };
    let law = if matches!(
        (previous.law.status, requested.law.status),
        (
            LawCertificateStatusV1::Pass,
            LawCertificateStatusV1::Partial | LawCertificateStatusV1::Legacy
        ) | (
            LawCertificateStatusV1::Rejected,
            LawCertificateStatusV1::Partial
                | LawCertificateStatusV1::Pass
                | LawCertificateStatusV1::Legacy
        ) | (
            LawCertificateStatusV1::Legacy,
            LawCertificateStatusV1::Partial | LawCertificateStatusV1::Pass
        ) | (
            LawCertificateStatusV1::Partial,
            LawCertificateStatusV1::Legacy
        )
    ) {
        previous.law.clone()
    } else {
        requested.law
    };
    let mechanism = if (matches!(
        previous.mechanism.status,
        MechanismCertificateStatusV1::Pass | MechanismCertificateStatusV1::Fail
    ) && previous.mechanism.status != requested.mechanism.status)
        || matches!(
            (previous.mechanism.status, requested.mechanism.status),
            (
                MechanismCertificateStatusV1::Collecting,
                MechanismCertificateStatusV1::NotEvaluated
            )
        ) {
        previous.mechanism.clone()
    } else {
        requested.mechanism
    };
    if execution == previous.execution
        && law == previous.law
        && mechanism == previous.mechanism
        && requested.false_bad_apply == previous.false_bad_apply
    {
        return Ok(Some((false, previous)));
    }
    let reconciled = OperatorCertificationEntryV1::seal(
        &requested.bundle_id_sha256,
        &requested.package_id,
        &requested.semantic_law_id_sha256,
        &requested.role_topology_id_sha256,
        execution,
        law,
        mechanism,
        requested.false_bad_apply,
    )
    .map_err(str::to_owned)?;
    let changed = ledger.append(reconciled.clone()).map_err(str::to_owned)?;
    Ok(Some((changed, reconciled)))
}

#[cfg(test)]
mod tests {
    use nando_operator_admission::{
        ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1,
        LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
        OperatorCertificationEntryV1, OperatorCertificationLedgerV1, OperatorMechanismClassV1,
    };
    use nando_operator_kernel::sha256_bytes;

    use super::preserve_monotonic_certificates;

    fn root(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn entry(status: MechanismCertificateStatusV1) -> OperatorCertificationEntryV1 {
        let bundle = root("bundle");
        let package = "natural-package";
        OperatorCertificationEntryV1::seal(
            &bundle,
            package,
            &root("law"),
            &root("topology"),
            ExecutionCertificateV1::seal(
                &bundle,
                package,
                ExecutionCertificateStatusV1::Pass,
                vec![root("execution")],
                "",
            )
            .expect("execution"),
            LawCertificateV1::seal(
                &bundle,
                package,
                LawCertificateStatusV1::Pass,
                vec![root("law-evidence")],
                Some(root("cleanup")),
                "",
            )
            .expect("law"),
            MechanismCertificateV1::seal(
                &bundle,
                package,
                status,
                OperatorMechanismClassV1::Unresolved,
                vec![root("mechanism")],
                match status {
                    MechanismCertificateStatusV1::Collecting => "holdout_collecting",
                    MechanismCertificateStatusV1::Fail => "wave_causal_not_proven",
                    _ => "",
                },
            )
            .expect("mechanism"),
            0,
        )
        .expect("entry")
    }

    #[test]
    fn terminal_fail_cannot_regress_to_collecting_without_topology_change() {
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("ledger");
        ledger
            .append(entry(MechanismCertificateStatusV1::Collecting))
            .expect("collecting");
        ledger
            .append(entry(MechanismCertificateStatusV1::Fail))
            .expect("terminal fail");
        let requested = entry(MechanismCertificateStatusV1::Collecting);
        assert_eq!(
            ledger.append(requested.clone()),
            Err("operator_certification_transition_invalid")
        );
        let (_, reconciled) = preserve_monotonic_certificates(&mut ledger, requested)
            .expect("reconcile")
            .expect("terminal reconciliation");
        assert_eq!(
            reconciled.mechanism.status,
            MechanismCertificateStatusV1::Fail
        );
        assert_eq!(ledger.revision, 2);
    }

    #[test]
    fn blocked_projection_cannot_regress_execution_or_terminal_mechanism() {
        let mut ledger = OperatorCertificationLedgerV1::empty().expect("ledger");
        ledger
            .append(entry(MechanismCertificateStatusV1::Collecting))
            .expect("collecting");
        let terminal = entry(MechanismCertificateStatusV1::Fail);
        ledger.append(terminal.clone()).expect("terminal fail");
        let mut requested = entry(MechanismCertificateStatusV1::Collecting);
        requested = OperatorCertificationEntryV1::seal(
            &requested.bundle_id_sha256,
            &requested.package_id,
            &requested.semantic_law_id_sha256,
            &requested.role_topology_id_sha256,
            ExecutionCertificateV1::seal(
                &requested.bundle_id_sha256,
                &requested.package_id,
                ExecutionCertificateStatusV1::Pending,
                vec![root("pending")],
                "ordinary_cpu_completion_pending",
            )
            .expect("pending execution"),
            requested.law,
            requested.mechanism,
            0,
        )
        .expect("regressed projection");
        assert_eq!(
            ledger.append(requested.clone()),
            Err("operator_certification_transition_invalid")
        );
        let (_, reconciled) = preserve_monotonic_certificates(&mut ledger, requested)
            .expect("reconcile")
            .expect("monotonic reconciliation");
        assert_eq!(
            reconciled.execution.status,
            ExecutionCertificateStatusV1::Pass
        );
        assert_eq!(
            reconciled.mechanism.status,
            MechanismCertificateStatusV1::Fail
        );
        assert_eq!(ledger.revision, 2);
    }
}
