use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1};
use super::{
    K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1,
    K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1, K2UncertaintyCasePreverificationV1,
    K2UncertaintyCasePreverificationV2, K2UncertaintyProbeArtifactsV1, K2UncertaintyProbeRequestV1,
    K2UncertaintyPublicCaseArtifactV1, K2UncertaintyPublicComponentArtifactV1,
    K2UncertaintyPublicComponentKindV1, K2UncertaintyPublicPrecommitReceiptV1,
    K2UncertaintyPublicPreparedCaseV1, reopen_self_formed_probe_output_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1,
};

const RECEIPT_PATH_V1: &str = "all-cases-precommitted.json";

pub fn publish_self_formed_public_case_v1(
    root: &Path,
    prepared: &K2UncertaintyPublicPreparedCaseV1,
) -> K2CompositionResultV1<K2UncertaintyPublicCaseArtifactV1> {
    prepared.validate()?;
    let case_root = format!("cases/case-{:02}", prepared.case_sequence);
    let mut components = vec![
        publish_component_v1(
            root,
            &case_root,
            K2UncertaintyPublicComponentKindV1::ProbeRequest,
            &prepared.probe_request,
            prepared.probe_request.request_root_sha256.clone(),
        )?,
        publish_component_v1(
            root,
            &case_root,
            K2UncertaintyPublicComponentKindV1::ProbeArtifacts,
            &prepared.probe_artifacts,
            prepared.probe_artifacts.artifacts_root_sha256.clone(),
        )?,
        publish_component_v1(
            root,
            &case_root,
            K2UncertaintyPublicComponentKindV1::SelectionPreverification,
            &prepared.selection_preverification,
            prepared
                .selection_preverification
                .receipt_root_sha256
                .clone(),
        )?,
        publish_component_v1(
            root,
            &case_root,
            K2UncertaintyPublicComponentKindV1::Preverification,
            &prepared.preverification,
            prepared.preverification.receipt_root_sha256.clone(),
        )?,
    ];
    components.sort();
    let mut artifact = K2UncertaintyPublicCaseArtifactV1 {
        schema: K2_UNCERTAINTY_PUBLIC_CASE_ARTIFACT_SCHEMA_V1.to_owned(),
        case_sequence: prepared.case_sequence,
        case_id_sha256: prepared
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256
            .clone(),
        components,
        prepared_case_root_sha256: prepared.prepared_case_root_sha256.clone(),
        artifact_root_sha256: String::new(),
    };
    artifact.artifact_root_sha256 = artifact.expected_root()?;
    artifact.validate()?;
    validate_self_formed_public_case_artifact_v1(root, &artifact)?;
    Ok(artifact)
}

pub fn publish_self_formed_public_precommit_v1(
    root: &Path,
    receipt: &K2UncertaintyPublicPrecommitReceiptV1,
) -> K2CompositionResultV1<()> {
    receipt.validate()?;
    for artifact in &receipt.case_artifacts {
        validate_self_formed_public_case_artifact_v1(root, artifact)?;
    }
    if root.join("private").exists() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_precommit_private_path_present",
        ));
    }
    atomic_create_v1(
        root,
        RECEIPT_PATH_V1,
        &uncertainty_bytes_v1(receipt)?,
        0o600,
    )?;
    let reopened = load_self_formed_public_precommit_v1(root)?;
    if reopened != *receipt {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_precommit_reopen_mismatch",
        ));
    }
    Ok(())
}

pub fn load_self_formed_public_precommit_v1(
    root: &Path,
) -> K2CompositionResultV1<K2UncertaintyPublicPrecommitReceiptV1> {
    let path = root.join(RECEIPT_PATH_V1);
    require_regular_mode_v1(&path, 0o600)?;
    let bytes = fs::read(&path)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_public_precommit"))?;
    let receipt: K2UncertaintyPublicPrecommitReceiptV1 = uncertainty_decode_v1(&bytes)?;
    receipt.validate()?;
    for artifact in &receipt.case_artifacts {
        validate_self_formed_public_case_artifact_v1(root, artifact)?;
    }
    Ok(receipt)
}

