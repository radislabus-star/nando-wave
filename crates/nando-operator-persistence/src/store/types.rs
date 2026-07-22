use std::path::{Path, PathBuf};

use crate::RestoredGenerationCheckpointV3;

pub const GENERATION_STORE_SLOT_A_FILE_V3: &str = "generation-slot-a.nwgc";
pub const GENERATION_STORE_SLOT_B_FILE_V3: &str = "generation-slot-b.nwgc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationStoreSlotV3 {
    A,
    B,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationStoreErrorV3 {
    InvalidRoot,
    Io,
    InvalidCheckpoint,
    NonMonotonicPublish,
    NonMonotonicGeneration,
    EvidenceRollback,
    SlotConflict,
}

pub struct GenerationStoreRestoreV3 {
    pub(super) checkpoint: Option<RestoredGenerationCheckpointV3>,
    pub(super) active_slot: Option<GenerationStoreSlotV3>,
    pub(super) quarantined_files: Box<[PathBuf]>,
    pub(super) recovered_previous: bool,
}

pub struct GenerationStorePublishV3 {
    slot: GenerationStoreSlotV3,
    publish_sequence: u64,
    checkpoint_sha256: String,
}

impl GenerationStoreSlotV3 {
    pub(super) const fn file_name(self) -> &'static str {
        match self {
            Self::A => GENERATION_STORE_SLOT_A_FILE_V3,
            Self::B => GENERATION_STORE_SLOT_B_FILE_V3,
        }
    }

    pub(super) const fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    pub(super) fn path(self, root: &Path) -> PathBuf {
        root.join(self.file_name())
    }

    pub(super) fn temporary_path(self, root: &Path) -> PathBuf {
        root.join(format!(".{}.new", self.file_name()))
    }
}

impl GenerationStoreRestoreV3 {
    #[must_use]
    pub const fn checkpoint(&self) -> Option<&RestoredGenerationCheckpointV3> {
        self.checkpoint.as_ref()
    }

    #[must_use]
    pub const fn active_slot(&self) -> Option<GenerationStoreSlotV3> {
        self.active_slot
    }

    #[must_use]
    pub fn quarantined_files(&self) -> &[PathBuf] {
        &self.quarantined_files
    }

    #[must_use]
    pub const fn recovered_previous(&self) -> bool {
        self.recovered_previous
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl GenerationStorePublishV3 {
    #[must_use]
    pub const fn slot(&self) -> GenerationStoreSlotV3 {
        self.slot
    }

    #[must_use]
    pub const fn publish_sequence(&self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub fn checkpoint_sha256(&self) -> &str {
        &self.checkpoint_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl GenerationStorePublishV3 {
    pub(super) fn new(
        slot: GenerationStoreSlotV3,
        publish_sequence: u64,
        checkpoint_sha256: String,
    ) -> Self {
        Self {
            slot,
            publish_sequence,
            checkpoint_sha256,
        }
    }
}
