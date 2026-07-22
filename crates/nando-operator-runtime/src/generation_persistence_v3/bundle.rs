use nando_operator_kernel::{
    ExecutableProtocolModeArtifactV3, OperatorGenerationManifestV3, canonical_json_sha256,
    executable_artifact_set_sha256_v3, sha256_bytes,
};
use serde::{Deserialize, Serialize};

use crate::{StructuralDispatchIndexV3, compile_structural_dispatch_index_v3};

pub const OPERATOR_GENERATION_RESTART_SCHEMA_V3: &str =
    "nando.operator-generation-restart-bundle.v3.f7";
pub const OPERATOR_GENERATION_RESTART_MAX_BYTES_V3: usize = 512 * 1024;

pub struct RestoredOperatorGenerationV3 {
    manifest: OperatorGenerationManifestV3,
    artifacts: Box<[ExecutableProtocolModeArtifactV3]>,
    index: StructuralDispatchIndexV3,
    bundle_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorGenerationRestartErrorV3 {
    Encode,
    Decode,
    BudgetExhausted,
    InvalidManifest,
    InvalidArtifact,
    DuplicateArtifact,
    IndexCompile,
    ManifestMismatch,
    DigestMismatch,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperatorGenerationRestartWireV3 {
    schema: String,
    manifest_bytes: Vec<u8>,
    artifacts: Vec<OperatorGenerationArtifactWireV3>,
    bundle_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OperatorGenerationArtifactWireV3 {
    artifact_sha256: String,
    canonical_bytes: Vec<u8>,
}

pub fn encode_operator_generation_restart_bundle_v3(
    manifest: &OperatorGenerationManifestV3,
    artifacts: &[ExecutableProtocolModeArtifactV3],
) -> Result<Box<[u8]>, OperatorGenerationRestartErrorV3> {
    let artifacts = canonical_artifact_wires(artifacts)?;
    let decoded_artifacts = decode_artifact_wires(&artifacts)?;
    let index = compile_structural_dispatch_index_v3(&decoded_artifacts)
        .map_err(|_| OperatorGenerationRestartErrorV3::IndexCompile)?;
    validate_manifest_alignment(manifest, &decoded_artifacts, &index)?;
    let manifest_bytes = manifest
        .canonical_bytes()
        .map_err(|_| OperatorGenerationRestartErrorV3::InvalidManifest)?;
    let bundle_sha256 = bundle_digest(manifest, &manifest_bytes, &artifacts)?;
    encode_wire(&OperatorGenerationRestartWireV3 {
        schema: OPERATOR_GENERATION_RESTART_SCHEMA_V3.to_owned(),
        manifest_bytes,
        artifacts,
        bundle_sha256,
    })
}

pub fn decode_operator_generation_restart_bundle_v3(
    bytes: &[u8],
) -> Result<RestoredOperatorGenerationV3, OperatorGenerationRestartErrorV3> {
    if bytes.len() > OPERATOR_GENERATION_RESTART_MAX_BYTES_V3 {
        return Err(OperatorGenerationRestartErrorV3::BudgetExhausted);
    }
    let wire: OperatorGenerationRestartWireV3 =
        serde_cbor::from_slice(bytes).map_err(|_| OperatorGenerationRestartErrorV3::Decode)?;
    if wire.schema != OPERATOR_GENERATION_RESTART_SCHEMA_V3 {
        return Err(OperatorGenerationRestartErrorV3::Decode);
    }
    let manifest = OperatorGenerationManifestV3::from_canonical_bytes(&wire.manifest_bytes)
        .map_err(|_| OperatorGenerationRestartErrorV3::InvalidManifest)?;
    let artifacts = decode_artifact_wires(&wire.artifacts)?;
    let index = compile_structural_dispatch_index_v3(&artifacts)
        .map_err(|_| OperatorGenerationRestartErrorV3::IndexCompile)?;
    validate_manifest_alignment(&manifest, &artifacts, &index)?;
    if bundle_digest(&manifest, &wire.manifest_bytes, &wire.artifacts)? != wire.bundle_sha256
        || encode_wire(&wire)?.as_ref() != bytes
    {
        return Err(OperatorGenerationRestartErrorV3::DigestMismatch);
    }
    Ok(RestoredOperatorGenerationV3 {
        manifest,
        artifacts: artifacts.into_boxed_slice(),
        index,
        bundle_sha256: wire.bundle_sha256,
    })
}

impl RestoredOperatorGenerationV3 {
    #[must_use]
    pub const fn manifest(&self) -> &OperatorGenerationManifestV3 {
        &self.manifest
    }

    #[must_use]
    pub const fn artifacts(&self) -> &[ExecutableProtocolModeArtifactV3] {
        &self.artifacts
    }

    #[must_use]
    pub const fn index(&self) -> &StructuralDispatchIndexV3 {
        &self.index
    }

    #[must_use]
    pub fn bundle_sha256(&self) -> &str {
        &self.bundle_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn canonical_artifact_wires(
    artifacts: &[ExecutableProtocolModeArtifactV3],
) -> Result<Vec<OperatorGenerationArtifactWireV3>, OperatorGenerationRestartErrorV3> {
    executable_artifact_set_sha256_v3(artifacts).map_err(map_artifact_set_error)?;
    let mut artifacts = artifacts.iter().collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.artifact_sha256().cmp(right.artifact_sha256()));
    artifacts
        .into_iter()
        .map(|artifact| {
            Ok(OperatorGenerationArtifactWireV3 {
                artifact_sha256: artifact.artifact_sha256().to_owned(),
                canonical_bytes: artifact
                    .canonical_bytes()
                    .map_err(|_| OperatorGenerationRestartErrorV3::InvalidArtifact)?,
            })
        })
        .collect()
}

fn decode_artifact_wires(
    artifacts: &[OperatorGenerationArtifactWireV3],
) -> Result<Vec<ExecutableProtocolModeArtifactV3>, OperatorGenerationRestartErrorV3> {
    if artifacts.is_empty() {
        return Err(OperatorGenerationRestartErrorV3::InvalidArtifact);
    }
    if artifacts.len() > nando_operator_kernel::OPERATOR_GENERATION_MAX_ARTIFACTS_V3 {
        return Err(OperatorGenerationRestartErrorV3::BudgetExhausted);
    }
    if artifacts
        .windows(2)
        .any(|pair| pair[0].artifact_sha256 >= pair[1].artifact_sha256)
    {
        return Err(OperatorGenerationRestartErrorV3::DuplicateArtifact);
    }
    artifacts
        .iter()
        .map(|artifact| {
            ExecutableProtocolModeArtifactV3::from_canonical_bytes(
                &artifact.canonical_bytes,
                &artifact.artifact_sha256,
            )
            .map_err(|_| OperatorGenerationRestartErrorV3::InvalidArtifact)
        })
        .collect()
}

fn validate_manifest_alignment(
    manifest: &OperatorGenerationManifestV3,
    artifacts: &[ExecutableProtocolModeArtifactV3],
    index: &StructuralDispatchIndexV3,
) -> Result<(), OperatorGenerationRestartErrorV3> {
    // Restore rebuilds executable structure from artifacts; persisted indexes
    // are never trusted as a second runtime truth.
    let artifact_set =
        executable_artifact_set_sha256_v3(artifacts).map_err(map_artifact_set_error)?;
    if manifest.components().artifact_set_sha256 != artifact_set
        || manifest.components().dispatch_index_sha256 != index.index_sha256()
    {
        return Err(OperatorGenerationRestartErrorV3::ManifestMismatch);
    }
    Ok(())
}

fn bundle_digest(
    manifest: &OperatorGenerationManifestV3,
    manifest_bytes: &[u8],
    artifacts: &[OperatorGenerationArtifactWireV3],
) -> Result<String, OperatorGenerationRestartErrorV3> {
    let artifact_commitments = artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.artifact_sha256.as_str(),
                sha256_bytes(&artifact.canonical_bytes),
            )
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        OPERATOR_GENERATION_RESTART_SCHEMA_V3,
        manifest.generation_id_sha256(),
        sha256_bytes(manifest_bytes),
        artifact_commitments,
    ))
    .map_err(|_| OperatorGenerationRestartErrorV3::Encode)
}

fn encode_wire(
    wire: &OperatorGenerationRestartWireV3,
) -> Result<Box<[u8]>, OperatorGenerationRestartErrorV3> {
    let bytes = serde_cbor::to_vec(wire).map_err(|_| OperatorGenerationRestartErrorV3::Encode)?;
    if bytes.len() > OPERATOR_GENERATION_RESTART_MAX_BYTES_V3 {
        return Err(OperatorGenerationRestartErrorV3::BudgetExhausted);
    }
    Ok(bytes.into_boxed_slice())
}

fn map_artifact_set_error(
    error: nando_operator_kernel::OperatorGenerationErrorV3,
) -> OperatorGenerationRestartErrorV3 {
    match error {
        nando_operator_kernel::OperatorGenerationErrorV3::DuplicateArtifact => {
            OperatorGenerationRestartErrorV3::DuplicateArtifact
        }
        nando_operator_kernel::OperatorGenerationErrorV3::ArtifactBudgetExhausted => {
            OperatorGenerationRestartErrorV3::BudgetExhausted
        }
        _ => OperatorGenerationRestartErrorV3::InvalidArtifact,
    }
}
