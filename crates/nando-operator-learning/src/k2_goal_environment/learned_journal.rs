use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::learned_capability::{
    K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1, K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1,
    K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1, K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1,
    K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1, K2_LEARNED_SUPPORT_PROBE_COUNT_V1,
    K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1, K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
    K2_SUPPORT_DISPATCH_SCHEMA_V1, K2_SUPPORT_OBSERVATION_SCHEMA_V1,
    K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1, K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1,
    K2_V1_EPISODE_EVIDENCE_SCHEMA_V1, K2LearnedAblationReceiptV1,
    K2LearnedCapabilityEvidenceClassV1, K2LearnedCapabilityFreezeV1, K2LearnedCapabilityOutcomeV1,
    K2LearnedCapabilitySealV1, K2LearnedEffectLawSetV1, K2LearnedEffectVerificationReceiptV1,
    K2LearnedTargetPredictionSetV1, K2LearnedToV1BindingV1, K2SupportDispatchV1,
    K2SupportObservationSetV1, K2SupportObservationV1, K2TargetIndependenceReceiptV1,
    K2V1EpisodeEvidenceV1,
};
use super::{K2AuthorityBoundaryV1, K2GoalEnvironmentErrorV1, K2GoalEnvironmentResultV1};

pub const K2_LEARNED_CAPABILITY_EVENT_SCHEMA_V1: &str = "nando.k2-learned-capability-event.v1";
pub const K2_LEARNED_CAPABILITY_PROJECTION_SCHEMA_V1: &str =
    "nando.k2-learned-capability-projection.v1";
pub const K2_LEARNED_CAPABILITY_MAX_EVENTS_V1: u64 = 24;
pub const K2_LEARNED_CAPABILITY_MAX_EVENT_BYTES_V1: u64 = 128 * 1024;
pub const K2_LEARNED_CAPABILITY_MAX_JOURNAL_BYTES_V1: u64 = 3 * 1024 * 1024;
pub const K2_LEARNED_CAPABILITY_MAX_RETAINED_EXPERIMENTS_V1: u64 = 8;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedCapabilityEventKindV1 {
    ExperimentFrozen,
    SupportDispatched,
    SupportObserved,
    SupportEvidenceFrozen,
    LearnedLawsFrozen,
    TargetIndependenceFrozen,
    TargetPredictionsFrozen,
    IndependentVerificationFrozen,
    LearnedToV1BindingFrozen,
    V1EpisodeObserved,
    AblationsFrozen,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2LearnedCapabilityStateV1 {
    Empty,
    Frozen,
    SupportRunning,
    SupportComplete,
    LawsFrozen,
    HoldoutVerified,
    TargetPredictionsFrozen,
    PredictionsVerified,
    LearnedToV1BindingFrozen,
    TargetEpisodeComplete,
    AblationsComplete,
    Terminal,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityEventV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub sequence: u64,
    pub event_kind: K2LearnedCapabilityEventKindV1,
    pub payload_schema: String,
    pub payload_root_sha256: String,
    pub previous_entry_root_sha256: Option<String>,
    pub entry_root_sha256: String,
    pub event_payload: Value,
    pub recorded_at_unix_ms: u64,
}

#[derive(Serialize)]
struct K2LearnedCapabilityEventDigestV1<'a> {
    schema: &'static str,
    experiment_id_sha256: &'a str,
    sequence: u64,
    event_kind: K2LearnedCapabilityEventKindV1,
    payload_schema: &'a str,
    payload_root_sha256: &'a str,
    previous_entry_root_sha256: Option<&'a str>,
    event_payload: &'a Value,
    recorded_at_unix_ms: u64,
}

struct K2LearnedCapabilityEventInputV1<'a, T> {
    experiment_id_sha256: String,
    sequence: u64,
    event_kind: K2LearnedCapabilityEventKindV1,
    payload_schema: &'a str,
    payload_root_sha256: &'a str,
    previous_entry_root_sha256: Option<String>,
    payload: &'a T,
    recorded_at_unix_ms: u64,
}

impl K2LearnedCapabilityEventV1 {
    fn seal<T: Serialize>(
        input: K2LearnedCapabilityEventInputV1<'_, T>,
    ) -> K2GoalEnvironmentResultV1<Self> {
        let event_payload = serde_json::to_value(input.payload)
            .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)?;
        let mut event = Self {
            schema: K2_LEARNED_CAPABILITY_EVENT_SCHEMA_V1.to_owned(),
            experiment_id_sha256: input.experiment_id_sha256,
            sequence: input.sequence,
            event_kind: input.event_kind,
            payload_schema: input.payload_schema.to_owned(),
            payload_root_sha256: input.payload_root_sha256.to_owned(),
            previous_entry_root_sha256: input.previous_entry_root_sha256,
            entry_root_sha256: String::new(),
            event_payload,
            recorded_at_unix_ms: input.recorded_at_unix_ms,
        };
        event.entry_root_sha256 = event.expected_root_v1()?;
        event.validate()?;
        Ok(event)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        for root in [
            self.experiment_id_sha256.as_str(),
            self.payload_root_sha256.as_str(),
            self.entry_root_sha256.as_str(),
        ] {
            if !valid_nonzero_sha256(root) {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_event_root_invalid",
                ));
            }
        }
        if self
            .previous_entry_root_sha256
            .as_deref()
            .is_some_and(|root| !valid_nonzero_sha256(root))
            || (self.sequence == 0) != self.previous_entry_root_sha256.is_none()
            || self.payload_schema != expected_payload_schema_v1(self.event_kind)
            || self.schema != K2_LEARNED_CAPABILITY_EVENT_SCHEMA_V1
            || self.entry_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_event_invalid",
            ));
        }
        Ok(())
    }

    pub fn canonical_bytes_v1(&self) -> K2GoalEnvironmentResultV1<Vec<u8>> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_json_sha256(&K2LearnedCapabilityEventDigestV1 {
            schema: K2_LEARNED_CAPABILITY_EVENT_SCHEMA_V1,
            experiment_id_sha256: &self.experiment_id_sha256,
            sequence: self.sequence,
            event_kind: self.event_kind,
            payload_schema: &self.payload_schema,
            payload_root_sha256: &self.payload_root_sha256,
            previous_entry_root_sha256: self.previous_entry_root_sha256.as_deref(),
            event_payload: &self.event_payload,
            recorded_at_unix_ms: self.recorded_at_unix_ms,
        })
        .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2LearnedCapabilityProjectionV1 {
    pub schema: String,
    pub projection_root_sha256: String,
    pub experiment_id_sha256: String,
    pub state: K2LearnedCapabilityStateV1,
    pub event_count: u64,
    pub total_event_bytes: u64,
    pub latest_entry_root_sha256: Option<String>,
    pub freeze_root_sha256: Option<String>,
    pub support_dispatch_roots_sha256: Vec<String>,
    pub support_observation_roots_sha256: Vec<String>,
    pub support_evidence_set_root_sha256: Option<String>,
    pub learning_request_root_sha256: Option<String>,
    pub learned_law_set_root_sha256: Option<String>,
    pub target_independence_receipt_root_sha256: Option<String>,
    pub target_prediction_request_root_sha256: Option<String>,
    pub target_prediction_set_root_sha256: Option<String>,
    pub independent_verification_root_sha256: Option<String>,
    pub learned_to_v1_binding_root_sha256: Option<String>,
    pub v1_episode_evidence_root_sha256: Option<String>,
    pub ablation_receipt_root_sha256: Option<String>,
    pub terminal_outcome_root_sha256: Option<String>,
    pub terminal_evidence_class: Option<K2LearnedCapabilityEvidenceClassV1>,
    pub next_support_ordinal: u64,
    pub same_identity_support_dispatch_allowed: bool,
    pub indeterminate_after_support_dispatch: bool,
    pub authority: K2AuthorityBoundaryV1,
}

