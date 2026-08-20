use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    K2InquiryBaselineKindV1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CONFIRM_CASES_V1, K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_ORACLE_BASELINE_AGGREGATE_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_BATCH_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_EVIDENCE_ENTRY_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_EVIDENCE_MANIFEST_SCHEMA_V1,
    K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_MAX_PLANS_PER_CASE_V1,
    K2_UNCERTAINTY_ORACLE_PLAN_RESULT_SCHEMA_V1, K2_UNCERTAINTY_ORACLE_PUBLIC_BINDINGS_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2UncertaintyPublicPrecommitReceiptV1,
    K2UncertaintyPublicPreparedCaseV1, denied_authority_v1, require_denied_authority_v1,
    require_exact_len_v1, uncertainty_root_v1,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleBaselineCaseDescriptorV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub batch_precommit_root_sha256: String,
    pub all_cases_precommitted_root_sha256: String,
    pub case_id_sha256: String,
    pub case_sequence: u64,
    pub public_case_root_sha256: String,
    pub prepared_case_root_sha256: String,
    pub closure_plan_root_sha256: String,
    pub baseline_summary_root_sha256: String,
    pub observation_vector_root_sha256: String,
    pub final_verifier_receipt_root_sha256: String,
    pub private_truth_artifact_root_sha256: String,
    pub case_evidence_manifest_root_sha256: String,
    pub oracle_evaluator_executable_sha256: String,
}

