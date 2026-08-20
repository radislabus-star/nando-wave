use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, require_composition_root_v1};
use super::{
    K2UncertaintyCleanupArtifactKindV1, K2UncertaintyCleanupAuthorizationRequestV1,
    K2UncertaintyCleanupManifestV1, K2UncertaintyCleanupOwnerReceiptV1,
    K2UncertaintyCleanupOwnerRequestV1, K2UncertaintyCleanupReceiptV1,
    K2UncertaintyCleanupRegistryEntryV1, K2UncertaintyCleanupVerifyRequestV1,
    K2UncertaintyTerminalDispositionV1, K2UncertaintyTerminalEvaluationReceiptV1,
    K2UncertaintyTerminalModeV1, authorize_self_formed_cleanup_v1,
    census_self_formed_cleanup_artifacts_v1, execute_self_formed_cleanup_v1,
    publish_self_formed_cleanup_manifest_v1, uncertainty_root_v1, verify_self_formed_cleanup_v1,
};

pub fn run_self_formed_k12_cleanup_control_v1(
    scratch_root: &Path,
    experiment_root_sha256: String,
    owner_executable_sha256: String,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&experiment_root_sha256)?;
    require_composition_root_v1(&owner_executable_sha256)?;
    create_private_directory_v1(scratch_root)?;

    let retained = prepare_k12_cleanup_clone_v1(
        &scratch_root.join("retained-deletion"),
        &experiment_root_sha256,
        &owner_executable_sha256,
        "retained-deletion",
    )?;
    fs::remove_file(retained.governed_root.join("retained.json"))
        .map_err(|_| K2CompositionErrorV1::Io("remove_self_formed_k12_retained_evidence"))?;
    sync_directory_v1(&retained.governed_root)?;
    require_cleanup_rejection_v1(
        verify_k12_cleanup_clone_v1(&retained),
        "k2_composition_invalid:self_formed_cleanup_retained_path_missing",
    )?;

    let residue = prepare_k12_cleanup_clone_v1(
        &scratch_root.join("disposable-residue"),
        &experiment_root_sha256,
        &owner_executable_sha256,
        "disposable-residue",
    )?;
    let disposable_root = residue.governed_root.join("scratch");
    create_private_directory_v1(&disposable_root)?;
    create_private_file_v1(&disposable_root.join("temp.bin"), b"temporary")?;
    sync_directory_v1(&disposable_root)?;
    sync_directory_v1(&residue.governed_root)?;
    require_cleanup_rejection_v1(
        verify_k12_cleanup_clone_v1(&residue),
        "k2_composition_invalid:self_formed_cleanup_disposable_residue_present",
    )
}

struct K12CleanupCloneV1 {
    governed_root: PathBuf,
    control_root: PathBuf,
    manifest: K2UncertaintyCleanupManifestV1,
    owner_receipt: K2UncertaintyCleanupOwnerReceiptV1,
    verifier_executable_sha256: String,
}

