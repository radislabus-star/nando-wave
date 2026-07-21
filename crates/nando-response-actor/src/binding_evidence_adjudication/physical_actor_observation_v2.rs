use serde::{Deserialize, Serialize};

use super::canonical::{is_sha256, pretty_json_bytes, sha256_json};
use super::physical_trial_v2::PhysicalTrialV2Error;

pub const PHYSICAL_ACTOR_OBSERVATION_SCHEMA_V2: &str =
    "nando.binding-physical-actor-observation.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalActorOutcomeV2 {
    Applied,
    Failed,
    Abstained,
    Censored,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalActorObservationInputV2 {
    pub frozen_row_root_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_root_sha256: String,
    pub pre_state_root_sha256: String,
    pub actor_program_digest_sha256: String,
    pub candidate_action_digest_sha256: String,
    pub observed_post_state_root_sha256: String,
    pub observed_delta_root_sha256: String,
    pub actor_outcome: PhysicalActorOutcomeV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhysicalActorObservationV2 {
    pub schema: String,
    pub observation_sha256: String,
    pub frozen_row_root_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub capture_root_sha256: String,
    pub pre_state_root_sha256: String,
    pub actor_program_digest_sha256: String,
    pub candidate_action_digest_sha256: String,
    pub observed_post_state_root_sha256: String,
    pub observed_delta_root_sha256: String,
    pub actor_outcome: PhysicalActorOutcomeV2,
}

impl PhysicalActorObservationV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, PhysicalTrialV2Error> {
        pretty_json_bytes(self).map_err(PhysicalTrialV2Error::from)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, PhysicalTrialV2Error> {
        let observation: Self =
            serde_json::from_slice(bytes).map_err(|_| PhysicalTrialV2Error::InvalidActor)?;
        if observation.canonical_bytes()? != bytes {
            return Err(PhysicalTrialV2Error::InvalidActor);
        }
        validate_physical_actor_observation_v2(&observation)?;
        Ok(observation)
    }
}

pub fn observe_physical_actor_v2(
    input: PhysicalActorObservationInputV2,
) -> Result<PhysicalActorObservationV2, PhysicalTrialV2Error> {
    let mut observation = PhysicalActorObservationV2 {
        schema: PHYSICAL_ACTOR_OBSERVATION_SCHEMA_V2.to_owned(),
        observation_sha256: String::new(),
        frozen_row_root_sha256: input.frozen_row_root_sha256,
        frozen_graph_root_sha256: input.frozen_graph_root_sha256,
        capture_root_sha256: input.capture_root_sha256,
        pre_state_root_sha256: input.pre_state_root_sha256,
        actor_program_digest_sha256: input.actor_program_digest_sha256,
        candidate_action_digest_sha256: input.candidate_action_digest_sha256,
        observed_post_state_root_sha256: input.observed_post_state_root_sha256,
        observed_delta_root_sha256: input.observed_delta_root_sha256,
        actor_outcome: input.actor_outcome,
    };
    validate_actor_roots_v2(&observation)?;
    observation.observation_sha256 = physical_actor_observation_digest_v2(&observation)?;
    Ok(observation)
}

pub(crate) fn validate_physical_actor_observation_v2(
    observation: &PhysicalActorObservationV2,
) -> Result<(), PhysicalTrialV2Error> {
    if observation.schema != PHYSICAL_ACTOR_OBSERVATION_SCHEMA_V2
        || observation.observation_sha256 != physical_actor_observation_digest_v2(observation)?
    {
        return Err(PhysicalTrialV2Error::InvalidActor);
    }
    validate_actor_roots_v2(observation)
}

pub(crate) fn physical_actor_observation_digest_v2(
    observation: &PhysicalActorObservationV2,
) -> Result<String, PhysicalTrialV2Error> {
    sha256_json(&(
        observation.schema.as_str(),
        observation.frozen_row_root_sha256.as_str(),
        observation.frozen_graph_root_sha256.as_str(),
        observation.capture_root_sha256.as_str(),
        observation.pre_state_root_sha256.as_str(),
        observation.actor_program_digest_sha256.as_str(),
        observation.candidate_action_digest_sha256.as_str(),
        observation.observed_post_state_root_sha256.as_str(),
        observation.observed_delta_root_sha256.as_str(),
        observation.actor_outcome,
    ))
    .map_err(PhysicalTrialV2Error::from)
}

fn validate_actor_roots_v2(
    observation: &PhysicalActorObservationV2,
) -> Result<(), PhysicalTrialV2Error> {
    let roots = [
        observation.frozen_row_root_sha256.as_str(),
        observation.frozen_graph_root_sha256.as_str(),
        observation.capture_root_sha256.as_str(),
        observation.pre_state_root_sha256.as_str(),
        observation.actor_program_digest_sha256.as_str(),
        observation.candidate_action_digest_sha256.as_str(),
        observation.observed_post_state_root_sha256.as_str(),
        observation.observed_delta_root_sha256.as_str(),
    ];
    if roots.into_iter().all(is_sha256) {
        Ok(())
    } else {
        Err(PhysicalTrialV2Error::InvalidDigest)
    }
}
