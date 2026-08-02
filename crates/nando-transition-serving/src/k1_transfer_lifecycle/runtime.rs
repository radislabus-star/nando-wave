use std::path::Path;

use nando_operator_learning::{TeacherTransition, multi_source::K1TransferSettlementV1};

use crate::k1_natural_scheduler::append_transfer_settlement;
use crate::live_economics::durable_package_completions;
use crate::operator_certification::{CertificationAuthorityConfigV1, restore_cleanup_receipt};
use crate::operator_cleanup::request_cleanup;

use super::candidate::{
    active_package, candidate_from_terminal, cleanup_request, package_candidate_root,
};
use super::certification::certify_transfer;
use super::completion::first_post_terminal_completion;
use super::model::{K1TransferLifecycleReportV1, K1TransferLifecycleStageV1};
use nando_operator_learning::multi_source::K1GenerationTerminalVerdictV1;

pub(crate) fn advance_transfer_lifecycle(
    certification: &CertificationAuthorityConfigV1,
    economics_path: &Path,
    terminal: &K1GenerationTerminalVerdictV1,
    transitions: &[TeacherTransition],
    generated_at_unix: u64,
) -> Result<K1TransferLifecycleReportV1, String> {
    let candidate = candidate_from_terminal(terminal, transitions)?;
    let mut report = K1TransferLifecycleReportV1::pending(
        terminal,
        generated_at_unix,
        "external_admission_pending",
    )?;
    report.package_id = candidate.package.package_id.clone();
    let Some(package) = active_package(certification, &candidate)? else {
        report.stage = K1TransferLifecycleStageV1::ExternalAdmissionPending;
        report.reseal()?;
        return Ok(report);
    };
    let (package_candidate_root_sha256, bundle_id_sha256) =
        package_candidate_root(terminal, &package)?;
    report.package_candidate_root_sha256 = Some(package_candidate_root_sha256.clone());
    report.bundle_id_sha256 = Some(bundle_id_sha256.clone());
    report.external_admission_pass = true;

    let completions = durable_package_completions(economics_path, &package.package_id)?;
    let completion = first_post_terminal_completion(&completions, terminal.terminal_at_unix);
    let Some(completion) = completion else {
        report.stage = K1TransferLifecycleStageV1::OrdinaryCpuPending;
        report.blocker = "ordinary_cpu_accept_pending".to_owned();
        report.reseal()?;
        return Ok(report);
    };
    report.ordinary_cpu_receipt_root_sha256 =
        Some(completion.verification_receipt_root_sha256.clone());
    report.ordinary_cpu_completion_root_sha256 = Some(completion.completion_root_sha256.clone());

    let cleanup = restore_cleanup_receipt(
        certification,
        &bundle_id_sha256,
        &package.package_id,
        &package_candidate_root_sha256,
    )?;
    let certification_projection = certify_transfer(
        certification,
        terminal,
        &candidate,
        &package,
        &package_candidate_root_sha256,
        completion,
        cleanup.as_ref(),
    )?;
    report.certification_entry_root_sha256 =
        Some(certification_projection.entry.entry_root_sha256.clone());
    report.certification_ledger_root_sha256 =
        Some(certification_projection.ledger_root_sha256.clone());
    report.law_certificate_root_sha256 = Some(
        certification_projection
            .entry
            .law
            .certificate_root_sha256
            .clone(),
    );
    if certification_projection.entry.false_bad_apply > 0 {
        report.stage = K1TransferLifecycleStageV1::Revoked;
        report.blocker = "runtime_false_bad_apply".to_owned();
        report.reseal()?;
        return Ok(report);
    }

    let Some(cleanup) = cleanup else {
        let response = request_cleanup(
            certification,
            cleanup_request(terminal, &candidate, package_candidate_root_sha256)?,
        )?;
        if response.bundle_id_sha256.as_deref() != Some(bundle_id_sha256.as_str()) {
            return Err("k1_transfer_cleanup_authority_binding_mismatch".to_owned());
        }
        report.stage = K1TransferLifecycleStageV1::CleanupVerifierPending;
        report.blocker = if response.already_complete {
            "cleanup_receipt_visibility_pending"
        } else {
            "cleanup_verifier_pending"
        }
        .to_owned();
        report.reseal()?;
        return Ok(report);
    };
    report.cleanup_receipt_root_sha256 = Some(cleanup.receipt_root_sha256.clone());
    if !certification_projection.entry.k1_unit_eligible {
        return Err("k1_transfer_certification_not_k1_eligible".to_owned());
    }
    let settlement = K1TransferSettlementV1::seal(
        terminal,
        package.package_id.clone(),
        package_candidate_root_sha256,
        certification_projection.entry.entry_root_sha256.clone(),
        certification_projection.ledger_root_sha256.clone(),
        certification_projection
            .entry
            .law
            .certificate_root_sha256
            .clone(),
        generated_at_unix,
    )
    .map_err(str::to_owned)?;
    let scheduler = append_transfer_settlement(certification, settlement.clone())?;
    if scheduler.pending_terminal_transfer.is_some()
        || scheduler
            .latest_transfer_settlement
            .as_ref()
            .is_none_or(|value| value != &settlement)
    {
        return Err("k1_transfer_settlement_restart_parity_failed".to_owned());
    }
    report.settlement_root_sha256 = Some(settlement.settlement_root_sha256);
    report.stage = K1TransferLifecycleStageV1::Settled;
    report.blocker.clear();
    report.reseal()?;
    Ok(report)
}
