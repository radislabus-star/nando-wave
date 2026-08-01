use std::path::PathBuf;

use nando_transition_serving::ServingConfig;
use nando_transition_serving::operator_certification::{
    CertificationAuthorityConfigV1, read_signing_key, run_authority,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.first().and_then(|value| value.to_str()) == Some("derive-public-key") {
        if arguments.len() != 5 || arguments[1] != "--private" || arguments[3] != "--output" {
            return Err(
                "usage: nando-operator-certification-authority derive-public-key --private PATH --output PATH"
                    .to_owned(),
            );
        }
        let signing_key = read_signing_key(std::path::Path::new(&arguments[2]))?;
        let public = signing_key
            .verifying_key()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        std::fs::write(&arguments[4], public)
            .map_err(|error| format!("operator_certification_public_key_write:{error}"))?;
        return Ok(());
    }
    let serving = ServingConfig::from_env()?;
    let config = CertificationAuthorityConfigV1 {
        root: serving.ms4_closed_loop_path,
        cleanup_receipts_path: serving.operator_cleanup_receipts_path,
        anchor_path: serving.operator_certification_anchor_path,
        authority_socket_path: serving.operator_certification_authority_socket_path,
        authority_public_key_path: serving.operator_certification_authority_public_key_path,
        cleanup_public_key_path: serving.operator_cleanup_verifier_public_key_path,
        response_registry_path: serving.response_registry_path,
        runtime_revocations_path: serving.runtime_package_revocations_path,
    };
    let private_key_path = std::env::var_os("NANDO_OPERATOR_CERTIFICATION_AUTHORITY_PRIVATE_KEY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/nando-wave/certification/authority-ed25519.key"));
    run_authority(config, &private_key_path)
}
