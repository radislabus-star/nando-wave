use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_response_actor::VerifiedOperatorRestartBundle;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1: &str = "nando.cleanup-authority-request.v1";
pub const CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1: &str = "nando.cleanup-authority-response.v1";
pub const BUNDLE_INPUT_SCHEMA_V1: &str = "nando.cleanup-verifier-bundle-input.v1";
pub const CHALLENGE_SCHEMA_V1: &str = "nando.cleanup-verifier-challenge.v1";
const K1_PACKAGE_CANDIDATE_SCHEMA_V1: &str = "nando.k1-package-candidate.v1";
const MAX_CLEANUP_CHALLENGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupAuthorityRequestV1 {
    pub schema: String,
    pub terminal_verdict_root_sha256: String,
    pub identification_report_root_sha256: String,
    pub package_id: String,
    pub package_candidate_root_sha256: String,
    pub source_receipt_root_sha256: String,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupAuthorityResponseV1 {
    pub schema: String,
    pub bundle_id_sha256: Option<String>,
    pub already_complete: bool,
    pub error: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BundleInputV1 {
    pub schema: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub candidate_root_sha256: String,
    pub active_registry_root_sha256: String,
    pub execution_payload_sha256: String,
    pub restart_bundle: VerifiedOperatorRestartBundle,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupChallengeV1 {
    pub schema: String,
    pub bundle_id_sha256: String,
    pub package_id: String,
    pub source_receipt_root_sha256: String,
    pub request_text: String,
    pub provider_payload: Value,
    pub expected_response_sha256: String,
}

impl CleanupAuthorityRequestV1 {
    pub fn validate(&self) -> Result<(), String> {
        let payload_bytes = serde_json::to_vec(&self.provider_payload)
            .map_err(|error| format!("cleanup_authority_payload_encode:{error}"))?;
        let roots = [
            self.terminal_verdict_root_sha256.as_str(),
            self.identification_report_root_sha256.as_str(),
            self.package_candidate_root_sha256.as_str(),
            self.source_receipt_root_sha256.as_str(),
            self.expected_response_sha256.as_str(),
        ];
        if self.schema != CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1
            || self.package_id.is_empty()
            || self.request_text.len() > MAX_CLEANUP_CHALLENGE_BYTES
            || payload_bytes.len() > MAX_CLEANUP_CHALLENGE_BYTES
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.source_receipt_root_sha256 != self.terminal_verdict_root_sha256
        {
            return Err("cleanup_authority_request_invalid".to_owned());
        }
        Ok(())
    }
}

pub fn validate_bundle_input(input: &BundleInputV1) -> Result<(), String> {
    if input.schema != BUNDLE_INPUT_SCHEMA_V1
        || input.package_id.is_empty()
        || !valid_nonzero_sha256(&input.bundle_id_sha256)
        || !valid_nonzero_sha256(&input.candidate_root_sha256)
        || !valid_nonzero_sha256(&input.active_registry_root_sha256)
        || !valid_nonzero_sha256(&input.execution_payload_sha256)
        || !input.restart_bundle.has_canonical_bundle_v4()
        || canonical_bundle_id(&input.restart_bundle)? != input.bundle_id_sha256
    {
        return Err("cleanup_verifier_bundle_input_invalid".to_owned());
    }
    Ok(())
}

pub fn validate_challenge(challenge: &CleanupChallengeV1) -> Result<(), String> {
    if challenge.schema != CHALLENGE_SCHEMA_V1
        || challenge.package_id.is_empty()
        || challenge.request_text.len() > MAX_CLEANUP_CHALLENGE_BYTES
        || serde_json::to_vec(&challenge.provider_payload)
            .map_or(true, |payload| payload.len() > MAX_CLEANUP_CHALLENGE_BYTES)
        || !valid_nonzero_sha256(&challenge.bundle_id_sha256)
        || !valid_nonzero_sha256(&challenge.source_receipt_root_sha256)
        || !valid_nonzero_sha256(&challenge.expected_response_sha256)
    {
        return Err("cleanup_verifier_challenge_invalid".to_owned());
    }
    Ok(())
}

pub fn k1_package_candidate_root(
    terminal_verdict_root_sha256: &str,
    identification_report_root_sha256: &str,
    package_id: &str,
    bundle_id_sha256: &str,
    execution_payload_sha256: &str,
) -> Result<String, String> {
    if package_id.is_empty()
        || ![
            terminal_verdict_root_sha256,
            identification_report_root_sha256,
            bundle_id_sha256,
            execution_payload_sha256,
        ]
        .into_iter()
        .all(valid_nonzero_sha256)
    {
        return Err("k1_package_candidate_binding_invalid".to_owned());
    }
    canonical_json_sha256(&(
        K1_PACKAGE_CANDIDATE_SCHEMA_V1,
        terminal_verdict_root_sha256,
        identification_report_root_sha256,
        package_id,
        bundle_id_sha256,
        execution_payload_sha256,
    ))
    .map_err(str::to_owned)
}

pub fn canonical_bundle_id(bundle: &VerifiedOperatorRestartBundle) -> Result<String, String> {
    bundle
        .canonical_bundle_id()
        .map(|id| id.iter().map(|byte| format!("{byte:02x}")).collect())
        .ok_or_else(|| "cleanup_verifier_bundle_id_missing".to_owned())
}

pub fn challenge_root(challenge: &CleanupChallengeV1) -> Result<String, String> {
    canonical_json_sha256(challenge).map_err(str::to_owned)
}

pub fn read_canonical_json<T>(path: &Path, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let bytes = fs::read(path).map_err(|error| format!("{label}_read:{error}"))?;
    let value: T =
        serde_json::from_slice(&bytes).map_err(|error| format!("{label}_decode:{error}"))?;
    if nando_operator_kernel::canonical_json_bytes(&value).map_err(str::to_owned)? != bytes {
        return Err(format!("{label}_noncanonical"));
    }
    Ok(value)
}

pub fn write_once(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "cleanup_verifier_output_parent_missing".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cleanup_verifier_output_parent:{error}"))?;
    if let Ok(existing) = fs::read(path) {
        return if existing == bytes {
            Ok(())
        } else {
            Err("cleanup_verifier_output_rebind".to_owned())
        };
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("cleanup_verifier_output_create:{error}"))?;
    let result = (|| {
        file.write_all(bytes)
            .map_err(|error| format!("cleanup_verifier_output_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("cleanup_verifier_output_sync:{error}"))?;
        fs::hard_link(&temporary, path)
            .map_err(|error| format!("cleanup_verifier_output_link:{error}"))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("cleanup_verifier_output_dir_sync:{error}"))
    })();
    let _ = fs::remove_file(&temporary);
    result
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
