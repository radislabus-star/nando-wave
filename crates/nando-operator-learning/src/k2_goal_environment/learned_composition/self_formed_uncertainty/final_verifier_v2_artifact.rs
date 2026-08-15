use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_FINAL_VERIFIER_ARTIFACT_SCHEMA_V2,
    K2_UNCERTAINTY_FINAL_VERIFIER_MATERIAL_SCHEMA_V2, K2UncertaintyBatchPrecommitV2,
    K2UncertaintyCasePreverificationV2, denied_authority_v1, require_denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

const FINAL_V2_ARTIFACT_DIRECTORY: &str = "final-v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum K2UncertaintyFinalVerifierArtifactFaultV2 {
    None,
    BeforeRename,
    AfterRename,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyFinalVerifierArtifactKindV2 {
    BatchPrecommit,
    CasePreverification,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFinalVerifierArtifactV2 {
    pub schema: String,
    pub kind: K2UncertaintyFinalVerifierArtifactKindV2,
    pub relative_path: String,
    pub byte_len: u64,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub artifact_root_sha256: String,
}

impl K2UncertaintyFinalVerifierArtifactV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.content_sha256)?;
        require_composition_root_v1(&self.semantic_root_sha256)?;
        let path = Path::new(&self.relative_path);
        let path_valid = !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
            && self.relative_path
                == format!("{FINAL_V2_ARTIFACT_DIRECTORY}/{}.json", self.content_sha256);
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_FINAL_VERIFIER_ARTIFACT_SCHEMA_V2
            || !path_valid
            || self.byte_len == 0
            || self.artifact_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_artifact_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_FINAL_VERIFIER_ARTIFACT_SCHEMA_V2,
            self.kind,
            &self.relative_path,
            self.byte_len,
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyFinalVerifierMaterialV2 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub case_id_sha256: String,
    pub batch_precommit: K2UncertaintyFinalVerifierArtifactV2,
    pub case_preverification: K2UncertaintyFinalVerifierArtifactV2,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub material_root_sha256: String,
}

impl K2UncertaintyFinalVerifierMaterialV2 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.case_id_sha256)?;
        self.batch_precommit.validate()?;
        self.case_preverification.validate()?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_FINAL_VERIFIER_MATERIAL_SCHEMA_V2
            || self.batch_precommit.kind != K2UncertaintyFinalVerifierArtifactKindV2::BatchPrecommit
            || self.case_preverification.kind
                != K2UncertaintyFinalVerifierArtifactKindV2::CasePreverification
            || self.batch_precommit.content_sha256 == self.case_preverification.content_sha256
            || self.material_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_material_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_FINAL_VERIFIER_MATERIAL_SCHEMA_V2,
            &self.experiment_id_sha256,
            &self.case_id_sha256,
            &self.batch_precommit.artifact_root_sha256,
            &self.case_preverification.artifact_root_sha256,
            &self.authority,
        ))
    }
}

pub struct K2UncertaintyResolvedFinalVerifierMaterialV2 {
    pub batch_precommit: K2UncertaintyBatchPrecommitV2,
    pub case_preverification: K2UncertaintyCasePreverificationV2,
}

pub fn publish_self_formed_final_verifier_material_v2(
    root: &Path,
    batch: &K2UncertaintyBatchPrecommitV2,
    case: &K2UncertaintyCasePreverificationV2,
) -> K2CompositionResultV1<K2UncertaintyFinalVerifierMaterialV2> {
    batch.validate()?;
    case.validate()?;
    let case_id = &case.selection_preverification.case_id_sha256;
    if !batch
        .cases
        .iter()
        .any(|entry| &entry.case_id_sha256 == case_id)
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_verifier_material_case_missing",
        ));
    }
    let batch_artifact = publish_artifact_v2(
        root,
        K2UncertaintyFinalVerifierArtifactKindV2::BatchPrecommit,
        &uncertainty_bytes_v1(batch)?,
        batch.batch_root_sha256.clone(),
    )?;
    let case_artifact = publish_artifact_v2(
        root,
        K2UncertaintyFinalVerifierArtifactKindV2::CasePreverification,
        &uncertainty_bytes_v1(case)?,
        case.receipt_root_sha256.clone(),
    )?;
    let mut material = K2UncertaintyFinalVerifierMaterialV2 {
        schema: K2_UNCERTAINTY_FINAL_VERIFIER_MATERIAL_SCHEMA_V2.to_owned(),
        experiment_id_sha256: batch.experiment_id_sha256.clone(),
        case_id_sha256: case_id.clone(),
        batch_precommit: batch_artifact,
        case_preverification: case_artifact,
        authority: denied_authority_v1(),
        material_root_sha256: String::new(),
    };
    material.material_root_sha256 = material.expected_root()?;
    material.validate()?;
    Ok(material)
}

