use nando_operator_kernel::{RelationFrame, canonical_json_sha256, sha256_bytes};
use serde::{Deserialize, Serialize};

use crate::{OperatorIdentificationMachineV1, OperatorIdentificationStateV1};

use super::{FrozenVersionSpaceEnvelopeV1, Ms3FuturePredictionV1};
use crate::multi_source::{
    TransportBoundJoinedTransitionV1,
    identification::observation_for_transition,
    source_neutral_t1::{pre_action_t1_binding_root, t1_program_consistency_blocker},
};

pub const MS3_INDEPENDENT_FUTURE_RECEIPT_SCHEMA_V1: &str =
    "nando.ms3-independent-future-receipt.v1";
pub const MS3_INDEPENDENT_FUTURE_ENVELOPE_SCHEMA_V1: &str =
    "nando.ms3-independent-future-envelope.v1";
const MAX_FUTURE_CHECKPOINT_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3IndependentFutureVerdictV1 {
    Pass,
    Contradiction,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3IndependentFutureReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub contract_root_sha256: String,
    pub prediction_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub canonical_program_root_sha256: String,
    pub capture_sequence: u64,
    pub topology_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub transport_binding_root_sha256: String,
    pub session_lineage_sha256: String,
    pub verifier_receipt_root_sha256: String,
    pub verdict: Ms3IndependentFutureVerdictV1,
    pub blocker: String,
    pub exact_transfer_parity: bool,
    pub runtime_actor_verifier_parity: bool,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3IndependentFutureEnvelopeV1 {
    pub schema: String,
    pub envelope_root_sha256: String,
    pub receipt: Ms3IndependentFutureReceiptV1,
    pub machine_checkpoint_sha256: String,
    pub machine_checkpoint_bytes: usize,
    machine_checkpoint: Vec<u8>,
}

#[derive(Serialize)]
struct FutureReceiptDigestV1<'a> {
    schema: &'a str,
    contract_root_sha256: &'a str,
    prediction_root_sha256: &'a str,
    candidate_freeze_root_sha256: &'a str,
    canonical_program_root_sha256: &'a str,
    capture_sequence: u64,
    topology_root_sha256: &'a str,
    completed_frame_root_sha256: &'a str,
    terminal_receipt_root_sha256: &'a str,
    transport_binding_root_sha256: &'a str,
    session_lineage_sha256: &'a str,
    verifier_receipt_root_sha256: &'a str,
    verdict: Ms3IndependentFutureVerdictV1,
    blocker: &'a str,
    exact_transfer_parity: bool,
    runtime_actor_verifier_parity: bool,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

pub fn seal_ms3_independent_future_v1(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    prediction: &Ms3FuturePredictionV1,
    bound: &TransportBoundJoinedTransitionV1,
    frame: &RelationFrame,
) -> Result<Ms3IndependentFutureEnvelopeV1, &'static str> {
    prediction.validate(frozen)?;
    let mut machine =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(frozen.machine_checkpoint())
            .map_err(|_| "future_machine_restore_failed")?;
    let programs = machine.candidate_programs();
    let program = programs
        .get(&prediction.canonical_program_root_sha256)
        .ok_or("future_canonical_program_missing")?;
    let frame_root = canonical_json_sha256(frame).map_err(|_| "future_frame_root_failed")?;
    if prediction.capture_sequence != bound.joined.capture_sequence
        || prediction.topology_root_sha256 != bound.binding.topology_commitment_root_sha256
        || prediction.request_event_id_sha256 != bound.binding.request_event_id_sha256
        || prediction.turn_intent_id_sha256 != bound.binding.turn_intent_id_sha256
        || prediction.session_lineage_sha256 != bound.binding.session_lineage_sha256
        || frame_root != bound.binding.completed_frame_root_sha256
        || prediction.predicted_at_unix_nanos >= bound.binding.request_completed_at_unix_nanos
    {
        return Err("future_prediction_binding_mismatch");
    }
    let binding_root = pre_action_t1_binding_root(program, &bound.joined.topology)
        .map_err(|_| "future_pre_action_binding_missing")?;
    if binding_root != prediction.pre_action_binding_root_sha256 {
        return Err("future_pre_action_binding_mismatch");
    }
    let blocker = t1_program_consistency_blocker(program, &bound.joined, frame);
    let mut verdict = Ms3IndependentFutureVerdictV1::Contradiction;
    let mut detail = blocker.unwrap_or_default().to_owned();
    if blocker.is_none() && bound.joined.accepted {
        let observation = observation_for_transition(&bound.joined, frame, &programs)
            .map_err(|_| "future_observation_invalid")?;
        if machine.apply_future(observation).is_ok() {
            verdict = Ms3IndependentFutureVerdictV1::Pass;
            detail.clear();
        } else {
            detail = "future_identification_ledger_rejected".to_owned();
        }
    } else if detail.is_empty() {
        detail = "future_verified_outcome_rejected".to_owned();
    }
    let checkpoint = machine
        .checkpoint_bytes()
        .map_err(|_| "future_checkpoint_failed")?;
    let mut receipt = Ms3IndependentFutureReceiptV1 {
        schema: MS3_INDEPENDENT_FUTURE_RECEIPT_SCHEMA_V1.to_owned(),
        receipt_root_sha256: String::new(),
        contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
        prediction_root_sha256: prediction.prediction_root_sha256.clone(),
        candidate_freeze_root_sha256: prediction.candidate_freeze_root_sha256.clone(),
        canonical_program_root_sha256: prediction.canonical_program_root_sha256.clone(),
        capture_sequence: prediction.capture_sequence,
        topology_root_sha256: prediction.topology_root_sha256.clone(),
        completed_frame_root_sha256: frame_root,
        terminal_receipt_root_sha256: bound.binding.terminal_receipt_root_sha256.clone(),
        transport_binding_root_sha256: bound.binding.binding_root_sha256.clone(),
        session_lineage_sha256: prediction.session_lineage_sha256.clone(),
        verifier_receipt_root_sha256: bound.joined.verifier_receipt_root_sha256.clone(),
        verdict,
        blocker: detail,
        exact_transfer_parity: verdict == Ms3IndependentFutureVerdictV1::Pass,
        runtime_actor_verifier_parity: false,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    receipt.receipt_root_sha256 = receipt.expected_root()?;
    let mut envelope = Ms3IndependentFutureEnvelopeV1 {
        schema: MS3_INDEPENDENT_FUTURE_ENVELOPE_SCHEMA_V1.to_owned(),
        envelope_root_sha256: String::new(),
        receipt,
        machine_checkpoint_sha256: sha256_bytes(&checkpoint),
        machine_checkpoint_bytes: checkpoint.len(),
        machine_checkpoint: checkpoint,
    };
    envelope.envelope_root_sha256 = envelope.expected_root()?;
    envelope.validate(frozen)?;
    Ok(envelope)
}

impl Ms3IndependentFutureReceiptV1 {
    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&FutureReceiptDigestV1 {
            schema: MS3_INDEPENDENT_FUTURE_RECEIPT_SCHEMA_V1,
            contract_root_sha256: &self.contract_root_sha256,
            prediction_root_sha256: &self.prediction_root_sha256,
            candidate_freeze_root_sha256: &self.candidate_freeze_root_sha256,
            canonical_program_root_sha256: &self.canonical_program_root_sha256,
            capture_sequence: self.capture_sequence,
            topology_root_sha256: &self.topology_root_sha256,
            completed_frame_root_sha256: &self.completed_frame_root_sha256,
            terminal_receipt_root_sha256: &self.terminal_receipt_root_sha256,
            transport_binding_root_sha256: &self.transport_binding_root_sha256,
            session_lineage_sha256: &self.session_lineage_sha256,
            verifier_receipt_root_sha256: &self.verifier_receipt_root_sha256,
            verdict: self.verdict,
            blocker: &self.blocker,
            exact_transfer_parity: self.exact_transfer_parity,
            runtime_actor_verifier_parity: false,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(|_| "future_receipt_root_failed")
    }
}

impl Ms3IndependentFutureEnvelopeV1 {
    pub fn canonical_bytes(
        &self,
        frozen: &FrozenVersionSpaceEnvelopeV1,
    ) -> Result<Vec<u8>, &'static str> {
        self.validate(frozen)?;
        serde_cbor::to_vec(self).map_err(|_| "future_envelope_encode_failed")
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        frozen: &FrozenVersionSpaceEnvelopeV1,
    ) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MAX_FUTURE_CHECKPOINT_BYTES {
            return Err("future_envelope_budget");
        }
        let envelope: Self =
            serde_cbor::from_slice(bytes).map_err(|_| "future_envelope_decode_failed")?;
        envelope.validate(frozen)?;
        if envelope.canonical_bytes(frozen)? != bytes {
            return Err("future_envelope_noncanonical");
        }
        Ok(envelope)
    }

    pub fn validate(&self, frozen: &FrozenVersionSpaceEnvelopeV1) -> Result<(), &'static str> {
        let restored =
            OperatorIdentificationMachineV1::from_checkpoint_bytes(&self.machine_checkpoint)
                .map_err(|_| "future_checkpoint_restore_failed")?;
        let accounting = restored
            .evidence_ledger()
            .map(|ledger| ledger.accounting())
            .ok_or("future_ledger_missing")?;
        let pass = self.receipt.verdict == Ms3IndependentFutureVerdictV1::Pass;
        if self.schema != MS3_INDEPENDENT_FUTURE_ENVELOPE_SCHEMA_V1
            || self.receipt.schema != MS3_INDEPENDENT_FUTURE_RECEIPT_SCHEMA_V1
            || self.receipt.contract_root_sha256 != frozen.contract.contract_root_sha256
            || self.receipt.exact_transfer_parity != pass
            || self.receipt.runtime_actor_verifier_parity
            || self.receipt.authority_ready
            || self.receipt.phase_mutation_allowed
            || (pass && !self.receipt.blocker.is_empty())
            || (!pass && self.receipt.blocker.is_empty())
            || self.receipt.receipt_root_sha256 != self.receipt.expected_root()?
            || self.machine_checkpoint.is_empty()
            || self.machine_checkpoint.len() != self.machine_checkpoint_bytes
            || self.machine_checkpoint.len() > MAX_FUTURE_CHECKPOINT_BYTES
            || sha256_bytes(&self.machine_checkpoint) != self.machine_checkpoint_sha256
            || !matches!(
                restored.state(),
                Ok(OperatorIdentificationStateV1::Frozen { .. })
            )
            || accounting.support_rows != 1
            || accounting.support_lineages != 1
            || accounting.future_rows != usize::from(pass)
            || accounting.future_lineages != usize::from(pass)
            || self.envelope_root_sha256 != self.expected_root()?
        {
            return Err("future_envelope_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS3_INDEPENDENT_FUTURE_ENVELOPE_SCHEMA_V1,
            self.receipt.receipt_root_sha256.as_str(),
            self.machine_checkpoint_sha256.as_str(),
        ))
        .map_err(|_| "future_envelope_root_failed")
    }
}