#[derive(Serialize)]
struct K2LearnedCapabilityProjectionDigestV1<'a> {
    schema: &'static str,
    experiment_id_sha256: &'a str,
    state: K2LearnedCapabilityStateV1,
    event_count: u64,
    total_event_bytes: u64,
    latest_entry_root_sha256: Option<&'a str>,
    freeze_root_sha256: Option<&'a str>,
    support_dispatch_roots_sha256: &'a [String],
    support_observation_roots_sha256: &'a [String],
    support_evidence_set_root_sha256: Option<&'a str>,
    learning_request_root_sha256: Option<&'a str>,
    learned_law_set_root_sha256: Option<&'a str>,
    target_independence_receipt_root_sha256: Option<&'a str>,
    target_prediction_request_root_sha256: Option<&'a str>,
    target_prediction_set_root_sha256: Option<&'a str>,
    independent_verification_root_sha256: Option<&'a str>,
    learned_to_v1_binding_root_sha256: Option<&'a str>,
    v1_episode_evidence_root_sha256: Option<&'a str>,
    ablation_receipt_root_sha256: Option<&'a str>,
    terminal_outcome_root_sha256: Option<&'a str>,
    terminal_evidence_class: Option<K2LearnedCapabilityEvidenceClassV1>,
    next_support_ordinal: u64,
    same_identity_support_dispatch_allowed: bool,
    indeterminate_after_support_dispatch: bool,
    authority: &'a K2AuthorityBoundaryV1,
}

