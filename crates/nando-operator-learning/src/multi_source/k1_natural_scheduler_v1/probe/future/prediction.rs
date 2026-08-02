use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

const K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1: &str = "nando.k1-future-prediction-receipt.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1FuturePredictionReceiptV1 {
    pub schema: String,
    pub prediction_root_sha256: String,
    pub contract_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub identification_freeze_root_sha256: String,
    pub semantic_class_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub provider_capture_request_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub pre_action_binding_root_sha256: String,
    pub predicted_symbolic_action_root_sha256: String,
    pub capture_sequence: u64,
    pub topology_captured_at_unix_ms: u64,
    pub predicted_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1FuturePredictionReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        contract_root_sha256: String,
        candidate_freeze_root_sha256: String,
        identification_freeze_root_sha256: String,
        semantic_class_root_sha256: String,
        topology_commitment_root_sha256: String,
        provider_capture_request_root_sha256: String,
        turn_intent_id_sha256: String,
        pre_action_binding_root_sha256: String,
        canonical_program_root_sha256: &str,
        capture_sequence: u64,
        topology_captured_at_unix_ms: u64,
        predicted_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        let predicted_symbolic_action_root_sha256 = canonical_json_sha256(&(
            "nando.k1-symbolic-action-prediction.v1",
            canonical_program_root_sha256,
            pre_action_binding_root_sha256.as_str(),
        ))?;
        let mut receipt = Self {
            schema: K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1.to_owned(),
            prediction_root_sha256: String::new(),
            contract_root_sha256,
            candidate_freeze_root_sha256,
            identification_freeze_root_sha256,
            semantic_class_root_sha256,
            topology_commitment_root_sha256,
            provider_capture_request_root_sha256,
            turn_intent_id_sha256,
            pre_action_binding_root_sha256,
            predicted_symbolic_action_root_sha256,
            capture_sequence,
            topology_captured_at_unix_ms,
            predicted_at_unix_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.prediction_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1
            || ![
                self.prediction_root_sha256.as_str(),
                self.contract_root_sha256.as_str(),
                self.candidate_freeze_root_sha256.as_str(),
                self.identification_freeze_root_sha256.as_str(),
                self.semantic_class_root_sha256.as_str(),
                self.topology_commitment_root_sha256.as_str(),
                self.provider_capture_request_root_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.pre_action_binding_root_sha256.as_str(),
                self.predicted_symbolic_action_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.capture_sequence == 0
            || self.topology_captured_at_unix_ms == 0
            || self.predicted_at_unix_nanos
                < self.topology_captured_at_unix_ms.saturating_mul(1_000_000)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.prediction_root_sha256 != self.expected_root()?
        {
            return Err("k1_future_prediction_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1,
            self.contract_root_sha256.as_str(),
            self.candidate_freeze_root_sha256.as_str(),
            self.identification_freeze_root_sha256.as_str(),
            self.semantic_class_root_sha256.as_str(),
            self.topology_commitment_root_sha256.as_str(),
            self.provider_capture_request_root_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.pre_action_binding_root_sha256.as_str(),
            self.predicted_symbolic_action_root_sha256.as_str(),
            self.capture_sequence,
            self.topology_captured_at_unix_ms,
            self.predicted_at_unix_nanos,
            false,
            false,
        ))
    }
}
