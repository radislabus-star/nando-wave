use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1, K2CompositionAuthorityBoundaryV1,
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryBaselineDecisionV1,
    K2InquiryBaselineKindV1, K2InquirySelectionVerificationReceiptV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_BASELINE_SOURCE_SHA256_V1, K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1,
    K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V1, K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1,
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PREDICTIONS_V1, K2_UNCERTAINTY_RAW_PROBES_V1,
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2UncertaintyTournamentV1, denied_authority_v1,
    require_denied_authority_v1, require_exact_len_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyBaselineSummaryV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub frontier_root_sha256: String,
    pub baseline_source_sha256: String,
    pub baseline_executable_sha256: String,
    pub decisions: Vec<K2InquiryBaselineDecisionV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub summary_root_sha256: String,
}

impl K2UncertaintyBaselineSummaryV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.baseline_source_sha256,
            &self.baseline_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_exact_len_v1(
            self.decisions.len(),
            4,
            "self_formed_baseline_summary_count_invalid",
        )?;
        let expected_kinds = [
            K2InquiryBaselineKindV1::Passive,
            K2InquiryBaselineKindV1::StableHash,
            K2InquiryBaselineKindV1::CheapestFirst,
            K2InquiryBaselineKindV1::ExplicitHeuristic,
        ];
        for (decision, expected_kind) in self.decisions.iter().zip(expected_kinds) {
            if decision.kind != expected_kind
                || (decision.kind == K2InquiryBaselineKindV1::Passive)
                    != decision.selected_probe_root_sha256.is_none()
                || decision.decision_root_sha256
                    != uncertainty_root_v1(&(
                        "nando.k2-inquiry-baseline-decision.v1",
                        decision.kind,
                        &decision.selected_probe_root_sha256,
                    ))?
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_baseline_summary_decision_invalid",
                ));
            }
            if let Some(root) = &decision.selected_probe_root_sha256 {
                require_composition_root_v1(root)?;
            }
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.baseline_source_sha256,
            &self.baseline_executable_sha256,
            &self.decisions,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1
            || self.baseline_source_sha256 != K2_UNCERTAINTY_BASELINE_SOURCE_SHA256_V1
            || self.summary_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_baseline_summary_invalid",
            ));
        }
        Ok(())
    }

    pub fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.decisions.sort_by_key(|decision| decision.kind);
        self.summary_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1,
            &self.case_id_sha256,
            &self.frontier_root_sha256,
            &self.baseline_source_sha256,
            &self.baseline_executable_sha256,
            &self.decisions,
            &self.authority,
        ))?;
        self.validate()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCasePreverificationV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub probe_artifacts_root_sha256: String,
    pub tournament: K2UncertaintyTournamentV1,
    pub selection_verifier_executable_sha256: String,
    pub step_verifications: Vec<K2InquirySelectionVerificationReceiptV1>,
    pub baseline_summary: K2UncertaintyBaselineSummaryV1,
    pub raw_probe_count: u64,
    pub raw_prediction_count: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyCasePreverificationV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.probe_artifacts_root_sha256,
            &self.selection_verifier_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.tournament.validate()?;
        self.baseline_summary.validate()?;
        if self.step_verifications.len() as u64 != self.tournament.request_count {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_preverification_step_count_invalid",
            ));
        }
        let mut receipt_roots = BTreeSet::new();
        for receipt in &self.step_verifications {
            require_denied_authority_v1(&receipt.authority)?;
            for root in [
                &receipt.verifier_executable_sha256,
                &receipt.public_case_root_sha256,
                &receipt.precommit_root_sha256,
                &receipt.selected_probe_root_sha256,
                &receipt.receipt_root_sha256,
            ] {
                require_composition_root_v1(root)?;
            }
            if receipt.verifier_executable_sha256 != self.selection_verifier_executable_sha256
                || !receipt.selection_verified
                || receipt.prediction_count != (K2_UNCERTAINTY_SELECTOR_PROBES_V1 * 4) as u64
                || receipt.receipt_root_sha256
                    != uncertainty_root_v1(&(
                        K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1,
                        &receipt.verifier_executable_sha256,
                        &receipt.public_case_root_sha256,
                        &receipt.precommit_root_sha256,
                        &receipt.selected_probe_root_sha256,
                        receipt.prediction_count,
                        receipt.selection_verified,
                        &receipt.authority,
                    ))?
                || !receipt_roots.insert(&receipt.receipt_root_sha256)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_preverification_step_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = self.expected_root()?;
        if self.schema != K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1
            || self.case_id_sha256 != self.tournament.case_id_sha256
            || self.case_id_sha256 != self.baseline_summary.case_id_sha256
            || self.tournament.frontier_root_sha256 != self.baseline_summary.frontier_root_sha256
            || self.raw_probe_count != K2_UNCERTAINTY_RAW_PROBES_V1 as u64
            || self.raw_prediction_count != K2_UNCERTAINTY_RAW_PREDICTIONS_V1 as u64
            || self.receipt_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_case_preverification_invalid",
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
            K2_UNCERTAINTY_CASE_PREVERIFICATION_SCHEMA_V1,
            &self.case_id_sha256,
            &self.probe_artifacts_root_sha256,
            &self.tournament,
            &self.selection_verifier_executable_sha256,
            &self.step_verifications,
            &self.baseline_summary,
            self.raw_probe_count,
            self.raw_prediction_count,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyBatchPrecommitV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub private_expected_denominator_commitment_sha256: String,
    pub cases: Vec<K2UncertaintyCasePreverificationV1>,
    pub execution_order_case_roots_sha256: Vec<String>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub batch_root_sha256: String,
}

