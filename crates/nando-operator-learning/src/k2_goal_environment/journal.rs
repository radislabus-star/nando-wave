use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1, LawLabSandboxPurposeV1, LawLabSandboxReceiptV1};

use super::{
    K2_MAX_EPISODE_BYTES_V1, K2_MAX_EVENT_BYTES_V1, K2_MAX_EVENTS_PER_EPISODE_V1,
    K2_MAX_RETAINED_CAPABILITY_EPISODES_V1, K2AlternativePredictionSetV1, K2AuthorityBoundaryV1,
    K2DecisionEpisodeSealV1, K2DecisionFreezeV1, K2DecisionOutcomeReceiptV1,
    K2EvidenceProvenanceV1, K2ExactGoalReceiptV1, K2GoalEnvironmentErrorV1,
    K2GoalEnvironmentResultV1, K2LawLabBindingV1,
};

pub const K2_EPISODE_EVENT_SCHEMA_V1: &str = "nando.k2-episode-event.v1";
pub const K2_EPISODE_PROJECTION_SCHEMA_V1: &str = "nando.k2-episode-projection.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2EpisodeEventKindV1 {
    ContractFrozen,
    PredictionsPrecommitted,
    ProbePlanned,
    ProbeDispatched,
    ProbeExecuted,
    OutcomeVerified,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2EpisodeStateV1 {
    Empty,
    ContractFrozen,
    PredictionsPrecommitted,
    ProbePlanned,
    ProbeDispatched,
    ProbeExecuted,
    OutcomeVerified,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EpisodeEventV1 {
    pub schema: String,
    pub entry_root_sha256: String,
    pub episode_id_sha256: String,
    pub sequence: u64,
    pub previous_entry_root_sha256: Option<String>,
    pub event_kind: K2EpisodeEventKindV1,
    pub event_payload_root_sha256: String,
    pub event_payload: Value,
    pub written_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2EpisodeEventDigestV1<'a> {
    schema: &'static str,
    episode_id_sha256: &'a str,
    sequence: u64,
    previous_entry_root_sha256: Option<&'a str>,
    event_kind: K2EpisodeEventKindV1,
    event_payload_root_sha256: &'a str,
    event_payload: &'a Value,
    written_at_unix_ms: u64,
}

impl K2EpisodeEventV1 {
    fn seal<T: Serialize>(
        episode_id_sha256: String,
        sequence: u64,
        previous_entry_root_sha256: Option<String>,
        event_kind: K2EpisodeEventKindV1,
        payload: &T,
        written_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let event_payload =
            serde_json::to_value(payload).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
        let event_payload_root_sha256 = canonical_json_sha256(&event_payload)
            .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
        let mut event = Self {
            schema: K2_EPISODE_EVENT_SCHEMA_V1.to_owned(),
            entry_root_sha256: String::new(),
            episode_id_sha256,
            sequence,
            previous_entry_root_sha256,
            event_kind,
            event_payload_root_sha256,
            event_payload,
            written_at_unix_ms,
        };
        event.entry_root_sha256 = event.expected_root()?;
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        if !valid_nonzero_sha256(&self.entry_root_sha256)
            || !valid_nonzero_sha256(&self.episode_id_sha256)
            || !valid_nonzero_sha256(&self.event_payload_root_sha256)
            || self
                .previous_entry_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || (self.sequence == 0) != self.previous_entry_root_sha256.is_none()
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_event_root_invalid",
            ));
        }
        let expected_payload_root = canonical_json_sha256(&self.event_payload)
            .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
        if self.schema != K2_EPISODE_EVENT_SCHEMA_V1
            || self.event_payload_root_sha256 != expected_payload_root
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_event_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_json_sha256(&K2EpisodeEventDigestV1 {
            schema: K2_EPISODE_EVENT_SCHEMA_V1,
            episode_id_sha256: &self.episode_id_sha256,
            sequence: self.sequence,
            previous_entry_root_sha256: self.previous_entry_root_sha256.as_deref(),
            event_kind: self.event_kind,
            event_payload_root_sha256: &self.event_payload_root_sha256,
            event_payload: &self.event_payload,
            written_at_unix_ms: self.written_at_unix_ms,
        })
        .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2EpisodeProjectionV1 {
    pub schema: String,
    pub projection_root_sha256: String,
    pub episode_id_sha256: String,
    pub state: K2EpisodeStateV1,
    pub event_count: u64,
    pub total_event_bytes: u64,
    pub latest_entry_root_sha256: Option<String>,
    pub terminal_outcome_root_sha256: Option<String>,
    pub same_identity_execution_allowed: bool,
    pub indeterminate_after_crash: bool,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2EpisodeProjectionDigestV1<'a> {
    schema: &'static str,
    episode_id_sha256: &'a str,
    state: K2EpisodeStateV1,
    event_count: u64,
    total_event_bytes: u64,
    latest_entry_root_sha256: Option<&'a str>,
    terminal_outcome_root_sha256: Option<&'a str>,
    same_identity_execution_allowed: bool,
    indeterminate_after_crash: bool,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2EpisodeProjectionV1 {
    pub fn project(
        episode_id_sha256: &str,
        events: &[K2EpisodeEventV1],
    ) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(episode_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_episode_id_invalid"));
        }
        if events.len() as u64 > K2_MAX_EVENTS_PER_EPISODE_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_event_budget_exhausted",
            ));
        }
        let mut state = K2EpisodeStateV1::Empty;
        let mut previous_root: Option<&str> = None;
        let mut total_event_bytes = 0_u64;
        let mut terminal_outcome_root_sha256 = None;
        let mut replay = K2EpisodeRootReplayV1::default();
        for (index, event) in events.iter().enumerate() {
            event.validate()?;
            let bytes = event.canonical_bytes()?;
            let event_bytes = u64::try_from(bytes.len())
                .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_episode_event_size_overflow"))?;
            if event_bytes > K2_MAX_EVENT_BYTES_V1 {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_episode_event_bytes_exhausted",
                ));
            }
            total_event_bytes = total_event_bytes.checked_add(event_bytes).ok_or(
                K2GoalEnvironmentErrorV1::Invalid("k2_episode_total_bytes_overflow"),
            )?;
            if total_event_bytes > K2_MAX_EPISODE_BYTES_V1
                || event.episode_id_sha256 != episode_id_sha256
                || event.sequence != index as u64
                || event.previous_entry_root_sha256.as_deref() != previous_root
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_episode_chain_invalid",
                ));
            }
            state = transition_v1(state, event.event_kind)?;
            replay.observe(event)?;
            if event.event_kind == K2EpisodeEventKindV1::Terminal {
                let outcome: K2DecisionOutcomeReceiptV1 =
                    serde_json::from_value(event.event_payload.clone()).map_err(|_| {
                        K2GoalEnvironmentErrorV1::Invalid("k2_terminal_payload_invalid")
                    })?;
                outcome.validate()?;
                terminal_outcome_root_sha256 = Some(outcome.outcome_root_sha256);
            }
            previous_root = Some(&event.entry_root_sha256);
        }
        let latest_entry_root_sha256 = events.last().map(|event| event.entry_root_sha256.clone());
        let same_identity_execution_allowed = state == K2EpisodeStateV1::ProbePlanned;
        let indeterminate_after_crash = state == K2EpisodeStateV1::ProbeDispatched;
        let mut projection = Self {
            schema: K2_EPISODE_PROJECTION_SCHEMA_V1.to_owned(),
            projection_root_sha256: String::new(),
            episode_id_sha256: episode_id_sha256.to_owned(),
            state,
            event_count: events.len() as u64,
            total_event_bytes,
            latest_entry_root_sha256,
            terminal_outcome_root_sha256,
            same_identity_execution_allowed,
            indeterminate_after_crash,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        projection.projection_root_sha256 = projection.expected_root()?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        if self.schema != K2_EPISODE_PROJECTION_SCHEMA_V1
            || !valid_nonzero_sha256(&self.projection_root_sha256)
            || !valid_nonzero_sha256(&self.episode_id_sha256)
            || self
                .latest_entry_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || self
                .terminal_outcome_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || self.event_count > K2_MAX_EVENTS_PER_EPISODE_V1
            || self.total_event_bytes > K2_MAX_EPISODE_BYTES_V1
            || self.same_identity_execution_allowed
                != (self.state == K2EpisodeStateV1::ProbePlanned)
            || self.indeterminate_after_crash != (self.state == K2EpisodeStateV1::ProbeDispatched)
            || self.terminal_outcome_root_sha256.is_some()
                != (self.state == K2EpisodeStateV1::Terminal)
            || self.projection_root_sha256 != self.expected_root()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_projection_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_json_sha256(&K2EpisodeProjectionDigestV1 {
            schema: K2_EPISODE_PROJECTION_SCHEMA_V1,
            episode_id_sha256: &self.episode_id_sha256,
            state: self.state,
            event_count: self.event_count,
            total_event_bytes: self.total_event_bytes,
            latest_entry_root_sha256: self.latest_entry_root_sha256.as_deref(),
            terminal_outcome_root_sha256: self.terminal_outcome_root_sha256.as_deref(),
            same_identity_execution_allowed: self.same_identity_execution_allowed,
            indeterminate_after_crash: self.indeterminate_after_crash,
            authority: &self.authority,
        })
        .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }
}

