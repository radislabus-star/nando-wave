use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use nando_operator_learning::GenerationShadowReceiptLedgerV3;

use super::{
    GenerationShadowStoreErrorV3, GenerationShadowStorePublishV3, GenerationShadowStoreRestoreV3,
    GenerationShadowStoreSlotV3,
    io::{
        prepare_root, quarantine_file, quarantine_stale_temporary, read_slot, write_slot_atomically,
    },
};

pub struct GenerationShadowReceiptStoreV3 {
    root: PathBuf,
    operation_lock: Mutex<()>,
}

struct SlotCandidateV3 {
    slot: GenerationShadowStoreSlotV3,
    ledger: GenerationShadowReceiptLedgerV3,
    bytes: Vec<u8>,
}

impl GenerationShadowReceiptStoreV3 {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, GenerationShadowStoreErrorV3> {
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

    pub fn restore(&self) -> Result<GenerationShadowStoreRestoreV3, GenerationShadowStoreErrorV3> {
        let _guard = self.lock()?;
        self.restore_unlocked()
    }

    pub fn publish(
        &self,
        next: &GenerationShadowReceiptLedgerV3,
    ) -> Result<GenerationShadowStorePublishV3, GenerationShadowStoreErrorV3> {
        let _guard = self.lock()?;
        let current = self.restore_unlocked()?;
        match current.ledger() {
            Some(previous) => next
                .validate_extension_from(previous)
                .map_err(map_ledger_error)?,
            None if next.publish_sequence() == 1 => {}
            None => return Err(GenerationShadowStoreErrorV3::NonMonotonicPublish),
        }
        let bytes = next.canonical_bytes().map_err(map_ledger_error)?;
        let slot = current.active_slot().map_or(
            GenerationShadowStoreSlotV3::A,
            GenerationShadowStoreSlotV3::other,
        );
        write_slot_atomically(&self.root, slot, &bytes)?;
        let persisted = read_slot(&self.root, slot)?
            .ok_or(GenerationShadowStoreErrorV3::CommittedSlotCorrupt)?;
        let decoded = GenerationShadowReceiptLedgerV3::from_canonical_bytes(&persisted)
            .map_err(|_| GenerationShadowStoreErrorV3::CommittedSlotCorrupt)?;
        if &decoded != next || persisted.as_slice() != bytes.as_ref() {
            return Err(GenerationShadowStoreErrorV3::CommittedSlotCorrupt);
        }
        Ok(GenerationShadowStorePublishV3::new(
            slot,
            decoded.publish_sequence(),
            decoded.ledger_sha256().to_owned(),
        ))
    }

    fn restore_unlocked(
        &self,
    ) -> Result<GenerationShadowStoreRestoreV3, GenerationShadowStoreErrorV3> {
        let mut quarantined = Vec::new();
        for slot in [
            GenerationShadowStoreSlotV3::A,
            GenerationShadowStoreSlotV3::B,
        ] {
            if let Some(path) = quarantine_stale_temporary(&self.root, slot)? {
                quarantined.push(path);
            }
        }
        let mut candidates = Vec::new();
        for slot in [
            GenerationShadowStoreSlotV3::A,
            GenerationShadowStoreSlotV3::B,
        ] {
            let Some(bytes) = read_slot(&self.root, slot)? else {
                continue;
            };
            match GenerationShadowReceiptLedgerV3::from_canonical_bytes(&bytes) {
                Ok(ledger) => candidates.push(SlotCandidateV3 {
                    slot,
                    ledger,
                    bytes,
                }),
                Err(_) => quarantined.push(quarantine_file(&self.root, &slot.path(&self.root))?),
            }
        }
        candidates.sort_by_key(|candidate| candidate.ledger.publish_sequence());
        if candidates.len() == 2 {
            let first = &candidates[0];
            let second = &candidates[1];
            if first.ledger.publish_sequence() == second.ledger.publish_sequence() {
                if first.bytes != second.bytes {
                    return Err(GenerationShadowStoreErrorV3::SlotConflict);
                }
            } else if second
                .ledger
                .validate_extension_from(&first.ledger)
                .is_err()
            {
                return Err(GenerationShadowStoreErrorV3::EvidenceRollback);
            }
        }
        let chosen = candidates.pop();
        let (ledger, active_slot) = chosen
            .map(|candidate| (Some(candidate.ledger), Some(candidate.slot)))
            .unwrap_or((None, None));
        Ok(GenerationShadowStoreRestoreV3 {
            ledger,
            active_slot,
            quarantined_files: quarantined.into_boxed_slice(),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, ()>, GenerationShadowStoreErrorV3> {
        self.operation_lock
            .lock()
            .map_err(|_| GenerationShadowStoreErrorV3::Io)
    }
}

fn map_ledger_error(
    error: nando_operator_learning::GenerationShadowLedgerErrorV3,
) -> GenerationShadowStoreErrorV3 {
    use nando_operator_learning::GenerationShadowLedgerErrorV3;
    match error {
        GenerationShadowLedgerErrorV3::InvalidGeneration => {
            GenerationShadowStoreErrorV3::ForeignGeneration
        }
        GenerationShadowLedgerErrorV3::EvidenceRollback => {
            GenerationShadowStoreErrorV3::EvidenceRollback
        }
        GenerationShadowLedgerErrorV3::NonMonotonicCapture => {
            GenerationShadowStoreErrorV3::NonMonotonicPublish
        }
        _ => GenerationShadowStoreErrorV3::InvalidLedger,
    }
}
