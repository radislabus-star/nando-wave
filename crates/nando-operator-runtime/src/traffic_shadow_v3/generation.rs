use std::sync::{Arc, RwLock};

use nando_operator_kernel::canonical_json_sha256;

use super::TrafficShadowGenerationErrorV3;
use crate::{RestoredOperatorGenerationV3, StructuralDispatchIndexV3};

pub struct TrafficShadowGenerationV3 {
    sequence: u64,
    generation_root_sha256: String,
    index: StructuralDispatchIndexV3,
}

impl TrafficShadowGenerationV3 {
    pub fn new(
        sequence: u64,
        index: StructuralDispatchIndexV3,
    ) -> Result<Self, TrafficShadowGenerationErrorV3> {
        if sequence == 0 {
            return Err(TrafficShadowGenerationErrorV3::InvalidSequence);
        }
        let generation_root_sha256 = canonical_json_sha256(&(
            "nando.traffic-shadow-generation.v3",
            sequence,
            index.index_sha256(),
        ))
        .map_err(|_| TrafficShadowGenerationErrorV3::InvalidRoot)?;
        Ok(Self {
            sequence,
            generation_root_sha256,
            index,
        })
    }

    pub fn from_restored_generation(
        generation: &RestoredOperatorGenerationV3,
    ) -> Result<Self, TrafficShadowGenerationErrorV3> {
        let manifest = generation.manifest();
        if manifest.sequence() == 0
            || manifest.components().dispatch_index_sha256 != generation.index().index_sha256()
        {
            return Err(TrafficShadowGenerationErrorV3::GenerationMismatch);
        }
        Ok(Self {
            sequence: manifest.sequence(),
            generation_root_sha256: manifest.generation_id_sha256().to_owned(),
            index: generation.index().clone(),
        })
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn generation_root_sha256(&self) -> &str {
        &self.generation_root_sha256
    }

    #[must_use]
    pub fn index_sha256(&self) -> &str {
        self.index.index_sha256()
    }

    pub(super) const fn index(&self) -> &StructuralDispatchIndexV3 {
        &self.index
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

pub struct TrafficShadowRegistryV3 {
    current: RwLock<Arc<TrafficShadowGenerationV3>>,
}

impl TrafficShadowRegistryV3 {
    #[must_use]
    pub fn new(initial: TrafficShadowGenerationV3) -> Self {
        Self {
            current: RwLock::new(Arc::new(initial)),
        }
    }

    pub fn pin(&self) -> Result<Arc<TrafficShadowGenerationV3>, TrafficShadowGenerationErrorV3> {
        self.current
            .read()
            .map(|current| Arc::clone(&current))
            .map_err(|_| TrafficShadowGenerationErrorV3::RegistryPoisoned)
    }

    pub fn swap(
        &self,
        next: TrafficShadowGenerationV3,
    ) -> Result<Arc<TrafficShadowGenerationV3>, TrafficShadowGenerationErrorV3> {
        let mut current = self
            .current
            .write()
            .map_err(|_| TrafficShadowGenerationErrorV3::RegistryPoisoned)?;
        if next.sequence <= current.sequence {
            return Err(TrafficShadowGenerationErrorV3::NonMonotonicSwap);
        }
        Ok(std::mem::replace(&mut *current, Arc::new(next)))
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
