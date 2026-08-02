use nando_operator_admission::{
    ExactMemoryCleanupReceiptV1, ExecutionCertificateStatusV1, ExecutionCertificateV1,
    LawCertificateStatusV1, LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
    OperatorCertificationEntryV1, OperatorMechanismClassV1,
};
use nando_operator_learning::multi_source::K1GenerationTerminalVerdictV1;
use nando_response_actor::{LiveScalarAdmissionCandidate, ResponsePackage};

use crate::live_economics::PackageCpuCompletionReceiptV1;
use crate::operator_certification::{
    CertificationAuthorityConfigV1, CertificationProjectionV1, append_entry,
    canonical_role_topology_id, durable_false_bad_apply_evidence,
};
use crate::operator_cleanup::canonical_bundle_id;

#[allow(clippy::too_many_arguments)]
pub(super) fn certify_transfer(
    config: &CertificationAuthorityConfigV1,
    terminal: &K1GenerationTerminalVerdictV1,
    candidate: &LiveScalarAdmissionCandidate,
    package: &ResponsePackage,
    package_candidate_root_sha256: &str,
    completion: &PackageCpuCompletionReceiptV1,
    cleanup: Option<&ExactMemoryCleanupReceiptV1>,
) -> Result<CertificationProjectionV1, String> {
    let identification = terminal
        .transfer_identification
        .as_ref()
        .ok_or_else(|| "k1_transfer_identification_missing".to_owned())?;
    let bundle = package
        .crystallized_operator
        .as_ref()
        .ok_or_else(|| "k1_transfer_bundle_missing".to_owned())?;
    let bundle_id = canonical_bundle_id(bundle)?;
    let law_id = bundle
        .canonical_law_id_sha256()
        .map_err(|error| format!("k1_transfer_law_id:{error:?}"))?
        .ok_or_else(|| "k1_transfer_law_id_missing".to_owned())?;
    let role_topology_id = canonical_role_topology_id(package)?;
    let (false_bad_apply, live_safety_evidence) =
        durable_false_bad_apply_evidence(config, &package.package_id)?;

    let mut execution_evidence = vec![
        terminal.verdict_root_sha256.clone(),
        identification.report_root_sha256.clone(),
        package_candidate_root_sha256.to_owned(),
        completion.verification_receipt_root_sha256.clone(),
        completion.completion_root_sha256.clone(),
    ];
    execution_evidence.extend(live_safety_evidence);
    let revoked = false_bad_apply > 0;
    let execution = ExecutionCertificateV1::seal(
        &bundle_id,
        &package.package_id,
        if revoked {
            ExecutionCertificateStatusV1::Revoked
        } else {
            ExecutionCertificateStatusV1::Pass
        },
        execution_evidence,
        if revoked {
            "runtime_false_bad_apply"
        } else {
            ""
        },
    )
    .map_err(str::to_owned)?;

    let adaptive_proof_root = package
        .proof
        .adaptive_identification
        .as_ref()
        .ok_or_else(|| "k1_transfer_adaptive_proof_missing".to_owned())?
        .proof_root_sha256()
        .to_owned();
    let mut law_evidence = vec![
        terminal.verdict_root_sha256.clone(),
        identification.report_root_sha256.clone(),
        package_candidate_root_sha256.to_owned(),
        adaptive_proof_root,
        candidate.support_root_sha256.clone(),
        candidate.future_evidence_root_sha256.clone(),
        candidate.future_lineage_root_sha256.clone(),
        candidate.executable_parity_seal_sha256.clone(),
    ];
    law_evidence.extend(
        cleanup
            .iter()
            .map(|receipt| receipt.receipt_root_sha256.clone()),
    );
    let law = LawCertificateV1::seal(
        &bundle_id,
        &package.package_id,
        if cleanup.is_some() {
            LawCertificateStatusV1::Pass
        } else {
            LawCertificateStatusV1::Partial
        },
        law_evidence,
        cleanup.map(|receipt| receipt.receipt_root_sha256.clone()),
        if cleanup.is_some() {
            ""
        } else {
            "exact_memory_cleanup_receipt_missing"
        },
    )
    .map_err(str::to_owned)?;

    let mechanism = MechanismCertificateV1::seal(
        &bundle_id,
        &package.package_id,
        MechanismCertificateStatusV1::NotEvaluated,
        OperatorMechanismClassV1::Unresolved,
        vec![
            terminal.verdict_root_sha256.clone(),
            identification.report_root_sha256.clone(),
        ],
        "independent_wave_mechanism_not_evaluated",
    )
    .map_err(str::to_owned)?;
    let entry = OperatorCertificationEntryV1::seal(
        &bundle_id,
        &package.package_id,
        &law_id,
        &role_topology_id,
        execution,
        law,
        mechanism,
        false_bad_apply,
    )
    .map_err(str::to_owned)?;
    append_entry(config, entry)
}
