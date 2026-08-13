use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::super::{ExactAttemptIndexV1, IdentifierCausalInputManifestV1};
use super::evidence::K1ConsequenceTypeV1;
use super::{
    K1_DEFICIT_SNAPSHOT_SCHEMA_V1, K1_NATURAL_CANDIDATE_MAX_ROWS_V1,
    K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1, K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2,
    K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3, K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4,
    canonical_root_slice, canonical_roots, strict_values,
};

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1DeficitSnapshotV1 {
    pub schema: String,
    pub snapshot_root_sha256: String,
    pub epistemic_registry_revision: u64,
    pub epistemic_registry_root_sha256: String,
    pub k1_gate_root_sha256: String,
    pub law_certificates: u64,
    pub semantic_laws: u64,
    pub role_topologies: u64,
    pub cleanup_receipts: u64,
    pub false_bad_apply: u64,
    pub minimum_law_certificates: u64,
    pub minimum_semantic_laws: u64,
    pub minimum_role_topologies: u64,
    pub law_deficit: u64,
    pub semantic_deficit: u64,
    pub topology_deficit: u64,
    pub eligible_semantic_law_roots_sha256: Vec<String>,
    pub eligible_role_topology_roots_sha256: Vec<String>,
    pub known_consequence_types: Vec<K1ConsequenceTypeV1>,
    pub k1_open: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1CandidateScoreV1 {
    pub total_k1_gain: u8,
    pub law_gain: u8,
    pub semantic_gain: u8,
    pub topology_gain: u8,
    pub readiness_rank: u8,
    pub bounded_discovery_cost_units: u64,
    pub expected_verified_input_tokens: u64,
    pub stable_hash_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalCandidateQueueRowV1 {
    pub candidate_root_sha256: String,
    pub readiness_receipt_root_sha256: String,
    pub score: K1CandidateScoreV1,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub terminal_failure_family_novelty_rank: u8,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub causal_manifest_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub opportunity_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub exact_attempt_state: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalCandidateQueueV1 {
    pub schema: String,
    pub queue_root_sha256: String,
    pub catalog_root_sha256: String,
    pub k1_deficit_snapshot_root_sha256: String,
    pub fixture_exclusion_root_sha256: String,
    pub catalog_candidates: u64,
    pub completed_candidates_excluded: u64,
    pub scored_candidates: u64,
    pub capacity_excluded_candidates: u64,
    pub readiness_rescue_included: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub terminal_failure_quotient_root_sha256: String,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub terminal_failure_observations: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub terminal_failure_exhausted_families: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub terminal_failure_demoted_current_candidates: u64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub exact_attempt_index_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub artifact_source_snapshot_root_sha256: String,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub exact_unseen_opportunities: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub exact_attempted_deterministic_roots: u64,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub legacy_unbound_terminals: u64,
    pub rows: Vec<K1NaturalCandidateQueueRowV1>,
    pub authority_ready: bool,
}

#[derive(Serialize)]
struct DeficitDigestV1<'a> {
    schema: &'static str,
    epistemic_registry_revision: u64,
    epistemic_registry_root_sha256: &'a str,
    k1_gate_root_sha256: &'a str,
    law_certificates: u64,
    semantic_laws: u64,
    role_topologies: u64,
    cleanup_receipts: u64,
    false_bad_apply: u64,
    minimum_law_certificates: u64,
    minimum_semantic_laws: u64,
    minimum_role_topologies: u64,
    law_deficit: u64,
    semantic_deficit: u64,
    topology_deficit: u64,
    eligible_semantic_law_roots_sha256: &'a [String],
    eligible_role_topology_roots_sha256: &'a [String],
    known_consequence_types: &'a [K1ConsequenceTypeV1],
    k1_open: bool,
}

impl K1DeficitSnapshotV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        epistemic_registry_revision: u64,
        epistemic_registry_root_sha256: String,
        k1_gate_root_sha256: String,
        law_certificates: u64,
        semantic_laws: u64,
        role_topologies: u64,
        cleanup_receipts: u64,
        false_bad_apply: u64,
        minimum_law_certificates: u64,
        minimum_semantic_laws: u64,
        minimum_role_topologies: u64,
        mut eligible_semantic_law_roots_sha256: Vec<String>,
        mut eligible_role_topology_roots_sha256: Vec<String>,
        mut known_consequence_types: Vec<K1ConsequenceTypeV1>,
        k1_open: bool,
    ) -> Result<Self, &'static str> {
        canonical_roots(&mut eligible_semantic_law_roots_sha256)?;
        canonical_roots(&mut eligible_role_topology_roots_sha256)?;
        known_consequence_types.sort_unstable();
        known_consequence_types.dedup();
        let mut snapshot = Self {
            schema: K1_DEFICIT_SNAPSHOT_SCHEMA_V1.to_owned(),
            snapshot_root_sha256: String::new(),
            epistemic_registry_revision,
            epistemic_registry_root_sha256,
            k1_gate_root_sha256,
            law_certificates,
            semantic_laws,
            role_topologies,
            cleanup_receipts,
            false_bad_apply,
            minimum_law_certificates,
            minimum_semantic_laws,
            minimum_role_topologies,
            law_deficit: minimum_law_certificates.saturating_sub(law_certificates),
            semantic_deficit: minimum_semantic_laws.saturating_sub(semantic_laws),
            topology_deficit: minimum_role_topologies.saturating_sub(role_topologies),
            eligible_semantic_law_roots_sha256,
            eligible_role_topology_roots_sha256,
            known_consequence_types,
            k1_open,
        };
        snapshot.snapshot_root_sha256 = snapshot.expected_root()?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let expected_open = self.law_certificates >= self.minimum_law_certificates
            && self.semantic_laws >= self.minimum_semantic_laws
            && self.role_topologies >= self.minimum_role_topologies
            && self.cleanup_receipts == self.law_certificates
            && self.false_bad_apply == 0;
        if self.schema != K1_DEFICIT_SNAPSHOT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.snapshot_root_sha256)
            || !valid_nonzero_sha256(&self.epistemic_registry_root_sha256)
            || !valid_nonzero_sha256(&self.k1_gate_root_sha256)
            || self.minimum_law_certificates == 0
            || self.minimum_semantic_laws == 0
            || self.minimum_role_topologies == 0
            || self.law_deficit
                != self
                    .minimum_law_certificates
                    .saturating_sub(self.law_certificates)
            || self.semantic_deficit
                != self
                    .minimum_semantic_laws
                    .saturating_sub(self.semantic_laws)
            || self.topology_deficit
                != self
                    .minimum_role_topologies
                    .saturating_sub(self.role_topologies)
            || self.k1_open != expected_open
            || !canonical_root_slice(&self.eligible_semantic_law_roots_sha256)
            || !canonical_root_slice(&self.eligible_role_topology_roots_sha256)
            || !strict_values(&self.known_consequence_types)
            || self.snapshot_root_sha256 != self.expected_root()?
        {
            return Err("k1_deficit_snapshot_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&DeficitDigestV1 {
            schema: K1_DEFICIT_SNAPSHOT_SCHEMA_V1,
            epistemic_registry_revision: self.epistemic_registry_revision,
            epistemic_registry_root_sha256: &self.epistemic_registry_root_sha256,
            k1_gate_root_sha256: &self.k1_gate_root_sha256,
            law_certificates: self.law_certificates,
            semantic_laws: self.semantic_laws,
            role_topologies: self.role_topologies,
            cleanup_receipts: self.cleanup_receipts,
            false_bad_apply: self.false_bad_apply,
            minimum_law_certificates: self.minimum_law_certificates,
            minimum_semantic_laws: self.minimum_semantic_laws,
            minimum_role_topologies: self.minimum_role_topologies,
            law_deficit: self.law_deficit,
            semantic_deficit: self.semantic_deficit,
            topology_deficit: self.topology_deficit,
            eligible_semantic_law_roots_sha256: &self.eligible_semantic_law_roots_sha256,
            eligible_role_topology_roots_sha256: &self.eligible_role_topology_roots_sha256,
            known_consequence_types: &self.known_consequence_types,
            k1_open: self.k1_open,
        })
    }
}

