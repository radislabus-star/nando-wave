use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::evidence::K1ConsequenceTypeV1;
use super::{
    K1_MOTIF_CANDIDATE_SUPPORT_SCHEMA_V1, K1_MOTIF_DISPOSITION_SUMMARY_SCHEMA_V1,
    K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V2, K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3,
    K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4, K1_NATURAL_COHORT_CATALOG_SCHEMA_V1,
    K1_NATURAL_COHORT_CATALOG_SCHEMA_V2, K1CandidateReadinessV1, strict_roots,
};

fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalCohortCandidateV1 {
    pub schema: String,
    pub candidate_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub candidate_structural_root_sha256: String,
    pub source_neutral_topology_root_sha256: String,
    pub semantic_novelty_signature_root_sha256: String,
    pub consequence_type: K1ConsequenceTypeV1,
    pub evidence_manifest_root_sha256: String,
    pub evidence_rows: u64,
    pub settled_rows: u64,
    pub verified_rows: u64,
    pub independent_lineages: u64,
    pub expected_verified_input_tokens: u64,
    pub bounded_discovery_cost_units: u64,
    pub first_capture_sequence: u64,
    pub last_capture_sequence: u64,
    pub generator_schema: String,
    pub readiness: K1CandidateReadinessV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub complete_topology_manifest_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub motif_embedding_manifest_root_sha256: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub motif_support_overflow_occurrences: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub motif_support_overflow_manifest_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalCohortCatalogV1 {
    pub schema: String,
    pub catalog_root_sha256: String,
    pub evidence_epoch_root_sha256: String,
    pub fixture_exclusion_root_sha256: String,
    pub scanned_rows: u64,
    pub natural_rows: u64,
    pub controlled_rows_excluded: u64,
    pub generated_fixture_rows_excluded: u64,
    pub unknown_rows_excluded: u64,
    pub safety_veto_rows_excluded: u64,
    pub candidates: Vec<K1NaturalCohortCandidateV1>,
    pub authority_ready: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub motif_retained_occurrences: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motif_disposition: Option<K1MotifDispositionSummaryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1MotifDispositionSummaryV1 {
    pub schema: String,
    pub summary_root_sha256: String,
    pub enumeration_config_root_sha256: String,
    pub scanned_source_rows: u64,
    pub motif_source_rows: u64,
    pub retained_motif_occurrences: u64,
    pub support_overflow_occurrences: u64,
    pub support_overflow_manifest_root_sha256: String,
    pub budget_censored_rows: u64,
    pub budget_censored_manifest_root_sha256: String,
    pub empty_or_incomplete_rows: u64,
    pub empty_or_incomplete_manifest_root_sha256: String,
    pub invalid_embedding_rows: u64,
    pub invalid_embedding_manifest_root_sha256: String,
    pub fixture_or_controlled_excluded_rows: u64,
    pub fixture_or_controlled_manifest_root_sha256: String,
    pub safety_veto_rows: u64,
    pub safety_veto_manifest_root_sha256: String,
    pub source_disposition_manifest_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1MotifCandidateSupportV1 {
    pub schema: String,
    pub support_root_sha256: String,
    pub capture_generation_root_sha256: String,
    pub motif_root_sha256: String,
    pub semantic_novelty_signature_root_sha256: String,
    pub consequence_type: K1ConsequenceTypeV1,
    pub retained_rows: u64,
    pub retained_manifest_root_sha256: String,
    pub overflow_occurrences: u64,
    pub overflow_manifest_root_sha256: String,
}

#[derive(Serialize)]
struct CandidateDigestV1<'a> {
    schema: &'a str,
    candidate_structural_root_sha256: &'a str,
    capture_generation_root_sha256: &'a str,
    source_neutral_topology_root_sha256: &'a str,
    semantic_novelty_signature_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    evidence_manifest_root_sha256: &'a str,
    evidence_rows: u64,
    settled_rows: u64,
    verified_rows: u64,
    independent_lineages: u64,
    expected_verified_input_tokens: u64,
    bounded_discovery_cost_units: u64,
    first_capture_sequence: u64,
    last_capture_sequence: u64,
    generator_schema: &'a str,
    readiness_receipt_root_sha256: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct CandidateDigestV4<'a> {
    schema: &'static str,
    capture_generation_root_sha256: &'a str,
    candidate_structural_root_sha256: &'a str,
    source_neutral_topology_root_sha256: &'a str,
    semantic_novelty_signature_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    evidence_manifest_root_sha256: &'a str,
    complete_topology_manifest_root_sha256: &'a str,
    motif_embedding_manifest_root_sha256: &'a str,
    motif_support_overflow_occurrences: u64,
    motif_support_overflow_manifest_root_sha256: &'a str,
    evidence_rows: u64,
    settled_rows: u64,
    verified_rows: u64,
    independent_lineages: u64,
    expected_verified_input_tokens: u64,
    bounded_discovery_cost_units: u64,
    first_capture_sequence: u64,
    last_capture_sequence: u64,
    generator_schema: &'a str,
    readiness_receipt_root_sha256: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct MotifDispositionDigestV1<'a> {
    schema: &'static str,
    enumeration_config_root_sha256: &'a str,
    scanned_source_rows: u64,
    motif_source_rows: u64,
    retained_motif_occurrences: u64,
    support_overflow_occurrences: u64,
    support_overflow_manifest_root_sha256: &'a str,
    budget_censored_rows: u64,
    budget_censored_manifest_root_sha256: &'a str,
    empty_or_incomplete_rows: u64,
    empty_or_incomplete_manifest_root_sha256: &'a str,
    invalid_embedding_rows: u64,
    invalid_embedding_manifest_root_sha256: &'a str,
    fixture_or_controlled_excluded_rows: u64,
    fixture_or_controlled_manifest_root_sha256: &'a str,
    safety_veto_rows: u64,
    safety_veto_manifest_root_sha256: &'a str,
    source_disposition_manifest_root_sha256: &'a str,
}

impl K1NaturalCohortCandidateV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.readiness.validate()?;
        let roots = [
            self.candidate_root_sha256.as_str(),
            self.capture_generation_root_sha256.as_str(),
            self.candidate_structural_root_sha256.as_str(),
            self.source_neutral_topology_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.evidence_manifest_root_sha256.as_str(),
        ];
        let motif_fields_valid = match self.schema.as_str() {
            K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V2 | K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3 => {
                self.complete_topology_manifest_root_sha256.is_empty()
                    && self.motif_embedding_manifest_root_sha256.is_empty()
                    && self.motif_support_overflow_occurrences == 0
                    && self.motif_support_overflow_manifest_root_sha256.is_empty()
            }
            K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4 => {
                valid_nonzero_sha256(&self.complete_topology_manifest_root_sha256)
                    && valid_nonzero_sha256(&self.motif_embedding_manifest_root_sha256)
                    && valid_nonzero_sha256(&self.motif_support_overflow_manifest_root_sha256)
            }
            _ => false,
        };
        if !motif_fields_valid
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.generator_schema.is_empty()
            || self.evidence_rows == 0
            || self.settled_rows > self.evidence_rows
            || self.verified_rows > self.settled_rows
            || self.first_capture_sequence == 0
            || self.last_capture_sequence < self.first_capture_sequence
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.candidate_root_sha256 != self.expected_root()?
        {
            return Err("k1_natural_cohort_candidate_invalid");
        }
        Ok(())
    }

    pub(in super::super) fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4 {
            return canonical_json_sha256(&CandidateDigestV4 {
                schema: K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4,
                capture_generation_root_sha256: &self.capture_generation_root_sha256,
                candidate_structural_root_sha256: &self.candidate_structural_root_sha256,
                source_neutral_topology_root_sha256: &self.source_neutral_topology_root_sha256,
                semantic_novelty_signature_root_sha256: &self
                    .semantic_novelty_signature_root_sha256,
                consequence_type: self.consequence_type,
                evidence_manifest_root_sha256: &self.evidence_manifest_root_sha256,
                complete_topology_manifest_root_sha256: &self
                    .complete_topology_manifest_root_sha256,
                motif_embedding_manifest_root_sha256: &self.motif_embedding_manifest_root_sha256,
                motif_support_overflow_occurrences: self.motif_support_overflow_occurrences,
                motif_support_overflow_manifest_root_sha256: &self
                    .motif_support_overflow_manifest_root_sha256,
                evidence_rows: self.evidence_rows,
                settled_rows: self.settled_rows,
                verified_rows: self.verified_rows,
                independent_lineages: self.independent_lineages,
                expected_verified_input_tokens: self.expected_verified_input_tokens,
                bounded_discovery_cost_units: self.bounded_discovery_cost_units,
                first_capture_sequence: self.first_capture_sequence,
                last_capture_sequence: self.last_capture_sequence,
                generator_schema: &self.generator_schema,
                readiness_receipt_root_sha256: &self.readiness.readiness_receipt_root_sha256,
                authority_ready: false,
                phase_mutation_allowed: false,
            });
        }
        canonical_json_sha256(&CandidateDigestV1 {
            schema: self.schema.as_str(),
            capture_generation_root_sha256: &self.capture_generation_root_sha256,
            candidate_structural_root_sha256: &self.candidate_structural_root_sha256,
            source_neutral_topology_root_sha256: &self.source_neutral_topology_root_sha256,
            semantic_novelty_signature_root_sha256: &self.semantic_novelty_signature_root_sha256,
            consequence_type: self.consequence_type,
            evidence_manifest_root_sha256: &self.evidence_manifest_root_sha256,
            evidence_rows: self.evidence_rows,
            settled_rows: self.settled_rows,
            verified_rows: self.verified_rows,
            independent_lineages: self.independent_lineages,
            expected_verified_input_tokens: self.expected_verified_input_tokens,
            bounded_discovery_cost_units: self.bounded_discovery_cost_units,
            first_capture_sequence: self.first_capture_sequence,
            last_capture_sequence: self.last_capture_sequence,
            generator_schema: &self.generator_schema,
            readiness_receipt_root_sha256: &self.readiness.readiness_receipt_root_sha256,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
    }
}

impl K1NaturalCohortCatalogV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        let candidate_evidence_rows = self
            .candidates
            .iter()
            .map(|candidate| candidate.evidence_rows)
            .try_fold(0u64, u64::checked_add)
            .ok_or("k1_natural_cohort_catalog_count")?;
        let version_fields_valid = match self.schema.as_str() {
            K1_NATURAL_COHORT_CATALOG_SCHEMA_V1 => {
                self.motif_disposition.is_none()
                    && self.motif_retained_occurrences == 0
                    && self.candidates.iter().all(|candidate| {
                        matches!(
                            candidate.schema.as_str(),
                            K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V2
                                | K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V3
                        )
                    })
                    && self.scanned_rows
                        == self
                            .natural_rows
                            .saturating_add(self.controlled_rows_excluded)
                            .saturating_add(self.generated_fixture_rows_excluded)
                            .saturating_add(self.unknown_rows_excluded)
                            .saturating_add(self.safety_veto_rows_excluded)
            }
            K1_NATURAL_COHORT_CATALOG_SCHEMA_V2 => {
                self.motif_disposition.as_ref().is_some_and(|summary| {
                    summary.validate().is_ok()
                        && self.candidates.iter().all(|candidate| {
                            candidate.schema == K1_NATURAL_COHORT_CANDIDATE_SCHEMA_V4
                        })
                        && self.scanned_rows == summary.scanned_source_rows
                        && self.natural_rows == summary.motif_source_rows
                        && self.motif_retained_occurrences == summary.retained_motif_occurrences
                        && self.controlled_rows_excluded
                            == summary.fixture_or_controlled_excluded_rows
                        && self.generated_fixture_rows_excluded == 0
                        && self.unknown_rows_excluded == 0
                        && self.safety_veto_rows_excluded == summary.safety_veto_rows
                })
            }
            _ => false,
        };
        if !version_fields_valid
            || !valid_nonzero_sha256(&self.catalog_root_sha256)
            || !valid_nonzero_sha256(&self.evidence_epoch_root_sha256)
            || !valid_nonzero_sha256(&self.fixture_exclusion_root_sha256)
            || self.candidates.iter().any(|row| row.validate().is_err())
            || candidate_evidence_rows
                != if self.schema == K1_NATURAL_COHORT_CATALOG_SCHEMA_V2 {
                    self.motif_retained_occurrences
                } else {
                    self.natural_rows
                }
            || !strict_roots(
                self.candidates
                    .iter()
                    .map(|row| row.candidate_root_sha256.as_str()),
            )
            || self.authority_ready
            || self.catalog_root_sha256 != self.expected_root()?
        {
            return Err("k1_natural_cohort_catalog_invalid");
        }
        Ok(())
    }

    pub(in super::super) fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == K1_NATURAL_COHORT_CATALOG_SCHEMA_V2 {
            return canonical_json_sha256(&(
                K1_NATURAL_COHORT_CATALOG_SCHEMA_V2,
                self.evidence_epoch_root_sha256.as_str(),
                self.fixture_exclusion_root_sha256.as_str(),
                self.scanned_rows,
                self.natural_rows,
                self.controlled_rows_excluded,
                self.generated_fixture_rows_excluded,
                self.unknown_rows_excluded,
                self.safety_veto_rows_excluded,
                self.motif_retained_occurrences,
                self.candidates
                    .iter()
                    .map(|row| row.candidate_root_sha256.as_str())
                    .collect::<Vec<_>>(),
                self.motif_disposition
                    .as_ref()
                    .map(|summary| summary.summary_root_sha256.as_str()),
                false,
            ));
        }
        canonical_json_sha256(&(
            K1_NATURAL_COHORT_CATALOG_SCHEMA_V1,
            self.evidence_epoch_root_sha256.as_str(),
            self.fixture_exclusion_root_sha256.as_str(),
            self.scanned_rows,
            self.natural_rows,
            self.controlled_rows_excluded,
            self.generated_fixture_rows_excluded,
            self.unknown_rows_excluded,
            self.safety_veto_rows_excluded,
            self.candidates
                .iter()
                .map(|row| row.candidate_root_sha256.as_str())
                .collect::<Vec<_>>(),
            false,
        ))
    }
}

