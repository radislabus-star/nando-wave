use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionLearnedEffectV1,
    K2CompositionResultV1, K2CompositionTreeManifestV1, composition_root_v1,
    require_composition_root_v1,
};

pub const K2_INQUIRY_WORLD_MODELS_V1: usize = 4;
pub const K2_INQUIRY_PROBES_V1: usize = 8;
pub const K2_INQUIRY_KNOWN_ACTIONS_V1: usize = 7;
pub const K2_INQUIRY_CONFIRM_CASES_V1: usize = 8;
pub const K2_INQUIRY_MAX_RISK_UNITS_V1: u64 = 10;
pub const K2_INQUIRY_MAX_COST_UNITS_V1: u64 = 10;
pub const K2_INQUIRY_MAX_PROTOCOL_BYTES_V1: usize = 1024 * 1024;
pub const K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS_V1: &str = "K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS";

pub const K2_INQUIRY_WORLD_MODEL_SCHEMA_V1: &str = "nando.k2-inquiry-world-model.v1";
pub const K2_INQUIRY_PROBE_SCHEMA_V1: &str = "nando.k2-inquiry-probe.v1";
pub const K2_INQUIRY_PUBLIC_CASE_SCHEMA_V1: &str = "nando.k2-inquiry-public-case.v1";
pub const K2_INQUIRY_SELECTOR_REQUEST_SCHEMA_V1: &str = "nando.k2-inquiry-selector-request.v1";
pub const K2_INQUIRY_BASELINE_REQUEST_SCHEMA_V1: &str = "nando.k2-inquiry-baseline-request.v1";
pub const K2_INQUIRY_PREDICTION_SCHEMA_V1: &str = "nando.k2-inquiry-prediction.v1";
pub const K2_INQUIRY_ELIGIBILITY_SCHEMA_V1: &str = "nando.k2-inquiry-eligibility.v1";
pub const K2_INQUIRY_EVALUATION_SCHEMA_V1: &str = "nando.k2-inquiry-evaluation.v1";
pub const K2_INQUIRY_PRECOMMIT_SCHEMA_V1: &str = "nando.k2-inquiry-precommit.v1";
pub const K2_INQUIRY_BASELINES_SCHEMA_V1: &str = "nando.k2-inquiry-baselines.v1";
pub const K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-inquiry-selection-verification.v1";
pub const K2_INQUIRY_WORKER_REQUEST_SCHEMA_V1: &str = "nando.k2-inquiry-worker-request.v1";
pub const K2_INQUIRY_WORKER_OUTCOME_SCHEMA_V1: &str = "nando.k2-inquiry-worker-outcome.v1";
pub const K2_INQUIRY_OBSERVER_REQUEST_SCHEMA_V1: &str = "nando.k2-inquiry-observer-request.v1";
pub const K2_INQUIRY_OBSERVATION_SCHEMA_V1: &str = "nando.k2-inquiry-observation.v1";
pub const K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1: &str =
    "nando.k2-inquiry-outcome-verification-request.v1";
pub const K2_INQUIRY_OUTCOME_VERIFICATION_SCHEMA_V1: &str =
    "nando.k2-inquiry-outcome-verification.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2InquiryObservationModeV1 {
    ExactImmediate,
    Ambiguous,
    Delayed,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryModelActionV1 {
    pub action_id_sha256: String,
    pub effect: K2CompositionLearnedEffectV1,
}

impl K2InquiryModelActionV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.action_id_sha256)?;
        self.effect.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryWorldModelV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub model_id_sha256: String,
    pub common_evidence_root_sha256: String,
    pub source_neutral_provenance_root_sha256: String,
    pub actions: Vec<K2InquiryModelActionV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub model_root_sha256: String,
}

