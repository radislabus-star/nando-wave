use std::path::{Path, PathBuf};

use nando_operator_learning::GenerationShadowReceiptLedgerV3;

pub const GENERATION_SHADOW_STORE_SLOT_A_FILE_V3: &str = "generation-shadow-slot-a.nwsl";
pub const GENERATION_SHADOW_STORE_SLOT_B_FILE_V3: &str = "generation-shadow-slot-b.nwsl";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowStoreSlotV3 {
    A,
    B,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowStoreErrorV3 {
    InvalidRoot,
    Io,
    InvalidLedger,
    CommittedSlotCorrupt,
    NonMonotonicPublish,
    EvidenceRollback,
    ForeignGeneration,
    SlotConflict,
}

pub struct GenerationShadowStoreRestoreV3 {
    pub(super) ledger: Option<GenerationShadowReceiptLedgerV3>,
    pub(super) active_slot: Option<GenerationShadowStoreSlotV3>,
    pub(super) quarantined_files: Box<[PathBuf]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationShadowStorePublishV3 {
    slot: GenerationShadowStoreSlotV3,
    publish_sequence: u64,
    ledger_sha256: String,
}

impl GenerationShadowStoreSlotV3 {
    pub(super) const fn file_name(self) -> &'static str {
        match self {
            Self::A => GENERATION_SHADOW_STORE_SLOT_A_FILE_V3,
            Self::B => GENERATION_SHADOW_STORE_SLOT_B_FILE_V3,
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

impl GenerationShadowStoreRestoreV3 {
    #[must_use]
    pub const fn ledger(&self) -> Option<&GenerationShadowReceiptLedgerV3> {
        self.ledger.as_ref()
    }

    #[must_use]
    pub const fn active_slot(&self) -> Option<GenerationShadowStoreSlotV3> {
        self.active_slot
    }

    #[must_use]
    pub fn quarantined_files(&self) -> &[PathBuf] {
        &self.quarantined_files
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl GenerationShadowStorePublishV3 {
    pub(super) fn new(
        slot: GenerationShadowStoreSlotV3,
        publish_sequence: u64,
        ledger_sha256: String,
    ) -> Self {
        Self {
            slot,
            publish_sequence,
            ledger_sha256,
        }
    }

    #[must_use]
    pub const fn slot(&self) -> GenerationShadowStoreSlotV3 {
        self.slot
    }

    #[must_use]
    pub const fn publish_sequence(&self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub fn ledger_sha256(&self) -> &str {
        &self.ledger_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
