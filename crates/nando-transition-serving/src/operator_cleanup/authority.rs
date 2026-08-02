use std::env;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::k1_natural_scheduler::restore_projection;
use crate::operator_certification::{CertificationAuthorityConfigV1, restore_cleanup_receipt};

use super::protocol::{
    CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1, CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1,
    CleanupAuthorityRequestV1, CleanupAuthorityResponseV1,
};
use super::staging::stage_cleanup_request;

const DEFAULT_STAGING_ROOT: &str = "/var/lib/nando-wave/cleanup-verifier-staging";
const DEFAULT_START_COMMAND: &str = "/usr/bin/systemctl";
const DEFAULT_UNIT_TEMPLATE: &str = "nando-operator-cleanup-verifier@{}.service";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupAuthorityRuntimeConfigV1 {
    pub staging_root: PathBuf,
    pub start_command: PathBuf,
    pub unit_template: String,
}

impl CleanupAuthorityRuntimeConfigV1 {
    pub fn from_env() -> Result<Self, String> {
        let config = Self {
            staging_root: env::var_os("NANDO_OPERATOR_CLEANUP_STAGING")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGING_ROOT)),
            start_command: env::var_os("NANDO_OPERATOR_CLEANUP_START_COMMAND")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_START_COMMAND)),
            unit_template: env::var("NANDO_OPERATOR_CLEANUP_UNIT_TEMPLATE")
                .unwrap_or_else(|_| DEFAULT_UNIT_TEMPLATE.to_owned()),
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.staging_root.as_os_str().is_empty()
            || self.start_command.as_os_str().is_empty()
            || self.unit_template.matches("{}").count() != 1
            || !self
                .unit_template
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"@._-{}".contains(&byte))
        {
            return Err("cleanup_authority_runtime_config_invalid".to_owned());
        }
        Ok(())
    }
}

pub(crate) fn request_cleanup(
    certification: &CertificationAuthorityConfigV1,
    request: CleanupAuthorityRequestV1,
) -> Result<CleanupAuthorityResponseV1, String> {
    request.validate()?;
    send_authority_request(certification, &request)
}

pub(crate) fn handle_authority_line(
    certification: &CertificationAuthorityConfigV1,
    runtime: &CleanupAuthorityRuntimeConfigV1,
    line: &str,
) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("schema").and_then(Value::as_str) != Some(CLEANUP_AUTHORITY_REQUEST_SCHEMA_V1) {
        return None;
    }
    let result = serde_json::from_value::<CleanupAuthorityRequestV1>(value)
        .map_err(|error| format!("cleanup_authority_request_decode:{error}"))
        .and_then(|request| handle_request(certification, runtime, request));
    let response = result.unwrap_or_else(|error| CleanupAuthorityResponseV1 {
        schema: CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
        bundle_id_sha256: None,
        already_complete: false,
        error,
    });
    Some(serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(
            "{{\"schema\":\"{CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1}\",\"bundle_id_sha256\":null,\"already_complete\":false,\"error\":\"cleanup_authority_response_encode:{error}\"}}"
        )
    }))
}

fn handle_request(
    certification: &CertificationAuthorityConfigV1,
    runtime: &CleanupAuthorityRuntimeConfigV1,
    request: CleanupAuthorityRequestV1,
) -> Result<CleanupAuthorityResponseV1, String> {
    request.validate()?;
    let projection = restore_projection(certification)?;
    let terminal = projection
        .pending_terminal_transfer
        .as_ref()
        .ok_or_else(|| "cleanup_authority_pending_transfer_missing".to_owned())?;
    let identification = terminal
        .transfer_identification
        .as_ref()
        .ok_or_else(|| "cleanup_authority_identification_missing".to_owned())?;
    if request.terminal_verdict_root_sha256 != terminal.verdict_root_sha256
        || request.identification_report_root_sha256 != identification.report_root_sha256
    {
        return Err("cleanup_authority_scheduler_binding_mismatch".to_owned());
    }
    let staged = stage_cleanup_request(
        &certification.response_registry_path,
        &runtime.staging_root,
        &request,
    )?;
    if staged.directory != runtime.staging_root.join(&staged.bundle_id_sha256) {
        return Err("cleanup_authority_staging_binding_mismatch".to_owned());
    }
    if restore_cleanup_receipt(
        certification,
        &staged.bundle_id_sha256,
        &request.package_id,
        &request.package_candidate_root_sha256,
    )?
    .is_some()
    {
        return Ok(CleanupAuthorityResponseV1 {
            schema: CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
            bundle_id_sha256: Some(staged.bundle_id_sha256),
            already_complete: true,
            error: String::new(),
        });
    }
    launch_verifier(runtime, &staged.bundle_id_sha256)?;
    Ok(CleanupAuthorityResponseV1 {
        schema: CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1.to_owned(),
        bundle_id_sha256: Some(staged.bundle_id_sha256),
        already_complete: false,
        error: String::new(),
    })
}

fn launch_verifier(
    runtime: &CleanupAuthorityRuntimeConfigV1,
    bundle_id_sha256: &str,
) -> Result<(), String> {
    runtime.validate()?;
    if bundle_id_sha256.len() != 64
        || !bundle_id_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("cleanup_authority_bundle_id_invalid".to_owned());
    }
    let unit = runtime.unit_template.replace("{}", bundle_id_sha256);
    let status = Command::new(&runtime.start_command)
        .arg("start")
        .arg("--no-block")
        .arg(unit)
        .status()
        .map_err(|error| format!("cleanup_authority_start:{error}"))?;
    if !status.success() {
        return Err(format!(
            "cleanup_authority_start_status:{}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn send_authority_request<T: Serialize>(
    config: &CertificationAuthorityConfigV1,
    request: &T,
) -> Result<CleanupAuthorityResponseV1, String> {
    #[cfg(not(unix))]
    {
        let _ = (config, request);
        return Err("cleanup_authority_requires_unix".to_owned());
    }
    #[cfg(unix)]
    {
        let mut stream = UnixStream::connect(&config.authority_socket_path)
            .map_err(|error| format!("cleanup_authority_connect:{error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("cleanup_authority_read_timeout:{error}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|error| format!("cleanup_authority_write_timeout:{error}"))?;
        serde_json::to_writer(&mut stream, request)
            .map_err(|error| format!("cleanup_authority_encode:{error}"))?;
        stream
            .write_all(b"\n")
            .map_err(|error| format!("cleanup_authority_write:{error}"))?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|error| format!("cleanup_authority_shutdown:{error}"))?;
        let response: CleanupAuthorityResponseV1 = serde_json::from_reader(&mut stream)
            .map_err(|error| format!("cleanup_authority_decode:{error}"))?;
        if response.schema != CLEANUP_AUTHORITY_RESPONSE_SCHEMA_V1 || !response.error.is_empty() {
            return Err(if response.error.is_empty() {
                "cleanup_authority_response_invalid".to_owned()
            } else {
                response.error
            });
        }
        Ok(response)
    }
}
