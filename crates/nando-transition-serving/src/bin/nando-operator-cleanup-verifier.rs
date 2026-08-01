use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use nando_operator_admission::ExactMemoryCleanupReceiptV1;
use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use nando_response_actor::{
    Ms4ExternalAdmissionCandidateV1, ResponseRegistry, VerifiedOperatorRestartBundle,
    response_execution_payload_digest, response_registry_digest,
};
use nando_transition_serving::operator_certification::read_signing_key;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const BUNDLE_INPUT_SCHEMA: &str = "nando.cleanup-verifier-bundle-input.v1";
const CHALLENGE_SCHEMA: &str = "nando.cleanup-verifier-challenge.v1";
const VERIFIER_TCB_SCHEMA: &str = "nando.cleanup-verifier-tcb.v1";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BundleInputV1 {
    schema: String,
    bundle_id_sha256: String,
    package_id: String,
    candidate_root_sha256: String,
    active_registry_root_sha256: String,
    execution_payload_sha256: String,
    restart_bundle: VerifiedOperatorRestartBundle,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupChallengeV1 {
    schema: String,
    bundle_id_sha256: String,
    package_id: String,
    source_receipt_root_sha256: String,
    request_text: String,
    provider_payload: Value,
    expected_response_sha256: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match arguments.first().and_then(|value| value.to_str()) {
        Some("stage-candidate") if arguments.len() == 9 => stage_candidate(
            Path::new(&arguments[2]),
            arguments[4]
                .to_str()
                .ok_or_else(|| "cleanup_stage_package_id_utf8".to_owned())?,
            Path::new(&arguments[6]),
            Path::new(&arguments[8]),
        ),
        Some("stage") if arguments.len() == 11 => stage(
            Path::new(&arguments[2]),
            arguments[4]
                .to_str()
                .ok_or_else(|| "cleanup_stage_package_id_utf8".to_owned())?,
            arguments[6]
                .to_str()
                .ok_or_else(|| "cleanup_stage_candidate_root_utf8".to_owned())?,
            Path::new(&arguments[8]),
            Path::new(&arguments[10]),
        ),
        Some("verify") if arguments.len() == 9 => verify(
            Path::new(&arguments[2]),
            Path::new(&arguments[4]),
            Path::new(&arguments[6]),
            Path::new(&arguments[8]),
        ),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: nando-operator-cleanup-verifier stage-candidate --registry PATH --package-id ID --candidate PATH --output-dir DIR | stage --registry PATH --package-id ID --candidate-root SHA256 --challenge PATH --output PATH | verify --bundle PATH --challenge PATH --private-key PATH --output PATH".to_owned()
}

fn stage_candidate(
    registry_path: &Path,
    package_id: &str,
    candidate_path: &Path,
    output_dir: &Path,
) -> Result<(), String> {
    let candidate = Ms4ExternalAdmissionCandidateV1::from_canonical_bytes(
        &fs::read(candidate_path)
            .map_err(|error| format!("cleanup_stage_candidate_read:{error}"))?,
    )
    .map_err(str::to_owned)?;
    let package = candidate.admitted_package().map_err(str::to_owned)?;
    if package.package_id != package_id {
        return Err("cleanup_stage_candidate_package_mismatch".to_owned());
    }
    let parity = candidate.future_runtime_parity_case();
    let challenge = CleanupChallengeV1 {
        schema: CHALLENGE_SCHEMA.to_owned(),
        bundle_id_sha256: candidate.canonical_bundle_id_sha256().to_owned(),
        package_id: package_id.to_owned(),
        source_receipt_root_sha256: candidate.future_runtime_receipt_root_sha256().to_owned(),
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response_sha256: candidate.future_actor_response_sha256().to_owned(),
    };
    fs::create_dir_all(output_dir).map_err(|error| format!("cleanup_stage_output_dir:{error}"))?;
    let challenge_path = output_dir.join("challenge.json");
    write_atomic(
        &challenge_path,
        &canonical_json_bytes(&challenge).map_err(str::to_owned)?,
    )?;
    stage(
        registry_path,
        package_id,
        candidate.candidate_root_sha256(),
        &challenge_path,
        &output_dir.join("bundle.json"),
    )
}

fn stage(
    registry_path: &Path,
    package_id: &str,
    candidate_root_sha256: &str,
    challenge_path: &Path,
    output_path: &Path,
) -> Result<(), String> {
    if !valid_nonzero_sha256(candidate_root_sha256) {
        return Err("cleanup_stage_candidate_root_invalid".to_owned());
    }
    let registry: ResponseRegistry = serde_json::from_slice(
        &fs::read(registry_path).map_err(|error| format!("cleanup_stage_registry_read:{error}"))?,
    )
    .map_err(|error| format!("cleanup_stage_registry_decode:{error}"))?;
    registry.validate().map_err(str::to_owned)?;
    let active_registry_root_sha256 = response_registry_digest(&registry).map_err(str::to_owned)?;
    let package = registry
        .packages
        .iter()
        .find(|package| package.package_id == package_id)
        .ok_or_else(|| "cleanup_stage_package_missing".to_owned())?;
    let restart_bundle = package
        .crystallized_operator
        .clone()
        .ok_or_else(|| "cleanup_stage_bundle_missing".to_owned())?;
    let execution_payload_sha256 =
        response_execution_payload_digest(package).map_err(str::to_owned)?;
    if !restart_bundle.has_canonical_bundle_v4() {
        return Err("cleanup_stage_bundle_v4_required".to_owned());
    }
    let bundle_id_sha256 = bundle_id(&restart_bundle)?;
    let challenge: CleanupChallengeV1 = read_canonical_json(challenge_path, "cleanup_challenge")?;
    validate_challenge(&challenge)?;
    if challenge.bundle_id_sha256 != bundle_id_sha256 || challenge.package_id != package_id {
        return Err("cleanup_stage_challenge_binding_mismatch".to_owned());
    }
    let input = BundleInputV1 {
        schema: BUNDLE_INPUT_SCHEMA.to_owned(),
        bundle_id_sha256,
        package_id: package_id.to_owned(),
        candidate_root_sha256: candidate_root_sha256.to_owned(),
        active_registry_root_sha256,
        execution_payload_sha256,
        restart_bundle,
    };
    write_atomic(
        output_path,
        &canonical_json_bytes(&input).map_err(str::to_owned)?,
    )
}

fn verify(
    bundle_path: &Path,
    challenge_path: &Path,
    private_key_path: &Path,
    output_path: &Path,
) -> Result<(), String> {
    let bundle: BundleInputV1 = read_canonical_json(bundle_path, "cleanup_bundle")?;
    let challenge: CleanupChallengeV1 = read_canonical_json(challenge_path, "cleanup_challenge")?;
    validate_bundle_input(&bundle)?;
    validate_challenge(&challenge)?;
    if bundle.bundle_id_sha256 != challenge.bundle_id_sha256
        || bundle.package_id != challenge.package_id
    {
        return Err("cleanup_verifier_challenge_binding_mismatch".to_owned());
    }
    let restored = bundle
        .restart_bundle
        .restore_verified()
        .map_err(|_| "cleanup_verifier_bundle_restore_failed".to_owned())?;
    let bound = restored
        .bind_pre_action(&challenge.request_text, &challenge.provider_payload)
        .map_err(|_| "cleanup_verifier_pre_action_binding_failed".to_owned())?;
    let response = bound
        .execute_verified()
        .map_err(|_| "cleanup_verifier_execution_failed".to_owned())?;
    let response_sha256 = sha256(response.as_bytes());
    if response_sha256 != challenge.expected_response_sha256 {
        return Err("cleanup_verifier_expected_response_mismatch".to_owned());
    }
    let challenge_root_sha256 = canonical_json_sha256(&challenge).map_err(str::to_owned)?;
    let standalone_restart_root_sha256 = canonical_json_sha256(&(
        "nando.cleanup-standalone-restart.v1",
        bundle.bundle_id_sha256.as_str(),
        sha256(bundle.restart_bundle.page_bytes()),
        sha256(bundle.restart_bundle.registry_cbor()),
        restored.actor_sha256(),
        restored.verifier_sha256(),
        challenge_root_sha256.as_str(),
        response_sha256.as_str(),
    ))
    .map_err(str::to_owned)?;
    let executable =
        std::env::current_exe().map_err(|error| format!("cleanup_verifier_current_exe:{error}"))?;
    let verifier_tcb_root_sha256 = canonical_json_sha256(&(
        VERIFIER_TCB_SCHEMA,
        sha256(
            &fs::read(executable)
                .map_err(|error| format!("cleanup_verifier_executable_read:{error}"))?,
        ),
        BUNDLE_INPUT_SCHEMA,
        CHALLENGE_SCHEMA,
    ))
    .map_err(str::to_owned)?;
    let signing_key = read_signing_key(private_key_path)?;
    let receipt = ExactMemoryCleanupReceiptV1::seal_verified(
        &bundle.bundle_id_sha256,
        &bundle.package_id,
        &bundle.candidate_root_sha256,
        &bundle.active_registry_root_sha256,
        &bundle.execution_payload_sha256,
        &standalone_restart_root_sha256,
        &challenge_root_sha256,
        &challenge.source_receipt_root_sha256,
        &verifier_tcb_root_sha256,
        &signing_key,
    )
    .map_err(str::to_owned)?;
    write_atomic(
        output_path,
        &canonical_json_bytes(&receipt).map_err(str::to_owned)?,
    )
}

fn validate_bundle_input(input: &BundleInputV1) -> Result<(), String> {
    if input.schema != BUNDLE_INPUT_SCHEMA
        || input.package_id.is_empty()
        || !valid_nonzero_sha256(&input.bundle_id_sha256)
        || !valid_nonzero_sha256(&input.candidate_root_sha256)
        || !valid_nonzero_sha256(&input.active_registry_root_sha256)
        || !valid_nonzero_sha256(&input.execution_payload_sha256)
        || !input.restart_bundle.has_canonical_bundle_v4()
        || bundle_id(&input.restart_bundle)? != input.bundle_id_sha256
    {
        return Err("cleanup_verifier_bundle_input_invalid".to_owned());
    }
    Ok(())
}

fn validate_challenge(challenge: &CleanupChallengeV1) -> Result<(), String> {
    if challenge.schema != CHALLENGE_SCHEMA
        || challenge.package_id.is_empty()
        || challenge.request_text.len() > 16 * 1024 * 1024
        || !valid_nonzero_sha256(&challenge.bundle_id_sha256)
        || !valid_nonzero_sha256(&challenge.source_receipt_root_sha256)
        || !valid_nonzero_sha256(&challenge.expected_response_sha256)
    {
        return Err("cleanup_verifier_challenge_invalid".to_owned());
    }
    Ok(())
}

fn bundle_id(bundle: &VerifiedOperatorRestartBundle) -> Result<String, String> {
    let id = bundle
        .canonical_bundle_id()
        .ok_or_else(|| "cleanup_verifier_bundle_id_missing".to_owned())?;
    Ok(id.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn read_canonical_json<T>(path: &Path, label: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let bytes = fs::read(path).map_err(|error| format!("{label}_read:{error}"))?;
    let value: T =
        serde_json::from_slice(&bytes).map_err(|error| format!("{label}_decode:{error}"))?;
    if canonical_json_bytes(&value).map_err(str::to_owned)? != bytes {
        return Err(format!("{label}_noncanonical"));
    }
    Ok(value)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "cleanup_verifier_output_parent_missing".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cleanup_verifier_output_parent:{error}"))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("cleanup_verifier_output_create:{error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("cleanup_verifier_output_write:{error}"))?;
    file.sync_all()
        .map_err(|error| format!("cleanup_verifier_output_sync:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cleanup_verifier_output_rename:{error}"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("cleanup_verifier_output_dir_sync:{error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