fn transition_v1(
    state: K2EpisodeStateV1,
    event: K2EpisodeEventKindV1,
) -> K2GoalEnvironmentResultV1<K2EpisodeStateV1> {
    match (state, event) {
        (K2EpisodeStateV1::Empty, K2EpisodeEventKindV1::ContractFrozen) => {
            Ok(K2EpisodeStateV1::ContractFrozen)
        }
        (K2EpisodeStateV1::ContractFrozen, K2EpisodeEventKindV1::PredictionsPrecommitted) => {
            Ok(K2EpisodeStateV1::PredictionsPrecommitted)
        }
        (K2EpisodeStateV1::PredictionsPrecommitted, K2EpisodeEventKindV1::ProbePlanned) => {
            Ok(K2EpisodeStateV1::ProbePlanned)
        }
        (K2EpisodeStateV1::ProbePlanned, K2EpisodeEventKindV1::ProbeDispatched) => {
            Ok(K2EpisodeStateV1::ProbeDispatched)
        }
        (K2EpisodeStateV1::ProbeDispatched, K2EpisodeEventKindV1::ProbeExecuted) => {
            Ok(K2EpisodeStateV1::ProbeExecuted)
        }
        (K2EpisodeStateV1::ProbeExecuted, K2EpisodeEventKindV1::OutcomeVerified) => {
            Ok(K2EpisodeStateV1::OutcomeVerified)
        }
        (K2EpisodeStateV1::OutcomeVerified, K2EpisodeEventKindV1::Terminal) => {
            Ok(K2EpisodeStateV1::Terminal)
        }
        _ => Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_episode_transition_invalid",
        )),
    }
}