impl K2LearnedCapabilityProjectionV1 {
    pub fn project(
        experiment_id_sha256: &str,
        events: &[K2LearnedCapabilityEventV1],
    ) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(experiment_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_experiment_id_invalid",
            ));
        }
        if events.len() as u64 > K2_LEARNED_CAPABILITY_MAX_EVENTS_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_event_budget_exhausted",
            ));
        }
        let mut replay = K2LearnedRootReplayV1::default();
        let mut previous_root = None;
        let mut total_event_bytes = 0_u64;
        for (sequence, event) in events.iter().enumerate() {
            event.validate()?;
            let bytes = event.canonical_bytes_v1()?;
            let event_bytes = bytes.len() as u64;
            total_event_bytes = total_event_bytes.checked_add(event_bytes).ok_or(
                K2GoalEnvironmentErrorV1::Invalid("k2_learned_journal_bytes_overflow"),
            )?;
            if event.experiment_id_sha256 != experiment_id_sha256 {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_cross_experiment_replay",
                ));
            }
            if event_bytes > K2_LEARNED_CAPABILITY_MAX_EVENT_BYTES_V1
                || total_event_bytes > K2_LEARNED_CAPABILITY_MAX_JOURNAL_BYTES_V1
                || event.sequence != sequence as u64
                || event.event_kind != expected_event_kind_v1(sequence as u64)?
                || event.previous_entry_root_sha256.as_deref() != previous_root
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_journal_chain_invalid",
                ));
            }
            replay.observe(event)?;
            previous_root = Some(event.entry_root_sha256.as_str());
        }
        let state = state_for_event_count_v1(events.len() as u64)?;
        let indeterminate_after_support_dispatch = events.last().is_some_and(|event| {
            event.event_kind == K2LearnedCapabilityEventKindV1::SupportDispatched
        });
        let same_identity_support_dispatch_allowed = matches!(
            next_event_kind_v1(events.len() as u64),
            Ok(K2LearnedCapabilityEventKindV1::SupportDispatched)
        );
        let mut projection = Self {
            schema: K2_LEARNED_CAPABILITY_PROJECTION_SCHEMA_V1.to_owned(),
            projection_root_sha256: String::new(),
            experiment_id_sha256: experiment_id_sha256.to_owned(),
            state,
            event_count: events.len() as u64,
            total_event_bytes,
            latest_entry_root_sha256: events.last().map(|event| event.entry_root_sha256.clone()),
            freeze_root_sha256: replay.freeze.map(|value| value.freeze_root_sha256),
            support_dispatch_roots_sha256: replay.dispatch_roots,
            support_observation_roots_sha256: replay.observation_roots,
            support_evidence_set_root_sha256: replay
                .observations
                .map(|value| value.observation_set_root_sha256),
            learning_request_root_sha256: replay
                .laws
                .as_ref()
                .map(|value| value.learning_request_root_sha256.clone()),
            learned_law_set_root_sha256: replay.laws.map(|value| value.law_set_root_sha256),
            target_independence_receipt_root_sha256: replay
                .independence
                .map(|value| value.receipt_root_sha256),
            target_prediction_request_root_sha256: replay
                .predictions
                .as_ref()
                .map(|value| value.target_prediction_request_root_sha256.clone()),
            target_prediction_set_root_sha256: replay
                .predictions
                .map(|value| value.prediction_set_root_sha256),
            independent_verification_root_sha256: replay
                .verification
                .map(|value| value.verification_root_sha256),
            learned_to_v1_binding_root_sha256: replay
                .binding
                .map(|value| value.binding_root_sha256),
            v1_episode_evidence_root_sha256: replay
                .v1_episode
                .map(|value| value.evidence_root_sha256),
            ablation_receipt_root_sha256: replay.ablations.map(|value| value.receipt_root_sha256),
            terminal_outcome_root_sha256: replay
                .outcome
                .as_ref()
                .map(|value| value.outcome_root_sha256.clone()),
            terminal_evidence_class: replay.outcome.map(|value| value.evidence_class),
            next_support_ordinal: events
                .iter()
                .filter(|event| event.event_kind == K2LearnedCapabilityEventKindV1::SupportObserved)
                .count() as u64,
            same_identity_support_dispatch_allowed,
            indeterminate_after_support_dispatch,
            authority: K2AuthorityBoundaryV1::authority_free_v1(),
        };
        projection.projection_root_sha256 = projection.expected_root_v1()?;
        projection.validate()?;
        Ok(projection)
    }

    pub fn validate(&self) -> K2GoalEnvironmentResultV1<()> {
        self.authority.validate()?;
        if self.schema != K2_LEARNED_CAPABILITY_PROJECTION_SCHEMA_V1
            || !valid_nonzero_sha256(&self.projection_root_sha256)
            || !valid_nonzero_sha256(&self.experiment_id_sha256)
            || self.event_count > K2_LEARNED_CAPABILITY_MAX_EVENTS_V1
            || self.total_event_bytes > K2_LEARNED_CAPABILITY_MAX_JOURNAL_BYTES_V1
            || self.support_dispatch_roots_sha256.len() > K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_observation_roots_sha256.len() > K2_LEARNED_SUPPORT_PROBE_COUNT_V1
            || self.support_observation_roots_sha256.len()
                > self.support_dispatch_roots_sha256.len()
            || self.next_support_ordinal != self.support_observation_roots_sha256.len() as u64
            || self.indeterminate_after_support_dispatch
                != (self.support_dispatch_roots_sha256.len()
                    == self.support_observation_roots_sha256.len() + 1)
            || self.terminal_outcome_root_sha256.is_some()
                != (self.state == K2LearnedCapabilityStateV1::Terminal)
            || self.terminal_evidence_class.is_some() != self.terminal_outcome_root_sha256.is_some()
            || self.projection_root_sha256 != self.expected_root_v1()?
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_projection_invalid",
            ));
        }
        for root in self
            .latest_entry_root_sha256
            .iter()
            .chain(self.freeze_root_sha256.iter())
            .chain(self.support_dispatch_roots_sha256.iter())
            .chain(self.support_observation_roots_sha256.iter())
            .chain(self.support_evidence_set_root_sha256.iter())
            .chain(self.learning_request_root_sha256.iter())
            .chain(self.learned_law_set_root_sha256.iter())
            .chain(self.target_independence_receipt_root_sha256.iter())
            .chain(self.target_prediction_request_root_sha256.iter())
            .chain(self.target_prediction_set_root_sha256.iter())
            .chain(self.independent_verification_root_sha256.iter())
            .chain(self.learned_to_v1_binding_root_sha256.iter())
            .chain(self.v1_episode_evidence_root_sha256.iter())
            .chain(self.ablation_receipt_root_sha256.iter())
            .chain(self.terminal_outcome_root_sha256.iter())
        {
            if !valid_nonzero_sha256(root) {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_projection_root_invalid",
                ));
            }
        }
        Ok(())
    }

    fn expected_root_v1(&self) -> K2GoalEnvironmentResultV1<String> {
        canonical_json_sha256(&K2LearnedCapabilityProjectionDigestV1 {
            schema: K2_LEARNED_CAPABILITY_PROJECTION_SCHEMA_V1,
            experiment_id_sha256: &self.experiment_id_sha256,
            state: self.state,
            event_count: self.event_count,
            total_event_bytes: self.total_event_bytes,
            latest_entry_root_sha256: self.latest_entry_root_sha256.as_deref(),
            freeze_root_sha256: self.freeze_root_sha256.as_deref(),
            support_dispatch_roots_sha256: &self.support_dispatch_roots_sha256,
            support_observation_roots_sha256: &self.support_observation_roots_sha256,
            support_evidence_set_root_sha256: self.support_evidence_set_root_sha256.as_deref(),
            learning_request_root_sha256: self.learning_request_root_sha256.as_deref(),
            learned_law_set_root_sha256: self.learned_law_set_root_sha256.as_deref(),
            target_independence_receipt_root_sha256: self
                .target_independence_receipt_root_sha256
                .as_deref(),
            target_prediction_request_root_sha256: self
                .target_prediction_request_root_sha256
                .as_deref(),
            target_prediction_set_root_sha256: self.target_prediction_set_root_sha256.as_deref(),
            independent_verification_root_sha256: self
                .independent_verification_root_sha256
                .as_deref(),
            learned_to_v1_binding_root_sha256: self.learned_to_v1_binding_root_sha256.as_deref(),
            v1_episode_evidence_root_sha256: self.v1_episode_evidence_root_sha256.as_deref(),
            ablation_receipt_root_sha256: self.ablation_receipt_root_sha256.as_deref(),
            terminal_outcome_root_sha256: self.terminal_outcome_root_sha256.as_deref(),
            terminal_evidence_class: self.terminal_evidence_class,
            next_support_ordinal: self.next_support_ordinal,
            same_identity_support_dispatch_allowed: self.same_identity_support_dispatch_allowed,
            indeterminate_after_support_dispatch: self.indeterminate_after_support_dispatch,
            authority: &self.authority,
        })
        .map_err(|_| K2GoalEnvironmentErrorV1::Serialization)
    }
}

