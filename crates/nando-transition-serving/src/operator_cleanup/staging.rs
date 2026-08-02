use std::fs;
use std::path::{Path, PathBuf};

use nando_operator_kernel::canonical_json_bytes;
use nando_response_actor::{
    ResponsePackageState, ResponseRegistry, response_execution_payload_digest,
    response_registry_digest,
};

use super::protocol::{
    BUNDLE_INPUT_SCHEMA_V1, BundleInputV1, CHALLENGE_SCHEMA_V1, CleanupAuthorityRequestV1,
    CleanupChallengeV1, canonical_bundle_id, k1_package_candidate_root, sha256,
    validate_bundle_input, validate_challenge, write_once,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StagedCleanupV1 {
    pub bundle_id_sha256: String,
    pub directory: PathBuf,
}

pub(super) fn stage_cleanup_request(
    registry_path: &Path,
    staging_root: &Path,
    request: &CleanupAuthorityRequestV1,
) -> Result<StagedCleanupV1, String> {
    request.validate()?;
    let registry: ResponseRegistry = serde_json::from_slice(
        &fs::read(registry_path).map_err(|error| format!("cleanup_stage_registry_read:{error}"))?,
    )
    .map_err(|error| format!("cleanup_stage_registry_decode:{error}"))?;
    registry.validate().map_err(str::to_owned)?;
    let package = registry
        .packages
        .iter()
        .find(|package| {
            package.package_id == request.package_id
                && package.state == ResponsePackageState::Active
        })
        .ok_or_else(|| "cleanup_stage_active_package_missing".to_owned())?;
    let restart_bundle = package
        .crystallized_operator
        .clone()
        .ok_or_else(|| "cleanup_stage_bundle_missing".to_owned())?;
    if !restart_bundle.has_canonical_bundle_v4() {
        return Err("cleanup_stage_bundle_v4_required".to_owned());
    }
    let bundle_id_sha256 = canonical_bundle_id(&restart_bundle)?;
    let execution_payload_sha256 =
        response_execution_payload_digest(package).map_err(str::to_owned)?;
    let expected_candidate_root = k1_package_candidate_root(
        &request.terminal_verdict_root_sha256,
        &request.identification_report_root_sha256,
        &request.package_id,
        &bundle_id_sha256,
        &execution_payload_sha256,
    )?;
    if request.package_candidate_root_sha256 != expected_candidate_root {
        return Err("cleanup_stage_candidate_binding_mismatch".to_owned());
    }

    let restored = restart_bundle
        .restore_verified()
        .map_err(|_| "cleanup_stage_bundle_restore_failed".to_owned())?;
    let bound = restored
        .bind_pre_action(&request.request_text, &request.provider_payload)
        .map_err(|_| "cleanup_stage_challenge_binding_failed".to_owned())?;
    let response = bound
        .execute_verified()
        .map_err(|_| "cleanup_stage_challenge_execution_failed".to_owned())?;
    if sha256(response.as_bytes()) != request.expected_response_sha256 {
        return Err("cleanup_stage_challenge_response_mismatch".to_owned());
    }

    let challenge = CleanupChallengeV1 {
        schema: CHALLENGE_SCHEMA_V1.to_owned(),
        bundle_id_sha256: bundle_id_sha256.clone(),
        package_id: request.package_id.clone(),
        source_receipt_root_sha256: request.source_receipt_root_sha256.clone(),
        request_text: request.request_text.clone(),
        provider_payload: request.provider_payload.clone(),
        expected_response_sha256: request.expected_response_sha256.clone(),
    };
    validate_challenge(&challenge)?;
    let input = BundleInputV1 {
        schema: BUNDLE_INPUT_SCHEMA_V1.to_owned(),
        bundle_id_sha256: bundle_id_sha256.clone(),
        package_id: request.package_id.clone(),
        candidate_root_sha256: request.package_candidate_root_sha256.clone(),
        active_registry_root_sha256: response_registry_digest(&registry).map_err(str::to_owned)?,
        execution_payload_sha256,
        restart_bundle,
    };
    validate_bundle_input(&input)?;
    let directory = staging_root.join(&bundle_id_sha256);
    fs::create_dir_all(&directory).map_err(|error| format!("cleanup_stage_output_dir:{error}"))?;
    persist_exact(
        &directory.join("challenge.json"),
        &canonical_json_bytes(&challenge).map_err(str::to_owned)?,
    )?;
    persist_exact(
        &directory.join("bundle.json"),
        &canonical_json_bytes(&input).map_err(str::to_owned)?,
    )?;
    Ok(StagedCleanupV1 {
        bundle_id_sha256,
        directory,
    })
}

fn persist_exact(path: &Path, bytes: &[u8]) -> Result<(), String> {
    match fs::read(path) {
        Ok(existing) if existing == bytes => Ok(()),
        Ok(_) => Err("cleanup_stage_rebind_forbidden".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => write_once(path, bytes),
        Err(error) => Err(format!("cleanup_stage_existing_read:{error}")),
    }
}
