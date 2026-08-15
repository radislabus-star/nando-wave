use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_MODELS_V1,
    K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_PROBES_V1,
    K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlannerRequestV1,
    K2UncertaintyClosureVerificationReceiptV1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClosurePlanV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub first_tournament_root_sha256: String,
    pub first_probe_root_sha256: String,
    pub first_partition_sizes: Vec<u64>,
    pub completion_required: bool,
    pub candidate_denominator_root_sha256: String,
    pub closure_census_root_sha256: String,
    pub disposition: K2UncertaintyClosureDispositionV1,
    pub selected_second_probe_root_sha256: Option<String>,
    pub selected_joint_partition_sizes: Vec<u64>,
    pub plan_length: u64,
    pub ordered_probe_roots_sha256: Vec<String>,
    pub ordered_prediction_roots_sha256: Vec<Vec<String>>,
    pub cumulative_risk_units: u64,
    pub cumulative_cost_units: u64,
    pub planner_executable_sha256: String,
    pub preverification_receipt_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub plan_root_sha256: String,
}

#[derive(Serialize)]
struct K2UncertaintyClosurePlanRootPreimageV1<'a> {
    schema: &'static str,
    case_id_sha256: &'a str,
    frontier_root_sha256: &'a str,
    first_tournament_root_sha256: &'a str,
    first_probe_root_sha256: &'a str,
    first_partition_sizes: &'a [u64],
    completion_required: bool,
    candidate_denominator_root_sha256: &'a str,
    closure_census_root_sha256: &'a str,
    disposition: K2UncertaintyClosureDispositionV1,
    selected_second_probe_root_sha256: &'a Option<String>,
    selected_joint_partition_sizes: &'a [u64],
    plan_length: u64,
    ordered_probe_roots_sha256: &'a [String],
    ordered_prediction_roots_sha256: &'a [Vec<String>],
    cumulative_risk_units: u64,
    cumulative_cost_units: u64,
    planner_executable_sha256: &'a str,
    preverification_receipt_root_sha256: &'a str,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

