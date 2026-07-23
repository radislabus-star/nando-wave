use std::fmt;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use crate::{ExactProgramEvaluation, GenerationCensoredReasonV3, GenerationLearningOutcomeV3};

pub const OPERATOR_OBSERVATION_SCHEMA_V1: &str = "nando.operator-observation.v1";
pub const OPERATOR_OBSERVATION_MAX_EVALUATIONS_V1: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorObservationV1 {
    observation_id_sha256: String,
    capture_sequence: u64,
    lineage_root_sha256: String,
    event_root_sha256: String,
    request_root_sha256: String,
    pre_action_relation_root_sha256: String,
    observed_action_root_sha256: String,
    observed_delta_root_sha256: String,
    verifier_receipt_root_sha256: String,
    outcome: GenerationLearningOutcomeV3,
    evaluations: Vec<ExactProgramEvaluation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorObservationInputV1 {
    pub capture_sequence: u64,
    pub lineage_root_sha256: String,
    pub event_root_sha256: String,
    pub request_root_sha256: String,
    pub pre_action_relation_root_sha256: String,
    pub observed_action_root_sha256: String,
    pub observed_delta_root_sha256: String,
    pub verifier_receipt_root_sha256: String,
    pub outcome: GenerationLearningOutcomeV3,
    pub evaluations: Vec<ExactProgramEvaluation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorObservationErrorV1 {
    InvalidSequence,
    InvalidRoot,
    MissingEvaluation,
    UnexpectedEvaluation,
    EvaluationBudgetExhausted,
    InvalidObservation,
    Serialization,
}

impl fmt::Display for OperatorObservationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSequence => "operator observation sequence is invalid",
            Self::InvalidRoot => "operator observation contains an invalid root",
            Self::MissingEvaluation => "semantic observation lacks exact program evaluations",
            Self::UnexpectedEvaluation => "censored observation cannot carry semantic evaluations",
            Self::EvaluationBudgetExhausted => {
                "operator observation exceeds the exact evaluation budget"
            }
            Self::InvalidObservation => "operator observation commitment is invalid",
            Self::Serialization => "operator observation serialization failed",
        })
    }
}

impl std::error::Error for OperatorObservationErrorV1 {}

pub fn seal_operator_observation_v1(
    input: OperatorObservationInputV1,
) -> Result<OperatorObservationV1, OperatorObservationErrorV1> {
    validate_input(&input)?;
    let observation_id_sha256 = observation_digest(&input)?;
    Ok(OperatorObservationV1 {
        observation_id_sha256,
        capture_sequence: input.capture_sequence,
        lineage_root_sha256: input.lineage_root_sha256,
        event_root_sha256: input.event_root_sha256,
        request_root_sha256: input.request_root_sha256,
        pre_action_relation_root_sha256: input.pre_action_relation_root_sha256,
        observed_action_root_sha256: input.observed_action_root_sha256,
        observed_delta_root_sha256: input.observed_delta_root_sha256,
        verifier_receipt_root_sha256: input.verifier_receipt_root_sha256,
        outcome: input.outcome,
        evaluations: input.evaluations,
    })
}

impl OperatorObservationV1 {
    pub fn validate(&self) -> Result<(), OperatorObservationErrorV1> {
        let input = OperatorObservationInputV1 {
            capture_sequence: self.capture_sequence,
            lineage_root_sha256: self.lineage_root_sha256.clone(),
            event_root_sha256: self.event_root_sha256.clone(),
            request_root_sha256: self.request_root_sha256.clone(),
            pre_action_relation_root_sha256: self.pre_action_relation_root_sha256.clone(),
            observed_action_root_sha256: self.observed_action_root_sha256.clone(),
            observed_delta_root_sha256: self.observed_delta_root_sha256.clone(),
            verifier_receipt_root_sha256: self.verifier_receipt_root_sha256.clone(),
            outcome: self.outcome,
            evaluations: self.evaluations.clone(),
        };
        validate_input(&input)?;
        if self.observation_id_sha256 != observation_digest(&input)? {
            return Err(OperatorObservationErrorV1::InvalidObservation);
        }
        Ok(())
    }