#[derive(Default)]
struct K2EpisodeRootReplayV1 {
    freeze: Option<K2DecisionFreezeV1>,
    predictions: Option<K2AlternativePredictionSetV1>,
    binding: Option<K2LawLabBindingV1>,
    sandbox_receipt: Option<LawLabSandboxReceiptV1>,
    exact_goal: Option<K2ExactGoalReceiptV1>,
}

impl K2EpisodeRootReplayV1 {
    fn observe(&mut self, event: &K2EpisodeEventV1) -> K2GoalEnvironmentResultV1<()> {
        match event.event_kind {
            K2EpisodeEventKindV1::ContractFrozen => {
                let freeze: K2DecisionFreezeV1 = decode_event_payload_v1(event)?;
                freeze.validate_persisted_v1()?;
                if freeze.episode_id_sha256 != event.episode_id_sha256 {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_contract_identity_mismatch",
                    ));
                }
                self.freeze = Some(freeze);
            }
            K2EpisodeEventKindV1::PredictionsPrecommitted => {
                let predictions: K2AlternativePredictionSetV1 = decode_event_payload_v1(event)?;
                predictions.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                if predictions.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
                    || predictions.provenance != freeze.provenance
                    || predictions.predictor_executable_sha256 != freeze.selector_executable_sha256
                    || predictions.goal_envelope_root_sha256 != freeze.goal_envelope_root_sha256
                    || predictions.vocabulary_snapshot_root_sha256
                        != freeze.vocabulary_snapshot_root_sha256
                    || predictions.alternative_set_root_sha256 != freeze.alternative_set_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_prediction_freeze_mismatch",
                    ));
                }
                self.predictions = Some(predictions);
            }
            K2EpisodeEventKindV1::ProbePlanned => {
                let binding: K2LawLabBindingV1 = decode_event_payload_v1(event)?;
                binding.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let predictions = self.predictions()?;
                let mut satisfying = predictions
                    .predictions
                    .iter()
                    .filter(|prediction| prediction.predicted_goal_satisfied);
                let selected = satisfying.next();
                if binding.episode_id_sha256 != freeze.episode_id_sha256
                    || binding.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
                    || binding.goal_envelope_root_sha256 != freeze.goal_envelope_root_sha256
                    || binding.vocabulary_snapshot_root_sha256
                        != freeze.vocabulary_snapshot_root_sha256
                    || binding.alternative_set_root_sha256 != freeze.alternative_set_root_sha256
                    || binding.prediction_set_root_sha256 != predictions.prediction_set_root_sha256
                    || binding.source_tree_root_sha256 != freeze.initial_environment_root_sha256
                    || binding.worker_sha256 != freeze.sandbox_worker_sha256
                    || binding.deterministic_seed_sha256 != freeze.deterministic_seed_sha256
                    || binding.budget_root_sha256 != freeze.budget_root_sha256
                    || selected.is_none_or(|prediction| {
                        prediction.action_root_sha256 != binding.selected_action_root_sha256
                    })
                    || satisfying.next().is_some()
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_probe_binding_mismatch",
                    ));
                }
                self.binding = Some(binding);
            }
            K2EpisodeEventKindV1::ProbeDispatched => {
                let dispatched_binding_root: String = decode_event_payload_v1(event)?;
                if !valid_nonzero_sha256(&dispatched_binding_root)
                    || dispatched_binding_root != self.binding()?.binding_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_dispatch_binding_mismatch",
                    ));
                }
            }
            K2EpisodeEventKindV1::ProbeExecuted => {
                let receipt: LawLabSandboxReceiptV1 = decode_event_payload_v1(event)?;
                validate_persisted_law_lab_receipt_v1(&receipt)?;
                let freeze = self.freeze()?;
                let predictions = self.predictions()?;
                let binding = self.binding()?;
                if receipt.purpose != LawLabSandboxPurposeV1::GeneratedCapabilitySelfTest
                    || receipt.request_root_sha256 != binding.law_lab_request_root_sha256
                    || receipt.candidate_root_sha256 != freeze.episode_id_sha256
                    || receipt.version_space_root_sha256 != freeze.alternative_set_root_sha256
                    || receipt.durable_prediction_ledger_root_sha256
                        != predictions.prediction_set_root_sha256
                    || receipt.executor_manifest_root_sha256
                        != binding.executor_manifest_root_sha256
                    || receipt.source_tree_root_sha256 != binding.source_tree_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_execution_binding_mismatch",
                    ));
                }
                self.sandbox_receipt = Some(receipt);
            }
            K2EpisodeEventKindV1::OutcomeVerified => {
                let exact_goal: K2ExactGoalReceiptV1 = decode_event_payload_v1(event)?;
                exact_goal.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let predictions = self.predictions()?;
                let binding = self.binding()?;
                let receipt = self.sandbox_receipt()?;
                let selected_prediction = predictions
                    .predictions
                    .iter()
                    .find(|prediction| {
                        prediction.action_root_sha256 == binding.selected_action_root_sha256
                    })
                    .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_selected_prediction_missing",
                    ))?;
                if exact_goal.provenance != K2EvidenceProvenanceV1::GeneratedCapabilitySelfTest
                    || exact_goal.provenance != freeze.provenance
                    || exact_goal.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
                    || exact_goal.law_lab_binding_root_sha256 != binding.binding_root_sha256
                    || exact_goal.law_lab_receipt_root_sha256 != receipt.receipt_root_sha256
                    || exact_goal.oracle_manifest_root_sha256 != freeze.oracle_manifest_root_sha256
                    || exact_goal.expected_terminal_tree_root_sha256
                        != selected_prediction.predicted_terminal_tree_root_sha256
                    || exact_goal.observed_terminal_tree_root_sha256
                        != receipt.post_tree_root_sha256
                    || !exact_goal.goal_satisfied
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_exact_goal_binding_mismatch",
                    ));
                }
                self.exact_goal = Some(exact_goal);
            }
            K2EpisodeEventKindV1::Terminal => {
                let outcome: K2DecisionOutcomeReceiptV1 = decode_event_payload_v1(event)?;
                outcome.validate()?;
                let freeze = self.freeze()?;
                let predictions = self.predictions()?;
                let binding = self.binding()?;
                let receipt = self.sandbox_receipt()?;
                let exact_goal = self.exact_goal()?;
                if outcome.provenance != freeze.provenance
                    || outcome.decision_freeze_root_sha256 != freeze.decision_freeze_root_sha256
                    || outcome.prediction_set_root_sha256 != predictions.prediction_set_root_sha256
                    || outcome.law_lab_binding_root_sha256 != binding.binding_root_sha256
                    || outcome.sandbox_receipt_root_sha256 != receipt.receipt_root_sha256
                    || outcome.exact_goal_receipt_root_sha256 != exact_goal.receipt_root_sha256
                {
                    return Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_episode_terminal_binding_mismatch",
                    ));
                }
            }
        }
        Ok(())
    }

    fn freeze(&self) -> K2GoalEnvironmentResultV1<&K2DecisionFreezeV1> {
        self.freeze
            .as_ref()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_freeze_reference_missing",
            ))
    }

    fn predictions(&self) -> K2GoalEnvironmentResultV1<&K2AlternativePredictionSetV1> {
        self.predictions
            .as_ref()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_prediction_reference_missing",
            ))
    }

    fn binding(&self) -> K2GoalEnvironmentResultV1<&K2LawLabBindingV1> {
        self.binding
            .as_ref()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_binding_reference_missing",
            ))
    }

    fn sandbox_receipt(&self) -> K2GoalEnvironmentResultV1<&LawLabSandboxReceiptV1> {
        self.sandbox_receipt
            .as_ref()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_sandbox_receipt_reference_missing",
            ))
    }

    fn exact_goal(&self) -> K2GoalEnvironmentResultV1<&K2ExactGoalReceiptV1> {
        self.exact_goal
            .as_ref()
            .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_exact_goal_reference_missing",
            ))
    }
}

