use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::K1_SCHEDULER_SCHEMA_V1;
use super::{
    K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1, K1FutureOutcomeReceiptV1, K1FuturePredictionContractV1,
    K1FuturePredictionReceiptV1, K1GenerationTerminalVerdictV1, K1GenerationVerdictClassV1,
    K1IdentificationFreezeV1, K1NaturalCandidateFreezeV1, K1ProbeBudgetRemainingV1,
    K1ProbeRoundReceiptV1, K1ProbeRoundStateV1, K1TransferSettlementV1,
};

const K1_SCHEDULER_EVENT_SCHEMA_V1: &str = "nando.k1-natural-scheduler-event.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "receipt")]
pub enum K1SchedulerEventPayloadV1 {
    CandidateFreeze(K1NaturalCandidateFreezeV1),
    IdentificationFreeze(K1IdentificationFreezeV1),
    FuturePredictionContract(K1FuturePredictionContractV1),
    FuturePrediction(K1FuturePredictionReceiptV1),
    FutureOutcome(K1FutureOutcomeReceiptV1),
    ProbeRound(K1ProbeRoundReceiptV1),
    TerminalVerdict(Box<K1GenerationTerminalVerdictV1>),
    TransferSettlement(K1TransferSettlementV1),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1SchedulerEventV1 {
    pub schema: String,
    pub event_root_sha256: String,
    pub sequence: u64,
    pub previous_event_root_sha256: String,
    pub payload: K1SchedulerEventPayloadV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1SchedulerLedgerV1 {
    pub schema: String,
    pub ledger_root_sha256: String,
    pub revision: u64,
    pub events: Vec<K1SchedulerEventV1>,
}

#[derive(Default)]
struct ReplayState {
    completed_generations: u64,
    completed_candidates: BTreeSet<String>,
    candidate: Option<K1NaturalCandidateFreezeV1>,
    identification: Option<K1IdentificationFreezeV1>,
    future_contract: Option<K1FuturePredictionContractV1>,
    future_predictions: BTreeMap<String, K1FuturePredictionReceiptV1>,
    future_outcomes: BTreeMap<String, K1FutureOutcomeReceiptV1>,
    pending: Option<K1ProbeRoundReceiptV1>,
    latest_outcome: Option<K1ProbeRoundReceiptV1>,
    pending_transfer: Option<K1GenerationTerminalVerdictV1>,
}

impl K1SchedulerEventPayloadV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::CandidateFreeze(receipt) => receipt.validate(),
            Self::IdentificationFreeze(receipt) => receipt.validate(),
            Self::FuturePredictionContract(receipt) => receipt.validate(),
            Self::FuturePrediction(receipt) => receipt.validate(),
            Self::FutureOutcome(receipt) => receipt.validate(),
            Self::ProbeRound(receipt) => receipt.validate(),
            Self::TerminalVerdict(receipt) => receipt.validate(),
            Self::TransferSettlement(receipt) => receipt.validate(),
        }
    }

    fn root_sha256(&self) -> &str {
        match self {
            Self::CandidateFreeze(receipt) => &receipt.freeze_root_sha256,
            Self::IdentificationFreeze(receipt) => &receipt.freeze_root_sha256,
            Self::FuturePredictionContract(receipt) => &receipt.contract_root_sha256,
            Self::FuturePrediction(receipt) => &receipt.prediction_root_sha256,
            Self::FutureOutcome(receipt) => &receipt.outcome_root_sha256,
            Self::ProbeRound(receipt) => &receipt.receipt_root_sha256,
            Self::TerminalVerdict(receipt) => &receipt.verdict_root_sha256,
            Self::TransferSettlement(receipt) => &receipt.settlement_root_sha256,
        }
    }
}