#[derive(Default)]
struct K2LearnedRootReplayV1 {
    freeze: Option<K2LearnedCapabilityFreezeV1>,
    dispatches: Vec<K2SupportDispatchV1>,
    dispatch_roots: Vec<String>,
    observation_roots: Vec<String>,
    observations: Option<K2SupportObservationSetV1>,
    laws: Option<K2LearnedEffectLawSetV1>,
    independence: Option<K2TargetIndependenceReceiptV1>,
    predictions: Option<K2LearnedTargetPredictionSetV1>,
    verification: Option<K2LearnedEffectVerificationReceiptV1>,
    binding: Option<K2LearnedToV1BindingV1>,
    v1_episode: Option<K2V1EpisodeEvidenceV1>,
    ablations: Option<K2LearnedAblationReceiptV1>,
    outcome: Option<K2LearnedCapabilityOutcomeV1>,
}

impl K2LearnedRootReplayV1 {
    fn observe(&mut self, event: &K2LearnedCapabilityEventV1) -> K2GoalEnvironmentResultV1<()> {
        match event.event_kind {
            K2LearnedCapabilityEventKindV1::ExperimentFrozen => {
                let freeze: K2LearnedCapabilityFreezeV1 = decode_payload_v1(event)?;
                freeze.validate_persisted_v1()?;
                if freeze.experiment_id_sha256 != event.experiment_id_sha256
                    || event.payload_root_sha256 != freeze.freeze_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.freeze = Some(freeze);
            }
            K2LearnedCapabilityEventKindV1::SupportDispatched => {
                let dispatch: K2SupportDispatchV1 = decode_payload_v1(event)?;
                dispatch.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                if dispatch.experiment_freeze_root_sha256 != freeze.freeze_root_sha256
                    || dispatch.probe_ordinal != self.dispatches.len() as u64
                    || event.payload_root_sha256 != dispatch.dispatch_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.dispatch_roots
                    .push(dispatch.dispatch_root_sha256.clone());
                self.dispatches.push(dispatch);
            }
            K2LearnedCapabilityEventKindV1::SupportObserved => {
                let observation: K2SupportObservationV1 = decode_payload_v1(event)?;
                observation.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let dispatch = self.dispatches.last().ok_or_else(replay_error_v1)?;
                if self.observation_roots.len() + 1 != self.dispatches.len()
                    || observation.probe_ordinal != self.observation_roots.len() as u64
                    || observation.dispatch_root_sha256 != dispatch.dispatch_root_sha256
                    || observation.public_context_root_sha256 != freeze.public_context_root_sha256
                    || observation.support_world_root_sha256 != dispatch.support_world_root_sha256
                    || observation.action_id_sha256 != dispatch.action_id_sha256
                    || event.payload_root_sha256 != observation.observation_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.observation_roots
                    .push(observation.observation_root_sha256);
            }
            K2LearnedCapabilityEventKindV1::SupportEvidenceFrozen => {
                let observations: K2SupportObservationSetV1 = decode_payload_v1(event)?;
                observations.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let observed_roots = observations
                    .observations
                    .iter()
                    .map(|value| value.observation_root_sha256.as_str())
                    .collect::<Vec<_>>();
                if observations.public_context_root_sha256 != freeze.public_context_root_sha256
                    || observed_roots
                        != self
                            .observation_roots
                            .iter()
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                    || event.payload_root_sha256 != observations.observation_set_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.observations = Some(observations);
            }
            K2LearnedCapabilityEventKindV1::LearnedLawsFrozen => {
                let laws: K2LearnedEffectLawSetV1 = decode_payload_v1(event)?;
                laws.validate()?;
                let freeze = self.freeze()?;
                let observations = self.observations()?;
                if laws.public_context_root_sha256 != freeze.public_context_root_sha256
                    || laws.learner_manifest_root_sha256 != freeze.learner_manifest_root_sha256
                    || laws.learner_executable_sha256 != freeze.learner_executable_sha256
                    || laws.support_observation_set_root_sha256
                        != observations.observation_set_root_sha256
                    || event.payload_root_sha256 != laws.law_set_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.laws = Some(laws);
            }
            K2LearnedCapabilityEventKindV1::TargetIndependenceFrozen => {
                let independence: K2TargetIndependenceReceiptV1 = decode_payload_v1(event)?;
                independence.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                if independence.support_set_root_sha256 != freeze.support_set_root_sha256
                    || event.payload_root_sha256 != independence.receipt_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.independence = Some(independence);
            }
            K2LearnedCapabilityEventKindV1::TargetPredictionsFrozen => {
                let predictions: K2LearnedTargetPredictionSetV1 = decode_payload_v1(event)?;
                predictions.validate()?;
                let freeze = self.freeze()?;
                let laws = self.laws()?;
                let independence = self.independence()?;
                if predictions.public_context_root_sha256 != freeze.public_context_root_sha256
                    || predictions.learner_manifest_root_sha256
                        != freeze.learner_manifest_root_sha256
                    || predictions.learner_executable_sha256 != freeze.learner_executable_sha256
                    || predictions.learned_law_set_root_sha256 != laws.law_set_root_sha256
                    || predictions.target_pre_tree_root_sha256
                        != independence.target_pre_tree_root_sha256
                    || event.payload_root_sha256 != predictions.prediction_set_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.predictions = Some(predictions);
            }
            K2LearnedCapabilityEventKindV1::IndependentVerificationFrozen => {
                let verification: K2LearnedEffectVerificationReceiptV1 = decode_payload_v1(event)?;
                verification.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let observations = self.observations()?;
                let laws = self.laws()?;
                let predictions = self.predictions()?;
                if verification.experiment_freeze_root_sha256 != freeze.freeze_root_sha256
                    || verification.verifier_contract_root_sha256
                        != freeze.independent_verifier_contract_root_sha256
                    || verification.support_observation_set_root_sha256
                        != observations.observation_set_root_sha256
                    || verification.learned_law_set_root_sha256 != laws.law_set_root_sha256
                    || verification.target_prediction_set_root_sha256
                        != predictions.prediction_set_root_sha256
                    || event.payload_root_sha256 != verification.verification_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.verification = Some(verification);
            }
            K2LearnedCapabilityEventKindV1::LearnedToV1BindingFrozen => {
                let binding: K2LearnedToV1BindingV1 = decode_payload_v1(event)?;
                binding.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                let laws = self.laws()?;
                let predictions = self.predictions()?;
                let verification = self.verification()?;
                if binding.experiment_freeze_root_sha256 != freeze.freeze_root_sha256
                    || binding.hidden_mapping_root_sha256 != freeze.hidden_mapping_root_sha256
                    || binding.learned_law_set_root_sha256 != laws.law_set_root_sha256
                    || binding.target_prediction_set_root_sha256
                        != predictions.prediction_set_root_sha256
                    || binding.independent_verification_root_sha256
                        != verification.verification_root_sha256
                    || event.payload_root_sha256 != binding.binding_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.binding = Some(binding);
            }
            K2LearnedCapabilityEventKindV1::V1EpisodeObserved => {
                let evidence: K2V1EpisodeEvidenceV1 = decode_payload_v1(event)?;
                evidence.validate_persisted_v1()?;
                let binding = self.binding()?;
                if evidence.learned_to_v1_binding_root_sha256 != binding.binding_root_sha256
                    || event.payload_root_sha256 != evidence.evidence_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.v1_episode = Some(evidence);
            }
            K2LearnedCapabilityEventKindV1::AblationsFrozen => {
                let ablations: K2LearnedAblationReceiptV1 = decode_payload_v1(event)?;
                ablations.validate_persisted_v1()?;
                let freeze = self.freeze()?;
                if ablations.experiment_freeze_root_sha256 != freeze.freeze_root_sha256
                    || event.payload_root_sha256 != ablations.receipt_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.ablations = Some(ablations);
            }
            K2LearnedCapabilityEventKindV1::Terminal => {
                let outcome: K2LearnedCapabilityOutcomeV1 = decode_payload_v1(event)?;
                outcome.validate_persisted_v1()?;
                if outcome.experiment_freeze_root_sha256 != self.freeze()?.freeze_root_sha256
                    || outcome.support_dispatch_roots_sha256 != self.dispatch_roots
                    || outcome.support_observation_roots_sha256 != self.observation_roots
                    || outcome.support_evidence_set_root_sha256
                        != self.observations()?.observation_set_root_sha256
                    || outcome.learned_law_set_root_sha256 != self.laws()?.law_set_root_sha256
                    || outcome.target_independence_receipt_root_sha256
                        != self.independence()?.receipt_root_sha256
                    || outcome.target_prediction_set_root_sha256
                        != self.predictions()?.prediction_set_root_sha256
                    || outcome.independent_verification_root_sha256
                        != self.verification()?.verification_root_sha256
                    || outcome.learned_to_v1_binding_root_sha256
                        != self.binding()?.binding_root_sha256
                    || outcome.learning_request_root_sha256
                        != self.laws()?.learning_request_root_sha256
                    || outcome.target_prediction_request_root_sha256
                        != self.predictions()?.target_prediction_request_root_sha256
                    || outcome.v1_decision_freeze_root_sha256
                        != self.v1_episode()?.v1_decision_freeze_root_sha256
                    || outcome.v1_prediction_set_root_sha256
                        != self.v1_episode()?.v1_prediction_set_root_sha256
                    || outcome.v1_selection_root_sha256
                        != self.v1_episode()?.v1_selection_root_sha256
                    || outcome.v1_law_lab_binding_root_sha256
                        != self.v1_episode()?.v1_law_lab_binding_root_sha256
                    || outcome.v1_sandbox_receipt_root_sha256
                        != self.v1_episode()?.v1_sandbox_receipt_root_sha256
                    || outcome.v1_exact_goal_receipt_root_sha256
                        != self.v1_episode()?.v1_exact_goal_receipt_root_sha256
                    || outcome.v1_terminal_outcome_root_sha256
                        != self.v1_episode()?.v1_terminal_outcome_root_sha256
                    || outcome.v1_episode_seal_root_sha256
                        != self.v1_episode()?.v1_episode_seal_root_sha256
                    || outcome.ablation_receipt_root_sha256 != self.ablations()?.receipt_root_sha256
                    || outcome.support_worlds != 3
                    || outcome.support_executions != self.dispatches.len() as u64
                    || outcome.learned_laws != self.laws()?.laws.len() as u64
                    || outcome.target_predictions != self.predictions()?.predictions.len() as u64
                    || outcome.wrong_predictions != self.verification()?.wrong_predictions
                    || event.payload_root_sha256 != outcome.outcome_root_sha256
                {
                    return Err(replay_error_v1());
                }
                self.outcome = Some(outcome);
            }
        }
        Ok(())
    }