impl K1CandidateScoreV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.total_k1_gain
            != self
                .law_gain
                .saturating_add(self.semantic_gain)
                .saturating_add(self.topology_gain)
            || self.law_gain > 1
            || self.semantic_gain > 1
            || self.topology_gain > 1
            || self.readiness_rank > 1
            || !valid_nonzero_sha256(&self.stable_hash_sha256)
        {
            return Err("k1_candidate_score_invalid");
        }
        Ok(())
    }
}

impl K1NaturalCandidateQueueV1 {
    pub fn bind_exact_opportunities_v4(
        mut self,
        exact_attempt_index: &ExactAttemptIndexV1,
        artifact_source_snapshot_root_sha256: String,
        causal_manifests_by_candidate: &std::collections::BTreeMap<
            String,
            IdentifierCausalInputManifestV1,
        >,
    ) -> Result<Self, &'static str> {
        self.validate()?;
        exact_attempt_index.validate()?;
        if self.schema != K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2
            || !valid_nonzero_sha256(&artifact_source_snapshot_root_sha256)
        {
            return Err("k1_exact_candidate_queue_input_invalid");
        }
        let mut unseen = 0u64;
        let mut attempted = 0u64;
        for row in &mut self.rows {
            if row.score.readiness_rank == 0 {
                if causal_manifests_by_candidate.contains_key(&row.candidate_root_sha256) {
                    return Err("k1_exact_non_ready_candidate_manifest_forbidden");
                }
                continue;
            }
            let manifest = causal_manifests_by_candidate
                .get(&row.candidate_root_sha256)
                .ok_or("k1_exact_ready_candidate_manifest_missing")?;
            manifest.validate()?;
            row.causal_manifest_root_sha256 = manifest.manifest_root_sha256.clone();
            row.opportunity_root_sha256 = manifest.opportunity_root_sha256.clone();
            if exact_attempt_index.contains(&manifest.opportunity_root_sha256) {
                row.exact_attempt_state = "attempted_deterministic".to_owned();
                attempted = attempted.saturating_add(1);
            } else {
                row.exact_attempt_state = "unseen".to_owned();
                unseen = unseen.saturating_add(1);
            }
        }
        if causal_manifests_by_candidate.len()
            != self
                .rows
                .iter()
                .filter(|row| row.score.readiness_rank == 1)
                .count()
        {
            return Err("k1_exact_candidate_manifest_set_invalid");
        }
        self.schema = K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4.to_owned();
        self.exact_attempt_index_root_sha256 = exact_attempt_index.index_root_sha256.clone();
        self.artifact_source_snapshot_root_sha256 = artifact_source_snapshot_root_sha256;
        self.exact_unseen_opportunities = unseen;
        self.exact_attempted_deterministic_roots = attempted;
        self.legacy_unbound_terminals = exact_attempt_index.legacy_unbound_terminals;
        self.queue_root_sha256 = self.expected_root()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let retained_candidates =
            u64::try_from(self.rows.len()).map_err(|_| "k1_natural_candidate_queue_count")?;
        let catalog_partition = self
            .completed_candidates_excluded
            .checked_add(self.scored_candidates);
        let scored_partition = retained_candidates.checked_add(self.capacity_excluded_candidates);
        if !matches!(
            self.schema.as_str(),
            K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V1
                | K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2
                | K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3
                | K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
        ) || !valid_nonzero_sha256(&self.queue_root_sha256)
            || !valid_nonzero_sha256(&self.catalog_root_sha256)
            || !valid_nonzero_sha256(&self.k1_deficit_snapshot_root_sha256)
            || !valid_nonzero_sha256(&self.fixture_exclusion_root_sha256)
            || catalog_partition != Some(self.catalog_candidates)
            || scored_partition != Some(self.scored_candidates)
            || (self.readiness_rescue_included
                && (self.capacity_excluded_candidates == 0
                    || !self.rows.iter().any(|row| row.score.readiness_rank == 1)))
            || self.rows.len() > K1_NATURAL_CANDIDATE_MAX_ROWS_V1
            || self.rows.iter().any(|row| {
                !valid_nonzero_sha256(&row.candidate_root_sha256)
                    || !valid_nonzero_sha256(&row.readiness_receipt_root_sha256)
                    || row.terminal_failure_family_novelty_rank > 1
                    || (self.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
                        && if row.score.readiness_rank == 1 {
                            !valid_nonzero_sha256(&row.causal_manifest_root_sha256)
                                || !valid_nonzero_sha256(&row.opportunity_root_sha256)
                                || !matches!(
                                    row.exact_attempt_state.as_str(),
                                    "unseen" | "attempted_deterministic"
                                )
                        } else {
                            !row.causal_manifest_root_sha256.is_empty()
                                || !row.opportunity_root_sha256.is_empty()
                                || !row.exact_attempt_state.is_empty()
                        })
                    || (self.schema != K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
                        && (!row.causal_manifest_root_sha256.is_empty()
                            || !row.opportunity_root_sha256.is_empty()
                            || !row.exact_attempt_state.is_empty()))
                    || row.score.validate().is_err()
            })
            || (self.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3
                && (!valid_nonzero_sha256(&self.terminal_failure_quotient_root_sha256)
                    || self.terminal_failure_demoted_current_candidates > self.catalog_candidates))
            || (self.schema != K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3
                && (!self.terminal_failure_quotient_root_sha256.is_empty()
                    || self.terminal_failure_observations != 0
                    || self.terminal_failure_exhausted_families != 0
                    || self.terminal_failure_demoted_current_candidates != 0
                    || self
                        .rows
                        .iter()
                        .any(|row| row.terminal_failure_family_novelty_rank != 0)))
            || (self.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
                && (!valid_nonzero_sha256(&self.exact_attempt_index_root_sha256)
                    || !valid_nonzero_sha256(&self.artifact_source_snapshot_root_sha256)
                    || self
                        .exact_unseen_opportunities
                        .saturating_add(self.exact_attempted_deterministic_roots)
                        != self
                            .rows
                            .iter()
                            .filter(|row| row.score.readiness_rank == 1)
                            .count() as u64))
            || (self.schema != K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
                && (!self.exact_attempt_index_root_sha256.is_empty()
                    || !self.artifact_source_snapshot_root_sha256.is_empty()
                    || self.exact_unseen_opportunities != 0
                    || self.exact_attempted_deterministic_roots != 0
                    || self.legacy_unbound_terminals != 0))
            || self
                .rows
                .iter()
                .map(|row| row.candidate_root_sha256.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != self.rows.len()
            || !self
                .rows
                .windows(2)
                .all(|pair| pair[0].ranks_before(&pair[1], &self.schema))
            || self.authority_ready
            || self.queue_root_sha256 != self.expected_root()?
        {
            return Err("k1_natural_candidate_queue_invalid");
        }
        Ok(())
    }