impl K1SchedulerEventV1 {
    fn seal(
        sequence: u64,
        previous_event_root_sha256: String,
        payload: K1SchedulerEventPayloadV1,
    ) -> Result<Self, &'static str> {
        payload.validate()?;
        let mut event = Self {
            schema: K1_SCHEDULER_EVENT_SCHEMA_V1.to_owned(),
            event_root_sha256: String::new(),
            sequence,
            previous_event_root_sha256,
            payload,
        };
        event.event_root_sha256 = event.expected_root()?;
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.payload.validate()?;
        if self.schema != K1_SCHEDULER_EVENT_SCHEMA_V1
            || self.sequence == 0
            || !valid_nonzero_sha256(&self.event_root_sha256)
            || !valid_nonzero_sha256(&self.previous_event_root_sha256)
            || self.event_root_sha256 != self.expected_root()?
        {
            return Err("k1_scheduler_event_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_SCHEDULER_EVENT_SCHEMA_V1,
            self.sequence,
            self.previous_event_root_sha256.as_str(),
            self.payload.root_sha256(),
        ))
    }
}

impl K1SchedulerLedgerV1 {
    pub fn empty() -> Result<Self, &'static str> {
        let mut ledger = Self {
            schema: K1_SCHEDULER_SCHEMA_V1.to_owned(),
            ledger_root_sha256: String::new(),
            revision: 0,
            events: Vec::new(),
        };
        ledger.ledger_root_sha256 = ledger.expected_root()?;
        ledger.validate()?;
        Ok(ledger)
    }

    pub fn append(
        &mut self,
        payload: K1SchedulerEventPayloadV1,
    ) -> Result<&K1SchedulerEventV1, &'static str> {
        self.validate()?;
        let mut state = ReplayState::default();
        for event in &self.events {
            state.apply(&event.payload)?;
        }
        state.apply(&payload)?;
        let sequence = self.revision.saturating_add(1);
        let previous_event_root_sha256 = self
            .events
            .last()
            .map_or_else(k1_scheduler_genesis_root, |event| {
                event.event_root_sha256.clone()
            });
        let event = K1SchedulerEventV1::seal(sequence, previous_event_root_sha256, payload)?;
        self.events.push(event);
        self.revision = sequence;
        self.ledger_root_sha256 = self.expected_root()?;
        self.validate()?;
        self.events.last().ok_or("k1_scheduler_event_missing")
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_SCHEDULER_SCHEMA_V1
            || self.revision
                != u64::try_from(self.events.len()).map_err(|_| "k1_scheduler_revision")?
            || !valid_nonzero_sha256(&self.ledger_root_sha256)
        {
            return Err("k1_scheduler_ledger_invalid");
        }
        let mut previous = k1_scheduler_genesis_root();
        let mut roots = BTreeSet::new();
        let mut state = ReplayState::default();
        for (index, event) in self.events.iter().enumerate() {
            event.validate()?;
            if event.sequence
                != u64::try_from(index.saturating_add(1)).map_err(|_| "k1_scheduler_revision")?
                || event.previous_event_root_sha256 != previous
                || !roots.insert(event.event_root_sha256.as_str())
            {
                return Err("k1_scheduler_event_chain_invalid");
            }
            state.apply(&event.payload)?;
            previous = event.event_root_sha256.clone();
        }
        if self.ledger_root_sha256 != self.expected_root()? {
            return Err("k1_scheduler_ledger_root_invalid");
        }
        Ok(())
    }

    pub fn active_candidate_freeze(&self) -> Option<&K1NaturalCandidateFreezeV1> {
        let mut candidate = None;
        for event in &self.events {
            match &event.payload {
                K1SchedulerEventPayloadV1::CandidateFreeze(freeze) => candidate = Some(freeze),
                K1SchedulerEventPayloadV1::TerminalVerdict(_) => candidate = None,
                _ => {}
            }
        }
        candidate
    }

    pub fn latest_event(&self) -> Option<&K1SchedulerEventV1> {
        self.events.last()
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_SCHEDULER_SCHEMA_V1,
            self.revision,
            self.events
                .iter()
                .map(|event| event.event_root_sha256.as_str())
                .collect::<Vec<_>>(),
        ))
    }
}