    #[must_use]
    pub fn observation_id_sha256(&self) -> &str {
        &self.observation_id_sha256
    }

    #[must_use]
    pub const fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    #[must_use]
    pub fn lineage_root_sha256(&self) -> &str {
        &self.lineage_root_sha256
    }

    #[must_use]
    pub fn event_root_sha256(&self) -> &str {
        &self.event_root_sha256
    }

    #[must_use]
    pub fn request_root_sha256(&self) -> &str {
        &self.request_root_sha256
    }

    #[must_use]
    pub fn pre_action_relation_root_sha256(&self) -> &str {
        &self.pre_action_relation_root_sha256
    }

    #[must_use]
    pub fn observed_action_root_sha256(&self) -> &str {
        &self.observed_action_root_sha256
    }

    #[must_use]
    pub fn observed_delta_root_sha256(&self) -> &str {
        &self.observed_delta_root_sha256
    }

    #[must_use]
    pub fn verifier_receipt_root_sha256(&self) -> &str {
        &self.verifier_receipt_root_sha256
    }

    #[must_use]
    pub const fn outcome(&self) -> GenerationLearningOutcomeV3 {
        self.outcome
    }

    #[must_use]
    pub fn evaluations(&self) -> &[ExactProgramEvaluation] {
        &self.evaluations
    }
}

fn validate_input(input: &OperatorObservationInputV1) -> Result<(), OperatorObservationErrorV1> {
    if input.capture_sequence == 0 {
        return Err(OperatorObservationErrorV1::InvalidSequence);
    }
    if input.evaluations.len() > OPERATOR_OBSERVATION_MAX_EVALUATIONS_V1 {
        return Err(OperatorObservationErrorV1::EvaluationBudgetExhausted);
    }
    let censored = matches!(
        input.outcome,
        GenerationLearningOutcomeV3::Censored(
            GenerationCensoredReasonV3::Timeout
                | GenerationCensoredReasonV3::EnvironmentUnavailable
                | GenerationCensoredReasonV3::MissingPayload
                | GenerationCensoredReasonV3::BudgetExhausted
                | GenerationCensoredReasonV3::VerifierUnavailable
        )
    );
    if censored && !input.evaluations.is_empty() {
        return Err(OperatorObservationErrorV1::UnexpectedEvaluation);
    }
    if !censored && input.evaluations.is_empty() {
        return Err(OperatorObservationErrorV1::MissingEvaluation);
    }
    [
        input.lineage_root_sha256.as_str(),
        input.event_root_sha256.as_str(),
        input.request_root_sha256.as_str(),
        input.pre_action_relation_root_sha256.as_str(),
        input.observed_action_root_sha256.as_str(),
        input.observed_delta_root_sha256.as_str(),
        input.verifier_receipt_root_sha256.as_str(),
    ]
    .into_iter()
    .all(valid_nonzero_sha256)
    .then_some(())
    .ok_or(OperatorObservationErrorV1::InvalidRoot)
}

fn observation_digest(
    input: &OperatorObservationInputV1,
) -> Result<String, OperatorObservationErrorV1> {
    canonical_json_sha256(&(
        OPERATOR_OBSERVATION_SCHEMA_V1,
        input.capture_sequence,
        input.lineage_root_sha256.as_str(),
        input.event_root_sha256.as_str(),
        input.request_root_sha256.as_str(),
        input.pre_action_relation_root_sha256.as_str(),
        input.observed_action_root_sha256.as_str(),
        input.observed_delta_root_sha256.as_str(),
        input.verifier_receipt_root_sha256.as_str(),
        input.outcome,
        &input.evaluations,
    ))
    .map_err(|_| OperatorObservationErrorV1::Serialization)
}