impl K2UncertaintyOracleBaselineCaseDescriptorV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.batch_precommit_root_sha256,
            &self.all_cases_precommitted_root_sha256,
            &self.case_id_sha256,
            &self.public_case_root_sha256,
            &self.prepared_case_root_sha256,
            &self.closure_plan_root_sha256,
            &self.baseline_summary_root_sha256,
            &self.observation_vector_root_sha256,
            &self.final_verifier_receipt_root_sha256,
            &self.private_truth_artifact_root_sha256,
            &self.case_evidence_manifest_root_sha256,
            &self.oracle_evaluator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_ORACLE_DESCRIPTOR_SCHEMA_V1
            || self.case_sequence >= K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || serde_json::to_vec(self)
                .map_err(|_| K2CompositionErrorV1::Invalid("self_formed_oracle_descriptor_encode"))?
                .len()
                >= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_descriptor_invalid",
            ));
        }
        Ok(())
    }

    pub fn descriptor_root(&self) -> K2CompositionResultV1<String> {
        self.validate()?;
        uncertainty_root_v1(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOraclePublicBindingsV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub public_batch_root_sha256: String,
    pub batch_precommit_root_sha256: String,
    pub all_cases_precommitted_root_sha256: String,
    pub case_sequence: u64,
    pub probe_request: super::K2UncertaintyProbeRequestV1,
    pub probe_artifacts_root_sha256: String,
    pub frontier_root_sha256: String,
    pub selection_preverification_root_sha256: String,
    pub closure_preverification_root_sha256: String,
    pub baseline_summary_root_sha256: String,
    pub prepared_case_root_sha256: String,
    pub bindings_root_sha256: String,
}

impl K2UncertaintyOraclePublicBindingsV1 {
    pub fn seal(
        public_precommit: K2UncertaintyPublicPrecommitReceiptV1,
        prepared_case: K2UncertaintyPublicPreparedCaseV1,
    ) -> K2CompositionResultV1<Self> {
        public_precommit.validate()?;
        prepared_case.validate()?;
        let case_id = &prepared_case
            .probe_request
            .public_case
            .vocabulary
            .case_id_sha256;
        let entry = public_precommit
            .batch_precommit
            .cases
            .iter()
            .find(|entry| &entry.case_id_sha256 == case_id)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_public_case_precommit_missing",
            ))?;
        if entry.case_preverification_root_sha256
            != prepared_case.preverification.receipt_root_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_public_case_precommit_mismatch",
            ));
        }
        let mut value = Self {
            schema: K2_UNCERTAINTY_ORACLE_PUBLIC_BINDINGS_SCHEMA_V1.to_owned(),
            experiment_id_sha256: public_precommit.experiment_id_sha256,
            public_batch_root_sha256: public_precommit.public_batch_root_sha256,
            batch_precommit_root_sha256: public_precommit.batch_precommit.batch_root_sha256,
            all_cases_precommitted_root_sha256: public_precommit.receipt_root_sha256,
            case_sequence: prepared_case.case_sequence,
            probe_request: prepared_case.probe_request,
            probe_artifacts_root_sha256: prepared_case.probe_artifacts.artifacts_root_sha256,
            frontier_root_sha256: prepared_case.probe_artifacts.frontier_root_sha256,
            selection_preverification_root_sha256: prepared_case
                .selection_preverification
                .receipt_root_sha256,
            closure_preverification_root_sha256: prepared_case.preverification.receipt_root_sha256,
            baseline_summary_root_sha256: prepared_case
                .selection_preverification
                .baseline_summary
                .summary_root_sha256,
            prepared_case_root_sha256: prepared_case.prepared_case_root_sha256,
            bindings_root_sha256: String::new(),
        };
        value.bindings_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        self.probe_request.validate()?;
        for root in [
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.batch_precommit_root_sha256,
            &self.all_cases_precommitted_root_sha256,
            &self.probe_artifacts_root_sha256,
            &self.frontier_root_sha256,
            &self.selection_preverification_root_sha256,
            &self.closure_preverification_root_sha256,
            &self.baseline_summary_root_sha256,
            &self.prepared_case_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_ORACLE_PUBLIC_BINDINGS_SCHEMA_V1
            || self.case_sequence >= K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.bindings_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_public_bindings_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_PUBLIC_BINDINGS_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.public_batch_root_sha256,
            &self.batch_precommit_root_sha256,
            &self.all_cases_precommitted_root_sha256,
            self.case_sequence,
            &self.probe_request,
            &self.probe_artifacts_root_sha256,
            &self.frontier_root_sha256,
            &self.selection_preverification_root_sha256,
            &self.closure_preverification_root_sha256,
            &self.baseline_summary_root_sha256,
            &self.prepared_case_root_sha256,
        ))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyOracleEvidenceKindV1 {
    PublicBindings,
    ModelSet,
    FrontierCensus,
    FrontierPage,
    ClosurePlan,
    ClosurePreverification,
    BaselineSummary,
    ObservationVector,
    FinalVerifierReceipt,
    PrivateTruth,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleEvidenceEntryV1 {
    pub schema: String,
    pub kind: K2UncertaintyOracleEvidenceKindV1,
    pub relative_path: String,
    pub byte_len: u64,
    pub mode: u32,
    pub content_sha256: String,
    pub semantic_root_sha256: String,
    pub entry_root_sha256: String,
}

impl K2UncertaintyOracleEvidenceEntryV1 {
    pub fn seal(
        kind: K2UncertaintyOracleEvidenceKindV1,
        relative_path: String,
        byte_len: u64,
        mode: u32,
        content_sha256: String,
        semantic_root_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_ORACLE_EVIDENCE_ENTRY_SCHEMA_V1.to_owned(),
            kind,
            relative_path,
            byte_len,
            mode,
            content_sha256,
            semantic_root_sha256,
            entry_root_sha256: String::new(),
        };
        value.entry_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.content_sha256,
            &self.semantic_root_sha256,
            &self.entry_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let path = std::path::Path::new(&self.relative_path);
        if self.schema != K2_UNCERTAINTY_ORACLE_EVIDENCE_ENTRY_SCHEMA_V1
            || self.relative_path.is_empty()
            || path.is_absolute()
            || path.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || self.byte_len == 0
            || self.byte_len > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
            || self.mode != 0o400
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_evidence_entry_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_EVIDENCE_ENTRY_SCHEMA_V1,
            self.kind,
            &self.relative_path,
            self.byte_len,
            self.mode,
            &self.content_sha256,
            &self.semantic_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleCaseEvidenceManifestV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub entries: Vec<K2UncertaintyOracleEvidenceEntryV1>,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyOracleCaseEvidenceManifestV1 {
    pub fn seal(
        case_id_sha256: String,
        mut entries: Vec<K2UncertaintyOracleEvidenceEntryV1>,
    ) -> K2CompositionResultV1<Self> {
        entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        let mut value = Self {
            schema: K2_UNCERTAINTY_ORACLE_EVIDENCE_MANIFEST_SCHEMA_V1.to_owned(),
            case_id_sha256,
            entries,
            manifest_root_sha256: String::new(),
        };
        value.manifest_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        let page_count =
            K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1);
        let mut paths = BTreeSet::new();
        let mut roots = BTreeSet::new();
        let mut counts = std::collections::BTreeMap::new();
        for entry in &self.entries {
            entry.validate()?;
            if !paths.insert(entry.relative_path.as_str())
                || !roots.insert(entry.entry_root_sha256.as_str())
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_evidence_duplicate",
                ));
            }
            *counts.entry(entry.kind).or_insert(0_usize) += 1;
        }
        let singleton_kinds = [
            K2UncertaintyOracleEvidenceKindV1::PublicBindings,
            K2UncertaintyOracleEvidenceKindV1::ModelSet,
            K2UncertaintyOracleEvidenceKindV1::FrontierCensus,
            K2UncertaintyOracleEvidenceKindV1::ClosurePlan,
            K2UncertaintyOracleEvidenceKindV1::ClosurePreverification,
            K2UncertaintyOracleEvidenceKindV1::BaselineSummary,
            K2UncertaintyOracleEvidenceKindV1::ObservationVector,
            K2UncertaintyOracleEvidenceKindV1::FinalVerifierReceipt,
            K2UncertaintyOracleEvidenceKindV1::PrivateTruth,
        ];
        if self.schema != K2_UNCERTAINTY_ORACLE_EVIDENCE_MANIFEST_SCHEMA_V1
            || !self
                .entries
                .windows(2)
                .all(|pair| pair[0].relative_path < pair[1].relative_path)
            || singleton_kinds
                .iter()
                .any(|kind| counts.get(kind).copied() != Some(1))
            || counts
                .get(&K2UncertaintyOracleEvidenceKindV1::FrontierPage)
                .copied()
                != Some(page_count)
            || self.entries.len() != page_count + singleton_kinds.len()
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_evidence_manifest_invalid",
            ));
        }
        Ok(())
    }

    pub fn entry(
        &self,
        kind: K2UncertaintyOracleEvidenceKindV1,
    ) -> K2CompositionResultV1<&K2UncertaintyOracleEvidenceEntryV1> {
        let mut values = self.entries.iter().filter(|entry| entry.kind == kind);
        let first = values.next().ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_oracle_evidence_kind_missing",
        ))?;
        if values.next().is_some() {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_evidence_kind_duplicate",
            ));
        }
        Ok(first)
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_EVIDENCE_MANIFEST_SCHEMA_V1,
            &self.case_id_sha256,
            &self.entries,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleFrontierReceiptV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub raw_probe_count: u64,
    pub raw_member_count: u64,
    pub duplicate_member_count: u64,
    pub unclassified_member_count: u64,
    pub class_count: u64,
    pub classes_root_sha256: String,
    pub representative_probe_roots_sha256: Vec<String>,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyOracleFrontierReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.case_id_sha256)?;
        require_composition_root_v1(&self.classes_root_sha256)?;
        for root in &self.representative_probe_roots_sha256 {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1
            || self.raw_probe_count != K2_UNCERTAINTY_RAW_PROBES_V1 as u64
            || self.raw_member_count != self.raw_probe_count
            || self.duplicate_member_count != 0
            || self.unclassified_member_count != 0
            || self.class_count == 0
            || self.class_count != self.representative_probe_roots_sha256.len() as u64
            || !self
                .representative_probe_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_frontier_receipt_invalid",
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
            K2_UNCERTAINTY_ORACLE_FRONTIER_RECEIPT_SCHEMA_V1,
            &self.case_id_sha256,
            self.raw_probe_count,
            self.raw_member_count,
            self.duplicate_member_count,
            self.unclassified_member_count,
            self.class_count,
            &self.classes_root_sha256,
            &self.representative_probe_roots_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOraclePlanResultV1 {
    pub schema: String,
    pub ordered_probe_roots_sha256: Vec<String>,
    pub residual_syntax_roots_sha256: Vec<String>,
    pub residual_semantic_class_roots_sha256: Vec<String>,
    pub true_class_retained: bool,
    pub cumulative_risk_units: u64,
    pub cumulative_cost_units: u64,
    pub result_root_sha256: String,
}

impl K2UncertaintyOraclePlanResultV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.ordered_probe_roots_sha256.is_empty() || self.ordered_probe_roots_sha256.len() > 2 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_plan_length_invalid",
            ));
        }
        for root in self
            .ordered_probe_roots_sha256
            .iter()
            .chain(&self.residual_syntax_roots_sha256)
            .chain(&self.residual_semantic_class_roots_sha256)
        {
            require_composition_root_v1(root)?;
        }
        if self.ordered_probe_roots_sha256.len() == 2
            && self.ordered_probe_roots_sha256[0] == self.ordered_probe_roots_sha256[1]
            || self.residual_syntax_roots_sha256.is_empty()
            || self.residual_semantic_class_roots_sha256.is_empty()
            || !self
                .residual_syntax_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || !self
                .residual_semantic_class_roots_sha256
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.result_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_plan_result_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.result_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_PLAN_RESULT_SCHEMA_V1,
            &self.ordered_probe_roots_sha256,
            &self.residual_syntax_roots_sha256,
            &self.residual_semantic_class_roots_sha256,
            self.true_class_retained,
            self.cumulative_risk_units,
            self.cumulative_cost_units,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleEnumerationCensusV1 {
    pub schema: String,
    pub representative_count: u64,
    pub expected_plan_count: u64,
    pub enumerated: u64,
    pub eligible: u64,
    pub rejected_probe_ineligible: u64,
    pub rejected_risk_budget: u64,
    pub rejected_cost_budget: u64,
    pub enumeration_chain_root_sha256: String,
    pub census_root_sha256: String,
}