    fn freeze(&self) -> K2GoalEnvironmentResultV1<&K2LearnedCapabilityFreezeV1> {
        self.freeze.as_ref().ok_or_else(replay_error_v1)
    }

    fn observations(&self) -> K2GoalEnvironmentResultV1<&K2SupportObservationSetV1> {
        self.observations.as_ref().ok_or_else(replay_error_v1)
    }

    fn laws(&self) -> K2GoalEnvironmentResultV1<&K2LearnedEffectLawSetV1> {
        self.laws.as_ref().ok_or_else(replay_error_v1)
    }

    fn independence(&self) -> K2GoalEnvironmentResultV1<&K2TargetIndependenceReceiptV1> {
        self.independence.as_ref().ok_or_else(replay_error_v1)
    }

    fn predictions(&self) -> K2GoalEnvironmentResultV1<&K2LearnedTargetPredictionSetV1> {
        self.predictions.as_ref().ok_or_else(replay_error_v1)
    }

    fn verification(&self) -> K2GoalEnvironmentResultV1<&K2LearnedEffectVerificationReceiptV1> {
        self.verification.as_ref().ok_or_else(replay_error_v1)
    }

    fn binding(&self) -> K2GoalEnvironmentResultV1<&K2LearnedToV1BindingV1> {
        self.binding.as_ref().ok_or_else(replay_error_v1)
    }