pub fn resolve_self_formed_final_verifier_material_v2(
    root: &Path,
    material: &K2UncertaintyFinalVerifierMaterialV2,
) -> K2CompositionResultV1<K2UncertaintyResolvedFinalVerifierMaterialV2> {
    material.validate()?;
    let batch_bytes = read_artifact_v2(root, &material.batch_precommit)?;
    let case_bytes = read_artifact_v2(root, &material.case_preverification)?;
    let batch: K2UncertaintyBatchPrecommitV2 = uncertainty_decode_v1(&batch_bytes)?;
    let case: K2UncertaintyCasePreverificationV2 = uncertainty_decode_v1(&case_bytes)?;
    batch.validate()?;
    case.validate()?;
    if batch.batch_root_sha256 != material.batch_precommit.semantic_root_sha256
        || case.receipt_root_sha256 != material.case_preverification.semantic_root_sha256
        || batch.experiment_id_sha256 != material.experiment_id_sha256
        || case.selection_preverification.case_id_sha256 != material.case_id_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_verifier_material_binding_v2_invalid",
        ));
    }
    Ok(K2UncertaintyResolvedFinalVerifierMaterialV2 {
        batch_precommit: batch,
        case_preverification: case,
    })
}

fn publish_artifact_v2(
    root: &Path,
    kind: K2UncertaintyFinalVerifierArtifactKindV2,
    bytes: &[u8],
    semantic_root_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyFinalVerifierArtifactV2> {
    publish_artifact_with_fault_v2(
        root,
        kind,
        bytes,
        semantic_root_sha256,
        K2UncertaintyFinalVerifierArtifactFaultV2::None,
    )
}

fn publish_artifact_with_fault_v2(
    root: &Path,
    kind: K2UncertaintyFinalVerifierArtifactKindV2,
    bytes: &[u8],
    semantic_root_sha256: String,
    fault: K2UncertaintyFinalVerifierArtifactFaultV2,
) -> K2CompositionResultV1<K2UncertaintyFinalVerifierArtifactV2> {
    let content_sha256 = composition_sha256_bytes_v1(bytes);
    let relative_path = format!("{FINAL_V2_ARTIFACT_DIRECTORY}/{content_sha256}.json");
    let directory = root.join(FINAL_V2_ARTIFACT_DIRECTORY);
    fs::create_dir_all(&directory)
        .map_err(|_| K2CompositionErrorV1::Io("create_final_verifier_artifact_v2_root"))?;
    File::open(root)
        .and_then(|handle| handle.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_final_verifier_artifact_v2_parent"))?;
    let path = root.join(&relative_path);
    if path.exists() {
        let existing = fs::read(&path)
            .map_err(|_| K2CompositionErrorV1::Io("read_existing_final_verifier_artifact_v2"))?;
        if existing != bytes {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_artifact_collision_v2",
            ));
        }
    } else {
        let temporary = directory.join(format!(".{content_sha256}.tmp"));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| K2CompositionErrorV1::Io("create_final_verifier_artifact_v2_temp"))?;
        if file
            .write_all(bytes)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            let _ = fs::remove_file(&temporary);
            return Err(K2CompositionErrorV1::Io(
                "sync_final_verifier_artifact_v2_temp",
            ));
        }
        if fault == K2UncertaintyFinalVerifierArtifactFaultV2::BeforeRename {
            fs::remove_file(&temporary).map_err(|_| {
                K2CompositionErrorV1::Io("remove_final_verifier_artifact_v2_fault_temp")
            })?;
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_artifact_v2_fault_before_rename",
            ));
        }
        fs::rename(&temporary, &path)
            .map_err(|_| K2CompositionErrorV1::Io("rename_final_verifier_artifact_v2"))?;
        if fault == K2UncertaintyFinalVerifierArtifactFaultV2::AfterRename {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_verifier_artifact_v2_fault_after_rename",
            ));
        }
    }
    File::open(&directory)
        .and_then(|handle| handle.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_final_verifier_artifact_v2_root"))?;
    let mut artifact = K2UncertaintyFinalVerifierArtifactV2 {
        schema: K2_UNCERTAINTY_FINAL_VERIFIER_ARTIFACT_SCHEMA_V2.to_owned(),
        kind,
        relative_path,
        byte_len: bytes.len() as u64,
        content_sha256,
        semantic_root_sha256,
        authority: denied_authority_v1(),
        artifact_root_sha256: String::new(),
    };
    artifact.artifact_root_sha256 = artifact.expected_root()?;
    artifact.validate()?;
    Ok(artifact)
}

