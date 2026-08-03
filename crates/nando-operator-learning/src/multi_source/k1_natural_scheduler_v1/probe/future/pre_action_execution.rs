use nando_operator_kernel::{
    AtomSource, AtomValueType, RelationAtom, RelationFrame, canonical_json_sha256,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

pub const K1_PRE_ACTION_EXECUTION_RECEIPT_SCHEMA_V1: &str =
    "nando.k1-pre-action-execution-receipt.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1PreActionExecutionReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub contract_root_sha256: String,
    pub canonical_program_root_sha256: String,
    pub provider_capture_event_root_sha256: String,
    pub provider_capture_request_root_sha256: String,
    pub turn_intent_id_sha256: String,
    pub complete_pre_action_binding_root_sha256: String,
    pub predicted_typed_consequence_root_sha256: String,
    pub execution_verifier_contract_root_sha256: String,
    pub capture_sequence: u64,
    pub captured_at_unix_ms: u64,
    pub executed_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1PreActionExecutionReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        contract_root_sha256: String,
        canonical_program_root_sha256: String,
        provider_capture_event_root_sha256: String,
        provider_capture_request_root_sha256: String,
        turn_intent_id_sha256: String,
        complete_pre_action_binding_root_sha256: String,
        predicted_typed_consequence_root_sha256: String,
        execution_verifier_contract_root_sha256: String,
        capture_sequence: u64,
        captured_at_unix_ms: u64,
        executed_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        let mut receipt = Self {
            schema: K1_PRE_ACTION_EXECUTION_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            contract_root_sha256,
            canonical_program_root_sha256,
            provider_capture_event_root_sha256,
            provider_capture_request_root_sha256,
            turn_intent_id_sha256,
            complete_pre_action_binding_root_sha256,
            predicted_typed_consequence_root_sha256,
            execution_verifier_contract_root_sha256,
            capture_sequence,
            captured_at_unix_ms,
            executed_at_unix_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_PRE_ACTION_EXECUTION_RECEIPT_SCHEMA_V1
            || ![
                self.receipt_root_sha256.as_str(),
                self.contract_root_sha256.as_str(),
                self.canonical_program_root_sha256.as_str(),
                self.provider_capture_event_root_sha256.as_str(),
                self.provider_capture_request_root_sha256.as_str(),
                self.turn_intent_id_sha256.as_str(),
                self.complete_pre_action_binding_root_sha256.as_str(),
                self.predicted_typed_consequence_root_sha256.as_str(),
                self.execution_verifier_contract_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.capture_sequence == 0
            || self.captured_at_unix_ms == 0
            || self.executed_at_unix_nanos < self.captured_at_unix_ms.saturating_mul(1_000_000)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err("k1_pre_action_execution_receipt_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_PRE_ACTION_EXECUTION_RECEIPT_SCHEMA_V1,
            self.contract_root_sha256.as_str(),
            self.canonical_program_root_sha256.as_str(),
            self.provider_capture_event_root_sha256.as_str(),
            self.provider_capture_request_root_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.complete_pre_action_binding_root_sha256.as_str(),
            self.predicted_typed_consequence_root_sha256.as_str(),
            self.execution_verifier_contract_root_sha256.as_str(),
            self.capture_sequence,
            self.captured_at_unix_ms,
            self.executed_at_unix_nanos,
            false,
            false,
        ))
    }
}

pub fn typed_consequence_root_v1(
    value_type: AtomValueType,
    value_sha256: &str,
) -> Result<String, &'static str> {
    if !valid_nonzero_sha256(value_sha256) {
        return Err("k1_typed_consequence_value_invalid");
    }
    canonical_json_sha256(&("nando.k1-typed-consequence.v1", value_type, value_sha256))
}

pub fn observed_typed_consequence_root_v1(frame: &RelationFrame) -> Result<String, &'static str> {
    let values = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                value_type,
                source: AtomSource::Action,
                value_sha256,
                ..
            } => Some((*value_type, value_sha256.as_str())),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let values = values.into_iter().collect::<Vec<_>>();
    let [(value_type, value_sha256)] = values.as_slice() else {
        return Err("k1_observed_typed_consequence_ambiguous");
    };
    typed_consequence_root_v1(*value_type, value_sha256)
}
