use std::path::{Path, PathBuf};

use crate::{RestoredGenerationCheckpointV3, decode_generation_checkpoint_v3};

use super::{
    GenerationStoreErrorV3, GenerationStorePublishV3, GenerationStoreRestoreV3,
    GenerationStoreSlotV3,
    io::{
        prepare_store_root, quarantine_file, quarantine_stale_temporary, read_slot,
        write_slot_atomically,
    },
};

pub struct GenerationCheckpointStoreV3 {
    root: PathBuf,
}

struct SlotCandidateV3 {
    slot: GenerationStoreSlotV3,
    checkpoint: RestoredGenerationCheckpointV3,
}

impl GenerationCheckpointStoreV3 {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, GenerationStoreErrorV3> {
        let root = root.into();
        prepare_store_root(&root)?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn restore(&self) -> Result<GenerationStoreRestoreV3, GenerationStoreErrorV3> {
        let mut quarantined = Vec::new();
        for slot in [GenerationStoreSlotV3::A, GenerationStoreSlotV3::B] {
            if let Some(path) = quarantine_stale_temporary(&self.root, slot)? {
                quarantined.push(path);
            }
        }
        let mut candidates = Vec::new();
        for slot in [GenerationStoreSlotV3::A, GenerationStoreSlotV3::B] {
            match read_slot(&self.root, slot) {
                Ok(Some(bytes)) => match decode_generation_checkpoint_v3(&bytes) {
                    Ok(checkpoint) => candidates.push(SlotCandidateV3 { slot, checkpoint }),
                    Err(_) => {
                        quarantined.push(quarantine_file(&self.root, &slot.path(&self.root))?)
                    }
                },
                Ok(None) => {}
                Err(GenerationStoreErrorV3::InvalidCheckpoint) => {
                    quarantined.push(quarantine_file(&self.root, &slot.path(&self.root))?);
                }
                Err(error) => return Err(error),
            }
        }
        candidates.sort_by_key(|candidate| candidate.checkpoint.publish_sequence());
        if candidates.len() == 2 {
            let same_publish = candidates[0].checkpoint.publish_sequence()
                == candidates[1].checkpoint.publish_sequence();
            if same_publish
                && candidates[0].checkpoint.canonical_bytes()
                    != candidates[1].checkpoint.canonical_bytes()
            {
                return Err(GenerationStoreErrorV3::SlotConflict);
            }
            if !same_publish
                && validate_generation_checkpoint_transition_v3(
                    &candidates[0].checkpoint,
                    &candidates[1].checkpoint,
                )
                .is_err()
            {
                let invalid = candidates
                    .pop()
                    .ok_or(GenerationStoreErrorV3::SlotConflict)?;
                quarantined.push(quarantine_file(&self.root, &invalid.slot.path(&self.root))?);
            }
        } else if candidates.len() > 2 {
            return Err(GenerationStoreErrorV3::SlotConflict);
        }
        let chosen = candidates.pop();
        let recovered_previous = !quarantined.is_empty();
        let (checkpoint, active_slot) = chosen
            .map(|candidate| (Some(candidate.checkpoint), Some(candidate.slot)))
            .unwrap_or((None, None));
        Ok(GenerationStoreRestoreV3 {
            checkpoint,
            active_slot,
            quarantined_files: quarantined.into_boxed_slice(),
            recovered_previous,
        })
    }

    pub fn publish(
        &self,
        checkpoint_bytes: &[u8],
    ) -> Result<GenerationStorePublishV3, GenerationStoreErrorV3> {
        let next = decode_generation_checkpoint_v3(checkpoint_bytes)
            .map_err(|_| GenerationStoreErrorV3::InvalidCheckpoint)?;
        let current = self.restore()?;
        if let Some(previous) = current.checkpoint() {
            validate_generation_checkpoint_transition_v3(previous, &next)?;
        }
        let slot = current
            .active_slot()
            .map_or(GenerationStoreSlotV3::A, GenerationStoreSlotV3::other);
        write_slot_atomically(&self.root, slot, checkpoint_bytes)?;
        let persisted =
            read_slot(&self.root, slot)?.ok_or(GenerationStoreErrorV3::InvalidCheckpoint)?;
        let restored = decode_generation_checkpoint_v3(&persisted)
            .map_err(|_| GenerationStoreErrorV3::InvalidCheckpoint)?;
        if restored.canonical_bytes() != checkpoint_bytes {
            return Err(GenerationStoreErrorV3::InvalidCheckpoint);
        }
        Ok(GenerationStorePublishV3::new(
            slot,
            restored.publish_sequence(),
            restored.checkpoint_sha256().to_owned(),
        ))
    }
}

pub fn validate_generation_checkpoint_transition_v3(
    current: &RestoredGenerationCheckpointV3,
    next: &RestoredGenerationCheckpointV3,
) -> Result<(), GenerationStoreErrorV3> {
    if next.publish_sequence() != current.publish_sequence().saturating_add(1) {
        return Err(GenerationStoreErrorV3::NonMonotonicPublish);
    }
    let current_manifest = current.generation().manifest();
    let next_manifest = next.generation().manifest();
    if current_manifest.generation_id_sha256() == next_manifest.generation_id_sha256() {
        if current.generation().bundle_sha256() != next.generation().bundle_sha256()
            || !same_generation_evidence_extends(current, next)
        {
            return Err(GenerationStoreErrorV3::EvidenceRollback);
        }
        return Ok(());
    }
    if next_manifest.sequence() != current_manifest.sequence().saturating_add(1)
        || next_manifest.parent_generation_id_sha256()
            != Some(current_manifest.generation_id_sha256())
    {
        return Err(GenerationStoreErrorV3::NonMonotonicGeneration);
    }
    Ok(())
}

fn same_generation_evidence_extends(
    current: &RestoredGenerationCheckpointV3,
    next: &RestoredGenerationCheckpointV3,
) -> bool {
    let current_ledger = current.ledger();
    let next_ledger = next.ledger();
    let support_extends = next_ledger.support().starts_with(current_ledger.support());
    let future_extends = next_ledger.future().starts_with(current_ledger.future());
    let freeze_extends = match (current_ledger.freeze(), next_ledger.freeze()) {
        (None, _) => true,
        (Some(current), Some(next)) => current == next,
        (Some(_), None) => false,
    };
    let current_receipts = current
        .receipts()
        .iter()
        .map(|pair| pair.generation_receipt().generation_receipt_sha256());
    let next_receipts = next
        .receipts()
        .iter()
        .map(|pair| pair.generation_receipt().generation_receipt_sha256());
    support_extends
        && future_extends
        && freeze_extends
        && next_receipts
            .clone()
            .take(current.receipts().len())
            .eq(current_receipts)
        && next.receipts().len() >= current.receipts().len()
}