impl K2UncertaintyOracleEnumerationCensusV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.enumeration_chain_root_sha256)?;
        let expected = self
            .representative_count
            .checked_mul(self.representative_count)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_plan_count_overflow",
            ))?;
        let accounted = self
            .eligible
            .checked_add(self.rejected_probe_ineligible)
            .and_then(|value| value.checked_add(self.rejected_risk_budget))
            .and_then(|value| value.checked_add(self.rejected_cost_budget))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_plan_accounting_overflow",
            ))?;
        if self.schema != K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1
            || expected > K2_UNCERTAINTY_ORACLE_MAX_PLANS_PER_CASE_V1
            || self.expected_plan_count != expected
            || self.enumerated != expected
            || accounted != self.enumerated
            || self.eligible == 0
            || self.census_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_enumeration_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.census_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_ENUMERATION_SCHEMA_V1,
            self.representative_count,
            self.expected_plan_count,
            self.enumerated,
            self.eligible,
            self.rejected_probe_ineligible,
            self.rejected_risk_budget,
            self.rejected_cost_budget,
            &self.enumeration_chain_root_sha256,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleBaselineResultV1 {
    pub schema: String,
    pub kind: K2InquiryBaselineKindV1,
    pub selected_probe_root_sha256: Option<String>,
    pub residual_semantic_classes: u64,
    pub true_class_retained: bool,
    pub risk_units: u64,
    pub cost_units: u64,
    pub result_root_sha256: String,
}

