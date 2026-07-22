use std::fs;

use nando_operator_learning::GenerationCaptureIndexV3;
use nando_operator_persistence::{
    GenerationCheckpointStoreV3, join_generation_checkpoint_to_capture_index_v3,
};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierArtifactSetV3;
use nando_operator_runtime::TrafficShadowGenerationV3;

use super::{GenerationShadowConfigV3, GenerationShadowSnapshotV3};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowLoadErrorV3 {
    InvalidConfig,
    Store,
    CaptureIndexMissing,
    CaptureIndexInvalid,
    CaptureJoin,
    SupportNotFrozen,
    RuntimeGeneration,
    VerifierArtifacts,
    ArtifactRootMismatch,
}

pub(super) fn load_generation_shadow_snapshot_v3(
    config: &GenerationShadowConfigV3,
) -> Result<Option<GenerationShadowSnapshotV3>, GenerationShadowLoadErrorV3> {
    config
        .validate()
        .map_err(|_| GenerationShadowLoadErrorV3::InvalidConfig)?;
    let store = GenerationCheckpointStoreV3::open(&config.store_path)
        .map_err(|_| GenerationShadowLoadErrorV3::Store)?;
    let restored = store
        .restore()
        .map_err(|_| GenerationShadowLoadErrorV3::Store)?;
    let Some(checkpoint) = restored.into_checkpoint() else {
        return Ok(None);
    };
    let capture_bytes = fs::read(&config.capture_index_path)
        .map_err(|_| GenerationShadowLoadErrorV3::CaptureIndexMissing)?;
    let capture_index = GenerationCaptureIndexV3::from_canonical_bytes(&capture_bytes)
        .map_err(|_| GenerationShadowLoadErrorV3::CaptureIndexInvalid)?;
    let joined = join_generation_checkpoint_to_capture_index_v3(checkpoint, &capture_index)
        .map_err(|_| GenerationShadowLoadErrorV3::CaptureJoin)?;
    if joined.checkpoint().ledger().support().is_empty()
        || joined.checkpoint().ledger().freeze().is_none()
    {
        return Err(GenerationShadowLoadErrorV3::SupportNotFrozen);
    }
    let traffic_generation =
        TrafficShadowGenerationV3::from_restored_generation(joined.checkpoint().generation())
            .map_err(|_| GenerationShadowLoadErrorV3::RuntimeGeneration)?;
    let verifier_artifacts =
        IndependentVerifierArtifactSetV3::new(joined.checkpoint().generation().artifacts())
            .map_err(|_| GenerationShadowLoadErrorV3::VerifierArtifacts)?;
    if verifier_artifacts.artifact_set_sha256()
        != joined
            .checkpoint()
            .generation()
            .manifest()
            .components()
            .artifact_set_sha256
    {
        return Err(GenerationShadowLoadErrorV3::ArtifactRootMismatch);
    }
    let capture_index_sha256 = joined.capture_index_sha256().to_owned();
    Ok(Some(GenerationShadowSnapshotV3::new(
        joined.into_checkpoint(),
        capture_index_sha256,
        traffic_generation,
        verifier_artifacts,
    )))
}