impl K1MotifDispositionSummaryV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        enumeration_config_root_sha256: String,
        scanned_source_rows: u64,
        motif_source_rows: u64,
        retained_motif_occurrences: u64,
        support_overflow_occurrences: u64,
        support_overflow_manifest_root_sha256: String,
        budget_censored_rows: u64,
        budget_censored_manifest_root_sha256: String,
        empty_or_incomplete_rows: u64,
        empty_or_incomplete_manifest_root_sha256: String,
        invalid_embedding_rows: u64,
        invalid_embedding_manifest_root_sha256: String,
        fixture_or_controlled_excluded_rows: u64,
        fixture_or_controlled_manifest_root_sha256: String,
        safety_veto_rows: u64,
        safety_veto_manifest_root_sha256: String,
        source_disposition_manifest_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut summary = Self {
            schema: K1_MOTIF_DISPOSITION_SUMMARY_SCHEMA_V1.to_owned(),
            summary_root_sha256: String::new(),
            enumeration_config_root_sha256,
            scanned_source_rows,
            motif_source_rows,
            retained_motif_occurrences,
            support_overflow_occurrences,
            support_overflow_manifest_root_sha256,
            budget_censored_rows,
            budget_censored_manifest_root_sha256,
            empty_or_incomplete_rows,
            empty_or_incomplete_manifest_root_sha256,
            invalid_embedding_rows,
            invalid_embedding_manifest_root_sha256,
            fixture_or_controlled_excluded_rows,
            fixture_or_controlled_manifest_root_sha256,
            safety_veto_rows,
            safety_veto_manifest_root_sha256,
            source_disposition_manifest_root_sha256,
        };
        summary.summary_root_sha256 = summary.expected_root()?;
        summary.validate()?;
        Ok(summary)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_MOTIF_DISPOSITION_SUMMARY_SCHEMA_V1
            || ![
                self.summary_root_sha256.as_str(),
                self.enumeration_config_root_sha256.as_str(),
                self.support_overflow_manifest_root_sha256.as_str(),
                self.budget_censored_manifest_root_sha256.as_str(),
                self.empty_or_incomplete_manifest_root_sha256.as_str(),
                self.invalid_embedding_manifest_root_sha256.as_str(),
                self.fixture_or_controlled_manifest_root_sha256.as_str(),
                self.safety_veto_manifest_root_sha256.as_str(),
                self.source_disposition_manifest_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.scanned_source_rows
                != self
                    .motif_source_rows
                    .saturating_add(self.budget_censored_rows)
                    .saturating_add(self.empty_or_incomplete_rows)
                    .saturating_add(self.invalid_embedding_rows)
                    .saturating_add(self.fixture_or_controlled_excluded_rows)
                    .saturating_add(self.safety_veto_rows)
            || self.summary_root_sha256 != self.expected_root()?
        {
            return Err("k1_motif_disposition_summary_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&MotifDispositionDigestV1 {
            schema: K1_MOTIF_DISPOSITION_SUMMARY_SCHEMA_V1,
            enumeration_config_root_sha256: &self.enumeration_config_root_sha256,
            scanned_source_rows: self.scanned_source_rows,
            motif_source_rows: self.motif_source_rows,
            retained_motif_occurrences: self.retained_motif_occurrences,
            support_overflow_occurrences: self.support_overflow_occurrences,
            support_overflow_manifest_root_sha256: &self.support_overflow_manifest_root_sha256,
            budget_censored_rows: self.budget_censored_rows,
            budget_censored_manifest_root_sha256: &self.budget_censored_manifest_root_sha256,
            empty_or_incomplete_rows: self.empty_or_incomplete_rows,
            empty_or_incomplete_manifest_root_sha256: &self
                .empty_or_incomplete_manifest_root_sha256,
            invalid_embedding_rows: self.invalid_embedding_rows,
            invalid_embedding_manifest_root_sha256: &self.invalid_embedding_manifest_root_sha256,
            fixture_or_controlled_excluded_rows: self.fixture_or_controlled_excluded_rows,
            fixture_or_controlled_manifest_root_sha256: &self
                .fixture_or_controlled_manifest_root_sha256,
            safety_veto_rows: self.safety_veto_rows,
            safety_veto_manifest_root_sha256: &self.safety_veto_manifest_root_sha256,
            source_disposition_manifest_root_sha256: &self.source_disposition_manifest_root_sha256,
        })
    }
}

