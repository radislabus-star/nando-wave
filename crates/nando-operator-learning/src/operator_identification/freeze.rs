use std::fmt;

use nando_operator_kernel::{
    ProgramSemanticClassIdV1, canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

pub const CANDIDATE_FREEZE_SCHEMA_V1: &str = "nando.operator-candidate-freeze.v1";
pub const CANDIDATE_FREEZE_MAX_BYTES_V1: usize = 8 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateFreezeReceiptV1 {
    schema: String,
    generation_id_sha256: String,
    semantic_class_id: ProgramSemanticClassIdV1,
    canonical_program_root_sha256: String,
    support_evidence_root_sha256: String,
    support_watermark_next_sequence: u64,
    search_completion_root_sha256: String,
    eliminated_class_root_sha256: String,
    applicability_scope_root_sha256: String,
    freeze_root_sha256: String,
    execution_authority: bool,
}

pub(crate) struct CandidateFreezeInputV1 {
    pub generation_id_sha256: String,
    pub semantic_class_id: ProgramSemanticClassIdV1,
    pub canonical_program_root_sha256: String,
    pub support_evidence_root_sha256: String,
    pub support_watermark_next_sequence: u64,
    pub search_completion_root_sha256: String,
    pub eliminated_class_root_sha256: String,
    pub applicability_scope_root_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateFreezeErrorV1 {
    InvalidRoot,
    InvalidWatermark,
    InvalidReceipt,
    BudgetExhausted,
    Serialization,
}

impl fmt::Display for CandidateFreezeErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoot => "candidate freeze contains an invalid root",
            Self::InvalidWatermark => "candidate freeze watermark is invalid",
            Self::InvalidReceipt => "candidate freeze receipt is invalid",
            Self::BudgetExhausted => "candidate freeze exceeds its byte budget",
            Self::Serialization => "candidate freeze serialization failed",
        })
    }
}

impl std::error::Error for CandidateFreezeErrorV1 {}

pub(crate) fn seal_candidate_freeze_v1(
    input: CandidateFreezeInputV1,
) -> Result<CandidateFreezeReceiptV1, CandidateFreezeErrorV1> {
    validate_input(&input)?;
    let freeze_root_sha256 = freeze_digest(&input)?;
    Ok(CandidateFreezeReceiptV1 {
        schema: CANDIDATE_FREEZE_SCHEMA_V1.to_owned(),
        generation_id_sha256: input.generation_id_sha256,
        semantic_class_id: input.semantic_class_id,
        canonical_program_root_sha256: input.canonical_program_root_sha256,
        support_evidence_root_sha256: input.support_evidence_root_sha256,
        support_watermark_next_sequence: input.support_watermark_next_sequence,
        search_completion_root_sha256: input.search_completion_root_sha256,
        eliminated_class_root_sha256: input.eliminated_class_root_sha256,
        applicability_scope_root_sha256: input.applicability_scope_root_sha256,
        freeze_root_sha256,
        execution_authority: false,
    })
}

impl CandidateFreezeReceiptV1 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CandidateFreezeErrorV1> {
        self.validate()?;
        let bytes =
            canonical_json_bytes(self).map_err(|_| CandidateFreezeErrorV1::Serialization)?;
        if bytes.len() > CANDIDATE_FREEZE_MAX_BYTES_V1 {
            return Err(CandidateFreezeErrorV1::BudgetExhausted);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CandidateFreezeErrorV1> {
        if bytes.len() > CANDIDATE_FREEZE_MAX_BYTES_V1 {
            return Err(CandidateFreezeErrorV1::BudgetExhausted);
        }
        let receipt: Self =
            serde_json::from_slice(bytes).map_err(|_| CandidateFreezeErrorV1::InvalidReceipt)?;
        receipt.validate()?;
        if receipt.canonical_bytes()? != bytes {
            return Err(CandidateFreezeErrorV1::InvalidReceipt);
        }
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), CandidateFreezeErrorV1> {
        if self.schema != CANDIDATE_FREEZE_SCHEMA_V1 || self.execution_authority {
            return Err(CandidateFreezeErrorV1::InvalidReceipt);
        }
        let input = CandidateFreezeInputV1 {
            generation_id_sha256: self.generation_id_sha256.clone(),
            semantic_class_id: self.semantic_class_id.clone(),
            canonical_program_root_sha256: self.canonical_program_root_sha256.clone(),
            support_evidence_root_sha256: self.support_evidence_root_sha256.clone(),
            support_watermark_next_sequence: self.support_watermark_next_sequence,
            search_completion_root_sha256: self.search_completion_root_sha256.clone(),
            eliminated_class_root_sha256: self.eliminated_class_root_sha256.clone(),
            applicability_scope_root_sha256: self.applicability_scope_root_sha256.clone(),
        };
        validate_input(&input)?;
        if self.freeze_root_sha256 != freeze_digest(&input)? {
            return Err(CandidateFreezeErrorV1::InvalidReceipt);
        }
        Ok(())
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub const fn semantic_class_id(&self) -> &ProgramSemanticClassIdV1 {
        &self.semantic_class_id
    }

    #[must_use]
    pub fn canonical_program_root_sha256(&self) -> &str {
        &self.canonical_program_root_sha256
    }

    #[must_use]
    pub fn support_evidence_root_sha256(&self) -> &str {
        &self.support_evidence_root_sha256
    }

    #[must_use]
    pub const fn support_watermark_next_sequence(&self) -> u64 {
        self.support_watermark_next_sequence
    }

    #[must_use]
    pub fn applicability_scope_root_sha256(&self) -> &str {
        &self.applicability_scope_root_sha256
    }

    #[must_use]
    pub fn freeze_root_sha256(&self) -> &str {
        &self.freeze_root_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn validate_input(input: &CandidateFreezeInputV1) -> Result<(), CandidateFreezeErrorV1> {
    if input.support_watermark_next_sequence == 0 {
        return Err(CandidateFreezeErrorV1::InvalidWatermark);
    }
    [
        input.generation_id_sha256.as_str(),
        input.semantic_class_id.as_str(),
        input.canonical_program_root_sha256.as_str(),
        input.support_evidence_root_sha256.as_str(),
        input.search_completion_root_sha256.as_str(),
        input.eliminated_class_root_sha256.as_str(),
        input.applicability_scope_root_sha256.as_str(),
    ]
    .into_iter()
    .all(valid_nonzero_sha256)
    .then_some(())
    .ok_or(CandidateFreezeErrorV1::InvalidRoot)
}

fn freeze_digest(input: &CandidateFreezeInputV1) -> Result<String, CandidateFreezeErrorV1> {
    canonical_json_sha256(&(
        CANDIDATE_FREEZE_SCHEMA_V1,
        input.generation_id_sha256.as_str(),
        input.semantic_class_id.as_str(),
        input.canonical_program_root_sha256.as_str(),
        input.support_evidence_root_sha256.as_str(),
        input.support_watermark_next_sequence,
        input.search_completion_root_sha256.as_str(),
        input.eliminated_class_root_sha256.as_str(),
        input.applicability_scope_root_sha256.as_str(),
        false,
    ))
    .map_err(|_| CandidateFreezeErrorV1::Serialization)
}
