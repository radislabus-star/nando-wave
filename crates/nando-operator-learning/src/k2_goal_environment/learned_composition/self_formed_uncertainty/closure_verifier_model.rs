use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1, K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_CLOSURE_VERIFICATION_REQUEST_SCHEMA_V1, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlannerRequestV1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClosureVerificationRequestV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub planner_request: K2UncertaintyClosurePlannerRequestV1,
    pub planner_census: K2UncertaintyClosureCensusV1,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub request_root_sha256: String,
}

impl K2UncertaintyClosureVerificationRequestV1 {
    pub fn seal(
        verifier_executable_sha256: String,
        planner_request: K2UncertaintyClosurePlannerRequestV1,
        planner_census: K2UncertaintyClosureCensusV1,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLOSURE_VERIFICATION_REQUEST_SCHEMA_V1.to_owned(),
            verifier_executable_sha256,
            planner_request,
            planner_census,
            authority: denied_authority_v1(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.verifier_executable_sha256)?;
        self.planner_request.validate()?;
        for root in [
            &self.planner_census.census_root_sha256,
            &self.planner_census.candidate_denominator_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.planner_census.authority)?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLOSURE_VERIFICATION_REQUEST_SCHEMA_V1
            || self.planner_census.schema != K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1
            || self.planner_census.planner_request_root_sha256
                != self.planner_request.request_root_sha256
            || self.planner_census.case_id_sha256 != self.planner_request.case_id_sha256
            || self.planner_census.frontier_root_sha256 != self.planner_request.frontier_root_sha256
            || self.planner_census.first_tournament_root_sha256
                != self.planner_request.first_tournament_root_sha256
            || self.planner_census.first_probe_root_sha256
                != self.planner_request.first_probe_root_sha256
            || self.planner_census.planner_executable_sha256
                != self.planner_request.planner_executable_sha256
            || self.request_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_verification_request_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLOSURE_VERIFICATION_REQUEST_SCHEMA_V1,
            &self.verifier_executable_sha256,
            &self.planner_request.request_root_sha256,
            &self.planner_census.census_root_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyClosureVerificationReceiptV1 {
    pub schema: String,
    pub verifier_executable_sha256: String,
    pub verification_request_root_sha256: String,
    pub case_id_sha256: String,
    pub planner_census_root_sha256: String,
    pub reconstructed_census_root_sha256: String,
    pub candidate_denominator_root_sha256: String,
    pub candidate_count: u64,
    pub joint_pairwise_comparison_count: u64,
    pub disposition: K2UncertaintyClosureDispositionV1,
    pub selected_second_probe_root_sha256: Option<String>,
    pub census_verified: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyClosureVerificationReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.verifier_executable_sha256,
            &self.verification_request_root_sha256,
            &self.case_id_sha256,
            &self.planner_census_root_sha256,
            &self.reconstructed_census_root_sha256,
            &self.candidate_denominator_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if let Some(root) = &self.selected_second_probe_root_sha256 {
            require_composition_root_v1(root)?;
        }
        let comparison_count =
            self.candidate_count
                .checked_mul(6)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_closure_verification_comparison_overflow",
                ))?;
        let selection_shape_valid = match self.disposition {
            K2UncertaintyClosureDispositionV1::SingleProbe
            | K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
                self.selected_second_probe_root_sha256.is_none()
            }
            K2UncertaintyClosureDispositionV1::TwoProbe => {
                self.selected_second_probe_root_sha256.is_some()
            }
        };
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1
            || !self.census_verified
            || self.planner_census_root_sha256 != self.reconstructed_census_root_sha256
            || self.joint_pairwise_comparison_count != comparison_count
            || !selection_shape_valid
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_verification_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1,
            &self.verifier_executable_sha256,
            &self.verification_request_root_sha256,
            &self.case_id_sha256,
            &self.planner_census_root_sha256,
            &self.reconstructed_census_root_sha256,
            &self.candidate_denominator_root_sha256,
            self.candidate_count,
            self.joint_pairwise_comparison_count,
            self.disposition,
            &self.selected_second_probe_root_sha256,
            self.census_verified,
            &self.authority,
        ))
    }
}
