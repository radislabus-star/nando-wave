use std::path::{Path, PathBuf};

use nando_operator_kernel::Sha256CommitmentV3;
use nando_operator_learning::ProviderCaptureIndexV3;

pub const PROVIDER_CAPTURE_STORE_SLOT_A_FILE_V3: &str = "provider-capture-slot-a.nwpc";
pub const PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3: &str = "provider-capture-slot-b.nwpc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderCaptureStoreSlotV3 {
    A,
    B,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderCaptureStoreErrorV3 {
    InvalidRoot,
    Io,
    InvalidIndex,
    CommittedSlotCorrupt,
    NonMonotonicPublish,
    EvidenceRollback,
    SlotConflict,
    SequenceExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderCaptureSequenceLeaseV3 {
    first_sequence: u64,
    last_sequence: u64,
    epoch_root_sha256: Sha256CommitmentV3,
    reserved_index_sha256: Sha256CommitmentV3,
}

pub struct ProviderCaptureStoreRestoreV3 {
    pub(super) index: Option<ProviderCaptureIndexV3>,
    pub(super) active_slot: Option<ProviderCaptureStoreSlotV3>,
    pub(super) quarantined_files: Box<[PathBuf]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderCaptureStorePublishV3 {
    slot: ProviderCaptureStoreSlotV3,
    publish_sequence: u64,
    index_sha256: Sha256CommitmentV3,
}

impl ProviderCaptureStoreSlotV3 {
    pub(super) const fn file_name(self) -> &'static str {
        match self {
            Self::A => PROVIDER_CAPTURE_STORE_SLOT_A_FILE_V3,
            Self::B => PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3,
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

impl ProviderCaptureSequenceLeaseV3 {
    pub(super) const fn new(
        first_sequence: u64,
        last_sequence: u64,
        epoch_root_sha256: Sha256CommitmentV3,
        reserved_index_sha256: Sha256CommitmentV3,
    ) -> Self {
        Self {
            first_sequence,
            last_sequence,
            epoch_root_sha256,
            reserved_index_sha256,
        }
    }

    #[must_use]
    pub const fn first_sequence(self) -> u64 {
        self.first_sequence
    }

    #[must_use]
    pub const fn last_sequence(self) -> u64 {
        self.last_sequence
    }

    #[must_use]
    pub const fn epoch_root_sha256(self) -> Sha256CommitmentV3 {
        self.epoch_root_sha256
    }

    #[must_use]
    pub const fn reserved_index_sha256(self) -> Sha256CommitmentV3 {
        self.reserved_index_sha256
    }

    #[must_use]
    pub const fn execution_authority(self) -> bool {
        false
    }
}

impl ProviderCaptureStoreRestoreV3 {
    #[must_use]
    pub const fn index(&self) -> Option<&ProviderCaptureIndexV3> {
        self.index.as_ref()
    }

    #[must_use]
    pub const fn active_slot(&self) -> Option<ProviderCaptureStoreSlotV3> {
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

impl ProviderCaptureStorePublishV3 {
    pub(super) const fn new(
        slot: ProviderCaptureStoreSlotV3,
        publish_sequence: u64,
        index_sha256: Sha256CommitmentV3,
    ) -> Self {
        Self {
            slot,
            publish_sequence,
            index_sha256,
        }
    }

    #[must_use]
    pub const fn slot(self) -> ProviderCaptureStoreSlotV3 {
        self.slot
    }

    #[must_use]
    pub const fn publish_sequence(self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub const fn index_sha256(self) -> Sha256CommitmentV3 {
        self.index_sha256
    }

    #[must_use]
    pub const fn execution_authority(self) -> bool {
        false
    }
}
