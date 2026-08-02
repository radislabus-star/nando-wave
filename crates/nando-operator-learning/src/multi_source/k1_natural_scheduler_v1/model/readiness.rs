use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    K1_CANDIDATE_READINESS_MIN_LINEAGES_V1, K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1,
    K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1CandidateReadinessV1 {
    pub schema: String,
    pub readiness_receipt_root_sha256: String,
    pub settled_rows: u64,
    pub verified_rows: u64,
    pub independent_lineages: u64,
    pub minimum_settled_rows: u64,
    pub minimum_verified_rows: u64,
    pub minimum_independent_lineages: u64,
    pub pass: bool,
    pub blocker: String,
}

impl K1CandidateReadinessV1 {
    pub(in super::super) fn seal(
        settled_rows: u64,
        verified_rows: u64,
        independent_lineages: u64,
    ) -> Result<Self, &'static str> {
        let pass = settled_rows >= K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1
            && verified_rows >= K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1
            && independent_lineages >= K1_CANDIDATE_READINESS_MIN_LINEAGES_V1;
        let blocker = if pass {
            ""
        } else if settled_rows < K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1 {
            "settled_evidence_below_freeze_minimum"
        } else if verified_rows < K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1 {
            "verified_evidence_below_freeze_minimum"
        } else {
            "independent_lineages_below_freeze_minimum"
        }
        .to_owned();
        let mut receipt = Self {
            schema: "nando.k1-candidate-readiness-receipt.v1".to_owned(),
            readiness_receipt_root_sha256: String::new(),
            settled_rows,
            verified_rows,
            independent_lineages,
            minimum_settled_rows: K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1,
            minimum_verified_rows: K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1,
            minimum_independent_lineages: K1_CANDIDATE_READINESS_MIN_LINEAGES_V1,
            pass,
            blocker,
        };
        receipt.readiness_receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let pass = self.settled_rows >= self.minimum_settled_rows
            && self.verified_rows >= self.minimum_verified_rows
            && self.independent_lineages >= self.minimum_independent_lineages;
        if self.schema != "nando.k1-candidate-readiness-receipt.v1"
            || self.minimum_settled_rows != K1_CANDIDATE_READINESS_MIN_SETTLED_ROWS_V1
            || self.minimum_verified_rows != K1_CANDIDATE_READINESS_MIN_VERIFIED_ROWS_V1
            || self.minimum_independent_lineages != K1_CANDIDATE_READINESS_MIN_LINEAGES_V1
            || self.pass != pass
            || self.pass != self.blocker.is_empty()
            || !valid_nonzero_sha256(&self.readiness_receipt_root_sha256)
            || self.readiness_receipt_root_sha256 != self.expected_root()?
        {
            return Err("k1_candidate_readiness_invalid");
        }
        Ok(())
    }

    pub fn freeze_ready_at(
        &self,
        evidence_rows: u64,
        first_capture_sequence: u64,
        last_capture_sequence: u64,
        contract_watermark: u64,
    ) -> Result<bool, &'static str> {
        self.validate()?;
        if evidence_rows == 0
            || first_capture_sequence == 0
            || last_capture_sequence < first_capture_sequence
            || contract_watermark < last_capture_sequence
        {
            return Err("k1_candidate_recency_input_invalid");
        }
        let observed_span = last_capture_sequence - first_capture_sequence;
        let maximum_staleness = observed_span.max(evidence_rows);
        let staleness = contract_watermark - last_capture_sequence;
        Ok(self.pass && staleness <= maximum_staleness)
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            self.schema.as_str(),
            self.settled_rows,
            self.verified_rows,
            self.independent_lineages,
            self.minimum_settled_rows,
            self.minimum_verified_rows,
            self.minimum_independent_lineages,
            self.pass,
            self.blocker.as_str(),
        ))
    }
}