    fn v1_episode(&self) -> K2GoalEnvironmentResultV1<&K2V1EpisodeEvidenceV1> {
        self.v1_episode.as_ref().ok_or_else(replay_error_v1)
    }

    fn ablations(&self) -> K2GoalEnvironmentResultV1<&K2LearnedAblationReceiptV1> {
        self.ablations.as_ref().ok_or_else(replay_error_v1)
    }
}

fn replay_error_v1() -> K2GoalEnvironmentErrorV1 {
    K2GoalEnvironmentErrorV1::Invalid("k2_learned_cross_event_replay_invalid")
}

fn decode_payload_v1<T: DeserializeOwned>(
    event: &K2LearnedCapabilityEventV1,
) -> K2GoalEnvironmentResultV1<T> {
    serde_json::from_value(event.event_payload.clone())
        .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_learned_event_payload_invalid"))
}

fn expected_event_kind_v1(
    sequence: u64,
) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventKindV1> {
    match sequence {
        0 => Ok(K2LearnedCapabilityEventKindV1::ExperimentFrozen),
        1..=12 if sequence % 2 == 1 => Ok(K2LearnedCapabilityEventKindV1::SupportDispatched),
        1..=12 => Ok(K2LearnedCapabilityEventKindV1::SupportObserved),
        13 => Ok(K2LearnedCapabilityEventKindV1::SupportEvidenceFrozen),
        14 => Ok(K2LearnedCapabilityEventKindV1::LearnedLawsFrozen),
        15 => Ok(K2LearnedCapabilityEventKindV1::TargetIndependenceFrozen),
        16 => Ok(K2LearnedCapabilityEventKindV1::TargetPredictionsFrozen),
        17 => Ok(K2LearnedCapabilityEventKindV1::IndependentVerificationFrozen),
        18 => Ok(K2LearnedCapabilityEventKindV1::LearnedToV1BindingFrozen),
        19 => Ok(K2LearnedCapabilityEventKindV1::V1EpisodeObserved),
        20 => Ok(K2LearnedCapabilityEventKindV1::AblationsFrozen),
        21 => Ok(K2LearnedCapabilityEventKindV1::Terminal),
        _ => Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_learned_event_sequence_invalid",
        )),
    }
}

fn next_event_kind_v1(
    event_count: u64,
) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventKindV1> {
    expected_event_kind_v1(event_count)
}

fn expected_payload_schema_v1(kind: K2LearnedCapabilityEventKindV1) -> &'static str {
    match kind {
        K2LearnedCapabilityEventKindV1::ExperimentFrozen => K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::SupportDispatched => K2_SUPPORT_DISPATCH_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::SupportObserved => K2_SUPPORT_OBSERVATION_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::SupportEvidenceFrozen => {
            K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1
        }
        K2LearnedCapabilityEventKindV1::LearnedLawsFrozen => K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::TargetIndependenceFrozen => {
            K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1
        }
        K2LearnedCapabilityEventKindV1::TargetPredictionsFrozen => {
            K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1
        }
        K2LearnedCapabilityEventKindV1::IndependentVerificationFrozen => {
            K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1
        }
        K2LearnedCapabilityEventKindV1::LearnedToV1BindingFrozen => {
            K2_LEARNED_TO_V1_BINDING_SCHEMA_V1
        }
        K2LearnedCapabilityEventKindV1::V1EpisodeObserved => K2_V1_EPISODE_EVIDENCE_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::AblationsFrozen => K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1,
        K2LearnedCapabilityEventKindV1::Terminal => K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1,
    }
}

