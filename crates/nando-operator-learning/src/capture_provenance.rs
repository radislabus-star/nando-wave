use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use nando_operator_kernel::canonical_json_sha256;

pub const CAPTURE_EVIDENCE_RECEIPT_SCHEMA_V1: &str = "nando.capture-evidence-receipt.v1";
pub const CAPTURE_TRANSITION_BINDING_SCHEMA_V1: &str = "nando.capture-transition-binding.v1";
pub const CAPTURE_COMMITMENT_INDEX_SCHEMA_V1: &str = "nando.capture-commitment-index.v1";
pub const MAX_CAPTURE_RECEIPT_RECORDS: usize = 512;
pub const MAX_CAPTURE_COMMITMENT_INDEX_RECORDS: usize = 16_384;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CaptureRecordCommitment {
    pub sequence: u64,
    pub record_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CaptureTransitionBinding {
    pub schema: String,
    pub sequence: u64,
    pub frame_id_sha256: String,
    pub records_root_sha256: String,
    pub source_record: CaptureRecordCommitment,
    pub record_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CaptureEvidenceReceipt {
    pub schema: String,
    pub records: Vec<CaptureRecordCommitment>,
    pub records_root_sha256: String,
    #[serde(default)]
    pub transition_binding: Option<CaptureTransitionBinding>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CaptureCommitmentIndex {
    pub schema: String,
    pub records: Vec<CaptureRecordCommitment>,
    pub index_sha256: String,
}

#[derive(Serialize)]
struct ReceiptDigest<'a> {
    schema: &'a str,
    records: &'a [CaptureRecordCommitment],
}

#[derive(Serialize)]
struct TransitionBindingDigest<'a> {
    schema: &'a str,
    sequence: u64,
    frame_id_sha256: &'a str,
    records_root_sha256: &'a str,
    source_record_sequence: u64,
    source_record_sha256: &'a str,
}

#[derive(Serialize)]
struct IndexDigest<'a> {
    schema: &'a str,
    records: &'a [CaptureRecordCommitment],
}

impl CaptureEvidenceReceipt {
    pub fn new(records: Vec<CaptureRecordCommitment>) -> Result<Self, &'static str> {
        validate_records(&records, MAX_CAPTURE_RECEIPT_RECORDS, true)?;
        let records_root_sha256 = canonical_json_sha256(&ReceiptDigest {
            schema: CAPTURE_EVIDENCE_RECEIPT_SCHEMA_V1,
            records: &records,
        })
        .map_err(|_| "capture_receipt_digest_failed")?;
        Ok(Self {
            schema: CAPTURE_EVIDENCE_RECEIPT_SCHEMA_V1.to_owned(),
            records,
            records_root_sha256,
            transition_binding: None,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != CAPTURE_EVIDENCE_RECEIPT_SCHEMA_V1 {
            return Err("capture_receipt_schema_invalid");
        }
        validate_records(&self.records, MAX_CAPTURE_RECEIPT_RECORDS, true)?;
        let expected = canonical_json_sha256(&ReceiptDigest {
            schema: &self.schema,
            records: &self.records,
        })
        .map_err(|_| "capture_receipt_digest_failed")?;
        if expected != self.records_root_sha256 {
            return Err("capture_receipt_digest_mismatch");
        }
        if let Some(binding) = &self.transition_binding {
            binding.validate(self)?;
        }
        Ok(())
    }

    pub fn bind_transition(
        &mut self,
        binding: CaptureTransitionBinding,
    ) -> Result<(), &'static str> {
        binding.validate(self)?;
        self.transition_binding = Some(binding);
        Ok(())
    }
}

