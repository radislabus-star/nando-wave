use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::GenerationEvidenceLedgerV3;
use nando_operator_runtime::decode_operator_generation_restart_bundle_v3;

use super::{
    GENERATION_CHECKPOINT_MAX_BYTES_V3, GENERATION_CHECKPOINT_MAX_RECEIPTS_V3,
    GENERATION_CHECKPOINT_SCHEMA_V3, GenerationCheckpointErrorV3, GenerationCheckpointReceiptRefV3,
    RestoredGenerationCheckpointV3,
    validate::validate_checkpoint_wire_v3,
    wire::{
        GenerationCheckpointReceiptWireV3, GenerationCheckpointWireV3, checkpoint_digest_v3,
        receipt_set_digest_v3,
    },
};

pub fn encode_generation_checkpoint_v3(
    publish_sequence: u64,
    generation_bundle_bytes: &[u8],
    ledger: &GenerationEvidenceLedgerV3,
    receipts: &[GenerationCheckpointReceiptRefV3<'_>],
) -> Result<Box<[u8]>, GenerationCheckpointErrorV3> {
    if publish_sequence == 0 {
        return Err(GenerationCheckpointErrorV3::InvalidPublishSequence);
    }
    if receipts.len() > GENERATION_CHECKPOINT_MAX_RECEIPTS_V3 {
        return Err(GenerationCheckpointErrorV3::BudgetExhausted);
    }
    let generation = decode_operator_generation_restart_bundle_v3(generation_bundle_bytes)
        .map_err(|_| GenerationCheckpointErrorV3::InvalidGenerationBundle)?;
    if ledger.generation_id_sha256() != generation.manifest().generation_id_sha256() {
        return Err(GenerationCheckpointErrorV3::GenerationMismatch);
    }
    let mut receipt_wires = receipts
        .iter()
        .map(|pair| {
            let f6_receipt_bytes = pair
                .f6_receipt
                .canonical_bytes()
                .map_err(|_| GenerationCheckpointErrorV3::InvalidVerifierReceipt)?;
            let generation_receipt_bytes = pair
                .generation_receipt
                .canonical_bytes()
                .map_err(|_| GenerationCheckpointErrorV3::InvalidGenerationReceipt)?;
            Ok(GenerationCheckpointReceiptWireV3 {
                capture_sequence: pair.generation_receipt.capture_sequence(),
                f6_receipt_sha256: pair.f6_receipt.receipt_sha256().to_owned(),
                f6_receipt_bytes,
                generation_receipt_sha256: pair
                    .generation_receipt
                    .generation_receipt_sha256()
                    .to_owned(),
                generation_receipt_bytes,
            })
        })
        .collect::<Result<Vec<_>, GenerationCheckpointErrorV3>>()?;
    receipt_wires.sort_by_key(|receipt| receipt.capture_sequence);
    let generation_id_sha256 = generation.manifest().generation_id_sha256().to_owned();
    let evidence_ledger_bytes = ledger
        .canonical_bytes()
        .map_err(|_| GenerationCheckpointErrorV3::InvalidEvidenceLedger)?;
    let evidence_root_sha256 = ledger
        .evidence_root_sha256()
        .map_err(|_| GenerationCheckpointErrorV3::InvalidEvidenceLedger)?;
    let receipt_set_sha256 = receipt_set_digest_v3(&generation_id_sha256, &receipt_wires)?;
    let mut wire = GenerationCheckpointWireV3 {
        schema: GENERATION_CHECKPOINT_SCHEMA_V3.to_owned(),
        publish_sequence,
        generation_id_sha256,
        generation_bundle_sha256: generation.bundle_sha256().to_owned(),
        generation_bundle_bytes: generation_bundle_bytes.to_vec(),
        evidence_root_sha256,
        evidence_ledger_bytes,
        receipt_set_sha256,
        receipts: receipt_wires,
        checkpoint_sha256: String::new(),
        raw_payloads_persisted: 0,
        execution_authority: false,
    };
    wire.checkpoint_sha256 = checkpoint_digest_v3(&wire)?;
    let bytes = encode_wire(&wire)?;
    let restored = decode_generation_checkpoint_v3(&bytes)?;
    if restored.checkpoint_sha256() != wire.checkpoint_sha256
        || sha256_bytes(restored.canonical_bytes()) != sha256_bytes(&bytes)
    {
        return Err(GenerationCheckpointErrorV3::InvalidCheckpoint);
    }
    Ok(bytes.into_boxed_slice())
}

pub fn decode_generation_checkpoint_v3(
    bytes: &[u8],
) -> Result<RestoredGenerationCheckpointV3, GenerationCheckpointErrorV3> {
    if bytes.len() > GENERATION_CHECKPOINT_MAX_BYTES_V3 {
        return Err(GenerationCheckpointErrorV3::BudgetExhausted);
    }
    let wire: GenerationCheckpointWireV3 = serde_cbor::from_slice(bytes)
        .map_err(|_| GenerationCheckpointErrorV3::InvalidCheckpoint)?;
    if wire.receipts.len() > GENERATION_CHECKPOINT_MAX_RECEIPTS_V3 {
        return Err(GenerationCheckpointErrorV3::BudgetExhausted);
    }
    let parts = validate_checkpoint_wire_v3(&wire)?;
    if encode_wire(&wire)? != bytes {
        return Err(GenerationCheckpointErrorV3::InvalidCheckpoint);
    }
    Ok(RestoredGenerationCheckpointV3::from_parts(
        parts,
        bytes.to_vec().into_boxed_slice(),
    ))
}

fn encode_wire(wire: &GenerationCheckpointWireV3) -> Result<Vec<u8>, GenerationCheckpointErrorV3> {
    let bytes = serde_cbor::to_vec(wire).map_err(|_| GenerationCheckpointErrorV3::Serialization)?;
    if bytes.len() > GENERATION_CHECKPOINT_MAX_BYTES_V3 {
        return Err(GenerationCheckpointErrorV3::BudgetExhausted);
    }
    Ok(bytes)
}
