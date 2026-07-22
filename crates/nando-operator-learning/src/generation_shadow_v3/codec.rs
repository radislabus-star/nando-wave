use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_bytes, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    GENERATION_SHADOW_LEDGER_MAX_BYTES_V3, GENERATION_SHADOW_LEDGER_SCHEMA_V3,
    GenerationShadowLedgerErrorV3, GenerationShadowReceiptLedgerV3, GenerationShadowReceiptV3,
    ledger::ledger_digest,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct GenerationShadowLedgerWireV3 {
    schema: String,
    generation_id_sha256: String,
    generation_publish_sequence: u64,
    generation_checkpoint_sha256: String,
    publish_sequence: u64,
    receipts: Vec<GenerationShadowReceiptV3>,
    ledger_sha256: String,
    raw_payloads_persisted: u8,
    execution_authority: bool,
}

impl GenerationShadowReceiptLedgerV3 {
    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, GenerationShadowLedgerErrorV3> {
        let bytes = canonical_json_bytes(&GenerationShadowLedgerWireV3 {
            schema: GENERATION_SHADOW_LEDGER_SCHEMA_V3.to_owned(),
            generation_id_sha256: self.generation_id_sha256.clone(),
            generation_publish_sequence: self.generation_publish_sequence,
            generation_checkpoint_sha256: self.generation_checkpoint_sha256.clone(),
            publish_sequence: self.publish_sequence,
            receipts: self.receipts.clone(),
            ledger_sha256: self.ledger_sha256.clone(),
            raw_payloads_persisted: 0,
            execution_authority: false,
        })
        .map_err(|_| GenerationShadowLedgerErrorV3::Serialization)?;
        if bytes.len() > GENERATION_SHADOW_LEDGER_MAX_BYTES_V3 {
            return Err(GenerationShadowLedgerErrorV3::BudgetExhausted);
        }
        Ok(bytes.into_boxed_slice())
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, GenerationShadowLedgerErrorV3> {
        if bytes.len() > GENERATION_SHADOW_LEDGER_MAX_BYTES_V3 {
            return Err(GenerationShadowLedgerErrorV3::BudgetExhausted);
        }
        let wire: GenerationShadowLedgerWireV3 = serde_json::from_slice(bytes)
            .map_err(|_| GenerationShadowLedgerErrorV3::InvalidLedger)?;
        if wire.schema != GENERATION_SHADOW_LEDGER_SCHEMA_V3
            || !valid_nonzero_sha256(&wire.generation_id_sha256)
            || wire.generation_publish_sequence == 0
            || !valid_nonzero_sha256(&wire.generation_checkpoint_sha256)
            || !valid_nonzero_sha256(&wire.ledger_sha256)
            || wire.publish_sequence != wire.receipts.len() as u64
            || wire.raw_payloads_persisted != 0
            || wire.execution_authority
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
        }
        validate_receipt_chain(&wire)?;
        let expected_root = ledger_digest(
            &wire.generation_id_sha256,
            wire.generation_publish_sequence,
            &wire.generation_checkpoint_sha256,
            wire.publish_sequence,
            &wire.receipts,
        )?;
        if expected_root != wire.ledger_sha256 {
            return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
        }
        let mut ledger = Self {
            generation_id_sha256: wire.generation_id_sha256,
            generation_publish_sequence: wire.generation_publish_sequence,
            generation_checkpoint_sha256: wire.generation_checkpoint_sha256,
            publish_sequence: wire.publish_sequence,
            receipts: wire.receipts,
            ledger_sha256: wire.ledger_sha256,
            capture_sequences: BTreeSet::new(),
            event_roots: BTreeSet::new(),
            request_roots: BTreeSet::new(),
            capture_receipt_roots: BTreeSet::new(),
            traffic_receipt_roots: BTreeSet::new(),
            verifier_receipt_roots: BTreeSet::new(),
        };
        for receipt in &ledger.receipts {
            if !ledger.capture_sequences.insert(receipt.capture_sequence)
                || !ledger.event_roots.insert(receipt.capture_event_sha256)
                || !ledger.request_roots.insert(receipt.request_sha256)
                || !ledger
                    .capture_receipt_roots
                    .insert(receipt.capture_receipt_sha256)
                || !ledger
                    .traffic_receipt_roots
                    .insert(receipt.traffic_receipt_sha256.clone())
                || receipt
                    .verifier_receipt_sha256
                    .as_ref()
                    .is_some_and(|root| !ledger.verifier_receipt_roots.insert(root.clone()))
            {
                return Err(GenerationShadowLedgerErrorV3::DuplicateCommitment);
            }
        }
        if ledger.canonical_bytes()?.as_ref() != bytes {
            return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
        }
        Ok(ledger)
    }