fn state_for_event_count_v1(
    event_count: u64,
) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityStateV1> {
    match event_count {
        0 => Ok(K2LearnedCapabilityStateV1::Empty),
        1 => Ok(K2LearnedCapabilityStateV1::Frozen),
        2..=12 => Ok(K2LearnedCapabilityStateV1::SupportRunning),
        13..=14 => Ok(K2LearnedCapabilityStateV1::SupportComplete),
        15 => Ok(K2LearnedCapabilityStateV1::LawsFrozen),
        16 => Ok(K2LearnedCapabilityStateV1::HoldoutVerified),
        17 => Ok(K2LearnedCapabilityStateV1::TargetPredictionsFrozen),
        18 => Ok(K2LearnedCapabilityStateV1::PredictionsVerified),
        19 => Ok(K2LearnedCapabilityStateV1::LearnedToV1BindingFrozen),
        20 => Ok(K2LearnedCapabilityStateV1::TargetEpisodeComplete),
        21 => Ok(K2LearnedCapabilityStateV1::AblationsComplete),
        22 => Ok(K2LearnedCapabilityStateV1::Terminal),
        _ => Err(K2GoalEnvironmentErrorV1::Invalid(
            "k2_learned_projection_event_count_invalid",
        )),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2LearnedJournalFaultPointV1 {
    None,
    AfterTempSync,
    AfterPublishBeforeDirectorySync,
}

pub struct K2LearnedCapabilityJournalV1 {
    experiment_directory: PathBuf,
    experiment_id_sha256: String,
    events: Vec<K2LearnedCapabilityEventV1>,
    projection: K2LearnedCapabilityProjectionV1,
    reopened_after_unobserved_dispatch: bool,
    next_fault: K2LearnedJournalFaultPointV1,
}

impl K2LearnedCapabilityJournalV1 {
    pub fn create(
        store_root: &Path,
        experiment_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(&experiment_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_experiment_id_invalid",
            ));
        }
        fs::create_dir_all(store_root).map_err(journal_io_error_v1("create_store_root"))?;
        let retained = fs::read_dir(store_root)
            .map_err(journal_io_error_v1("read_store_root"))?
            .try_fold(0_u64, |count, entry| {
                let entry = entry.map_err(journal_io_error_v1("read_store_entry"))?;
                if entry
                    .file_type()
                    .map_err(journal_io_error_v1("stat_store_entry"))?
                    .is_dir()
                {
                    count
                        .checked_add(1)
                        .ok_or(K2GoalEnvironmentErrorV1::Invalid(
                            "k2_learned_retained_count_overflow",
                        ))
                } else {
                    Err(K2GoalEnvironmentErrorV1::Invalid(
                        "k2_learned_store_unknown_entry",
                    ))
                }
            })?;
        if retained >= K2_LEARNED_CAPABILITY_MAX_RETAINED_EXPERIMENTS_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_retained_budget_exhausted",
            ));
        }
        let experiment_directory = store_root.join(&experiment_id_sha256);
        fs::create_dir(&experiment_directory)
            .map_err(journal_io_error_v1("create_experiment_directory"))?;
        sync_journal_directory_v1(store_root)?;
        let projection = K2LearnedCapabilityProjectionV1::project(&experiment_id_sha256, &[])?;
        Ok(Self {
            experiment_directory,
            experiment_id_sha256,
            events: Vec::new(),
            projection,
            reopened_after_unobserved_dispatch: false,
            next_fault: K2LearnedJournalFaultPointV1::None,
        })
    }

    pub fn open_existing(
        store_root: &Path,
        experiment_id_sha256: String,
    ) -> K2GoalEnvironmentResultV1<Self> {
        if !valid_nonzero_sha256(&experiment_id_sha256) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_experiment_id_invalid",
            ));
        }
        let experiment_directory = store_root.join(&experiment_id_sha256);
        if !experiment_directory.is_dir() {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_experiment_directory_missing",
            ));
        }
        let mut paths = fs::read_dir(&experiment_directory)
            .map_err(journal_io_error_v1("read_experiment_directory"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(journal_io_error_v1("read_experiment_entry"))?;
        paths.sort();
        if paths.len() as u64 > K2_LEARNED_CAPABILITY_MAX_EVENTS_V1 {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_event_budget_exhausted",
            ));
        }
        let mut events = Vec::with_capacity(paths.len());
        for (sequence, path) in paths.iter().enumerate() {
            if path.file_name().and_then(|name| name.to_str())
                != Some(event_filename_v1(sequence as u64).as_str())
            {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_journal_unknown_entry",
                ));
            }
            let bytes = fs::read(path).map_err(journal_io_error_v1("read_event"))?;
            if bytes.len() as u64 > K2_LEARNED_CAPABILITY_MAX_EVENT_BYTES_V1 {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_event_bytes_exhausted",
                ));
            }
            let event: K2LearnedCapabilityEventV1 = serde_json::from_slice(&bytes)
                .map_err(|_| K2GoalEnvironmentErrorV1::Invalid("k2_learned_event_decode_failed"))?;
            if event.canonical_bytes_v1()? != bytes {
                return Err(K2GoalEnvironmentErrorV1::Invalid(
                    "k2_learned_event_not_canonical",
                ));
            }
            events.push(event);
        }
        let projection = K2LearnedCapabilityProjectionV1::project(&experiment_id_sha256, &events)?;
        let reopened_after_unobserved_dispatch = projection.indeterminate_after_support_dispatch;
        Ok(Self {
            experiment_directory,
            experiment_id_sha256,
            events,
            projection,
            reopened_after_unobserved_dispatch,
            next_fault: K2LearnedJournalFaultPointV1::None,
        })
    }

    pub fn projection(&self) -> &K2LearnedCapabilityProjectionV1 {
        &self.projection
    }

    pub fn events(&self) -> &[K2LearnedCapabilityEventV1] {
        &self.events
    }

    pub fn set_next_fault_for_test_v1(&mut self, fault: K2LearnedJournalFaultPointV1) {
        self.next_fault = fault;
    }

    pub fn append_freeze(
        &mut self,
        value: &K2LearnedCapabilityFreezeV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::ExperimentFrozen,
            K2_LEARNED_CAPABILITY_FREEZE_SCHEMA_V1,
            &value.freeze_root_sha256,
            value,
            at,
        )
    }

    pub fn append_support_dispatch(
        &mut self,
        value: &K2SupportDispatchV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::SupportDispatched,
            K2_SUPPORT_DISPATCH_SCHEMA_V1,
            &value.dispatch_root_sha256,
            value,
            at,
        )
    }

    pub fn append_support_observation(
        &mut self,
        value: &K2SupportObservationV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::SupportObserved,
            K2_SUPPORT_OBSERVATION_SCHEMA_V1,
            &value.observation_root_sha256,
            value,
            at,
        )
    }

    pub fn append_support_evidence(
        &mut self,
        value: &K2SupportObservationSetV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::SupportEvidenceFrozen,
            K2_SUPPORT_OBSERVATION_SET_SCHEMA_V1,
            &value.observation_set_root_sha256,
            value,
            at,
        )
    }

    pub fn append_laws(
        &mut self,
        value: &K2LearnedEffectLawSetV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::LearnedLawsFrozen,
            K2_LEARNED_EFFECT_LAW_SET_SCHEMA_V1,
            &value.law_set_root_sha256,
            value,
            at,
        )
    }

    pub fn append_independence(
        &mut self,
        value: &K2TargetIndependenceReceiptV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::TargetIndependenceFrozen,
            K2_TARGET_INDEPENDENCE_RECEIPT_SCHEMA_V1,
            &value.receipt_root_sha256,
            value,
            at,
        )
    }

    pub fn append_predictions(
        &mut self,
        value: &K2LearnedTargetPredictionSetV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::TargetPredictionsFrozen,
            K2_LEARNED_TARGET_PREDICTION_SET_SCHEMA_V1,
            &value.prediction_set_root_sha256,
            value,
            at,
        )
    }

    pub fn append_verification(
        &mut self,
        value: &K2LearnedEffectVerificationReceiptV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::IndependentVerificationFrozen,
            K2_LEARNED_EFFECT_VERIFICATION_SCHEMA_V1,
            &value.verification_root_sha256,
            value,
            at,
        )
    }

    pub fn append_v1_binding(
        &mut self,
        value: &K2LearnedToV1BindingV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::LearnedToV1BindingFrozen,
            K2_LEARNED_TO_V1_BINDING_SCHEMA_V1,
            &value.binding_root_sha256,
            value,
            at,
        )
    }

    pub fn append_v1_episode(
        &mut self,
        value: &K2V1EpisodeEvidenceV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::V1EpisodeObserved,
            K2_V1_EPISODE_EVIDENCE_SCHEMA_V1,
            &value.evidence_root_sha256,
            value,
            at,
        )
    }

    pub fn append_ablations(
        &mut self,
        value: &K2LearnedAblationReceiptV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::AblationsFrozen,
            K2_LEARNED_ABLATION_RECEIPT_SCHEMA_V1,
            &value.receipt_root_sha256,
            value,
            at,
        )
    }

    pub fn append_terminal(
        &mut self,
        value: &K2LearnedCapabilityOutcomeV1,
        at: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        self.append_typed_v1(
            K2LearnedCapabilityEventKindV1::Terminal,
            K2_LEARNED_CAPABILITY_OUTCOME_SCHEMA_V1,
            &value.outcome_root_sha256,
            value,
            at,
        )
    }

    pub fn derive_terminal_seal(
        &self,
        outcome: &K2LearnedCapabilityOutcomeV1,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilitySealV1> {
        outcome.validate_persisted_v1()?;
        let terminal = self.events.last().ok_or(K2GoalEnvironmentErrorV1::Invalid(
            "k2_learned_terminal_event_missing",
        ))?;
        if self.projection.state != K2LearnedCapabilityStateV1::Terminal
            || terminal.event_kind != K2LearnedCapabilityEventKindV1::Terminal
            || terminal.payload_root_sha256 != outcome.outcome_root_sha256
            || self.projection.terminal_outcome_root_sha256.as_deref()
                != Some(outcome.outcome_root_sha256.as_str())
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_terminal_seal_binding_invalid",
            ));
        }
        K2LearnedCapabilitySealV1::derive(
            self.experiment_id_sha256.clone(),
            outcome.outcome_root_sha256.clone(),
            terminal.entry_root_sha256.clone(),
            self.projection.projection_root_sha256.clone(),
        )
    }

    fn append_typed_v1<T: Serialize>(
        &mut self,
        event_kind: K2LearnedCapabilityEventKindV1,
        payload_schema: &str,
        payload_root_sha256: &str,
        payload: &T,
        recorded_at_unix_ms: u64,
    ) -> K2GoalEnvironmentResultV1<K2LearnedCapabilityEventV1> {
        if self.reopened_after_unobserved_dispatch {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_indeterminate_after_support_dispatch",
            ));
        }
        let expected = next_event_kind_v1(self.events.len() as u64)?;
        if event_kind != expected || payload_schema != expected_payload_schema_v1(event_kind) {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_event_transition_invalid",
            ));
        }
        let event = K2LearnedCapabilityEventV1::seal(K2LearnedCapabilityEventInputV1 {
            experiment_id_sha256: self.experiment_id_sha256.clone(),
            sequence: self.events.len() as u64,
            event_kind,
            payload_schema,
            payload_root_sha256,
            previous_entry_root_sha256: self
                .events
                .last()
                .map(|event| event.entry_root_sha256.clone()),
            payload,
            recorded_at_unix_ms,
        })?;
        let bytes = event.canonical_bytes_v1()?;
        if bytes.len() as u64 > K2_LEARNED_CAPABILITY_MAX_EVENT_BYTES_V1
            || self
                .projection
                .total_event_bytes
                .checked_add(bytes.len() as u64)
                .is_none_or(|total| total > K2_LEARNED_CAPABILITY_MAX_JOURNAL_BYTES_V1)
        {
            return Err(K2GoalEnvironmentErrorV1::Invalid(
                "k2_learned_journal_byte_budget_exhausted",
            ));
        }
        let mut projected = self.events.clone();
        projected.push(event.clone());
        let next_projection =
            K2LearnedCapabilityProjectionV1::project(&self.experiment_id_sha256, &projected)?;
        let fault = std::mem::replace(&mut self.next_fault, K2LearnedJournalFaultPointV1::None);
        publish_event_v1(&self.experiment_directory, event.sequence, &bytes, fault)?;
        self.events = projected;
        self.projection = next_projection;
        Ok(event)
    }
}