impl CaptureTransitionBinding {
    pub fn new(
        sequence: u64,
        frame_id_sha256: &str,
        receipt: &CaptureEvidenceReceipt,
    ) -> Result<Self, &'static str> {
        receipt.validate()?;
        let source_record = receipt
            .records
            .last()
            .cloned()
            .ok_or("capture_binding_source_record_missing")?;
        let record_sha256 = canonical_json_sha256(&TransitionBindingDigest {
            schema: CAPTURE_TRANSITION_BINDING_SCHEMA_V1,
            sequence,
            frame_id_sha256,
            records_root_sha256: &receipt.records_root_sha256,
            source_record_sequence: source_record.sequence,
            source_record_sha256: &source_record.record_sha256,
        })
        .map_err(|_| "capture_transition_binding_digest_failed")?;
        let binding = Self {
            schema: CAPTURE_TRANSITION_BINDING_SCHEMA_V1.to_owned(),
            sequence,
            frame_id_sha256: frame_id_sha256.to_owned(),
            records_root_sha256: receipt.records_root_sha256.clone(),
            source_record,
            record_sha256,
        };
        binding.validate(receipt)?;
        Ok(binding)
    }

    pub fn validate(&self, receipt: &CaptureEvidenceReceipt) -> Result<(), &'static str> {
        if self.schema != CAPTURE_TRANSITION_BINDING_SCHEMA_V1
            || !valid_sha256(&self.frame_id_sha256)
            || !valid_sha256(&self.records_root_sha256)
            || !valid_sha256(&self.source_record.record_sha256)
            || !valid_sha256(&self.record_sha256)
            || self.records_root_sha256 != receipt.records_root_sha256
            || !receipt.records.contains(&self.source_record)
        {
            return Err("capture_transition_binding_invalid");
        }
        self.verify_digest()
    }

    pub fn verify_digest(&self) -> Result<(), &'static str> {
        let expected = canonical_json_sha256(&TransitionBindingDigest {
            schema: &self.schema,
            sequence: self.sequence,
            frame_id_sha256: &self.frame_id_sha256,
            records_root_sha256: &self.records_root_sha256,
            source_record_sequence: self.source_record.sequence,
            source_record_sha256: &self.source_record.record_sha256,
        })
        .map_err(|_| "capture_transition_binding_digest_failed")?;
        if expected != self.record_sha256 {
            return Err("capture_transition_binding_digest_mismatch");
        }
        Ok(())
    }
}

impl CaptureCommitmentIndex {
    pub fn new(records: Vec<CaptureRecordCommitment>) -> Result<Self, &'static str> {
        validate_records(&records, MAX_CAPTURE_COMMITMENT_INDEX_RECORDS, false)?;
        let index_sha256 = canonical_json_sha256(&IndexDigest {
            schema: CAPTURE_COMMITMENT_INDEX_SCHEMA_V1,
            records: &records,
        })
        .map_err(|_| "capture_index_digest_failed")?;
        Ok(Self {
            schema: CAPTURE_COMMITMENT_INDEX_SCHEMA_V1.to_owned(),
            records,
            index_sha256,
        })
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != CAPTURE_COMMITMENT_INDEX_SCHEMA_V1 {
            return Err("capture_index_schema_invalid");
        }
        validate_records(&self.records, MAX_CAPTURE_COMMITMENT_INDEX_RECORDS, false)?;
        let expected = canonical_json_sha256(&IndexDigest {
            schema: &self.schema,
            records: &self.records,
        })
        .map_err(|_| "capture_index_digest_failed")?;
        if expected != self.index_sha256 {
            return Err("capture_index_digest_mismatch");
        }
        Ok(())
    }

    pub fn verify_receipt(&self, receipt: &CaptureEvidenceReceipt) -> Result<(), &'static str> {
        self.validate()?;
        receipt.validate()?;
        let indexed = self
            .records
            .iter()
            .map(|record| (record.sequence, record.record_sha256.as_str()))
            .collect::<BTreeMap<_, _>>();
        for record in &receipt.records {
            if indexed.get(&record.sequence).copied() != Some(record.record_sha256.as_str()) {
                return Err("capture_receipt_record_not_indexed");
            }
        }
        Ok(())
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_records(
    records: &[CaptureRecordCommitment],
    limit: usize,
    require_nonempty: bool,
) -> Result<(), &'static str> {
    if (require_nonempty && records.is_empty()) || records.len() > limit {
        return Err("capture_records_cardinality_invalid");
    }
    let mut previous = None;
    for record in records {
        if !valid_sha256(&record.record_sha256)
            || previous.is_some_and(|sequence| sequence >= record.sequence)
        {
            return Err("capture_record_invalid");
        }
        previous = Some(record.sequence);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records() -> Vec<CaptureRecordCommitment> {
        vec![
            CaptureRecordCommitment {
                sequence: 7,
                record_sha256: "a".repeat(64),
            },
            CaptureRecordCommitment {
                sequence: 8,
                record_sha256: "b".repeat(64),
            },
        ]
    }

    #[test]
    fn capture_index_accepts_exact_receipt_and_rejects_tampering() {
        let index = CaptureCommitmentIndex::new(records()).expect("capture index");
        let receipt = CaptureEvidenceReceipt::new(records()).expect("capture receipt");
        assert_eq!(index.verify_receipt(&receipt), Ok(()));

        let mut tampered = receipt;
        tampered.records[1].record_sha256 = "c".repeat(64);
        assert_eq!(
            index.verify_receipt(&tampered),
            Err("capture_receipt_digest_mismatch")
        );
    }

    #[test]
    fn empty_capture_index_is_valid_before_first_event() {
        let index = CaptureCommitmentIndex::new(Vec::new()).expect("empty startup index");
        assert_eq!(index.validate(), Ok(()));
    }
}
