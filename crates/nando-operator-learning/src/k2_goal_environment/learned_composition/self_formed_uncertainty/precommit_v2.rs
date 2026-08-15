use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2, K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2,
    K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V2, K2_UNCERTAINTY_CONFIRM_CASES_V1,
    K2UncertaintyCasePreverificationV1, K2UncertaintyClosureDispositionV1,
    K2UncertaintyClosurePlanV1, K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosureVerificationRequestV1, denied_authority_v1, require_denied_authority_v1,
    require_exact_len_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCasePreverificationV2 {
    pub schema: String,
    pub selection_preverification: K2UncertaintyCasePreverificationV1,
    pub closure_verification_request: K2UncertaintyClosureVerificationRequestV1,
    pub closure_verification_receipt: K2UncertaintyClosureVerificationReceiptV1,
    pub closure_plan: Option<K2UncertaintyClosurePlanV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCasePreverificationV2 {
    pub fn seal(
        selection_preverification: K2UncertaintyCasePreverificationV1,
        closure_verification_request: K2UncertaintyClosureVerificationRequestV1,
        closure_verification_receipt: K2UncertaintyClosureVerificationReceiptV1,
        closure_plan: Option<K2UncertaintyClosurePlanV1>,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V2.to_owned(),
            selection_preverification,
            closure_verification_request,
            closure_verification_receipt,
            closure_plan,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.selection_preverification.validate()?;
        self.closure_verification_request.validate()?;
        self.closure_verification_receipt.validate()?;
        if let Some(plan) = &self.closure_plan {
            plan.validate()?;
        }
        let planner = &self.closure_verification_request.planner_request;
        let census = &self.closure_verification_request.planner_census;
        let receipt = &self.closure_verification_receipt;
        let expected_plan = match census.disposition {
            K2UncertaintyClosureDispositionV1::SingleProbe
            | K2UncertaintyClosureDispositionV1::TwoProbe => true,
            K2UncertaintyClosureDispositionV1::ClosureUnavailable => false,
        };
        let plan_binding_valid = match &self.closure_plan {
            Some(plan) => {
                expected_plan
                    && plan.case_id_sha256 == planner.case_id_sha256
                    && plan.closure_census_root_sha256 == census.census_root_sha256
                    && plan.preverification_receipt_root_sha256 == receipt.receipt_root_sha256
                    && plan.disposition == census.disposition
            }
            None => !expected_plan,
        };
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V2
            || self.selection_preverification.case_id_sha256 != planner.case_id_sha256
            || self
                .selection_preverification
                .tournament
                .frontier_root_sha256
                != planner.frontier_root_sha256
            || self
                .selection_preverification
                .tournament
                .tournament_root_sha256
                != planner.first_tournament_root_sha256
            || self
                .selection_preverification
                .tournament
                .tournament_winner_probe_root_sha256
                != planner.first_probe_root_sha256
            || receipt.verification_request_root_sha256
                != self.closure_verification_request.request_root_sha256
            || receipt.planner_census_root_sha256 != census.census_root_sha256
            || receipt.disposition != census.disposition
            || receipt.selected_second_probe_root_sha256 != census.selected_second_probe_root_sha256
            || !plan_binding_valid
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_preverification_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V2,
            &self.selection_preverification.receipt_root_sha256,
            &self.closure_verification_request.request_root_sha256,
            &self.closure_verification_receipt.receipt_root_sha256,
            self.closure_plan
                .as_ref()
                .map(|value| &value.plan_root_sha256),
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCasePrecommitEntryV2 {
    pub schema: String,
    pub case_id_sha256: String,
    pub case_preverification_root_sha256: String,
    pub selection_preverification_root_sha256: String,
    pub closure_planner_request_root_sha256: String,
    pub closure_census_root_sha256: String,
    pub closure_disposition: K2UncertaintyClosureDispositionV1,
    pub closure_verification_receipt_root_sha256: String,
    pub closure_plan_root_sha256: Option<String>,
    pub dispatchable: bool,
    pub entry_root_sha256: String,
}

impl K2UncertaintyCasePrecommitEntryV2 {
    pub fn seal(case: &K2UncertaintyCasePreverificationV2) -> K2CompositionResultV1<Self> {
        case.validate()?;
        let planner = &case.closure_verification_request.planner_request;
        let census = &case.closure_verification_request.planner_census;
        let mut value = Self {
            schema: K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2.to_owned(),
            case_id_sha256: planner.case_id_sha256.clone(),
            case_preverification_root_sha256: case.receipt_root_sha256.clone(),
            selection_preverification_root_sha256: case
                .selection_preverification
                .receipt_root_sha256
                .clone(),
            closure_planner_request_root_sha256: planner.request_root_sha256.clone(),
            closure_census_root_sha256: census.census_root_sha256.clone(),
            closure_disposition: census.disposition,
            closure_verification_receipt_root_sha256: case
                .closure_verification_receipt
                .receipt_root_sha256
                .clone(),
            closure_plan_root_sha256: case
                .closure_plan
                .as_ref()
                .map(|value| value.plan_root_sha256.clone()),
            dispatchable: case.closure_plan.is_some(),
            entry_root_sha256: String::new(),
        };
        value.entry_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.case_preverification_root_sha256,
            &self.selection_preverification_root_sha256,
            &self.closure_planner_request_root_sha256,
            &self.closure_census_root_sha256,
            &self.closure_verification_receipt_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if let Some(root) = &self.closure_plan_root_sha256 {
            require_composition_root_v1(root)?;
        }
        let shape_valid = match self.closure_disposition {
            K2UncertaintyClosureDispositionV1::SingleProbe
            | K2UncertaintyClosureDispositionV1::TwoProbe => {
                self.dispatchable && self.closure_plan_root_sha256.is_some()
            }
            K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
                !self.dispatchable && self.closure_plan_root_sha256.is_none()
            }
        };
        if self.schema != K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2
            || !shape_valid
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_precommit_entry_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CASE_PRECOMMIT_ENTRY_SCHEMA_V2,
            &self.case_id_sha256,
            &self.case_preverification_root_sha256,
            &self.selection_preverification_root_sha256,
            &self.closure_planner_request_root_sha256,
            &self.closure_census_root_sha256,
            self.closure_disposition,
            &self.closure_verification_receipt_root_sha256,
            &self.closure_plan_root_sha256,
            self.dispatchable,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyBatchPrecommitV2 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub private_expected_denominator_commitment_sha256: String,
    pub cases: Vec<K2UncertaintyCasePrecommitEntryV2>,
    pub execution_order_case_roots_sha256: Vec<String>,
    pub closure_census_denominator_root_sha256: String,
    pub closure_plan_denominator_root_sha256: String,
    pub dispatch_permitted: bool,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub batch_root_sha256: String,
}

impl K2UncertaintyBatchPrecommitV2 {
    pub fn seal(
        experiment_id_sha256: String,
        private_expected_denominator_commitment_sha256: String,
        cases: &[K2UncertaintyCasePreverificationV2],
        execution_order_case_roots_sha256: Vec<String>,
    ) -> K2CompositionResultV1<Self> {
        let mut entries = cases
            .iter()
            .map(K2UncertaintyCasePrecommitEntryV2::seal)
            .collect::<K2CompositionResultV1<Vec<_>>>()?;
        entries.sort_by(|left, right| left.case_id_sha256.cmp(&right.case_id_sha256));
        let census_denominator = closure_census_denominator_v2(&entries)?;
        let plan_denominator = closure_plan_denominator_v2(&entries)?;
        let dispatch_permitted = entries.iter().all(|value| value.dispatchable);
        let mut value = Self {
            schema: K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2.to_owned(),
            experiment_id_sha256,
            private_expected_denominator_commitment_sha256,
            cases: entries,
            execution_order_case_roots_sha256,
            closure_census_denominator_root_sha256: census_denominator,
            closure_plan_denominator_root_sha256: plan_denominator,
            dispatch_permitted,
            authority: denied_authority_v1(),
            batch_root_sha256: String::new(),
        };
        value.batch_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.private_expected_denominator_commitment_sha256,
            &self.closure_census_denominator_root_sha256,
            &self.closure_plan_denominator_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.cases.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_batch_precommit_v2_case_count_invalid",
        )?;
        require_exact_len_v1(
            self.execution_order_case_roots_sha256.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_batch_precommit_v2_order_count_invalid",
        )?;
        let mut case_roots = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if !case_roots.insert(case.case_id_sha256.clone()) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_batch_precommit_v2_duplicate_case",
                ));
            }
        }
        let execution_roots = self
            .execution_order_case_roots_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dispatch_permitted = self.cases.iter().all(|value| value.dispatchable);
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2
            || self
                .cases
                .windows(2)
                .any(|pair| pair[0].case_id_sha256 >= pair[1].case_id_sha256)
            || execution_roots != case_roots
            || self.closure_census_denominator_root_sha256
                != closure_census_denominator_v2(&self.cases)?
            || self.closure_plan_denominator_root_sha256
                != closure_plan_denominator_v2(&self.cases)?
            || self.dispatch_permitted != dispatch_permitted
            || self.batch_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_precommit_v2_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V2,
            &self.experiment_id_sha256,
            &self.private_expected_denominator_commitment_sha256,
            &self.cases,
            &self.execution_order_case_roots_sha256,
            &self.closure_census_denominator_root_sha256,
            &self.closure_plan_denominator_root_sha256,
            self.dispatch_permitted,
            &self.authority,
        ))
    }
}

fn closure_census_denominator_v2(
    cases: &[K2UncertaintyCasePrecommitEntryV2],
) -> K2CompositionResultV1<String> {
    let values = cases
        .iter()
        .map(|value| {
            (
                &value.case_id_sha256,
                &value.closure_census_root_sha256,
                value.closure_disposition,
            )
        })
        .collect::<Vec<_>>();
    uncertainty_root_v1(&("nando.k2-self-formed-closure-census-denominator.v2", values))
}

fn closure_plan_denominator_v2(
    cases: &[K2UncertaintyCasePrecommitEntryV2],
) -> K2CompositionResultV1<String> {
    let values = cases
        .iter()
        .map(|value| (&value.case_id_sha256, &value.closure_plan_root_sha256))
        .collect::<Vec<_>>();
    uncertainty_root_v1(&("nando.k2-self-formed-closure-plan-denominator.v2", values))
}
