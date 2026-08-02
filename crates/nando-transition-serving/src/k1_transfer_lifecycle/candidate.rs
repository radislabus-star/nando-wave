use std::fs;

use nando_operator_learning::{TeacherTransition, multi_source::K1GenerationTerminalVerdictV1};
use nando_response_actor::{
    LiveScalarAdmissionCandidate, ResponsePackage, ResponsePackageState, ResponseRegistry,
    crystallize_multi_source_t1_candidate_v1, response_execution_payload_digest,
};

use crate::operator_certification::CertificationAuthorityConfigV1;
use crate::operator_cleanup::{
    CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1, CleanupAuthorityRequestV1, canonical_bundle_id,
    k1_package_candidate_root, sha256,
};

pub(crate) fn candidate_from_terminal(
    terminal: &K1GenerationTerminalVerdictV1,
    transitions: &[TeacherTransition],
) -> Result<LiveScalarAdmissionCandidate, String> {
    terminal.validate().map_err(str::to_owned)?;
    let identification = terminal
        .transfer_identification
        .as_ref()
        .ok_or_else(|| "k1_transfer_identification_missing".to_owned())?;
    let candidate = crystallize_multi_source_t1_candidate_v1(identification, transitions)?;
    if candidate
        .multi_source_identification
        .as_ref()
        .is_none_or(|value| value.report_root_sha256 != identification.report_root_sha256)
    {
        return Err("k1_transfer_candidate_identification_mismatch".to_owned());
    }
    Ok(candidate)
}

pub(super) fn active_package(
    config: &CertificationAuthorityConfigV1,
    candidate: &LiveScalarAdmissionCandidate,
) -> Result<Option<ResponsePackage>, String> {
    let bytes = match fs::read(&config.response_registry_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("k1_transfer_registry_read:{error}")),
    };
    let registry: ResponseRegistry = serde_json::from_slice(&bytes)
        .map_err(|error| format!("k1_transfer_registry_decode:{error}"))?;
    registry.validate().map_err(str::to_owned)?;
    let Some(package) = registry.packages.iter().find(|package| {
        package.package_id == candidate.package.package_id
            && package.state == ResponsePackageState::Active
    }) else {
        return Ok(None);
    };
    let submitted_payload =
        response_execution_payload_digest(&candidate.package).map_err(str::to_owned)?;
    let active_payload = response_execution_payload_digest(package).map_err(str::to_owned)?;
    if submitted_payload != active_payload {
        return Err("k1_transfer_active_package_rebound".to_owned());
    }
    Ok(Some(package.clone()))
}

pub(super) fn package_candidate_root(
    terminal: &K1GenerationTerminalVerdictV1,
    package: &ResponsePackage,
) -> Result<(String, String), String> {
    let identification = terminal
        .transfer_identification
        .as_ref()
        .ok_or_else(|| "k1_transfer_identification_missing".to_owned())?;
    let bundle = package
        .crystallized_operator
        .as_ref()
        .ok_or_else(|| "k1_transfer_bundle_missing".to_owned())?;
    let bundle_id_sha256 = canonical_bundle_id(bundle)?;
    let execution_payload_sha256 =
        response_execution_payload_digest(package).map_err(str::to_owned)?;
    let candidate_root = k1_package_candidate_root(
        &terminal.verdict_root_sha256,
        &identification.report_root_sha256,
        &package.package_id,
        &bundle_id_sha256,
        &execution_payload_sha256,
    )?;
    Ok((candidate_root, bundle_id_sha256))
}

pub(super) fn cleanup_request(
    terminal: &K1GenerationTerminalVerdictV1,
    candidate: &LiveScalarAdmissionCandidate,
    package_candidate_root_sha256: String,
) -> Result<CleanupAuthorityRequestV1, String> {
    let identification = terminal
        .transfer_identification
        .as_ref()
        .ok_or_else(|| "k1_transfer_identification_missing".to_owned())?;
    let parity = candidate
        .future
        .iter()
        .filter_map(|row| row.runtime_parity_case.as_ref())
        .next()
        .ok_or_else(|| "k1_transfer_cleanup_challenge_missing".to_owned())?;
    let request = CleanupAuthorityRequestV1 {
        schema: CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        terminal_verdict_root_sha256: terminal.verdict_root_sha256.clone(),
        identification_report_root_sha256: identification.report_root_sha256.clone(),
        package_id: candidate.package.package_id.clone(),
        package_candidate_root_sha256,
        source_receipt_root_sha256: terminal.verdict_root_sha256.clone(),
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response_sha256: sha256(parity.expected_response.as_bytes()),
    };
    request.validate()?;
    Ok(request)
}
