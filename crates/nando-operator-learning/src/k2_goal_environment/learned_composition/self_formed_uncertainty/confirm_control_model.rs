use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONTROL_PROCESS_OUTCOME_SCHEMA_V1, K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_CONTROL_REQUEST_SCHEMA_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_SUCCESSOR_STATIC_LEGACY_CONTROLS_V1,
    K2_UNCERTAINTY_SUCCESSOR_STATIC_V3_CONTROLS_V1, K2_UNCERTAINTY_SUCCESSOR_STATIC_V4_CONTROLS_V1,
    K2_UNCERTAINTY_V5_CONTROLS_V1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyControlScopeV1 {
    SuccessorStaticLegacy,
    SuccessorStaticV3,
    SuccessorStaticV4,
    DevelopmentRehearsalV5,
    SealedAttemptV5,
}

impl K2UncertaintyControlScopeV1 {
    pub const fn expected_count(self) -> usize {
        match self {
            Self::SuccessorStaticLegacy => K2_UNCERTAINTY_SUCCESSOR_STATIC_LEGACY_CONTROLS_V1,
            Self::SuccessorStaticV3 => K2_UNCERTAINTY_SUCCESSOR_STATIC_V3_CONTROLS_V1,
            Self::SuccessorStaticV4 => K2_UNCERTAINTY_SUCCESSOR_STATIC_V4_CONTROLS_V1,
            Self::DevelopmentRehearsalV5 | Self::SealedAttemptV5 => K2_UNCERTAINTY_V5_CONTROLS_V1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyControlStdoutV1 {
    pub control_id: String,
    pub disposition: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyControlProcessOutcomeV1 {
    pub schema: String,
    pub scope: K2UncertaintyControlScopeV1,
    pub control_id: String,
    pub experiment_root_sha256: String,
    pub freeze_root_sha256: Option<String>,
    pub attempt_root_sha256: Option<String>,
    pub runner_executable_sha256: String,
    pub test_executable_sha256: String,
    pub control_request_root_sha256: String,
    pub normal_exit: bool,
    pub exit_code: i32,
    pub stdout_bytes: Vec<u8>,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub timed_out: bool,
    pub panicked: bool,
    pub decoded_disposition: String,
    pub source_artifact_root_sha256: String,
    pub log_artifact_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2UncertaintyControlProcessOutcomeV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        scope: K2UncertaintyControlScopeV1,
        control_id: String,
        experiment_root_sha256: String,
        freeze_root_sha256: Option<String>,
        attempt_root_sha256: Option<String>,
        runner_executable_sha256: String,
        test_executable_sha256: String,
        control_request_root_sha256: String,
        normal_exit: bool,
        exit_code: i32,
        stdout_bytes: Vec<u8>,
        stderr_sha256: String,
        timed_out: bool,
        panicked: bool,
        decoded_disposition: String,
        source_artifact_root_sha256: String,
        log_artifact_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let stdout_sha256 = super::super::composition_sha256_bytes_v1(&stdout_bytes);
        let mut value = Self {
            schema: K2_UNCERTAINTY_CONTROL_PROCESS_OUTCOME_SCHEMA_V1.to_owned(),
            scope,
            control_id,
            experiment_root_sha256,
            freeze_root_sha256,
            attempt_root_sha256,
            runner_executable_sha256,
            test_executable_sha256,
            control_request_root_sha256,
            normal_exit,
            exit_code,
            stdout_bytes,
            stdout_sha256,
            stderr_sha256,
            timed_out,
            panicked,
            decoded_disposition,
            source_artifact_root_sha256,
            log_artifact_root_sha256,
            authority: denied_authority_v1(),
            outcome_root_sha256: String::new(),
        };
        value.outcome_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_root_sha256,
            &self.runner_executable_sha256,
            &self.test_executable_sha256,
            &self.control_request_root_sha256,
            &self.stdout_sha256,
            &self.stderr_sha256,
            &self.source_artifact_root_sha256,
            &self.log_artifact_root_sha256,
            &self.outcome_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for root in [&self.freeze_root_sha256, &self.attempt_root_sha256]
            .into_iter()
            .flatten()
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONTROL_PROCESS_OUTCOME_SCHEMA_V1
            || self.control_id.is_empty()
            || self.decoded_disposition.is_empty()
            || self.stdout_bytes.is_empty()
            || self.stdout_bytes.len() >= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
            || self.stdout_sha256 != super::super::composition_sha256_bytes_v1(&self.stdout_bytes)
            || self.outcome_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_control_process_outcome_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.stdout_sha256 = super::super::composition_sha256_bytes_v1(&self.stdout_bytes);
        self.outcome_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyControlProcessOutcomeRootV1 {
            schema: K2_UNCERTAINTY_CONTROL_PROCESS_OUTCOME_SCHEMA_V1,
            scope: self.scope,
            control_id: &self.control_id,
            experiment_root_sha256: &self.experiment_root_sha256,
            freeze_root_sha256: &self.freeze_root_sha256,
            attempt_root_sha256: &self.attempt_root_sha256,
            runner_executable_sha256: &self.runner_executable_sha256,
            test_executable_sha256: &self.test_executable_sha256,
            control_request_root_sha256: &self.control_request_root_sha256,
            normal_exit: self.normal_exit,
            exit_code: self.exit_code,
            stdout_bytes: &self.stdout_bytes,
            stdout_sha256: &self.stdout_sha256,
            stderr_sha256: &self.stderr_sha256,
            timed_out: self.timed_out,
            panicked: self.panicked,
            decoded_disposition: &self.decoded_disposition,
            source_artifact_root_sha256: &self.source_artifact_root_sha256,
            log_artifact_root_sha256: &self.log_artifact_root_sha256,
            authority: &self.authority,
        })
    }
}

#[derive(Serialize)]
struct K2UncertaintyControlProcessOutcomeRootV1<'a> {
    schema: &'static str,
    scope: K2UncertaintyControlScopeV1,
    control_id: &'a str,
    experiment_root_sha256: &'a str,
    freeze_root_sha256: &'a Option<String>,
    attempt_root_sha256: &'a Option<String>,
    runner_executable_sha256: &'a str,
    test_executable_sha256: &'a str,
    control_request_root_sha256: &'a str,
    normal_exit: bool,
    exit_code: i32,
    stdout_bytes: &'a [u8],
    stdout_sha256: &'a str,
    stderr_sha256: &'a str,
    timed_out: bool,
    panicked: bool,
    decoded_disposition: &'a str,
    source_artifact_root_sha256: &'a str,
    log_artifact_root_sha256: &'a str,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyControlEvaluationRequestV1 {
    pub schema: String,
    pub scope: K2UncertaintyControlScopeV1,
    pub experiment_root_sha256: String,
    pub freeze_root_sha256: Option<String>,
    pub attempt_root_sha256: Option<String>,
    pub outcomes: Vec<K2UncertaintyControlProcessOutcomeV1>,
    pub evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyControlEvaluationRequestV1 {
    pub fn seal(
        scope: K2UncertaintyControlScopeV1,
        experiment_root_sha256: String,
        freeze_root_sha256: Option<String>,
        attempt_root_sha256: Option<String>,
        outcomes: Vec<K2UncertaintyControlProcessOutcomeV1>,
        evaluator_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CONTROL_REQUEST_SCHEMA_V1.to_owned(),
            scope,
            experiment_root_sha256,
            freeze_root_sha256,
            attempt_root_sha256,
            outcomes,
            evaluator_executable_sha256,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_root_sha256)?;
        require_composition_root_v1(&self.evaluator_executable_sha256)?;
        for root in [&self.freeze_root_sha256, &self.attempt_root_sha256]
            .into_iter()
            .flatten()
        {
            require_composition_root_v1(root)?;
        }
        let binding_shape = match self.scope {
            K2UncertaintyControlScopeV1::SuccessorStaticLegacy
            | K2UncertaintyControlScopeV1::SuccessorStaticV3
            | K2UncertaintyControlScopeV1::SuccessorStaticV4 => {
                self.freeze_root_sha256.is_none() && self.attempt_root_sha256.is_none()
            }
            K2UncertaintyControlScopeV1::DevelopmentRehearsalV5 => {
                self.freeze_root_sha256.is_some() && self.attempt_root_sha256.is_none()
            }
            K2UncertaintyControlScopeV1::SealedAttemptV5 => {
                self.freeze_root_sha256.is_some() && self.attempt_root_sha256.is_some()
            }
        };
        let mut ids = BTreeSet::new();
        for outcome in &self.outcomes {
            outcome.validate()?;
            if outcome.scope != self.scope
                || outcome.experiment_root_sha256 != self.experiment_root_sha256
                || outcome.freeze_root_sha256 != self.freeze_root_sha256
                || outcome.attempt_root_sha256 != self.attempt_root_sha256
                || !ids.insert(outcome.control_id.as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_control_request_outcome_binding_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONTROL_REQUEST_SCHEMA_V1
            || !binding_shape
            || self.outcomes.len() != self.scope.expected_count()
            || self.request_root_sha256 != self.expected_root()?
            || serde_json::to_vec(self)
                .map_err(|_| K2CompositionErrorV1::Invalid("self_formed_control_request_encode"))?
                .len()
                >= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_control_request_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.request_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONTROL_REQUEST_SCHEMA_V1,
            self.scope,
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.attempt_root_sha256,
            &self.outcomes,
            &self.evaluator_executable_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyControlEvaluationReceiptV1 {
    pub schema: String,
    pub scope: K2UncertaintyControlScopeV1,
    pub request_root_sha256: String,
    pub experiment_root_sha256: String,
    pub freeze_root_sha256: Option<String>,
    pub attempt_root_sha256: Option<String>,
    pub outcomes: Vec<K2UncertaintyControlProcessOutcomeV1>,
    pub passed: u64,
    pub expected: u64,
    pub all_pass: bool,
    pub evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyControlEvaluationReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.request_root_sha256,
            &self.experiment_root_sha256,
            &self.evaluator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for outcome in &self.outcomes {
            outcome.validate()?;
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1
            || self.outcomes.len() != self.scope.expected_count()
            || self.expected != self.scope.expected_count() as u64
            || self.passed != self.expected
            || !self.all_pass
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_control_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CONTROL_RECEIPT_SCHEMA_V1,
            self.scope,
            &self.request_root_sha256,
            &self.experiment_root_sha256,
            &self.freeze_root_sha256,
            &self.attempt_root_sha256,
            &self.outcomes,
            self.passed,
            self.expected,
            self.all_pass,
            &self.evaluator_executable_sha256,
            &self.authority,
        ))
    }
}