fn prepare_k12_cleanup_clone_v1(
    root: &Path,
    experiment_root_sha256: &str,
    owner_executable_sha256: &str,
    label: &str,
) -> K2CompositionResultV1<K12CleanupCloneV1> {
    create_private_directory_v1(root)?;
    let governed_root = root.join("governed");
    let control_root = root.join("control");
    create_private_directory_v1(&governed_root)?;
    create_private_directory_v1(&control_root)?;
    create_private_file_v1(&governed_root.join("retained.json"), b"retained")?;
    create_private_file_v1(&governed_root.join("superseded.json"), b"superseded")?;
    create_private_directory_v1(&governed_root.join("scratch"))?;
    create_private_file_v1(&governed_root.join("scratch/temp.bin"), b"temporary")?;

    let scoped_root = |kind: &str| {
        uncertainty_root_v1(&(
            "nando.k2-self-formed-r7k-k12-control.v1",
            experiment_root_sha256,
            label,
            kind,
        ))
    };
    let registry = [
        (
            "scratch/temp.bin",
            K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace,
        ),
        (
            "retained.json",
            K2UncertaintyCleanupArtifactKindV1::RetainedEvidence,
        ),
        (
            "scratch",
            K2UncertaintyCleanupArtifactKindV1::DisposableWorkspace,
        ),
        (
            "superseded.json",
            K2UncertaintyCleanupArtifactKindV1::SupersededEvidence,
        ),
    ]
    .into_iter()
    .map(|(relative_path, artifact_kind)| {
        Ok(K2UncertaintyCleanupRegistryEntryV1 {
            relative_path: relative_path.to_owned(),
            artifact_kind,
            producer_executable_sha256: scoped_root(&format!("producer-{relative_path}"))?,
            producing_journal_event_root_sha256: scoped_root(&format!("journal-{relative_path}"))?,
        })
    })
    .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let (manifest, pages) = census_self_formed_cleanup_artifacts_v1(
        &governed_root,
        experiment_root_sha256.to_owned(),
        registry,
        scoped_root("census-executable")?,
    )?;
    publish_self_formed_cleanup_manifest_v1(&governed_root, &control_root, &manifest, &pages)?;
    let terminal = K2UncertaintyTerminalEvaluationReceiptV1::seal(
        K2UncertaintyTerminalModeV1::DevelopmentRehearsal,
        scoped_root("terminal-request")?,
        K2UncertaintyTerminalDispositionV1::DevelopmentRehearsalPass,
        "development_component_routes_complete".to_owned(),
        scoped_root("terminal-evaluator")?,
    )?;
    let authorization_request = K2UncertaintyCleanupAuthorizationRequestV1::seal(
        control_root.to_string_lossy().into_owned(),
        experiment_root_sha256.to_owned(),
        terminal,
        manifest.clone(),
        scoped_root("journal-projection")?,
        scoped_root("observer-durable")?,
        scoped_root("terminal-durable")?,
        scoped_root("cleanup-authorizer")?,
    )?;
    let authorization = authorize_self_formed_cleanup_v1(&authorization_request)?;
    let owner_request = K2UncertaintyCleanupOwnerRequestV1::seal(
        governed_root.to_string_lossy().into_owned(),
        control_root.to_string_lossy().into_owned(),
        authorization,
        owner_executable_sha256.to_owned(),
    )?;
    let owner_receipt = execute_self_formed_cleanup_v1(&owner_request)?;
    Ok(K12CleanupCloneV1 {
        governed_root,
        control_root,
        manifest,
        owner_receipt,
        verifier_executable_sha256: scoped_root("cleanup-verifier")?,
    })
}

fn verify_k12_cleanup_clone_v1(
    clone: &K12CleanupCloneV1,
) -> K2CompositionResultV1<K2UncertaintyCleanupReceiptV1> {
    verify_self_formed_cleanup_v1(&K2UncertaintyCleanupVerifyRequestV1::seal(
        clone.governed_root.to_string_lossy().into_owned(),
        clone.control_root.to_string_lossy().into_owned(),
        clone.manifest.clone(),
        clone.owner_receipt.clone(),
        clone.verifier_executable_sha256.clone(),
    )?)
}

fn require_cleanup_rejection_v1<T>(
    result: K2CompositionResultV1<T>,
    expected: &str,
) -> K2CompositionResultV1<()> {
    let error = result.err().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_k12_cleanup_unexpected_accept",
    ))?;
    if error.to_string() != expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_k12_cleanup_error_mismatch",
        ));
    }
    Ok(())
}

fn create_private_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    fs::create_dir(path)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_k12_directory"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_k12_directory"))
}

fn create_private_file_v1(path: &Path, bytes: &[u8]) -> K2CompositionResultV1<()> {
    fs::write(path, bytes).map_err(|_| K2CompositionErrorV1::Io("write_self_formed_k12_file"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_k12_file"))
}

fn sync_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_k12_directory"))
}