fn decode_event_payload_v1<T: DeserializeOwned>(
    event: &K2EpisodeEventV1,
) -> K2GoalEnvironmentResultV1<T> {
    serde_json::from_value(event.event_payload.clone())
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_episode_typed_payload_invalid"))
}

fn validate_persisted_law_lab_receipt_v1(
    receipt: &LawLabSandboxReceiptV1,
) -> K2GoalEnvironmentResultV1<()> {
    receipt
        .cleanup
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    receipt
        .authority
        .validate()
        .map_err(|error| K2GoalEnvironmentErrorV1::Sandbox(error.to_string()))?;
    for root in [
        receipt.receipt_root_sha256.as_str(),
        receipt.contract_root_sha256.as_str(),
        receipt.executor_manifest_root_sha256.as_str(),
        receipt.request_root_sha256.as_str(),
        receipt.candidate_root_sha256.as_str(),
        receipt.version_space_root_sha256.as_str(),
        receipt.durable_prediction_ledger_root_sha256.as_str(),
        receipt.probe_root_sha256.as_str(),
        receipt.worker_outcome_root_sha256.as_str(),
        receipt.exact_outcome_root_sha256.as_str(),
        receipt.source_tree_root_sha256.as_str(),
        receipt.post_tree_root_sha256.as_str(),
        receipt.isolation_attestation_root_sha256.as_str(),
    ] {
        if !valid_nonzero_sha256(root) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_persisted_law_lab_receipt_root_invalid",
            ));
        }
    }
    let mut digest =
        serde_json::to_value(receipt).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
    let object = digest
        .as_object_mut()
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_persisted_law_lab_receipt_shape_invalid",
        ))?;
    object
        .remove("receipt_root_sha256")
        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_persisted_law_lab_receipt_root_missing",
        ))?;
    let expected_root =
        canonical_json_sha256(&digest).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
    if receipt.schema != LAW_LAB_SANDBOX_RECEIPT_SCHEMA_V1
        || receipt.receipt_root_sha256 != expected_root
    {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_persisted_law_lab_receipt_invalid",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K2JournalFaultPointV1 {
    None,
    AfterTempSync,
    AfterPublishBeforeDirectorySync,
}