    pub(in super::super) fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4 {
            return canonical_json_sha256(&(
                K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4,
                self.catalog_root_sha256.as_str(),
                self.k1_deficit_snapshot_root_sha256.as_str(),
                self.fixture_exclusion_root_sha256.as_str(),
                self.catalog_candidates,
                self.completed_candidates_excluded,
                self.scored_candidates,
                self.capacity_excluded_candidates,
                self.readiness_rescue_included,
                self.exact_attempt_index_root_sha256.as_str(),
                self.artifact_source_snapshot_root_sha256.as_str(),
                self.exact_unseen_opportunities,
                self.exact_attempted_deterministic_roots,
                self.legacy_unbound_terminals,
                &self.rows,
                false,
            ));
        }
        if self.schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3 {
            return canonical_json_sha256(&(
                K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3,
                self.catalog_root_sha256.as_str(),
                self.k1_deficit_snapshot_root_sha256.as_str(),
                self.fixture_exclusion_root_sha256.as_str(),
                self.catalog_candidates,
                self.completed_candidates_excluded,
                self.scored_candidates,
                self.capacity_excluded_candidates,
                self.readiness_rescue_included,
                self.terminal_failure_quotient_root_sha256.as_str(),
                self.terminal_failure_observations,
                self.terminal_failure_exhausted_families,
                self.terminal_failure_demoted_current_candidates,
                &self.rows,
                false,
            ));
        }
        canonical_json_sha256(&(
            self.schema.as_str(),
            self.catalog_root_sha256.as_str(),
            self.k1_deficit_snapshot_root_sha256.as_str(),
            self.fixture_exclusion_root_sha256.as_str(),
            self.catalog_candidates,
            self.completed_candidates_excluded,
            self.scored_candidates,
            self.capacity_excluded_candidates,
            self.readiness_rescue_included,
            &self.rows,
            false,
        ))
    }

    pub fn first_readiness_pass(&self) -> Option<&K1NaturalCandidateQueueRowV1> {
        self.rows.iter().find(|row| {
            row.score.readiness_rank == 1
                && (self.schema != K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
                    || row.exact_attempt_state == "unseen")
        })
    }
}

