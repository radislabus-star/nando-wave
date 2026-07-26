use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::OperatorIdentificationMachineV1;

use super::{FrozenVersionSpaceEnvelopeV1, Ms3FrozenVersionSpaceStateV1};
use crate::multi_source::{
    PreActionTopologyAuditRowV1, source_neutral_t1::pre_action_t1_binding_root,
};

pub const MS3_FUTURE_PREDICTION_SCHEMA_V1: &str = "nando.ms3-future-prediction.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FuturePredictionV1 {
    pub schema: String,
    pub prediction_root_sha256: String,
    pub contract_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub canonical_program_root_sha256: String,
    pub capture_sequence: u64,
    pub topology_root_sha256: String,
    pub request_event_id_sha256: String,
    pub turn_intent_id_sha256: String,
    pub session_lineage_sha256: String,
    pub pre_action_binding_root_sha256: String,
    pub predicted_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

pub fn predict_ms3_unique_law_v1(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    topology: &PreActionTopologyAuditRowV1,
    predicted_at_unix_nanos: u64,
) -> Result<Option<Ms3FuturePredictionV1>, &'static str> {
    frozen
        .validate()
        .map_err(|_| "future_frozen_contract_invalid")?;
    let Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen {
        candidate_freeze_root_sha256,
        ..
    } = &frozen.contract.state
    else {
        return Ok(None);
    };
    let capture_sequence = topology
        .bridge_sequence
        .ok_or("future_capture_sequence_missing")?;
    let lineage = topology
        .session_lineage_sha256
        .as_ref()
        .ok_or("future_lineage_missing")?;
    if capture_sequence < frozen.contract.future_min_sequence
        || lineage == &frozen.contract.session_lineage_sha256
    {
        return Ok(None);
    }
    let machine =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(frozen.machine_checkpoint())
            .map_err(|_| "future_machine_restore_failed")?;
    let freeze = machine.freeze().ok_or("future_candidate_freeze_missing")?;
    if freeze.freeze_root_sha256() != candidate_freeze_root_sha256 {
        return Err("future_candidate_freeze_mismatch");
    }
    let canonical_program_root_sha256 = freeze.canonical_program_root_sha256().to_owned();
    let programs = machine.candidate_programs();
    let program = programs
        .get(&canonical_program_root_sha256)
        .ok_or("future_canonical_program_missing")?;
    let Ok(pre_action_binding_root_sha256) =
        pre_action_t1_binding_root(program, &topology.structure.topology)
    else {
        return Ok(None);
    };
    let mut prediction = Ms3FuturePredictionV1 {
        schema: MS3_FUTURE_PREDICTION_SCHEMA_V1.to_owned(),
        prediction_root_sha256: String::new(),
        contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
        candidate_freeze_root_sha256: candidate_freeze_root_sha256.clone(),
        canonical_program_root_sha256,
        capture_sequence,
        topology_root_sha256: topology.commit.commitment_root_sha256.clone(),
        request_event_id_sha256: topology.structure.request_event_id_sha256.clone(),
        turn_intent_id_sha256: topology.structure.turn_intent_id_sha256.clone(),
        session_lineage_sha256: lineage.clone(),
        pre_action_binding_root_sha256,
        predicted_at_unix_nanos,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    prediction.prediction_root_sha256 = prediction.expected_root()?;
    prediction.validate(frozen)?;
    Ok(Some(prediction))
}

impl Ms3FuturePredictionV1 {
    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS3_FUTURE_PREDICTION_SCHEMA_V1,
            self.contract_root_sha256.as_str(),
            self.candidate_freeze_root_sha256.as_str(),
            self.canonical_program_root_sha256.as_str(),
            self.capture_sequence,
            self.topology_root_sha256.as_str(),
            self.request_event_id_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.pre_action_binding_root_sha256.as_str(),
            self.predicted_at_unix_nanos,
            false,
            false,
        ))
        .map_err(|_| "future_prediction_root_failed")
    }

    pub fn validate(&self, frozen: &FrozenVersionSpaceEnvelopeV1) -> Result<(), &'static str> {
        if self.schema != MS3_FUTURE_PREDICTION_SCHEMA_V1
            || self.contract_root_sha256 != frozen.contract.contract_root_sha256
            || self.capture_sequence < frozen.contract.future_min_sequence
            || self.session_lineage_sha256 == frozen.contract.session_lineage_sha256
            || self.predicted_at_unix_nanos == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || ![
                &self.prediction_root_sha256,
                &self.candidate_freeze_root_sha256,
                &self.canonical_program_root_sha256,
                &self.topology_root_sha256,
                &self.request_event_id_sha256,
                &self.turn_intent_id_sha256,
                &self.session_lineage_sha256,
                &self.pre_action_binding_root_sha256,
            ]
            .into_iter()
            .all(|root| valid_nonzero_sha256(root))
            || self.prediction_root_sha256 != self.expected_root()?
        {
            return Err("future_prediction_invalid");
        }
        Ok(())
    }
}