impl K2UncertaintyClosurePlanV1 {
    pub fn seal(
        planner_request: &K2UncertaintyClosurePlannerRequestV1,
        census: &K2UncertaintyClosureCensusV1,
        preverification: &K2UncertaintyClosureVerificationReceiptV1,
    ) -> K2CompositionResultV1<Self> {
        planner_request.validate()?;
        census.validate()?;
        preverification.validate()?;
        if census.planner_request_root_sha256 != planner_request.request_root_sha256
            || preverification.verification_request_root_sha256.is_empty()
            || preverification.case_id_sha256 != planner_request.case_id_sha256
            || preverification.planner_census_root_sha256 != census.census_root_sha256
            || preverification.reconstructed_census_root_sha256 != census.census_root_sha256
            || preverification.disposition != census.disposition
            || preverification.selected_second_probe_root_sha256
                != census.selected_second_probe_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_plan_input_binding_invalid",
            ));
        }
        let first = planner_request
            .representatives
            .iter()
            .find(|value| value.probe.probe_root_sha256 == planner_request.first_probe_root_sha256)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_closure_plan_first_probe_missing",
            ))?;
        let first_predictions = first
            .predictions
            .iter()
            .map(|value| value.prediction_root_sha256.clone())
            .collect::<Vec<_>>();
        let (ordered_probe_roots_sha256, ordered_prediction_roots_sha256, risk, cost) = match census
            .disposition
        {
            K2UncertaintyClosureDispositionV1::SingleProbe => (
                vec![planner_request.first_probe_root_sha256.clone()],
                vec![first_predictions],
                first.probe.risk_units,
                first.probe.cost_units,
            ),
            K2UncertaintyClosureDispositionV1::TwoProbe => {
                let second_root = census.selected_second_probe_root_sha256.as_ref().ok_or(
                    K2CompositionErrorV1::Invalid("self_formed_closure_plan_second_probe_missing"),
                )?;
                let candidate = census
                    .candidates
                    .iter()
                    .find(|value| &value.second_probe_root_sha256 == second_root)
                    .ok_or(K2CompositionErrorV1::Invalid(
                        "self_formed_closure_plan_candidate_missing",
                    ))?;
                (
                    vec![
                        planner_request.first_probe_root_sha256.clone(),
                        second_root.clone(),
                    ],
                    vec![
                        candidate.first_prediction_roots_sha256.clone(),
                        candidate.second_prediction_roots_sha256.clone(),
                    ],
                    candidate.cumulative_risk_units,
                    candidate.cumulative_cost_units,
                )
            }
            K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_closure_plan_unavailable",
                ));
            }
        };
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1.to_owned(),
            case_id_sha256: planner_request.case_id_sha256.clone(),
            frontier_root_sha256: planner_request.frontier_root_sha256.clone(),
            first_tournament_root_sha256: planner_request.first_tournament_root_sha256.clone(),
            first_probe_root_sha256: planner_request.first_probe_root_sha256.clone(),
            first_partition_sizes: census.first_partition_sizes.clone(),
            completion_required: census.completion_required,
            candidate_denominator_root_sha256: census.candidate_denominator_root_sha256.clone(),
            closure_census_root_sha256: census.census_root_sha256.clone(),
            disposition: census.disposition,
            selected_second_probe_root_sha256: census.selected_second_probe_root_sha256.clone(),
            selected_joint_partition_sizes: census
                .selected_joint_partition_sizes
                .clone()
                .unwrap_or_else(|| census.first_partition_sizes.clone()),
            plan_length: ordered_probe_roots_sha256.len() as u64,
            ordered_probe_roots_sha256,
            ordered_prediction_roots_sha256,
            cumulative_risk_units: risk,
            cumulative_cost_units: cost,
            planner_executable_sha256: planner_request.planner_executable_sha256.clone(),
            preverification_receipt_root_sha256: preverification.receipt_root_sha256.clone(),
            authority: denied_authority_v1(),
            plan_root_sha256: String::new(),
        };
        value.plan_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.first_tournament_root_sha256,
            &self.first_probe_root_sha256,
            &self.candidate_denominator_root_sha256,
            &self.closure_census_root_sha256,
            &self.planner_executable_sha256,
            &self.preverification_receipt_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        for root in &self.ordered_probe_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        for predictions in &self.ordered_prediction_roots_sha256 {
            if predictions.len() != K2_UNCERTAINTY_CONFIRM_MODELS_V1 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_closure_plan_prediction_count_invalid",
                ));
            }
            for root in predictions {
                require_composition_root_v1(root)?;
            }
        }
        let length = usize::try_from(self.plan_length).map_err(|_| {
            K2CompositionErrorV1::Invalid("self_formed_closure_plan_length_invalid")
        })?;
        let shape_valid = match self.disposition {
            K2UncertaintyClosureDispositionV1::SingleProbe => {
                !self.completion_required
                    && length == 1
                    && self.selected_second_probe_root_sha256.is_none()
            }
            K2UncertaintyClosureDispositionV1::TwoProbe => {
                self.completion_required
                    && length == 2
                    && self.selected_second_probe_root_sha256.as_ref()
                        == self.ordered_probe_roots_sha256.get(1)
            }
            K2UncertaintyClosureDispositionV1::ClosureUnavailable => false,
        };
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1
            || length == 0
            || length > K2_UNCERTAINTY_MAX_PLAN_PROBES_V1
            || self.ordered_probe_roots_sha256.len() != length
            || self.ordered_prediction_roots_sha256.len() != length
            || self.ordered_probe_roots_sha256.first() != Some(&self.first_probe_root_sha256)
            || self.ordered_probe_roots_sha256.len() == 2
                && self.ordered_probe_roots_sha256[0] == self.ordered_probe_roots_sha256[1]
            || self.selected_joint_partition_sizes != [1_u64, 1, 1, 1]
            || self.cumulative_risk_units > K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1
            || self.cumulative_cost_units > K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1
            || !shape_valid
            || self.plan_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_plan_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyClosurePlanRootPreimageV1 {
            schema: K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1,
            case_id_sha256: &self.case_id_sha256,
            frontier_root_sha256: &self.frontier_root_sha256,
            first_tournament_root_sha256: &self.first_tournament_root_sha256,
            first_probe_root_sha256: &self.first_probe_root_sha256,
            first_partition_sizes: &self.first_partition_sizes,
            completion_required: self.completion_required,
            candidate_denominator_root_sha256: &self.candidate_denominator_root_sha256,
            closure_census_root_sha256: &self.closure_census_root_sha256,
            disposition: self.disposition,
            selected_second_probe_root_sha256: &self.selected_second_probe_root_sha256,
            selected_joint_partition_sizes: &self.selected_joint_partition_sizes,
            plan_length: self.plan_length,
            ordered_probe_roots_sha256: &self.ordered_probe_roots_sha256,
            ordered_prediction_roots_sha256: &self.ordered_prediction_roots_sha256,
            cumulative_risk_units: self.cumulative_risk_units,
            cumulative_cost_units: self.cumulative_cost_units,
            planner_executable_sha256: &self.planner_executable_sha256,
            preverification_receipt_root_sha256: &self.preverification_receipt_root_sha256,
            authority: &self.authority,
        })
    }
}