impl K2UncertaintyOracleBaselineResultV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if let Some(root) = &self.selected_probe_root_sha256 {
            require_composition_root_v1(root)?;
        }
        if self.schema != K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1
            || (self.kind == K2InquiryBaselineKindV1::Passive)
                != self.selected_probe_root_sha256.is_none()
            || self.residual_semantic_classes == 0
            || self.result_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_baseline_result_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.result_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_BASELINE_RESULT_SCHEMA_V1,
            self.kind,
            &self.selected_probe_root_sha256,
            self.residual_semantic_classes,
            self.true_class_retained,
            self.risk_units,
            self.cost_units,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleBaselineCaseReceiptV1 {
    pub schema: String,
    pub case_sequence: u64,
    pub case_id_sha256: String,
    pub descriptor_root_sha256: String,
    pub evidence_manifest_root_sha256: String,
    pub reconstructed_frontier: K2UncertaintyOracleFrontierReceiptV1,
    pub exact_plan_denominator: u64,
    pub enumeration: K2UncertaintyOracleEnumerationCensusV1,
    pub true_syntax_root_sha256: String,
    pub true_semantic_class_root_sha256: String,
    pub model_guided: K2UncertaintyOraclePlanResultV1,
    pub model_guided_observation_parity: bool,
    pub oracle: K2UncertaintyOraclePlanResultV1,
    pub oracle_equality: bool,
    pub baselines: Vec<K2UncertaintyOracleBaselineResultV1>,
    pub final_verifier_receipt_root_sha256: String,
    pub false_accepts: u64,
    pub evaluator_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyOracleBaselineCaseReceiptV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.descriptor_root_sha256,
            &self.evidence_manifest_root_sha256,
            &self.true_syntax_root_sha256,
            &self.true_semantic_class_root_sha256,
            &self.final_verifier_receipt_root_sha256,
            &self.evaluator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        self.reconstructed_frontier.validate()?;
        self.enumeration.validate()?;
        self.model_guided.validate()?;
        self.oracle.validate()?;
        require_exact_len_v1(
            self.baselines.len(),
            4,
            "self_formed_oracle_case_baseline_count_invalid",
        )?;
        let expected_kinds = [
            K2InquiryBaselineKindV1::Passive,
            K2InquiryBaselineKindV1::StableHash,
            K2InquiryBaselineKindV1::CheapestFirst,
            K2InquiryBaselineKindV1::ExplicitHeuristic,
        ];
        for (baseline, expected) in self.baselines.iter().zip(expected_kinds) {
            baseline.validate()?;
            if baseline.kind != expected {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_case_baseline_order_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        let equality = self.model_guided.residual_semantic_class_roots_sha256.len() == 1
            && self.model_guided.true_class_retained
            && self.oracle.residual_semantic_class_roots_sha256.len() == 1
            && self.oracle.true_class_retained;
        if self.schema != K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1
            || self.case_sequence >= K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.case_id_sha256 != self.reconstructed_frontier.case_id_sha256
            || self.exact_plan_denominator != self.enumeration.expected_plan_count
            || !self.model_guided_observation_parity
            || self.oracle_equality != equality
            || self.false_accepts != 0
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_case_receipt_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.receipt_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&K2UncertaintyOracleCaseReceiptRootV1 {
            schema: K2_UNCERTAINTY_ORACLE_CASE_RECEIPT_SCHEMA_V1,
            case_sequence: self.case_sequence,
            case_id_sha256: &self.case_id_sha256,
            descriptor_root_sha256: &self.descriptor_root_sha256,
            evidence_manifest_root_sha256: &self.evidence_manifest_root_sha256,
            reconstructed_frontier_root_sha256: &self.reconstructed_frontier.receipt_root_sha256,
            exact_plan_denominator: self.exact_plan_denominator,
            enumeration_root_sha256: &self.enumeration.census_root_sha256,
            true_syntax_root_sha256: &self.true_syntax_root_sha256,
            true_semantic_class_root_sha256: &self.true_semantic_class_root_sha256,
            model_guided_root_sha256: &self.model_guided.result_root_sha256,
            model_guided_observation_parity: self.model_guided_observation_parity,
            oracle_root_sha256: &self.oracle.result_root_sha256,
            oracle_equality: self.oracle_equality,
            baselines: &self.baselines,
            final_verifier_receipt_root_sha256: &self.final_verifier_receipt_root_sha256,
            false_accepts: self.false_accepts,
            evaluator_executable_sha256: &self.evaluator_executable_sha256,
            authority: &self.authority,
        })
    }
}