    pub fn validate_extension_from(
        &self,
        previous: &Self,
    ) -> Result<(), GenerationShadowLedgerErrorV3> {
        if self.generation_id_sha256 != previous.generation_id_sha256
            || self.generation_publish_sequence != previous.generation_publish_sequence
            || self.generation_checkpoint_sha256 != previous.generation_checkpoint_sha256
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidGeneration);
        }
        if self.publish_sequence != previous.publish_sequence.saturating_add(1)
            || !self.receipts.starts_with(&previous.receipts)
        {
            return Err(GenerationShadowLedgerErrorV3::EvidenceRollback);
        }
        Ok(())
    }
}

fn validate_receipt_chain(
    wire: &GenerationShadowLedgerWireV3,
) -> Result<(), GenerationShadowLedgerErrorV3> {
    let genesis = nando_operator_kernel::canonical_json_sha256(&(
        GENERATION_SHADOW_LEDGER_SCHEMA_V3,
        "genesis",
        wire.generation_id_sha256.as_str(),
        wire.generation_publish_sequence,
        wire.generation_checkpoint_sha256.as_str(),
    ))
    .map_err(|_| GenerationShadowLedgerErrorV3::Serialization)?;
    let mut previous = genesis;
    let mut last_capture_sequence = 0;
    for (ordinal, receipt) in wire.receipts.iter().enumerate() {
        receipt.validate_fields()?;
        validate_embedded_verifier(receipt)?;
        if receipt.ordinal != ordinal as u32
            || receipt.previous_receipt_sha256 != previous
            || receipt.generation_id_sha256 != wire.generation_id_sha256
            || receipt.generation_publish_sequence != wire.generation_publish_sequence
            || receipt.generation_checkpoint_sha256 != wire.generation_checkpoint_sha256
            || receipt.capture_sequence <= last_capture_sequence
            || super::ledger::receipt_digest(receipt)? != receipt.receipt_sha256
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
        }
        previous.clone_from(&receipt.receipt_sha256);
        last_capture_sequence = receipt.capture_sequence;
    }
    Ok(())
}

fn validate_embedded_verifier(
    receipt: &GenerationShadowReceiptV3,
) -> Result<(), GenerationShadowLedgerErrorV3> {
    match (&receipt.verifier_receipt, &receipt.verifier_receipt_sha256) {
        (None, None) => Ok(()),
        (Some(verifier), Some(root)) => {
            let bytes = verifier
                .canonical_bytes()
                .map_err(|_| GenerationShadowLedgerErrorV3::InvalidVerifierReceipt)?;
            let restored = nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3::from_canonical_bytes(&bytes)
                .map_err(|_| GenerationShadowLedgerErrorV3::InvalidVerifierReceipt)?;
            if &restored != verifier
                || verifier.receipt_sha256() != root
                || verifier.request_sha256() != receipt.request_sha256.to_hex()
                || verifier.actor_physical_action_sha256()
                    != receipt.actor_action_sha256.as_deref().unwrap_or_default()
                || verifier.actor_output_sha256()
                    != receipt.actor_output_sha256.as_deref().unwrap_or_default()
            {
                return Err(GenerationShadowLedgerErrorV3::InvalidVerifierReceipt);
            }
            Ok(())
        }
        _ => Err(GenerationShadowLedgerErrorV3::InvalidVerifierReceipt),
    }
}