impl K2InquiryWorldModelV1 {
    pub fn seal(
        experiment_id_sha256: String,
        model_id_sha256: String,
        common_evidence_root_sha256: String,
        source_neutral_provenance_root_sha256: String,
        mut actions: Vec<K2InquiryModelActionV1>,
    ) -> K2CompositionResultV1<Self> {
        actions.sort();
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let model_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_WORLD_MODEL_SCHEMA_V1,
            &experiment_id_sha256,
            &model_id_sha256,
            &common_evidence_root_sha256,
            &source_neutral_provenance_root_sha256,
            &actions,
            &authority,
        ))?;
        let model = Self {
            schema: K2_INQUIRY_WORLD_MODEL_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            model_id_sha256,
            common_evidence_root_sha256,
            source_neutral_provenance_root_sha256,
            actions,
            authority,
            model_root_sha256,
        };
        model.validate()?;
        Ok(model)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.model_id_sha256,
            &self.common_evidence_root_sha256,
            &self.source_neutral_provenance_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.authority.validate()?;
        if self.schema != K2_INQUIRY_WORLD_MODEL_SCHEMA_V1
            || self.actions.len() != K2_INQUIRY_KNOWN_ACTIONS_V1
            || self
                .actions
                .windows(2)
                .any(|pair| pair[0].action_id_sha256 >= pair[1].action_id_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_world_model_shape_invalid",
            ));
        }
        for action in &self.actions {
            action.validate()?;
        }
        let expected = composition_root_v1(&(
            K2_INQUIRY_WORLD_MODEL_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.model_id_sha256,
            &self.common_evidence_root_sha256,
            &self.source_neutral_provenance_root_sha256,
            &self.actions,
            &self.authority,
        ))?;
        if expected != self.model_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_world_model_root_mismatch",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn effect(&self, action_id: &str) -> Option<&K2CompositionLearnedEffectV1> {
        self.actions
            .binary_search_by(|action| action.action_id_sha256.as_str().cmp(action_id))
            .ok()
            .map(|index| &self.actions[index].effect)
    }

    #[must_use]
    pub fn action_ids(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| action.action_id_sha256.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryProbeV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub probe_id_sha256: String,
    pub action_id_sha256: String,
    pub initial_manifest: K2CompositionTreeManifestV1,
    pub reversible: bool,
    pub observation_mode: K2InquiryObservationModeV1,
    pub risk_units: u64,
    pub cost_units: u64,
    pub applicability_hint: bool,
    pub dependency_hint: bool,
    pub cleanup_hint: bool,
    pub generated_provenance_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub probe_root_sha256: String,
}

impl K2InquiryProbeV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        probe_id_sha256: String,
        action_id_sha256: String,
        initial_manifest: K2CompositionTreeManifestV1,
        reversible: bool,
        observation_mode: K2InquiryObservationModeV1,
        risk_units: u64,
        cost_units: u64,
        applicability_hint: bool,
        dependency_hint: bool,
        cleanup_hint: bool,
        generated_provenance_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let probe_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_PROBE_SCHEMA_V1,
            &experiment_id_sha256,
            &probe_id_sha256,
            &action_id_sha256,
            &initial_manifest,
            reversible,
            observation_mode,
            risk_units,
            cost_units,
            applicability_hint,
            dependency_hint,
            cleanup_hint,
            &generated_provenance_root_sha256,
            &authority,
        ))?;
        let probe = Self {
            schema: K2_INQUIRY_PROBE_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            probe_id_sha256,
            action_id_sha256,
            initial_manifest,
            reversible,
            observation_mode,
            risk_units,
            cost_units,
            applicability_hint,
            dependency_hint,
            cleanup_hint,
            generated_provenance_root_sha256,
            authority,
            probe_root_sha256,
        };
        probe.validate()?;
        Ok(probe)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.probe_id_sha256,
            &self.action_id_sha256,
            &self.generated_provenance_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.initial_manifest.validate()?;
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_INQUIRY_PROBE_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.probe_id_sha256,
            &self.action_id_sha256,
            &self.initial_manifest,
            self.reversible,
            self.observation_mode,
            self.risk_units,
            self.cost_units,
            self.applicability_hint,
            self.dependency_hint,
            self.cleanup_hint,
            &self.generated_provenance_root_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_INQUIRY_PROBE_SCHEMA_V1 || expected != self.probe_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid("inquiry_probe_invalid"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryPublicCaseV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub generator_schema_root_sha256: String,
    pub split_commitment_root_sha256: String,
    pub models: Vec<K2InquiryWorldModelV1>,
    pub probes: Vec<K2InquiryProbeV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub case_root_sha256: String,
}