#[derive(Serialize)]
struct K2UncertaintyOracleCaseReceiptRootV1<'a> {
    schema: &'static str,
    case_sequence: u64,
    case_id_sha256: &'a str,
    descriptor_root_sha256: &'a str,
    evidence_manifest_root_sha256: &'a str,
    reconstructed_frontier_root_sha256: &'a str,
    exact_plan_denominator: u64,
    enumeration_root_sha256: &'a str,
    true_syntax_root_sha256: &'a str,
    true_semantic_class_root_sha256: &'a str,
    model_guided_root_sha256: &'a str,
    model_guided_observation_parity: bool,
    oracle_root_sha256: &'a str,
    oracle_equality: bool,
    baselines: &'a [K2UncertaintyOracleBaselineResultV1],
    final_verifier_receipt_root_sha256: &'a str,
    false_accepts: u64,
    evaluator_executable_sha256: &'a str,
    authority: &'a K2CompositionAuthorityBoundaryV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleBaselineAggregateV1 {
    pub schema: String,
    pub kind: K2InquiryBaselineKindV1,
    pub model_sum: u64,
    pub policy_sum: u64,
    pub strict_model_improvement_cases: u64,
    pub aggregate_superiority: bool,
    pub threshold_pass: bool,
    pub aggregate_root_sha256: String,
}