fn event_filename_v1(sequence: u64) -> String {
    format!("{sequence:020}.json")
}

fn publish_event_v1(
    directory: &Path,
    sequence: u64,
    bytes: &[u8],
    fault: K2LearnedJournalFaultPointV1,
) -> K2GoalEnvironmentResultV1<()> {
    let final_path = directory.join(event_filename_v1(sequence));
    let temp_path = directory.join(format!(".{}.tmp", event_filename_v1(sequence)));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_path)
        .map_err(journal_io_error_v1("create_event_temp"))?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp_path);
        return Err(K2GoalEnvironmentErrorV1::Io(format!(
            "write_event_temp:{error}"
        )));
    }
    drop(file);
    if fault == K2LearnedJournalFaultPointV1::AfterTempSync {
        fs::remove_file(&temp_path).map_err(journal_io_error_v1("remove_fault_temp"))?;
        sync_journal_directory_v1(directory)?;
        return Err(K2GoalEnvironmentErrorV1::Io(
            "fault_after_temp_sync".to_owned(),
        ));
    }
    if let Err(error) = fs::hard_link(&temp_path, &final_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(K2GoalEnvironmentErrorV1::Io(format!(
            "publish_event_no_replace:{error}"
        )));
    }
    fs::remove_file(&temp_path).map_err(journal_io_error_v1("remove_event_temp"))?;
    if fault == K2LearnedJournalFaultPointV1::AfterPublishBeforeDirectorySync {
        return Err(K2GoalEnvironmentErrorV1::Io(
            "fault_after_publish_before_directory_sync".to_owned(),
        ));
    }
    sync_journal_directory_v1(directory)
}

fn sync_journal_directory_v1(path: &Path) -> K2GoalEnvironmentResultV1<()> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(journal_io_error_v1("sync_journal_directory"))
}

fn journal_io_error_v1(
    operation: &'static str,
) -> impl FnOnce(std::io::Error) -> K2GoalEnvironmentErrorV1 {
    move |error| K2GoalEnvironmentErrorV1::Io(format!("{operation}:{error}"))
}