fn read_artifact_v2(
    root: &Path,
    artifact: &K2UncertaintyFinalVerifierArtifactV2,
) -> K2CompositionResultV1<Vec<u8>> {
    artifact.validate()?;
    let bytes = fs::read(root.join(&artifact.relative_path))
        .map_err(|_| K2CompositionErrorV1::Io("read_final_verifier_artifact_v2"))?;
    if bytes.len() as u64 != artifact.byte_len
        || composition_sha256_bytes_v1(&bytes) != artifact.content_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_verifier_artifact_content_v2_invalid",
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn final_verifier_artifacts_are_atomic_idempotent_and_content_bound() {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-final-v2-artifact-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create final V2 test root");
        let semantic_root =
            uncertainty_root_v1(&("final-v2-semantic", sequence)).expect("final V2 semantic root");
        let bytes = b"root-addressed-final-verifier-evidence";

        let before_root = root.join("before");
        fs::create_dir_all(&before_root).expect("create before-fault root");
        let before = publish_artifact_with_fault_v2(
            &before_root,
            K2UncertaintyFinalVerifierArtifactKindV2::BatchPrecommit,
            bytes,
            semantic_root.clone(),
            K2UncertaintyFinalVerifierArtifactFaultV2::BeforeRename,
        );
        assert_error(
            before,
            "self_formed_final_verifier_artifact_v2_fault_before_rename",
        );
        let before_directory = before_root.join(FINAL_V2_ARTIFACT_DIRECTORY);
        assert_eq!(
            fs::read_dir(&before_directory)
                .expect("read before-fault directory")
                .count(),
            0
        );

        let after_root = root.join("after");
        fs::create_dir_all(&after_root).expect("create after-fault root");
        let after = publish_artifact_with_fault_v2(
            &after_root,
            K2UncertaintyFinalVerifierArtifactKindV2::CasePreverification,
            bytes,
            semantic_root.clone(),
            K2UncertaintyFinalVerifierArtifactFaultV2::AfterRename,
        );
        assert_error(
            after,
            "self_formed_final_verifier_artifact_v2_fault_after_rename",
        );
        let recovered = publish_artifact_v2(
            &after_root,
            K2UncertaintyFinalVerifierArtifactKindV2::CasePreverification,
            bytes,
            semantic_root.clone(),
        )
        .expect("recover after-rename artifact");
        assert_eq!(
            read_artifact_v2(&after_root, &recovered).expect("read recovered artifact"),
            bytes
        );
        assert_eq!(
            fs::metadata(after_root.join(&recovered.relative_path))
                .expect("recovered artifact metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );

        let repeated = publish_artifact_v2(
            &after_root,
            K2UncertaintyFinalVerifierArtifactKindV2::CasePreverification,
            bytes,
            semantic_root,
        )
        .expect("idempotent artifact publication");
        assert_eq!(recovered, repeated);
        fs::write(after_root.join(&recovered.relative_path), b"tampered")
            .expect("tamper artifact bytes");
        assert_error(
            read_artifact_v2(&after_root, &recovered),
            "self_formed_final_verifier_artifact_content_v2_invalid",
        );

        fs::remove_dir_all(root).expect("remove final V2 test root");
    }

    fn assert_error<T>(result: K2CompositionResultV1<T>, code: &str) {
        let error = result.err().expect("fault control accepted");
        assert!(error.to_string().contains(code), "wrong error: {error}");
    }
}
