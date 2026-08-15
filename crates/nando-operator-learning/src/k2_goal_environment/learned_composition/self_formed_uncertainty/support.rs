use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2CompositionTreeManifestV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1,
    K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1, K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1,
    K2_UNCERTAINTY_SUPPORT_ROWS_V1, K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1,
    K2UncertaintyDomainVocabularyV1, denied_authority_v1, require_denied_authority_v1,
    require_exact_len_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyTransitionReasonV1 {
    Applied,
    CopySourceMissing,
    RemovePathMissing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySupportOutcomeV1 {
    pub schema: String,
    pub transition_applied: bool,
    pub transition_reason: K2UncertaintyTransitionReasonV1,
    pub post_manifest: K2CompositionTreeManifestV1,
    pub observable_outcome_root_sha256: String,
    pub outcome_root_sha256: String,
}

impl K2UncertaintySupportOutcomeV1 {
    pub fn seal(
        transition_reason: K2UncertaintyTransitionReasonV1,
        post_manifest: K2CompositionTreeManifestV1,
    ) -> K2CompositionResultV1<Self> {
        let transition_applied = transition_reason == K2UncertaintyTransitionReasonV1::Applied;
        let observable_outcome_root_sha256 = uncertainty_root_v1(&(
            "nando.k2-self-formed-observable-outcome.v1",
            transition_applied,
            transition_reason,
            &post_manifest,
        ))?;
        let outcome_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1,
            transition_applied,
            transition_reason,
            &post_manifest,
            &observable_outcome_root_sha256,
        ))?;
        let outcome = Self {
            schema: K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1.to_owned(),
            transition_applied,
            transition_reason,
            post_manifest,
            observable_outcome_root_sha256,
            outcome_root_sha256,
        };
        outcome.validate()?;
        Ok(outcome)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.post_manifest.validate()?;
        let applied = self.transition_reason == K2UncertaintyTransitionReasonV1::Applied;
        let observable = uncertainty_root_v1(&(
            "nando.k2-self-formed-observable-outcome.v1",
            applied,
            self.transition_reason,
            &self.post_manifest,
        ))?;
        let root = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1,
            applied,
            self.transition_reason,
            &self.post_manifest,
            &observable,
        ))?;
        if self.schema != K2_UNCERTAINTY_SUPPORT_OUTCOME_SCHEMA_V1
            || self.transition_applied != applied
            || self.observable_outcome_root_sha256 != observable
            || self.outcome_root_sha256 != root
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_support_outcome_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySupportObservationV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub support_sequence: u64,
    pub pre_manifest: K2CompositionTreeManifestV1,
    pub opaque_action_root_sha256: String,
    pub outcome: K2UncertaintySupportOutcomeV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub observation_root_sha256: String,
}

impl K2UncertaintySupportObservationV1 {
    pub fn seal(
        case_id_sha256: String,
        support_sequence: u64,
        pre_manifest: K2CompositionTreeManifestV1,
        opaque_action_root_sha256: String,
        outcome: K2UncertaintySupportOutcomeV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let observation_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1,
            &case_id_sha256,
            support_sequence,
            &pre_manifest,
            &opaque_action_root_sha256,
            &outcome,
            &authority,
        ))?;
        let observation = Self {
            schema: K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1.to_owned(),
            case_id_sha256,
            support_sequence,
            pre_manifest,
            opaque_action_root_sha256,
            outcome,
            authority,
            observation_root_sha256,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.opaque_action_root_sha256)?;
        self.pre_manifest.validate()?;
        self.outcome.validate()?;
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1,
            &self.case_id_sha256,
            self.support_sequence,
            &self.pre_manifest,
            &self.opaque_action_root_sha256,
            &self.outcome,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_SUPPORT_OBSERVATION_SCHEMA_V1
            || self.support_sequence >= K2_UNCERTAINTY_SUPPORT_ROWS_V1 as u64
            || self.observation_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_support_observation_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintySupportSetV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub vocabulary_root_sha256: String,
    pub observations: Vec<K2UncertaintySupportObservationV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub support_root_sha256: String,
}

impl K2UncertaintySupportSetV1 {
    pub fn seal(
        case_id_sha256: String,
        vocabulary_root_sha256: String,
        mut observations: Vec<K2UncertaintySupportObservationV1>,
    ) -> K2CompositionResultV1<Self> {
        observations.sort_by_key(|row| row.support_sequence);
        let authority = denied_authority_v1();
        let support_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1,
            &case_id_sha256,
            &vocabulary_root_sha256,
            &observations,
            &authority,
        ))?;
        let set = Self {
            schema: K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1.to_owned(),
            case_id_sha256,
            vocabulary_root_sha256,
            observations,
            authority,
            support_root_sha256,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.vocabulary_root_sha256)?;
        require_exact_len_v1(
            self.observations.len(),
            K2_UNCERTAINTY_SUPPORT_ROWS_V1,
            "self_formed_support_denominator_invalid",
        )?;
        let mut counts = BTreeMap::<&str, usize>::new();
        let mut pre_states = BTreeSet::new();
        for (sequence, observation) in self.observations.iter().enumerate() {
            observation.validate()?;
            if observation.case_id_sha256 != self.case_id_sha256
                || observation.support_sequence != sequence as u64
                || !pre_states.insert((
                    observation.opaque_action_root_sha256.as_str(),
                    observation.pre_manifest.tree_root_sha256.as_str(),
                ))
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_support_identity_invalid",
                ));
            }
            *counts
                .entry(&observation.opaque_action_root_sha256)
                .or_default() += 1;
        }
        if counts.len() != super::K2_UNCERTAINTY_ACTIONS_V1
            || counts
                .values()
                .any(|count| *count != K2_UNCERTAINTY_SUPPORT_ROWS_PER_ACTION_V1)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_support_action_denominator_invalid",
            ));
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1,
            &self.case_id_sha256,
            &self.vocabulary_root_sha256,
            &self.observations,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_SUPPORT_SET_SCHEMA_V1
            || self.support_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_support_set_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyLearnerRequestV1 {
    pub schema: String,
    pub vocabulary: K2UncertaintyDomainVocabularyV1,
    pub support: K2UncertaintySupportSetV1,
    pub learner_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyLearnerRequestV1 {
    pub fn seal(
        vocabulary: K2UncertaintyDomainVocabularyV1,
        support: K2UncertaintySupportSetV1,
        learner_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = denied_authority_v1();
        let request_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1,
            &vocabulary,
            &support,
            &learner_executable_sha256,
            &authority,
        ))?;
        let request = Self {
            schema: K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1.to_owned(),
            vocabulary,
            support,
            learner_executable_sha256,
            authority,
            request_root_sha256,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.vocabulary.validate()?;
        self.support.validate()?;
        require_composition_root_v1(&self.learner_executable_sha256)?;
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1,
            &self.vocabulary,
            &self.support,
            &self.learner_executable_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_LEARNER_REQUEST_SCHEMA_V1
            || self.vocabulary.case_id_sha256 != self.support.case_id_sha256
            || self.vocabulary.vocabulary_root_sha256 != self.support.vocabulary_root_sha256
            || self.request_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_learner_request_invalid",
            ));
        }
        Ok(())
    }
}