impl K2UncertaintyBatchPrecommitV1 {
    pub fn seal(
        experiment_id_sha256: String,
        private_expected_denominator_commitment_sha256: String,
        mut cases: Vec<K2UncertaintyCasePreverificationV1>,
        execution_order_case_roots_sha256: Vec<String>,
    ) -> K2CompositionResultV1<Self> {
        cases.sort_by(|left, right| left.case_id_sha256.cmp(&right.case_id_sha256));
        let mut value = Self {
            schema: K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            private_expected_denominator_commitment_sha256,
            cases,
            execution_order_case_roots_sha256,
            authority: denied_authority_v1(),
            batch_root_sha256: String::new(),
        };
        value.batch_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_composition_root_v1(&self.private_expected_denominator_commitment_sha256)?;
        require_exact_len_v1(
            self.cases.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_batch_precommit_case_count_invalid",
        )?;
        require_exact_len_v1(
            self.execution_order_case_roots_sha256.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_batch_precommit_order_count_invalid",
        )?;
        let mut cases = BTreeSet::new();
        for case in &self.cases {
            case.validate()?;
            if !cases.insert(case.case_id_sha256.clone()) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_batch_precommit_duplicate_case",
                ));
            }
        }
        if self
            .cases
            .windows(2)
            .any(|pair| pair[0].case_id_sha256 >= pair[1].case_id_sha256)
            || self
                .execution_order_case_roots_sha256
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                != cases
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_precommit_order_invalid",
            ));
        }
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V1
            || self.batch_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_batch_precommit_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_BATCH_PRECOMMIT_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.private_expected_denominator_commitment_sha256,
            &self.cases,
            &self.execution_order_case_roots_sha256,
            &self.authority,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyDispatchReceiptV1 {
    pub schema: String,
    pub batch_precommit_root_sha256: String,
    pub case_preverification_root_sha256: String,
    pub safety_receipt_root_sha256: String,
    pub selected_probe_root_sha256: String,
    pub selected_action_root_sha256: String,
    pub resolved_effect_root_sha256: String,
    pub worker_request_root_sha256: String,
    pub observer_request_root_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyDispatchReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.safety_receipt_root_sha256,
            &self.selected_probe_root_sha256,
            &self.selected_action_root_sha256,
            &self.resolved_effect_root_sha256,
            &self.worker_request_root_sha256,
            &self.observer_request_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1,
            &self.batch_precommit_root_sha256,
            &self.case_preverification_root_sha256,
            &self.safety_receipt_root_sha256,
            &self.selected_probe_root_sha256,
            &self.selected_action_root_sha256,
            &self.resolved_effect_root_sha256,
            &self.worker_request_root_sha256,
            &self.observer_request_root_sha256,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_DISPATCH_RECEIPT_SCHEMA_V1
            || self.receipt_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_dispatch_receipt_invalid",
            ));
        }
        Ok(())
    }
}