pub fn load_self_formed_public_case_v1(
    root: &Path,
    artifact: &K2UncertaintyPublicCaseArtifactV1,
) -> K2CompositionResultV1<K2UncertaintyPublicPreparedCaseV1> {
    validate_self_formed_public_case_artifact_v1(root, artifact)?;
    let probe_request: K2UncertaintyProbeRequestV1 = load_component_v1(
        root,
        artifact,
        K2UncertaintyPublicComponentKindV1::ProbeRequest,
    )?;
    let probe_artifacts: K2UncertaintyProbeArtifactsV1 = load_component_v1(
        root,
        artifact,
        K2UncertaintyPublicComponentKindV1::ProbeArtifacts,
    )?;
    let selection_preverification: K2UncertaintyCasePreverificationV1 = load_component_v1(
        root,
        artifact,
        K2UncertaintyPublicComponentKindV1::SelectionPreverification,
    )?;
    let preverification: K2UncertaintyCasePreverificationV2 = load_component_v1(
        root,
        artifact,
        K2UncertaintyPublicComponentKindV1::Preverification,
    )?;
    let value = K2UncertaintyPublicPreparedCaseV1::seal(
        artifact.case_sequence,
        probe_request,
        probe_artifacts,
        selection_preverification,
        preverification,
    )?;
    value.validate()?;
    if value.case_sequence != artifact.case_sequence
        || value.prepared_case_root_sha256 != artifact.prepared_case_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_case_semantic_binding_invalid",
        ));
    }
    reopen_self_formed_probe_output_v1(
        &root.join(format!("probes/case-{:02}", value.case_sequence)),
        &value.probe_artifacts,
    )?;
    Ok(value)
}

fn validate_self_formed_public_case_artifact_v1(
    root: &Path,
    artifact: &K2UncertaintyPublicCaseArtifactV1,
) -> K2CompositionResultV1<()> {
    artifact.validate()?;
    for component in &artifact.components {
        let path = root.join(&component.relative_path);
        require_regular_mode_v1(&path, component.mode)?;
        let bytes = fs::read(&path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_public_case_artifact"))?;
        if bytes.len() as u64 != component.byte_len
            || composition_sha256_bytes_v1(&bytes) != component.content_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_public_case_artifact_bytes_invalid",
            ));
        }
    }
    Ok(())
}

fn publish_component_v1<T: Serialize>(
    root: &Path,
    case_root: &str,
    kind: K2UncertaintyPublicComponentKindV1,
    value: &T,
    semantic_root_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyPublicComponentArtifactV1> {
    let bytes = uncertainty_bytes_v1(value)?;
    let relative_path = format!("{case_root}/{}", kind.file_name());
    let mut artifact = K2UncertaintyPublicComponentArtifactV1 {
        schema: K2_UNCERTAINTY_PUBLIC_COMPONENT_ARTIFACT_SCHEMA_V1.to_owned(),
        kind,
        relative_path: relative_path.clone(),
        content_sha256: composition_sha256_bytes_v1(&bytes),
        byte_len: bytes.len() as u64,
        mode: 0o600,
        semantic_root_sha256,
        artifact_root_sha256: String::new(),
    };
    artifact.artifact_root_sha256 = artifact.expected_root()?;
    artifact.validate()?;
    atomic_create_v1(root, &relative_path, &bytes, artifact.mode)?;
    Ok(artifact)
}

fn load_component_v1<T: DeserializeOwned + Serialize>(
    root: &Path,
    artifact: &K2UncertaintyPublicCaseArtifactV1,
    kind: K2UncertaintyPublicComponentKindV1,
) -> K2CompositionResultV1<T> {
    let component = artifact
        .components
        .iter()
        .find(|component| component.kind == kind)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_public_component_missing",
        ))?;
    uncertainty_decode_v1(
        &fs::read(root.join(&component.relative_path))
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_public_component"))?,
    )
}

fn atomic_create_v1(
    root: &Path,
    relative_path: &str,
    bytes: &[u8],
    mode: u32,
) -> K2CompositionResultV1<()> {
    let path = root.join(relative_path);
    let parent = path.parent().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_public_artifact_parent_missing",
    ))?;
    fs::create_dir_all(parent)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_public_artifact_parent"))?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_public_artifact_parent"))?;
    let temporary = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_public_artifact"))?;
    if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "write_self_formed_public_artifact",
        ));
    }
    if path.exists() || fs::rename(&temporary, &path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_artifact_already_exists",
        ));
    }
    sync_directory_v1(parent)?;
    if parent != root {
        sync_directory_v1(root)?;
    }
    Ok(())
}

fn require_regular_mode_v1(path: &Path, expected_mode: u32) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_public_artifact"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.permissions().mode() & 0o777 != expected_mode
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_public_artifact_mode_invalid",
        ));
    }
    Ok(())
}

fn sync_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_public_artifact_directory"))
}
