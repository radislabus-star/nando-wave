use serde::{Deserialize, Serialize};

use nando_operator_kernel::{canonical_json_sha256, stable_atom_id, valid_nonzero_sha256};

pub use nando_operator_kernel::{LearnedWaveRoute, LearnedWaveSubcenter};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePackageOrigin {
    GroundedSynthesis,
    LegacyTemplate,
    RawPhaseInduction,
    ImportedFixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsePackageState {
    Quarantine,
    Active,
    Revoked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseRoutingComparison {
    AtMost,
    AtLeast,
    OneOf,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ResponseRoutingPredicate {
    pub role: String,
    pub comparison: ResponseRoutingComparison,
    pub threshold: u32,
    #[serde(default)]
    pub allowed_counts: Vec<u32>,
}

impl ResponseRoutingPredicate {
    #[must_use]
    pub fn matches_count(&self, count: u32) -> bool {
        match self.comparison {
            ResponseRoutingComparison::AtMost => count <= self.threshold,
            ResponseRoutingComparison::AtLeast => count >= self.threshold,
            ResponseRoutingComparison::OneOf => self.allowed_counts.binary_search(&count).is_ok(),
        }
    }

    #[must_use]
    pub fn phase_atom_id(&self) -> u64 {
        let material = match self.comparison {
            ResponseRoutingComparison::AtMost => {
                format!("cardinality_at_most:{}:{}", self.role, self.threshold)
            }
            ResponseRoutingComparison::AtLeast => {
                format!("cardinality_at_least:{}:{}", self.role, self.threshold)
            }
            ResponseRoutingComparison::OneOf => format!(
                "cardinality_one_of:{}:{}",
                self.role,
                self.allowed_counts
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        };
        stable_atom_id(&material)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponsePackageProof {
    pub support_rows: usize,
    pub future_rows: usize,
    pub distinct_sessions: usize,
    pub distinct_surfaces: usize,
    pub wrong_accepts: usize,
    pub runtime_parity_failures: usize,
    pub exact_cache_overlap: usize,
    pub wave_causal_pass: bool,
    pub verifier_schema: String,
    #[serde(default)]
    pub adaptive_identification: Option<AdaptiveIdentificationProofV1>,
}

pub const ADAPTIVE_IDENTIFICATION_PROOF_SCHEMA_V1: &str = "nando.adaptive-identification-proof.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptiveIdentificationProofV1 {
    schema: String,
    candidate_freeze_root_sha256: String,
    semantic_class_id_sha256: String,
    canonical_program_root_sha256: String,
    applicability_scope_root_sha256: String,
    transfer_proof_root_sha256: String,
    proof_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdaptiveIdentificationProofInputV1 {
    pub candidate_freeze_root_sha256: String,
    pub semantic_class_id_sha256: String,
    pub canonical_program_root_sha256: String,
    pub applicability_scope_root_sha256: String,
    pub transfer_proof_root_sha256: String,
}

pub fn seal_adaptive_identification_proof_v1(
    input: AdaptiveIdentificationProofInputV1,
) -> Result<AdaptiveIdentificationProofV1, &'static str> {
    validate_adaptive_identification_roots(&input)?;
    let proof_root_sha256 = adaptive_identification_proof_root(&input)?;
    Ok(AdaptiveIdentificationProofV1 {
        schema: ADAPTIVE_IDENTIFICATION_PROOF_SCHEMA_V1.to_owned(),
        candidate_freeze_root_sha256: input.candidate_freeze_root_sha256,
        semantic_class_id_sha256: input.semantic_class_id_sha256,
        canonical_program_root_sha256: input.canonical_program_root_sha256,
        applicability_scope_root_sha256: input.applicability_scope_root_sha256,
        transfer_proof_root_sha256: input.transfer_proof_root_sha256,
        proof_root_sha256,
    })
}

impl AdaptiveIdentificationProofV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != ADAPTIVE_IDENTIFICATION_PROOF_SCHEMA_V1 {
            return Err("adaptive_identification_schema_invalid");
        }
        let input = AdaptiveIdentificationProofInputV1 {
            candidate_freeze_root_sha256: self.candidate_freeze_root_sha256.clone(),
            semantic_class_id_sha256: self.semantic_class_id_sha256.clone(),
            canonical_program_root_sha256: self.canonical_program_root_sha256.clone(),
            applicability_scope_root_sha256: self.applicability_scope_root_sha256.clone(),
            transfer_proof_root_sha256: self.transfer_proof_root_sha256.clone(),
        };
        validate_adaptive_identification_roots(&input)?;
        if self.proof_root_sha256 != adaptive_identification_proof_root(&input)? {
            return Err("adaptive_identification_proof_root_mismatch");
        }
        Ok(())
    }

    #[must_use]
    pub fn proof_root_sha256(&self) -> &str {
        &self.proof_root_sha256
    }

    #[must_use]
    pub fn canonical_program_root_sha256(&self) -> &str {
        &self.canonical_program_root_sha256
    }

    pub fn matches_input(
        &self,
        input: &AdaptiveIdentificationProofInputV1,
    ) -> Result<bool, &'static str> {
        self.validate()?;
        validate_adaptive_identification_roots(input)?;
        Ok(
            self.candidate_freeze_root_sha256 == input.candidate_freeze_root_sha256
                && self.semantic_class_id_sha256 == input.semantic_class_id_sha256
                && self.canonical_program_root_sha256 == input.canonical_program_root_sha256
                && self.applicability_scope_root_sha256 == input.applicability_scope_root_sha256
                && self.transfer_proof_root_sha256 == input.transfer_proof_root_sha256
                && self.proof_root_sha256 == adaptive_identification_proof_root(input)?,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageAdmissionFacts {
    pub validation_blocker: Option<&'static str>,
    pub grounded_authority: bool,
    pub package_active: bool,
    pub support_rows: usize,
    pub future_rows: usize,
    pub distinct_sessions: usize,
    pub distinct_surfaces: usize,
    pub wrong_accepts: usize,
    pub runtime_parity_failures: usize,
    pub exact_cache_overlap: usize,
    pub wave_causal_pass: bool,
    pub verifier_schema_bound: bool,
    pub verifier_program_bound: bool,
    pub exact_guard_bound: bool,
    pub adaptive_identification_bound: bool,
}

#[must_use]
pub const fn package_admission_candidate_blocker(
    facts: PackageAdmissionFacts,
) -> Option<&'static str> {
    if let Some(blocker) = facts.validation_blocker {
        Some(blocker)
    } else if !facts.grounded_authority {
        Some("grounded_authority_missing")
    } else if !facts.package_active {
        Some("package_not_active")
    } else if facts.adaptive_identification_bound && facts.support_rows == 0 {
        Some("adaptive_support_missing")
    } else if facts.adaptive_identification_bound && facts.future_rows == 0 {
        Some("adaptive_future_missing")
    } else if facts.adaptive_identification_bound && facts.distinct_sessions < 2 {
        Some("adaptive_independent_session_missing")
    } else if facts.adaptive_identification_bound && facts.distinct_surfaces < 2 {
        Some("adaptive_surface_missing")
    } else if !facts.adaptive_identification_bound && facts.support_rows < 32 {
        Some("support_rows_below_32")
    } else if !facts.adaptive_identification_bound && facts.future_rows < 32 {
        Some("future_rows_below_32")
    } else if !facts.adaptive_identification_bound && facts.distinct_sessions < 3 {
        Some("future_sessions_below_3")
    } else if !facts.adaptive_identification_bound && facts.distinct_surfaces < 2 {
        Some("surfaces_below_2")
    } else if facts.wrong_accepts != 0 {
        Some("wrong_accepts_nonzero")
    } else if facts.runtime_parity_failures != 0 {
        Some("runtime_parity_failures_nonzero")
    } else if facts.exact_cache_overlap != 0 {
        Some("exact_cache_overlap_nonzero")
    } else if !facts.wave_causal_pass {
        Some("wave_causal_proof_missing")
    } else if !facts.verifier_schema_bound {
        Some("verifier_schema_not_bound")
    } else if !facts.verifier_program_bound {
        Some("verifier_program_not_bound")
    } else if !facts.exact_guard_bound {
        Some("exact_guard_not_bound")
    } else {
        None
    }
}

fn validate_adaptive_identification_roots(
    input: &AdaptiveIdentificationProofInputV1,
) -> Result<(), &'static str> {
    [
        input.candidate_freeze_root_sha256.as_str(),
        input.semantic_class_id_sha256.as_str(),
        input.canonical_program_root_sha256.as_str(),
        input.applicability_scope_root_sha256.as_str(),
        input.transfer_proof_root_sha256.as_str(),
    ]
    .into_iter()
    .all(valid_nonzero_sha256)
    .then_some(())
    .ok_or("adaptive_identification_root_invalid")
}

fn adaptive_identification_proof_root(
    input: &AdaptiveIdentificationProofInputV1,
) -> Result<String, &'static str> {
    canonical_json_sha256(&(
        ADAPTIVE_IDENTIFICATION_PROOF_SCHEMA_V1,
        input.candidate_freeze_root_sha256.as_str(),
        input.semantic_class_id_sha256.as_str(),
        input.canonical_program_root_sha256.as_str(),
        input.applicability_scope_root_sha256.as_str(),
        input.transfer_proof_root_sha256.as_str(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admitted() -> PackageAdmissionFacts {
        PackageAdmissionFacts {
            validation_blocker: None,
            grounded_authority: true,
            package_active: true,
            support_rows: 32,
            future_rows: 32,
            distinct_sessions: 3,
            distinct_surfaces: 2,
            wrong_accepts: 0,
            runtime_parity_failures: 0,
            exact_cache_overlap: 0,
            wave_causal_pass: true,
            verifier_schema_bound: true,
            verifier_program_bound: true,
            exact_guard_bound: true,
            adaptive_identification_bound: false,
        }
    }

    #[test]
    fn policy_is_fail_closed_and_ordered() {
        assert_eq!(package_admission_candidate_blocker(admitted()), None);
        let mut facts = admitted();
        facts.support_rows = 31;
        facts.wrong_accepts = 1;
        assert_eq!(
            package_admission_candidate_blocker(facts),
            Some("support_rows_below_32")
        );
        facts.support_rows = 32;
        assert_eq!(
            package_admission_candidate_blocker(facts),
            Some("wrong_accepts_nonzero")
        );
    }

    #[test]
    fn adaptive_identification_uses_proof_progress_instead_of_fixed_rows() {
        let mut facts = admitted();
        facts.adaptive_identification_bound = true;
        facts.support_rows = 1;
        facts.future_rows = 1;
        facts.distinct_sessions = 2;
        facts.distinct_surfaces = 2;
        assert_eq!(package_admission_candidate_blocker(facts), None);

        facts.future_rows = 0;
        assert_eq!(
            package_admission_candidate_blocker(facts),
            Some("adaptive_future_missing")
        );

        facts.future_rows = 1;
        facts.distinct_surfaces = 1;
        assert_eq!(
            package_admission_candidate_blocker(facts),
            Some("adaptive_surface_missing")
        );
    }
}
