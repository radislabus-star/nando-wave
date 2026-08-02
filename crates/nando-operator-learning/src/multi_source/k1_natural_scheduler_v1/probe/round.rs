use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::super::model::{
    K1_PROBE_ROUND_RECEIPT_SCHEMA_V1, canonical_root_slice, canonical_roots, version_space_root,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1ProbeRoundStateV1 {
    ProbePending,
    OutcomeApplied,
    OutcomeCensored,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ProbeBudgetRemainingV1 {
    pub probe_rounds: u64,
    pub probe_cost_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ProbeClassPredictionV1 {
    pub class_id: String,
    pub outcome_partition_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1ProbeRoundReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub identification_freeze_root_sha256: String,
    pub round_index: u64,
    pub supersedes_pending_receipt_root_sha256: Option<String>,
    pub previous_version_space_root_sha256: String,
    pub previous_semantic_class_roots_sha256: Vec<String>,
    pub selected_probe_root_sha256: String,
    pub observable_difference_root_sha256: String,
    pub precommitted_predictions_root_sha256: String,
    pub class_partition_predictions: Vec<K1ProbeClassPredictionV1>,
    pub outcome_min_capture_sequence: u64,
    pub outcome_receipt_root_sha256: Option<String>,
    pub verifier_receipt_root_sha256: Option<String>,
    pub next_version_space_root_sha256: Option<String>,
    pub next_semantic_class_roots_sha256: Vec<String>,
    pub remaining_budget: K1ProbeBudgetRemainingV1,
    pub state: K1ProbeRoundStateV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct K1ProbeRoundDigestV1<'a> {
    schema: &'static str,
    identification_freeze_root_sha256: &'a str,
    round_index: u64,
    supersedes_pending_receipt_root_sha256: Option<&'a str>,
    previous_version_space_root_sha256: &'a str,
    previous_semantic_class_roots_sha256: &'a [String],
    selected_probe_root_sha256: &'a str,
    observable_difference_root_sha256: &'a str,
    precommitted_predictions_root_sha256: &'a str,
    class_partition_predictions: &'a [K1ProbeClassPredictionV1],
    outcome_min_capture_sequence: u64,
    outcome_receipt_root_sha256: Option<&'a str>,
    verifier_receipt_root_sha256: Option<&'a str>,
    next_version_space_root_sha256: Option<&'a str>,
    next_semantic_class_roots_sha256: &'a [String],
    remaining_budget: K1ProbeBudgetRemainingV1,
    state: K1ProbeRoundStateV1,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl K1ProbeBudgetRemainingV1 {
    pub const fn no_greater_than(self, previous: Self) -> bool {
        self.probe_rounds <= previous.probe_rounds
            && self.probe_cost_units <= previous.probe_cost_units
    }
}

impl K1ProbeRoundReceiptV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal_pending(
        identification_freeze_root_sha256: String,
        round_index: u64,
        mut previous_semantic_class_roots_sha256: Vec<String>,
        selected_probe_root_sha256: String,
        observable_difference_root_sha256: String,
        precommitted_predictions_root_sha256: String,
        mut class_partition_predictions: Vec<K1ProbeClassPredictionV1>,
        outcome_min_capture_sequence: u64,
        remaining_budget: K1ProbeBudgetRemainingV1,
    ) -> Result<Self, &'static str> {
        canonical_roots(&mut previous_semantic_class_roots_sha256)?;
        if previous_semantic_class_roots_sha256.len() < 2 {
            return Err("k1_probe_requires_ambiguity");
        }
        let previous_version_space_root_sha256 =
            version_space_root(&previous_semantic_class_roots_sha256)?;
        class_partition_predictions.sort();
        let mut receipt = Self {
            schema: K1_PROBE_ROUND_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            identification_freeze_root_sha256,
            round_index,
            supersedes_pending_receipt_root_sha256: None,
            previous_version_space_root_sha256,
            previous_semantic_class_roots_sha256,
            selected_probe_root_sha256,
            observable_difference_root_sha256,
            precommitted_predictions_root_sha256,
            class_partition_predictions,
            outcome_min_capture_sequence,
            outcome_receipt_root_sha256: None,
            verifier_receipt_root_sha256: None,
            next_version_space_root_sha256: None,
            next_semantic_class_roots_sha256: Vec::new(),
            remaining_budget,
            state: K1ProbeRoundStateV1::ProbePending,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn seal_outcome(
        pending: &Self,
        outcome_receipt_root_sha256: String,
        verifier_receipt_root_sha256: String,
        mut next_semantic_class_roots_sha256: Vec<String>,
        censored_no_information: bool,
    ) -> Result<Self, &'static str> {
        pending.validate()?;
        if pending.state != K1ProbeRoundStateV1::ProbePending {
            return Err("k1_probe_pending_receipt_required");
        }
        canonical_roots(&mut next_semantic_class_roots_sha256)?;
        let next_version_space_root_sha256 = version_space_root(&next_semantic_class_roots_sha256)?;
        let mut receipt = Self {
            schema: K1_PROBE_ROUND_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            identification_freeze_root_sha256: pending.identification_freeze_root_sha256.clone(),
            round_index: pending.round_index,
            supersedes_pending_receipt_root_sha256: Some(pending.receipt_root_sha256.clone()),
            previous_version_space_root_sha256: pending.previous_version_space_root_sha256.clone(),
            previous_semantic_class_roots_sha256: pending
                .previous_semantic_class_roots_sha256
                .clone(),
            selected_probe_root_sha256: pending.selected_probe_root_sha256.clone(),
            observable_difference_root_sha256: pending.observable_difference_root_sha256.clone(),
            precommitted_predictions_root_sha256: pending
                .precommitted_predictions_root_sha256
                .clone(),
            class_partition_predictions: pending.class_partition_predictions.clone(),
            outcome_min_capture_sequence: pending.outcome_min_capture_sequence,
            outcome_receipt_root_sha256: Some(outcome_receipt_root_sha256),
            verifier_receipt_root_sha256: Some(verifier_receipt_root_sha256),
            next_version_space_root_sha256: Some(next_version_space_root_sha256),
            next_semantic_class_roots_sha256,
            remaining_budget: pending.remaining_budget,
            state: if censored_no_information {
                K1ProbeRoundStateV1::OutcomeCensored
            } else {
                K1ProbeRoundStateV1::OutcomeApplied
            },
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let common_roots = [
            self.receipt_root_sha256.as_str(),
            self.identification_freeze_root_sha256.as_str(),
            self.previous_version_space_root_sha256.as_str(),
            self.selected_probe_root_sha256.as_str(),
            self.observable_difference_root_sha256.as_str(),
            self.precommitted_predictions_root_sha256.as_str(),
        ];
        if self.schema != K1_PROBE_ROUND_RECEIPT_SCHEMA_V1
            || !common_roots.into_iter().all(valid_nonzero_sha256)
            || self.round_index == 0
            || self.previous_semantic_class_roots_sha256.len() < 2
            || !canonical_root_slice(&self.previous_semantic_class_roots_sha256)
            || self.previous_version_space_root_sha256
                != version_space_root(&self.previous_semantic_class_roots_sha256)?
            || !self.prediction_contract_valid()
            || self.outcome_min_capture_sequence == 0
            || self.authority_ready
            || self.phase_mutation_allowed
        {
            return Err("k1_probe_round_common_binding_invalid");
        }
        match self.state {
            K1ProbeRoundStateV1::ProbePending => {
                if self.supersedes_pending_receipt_root_sha256.is_some()
                    || self.outcome_receipt_root_sha256.is_some()
                    || self.verifier_receipt_root_sha256.is_some()
                    || self.next_version_space_root_sha256.is_some()
                    || !self.next_semantic_class_roots_sha256.is_empty()
                {
                    return Err("k1_probe_pending_state_invalid");
                }
            }
            K1ProbeRoundStateV1::OutcomeApplied | K1ProbeRoundStateV1::OutcomeCensored => {
                let expected_next_root =
                    version_space_root(&self.next_semantic_class_roots_sha256)?;
                let terminal_roots = [
                    self.supersedes_pending_receipt_root_sha256.as_deref(),
                    self.outcome_receipt_root_sha256.as_deref(),
                    self.verifier_receipt_root_sha256.as_deref(),
                    self.next_version_space_root_sha256.as_deref(),
                ];
                if !terminal_roots
                    .into_iter()
                    .all(|root| root.is_some_and(valid_nonzero_sha256))
                    || !canonical_root_slice(&self.next_semantic_class_roots_sha256)
                    || self.next_version_space_root_sha256.as_deref()
                        != Some(expected_next_root.as_str())
                {
                    return Err("k1_probe_outcome_binding_invalid");
                }
                let previous = self
                    .previous_semantic_class_roots_sha256
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                let next = self
                    .next_semantic_class_roots_sha256
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                if !next.is_subset(&previous)
                    || (self.state == K1ProbeRoundStateV1::OutcomeApplied
                        && next.len() >= previous.len())
                    || (self.state == K1ProbeRoundStateV1::OutcomeCensored && next != previous)
                {
                    return Err("k1_probe_version_space_not_monotonic");
                }
                if self.state == K1ProbeRoundStateV1::OutcomeApplied
                    && !self.predicted_partitions().contains(&next)
                {
                    return Err("k1_probe_outcome_not_precommitted_partition");
                }
            }
        }
        if self.receipt_root_sha256 != self.expected_root()? {
            return Err("k1_probe_round_root_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&K1ProbeRoundDigestV1 {
            schema: K1_PROBE_ROUND_RECEIPT_SCHEMA_V1,
            identification_freeze_root_sha256: &self.identification_freeze_root_sha256,
            round_index: self.round_index,
            supersedes_pending_receipt_root_sha256: self
                .supersedes_pending_receipt_root_sha256
                .as_deref(),
            previous_version_space_root_sha256: &self.previous_version_space_root_sha256,
            previous_semantic_class_roots_sha256: &self.previous_semantic_class_roots_sha256,
            selected_probe_root_sha256: &self.selected_probe_root_sha256,
            observable_difference_root_sha256: &self.observable_difference_root_sha256,
            precommitted_predictions_root_sha256: &self.precommitted_predictions_root_sha256,
            class_partition_predictions: &self.class_partition_predictions,
            outcome_min_capture_sequence: self.outcome_min_capture_sequence,
            outcome_receipt_root_sha256: self.outcome_receipt_root_sha256.as_deref(),
            verifier_receipt_root_sha256: self.verifier_receipt_root_sha256.as_deref(),
            next_version_space_root_sha256: self.next_version_space_root_sha256.as_deref(),
            next_semantic_class_roots_sha256: &self.next_semantic_class_roots_sha256,
            remaining_budget: self.remaining_budget,
            state: self.state,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
    }

    fn prediction_contract_valid(&self) -> bool {
        let predicted_classes = self
            .class_partition_predictions
            .iter()
            .map(|prediction| prediction.class_id.as_str())
            .collect::<BTreeSet<_>>();
        let previous_classes = self
            .previous_semantic_class_roots_sha256
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let partitions = self
            .class_partition_predictions
            .iter()
            .map(|prediction| prediction.outcome_partition_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        self.class_partition_predictions
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            && predicted_classes == previous_classes
            && predicted_classes.len() == self.class_partition_predictions.len()
            && partitions.len() >= 2
            && self.class_partition_predictions.iter().all(|prediction| {
                valid_nonzero_sha256(&prediction.class_id)
                    && valid_nonzero_sha256(&prediction.outcome_partition_root_sha256)
            })
            && canonical_json_sha256(&(
                "nando.multi-source-t1-precommitted-probe-predictions.v1",
                self.selected_probe_root_sha256.as_str(),
                self.observable_difference_root_sha256.as_str(),
                &self.class_partition_predictions,
            ))
            .ok()
            .as_deref()
                == Some(self.precommitted_predictions_root_sha256.as_str())
    }

    fn predicted_partitions(&self) -> BTreeSet<BTreeSet<&str>> {
        let mut partitions = BTreeMap::<&str, BTreeSet<&str>>::new();
        for prediction in &self.class_partition_predictions {
            partitions
                .entry(prediction.outcome_partition_root_sha256.as_str())
                .or_default()
                .insert(prediction.class_id.as_str());
        }
        partitions.into_values().collect()
    }
}