impl K2InquiryPublicCaseV1 {
    pub fn seal(
        experiment_id_sha256: String,
        generator_schema_root_sha256: String,
        split_commitment_root_sha256: String,
        mut models: Vec<K2InquiryWorldModelV1>,
        probes: Vec<K2InquiryProbeV1>,
    ) -> K2CompositionResultV1<Self> {
        models.sort_by(|left, right| left.model_root_sha256.cmp(&right.model_root_sha256));
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let case_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_PUBLIC_CASE_SCHEMA_V1,
            &experiment_id_sha256,
            &generator_schema_root_sha256,
            &split_commitment_root_sha256,
            &models,
            &probes,
            &authority,
        ))?;
        let case = Self {
            schema: K2_INQUIRY_PUBLIC_CASE_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            generator_schema_root_sha256,
            split_commitment_root_sha256,
            models,
            probes,
            authority,
            case_root_sha256,
        };
        case.validate()?;
        Ok(case)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.generator_schema_root_sha256,
            &self.split_commitment_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.authority.validate()?;
        if self.schema != K2_INQUIRY_PUBLIC_CASE_SCHEMA_V1
            || self.models.len() != K2_INQUIRY_WORLD_MODELS_V1
            || self.probes.len() != K2_INQUIRY_PROBES_V1
            || self
                .models
                .windows(2)
                .any(|pair| pair[0].model_root_sha256 >= pair[1].model_root_sha256)
            || self
                .probes
                .iter()
                .map(|probe| &probe.probe_root_sha256)
                .collect::<BTreeSet<_>>()
                .len()
                != K2_INQUIRY_PROBES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_public_case_shape_invalid",
            ));
        }
        let mut expected_action_ids = None;
        let mut common_evidence_root = None;
        for model in &self.models {
            model.validate()?;
            if model.experiment_id_sha256 != self.experiment_id_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "inquiry_model_experiment_mismatch",
                ));
            }
            let action_ids = model.action_ids();
            if expected_action_ids
                .as_ref()
                .is_some_and(|expected| expected != &action_ids)
                || common_evidence_root
                    .as_ref()
                    .is_some_and(|expected| expected != &model.common_evidence_root_sha256)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "inquiry_model_vocabulary_mismatch",
                ));
            }
            expected_action_ids.get_or_insert(action_ids);
            common_evidence_root.get_or_insert(model.common_evidence_root_sha256.clone());
        }
        for probe in &self.probes {
            probe.validate()?;
            if probe.experiment_id_sha256 != self.experiment_id_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "inquiry_probe_experiment_mismatch",
                ));
            }
        }
        let expected = composition_root_v1(&(
            K2_INQUIRY_PUBLIC_CASE_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.generator_schema_root_sha256,
            &self.split_commitment_root_sha256,
            &self.models,
            &self.probes,
            &self.authority,
        ))?;
        if expected != self.case_root_sha256 {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_public_case_root_mismatch",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn probe(&self, probe_root: &str) -> Option<&K2InquiryProbeV1> {
        self.probes
            .iter()
            .find(|probe| probe.probe_root_sha256 == probe_root)
    }

    #[must_use]
    pub fn model(&self, model_root: &str) -> Option<&K2InquiryWorldModelV1> {
        self.models
            .iter()
            .find(|model| model.model_root_sha256 == model_root)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquirySelectorRequestV1 {
    pub schema: String,
    pub selector_executable_sha256: String,
    pub public_case: K2InquiryPublicCaseV1,
    pub sealed_before_outcome: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2InquirySelectorRequestV1 {
    pub fn seal(
        selector_executable_sha256: String,
        public_case: K2InquiryPublicCaseV1,
    ) -> K2CompositionResultV1<Self> {
        let sealed_before_outcome = true;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_SELECTOR_REQUEST_SCHEMA_V1,
            &selector_executable_sha256,
            &public_case,
            sealed_before_outcome,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_SELECTOR_REQUEST_SCHEMA_V1.to_owned(),
            selector_executable_sha256,
            public_case,
            sealed_before_outcome,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.selector_executable_sha256)?;
        self.public_case.validate()?;
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_INQUIRY_SELECTOR_REQUEST_SCHEMA_V1,
            &self.selector_executable_sha256,
            &self.public_case,
            self.sealed_before_outcome,
            &self.authority,
        ))?;
        if self.schema != K2_INQUIRY_SELECTOR_REQUEST_SCHEMA_V1
            || !self.sealed_before_outcome
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_selector_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryBaselineRequestV1 {
    pub schema: String,
    pub baseline_executable_sha256: String,
    pub public_case: K2InquiryPublicCaseV1,
    pub sealed_before_outcome: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2InquiryBaselineRequestV1 {
    pub fn seal(
        baseline_executable_sha256: String,
        public_case: K2InquiryPublicCaseV1,
    ) -> K2CompositionResultV1<Self> {
        let sealed_before_outcome = true;
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_BASELINE_REQUEST_SCHEMA_V1,
            &baseline_executable_sha256,
            &public_case,
            sealed_before_outcome,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_BASELINE_REQUEST_SCHEMA_V1.to_owned(),
            baseline_executable_sha256,
            public_case,
            sealed_before_outcome,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.baseline_executable_sha256)?;
        self.public_case.validate()?;
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_INQUIRY_BASELINE_REQUEST_SCHEMA_V1,
            &self.baseline_executable_sha256,
            &self.public_case,
            self.sealed_before_outcome,
            &self.authority,
        ))?;
        if self.schema != K2_INQUIRY_BASELINE_REQUEST_SCHEMA_V1
            || !self.sealed_before_outcome
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_baseline_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2InquiryEligibilityReasonV1 {
    Eligible,
    NonReversible,
    AmbiguousObservation,
    DelayedObservation,
    UnknownAction,
    RiskBudgetExceeded,
    CostBudgetExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryEligibilityV1 {
    pub schema: String,
    pub eligible: bool,
    pub reason: K2InquiryEligibilityReasonV1,
    pub eligibility_root_sha256: String,
}

impl K2InquiryEligibilityV1 {
    pub fn seal(reason: K2InquiryEligibilityReasonV1) -> K2CompositionResultV1<Self> {
        let eligible = reason == K2InquiryEligibilityReasonV1::Eligible;
        let eligibility_root_sha256 =
            composition_root_v1(&(K2_INQUIRY_ELIGIBILITY_SCHEMA_V1, eligible, reason))?;
        Ok(Self {
            schema: K2_INQUIRY_ELIGIBILITY_SCHEMA_V1.to_owned(),
            eligible,
            reason,
            eligibility_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryPredictionV1 {
    pub schema: String,
    pub model_root_sha256: String,
    pub probe_root_sha256: String,
    pub transition_applied: bool,
    pub transition_reason: String,
    pub predicted_post_manifest: K2CompositionTreeManifestV1,
    pub observable_outcome_root_sha256: String,
    pub prediction_root_sha256: String,
}

impl K2InquiryPredictionV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        model_root_sha256: String,
        probe_root_sha256: String,
        transition_applied: bool,
        transition_reason: String,
        predicted_post_manifest: K2CompositionTreeManifestV1,
        observation_mode: K2InquiryObservationModeV1,
    ) -> K2CompositionResultV1<Self> {
        let observable_outcome_root_sha256 =
            inquiry_observable_outcome_root_v1(observation_mode, &predicted_post_manifest)?;
        let prediction_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_PREDICTION_SCHEMA_V1,
            &model_root_sha256,
            &probe_root_sha256,
            transition_applied,
            &transition_reason,
            &predicted_post_manifest,
            &observable_outcome_root_sha256,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_PREDICTION_SCHEMA_V1.to_owned(),
            model_root_sha256,
            probe_root_sha256,
            transition_applied,
            transition_reason,
            predicted_post_manifest,
            observable_outcome_root_sha256,
            prediction_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryProbeEvaluationV1 {
    pub schema: String,
    pub probe_root_sha256: String,
    pub eligibility: K2InquiryEligibilityV1,
    pub predictions: Vec<K2InquiryPredictionV1>,
    pub partition_sizes: Vec<u64>,
    pub largest_partition: u64,
    pub minimax_eliminated: u64,
    pub pair_separation: u64,
    pub evaluation_root_sha256: String,
}

impl K2InquiryProbeEvaluationV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.evaluation_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_EVALUATION_SCHEMA_V1,
            &self.probe_root_sha256,
            &self.eligibility,
            &self.predictions,
            &self.partition_sizes,
            self.largest_partition,
            self.minimax_eliminated,
            self.pair_separation,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquirySelectionPrecommitV1 {
    pub schema: String,
    pub selector_request_root_sha256: String,
    pub public_case_root_sha256: String,
    pub evaluations: Vec<K2InquiryProbeEvaluationV1>,
    pub selected_probe_root_sha256: String,
    pub exact_best_ties: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub precommit_root_sha256: String,
}

impl K2InquirySelectionPrecommitV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.precommit_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_PRECOMMIT_SCHEMA_V1,
            &self.selector_request_root_sha256,
            &self.public_case_root_sha256,
            &self.evaluations,
            &self.selected_probe_root_sha256,
            self.exact_best_ties,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2InquiryBaselineKindV1 {
    Passive,
    StableHash,
    CheapestFirst,
    ExplicitHeuristic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryBaselineDecisionV1 {
    pub kind: K2InquiryBaselineKindV1,
    pub selected_probe_root_sha256: Option<String>,
    pub decision_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryBaselinesV1 {
    pub schema: String,
    pub baseline_request_root_sha256: String,
    pub public_case_root_sha256: String,
    pub decisions: Vec<K2InquiryBaselineDecisionV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub baselines_root_sha256: String,
}

impl K2InquiryBaselinesV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.baselines_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_BASELINES_SCHEMA_V1,
            &self.baseline_request_root_sha256,
            &self.public_case_root_sha256,
            &self.decisions,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquirySelectionVerificationReceiptV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub public_case_root_sha256: String,
    pub precommit_root_sha256: String,
    pub selected_probe_root_sha256: String,
    pub prediction_count: u64,
    pub selection_verified: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2InquirySelectionVerificationReceiptV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1,
            &self.verifier_executable_sha256,
            &self.public_case_root_sha256,
            &self.precommit_root_sha256,
            &self.selected_probe_root_sha256,
            self.prediction_count,
            self.selection_verified,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryWorkerRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub selection_verification_root_sha256: String,
    pub selected_probe_root_sha256: String,
    pub selected_action_id_sha256: String,
    pub worker_executable_sha256: String,
    pub initial_manifest: K2CompositionTreeManifestV1,
    pub resolved_effect: K2CompositionLearnedEffectV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2InquiryWorkerRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        experiment_id_sha256: String,
        selection_verification_root_sha256: String,
        selected_probe_root_sha256: String,
        selected_action_id_sha256: String,
        worker_executable_sha256: String,
        initial_manifest: K2CompositionTreeManifestV1,
        resolved_effect: K2CompositionLearnedEffectV1,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_WORKER_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &selection_verification_root_sha256,
            &selected_probe_root_sha256,
            &selected_action_id_sha256,
            &worker_executable_sha256,
            &initial_manifest,
            &resolved_effect,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_WORKER_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            selection_verification_root_sha256,
            selected_probe_root_sha256,
            selected_action_id_sha256,
            worker_executable_sha256,
            initial_manifest,
            resolved_effect,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.selection_verification_root_sha256,
            &self.selected_probe_root_sha256,
            &self.selected_action_id_sha256,
            &self.worker_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.initial_manifest.validate()?;
        self.resolved_effect.validate()?;
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_INQUIRY_WORKER_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.selection_verification_root_sha256,
            &self.selected_probe_root_sha256,
            &self.selected_action_id_sha256,
            &self.worker_executable_sha256,
            &self.initial_manifest,
            &self.resolved_effect,
            &self.authority,
        ))?;
        if self.schema != K2_INQUIRY_WORKER_REQUEST_SCHEMA_V1
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_worker_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryWorkerOutcomeV1 {
    pub schema: String,
    pub request_root_sha256: String,
    pub worker_executable_sha256: String,
    pub selected_probe_root_sha256: String,
    pub pre_manifest: K2CompositionTreeManifestV1,
    pub post_manifest: K2CompositionTreeManifestV1,
    pub transition_applied: bool,
    pub transition_reason: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub outcome_root_sha256: String,
}

impl K2InquiryWorkerOutcomeV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.outcome_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_WORKER_OUTCOME_SCHEMA_V1,
            &self.request_root_sha256,
            &self.worker_executable_sha256,
            &self.selected_probe_root_sha256,
            &self.pre_manifest,
            &self.post_manifest,
            self.transition_applied,
            &self.transition_reason,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryObserverRequestV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub selected_probe_root_sha256: String,
    pub observer_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2InquiryObserverRequestV1 {
    pub fn seal(
        experiment_id_sha256: String,
        selected_probe_root_sha256: String,
        observer_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_OBSERVER_REQUEST_SCHEMA_V1,
            &experiment_id_sha256,
            &selected_probe_root_sha256,
            &observer_executable_sha256,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_OBSERVER_REQUEST_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            selected_probe_root_sha256,
            observer_executable_sha256,
            authority,
            request_root_sha256,
        })
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.selected_probe_root_sha256,
            &self.observer_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.authority.validate()?;
        let expected = composition_root_v1(&(
            K2_INQUIRY_OBSERVER_REQUEST_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.selected_probe_root_sha256,
            &self.observer_executable_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_INQUIRY_OBSERVER_REQUEST_SCHEMA_V1
            || expected != self.request_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "inquiry_observer_request_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryObservationReceiptV1 {
    pub schema: String,
    pub observer_request_root_sha256: String,
    pub observer_executable_sha256: String,
    pub selected_probe_root_sha256: String,
    pub post_manifest: K2CompositionTreeManifestV1,
    pub observable_outcome_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2InquiryObservationReceiptV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.observable_outcome_root_sha256 = inquiry_observable_outcome_root_v1(
            K2InquiryObservationModeV1::ExactImmediate,
            &self.post_manifest,
        )?;
        self.receipt_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_OBSERVATION_SCHEMA_V1,
            &self.observer_request_root_sha256,
            &self.observer_executable_sha256,
            &self.selected_probe_root_sha256,
            &self.post_manifest,
            &self.observable_outcome_root_sha256,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryOutcomeVerificationRequestV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub selector_request: K2InquirySelectorRequestV1,
    pub precommit: K2InquirySelectionPrecommitV1,
    pub selection_verification: K2InquirySelectionVerificationReceiptV1,
    pub baseline_request: K2InquiryBaselineRequestV1,
    pub baselines: K2InquiryBaselinesV1,
    pub observation: K2InquiryObservationReceiptV1,
    pub private_true_model_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2InquiryOutcomeVerificationRequestV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        verifier_executable_sha256: String,
        selector_request: K2InquirySelectorRequestV1,
        precommit: K2InquirySelectionPrecommitV1,
        selection_verification: K2InquirySelectionVerificationReceiptV1,
        baseline_request: K2InquiryBaselineRequestV1,
        baselines: K2InquiryBaselinesV1,
        observation: K2InquiryObservationReceiptV1,
        private_true_model_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let authority = K2CompositionAuthorityBoundaryV1::denied();
        let request_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1,
            &verifier_executable_sha256,
            &selector_request,
            &precommit,
            &selection_verification,
            &baseline_request,
            &baselines,
            &observation,
            &private_true_model_root_sha256,
            &authority,
        ))?;
        Ok(Self {
            schema: K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1.to_owned(),
            verifier_executable_sha256,
            selector_request,
            precommit,
            selection_verification,
            baseline_request,
            baselines,
            observation,
            private_true_model_root_sha256,
            authority,
            request_root_sha256,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryBaselineSurvivorsV1 {
    pub kind: K2InquiryBaselineKindV1,
    pub selected_probe_root_sha256: Option<String>,
    pub survivors: u64,
    pub cost_units: u64,
    pub result_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2InquiryOutcomeVerificationReceiptV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub verification_request_root_sha256: String,
    pub public_case_root_sha256: String,
    pub selected_probe_root_sha256: String,
    pub surviving_model_roots_sha256: Vec<String>,
    pub baseline_survivors: Vec<K2InquiryBaselineSurvivorsV1>,
    pub oracle_probe_root_sha256: String,
    pub oracle_survivors: u64,
    pub selector_matches_oracle: bool,
    pub complete_prediction_count: u64,
    pub forbidden_probe_executions: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2InquiryOutcomeVerificationReceiptV1 {
    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = composition_root_v1(&(
            K2_INQUIRY_OUTCOME_VERIFICATION_SCHEMA_V1,
            &self.verifier_executable_sha256,
            &self.verification_request_root_sha256,
            &self.public_case_root_sha256,
            &self.selected_probe_root_sha256,
            &self.surviving_model_roots_sha256,
            &self.baseline_survivors,
            &self.oracle_probe_root_sha256,
            self.oracle_survivors,
            self.selector_matches_oracle,
            self.complete_prediction_count,
            self.forbidden_probe_executions,
            &self.authority,
        ))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2InquiryVerifierCommandV1 {
    VerifySelection {
        verifier_executable_sha256: String,
        selector_request: K2InquirySelectorRequestV1,
        precommit: K2InquirySelectionPrecommitV1,
    },
    VerifyOutcome {
        request: K2InquiryOutcomeVerificationRequestV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "receipt", rename_all = "snake_case", deny_unknown_fields)]
pub enum K2InquiryVerifierReceiptV1 {
    Selection {
        value: K2InquirySelectionVerificationReceiptV1,
    },
    Outcome {
        value: K2InquiryOutcomeVerificationReceiptV1,
    },
}

pub fn inquiry_observable_outcome_root_v1(
    mode: K2InquiryObservationModeV1,
    manifest: &K2CompositionTreeManifestV1,
) -> K2CompositionResultV1<String> {
    match mode {
        K2InquiryObservationModeV1::ExactImmediate => {
            composition_root_v1(&("nando.k2-inquiry-observable-exact-manifest.v1", manifest))
        }
        K2InquiryObservationModeV1::Ambiguous => {
            composition_root_v1(&"nando.k2-inquiry-observable-ambiguous.v1")
        }
        K2InquiryObservationModeV1::Delayed => {
            composition_root_v1(&"nando.k2-inquiry-observable-delayed.v1")
        }
    }
}