impl K2UncertaintyOracleBaselineAggregateV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        if self.schema != K2_UNCERTAINTY_ORACLE_BASELINE_AGGREGATE_SCHEMA_V1
            || self.aggregate_superiority != (self.model_sum < self.policy_sum)
            || self.threshold_pass != (self.strict_model_improvement_cases >= 12)
            || self.strict_model_improvement_cases > K2_UNCERTAINTY_CONFIRM_CASES_V1 as u64
            || self.aggregate_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_baseline_aggregate_invalid",
            ));
        }
        Ok(())
    }

    pub(crate) fn reseal(&mut self) -> K2CompositionResultV1<()> {
        self.aggregate_root_sha256 = self.expected_root()?;
        self.validate()
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_BASELINE_AGGREGATE_SCHEMA_V1,
            self.kind,
            self.model_sum,
            self.policy_sum,
            self.strict_model_improvement_cases,
            self.aggregate_superiority,
            self.threshold_pass,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyOracleBaselineBatchReceiptV1 {
    pub schema: String,
    pub experiment_id_sha256: String,
    pub case_receipts: Vec<K2UncertaintyOracleBaselineCaseReceiptV1>,
    pub aggregates: Vec<K2UncertaintyOracleBaselineAggregateV1>,
    pub oracle_equal_cases: u64,
    pub true_class_retained_cases: u64,
    pub false_accepts: u64,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub receipt_root_sha256: String,
}

impl K2UncertaintyOracleBaselineBatchReceiptV1 {
    pub fn seal(
        experiment_id_sha256: String,
        case_receipts: Vec<K2UncertaintyOracleBaselineCaseReceiptV1>,
    ) -> K2CompositionResultV1<Self> {
        require_exact_len_v1(
            case_receipts.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_oracle_batch_case_count_invalid",
        )?;
        let kinds = [
            K2InquiryBaselineKindV1::Passive,
            K2InquiryBaselineKindV1::StableHash,
            K2InquiryBaselineKindV1::CheapestFirst,
            K2InquiryBaselineKindV1::ExplicitHeuristic,
        ];
        let mut aggregates = Vec::new();
        for (index, kind) in kinds.into_iter().enumerate() {
            let model_sum = case_receipts
                .iter()
                .try_fold(0_u64, |total, case| {
                    total.checked_add(
                        case.model_guided.residual_semantic_class_roots_sha256.len() as u64
                    )
                })
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_sum_overflow",
                ))?;
            let policy_sum = case_receipts
                .iter()
                .try_fold(0_u64, |total, case| {
                    total.checked_add(case.baselines[index].residual_semantic_classes)
                })
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_sum_overflow",
                ))?;
            let strict = case_receipts
                .iter()
                .filter(|case| {
                    (case.model_guided.residual_semantic_class_roots_sha256.len() as u64)
                        < case.baselines[index].residual_semantic_classes
                })
                .count() as u64;
            let mut aggregate = K2UncertaintyOracleBaselineAggregateV1 {
                schema: K2_UNCERTAINTY_ORACLE_BASELINE_AGGREGATE_SCHEMA_V1.to_owned(),
                kind,
                model_sum,
                policy_sum,
                strict_model_improvement_cases: strict,
                aggregate_superiority: model_sum < policy_sum,
                threshold_pass: strict >= 12,
                aggregate_root_sha256: String::new(),
            };
            aggregate.reseal()?;
            aggregates.push(aggregate);
        }
        let oracle_equal_cases = case_receipts
            .iter()
            .filter(|case| case.oracle_equality)
            .count() as u64;
        let true_class_retained_cases = case_receipts
            .iter()
            .filter(|case| case.model_guided.true_class_retained && case.oracle.true_class_retained)
            .count() as u64;
        let false_accepts = case_receipts
            .iter()
            .try_fold(0_u64, |total, case| total.checked_add(case.false_accepts))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_batch_false_accept_overflow",
            ))?;
        let mut value = Self {
            schema: K2_UNCERTAINTY_ORACLE_BATCH_RECEIPT_SCHEMA_V1.to_owned(),
            experiment_id_sha256,
            case_receipts,
            aggregates,
            oracle_equal_cases,
            true_class_retained_cases,
            false_accepts,
            authority: denied_authority_v1(),
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        require_composition_root_v1(&self.experiment_id_sha256)?;
        require_exact_len_v1(
            self.case_receipts.len(),
            K2_UNCERTAINTY_CONFIRM_CASES_V1,
            "self_formed_oracle_batch_case_count_invalid",
        )?;
        require_exact_len_v1(
            self.aggregates.len(),
            4,
            "self_formed_oracle_batch_aggregate_count_invalid",
        )?;
        let mut ids = BTreeSet::new();
        for (sequence, case) in self.case_receipts.iter().enumerate() {
            case.validate()?;
            if case.case_sequence != sequence as u64 || !ids.insert(&case.case_id_sha256) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_case_order_invalid",
                ));
            }
        }
        for aggregate in &self.aggregates {
            aggregate.validate()?;
        }
        let expected_kinds = [
            K2InquiryBaselineKindV1::Passive,
            K2InquiryBaselineKindV1::StableHash,
            K2InquiryBaselineKindV1::CheapestFirst,
            K2InquiryBaselineKindV1::ExplicitHeuristic,
        ];
        for (index, (aggregate, expected_kind)) in
            self.aggregates.iter().zip(expected_kinds).enumerate()
        {
            let model_sum = self
                .case_receipts
                .iter()
                .try_fold(0_u64, |total, case| {
                    total.checked_add(
                        case.model_guided.residual_semantic_class_roots_sha256.len() as u64
                    )
                })
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_sum_overflow",
                ))?;
            let policy_sum = self
                .case_receipts
                .iter()
                .try_fold(0_u64, |total, case| {
                    total.checked_add(case.baselines[index].residual_semantic_classes)
                })
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_sum_overflow",
                ))?;
            let strict = self
                .case_receipts
                .iter()
                .filter(|case| {
                    (case.model_guided.residual_semantic_class_roots_sha256.len() as u64)
                        < case.baselines[index].residual_semantic_classes
                })
                .count() as u64;
            if aggregate.kind != expected_kind
                || aggregate.model_sum != model_sum
                || aggregate.policy_sum != policy_sum
                || aggregate.strict_model_improvement_cases != strict
                || aggregate.aggregate_superiority != (model_sum < policy_sum)
                || aggregate.threshold_pass != (strict >= 12)
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_oracle_batch_aggregate_mismatch",
                ));
            }
        }
        let oracle_equal_cases = self
            .case_receipts
            .iter()
            .filter(|case| case.oracle_equality)
            .count() as u64;
        let true_class_retained_cases = self
            .case_receipts
            .iter()
            .filter(|case| case.model_guided.true_class_retained && case.oracle.true_class_retained)
            .count() as u64;
        let false_accepts = self
            .case_receipts
            .iter()
            .try_fold(0_u64, |total, case| total.checked_add(case.false_accepts))
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_batch_false_accept_overflow",
            ))?;
        require_denied_authority_v1(&self.authority)?;
        if self.schema != K2_UNCERTAINTY_ORACLE_BATCH_RECEIPT_SCHEMA_V1
            || self.oracle_equal_cases != oracle_equal_cases
            || self.true_class_retained_cases != true_class_retained_cases
            || self.false_accepts != false_accepts
            || self.receipt_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_oracle_batch_receipt_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_ORACLE_BATCH_RECEIPT_SCHEMA_V1,
            &self.experiment_id_sha256,
            &self.case_receipts,
            &self.aggregates,
            self.oracle_equal_cases,
            self.true_class_retained_cases,
            self.false_accepts,
            &self.authority,
        ))
    }
}
