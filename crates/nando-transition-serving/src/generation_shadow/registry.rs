use std::sync::{Arc, RwLock};

use nando_operator_persistence::{
    GenerationStoreErrorV3, RestoredGenerationCheckpointV3,
    validate_generation_checkpoint_transition_v3,
};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierArtifactSetV3;
use nando_operator_runtime::TrafficShadowGenerationV3;

pub struct GenerationShadowSnapshotV3 {
    checkpoint: RestoredGenerationCheckpointV3,
    capture_index_sha256: String,
    traffic_generation: Arc<TrafficShadowGenerationV3>,
    verifier_artifacts: IndependentVerifierArtifactSetV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowRegistryErrorV3 {
    Poisoned,
    InvalidTransition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowRegistryUpdateV3 {
    Installed,
    Unchanged,
}

#[derive(Default)]
pub struct GenerationShadowRegistryV3 {
    current: RwLock<Option<Arc<GenerationShadowSnapshotV3>>>,
}

impl GenerationShadowSnapshotV3 {
    pub(super) fn new(
        checkpoint: RestoredGenerationCheckpointV3,
        capture_index_sha256: String,
        traffic_generation: TrafficShadowGenerationV3,
        verifier_artifacts: IndependentVerifierArtifactSetV3,
    ) -> Self {
        Self {
            checkpoint,
            capture_index_sha256,
            traffic_generation: Arc::new(traffic_generation),
            verifier_artifacts,
        }
    }

    #[must_use]
    pub const fn checkpoint(&self) -> &RestoredGenerationCheckpointV3 {
        &self.checkpoint
    }

    #[must_use]
    pub const fn traffic_generation(&self) -> &Arc<TrafficShadowGenerationV3> {
        &self.traffic_generation
    }

    #[must_use]
    pub const fn verifier_artifacts(&self) -> &IndependentVerifierArtifactSetV3 {
        &self.verifier_artifacts
    }

    #[must_use]
    pub fn capture_index_sha256(&self) -> &str {
        &self.capture_index_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl GenerationShadowRegistryV3 {
    pub fn pin(
        &self,
    ) -> Result<Option<Arc<GenerationShadowSnapshotV3>>, GenerationShadowRegistryErrorV3> {
        self.current
            .read()
            .map(|current| current.as_ref().map(Arc::clone))
            .map_err(|_| GenerationShadowRegistryErrorV3::Poisoned)
    }

    pub fn install(
        &self,
        next: GenerationShadowSnapshotV3,
    ) -> Result<GenerationShadowRegistryUpdateV3, GenerationShadowRegistryErrorV3> {
        let mut current = self
            .current
            .write()
            .map_err(|_| GenerationShadowRegistryErrorV3::Poisoned)?;
        let Some(previous) = current.as_ref() else {
            *current = Some(Arc::new(next));
            return Ok(GenerationShadowRegistryUpdateV3::Installed);
        };
        if previous.checkpoint().checkpoint_sha256() == next.checkpoint().checkpoint_sha256() {
            if previous.capture_index_sha256() == next.capture_index_sha256() {
                return Ok(GenerationShadowRegistryUpdateV3::Unchanged);
            }
            *current = Some(Arc::new(next));
            return Ok(GenerationShadowRegistryUpdateV3::Installed);
        }
        validate_generation_checkpoint_transition_v3(previous.checkpoint(), next.checkpoint())
            .map_err(map_transition_error)?;
        *current = Some(Arc::new(next));
        Ok(GenerationShadowRegistryUpdateV3::Installed)
    }

    pub(super) fn clear(&self) -> Result<(), GenerationShadowRegistryErrorV3> {
        let mut current = self
            .current
            .write()
            .map_err(|_| GenerationShadowRegistryErrorV3::Poisoned)?;
        *current = None;
        Ok(())
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn map_transition_error(_: GenerationStoreErrorV3) -> GenerationShadowRegistryErrorV3 {
    GenerationShadowRegistryErrorV3::InvalidTransition
}
