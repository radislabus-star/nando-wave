use serde::{Deserialize, Serialize};

use nando_operator_kernel::stable_atom_id;

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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearnedWaveSubcenter {
    pub center_delta_micro: Vec<i32>,
    pub threshold_micro: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LearnedWaveRoute {
    pub cells: u16,
    pub center_delta_micro: Vec<i32>,
    pub threshold_micro: i64,
    #[serde(default)]
    pub query_atom_ids: Vec<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subcenters: Vec<LearnedWaveSubcenter>,
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
    } else if facts.support_rows < 32 {
        Some("support_rows_below_32")
    } else if facts.future_rows < 32 {
        Some("future_rows_below_32")
    } else if facts.distinct_sessions < 3 {
        Some("future_sessions_below_3")
    } else if facts.distinct_surfaces < 2 {
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
}