pub struct K2EpisodeJournalV1 {
    episode_directory: PathBuf,
    episode_id_sha256: String,
    events: Vec<K2EpisodeEventV1>,
    projection: K2EpisodeProjectionV1,
}

impl K2EpisodeJournalV1 {
    pub fn create(store_root: &Path, episode_id_sha256: String) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(&episode_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_episode_id_invalid"));
        }
        fs::create_dir_all(store_root).map_err(io_error_v1("create_store_root"))?;
        let retained = fs::read_dir(store_root)
            .map_err(io_error_v1("read_store_root"))?
            .try_fold(0_u64, |count, entry| {
                let entry = entry.map_err(io_error_v1("read_store_entry"))?;
                if entry
                    .file_type()
                    .map_err(io_error_v1("stat_store_entry"))?
                    .is_dir()
                {
                    count
                        .checked_add(1)
                        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                            "k2_retained_episode_count_overflow",
                        ))
                } else {
                    Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_store_root_unknown_entry",
                    ))
                }
            })?;
        if retained >= K2_MAX_RETAINED_CAPABILITY_EPISODES_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_retained_episode_budget_exhausted",
            ));
        }
        let episode_directory = store_root.join(&episode_id_sha256);
        fs::create_dir(&episode_directory).map_err(io_error_v1("create_episode_directory"))?;
        sync_directory_v1(store_root)?;
        let projection = K2EpisodeProjectionV1::project(&episode_id_sha256, &[])?;
        Ok(Self {
            episode_directory,
            episode_id_sha256,
            events: Vec::new(),
            projection,
        })
    }

    pub fn open_existing(
        store_root: &Path,
        episode_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(&episode_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid("k2_episode_id_invalid"));
        }
        let episode_directory = store_root.join(&episode_id_sha256);
        if !episode_directory.is_dir() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_directory_missing",
            ));
        }
        let mut paths = fs::read_dir(&episode_directory)
            .map_err(io_error_v1("read_episode_directory"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(io_error_v1("read_episode_entry"))?;
        paths.sort();
        if paths.len() as u64 > K2_MAX_EVENTS_PER_EPISODE_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_event_budget_exhausted",
            ));
        }
        let mut events = Vec::with_capacity(paths.len());
        for (index, path) in paths.iter().enumerate() {
            let expected_name = event_filename_v1(index as u64);
            if path.file_name().and_then(|name| name.to_str()) != Some(expected_name.as_str()) {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_episode_unknown_or_stale_entry",
                ));
            }
            let bytes = fs::read(path).map_err(io_error_v1("read_episode_event"))?;
            if bytes.len() as u64 > K2_MAX_EVENT_BYTES_V1 {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_episode_event_bytes_exhausted",
                ));
            }
            let event: K2EpisodeEventV1 = serde_json::from_slice(&bytes)
                .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_episode_event_decode_failed"))?;
            if event.canonical_bytes()? != bytes {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_episode_event_not_canonical",
                ));
            }
            events.push(event);
        }
        let projection = K2EpisodeProjectionV1::project(&episode_id_sha256, &events)?;
        Ok(Self {
            episode_directory,
            episode_id_sha256,
            events,
            projection,
        })
    }

    pub fn append<T: Serialize>(
        &mut self,
        event_kind: K2EpisodeEventKindV1,
        payload: &T,
        written_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<K2EpisodeEventV1> {
        self.append_with_fault_v1(
            event_kind,
            payload,
            written_at_unix_ms,
            K2JournalFaultPointV1::None,
        )
    }

    pub fn projection(&self) -> &K2EpisodeProjectionV1 {
        &self.projection
    }

    pub fn events(&self) -> &[K2EpisodeEventV1] {
        &self.events
    }

    pub fn derive_terminal_seal(
        &self,
        outcome: &K2DecisionOutcomeReceiptV1,
    ) -> K2GoalEnvironmentResultV1<K2DecisionEpisodeSealV1> {
        outcome.validate()?;
        let terminal = self.events.last().ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_terminal_event_missing",
        ))?;
        let terminal_outcome: K2DecisionOutcomeReceiptV1 =
            serde_json::from_value(terminal.event_payload.clone())
                .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_terminal_payload_invalid"))?;
        if self.projection.state != K2EpisodeStateV1::Terminal
            || terminal.event_kind != K2EpisodeEventKindV1::Terminal
            || terminal_outcome != *outcome
            || self.projection.terminal_outcome_root_sha256.as_deref()
                != Some(outcome.outcome_root_sha256.as_str())
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_terminal_seal_binding_invalid",
            ));
        }
        K2DecisionEpisodeSealV1::derive(
            self.episode_id_sha256.clone(),
            outcome.outcome_root_sha256.clone(),
            terminal.entry_root_sha256.clone(),
            self.projection.projection_root_sha256.clone(),
        )
    }

    pub(crate) fn append_with_fault_v1<T: Serialize>(
        &mut self,
        event_kind: K2EpisodeEventKindV1,
        payload: &T,
        written_at_unix_ms: u64,
        fault: K2JournalFaultPointV1,
    ) -> K2GoalEnvironmentResultV1<K2EpisodeEventV1> {
        let event = K2EpisodeEventV1::seal(
            self.episode_id_sha256.clone(),
            self.events.len() as u64,
            self.events
                .last()
                .map(|event| event.entry_root_sha256.clone()),
            event_kind,
            payload,
            written_at_unix_ms,
        )?;
        let bytes = event.canonical_bytes()?;
        if bytes.len() as u64 > K2_MAX_EVENT_BYTES_V1
            || self
                .projection
                .total_event_bytes
                .checked_add(bytes.len() as u64)
                .is_none_or(|total| total > K2_MAX_EPISODE_BYTES_V1)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_episode_byte_budget_exhausted",
            ));
        }
        let mut projected = self.events.clone();
        projected.push(event.clone());
        let next_projection = K2EpisodeProjectionV1::project(&self.episode_id_sha256, &projected)?;
        publish_event_v1(&self.episode_directory, event.sequence, &bytes, fault)?;
        self.events = projected;
        self.projection = next_projection;
        Ok(event)
    }
}

