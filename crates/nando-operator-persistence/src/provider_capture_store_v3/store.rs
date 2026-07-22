use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use nando_operator_learning::ProviderCaptureIndexV3;

use super::{
    ProviderCaptureSequenceLeaseV3, ProviderCaptureStoreErrorV3, ProviderCaptureStorePublishV3,
    ProviderCaptureStoreRestoreV3, ProviderCaptureStoreSlotV3,
    io::{prepare_root, quarantine_stale_temporary, read_slot, write_slot_atomically},
};

pub struct ProviderCaptureStoreV3 {
    root: PathBuf,
    operation_lock: Mutex<()>,
}

struct SlotCandidateV3 {
    slot: ProviderCaptureStoreSlotV3,
    index: ProviderCaptureIndexV3,
    bytes: Vec<u8>,
}

impl ProviderCaptureStoreV3 {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ProviderCaptureStoreErrorV3> {
        let root = root.into();
        prepare_root(&root)?;
        Ok(Self {
            root,
            operation_lock: Mutex::new(()),
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn restore(&self) -> Result<ProviderCaptureStoreRestoreV3, ProviderCaptureStoreErrorV3> {
        let _guard = self.lock()?;
        self.restore_unlocked()
    }

    pub fn reserve_sequence_lease(
        &self,
    ) -> Result<ProviderCaptureSequenceLeaseV3, ProviderCaptureStoreErrorV3> {
        let _guard = self.lock()?;
        let restored = self.restore_unlocked()?;
        let current = restored
            .index
            .unwrap_or(ProviderCaptureIndexV3::empty().map_err(map_index_error)?);
        let (next, lease) = current.reserve_next_lease().map_err(map_index_error)?;
        self.publish_unlocked(restored.active_slot, Some(&current), &next)?;
        Ok(ProviderCaptureSequenceLeaseV3::new(
            lease.first_sequence(),
            lease.last_sequence(),
            lease.epoch_root_sha256(),
            next.index_sha256(),
        ))
    }

    pub fn publish_index(
        &self,
        next: &ProviderCaptureIndexV3,
    ) -> Result<ProviderCaptureStorePublishV3, ProviderCaptureStoreErrorV3> {
        let _guard = self.lock()?;
        let restored = self.restore_unlocked()?;
        self.publish_unlocked(restored.active_slot, restored.index.as_ref(), next)
    }

    fn publish_unlocked(
        &self,
        active_slot: Option<ProviderCaptureStoreSlotV3>,
        current: Option<&ProviderCaptureIndexV3>,
        next: &ProviderCaptureIndexV3,
    ) -> Result<ProviderCaptureStorePublishV3, ProviderCaptureStoreErrorV3> {
        let empty;
        let previous = if let Some(current) = current {
            current
        } else {
            empty = ProviderCaptureIndexV3::empty().map_err(map_index_error)?;
            &empty
        };
        next.validate_transition_from(previous)
            .map_err(map_index_error)?;
        let bytes = next.canonical_bytes().map_err(map_index_error)?;
        let slot = active_slot.map_or(
            ProviderCaptureStoreSlotV3::A,
            ProviderCaptureStoreSlotV3::other,
        );
        write_slot_atomically(&self.root, slot, &bytes)?;
        let persisted = read_slot(&self.root, slot)?
            .ok_or(ProviderCaptureStoreErrorV3::CommittedSlotCorrupt)?;
        let decoded = ProviderCaptureIndexV3::from_canonical_bytes(&persisted)
            .map_err(|_| ProviderCaptureStoreErrorV3::CommittedSlotCorrupt)?;
        if decoded != *next || persisted.as_slice() != bytes.as_ref() {
            return Err(ProviderCaptureStoreErrorV3::CommittedSlotCorrupt);
        }
        Ok(ProviderCaptureStorePublishV3::new(
            slot,
            decoded.publish_sequence(),
            decoded.index_sha256(),
        ))
    }

    fn restore_unlocked(
        &self,
    ) -> Result<ProviderCaptureStoreRestoreV3, ProviderCaptureStoreErrorV3> {
        let mut quarantined = Vec::new();
        for slot in [ProviderCaptureStoreSlotV3::A, ProviderCaptureStoreSlotV3::B] {
            if let Some(path) = quarantine_stale_temporary(&self.root, slot)? {
                quarantined.push(path);
            }
        }
        let mut candidates = Vec::new();
        for slot in [ProviderCaptureStoreSlotV3::A, ProviderCaptureStoreSlotV3::B] {
            if let Some(bytes) = read_slot(&self.root, slot)? {
                let index = ProviderCaptureIndexV3::from_canonical_bytes(&bytes)
                    .map_err(|_| ProviderCaptureStoreErrorV3::CommittedSlotCorrupt)?;
                candidates.push(SlotCandidateV3 { slot, index, bytes });
            }
        }
        candidates.sort_by_key(|candidate| candidate.index.publish_sequence());
        if candidates.len() == 2 {
            let first = &candidates[0];
            let second = &candidates[1];
            if first.index.publish_sequence() == second.index.publish_sequence() {
                if first.bytes != second.bytes {
                    return Err(ProviderCaptureStoreErrorV3::SlotConflict);
                }
            } else {
                second
                    .index
                    .validate_transition_from(&first.index)
                    .map_err(map_index_error)?;
            }
        }
        let chosen = candidates.pop();
        let (index, active_slot) = chosen
            .map(|candidate| (Some(candidate.index), Some(candidate.slot)))
            .unwrap_or((None, None));
        Ok(ProviderCaptureStoreRestoreV3 {
            index,
            active_slot,
            quarantined_files: quarantined.into_boxed_slice(),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, ()>, ProviderCaptureStoreErrorV3> {
        self.operation_lock
            .lock()
            .map_err(|_| ProviderCaptureStoreErrorV3::Io)
    }
}

fn map_index_error(
    error: nando_operator_learning::ProviderCaptureIndexErrorV3,
) -> ProviderCaptureStoreErrorV3 {
    use nando_operator_learning::ProviderCaptureIndexErrorV3;
    match error {
        ProviderCaptureIndexErrorV3::NonMonotonicPublish => {
            ProviderCaptureStoreErrorV3::NonMonotonicPublish
        }
        ProviderCaptureIndexErrorV3::EvidenceRollback => {
            ProviderCaptureStoreErrorV3::EvidenceRollback
        }
        ProviderCaptureIndexErrorV3::BudgetExhausted
        | ProviderCaptureIndexErrorV3::SequenceOutsideLease => {
            ProviderCaptureStoreErrorV3::SequenceExhausted
        }
        ProviderCaptureIndexErrorV3::DuplicateCommitment
        | ProviderCaptureIndexErrorV3::InvalidIndex
        | ProviderCaptureIndexErrorV3::InvalidReceipt
        | ProviderCaptureIndexErrorV3::Serialization => ProviderCaptureStoreErrorV3::InvalidIndex,
    }
}
