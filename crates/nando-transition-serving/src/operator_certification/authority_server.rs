use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

use super::*;

pub fn run_authority(
    config: CertificationAuthorityConfigV1,
    cleanup_runtime: crate::operator_cleanup::CleanupAuthorityRuntimeConfigV1,
    signing_key_path: &Path,
) -> Result<(), String> {
    let signing_key = read_signing_key(signing_key_path)?;
    let expected_public_key = read_verifying_key(&config.authority_public_key_path)?;
    if signing_key.verifying_key() != expected_public_key {
        return Err("operator_certification_authority_key_mismatch".to_owned());
    }
    if let Some(parent) = config.authority_socket_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("operator_certification_socket_parent:{error}"))?;
    }
    if config.authority_socket_path.exists() {
        fs::remove_file(&config.authority_socket_path)
            .map_err(|error| format!("operator_certification_socket_remove:{error}"))?;
    }
    let listener = UnixListener::bind(&config.authority_socket_path)
        .map_err(|error| format!("operator_certification_socket_bind:{error}"))?;
    fs::set_permissions(
        &config.authority_socket_path,
        fs::Permissions::from_mode(0o660),
    )
    .map_err(|error| format!("operator_certification_socket_permissions:{error}"))?;
    recover_anchor(&config, &signing_key)?;
    crate::k1_natural_scheduler::recover_authority(&config, &signing_key)?;

    for connection in listener.incoming() {
        match connection {
            Ok(mut stream) => {
                if let Err(error) =
                    handle_connection(&config, &cleanup_runtime, &signing_key, &mut stream)
                {
                    eprintln!("nando-operator-certification-connection: {error}");
                }
            }
            Err(error) => eprintln!("nando-operator-certification-accept: {error}"),
        }
    }
    Ok(())
}

fn handle_connection(
    config: &CertificationAuthorityConfigV1,
    cleanup_runtime: &crate::operator_cleanup::CleanupAuthorityRuntimeConfigV1,
    signing_key: &SigningKey,
    stream: &mut UnixStream,
) -> Result<(), String> {
    let line = transport::read_authority_line(stream)?;
    if let Some(response) =
        crate::operator_cleanup::handle_authority_line(config, cleanup_runtime, &line)
    {
        return stream
            .write_all(response.as_bytes())
            .map_err(|error| format!("cleanup_authority_response:{error}"));
    }
    if let Some(response) =
        crate::k1_natural_scheduler::handle_authority_line(config, signing_key, &line)
    {
        return stream
            .write_all(response.as_bytes())
            .map_err(|error| format!("k1_scheduler_authority_response:{error}"));
    }
    let payload = match handle_legacy_request(config, signing_key, &line) {
        Ok(projection) => AuthorityResponseV1 {
            schema: AUTHORITY_RESPONSE_SCHEMA.to_owned(),
            projection: Some(projection),
            error: String::new(),
        },
        Err(error) => AuthorityResponseV1 {
            schema: AUTHORITY_RESPONSE_SCHEMA.to_owned(),
            projection: None,
            error,
        },
    };
    serde_json::to_writer(stream, &payload)
        .map_err(|error| format!("operator_certification_authority_response:{error}"))
}

fn handle_legacy_request(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    line: &str,
) -> Result<CertificationProjectionV1, String> {
    let request: AuthorityRequestV1 = serde_json::from_str(line)
        .map_err(|error| format!("operator_certification_authority_request_decode:{error}"))?;
    if request.schema != AUTHORITY_REQUEST_SCHEMA {
        return Err("operator_certification_authority_request_schema_invalid".to_owned());
    }
    append_authoritative(config, signing_key, request.entry)
}