fn publish_event_v1(
    directory: &Path,
    sequence: u64,
    bytes: &[u8],
    fault: K2JournalFaultPointV1,
) -> K2GoalEnvironmentResultV1<()> {
    let final_path = directory.join(event_filename_v1(sequence));
    let temp_path = directory.join(format!(".{}.tmp", event_filename_v1(sequence)));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(0o600);
    let mut file = options
        .open(&temp_path)
        .map_err(io_error_v1("create_event_temp"))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp_path);
        return Err(K2GoalEnvironmentErrorV1::Io(format!(
            "write_event_temp:{error}"
        )));
    }
    drop(file);
    if fault == K2JournalFaultPointV1::AfterTempSync {
        fs::remove_file(&temp_path).map_err(io_error_v1("remove_fault_temp"))?;
        sync_directory_v1(directory)?;
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_injected_after_temp_sync",
        ));
    }
    fs::hard_link(&temp_path, &final_path).map_err(io_error_v1("publish_event_no_replace"))?;
    if fault == K2JournalFaultPointV1::AfterPublishBeforeDirectorySync {
        return Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_injected_after_event_publish",
        ));
    }
    sync_directory_v1(directory)?;
    fs::remove_file(&temp_path).map_err(io_error_v1("remove_event_temp"))?;
    sync_directory_v1(directory)
}

fn sync_directory_v1(directory: &Path) -> K2GoalEnvironmentResultV1<()> {
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(io_error_v1("sync_directory"))
}

fn event_filename_v1(sequence: u64) -> String {
    format!("{sequence:020}.json")
}

fn io_error_v1(operation: &'static str) -> impl FnOnce(std::io::Error) -> K2GoalEnvironmentErrorV1 {
    move |error| K2GoalEnvironmentErrorV1::Io(format!("{operation}:{error}"))
}
