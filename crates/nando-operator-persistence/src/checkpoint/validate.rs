use std::collections::BTreeMap;

use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::{GenerationEvidenceLedgerV3, GenerationLearningOutcomeV3};
use nando_operator_proof::{
    generation_receipt_v3::GenerationVerifierReceiptV3,
    independent_verifier_v3::IndependentVerifierReceiptV3,
};
use nando_operator_runtime::decode_operator_generation_restart_bundle_v3;

use super::{
    GENERATION_CHECKPOINT_SCHEMA_V3, GenerationCheckpointErrorV3,
    RestoredGenerationCheckpointPartsV3, RestoredGenerationReceiptPairV3,
    wire::{GenerationCheckpointWireV3, checkpoint_digest_v3, receipt_set_digest_v3},
};

pub(super) fn validate_checkpoint_wire_v3(
    wire: &GenerationCheckpointWireV3,
) -> Result<RestoredGenerationCheckpointPartsV3, GenerationCheckpointErrorV3> {
    if wire.schema != GENERATION_CHECKPOINT_SCHEMA_V3
        || wire.publish_sequence == 0
        || wire.raw_payloads_persisted != 0
        || wire.execution_authority
    {
        return Err(GenerationCheckpointErrorV3::InvalidCheckpoint);
    }
    let generation = decode_operator_generation_restart_bundle_v3(&wire.generation_bundle_bytes)
        .map_err(|_| GenerationCheckpointErrorV3::InvalidGenerationBundle)?;
    if wire.generation_id_sha256 != generation.manifest().generation_id_sha256()
        || wire.generation_bundle_sha256 != generation.bundle_sha256()
        || checkpoint_digest_v3(wire)? != wire.checkpoint_sha256
    {
        return Err(GenerationCheckpointErrorV3::GenerationMismatch);
    }
    let ledger = GenerationEvidenceLedgerV3::from_canonical_bytes(
        &wire.evidence_ledger_bytes,
        generation.manifest(),
    )
    .map_err(|_| GenerationCheckpointErrorV3::InvalidEvidenceLedger)?;
    if ledger
        .evidence_root_sha256()
        .map_err(|_| GenerationCheckpointErrorV3::InvalidEvidenceLedger)?
        != wire.evidence_root_sha256
        || receipt_set_digest_v3(&wire.generation_id_sha256, &wire.receipts)?
            != wire.receipt_set_sha256
    {
        return Err(GenerationCheckpointErrorV3::InvalidReceiptSet);
    }
    let receipts = restore_receipts(wire, generation.manifest())?;
    join_receipts_to_ledger(&ledger, &receipts)?;
    Ok(RestoredGenerationCheckpointPartsV3 {
        publish_sequence: wire.publish_sequence,
        generation,
        ledger,
        receipts,
        evidence_root_sha256: wire.evidence_root_sha256.clone(),
        receipt_set_sha256: wire.receipt_set_sha256.clone(),
        checkpoint_sha256: wire.checkpoint_sha256.clone(),
    })
}

fn restore_receipts(
    wire: &GenerationCheckpointWireV3,
    manifest: &nando_operator_kernel::OperatorGenerationManifestV3,
) -> Result<Vec<RestoredGenerationReceiptPairV3>, GenerationCheckpointErrorV3> {
    let mut restored = Vec::with_capacity(wire.receipts.len());
    let mut previous_sequence = None;
    for receipt in &wire.receipts {
        if previous_sequence.is_some_and(|previous| previous >= receipt.capture_sequence) {
            return Err(GenerationCheckpointErrorV3::InvalidReceiptSet);
        }
        let f6 = IndependentVerifierReceiptV3::from_canonical_bytes(&receipt.f6_receipt_bytes)
            .map_err(|_| GenerationCheckpointErrorV3::InvalidVerifierReceipt)?;
        if receipt.f6_receipt_sha256 != f6.receipt_sha256() {
            return Err(GenerationCheckpointErrorV3::InvalidVerifierReceipt);
        }
        let generation_receipt = GenerationVerifierReceiptV3::from_canonical_bytes(
            &receipt.generation_receipt_bytes,
            manifest,
            &f6,
        )
        .map_err(|_| GenerationCheckpointErrorV3::InvalidGenerationReceipt)?;
        if receipt.capture_sequence != generation_receipt.capture_sequence()
            || receipt.generation_receipt_sha256 != generation_receipt.generation_receipt_sha256()
            || sha256_bytes(&receipt.f6_receipt_bytes)
                != sha256_bytes(
                    &f6.canonical_bytes()
                        .map_err(|_| GenerationCheckpointErrorV3::InvalidVerifierReceipt)?,
                )
        {
            return Err(GenerationCheckpointErrorV3::InvalidReceiptSet);
        }
        previous_sequence = Some(receipt.capture_sequence);
        restored.push(RestoredGenerationReceiptPairV3::new(f6, generation_receipt));
    }
    Ok(restored)
}

fn join_receipts_to_ledger(
    ledger: &GenerationEvidenceLedgerV3,
    receipts: &[RestoredGenerationReceiptPairV3],
) -> Result<(), GenerationCheckpointErrorV3> {
    let by_root = receipts
        .iter()
        .map(|pair| (pair.generation_receipt().generation_receipt_sha256(), pair))
        .collect::<BTreeMap<_, _>>();
    let records = ledger.support().iter().chain(ledger.future());
    let mut matched = 0_usize;
    for record in records {
        let observation = record.observation();
        let pair = by_root
            .get(observation.verifier_receipt_root_sha256())
            .ok_or(GenerationCheckpointErrorV3::InvalidReceiptSet)?;
        let receipt = pair.generation_receipt();
        let positive = matches!(
            observation.outcome(),
            GenerationLearningOutcomeV3::VerifiedPass
        );
        if record.partition() != receipt.partition()
            || observation.generation_id_sha256() != receipt.generation_id_sha256()
            || observation.capture_sequence() != receipt.capture_sequence()
            || observation.support_watermark_next_sequence()
                != receipt.support_watermark_next_sequence()
            || observation.support_freeze_sha256() != receipt.support_freeze_sha256()
            || observation.lineage_root_sha256() != receipt.lineage_root_sha256()
            || observation.event_root_sha256() != receipt.event_root_sha256()
            || observation.request_root_sha256() != receipt.f6_request_sha256()
            || positive != receipt.is_verified_pass()
        {
            return Err(GenerationCheckpointErrorV3::InvalidReceiptSet);
        }
        matched = matched.saturating_add(1);
    }
    if matched != receipts.len() || matched != by_root.len() {
        return Err(GenerationCheckpointErrorV3::InvalidReceiptSet);
    }
    Ok(())
}