impl ReplayState {
    fn apply(&mut self, payload: &K1SchedulerEventPayloadV1) -> Result<(), &'static str> {
        payload.validate()?;
        match payload {
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze) => {
                if self.candidate.is_some()
                    || self.pending_transfer.is_some()
                    || self
                        .completed_candidates
                        .contains(&freeze.candidate_root_sha256)
                    || freeze.generation_sequence != self.completed_generations.saturating_add(1)
                {
                    return Err("k1_scheduler_candidate_replacement_forbidden");
                }
                self.candidate = Some(freeze.clone());
                self.identification = None;
                self.future_contract = None;
                self.future_predictions.clear();
                self.future_outcomes.clear();
                self.pending = None;
                self.latest_outcome = None;
            }
            K1SchedulerEventPayloadV1::IdentificationFreeze(freeze) => {
                let candidate = self
                    .candidate
                    .as_ref()
                    .ok_or("k1_scheduler_candidate_freeze_missing")?;
                if self.identification.is_some()
                    || freeze.candidate_freeze_root_sha256 != candidate.freeze_root_sha256
                    || freeze.budget != candidate.budget
                {
                    return Err("k1_scheduler_identification_freeze_invalid");
                }
                self.identification = Some(freeze.clone());
            }
            K1SchedulerEventPayloadV1::FuturePredictionContract(contract) => {
                self.apply_future_contract(contract)?;
            }
            K1SchedulerEventPayloadV1::FuturePrediction(prediction) => {
                self.apply_future_prediction(prediction)?;
            }
            K1SchedulerEventPayloadV1::FutureOutcome(outcome) => {
                self.apply_future_outcome(outcome)?;
            }
            K1SchedulerEventPayloadV1::ProbeRound(receipt) => {
                self.apply_probe(receipt)?;
            }
            K1SchedulerEventPayloadV1::TerminalVerdict(verdict) => {
                self.apply_terminal(verdict)?;
            }
            K1SchedulerEventPayloadV1::TransferSettlement(settlement) => {
                self.apply_transfer_settlement(settlement)?;
            }
        }
        Ok(())
    }

    fn apply_future_contract(
        &mut self,
        contract: &K1FuturePredictionContractV1,
    ) -> Result<(), &'static str> {
        let candidate = self
            .candidate
            .as_ref()
            .ok_or("k1_scheduler_candidate_freeze_missing")?;
        let identification = self
            .identification
            .as_ref()
            .ok_or("k1_scheduler_identification_freeze_missing")?;
        if self.future_contract.is_some()
            || contract.candidate_freeze_root_sha256 != candidate.freeze_root_sha256
            || contract.identification_freeze_root_sha256 != identification.freeze_root_sha256
            || identification.prediction_schema != K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
            || identification.initial_semantic_class_roots_sha256.len() != 1
            || identification.initial_semantic_class_roots_sha256[0]
                != contract.semantic_class_root_sha256
        {
            return Err("k1_scheduler_future_contract_invalid");
        }
        self.future_contract = Some(contract.clone());
        Ok(())
    }

    fn apply_future_prediction(
        &mut self,
        prediction: &K1FuturePredictionReceiptV1,
    ) -> Result<(), &'static str> {
        let contract = self
            .future_contract
            .as_ref()
            .ok_or("k1_scheduler_future_contract_missing")?;
        if prediction.contract_root_sha256 != contract.contract_root_sha256
            || prediction.candidate_freeze_root_sha256 != contract.candidate_freeze_root_sha256
            || prediction.identification_freeze_root_sha256
                != contract.identification_freeze_root_sha256
            || prediction.semantic_class_root_sha256 != contract.semantic_class_root_sha256
            || self
                .future_predictions
                .contains_key(&prediction.topology_commitment_root_sha256)
        {
            return Err("k1_scheduler_future_prediction_invalid");
        }
        self.future_predictions.insert(
            prediction.topology_commitment_root_sha256.clone(),
            prediction.clone(),
        );
        Ok(())
    }

    fn apply_future_outcome(
        &mut self,
        outcome: &K1FutureOutcomeReceiptV1,
    ) -> Result<(), &'static str> {
        let prediction = self
            .future_predictions
            .values()
            .find(|prediction| prediction.prediction_root_sha256 == outcome.prediction_root_sha256)
            .ok_or("k1_scheduler_future_prediction_missing")?;
        if outcome.observed_at_unix_nanos <= prediction.predicted_at_unix_nanos
            || self
                .future_outcomes
                .insert(outcome.prediction_root_sha256.clone(), outcome.clone())
                .is_some()
        {
            return Err("k1_scheduler_future_outcome_invalid");
        }
        Ok(())
    }

    fn apply_probe(&mut self, receipt: &K1ProbeRoundReceiptV1) -> Result<(), &'static str> {
        let identification = self
            .identification
            .as_ref()
            .ok_or("k1_scheduler_identification_freeze_missing")?;
        if receipt.identification_freeze_root_sha256 != identification.freeze_root_sha256 {
            return Err("k1_scheduler_probe_identification_mismatch");
        }
        match receipt.state {
            K1ProbeRoundStateV1::ProbePending => {
                if self.pending.is_some() {
                    return Err("k1_scheduler_probe_already_pending");
                }
                let (expected_round, expected_classes, previous_budget) =
                    self.latest_outcome.as_ref().map_or_else(
                        || {
                            (
                                1,
                                identification
                                    .initial_semantic_class_roots_sha256
                                    .as_slice(),
                                K1ProbeBudgetRemainingV1 {
                                    probe_rounds: identification.budget.maximum_probe_rounds,
                                    probe_cost_units: identification
                                        .budget
                                        .maximum_probe_cost_units,
                                },
                            )
                        },
                        |outcome| {
                            (
                                outcome.round_index.saturating_add(1),
                                outcome.next_semantic_class_roots_sha256.as_slice(),
                                outcome.remaining_budget,
                            )
                        },
                    );
                if receipt.round_index != expected_round
                    || receipt.previous_semantic_class_roots_sha256 != expected_classes
                    || !receipt.remaining_budget.no_greater_than(previous_budget)
                    || receipt.remaining_budget.probe_rounds >= previous_budget.probe_rounds
                {
                    return Err("k1_scheduler_probe_budget_or_version_mismatch");
                }
                self.pending = Some(receipt.clone());
            }
            K1ProbeRoundStateV1::OutcomeApplied | K1ProbeRoundStateV1::OutcomeCensored => {
                let pending = self
                    .pending
                    .as_ref()
                    .ok_or("k1_scheduler_probe_pending_missing")?;
                if receipt.supersedes_pending_receipt_root_sha256.as_deref()
                    != Some(pending.receipt_root_sha256.as_str())
                    || receipt.round_index != pending.round_index
                    || receipt.previous_version_space_root_sha256
                        != pending.previous_version_space_root_sha256
                    || receipt.previous_semantic_class_roots_sha256
                        != pending.previous_semantic_class_roots_sha256
                    || receipt.selected_probe_root_sha256 != pending.selected_probe_root_sha256
                    || receipt.observable_difference_root_sha256
                        != pending.observable_difference_root_sha256
                    || receipt.precommitted_predictions_root_sha256
                        != pending.precommitted_predictions_root_sha256
                    || receipt.class_partition_predictions != pending.class_partition_predictions
                    || receipt.outcome_min_capture_sequence != pending.outcome_min_capture_sequence
                    || receipt.remaining_budget != pending.remaining_budget
                {
                    return Err("k1_scheduler_probe_outcome_mismatch");
                }
                self.latest_outcome = Some(receipt.clone());
                self.pending = None;
            }
        }
        Ok(())
    }

    fn apply_terminal(
        &mut self,
        verdict: &K1GenerationTerminalVerdictV1,
    ) -> Result<(), &'static str> {
        let candidate = self
            .candidate
            .as_ref()
            .ok_or("k1_scheduler_candidate_freeze_missing")?;
        if verdict.candidate_freeze_root_sha256 != candidate.freeze_root_sha256 {
            return Err("k1_scheduler_terminal_candidate_mismatch");
        }
        if verdict.verdict != K1GenerationVerdictClassV1::AcquisitionFail {
            let identification = self
                .identification
                .as_ref()
                .ok_or("k1_scheduler_identification_freeze_missing")?;
            if verdict.identification_freeze_root_sha256.as_deref()
                != Some(identification.freeze_root_sha256.as_str())
                || self.pending.is_some()
                    && verdict.verdict != K1GenerationVerdictClassV1::ProbeExhausted
            {
                return Err("k1_scheduler_terminal_identification_mismatch");
            }
            let expected_classes = self.pending.as_ref().map_or_else(
                || {
                    self.latest_outcome.as_ref().map_or(
                        identification
                            .initial_semantic_class_roots_sha256
                            .as_slice(),
                        |outcome| outcome.next_semantic_class_roots_sha256.as_slice(),
                    )
                },
                |pending| pending.previous_semantic_class_roots_sha256.as_slice(),
            );
            if verdict.surviving_semantic_class_roots_sha256 != expected_classes {
                return Err("k1_scheduler_terminal_version_space_mismatch");
            }
            if identification.prediction_schema == K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1
                && verdict.verdict == K1GenerationVerdictClassV1::Pass
                && !self.future_outcomes.values().any(|outcome| {
                    outcome.program_consistent
                        && outcome.independent_verifier_pass
                        && verdict
                            .evidence_roots_sha256
                            .contains(&outcome.outcome_root_sha256)
                        && self.future_predictions.values().any(|prediction| {
                            prediction.prediction_root_sha256 == outcome.prediction_root_sha256
                                && verdict
                                    .evidence_roots_sha256
                                    .contains(&prediction.prediction_root_sha256)
                                && self.future_contract.as_ref().is_none_or(|contract| {
                                    !matches!(
                                        contract.canonical_program.operation,
                                        nando_operator_kernel::ResponseOperation::ComposeCollection {
                                            ..
                                        }
                                    ) || prediction.has_typed_consequence_precommit()
                                        && outcome.predicted_typed_consequence_root_sha256
                                            == prediction.predicted_typed_consequence_root_sha256
                                        && outcome.predicted_typed_consequence_root_sha256
                                            == outcome.observed_typed_consequence_root_sha256
                                })
                        })
                })
            {
                return Err("k1_scheduler_terminal_future_proof_missing");
            }
        }
        self.completed_generations = self.completed_generations.saturating_add(1);
        self.completed_candidates
            .insert(candidate.candidate_root_sha256.clone());
        if verdict.verdict == K1GenerationVerdictClassV1::Pass {
            self.pending_transfer = Some(verdict.clone());
        }
        self.candidate = None;
        self.identification = None;
        self.future_contract = None;
        self.future_predictions.clear();
        self.future_outcomes.clear();
        self.pending = None;
        self.latest_outcome = None;
        Ok(())
    }

    fn apply_transfer_settlement(
        &mut self,
        settlement: &K1TransferSettlementV1,
    ) -> Result<(), &'static str> {
        let terminal = self
            .pending_transfer
            .as_ref()
            .ok_or("k1_scheduler_transfer_missing")?;
        let identification = terminal
            .transfer_identification
            .as_ref()
            .ok_or("k1_scheduler_transfer_identification_missing")?;
        if settlement.terminal_verdict_root_sha256 != terminal.verdict_root_sha256
            || settlement.candidate_freeze_root_sha256 != terminal.candidate_freeze_root_sha256
            || settlement.identification_report_root_sha256 != identification.report_root_sha256
            || settlement.settled_at_unix < terminal.terminal_at_unix
        {
            return Err("k1_scheduler_transfer_settlement_mismatch");
        }
        self.pending_transfer = None;
        Ok(())
    }
}

fn k1_scheduler_genesis_root() -> String {
    format!(
        "{:x}",
        Sha256::digest(b"nando.k1-natural-scheduler-journal-genesis.v1")
    )
}
