use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

const K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1: &str = "nando.k1-future-prediction-receipt.v1";
const K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2: &str = "nando.k1-future-prediction-receipt.v2";

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_action_execution_receipt_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted_typed_consequence_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_verifier_contract_root_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executed_at_unix_nanos: Option<u64>,
    pub capture_sequence: u64,
    pub topology_captured_at_unix_ms: u64,
    pub predicted_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[cfg(test)]
mod tests;

#[derive(Serialize)]
struct K1FuturePredictionDigestV2<'a> {
    schema: &'static str,
    contract_root_sha256: &'a str,
    candidate_freeze_root_sha256: &'a str,
    identification_freeze_root_sha256: &'a str,
    semantic_class_root_sha256: &'a str,
    topology_commitment_root_sha256: &'a str,
    provider_capture_request_root_sha256: &'a str,
    turn_intent_id_sha256: &'a str,
    pre_action_binding_root_sha256: &'a str,
    predicted_symbolic_action_root_sha256: &'a str,
    pre_action_execution_receipt_root_sha256: Option<&'a str>,
    predicted_typed_consequence_root_sha256: Option<&'a str>,
    execution_verifier_contract_root_sha256: Option<&'a str>,
    capture_sequence: u64,
    topology_captured_at_unix_ms: u64,
    executed_at_unix_nanos: Option<u64>,
    predicted_at_unix_nanos: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
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
            pre_action_execution_receipt_root_sha256: None,
            predicted_typed_consequence_root_sha256: None,
            execution_verifier_contract_root_sha256: None,
            executed_at_unix_nanos: None,
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

    #[allow(clippy::too_many_arguments)]
    pub fn seal_typed(
        contract_root_sha256: String,
        candidate_freeze_root_sha256: String,
        identification_freeze_root_sha256: String,
        semantic_class_root_sha256: String,
        topology_commitment_root_sha256: String,
        provider_capture_request_root_sha256: String,
        turn_intent_id_sha256: String,
        complete_pre_action_binding_root_sha256: String,
        canonical_program_root_sha256: &str,
        pre_action_execution_receipt_root_sha256: String,
        predicted_typed_consequence_root_sha256: String,
        execution_verifier_contract_root_sha256: String,
        capture_sequence: u64,
        topology_captured_at_unix_ms: u64,
        executed_at_unix_nanos: u64,
        predicted_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self::seal(
            contract_root_sha256,
            candidate_freeze_root_sha256,
            identification_freeze_root_sha256,
            semantic_class_root_sha256,
            topology_commitment_root_sha256,
            provider_capture_request_root_sha256,
            turn_intent_id_sha256,
            complete_pre_action_binding_root_sha256,
            canonical_program_root_sha256,
            capture_sequence,
            topology_captured_at_unix_ms,
            predicted_at_unix_nanos,
        )?;
        receipt.schema = K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2.to_owned();
        receipt.pre_action_execution_receipt_root_sha256 =
            Some(pre_action_execution_receipt_root_sha256);
        receipt.predicted_typed_consequence_root_sha256 =
            Some(predicted_typed_consequence_root_sha256);
        receipt.execution_verifier_contract_root_sha256 =
            Some(execution_verifier_contract_root_sha256);
        receipt.executed_at_unix_nanos = Some(executed_at_unix_nanos);
        receipt.prediction_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    #[must_use]
    pub fn has_typed_consequence_precommit(&self) -> bool {
        self.schema == K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(
            self.schema.as_str(),
            K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1 | K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2
        ) || ![
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
            || match self.schema.as_str() {
                K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1 => {
                    self.pre_action_execution_receipt_root_sha256.is_some()
                        || self.predicted_typed_consequence_root_sha256.is_some()
                        || self.execution_verifier_contract_root_sha256.is_some()
                        || self.executed_at_unix_nanos.is_some()
                }
                K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2 => {
                    self.pre_action_execution_receipt_root_sha256
                        .as_deref()
                        .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self
                            .predicted_typed_consequence_root_sha256
                            .as_deref()
                            .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self
                            .execution_verifier_contract_root_sha256
                            .as_deref()
                            .is_none_or(|root| !valid_nonzero_sha256(root))
                        || self
                            .executed_at_unix_nanos
                            .is_none_or(|value| value == 0 || value > self.predicted_at_unix_nanos)
                }
                _ => true,
            }
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
        match self.schema.as_str() {
            K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V1 => canonical_json_sha256(&(
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
            )),
            K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2 => {
                canonical_json_sha256(&K1FuturePredictionDigestV2 {
                    schema: K1_FUTURE_PREDICTION_RECEIPT_SCHEMA_V2,
                    contract_root_sha256: self.contract_root_sha256.as_str(),
                    candidate_freeze_root_sha256: self.candidate_freeze_root_sha256.as_str(),
                    identification_freeze_root_sha256: self
                        .identification_freeze_root_sha256
                        .as_str(),
                    semantic_class_root_sha256: self.semantic_class_root_sha256.as_str(),
                    topology_commitment_root_sha256: self.topology_commitment_root_sha256.as_str(),
                    provider_capture_request_root_sha256: self
                        .provider_capture_request_root_sha256
                        .as_str(),
                    turn_intent_id_sha256: self.turn_intent_id_sha256.as_str(),
                    pre_action_binding_root_sha256: self.pre_action_binding_root_sha256.as_str(),
                    predicted_symbolic_action_root_sha256: self
                        .predicted_symbolic_action_root_sha256
                        .as_str(),
                    pre_action_execution_receipt_root_sha256: self
                        .pre_action_execution_receipt_root_sha256
                        .as_deref(),
                    predicted_typed_consequence_root_sha256: self
                        .predicted_typed_consequence_root_sha256
                        .as_deref(),
                    execution_verifier_contract_root_sha256: self
                        .execution_verifier_contract_root_sha256
                        .as_deref(),
                    capture_sequence: self.capture_sequence,
                    topology_captured_at_unix_ms: self.topology_captured_at_unix_ms,
                    executed_at_unix_nanos: self.executed_at_unix_nanos,
                    predicted_at_unix_nanos: self.predicted_at_unix_nanos,
                    authority_ready: false,
                    phase_mutation_allowed: false,
                })
            }
            _ => Err("k1_future_prediction_receipt_schema_invalid"),
        }
    }
}
