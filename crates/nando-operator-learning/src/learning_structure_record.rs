use nando_operator_kernel::{sha256_bytes, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::{
    LearningEvidenceEnvelopeErrorV1, LearningRequestStructureV1, ProviderRequestCaptureReceiptV3,
};

pub const LEARNING_STRUCTURE_RECORD_SCHEMA_V2: &str = "nando.learning-structure-record.v2";
pub const LEARNING_STRUCTURE_RECORD_MAX_BYTES_V2: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningStructureRecordV2 {
    bridge_epoch_sha256: String,
    bridge_sequence: u64,
    capture_receipt: ProviderRequestCaptureReceiptV3,
    structure: LearningRequestStructureV1,
    record_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningStructureRecordErrorV2 {
    InvalidInput,
    InvalidReceipt,
    InvalidStructure,
    DigestMismatch,
    BudgetExhausted,
    Serialization,
}

#[derive(Deserialize, Serialize)]
struct LearningStructureRecordWireV2(
    String,
    String,
    u64,
    #[serde(with = "serde_bytes")] Vec<u8>,
    #[serde(with = "serde_bytes")] Vec<u8>,
    String,
);

impl LearningStructureRecordV2 {
    pub fn new(
        bridge_epoch_sha256: String,
        bridge_sequence: u64,
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure: LearningRequestStructureV1,
    ) -> Result<Self, LearningStructureRecordErrorV2> {
        if bridge_sequence == 0 || !valid_nonzero_sha256(&bridge_epoch_sha256) {
            return Err(LearningStructureRecordErrorV2::InvalidInput);
        }
        let mut record = Self {
            bridge_epoch_sha256,
            bridge_sequence,
            capture_receipt,
            structure,
            record_sha256: String::new(),
        };
        record.record_sha256 = record.digest()?;
        record.validate()?;
        Ok(record)
    }

    pub fn canonical_cbor(&self) -> Result<Vec<u8>, LearningStructureRecordErrorV2> {
        self.validate()?;
        let bytes = serde_cbor::to_vec(&self.wire()?)
            .map_err(|_| LearningStructureRecordErrorV2::Serialization)?;
        if bytes.len() > LEARNING_STRUCTURE_RECORD_MAX_BYTES_V2 {
            return Err(LearningStructureRecordErrorV2::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_cbor(bytes: &[u8]) -> Result<Self, LearningStructureRecordErrorV2> {
        if bytes.is_empty() || bytes.len() > LEARNING_STRUCTURE_RECORD_MAX_BYTES_V2 {
            return Err(LearningStructureRecordErrorV2::BudgetExhausted);
        }
        let wire: LearningStructureRecordWireV2 = serde_cbor::from_slice(bytes)
            .map_err(|_| LearningStructureRecordErrorV2::Serialization)?;
        if wire.0 != LEARNING_STRUCTURE_RECORD_SCHEMA_V2 {
            return Err(LearningStructureRecordErrorV2::InvalidInput);
        }
        let record = Self {
            bridge_epoch_sha256: wire.1,
            bridge_sequence: wire.2,
            capture_receipt: ProviderRequestCaptureReceiptV3::from_canonical_bytes(&wire.3)
                .map_err(|_| LearningStructureRecordErrorV2::InvalidReceipt)?,
            structure: LearningRequestStructureV1::from_canonical_cbor(&wire.4)
                .map_err(map_structure_error)?,
            record_sha256: wire.5,
        };
        record.validate()?;
        if record.canonical_cbor()?.as_slice() != bytes {
            return Err(LearningStructureRecordErrorV2::InvalidInput);
        }
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), LearningStructureRecordErrorV2> {
        if self.bridge_sequence == 0
            || !valid_nonzero_sha256(&self.bridge_epoch_sha256)
            || !valid_nonzero_sha256(&self.record_sha256)
        {
            return Err(LearningStructureRecordErrorV2::InvalidInput);
        }
        self.capture_receipt
            .canonical_bytes()
            .map_err(|_| LearningStructureRecordErrorV2::InvalidReceipt)?;
        self.structure
            .canonical_cbor()
            .map_err(map_structure_error)?;
        if self.digest()? != self.record_sha256 {
            return Err(LearningStructureRecordErrorV2::DigestMismatch);
        }
        Ok(())
    }

    #[must_use]
    pub fn bridge_epoch_sha256(&self) -> &str {
        &self.bridge_epoch_sha256
    }

    #[must_use]
    pub const fn bridge_sequence(&self) -> u64 {
        self.bridge_sequence
    }

    #[must_use]
    pub const fn capture_receipt(&self) -> &ProviderRequestCaptureReceiptV3 {
        &self.capture_receipt
    }

    #[must_use]
    pub const fn structure(&self) -> &LearningRequestStructureV1 {
        &self.structure
    }

    #[must_use]
    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    fn digest(&self) -> Result<String, LearningStructureRecordErrorV2> {
        let receipt = self
            .capture_receipt
            .canonical_bytes()
            .map_err(|_| LearningStructureRecordErrorV2::InvalidReceipt)?;
        let structure = self
            .structure
            .canonical_cbor()
            .map_err(map_structure_error)?;
        let material = serde_cbor::to_vec(&(
            LEARNING_STRUCTURE_RECORD_SCHEMA_V2,
            &self.bridge_epoch_sha256,
            self.bridge_sequence,
            serde_bytes::Bytes::new(receipt.as_ref()),
            serde_bytes::Bytes::new(&structure),
        ))
        .map_err(|_| LearningStructureRecordErrorV2::Serialization)?;
        Ok(sha256_bytes(&material))
    }

    fn wire(&self) -> Result<LearningStructureRecordWireV2, LearningStructureRecordErrorV2> {
        Ok(LearningStructureRecordWireV2(
            LEARNING_STRUCTURE_RECORD_SCHEMA_V2.to_owned(),
            self.bridge_epoch_sha256.clone(),
            self.bridge_sequence,
            self.capture_receipt
                .canonical_bytes()
                .map_err(|_| LearningStructureRecordErrorV2::InvalidReceipt)?
                .into_vec(),
            self.structure
                .canonical_cbor()
                .map_err(map_structure_error)?,
            self.record_sha256.clone(),
        ))
    }
}

fn map_structure_error(error: LearningEvidenceEnvelopeErrorV1) -> LearningStructureRecordErrorV2 {
    match error {
        LearningEvidenceEnvelopeErrorV1::BudgetExhausted => {
            LearningStructureRecordErrorV2::BudgetExhausted
        }
        LearningEvidenceEnvelopeErrorV1::Serialization => {
            LearningStructureRecordErrorV2::Serialization
        }
        _ => LearningStructureRecordErrorV2::InvalidStructure,
    }
}
