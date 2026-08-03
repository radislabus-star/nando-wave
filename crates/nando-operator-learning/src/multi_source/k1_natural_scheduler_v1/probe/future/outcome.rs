use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

const K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1: &str = "nando.k1-future-outcome-receipt.v1";
const K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2: &str = "nando.k1-future-outcome-receipt.v2";
const K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3: &str = "nando.k1-future-outcome-receipt.v3";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1FutureOutcomeReceiptV1 {
    pub schema: String,
    pub outcome_root_sha256: String,
    pub prediction_root_sha256: String,
    pub join_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub observed_semantic_action_root_sha256: String,
    pub verifier_receipt_root_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_evidence_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_typed_consequence_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_typed_consequence_root_sha256: Option<String>,
    pub observed_at_unix_nanos: u64,
    pub program_consistent: bool,
    pub independent_verifier_pass: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1FutureOutcomeReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        prediction_root_sha256: String,
        join_root_sha256: String,
        completed_frame_root_sha256: String,
        observed_semantic_action_root_sha256: String,
        verifier_receipt_root_sha256: String,
        observed_at_unix_nanos: u64,
        program_consistent: bool,
        independent_verifier_pass: bool,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1.to_owned(),
            outcome_root_sha256: String::new(),
            prediction_root_sha256,
            join_root_sha256,
            completed_frame_root_sha256,
            observed_semantic_action_root_sha256,
            verifier_receipt_root_sha256,
            program_evidence_root_sha256: None,
            predicted_typed_consequence_root_sha256: None,
            observed_typed_consequence_root_sha256: None,
            observed_at_unix_nanos,
            program_consistent,
            independent_verifier_pass,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.outcome_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal_with_typed_consequence(
        prediction_root_sha256: String,
        join_root_sha256: String,
        completed_frame_root_sha256: String,
        observed_semantic_action_root_sha256: String,
        verifier_receipt_root_sha256: String,
        predicted_typed_consequence_root_sha256: String,
        observed_typed_consequence_root_sha256: String,
        observed_at_unix_nanos: u64,
        independent_verifier_pass: bool,
    ) -> Result<Self, &'static str> {
        let program_consistent =
            predicted_typed_consequence_root_sha256 == observed_typed_consequence_root_sha256;
        let mut receipt = Self::seal(
            prediction_root_sha256,
            join_root_sha256,
            completed_frame_root_sha256,
            observed_semantic_action_root_sha256,
            verifier_receipt_root_sha256,
            observed_at_unix_nanos,
            program_consistent,
            independent_verifier_pass && program_consistent,
        )?;
        receipt.schema = K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3.to_owned();
        receipt.predicted_typed_consequence_root_sha256 =
            Some(predicted_typed_consequence_root_sha256);
        receipt.observed_typed_consequence_root_sha256 =
            Some(observed_typed_consequence_root_sha256);
        receipt.outcome_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal_with_program_evidence(
        prediction_root_sha256: String,
        join_root_sha256: String,
        completed_frame_root_sha256: String,
        observed_semantic_action_root_sha256: String,
        verifier_receipt_root_sha256: String,
        program_evidence_root_sha256: String,
        observed_at_unix_nanos: u64,
        program_consistent: bool,
        independent_verifier_pass: bool,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self::seal(
            prediction_root_sha256,
            join_root_sha256,
            completed_frame_root_sha256,
            observed_semantic_action_root_sha256,
            verifier_receipt_root_sha256,
            observed_at_unix_nanos,
            program_consistent,
            independent_verifier_pass,
        )?;
        receipt.schema = K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2.to_owned();
        receipt.program_evidence_root_sha256 = Some(program_evidence_root_sha256);
        receipt.outcome_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(
            self.schema.as_str(),
            K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1
                | K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2
                | K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3
        ) || ![
            self.outcome_root_sha256.as_str(),
            self.prediction_root_sha256.as_str(),
            self.join_root_sha256.as_str(),
            self.completed_frame_root_sha256.as_str(),
            self.observed_semantic_action_root_sha256.as_str(),
            self.verifier_receipt_root_sha256.as_str(),
        ]
        .into_iter()
        .all(valid_nonzero_sha256)
            || match self.schema.as_str() {
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1 => {
                    self.program_evidence_root_sha256.is_some()
                        || self.predicted_typed_consequence_root_sha256.is_some()
                        || self.observed_typed_consequence_root_sha256.is_some()
                }
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2 => {
                    self.program_evidence_root_sha256
                        .as_deref()
                        .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self.predicted_typed_consequence_root_sha256.is_some()
                        || self.observed_typed_consequence_root_sha256.is_some()
                }
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3 => {
                    self.program_evidence_root_sha256.is_some()
                        || self
                            .predicted_typed_consequence_root_sha256
                            .as_deref()
                            .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self
                            .observed_typed_consequence_root_sha256
                            .as_deref()
                            .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self.program_consistent
                            != (self.predicted_typed_consequence_root_sha256
                                == self.observed_typed_consequence_root_sha256)
                        || self.independent_verifier_pass && !self.program_consistent
                }
                _ => true,
            }
            || self.observed_at_unix_nanos == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err("k1_future_outcome_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        match self.schema.as_str() {
            K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1 => canonical_json_sha256(&(
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V1,
                self.prediction_root_sha256.as_str(),
                self.join_root_sha256.as_str(),
                self.completed_frame_root_sha256.as_str(),
                self.observed_semantic_action_root_sha256.as_str(),
                self.verifier_receipt_root_sha256.as_str(),
                self.observed_at_unix_nanos,
                self.program_consistent,
                self.independent_verifier_pass,
                false,
                false,
            )),
            K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2 => canonical_json_sha256(&(
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V2,
                self.prediction_root_sha256.as_str(),
                self.join_root_sha256.as_str(),
                self.completed_frame_root_sha256.as_str(),
                self.observed_semantic_action_root_sha256.as_str(),
                self.verifier_receipt_root_sha256.as_str(),
                self.program_evidence_root_sha256.as_deref(),
                self.observed_at_unix_nanos,
                self.program_consistent,
                self.independent_verifier_pass,
                false,
                false,
            )),
            K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3 => canonical_json_sha256(&(
                K1_FUTURE_OUTCOME_RECEIPT_SCHEMA_V3,
                self.prediction_root_sha256.as_str(),
                self.join_root_sha256.as_str(),
                self.completed_frame_root_sha256.as_str(),
                self.observed_semantic_action_root_sha256.as_str(),
                self.verifier_receipt_root_sha256.as_str(),
                self.predicted_typed_consequence_root_sha256.as_deref(),
                self.observed_typed_consequence_root_sha256.as_deref(),
                self.observed_at_unix_nanos,
                self.program_consistent,
                self.independent_verifier_pass,
                false,
                false,
            )),
            _ => Err("k1_future_outcome_receipt_schema_invalid"),
        }
    }
}

#[cfg(test)]
mod tests;
