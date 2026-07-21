use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::canonical::{BindingProofCanonicalError, is_sha256, pretty_json_bytes, sha256_json};
use super::physical_trial_v2::PhysicalTrialOutcomeV2;
use super::trusted_resolver_v2::{
    BindingEvidencePartitionV2, BindingTrialEvidenceLabelV2, TrustedBindingResolverReceiptSourceV2,
    TrustedResolvedBindingRowV2, TrustedResolvedBindingRowsV2,
};

pub const BINDING_ADJUDICATION_REPORT_SCHEMA_V2: &str =
    "nando.binding-law-evidence-adjudication.v2";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptedBindingEvidenceScopeV2 {
    ControlledFixture,
    ExternalIndependent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingAdjudicationReportV2 {
    pub schema: String,
    pub report_sha256: String,
    pub trusted_resolved_root_sha256: String,
    pub relation_identity_sha256: String,
    pub support_rows: usize,
    pub future_rows: usize,
    pub positive_rows: usize,
    pub applicability_negative_rows: usize,
    pub censored_rows: usize,
    pub wrong_bindings: usize,
    pub negative_accepts: usize,
    pub verify_failed: usize,
    pub unique_surviving_relation: bool,
    pub real_independent_receipts: usize,
    pub production_admissible: bool,
    pub execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedBindingLawEvidenceV2 {
    report: BindingAdjudicationReportV2,
    evidence_scope: AcceptedBindingEvidenceScopeV2,
    rows: Vec<TrustedResolvedBindingRowV2>,
    capability_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingAdjudicationOutcomeV2 {
    Accepted(AcceptedBindingLawEvidenceV2),
    Insufficient(BindingAdjudicationReportV2),
    Rejected(BindingAdjudicationReportV2),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingLawEvidenceV2Error {
    InvalidDigest,
    InvalidResolvedRows,
    Serialization,
}

impl From<BindingProofCanonicalError> for BindingLawEvidenceV2Error {
    fn from(value: BindingProofCanonicalError) -> Self {
        match value {
            BindingProofCanonicalError::Serialization => Self::Serialization,
        }
    }
}

impl BindingAdjudicationReportV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingLawEvidenceV2Error> {
        pretty_json_bytes(self).map_err(BindingLawEvidenceV2Error::from)
    }
}

impl AcceptedBindingLawEvidenceV2 {
    pub fn capability_root_sha256(&self) -> &str {
        &self.capability_root_sha256
    }

    pub fn relation_identity_sha256(&self) -> &str {
        &self.report.relation_identity_sha256
    }

    pub fn trusted_resolved_root_sha256(&self) -> &str {
        &self.report.trusted_resolved_root_sha256
    }

    pub fn evidence_scope(&self) -> AcceptedBindingEvidenceScopeV2 {
        self.evidence_scope
    }

    pub fn production_admissible(&self) -> bool {
        self.report.production_admissible
    }

    pub fn execution_authority(&self) -> bool {
        self.report.execution_authority
    }

    pub fn rows(&self) -> &[TrustedResolvedBindingRowV2] {
        &self.rows
    }

    pub fn report(&self) -> &BindingAdjudicationReportV2 {
        &self.report
    }
}

pub fn adjudicate_binding_law_evidence_v2(
    resolved: &TrustedResolvedBindingRowsV2,
    relation_identity_sha256: &str,
) -> Result<BindingAdjudicationOutcomeV2, BindingLawEvidenceV2Error> {
    if !is_sha256(relation_identity_sha256) || resolved.execution_authority() {
        return Err(BindingLawEvidenceV2Error::InvalidDigest);
    }
    let rows = resolved.rows();
    if rows.is_empty() {
        return Err(BindingLawEvidenceV2Error::InvalidResolvedRows);
    }
    let non_censored = rows
        .iter()
        .filter(|row| row.trial_outcome != PhysicalTrialOutcomeV2::Censored)
        .collect::<Vec<_>>();
    let unique_relations = non_censored
        .iter()
        .map(|row| row.relation_identity_sha256.as_str())
        .collect::<BTreeSet<_>>();
    let positive_rows = non_censored
        .iter()
        .filter(|row| row.evidence_label == BindingTrialEvidenceLabelV2::Positive)
        .count();
    let applicability_negative_rows = non_censored
        .iter()
        .filter(|row| row.evidence_label == BindingTrialEvidenceLabelV2::ApplicabilityNegative)
        .count();
    let wrong_bindings = non_censored
        .iter()
        .filter(|row| {
            row.evidence_label == BindingTrialEvidenceLabelV2::Positive
                && row.trial_outcome != PhysicalTrialOutcomeV2::Pass
        })
        .count();
    let negative_accepts = non_censored
        .iter()
        .filter(|row| {
            row.evidence_label == BindingTrialEvidenceLabelV2::ApplicabilityNegative
                && row.trial_outcome == PhysicalTrialOutcomeV2::Pass
        })
        .count();
    let verify_failed = non_censored
        .iter()
        .filter(|row| row.trial_outcome == PhysicalTrialOutcomeV2::Fail)
        .count();
    let support_non_censored = non_censored
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Support)
        .count();
    let future_non_censored = non_censored
        .iter()
        .filter(|row| row.partition == BindingEvidencePartitionV2::Future)
        .count();
    let all_external = non_censored.iter().all(|row| {
        row.receipt_source == TrustedBindingResolverReceiptSourceV2::ExternalIndependent
    });
    let unique_surviving_relation =
        unique_relations.len() == 1 && unique_relations.contains(relation_identity_sha256);
    let accepted = support_non_censored > 0
        && future_non_censored > 0
        && positive_rows > 0
        && applicability_negative_rows > 0
        && wrong_bindings == 0
        && negative_accepts == 0
        && verify_failed == 0
        && unique_surviving_relation;
    let production_admissible = accepted && all_external;
    let mut report = BindingAdjudicationReportV2 {
        schema: BINDING_ADJUDICATION_REPORT_SCHEMA_V2.to_owned(),
        report_sha256: String::new(),
        trusted_resolved_root_sha256: resolved.resolved_root_sha256().to_owned(),
        relation_identity_sha256: relation_identity_sha256.to_owned(),
        support_rows: support_non_censored,
        future_rows: future_non_censored,
        positive_rows,
        applicability_negative_rows,
        censored_rows: resolved.censored_rows(),
        wrong_bindings,
        negative_accepts,
        verify_failed,
        unique_surviving_relation,
        real_independent_receipts: resolved.real_independent_receipts(),
        production_admissible,
        execution_authority: false,
    };
    report.report_sha256 = binding_adjudication_report_digest_v2(&report)?;
    if accepted {
        let rows = non_censored.into_iter().cloned().collect::<Vec<_>>();
        let evidence_scope = if all_external {
            AcceptedBindingEvidenceScopeV2::ExternalIndependent
        } else {
            AcceptedBindingEvidenceScopeV2::ControlledFixture
        };
        let capability_root_sha256 = accepted_binding_law_evidence_digest_v2(&report, &rows)?;
        Ok(BindingAdjudicationOutcomeV2::Accepted(
            AcceptedBindingLawEvidenceV2 {
                report,
                evidence_scope,
                rows,
                capability_root_sha256,
            },
        ))
    } else if wrong_bindings > 0 || negative_accepts > 0 || verify_failed > 0 {
        Ok(BindingAdjudicationOutcomeV2::Rejected(report))
    } else {
        Ok(BindingAdjudicationOutcomeV2::Insufficient(report))
    }
}

fn binding_adjudication_report_digest_v2(
    report: &BindingAdjudicationReportV2,
) -> Result<String, BindingLawEvidenceV2Error> {
    sha256_json(&(
        report.schema.as_str(),
        report.trusted_resolved_root_sha256.as_str(),
        report.relation_identity_sha256.as_str(),
        report.support_rows,
        report.future_rows,
        report.positive_rows,
        report.applicability_negative_rows,
        report.censored_rows,
        report.wrong_bindings,
        report.negative_accepts,
        report.verify_failed,
        report.unique_surviving_relation,
        report.real_independent_receipts,
        report.production_admissible,
        report.execution_authority,
    ))
    .map_err(BindingLawEvidenceV2Error::from)
}

fn accepted_binding_law_evidence_digest_v2(
    report: &BindingAdjudicationReportV2,
    rows: &[TrustedResolvedBindingRowV2],
) -> Result<String, BindingLawEvidenceV2Error> {
    sha256_json(&(report, rows)).map_err(BindingLawEvidenceV2Error::from)
}
