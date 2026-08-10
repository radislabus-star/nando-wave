use super::journal::scheduler_genesis_root;
use super::*;

pub(super) fn projection_for(
    ledger: &K1SchedulerLedgerV1,
) -> Result<K1SchedulerProjectionV1, String> {
    ledger.validate().map_err(str::to_owned)?;
    let mut active_candidate_freeze = None;
    let mut identification_freeze = None;
    let mut future_prediction_contract = None;
    let mut future_predictions = Vec::new();
    let mut future_prediction_censors = Vec::new();
    let mut future_outcomes = Vec::new();
    let mut latest_probe_round = None;
    let mut completed_probe_rounds = 0u64;
    let mut latest_applied_outcome = None;
    let mut consumed_outcome_roots_sha256 = Vec::new();
    let mut applied_outcome_roots_sha256 = Vec::new();
    let mut remaining_probe_budget = None;
    let mut latest_terminal_verdict = None;
    let mut pending_terminal_transfer = None;
    let mut latest_transfer_settlement = None;
    let mut completed_generations = 0u64;
    let mut completed_candidate_roots_sha256 = Vec::new();
    for event in &ledger.events {
        match &event.payload {
            K1SchedulerEventPayloadV1::CandidateFreeze(freeze) => {
                active_candidate_freeze = Some(freeze.clone());
                identification_freeze = None;
                future_prediction_contract = None;
                future_predictions.clear();
                future_prediction_censors.clear();
                future_outcomes.clear();
                latest_probe_round = None;
                completed_probe_rounds = 0;
                latest_applied_outcome = None;
                consumed_outcome_roots_sha256.clear();
                applied_outcome_roots_sha256.clear();
                remaining_probe_budget = None;
            }
            K1SchedulerEventPayloadV1::IdentificationFreeze(freeze) => {
                identification_freeze = Some(freeze.clone());
                remaining_probe_budget = Some(K1ProbeBudgetRemainingV1 {
                    probe_rounds: freeze.budget.maximum_probe_rounds,
                    probe_cost_units: freeze.budget.maximum_probe_cost_units,
                });
            }
            K1SchedulerEventPayloadV1::FuturePredictionContract(contract) => {
                future_prediction_contract = Some(contract.clone());
            }
            K1SchedulerEventPayloadV1::FuturePrediction(prediction) => {
                future_predictions.push(prediction.clone());
            }
            K1SchedulerEventPayloadV1::FuturePredictionCensored(receipt) => {
                future_prediction_censors.push(receipt.clone());
            }
            K1SchedulerEventPayloadV1::FutureOutcome(outcome) => {
                future_outcomes.push(outcome.clone());
            }
            K1SchedulerEventPayloadV1::ProbeRound(receipt) => {
                latest_probe_round = Some(receipt.clone());
                remaining_probe_budget = Some(receipt.remaining_budget);
                if matches!(
                    receipt.state,
                    K1ProbeRoundStateV1::OutcomeApplied | K1ProbeRoundStateV1::OutcomeCensored
                ) {
                    completed_probe_rounds = completed_probe_rounds.saturating_add(1);
                    let outcome_root = receipt
                        .outcome_receipt_root_sha256
                        .as_ref()
                        .ok_or_else(|| "k1_scheduler_projection_outcome_missing".to_owned())?;
                    consumed_outcome_roots_sha256.push(outcome_root.clone());
                    if receipt.state == K1ProbeRoundStateV1::OutcomeApplied {
                        applied_outcome_roots_sha256.push(outcome_root.clone());
                        latest_applied_outcome = Some(receipt.clone());
                    }
                }
            }
            K1SchedulerEventPayloadV1::TerminalVerdict(verdict) => {
                completed_generations = completed_generations.saturating_add(1);
                completed_candidate_roots_sha256.push(
                    active_candidate_freeze
                        .as_ref()
                        .ok_or_else(|| {
                            "k1_scheduler_projection_terminal_candidate_missing".to_owned()
                        })?
                        .candidate_root_sha256
                        .clone(),
                );
                latest_terminal_verdict = Some(verdict.as_ref().clone());
                pending_terminal_transfer = (verdict.verdict
                    == nando_operator_learning::multi_source::K1GenerationVerdictClassV1::Pass)
                    .then(|| verdict.as_ref().clone());
                active_candidate_freeze = None;
                identification_freeze = None;
                future_prediction_contract = None;
                future_predictions.clear();
                future_prediction_censors.clear();
                future_outcomes.clear();
                latest_probe_round = None;
                completed_probe_rounds = 0;
                latest_applied_outcome = None;
                consumed_outcome_roots_sha256.clear();
                applied_outcome_roots_sha256.clear();
                remaining_probe_budget = None;
            }
            K1SchedulerEventPayloadV1::TransferSettlement(settlement) => {
                pending_terminal_transfer = None;
                latest_transfer_settlement = Some(settlement.clone());
            }
        }
    }
    let consumed_count = consumed_outcome_roots_sha256.len();
    consumed_outcome_roots_sha256.sort();
    consumed_outcome_roots_sha256.dedup();
    if consumed_outcome_roots_sha256.len() != consumed_count {
        return Err("k1_scheduler_projection_outcome_reused".to_owned());
    }
    let applied_count = applied_outcome_roots_sha256.len();
    applied_outcome_roots_sha256.sort();
    applied_outcome_roots_sha256.dedup();
    if applied_outcome_roots_sha256.len() != applied_count {
        return Err("k1_scheduler_projection_applied_outcome_reused".to_owned());
    }
    completed_candidate_roots_sha256.sort();
    future_predictions.sort_by(|left, right| {
        left.prediction_root_sha256
            .cmp(&right.prediction_root_sha256)
    });
    future_prediction_censors
        .sort_by(|left, right| left.censor_root_sha256.cmp(&right.censor_root_sha256));
    future_outcomes.sort_by(|left, right| left.outcome_root_sha256.cmp(&right.outcome_root_sha256));
    let mut projection = K1SchedulerProjectionV1 {
        schema: K1_SCHEDULER_PROJECTION_SCHEMA_V1.to_owned(),
        projection_root_sha256: String::new(),
        ledger_revision: ledger.revision,
        ledger_root_sha256: ledger.ledger_root_sha256.clone(),
        latest_event_root_sha256: ledger
            .latest_event()
            .map_or_else(scheduler_genesis_root, |event| {
                event.event_root_sha256.clone()
            }),
        completed_generations,
        completed_candidate_roots_sha256,
        next_generation_sequence: completed_generations.saturating_add(1),
        active_candidate_freeze,
        identification_freeze,
        future_prediction_contract,
        future_predictions,
        future_prediction_censors,
        future_outcomes,
        latest_probe_round,
        completed_probe_rounds,
        latest_applied_outcome,
        consumed_outcome_roots_sha256,
        applied_outcome_roots_sha256,
        remaining_probe_budget,
        latest_terminal_verdict,
        pending_terminal_transfer,
        latest_transfer_settlement,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    projection.projection_root_sha256 = projection.expected_root()?;
    projection.validate()?;
    Ok(projection)
}

impl K1SchedulerProjectionV1 {
    pub(super) fn validate(&self) -> Result<(), String> {
        let roots = [
            self.projection_root_sha256.as_str(),
            self.ledger_root_sha256.as_str(),
            self.latest_event_root_sha256.as_str(),
        ];
        if self.schema != K1_SCHEDULER_PROJECTION_SCHEMA_V1
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self
                .active_candidate_freeze
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self
                .identification_freeze
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self
                .future_prediction_contract
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self
                .future_predictions
                .iter()
                .any(|value| value.validate().is_err())
            || self
                .future_prediction_censors
                .iter()
                .any(|value| value.validate().is_err())
            || self
                .future_outcomes
                .iter()
                .any(|value| value.validate().is_err())
            || self
                .latest_probe_round
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self
                .latest_applied_outcome
                .as_ref()
                .is_some_and(|value| {
                    value.validate().is_err()
                        || value.state != K1ProbeRoundStateV1::OutcomeApplied
                })
            || self
                .latest_terminal_verdict
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self
                .pending_terminal_transfer
                .as_ref()
                .is_some_and(|value| {
                    value.validate().is_err()
                        || value.verdict
                            != nando_operator_learning::multi_source::K1GenerationVerdictClassV1::Pass
                })
            || self
                .latest_transfer_settlement
                .as_ref()
                .is_some_and(|value| value.validate().is_err())
            || self.active_candidate_freeze.is_none()
                && (self.identification_freeze.is_some()
                    || self.latest_probe_round.is_some()
                    || self.future_prediction_contract.is_some()
                    || !self.future_predictions.is_empty()
                    || !self.future_prediction_censors.is_empty()
                    || !self.future_outcomes.is_empty()
                    || self.latest_applied_outcome.is_some()
                    || self.completed_probe_rounds != 0
                    || !self.consumed_outcome_roots_sha256.is_empty()
                    || !self.applied_outcome_roots_sha256.is_empty()
                    || self.remaining_probe_budget.is_some())
            || self.completed_probe_rounds
                != u64::try_from(self.consumed_outcome_roots_sha256.len())
                    .map_err(|_| "k1_scheduler_projection_probe_count".to_owned())?
            || !strict_unique_roots(&self.consumed_outcome_roots_sha256)
            || !strict_unique_roots(&self.applied_outcome_roots_sha256)
            || !self
                .applied_outcome_roots_sha256
                .iter()
                .all(|root| self.consumed_outcome_roots_sha256.binary_search(root).is_ok())
            || self.identification_freeze.is_some() != self.remaining_probe_budget.is_some()
            || self.next_generation_sequence != self.completed_generations.saturating_add(1)
            || self.completed_generations
                != u64::try_from(self.completed_candidate_roots_sha256.len())
                    .map_err(|_| "k1_scheduler_projection_generation_count".to_owned())?
            || !strict_unique_roots(
                &self
                    .future_predictions
                    .iter()
                    .map(|value| value.prediction_root_sha256.clone())
                    .collect::<Vec<_>>(),
            )
            || !strict_unique_roots(
                &self
                    .future_prediction_censors
                    .iter()
                    .map(|value| value.censor_root_sha256.clone())
                    .collect::<Vec<_>>(),
            )
            || !unique_roots(
                &self
                    .future_prediction_censors
                    .iter()
                    .map(|value| value.prediction_root_sha256.clone())
                    .collect::<Vec<_>>(),
            )
            || self.future_prediction_censors.iter().any(|censor| {
                !self.future_predictions.iter().any(|prediction| {
                    prediction.prediction_root_sha256 == censor.prediction_root_sha256
                        && prediction.topology_commitment_root_sha256
                            == censor.topology_commitment_root_sha256
                        && prediction.capture_sequence == censor.prediction_capture_sequence
                }) || self.future_outcomes.iter().any(|outcome| {
                    outcome.prediction_root_sha256 == censor.prediction_root_sha256
                })
            })
            || !strict_unique_roots(
                &self
                    .future_outcomes
                    .iter()
                    .map(|value| value.outcome_root_sha256.clone())
                    .collect::<Vec<_>>(),
            )
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.projection_root_sha256 != self.expected_root()?
        {
            return Err("k1_scheduler_projection_invalid".to_owned());
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, String> {
        canonical_json_sha256(&K1SchedulerProjectionDigestV1 {
            schema: K1_SCHEDULER_PROJECTION_SCHEMA_V1,
            ledger_revision: self.ledger_revision,
            ledger_root_sha256: self.ledger_root_sha256.as_str(),
            latest_event_root_sha256: self.latest_event_root_sha256.as_str(),
            completed_generations: self.completed_generations,
            completed_candidate_roots_sha256: &self.completed_candidate_roots_sha256,
            next_generation_sequence: self.next_generation_sequence,
            active_candidate_freeze_root_sha256: self
                .active_candidate_freeze
                .as_ref()
                .map(|value| value.freeze_root_sha256.as_str()),
            identification_freeze_root_sha256: self
                .identification_freeze
                .as_ref()
                .map(|value| value.freeze_root_sha256.as_str()),
            future_prediction_contract_root_sha256: self
                .future_prediction_contract
                .as_ref()
                .map(|value| value.contract_root_sha256.as_str()),
            future_prediction_roots_sha256: self
                .future_predictions
                .iter()
                .map(|value| value.prediction_root_sha256.as_str())
                .collect(),
            future_prediction_censor_roots_sha256: self
                .future_prediction_censors
                .iter()
                .map(|value| value.censor_root_sha256.as_str())
                .collect(),
            future_outcome_roots_sha256: self
                .future_outcomes
                .iter()
                .map(|value| value.outcome_root_sha256.as_str())
                .collect(),
            latest_probe_round_root_sha256: self
                .latest_probe_round
                .as_ref()
                .map(|value| value.receipt_root_sha256.as_str()),
            completed_probe_rounds: self.completed_probe_rounds,
            latest_applied_outcome_root_sha256: self
                .latest_applied_outcome
                .as_ref()
                .map(|value| value.receipt_root_sha256.as_str()),
            consumed_outcome_roots_sha256: &self.consumed_outcome_roots_sha256,
            applied_outcome_roots_sha256: &self.applied_outcome_roots_sha256,
            remaining_probe_budget: self.remaining_probe_budget,
            latest_terminal_verdict_root_sha256: self
                .latest_terminal_verdict
                .as_ref()
                .map(|value| value.verdict_root_sha256.as_str()),
            pending_terminal_transfer_root_sha256: self
                .pending_terminal_transfer
                .as_ref()
                .map(|value| value.verdict_root_sha256.as_str()),
            latest_transfer_settlement_root_sha256: self
                .latest_transfer_settlement
                .as_ref()
                .map(|value| value.settlement_root_sha256.as_str()),
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .map_err(str::to_owned)
    }
}

fn strict_unique_roots(roots: &[String]) -> bool {
    roots.iter().all(|root| valid_nonzero_sha256(root))
        && roots.windows(2).all(|pair| pair[0] < pair[1])
}

fn unique_roots(roots: &[String]) -> bool {
    roots.iter().collect::<BTreeSet<_>>().len() == roots.len()
}