impl K1NaturalCandidateQueueRowV1 {
    pub(super) fn ranks_before(&self, other: &Self, queue_schema: &str) -> bool {
        let order = other
            .score
            .total_k1_gain
            .cmp(&self.score.total_k1_gain)
            .then_with(|| other.score.readiness_rank.cmp(&self.score.readiness_rank));
        let order = if matches!(
            queue_schema,
            K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V2
                | K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3
                | K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V4
        ) {
            let order = if queue_schema == K1_NATURAL_CANDIDATE_QUEUE_SCHEMA_V3 {
                order.then_with(|| {
                    other
                        .terminal_failure_family_novelty_rank
                        .cmp(&self.terminal_failure_family_novelty_rank)
                })
            } else {
                order
            };
            order
                .then_with(|| {
                    self.score
                        .bounded_discovery_cost_units
                        .cmp(&other.score.bounded_discovery_cost_units)
                })
                .then_with(|| {
                    other
                        .score
                        .expected_verified_input_tokens
                        .cmp(&self.score.expected_verified_input_tokens)
                })
        } else {
            order
                .then_with(|| {
                    other
                        .score
                        .expected_verified_input_tokens
                        .cmp(&self.score.expected_verified_input_tokens)
                })
                .then_with(|| {
                    self.score
                        .bounded_discovery_cost_units
                        .cmp(&other.score.bounded_discovery_cost_units)
                })
        };
        order
            .then_with(|| {
                self.score
                    .stable_hash_sha256
                    .cmp(&other.score.stable_hash_sha256)
            })
            .is_lt()
    }
}