impl K1MotifCandidateSupportV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        capture_generation_root_sha256: String,
        motif_root_sha256: String,
        semantic_novelty_signature_root_sha256: String,
        consequence_type: K1ConsequenceTypeV1,
        retained_rows: u64,
        retained_manifest_root_sha256: String,
        overflow_occurrences: u64,
        overflow_manifest_root_sha256: String,
    ) -> Result<Self, &'static str> {
        let mut support = Self {
            schema: K1_MOTIF_CANDIDATE_SUPPORT_SCHEMA_V1.to_owned(),
            support_root_sha256: String::new(),
            capture_generation_root_sha256,
            motif_root_sha256,
            semantic_novelty_signature_root_sha256,
            consequence_type,
            retained_rows,
            retained_manifest_root_sha256,
            overflow_occurrences,
            overflow_manifest_root_sha256,
        };
        support.support_root_sha256 = support.expected_root()?;
        support.validate()?;
        Ok(support)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_MOTIF_CANDIDATE_SUPPORT_SCHEMA_V1
            || ![
                self.support_root_sha256.as_str(),
                self.capture_generation_root_sha256.as_str(),
                self.motif_root_sha256.as_str(),
                self.semantic_novelty_signature_root_sha256.as_str(),
                self.retained_manifest_root_sha256.as_str(),
                self.overflow_manifest_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.retained_rows == 0
            || self.retained_rows > 64
            || self.support_root_sha256 != self.expected_root()?
        {
            return Err("k1_motif_candidate_support_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_MOTIF_CANDIDATE_SUPPORT_SCHEMA_V1,
            self.capture_generation_root_sha256.as_str(),
            self.motif_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.consequence_type,
            self.retained_rows,
            self.retained_manifest_root_sha256.as_str(),
            self.overflow_occurrences,
            self.overflow_manifest_root_sha256.as_str(),
        ))
    }
}
