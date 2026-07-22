use std::collections::BTreeSet;

use nando_operator_kernel::canonical_json_sha256;
use serde::{Deserialize, Serialize};

use super::{
    GENERATION_CAPTURE_INDEX_MAX_BYTES_V3, GENERATION_CAPTURE_INDEX_MAX_RECORDS_V3,
    GenerationCaptureCommitmentV3, GenerationCaptureErrorV3,
};

pub const GENERATION_CAPTURE_INDEX_SCHEMA_V3: &str = "nando.generation-capture-index.v3.f7";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationCaptureIndexV3 {
    schema: String,
    records: Vec<GenerationCaptureCommitmentV3>,
    index_sha256: String,
}

impl GenerationCaptureIndexV3 {
    pub fn new(
        mut records: Vec<GenerationCaptureCommitmentV3>,
    ) -> Result<Self, GenerationCaptureErrorV3> {
        records.sort_by_key(GenerationCaptureCommitmentV3::capture_sequence);
        validate_records(&records)?;
        let index_sha256 = index_digest(&records)?;
        Ok(Self {
            schema: GENERATION_CAPTURE_INDEX_SCHEMA_V3.to_owned(),
            records,
            index_sha256,
        })
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, GenerationCaptureErrorV3> {
        if bytes.len() > GENERATION_CAPTURE_INDEX_MAX_BYTES_V3 {
            return Err(GenerationCaptureErrorV3::BudgetExhausted);
        }
        let index: Self =
            serde_cbor::from_slice(bytes).map_err(|_| GenerationCaptureErrorV3::InvalidIndex)?;
        index.validate()?;
        if index.canonical_bytes()?.as_ref() != bytes {
            return Err(GenerationCaptureErrorV3::InvalidIndex);
        }
        Ok(index)
    }

    pub fn canonical_bytes(&self) -> Result<Box<[u8]>, GenerationCaptureErrorV3> {
        self.validate()?;
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| GenerationCaptureErrorV3::Serialization)?;
        if bytes.len() > GENERATION_CAPTURE_INDEX_MAX_BYTES_V3 {
            return Err(GenerationCaptureErrorV3::BudgetExhausted);
        }
        Ok(bytes.into_boxed_slice())
    }

    pub fn validate(&self) -> Result<(), GenerationCaptureErrorV3> {
        if self.schema != GENERATION_CAPTURE_INDEX_SCHEMA_V3 {
            return Err(GenerationCaptureErrorV3::InvalidIndex);
        }
        validate_records(&self.records)?;
        if index_digest(&self.records)? != self.index_sha256 {
            return Err(GenerationCaptureErrorV3::InvalidIndex);
        }
        Ok(())
    }

    #[must_use]
    pub fn contains_exact(
        &self,
        capture_sequence: u64,
        lineage_root_sha256: &str,
        event_root_sha256: &str,
        request_root_sha256: &str,
    ) -> bool {
        self.records
            .binary_search_by_key(&capture_sequence, |record| record.capture_sequence())
            .ok()
            .and_then(|index| self.records.get(index))
            .is_some_and(|record| {
                record.lineage_root_sha256() == lineage_root_sha256
                    && record.event_root_sha256() == event_root_sha256
                    && record.request_root_sha256() == request_root_sha256
            })
    }

    #[must_use]
    pub fn records(&self) -> &[GenerationCaptureCommitmentV3] {
        &self.records
    }

    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }
}

fn validate_records(
    records: &[GenerationCaptureCommitmentV3],
) -> Result<(), GenerationCaptureErrorV3> {
    if records.len() > GENERATION_CAPTURE_INDEX_MAX_RECORDS_V3 {
        return Err(GenerationCaptureErrorV3::BudgetExhausted);
    }
    let mut sequences = BTreeSet::new();
    let mut record_roots = BTreeSet::new();
    for record in records {
        record.validate()?;
        if !sequences.insert(record.capture_sequence())
            || !record_roots.insert(record.record_root_sha256())
        {
            return Err(GenerationCaptureErrorV3::DuplicateCommitment);
        }
    }
    if records
        .windows(2)
        .any(|pair| pair[0].capture_sequence() >= pair[1].capture_sequence())
    {
        return Err(GenerationCaptureErrorV3::InvalidIndex);
    }
    Ok(())
}

fn index_digest(
    records: &[GenerationCaptureCommitmentV3],
) -> Result<String, GenerationCaptureErrorV3> {
    canonical_json_sha256(&(GENERATION_CAPTURE_INDEX_SCHEMA_V3, records))
        .map_err(|_| GenerationCaptureErrorV3::Serialization)
}
