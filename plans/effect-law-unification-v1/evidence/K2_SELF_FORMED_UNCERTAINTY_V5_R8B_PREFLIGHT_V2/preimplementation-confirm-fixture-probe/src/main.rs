use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2UncertaintyConfirmAttemptDescriptorV1, K2UncertaintyConfirmGeneratorRequestV1,
    K2UncertaintyConfirmGeneratorResponseV1, K2UncertaintyConfirmOwnerRequestV1,
    K2UncertaintyConfirmPrivateSplitReceiptV1, K2UncertaintyConfirmSplitReceiptV1,
    K2UncertaintyGeneratorRequestV1, composition_root_v1, composition_sha256_file_v1,
    execute_self_formed_confirm_owner_v1, generate_self_formed_confirm_batch_v1,
    generate_self_formed_development_batch_v1, publish_confirm_generator_split_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1,
};
use serde::{Serialize, de::DeserializeOwned};

const FIXTURE_DOMAIN: &str = "nando.k2-self-formed-r8b-preimplementation-fixture.v1";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let owner_executable = required_path(args.next(), "owner executable")?;
    let generator_executable = required_path(args.next(), "generator executable")?;
    let development_seed_path = required_path(args.next(), "development seed")?;
    let output_root = required_path(args.next(), "output root")?;
    if args.next().is_some() {
        return Err("expected exactly four arguments".into());
    }

    let owner_sha256 = composition_sha256_file_v1(&owner_executable)?;
    let generator_sha256 = composition_sha256_file_v1(&generator_executable)?;
    let development_seed = fs::read(&development_seed_path)?;

    create_private_directory(&output_root)?;
    let work_root = output_root.join("work");
    create_private_directory(&work_root)?;

    capture_historical_development_owner(
        &output_root,
        &work_root,
        &owner_executable,
        &generator_executable,
        &owner_sha256,
        &generator_sha256,
        development_seed,
    )?;
    capture_confirm_generator_split(&output_root, &work_root, &generator_sha256)?;

    println!("owner_executable_sha256={owner_sha256}");
    println!("generator_executable_sha256={generator_sha256}");
    println!("fixture_root={}", output_root.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn capture_historical_development_owner(
    output_root: &Path,
    work_root: &Path,
    owner_executable: &Path,
    generator_executable: &Path,
    owner_sha256: &str,
    generator_sha256: &str,
    development_seed: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let generator_request = K2UncertaintyGeneratorRequestV1::development(
        development_seed,
        generator_sha256.to_owned(),
    )?;
    let expected = generate_self_formed_development_batch_v1(&generator_request)?;
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        expected.public.experiment_id_sha256,
        fixture_root("development-freeze")?,
        fixture_root("development-manifest")?,
        owner_sha256.to_owned(),
        generator_sha256.to_owned(),
    )?;
    let lab_root = work_root.join("development-lab");
    create_private_directory(&lab_root)?;
    let owner_request = K2UncertaintyConfirmOwnerRequestV1::development_rehearsal(
        descriptor,
        lab_root.to_string_lossy().into_owned(),
        "attempt".to_owned(),
        generator_executable.to_string_lossy().into_owned(),
        generator_request,
    )?;
    let receipt = execute_self_formed_confirm_owner_v1(&owner_request, owner_executable)?;
    receipt.validate()?;
    receipt.pipe_receipt.validate()?;

    write_canonical(
        &output_root.join("historical-development-owner-receipt.json"),
        &receipt,
    )?;
    write_canonical(
        &output_root.join("historical-development-pipe-receipt.json"),
        &receipt.pipe_receipt,
    )
}

fn capture_confirm_generator_split(
    output_root: &Path,
    work_root: &Path,
    generator_sha256: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = K2UncertaintyConfirmGeneratorRequestV1::seal(
        vec![0xa5; 32],
        fixture_root("confirm-successor-freeze")?,
        fixture_root("confirm-authorization-receipt")?,
        generator_sha256.to_owned(),
    )?;
    let response = generate_self_formed_confirm_batch_v1(&request)?;
    let split_root = work_root.join("confirm-split");
    let split = publish_confirm_generator_split_v1(&split_root, &request, &response)?;
    let private: K2UncertaintyConfirmPrivateSplitReceiptV1 = uncertainty_decode_v1(&fs::read(
        split_root.join("private/private-split-receipt.json"),
    )?)?;

    request.validate()?;
    response.validate()?;
    split.validate()?;
    private.validate()?;
    require_split_binding(&request, &response, &split, &private)?;

    write_canonical(
        &output_root.join("confirm-generator-request.json"),
        &request,
    )?;
    write_canonical(
        &output_root.join("confirm-generator-response.json"),
        &response,
    )?;
    write_canonical(
        &output_root.join("confirm-private-split-receipt.json"),
        &private,
    )?;
    write_canonical(&output_root.join("confirm-split-receipt.json"), &split)?;

    let artifact_root = output_root.join("confirm-stored-artifacts");
    create_private_directory(&artifact_root)?;
    let artifacts = private
        .artifacts
        .iter()
        .chain(split.artifacts.iter())
        .collect::<Vec<_>>();
    for (index, artifact) in artifacts.iter().enumerate() {
        artifact.validate()?;
        write_canonical(
            &artifact_root.join(format!("artifact-{index:02}.json")),
            *artifact,
        )?;
    }
    Ok(())
}

fn require_split_binding(
    request: &K2UncertaintyConfirmGeneratorRequestV1,
    response: &K2UncertaintyConfirmGeneratorResponseV1,
    split: &K2UncertaintyConfirmSplitReceiptV1,
    private: &K2UncertaintyConfirmPrivateSplitReceiptV1,
) -> Result<(), Box<dyn std::error::Error>> {
    if response.generator_request_root_sha256 != request.request_root_sha256
        || split.generator_request_root_sha256 != request.request_root_sha256
        || split.generator_response_root_sha256 != response.response_root_sha256
        || private.generator_request_root_sha256 != request.request_root_sha256
        || private.generator_response_root_sha256 != response.response_root_sha256
        || split.private_split_root_sha256 != private.private_split_root_sha256
    {
        return Err("Confirm fixture root chain mismatch".into());
    }
    Ok(())
}

fn required_path(
    value: Option<std::ffi::OsString>,
    label: &'static str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    value.map(PathBuf::from).ok_or_else(|| label.into())
}

fn fixture_root(label: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(composition_root_v1(&(FIXTURE_DOMAIN, label))?)
}

fn create_private_directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn write_canonical<T>(path: &Path, value: &T) -> Result<(), Box<dyn std::error::Error>>
where
    T: DeserializeOwned + PartialEq + Serialize,
{
    let bytes = uncertainty_bytes_v1(value)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    let reopened_bytes = fs::read(path)?;
    let reopened: T = uncertainty_decode_v1(&reopened_bytes)?;
    if &reopened != value || uncertainty_bytes_v1(&reopened)? != bytes {
        return Err(format!("canonical roundtrip mismatch: {}", path.display()).into());
    }
    Ok(())
}
