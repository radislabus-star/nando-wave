use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::cohort::{K1NaturalCohortCandidateV1, K1NaturalCohortCatalogV1};
use super::evidence::K1ConsequenceTypeV1;
use super::queue::{K1CandidateScoreV1, K1DeficitSnapshotV1, K1NaturalCandidateQueueV1};
use super::{
    K1_IDENTIFICATION_FREEZE_SCHEMA_V1, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1,
    K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2, K1_VERSION_SPACE_MAX_CLASSES_V1, canonical_root_slice,
    canonical_roots, version_space_root,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1GenerationBudgetV1 {
    pub maximum_support_rows: u64,
    pub maximum_probe_rounds: u64,
    pub maximum_probe_cost_units: u64,
    pub maximum_generation_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1NaturalCandidateFreezeV1 {
    pub schema: String,
    pub freeze_root_sha256: String,
    pub generation_sequence: u64,
    pub catalog_root_sha256: String,
    pub k1_deficit_snapshot_root_sha256: String,
    pub epistemic_registry_revision: u64,
    pub epistemic_registry_root_sha256: String,
    pub fixture_exclusion_root_sha256: String,
    pub candidate_root_sha256: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub capture_generation_root_sha256: String,
    pub candidate_structural_root_sha256: String,
    pub source_neutral_topology_root_sha256: String,
    pub semantic_novelty_signature_root_sha256: String,
    pub consequence_type: K1ConsequenceTypeV1,
    pub evidence_manifest_root_sha256: String,
    pub generator_schema: String,
    pub readiness_receipt_root_sha256: String,
    pub scoring_tuple: K1CandidateScoreV1,
    pub scheduler_schema: String,
    pub budget: K1GenerationBudgetV1,
    pub support_watermark: u64,
    pub contract_watermark: u64,
    pub future_min_sequence: u64,
    pub selected_at_unix: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1IdentificationFreezeV1 {
    pub schema: String,
    pub freeze_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub support_manifest_root_sha256: String,
    pub generator_schema: String,
    pub initial_version_space_root_sha256: String,
    pub initial_semantic_class_roots_sha256: Vec<String>,
    pub semantic_quotient_root_sha256: String,
    pub probe_policy_root_sha256: String,
    pub prediction_schema: String,
    pub budget: K1GenerationBudgetV1,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct CandidateFreezeDigestV1<'a> {
    schema: &'static str,
    generation_sequence: u64,
    catalog_root_sha256: &'a str,
    k1_deficit_snapshot_root_sha256: &'a str,
    epistemic_registry_revision: u64,
    epistemic_registry_root_sha256: &'a str,
    fixture_exclusion_root_sha256: &'a str,
    candidate_root_sha256: &'a str,
    candidate_structural_root_sha256: &'a str,
    source_neutral_topology_root_sha256: &'a str,
    semantic_novelty_signature_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    evidence_manifest_root_sha256: &'a str,
    generator_schema: &'a str,
    readiness_receipt_root_sha256: &'a str,
    scoring_tuple: &'a K1CandidateScoreV1,
    scheduler_schema: &'a str,
    budget: K1GenerationBudgetV1,
    support_watermark: u64,
    contract_watermark: u64,
    future_min_sequence: u64,
    selected_at_unix: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

#[derive(Serialize)]
struct CandidateFreezeDigestV2<'a> {
    schema: &'static str,
    generation_sequence: u64,
    catalog_root_sha256: &'a str,
    k1_deficit_snapshot_root_sha256: &'a str,
    epistemic_registry_revision: u64,
    epistemic_registry_root_sha256: &'a str,
    fixture_exclusion_root_sha256: &'a str,
    candidate_root_sha256: &'a str,
    capture_generation_root_sha256: &'a str,
    candidate_structural_root_sha256: &'a str,
    source_neutral_topology_root_sha256: &'a str,
    semantic_novelty_signature_root_sha256: &'a str,
    consequence_type: K1ConsequenceTypeV1,
    evidence_manifest_root_sha256: &'a str,
    generator_schema: &'a str,
    readiness_receipt_root_sha256: &'a str,
    scoring_tuple: &'a K1CandidateScoreV1,
    scheduler_schema: &'a str,
    budget: K1GenerationBudgetV1,
    support_watermark: u64,
    contract_watermark: u64,
    future_min_sequence: u64,
    selected_at_unix: u64,
    authority_ready: bool,
    phase_mutation_allowed: bool,
}

impl K1GenerationBudgetV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.maximum_support_rows == 0
            || self.maximum_probe_rounds == 0
            || self.maximum_probe_cost_units == 0
            || self.maximum_generation_seconds == 0
        {
            return Err("k1_generation_budget_invalid");
        }
        Ok(())
    }
}

impl K1NaturalCandidateFreezeV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        generation_sequence: u64,
        catalog: &K1NaturalCohortCatalogV1,
        deficit: &K1DeficitSnapshotV1,
        queue: &K1NaturalCandidateQueueV1,
        candidate: &K1NaturalCohortCandidateV1,
        scoring_tuple: K1CandidateScoreV1,
        scheduler_schema: String,
        budget: K1GenerationBudgetV1,
        support_watermark: u64,
        contract_watermark: u64,
        selected_at_unix: u64,
    ) -> Result<Self, &'static str> {
        catalog.validate()?;
        deficit.validate()?;
        queue.validate()?;
        candidate.validate()?;
        scoring_tuple.validate()?;
        budget.validate()?;
        let queued = queue
            .rows
            .iter()
            .find(|row| row.candidate_root_sha256 == candidate.candidate_root_sha256)
            .ok_or("k1_freeze_candidate_not_queued")?;
        let freeze_ready = candidate.readiness.freeze_ready_at(
            candidate.evidence_rows,
            candidate.first_capture_sequence,
            candidate.last_capture_sequence,
            contract_watermark,
        )?;
        if deficit.k1_open
            || !freeze_ready
            || queue.first_readiness_pass() != Some(queued)
            || queued.readiness_receipt_root_sha256
                != candidate.readiness.readiness_receipt_root_sha256
            || queued.score != scoring_tuple
            || catalog.catalog_root_sha256 != queue.catalog_root_sha256
            || deficit.snapshot_root_sha256 != queue.k1_deficit_snapshot_root_sha256
            || catalog.fixture_exclusion_root_sha256 != queue.fixture_exclusion_root_sha256
            || support_watermark > contract_watermark
        {
            return Err("k1_candidate_freeze_binding_invalid");
        }
        let mut freeze = Self {
            schema: K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2.to_owned(),
            freeze_root_sha256: String::new(),
            generation_sequence,
            catalog_root_sha256: catalog.catalog_root_sha256.clone(),
            k1_deficit_snapshot_root_sha256: deficit.snapshot_root_sha256.clone(),
            epistemic_registry_revision: deficit.epistemic_registry_revision,
            epistemic_registry_root_sha256: deficit.epistemic_registry_root_sha256.clone(),
            fixture_exclusion_root_sha256: catalog.fixture_exclusion_root_sha256.clone(),
            candidate_root_sha256: candidate.candidate_root_sha256.clone(),
            capture_generation_root_sha256: candidate.capture_generation_root_sha256.clone(),
            candidate_structural_root_sha256: candidate.candidate_structural_root_sha256.clone(),
            source_neutral_topology_root_sha256: candidate
                .source_neutral_topology_root_sha256
                .clone(),
            semantic_novelty_signature_root_sha256: candidate
                .semantic_novelty_signature_root_sha256
                .clone(),
            consequence_type: candidate.consequence_type,
            evidence_manifest_root_sha256: candidate.evidence_manifest_root_sha256.clone(),
            generator_schema: candidate.generator_schema.clone(),
            readiness_receipt_root_sha256: candidate
                .readiness
                .readiness_receipt_root_sha256
                .clone(),
            scoring_tuple,
            scheduler_schema,
            budget,
            support_watermark,
            contract_watermark,
            future_min_sequence: contract_watermark.saturating_add(1),
            selected_at_unix,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        freeze.freeze_root_sha256 = freeze.expected_root()?;
        freeze.validate()?;
        Ok(freeze)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.scoring_tuple.validate()?;
        self.budget.validate()?;
        let roots = [
            self.freeze_root_sha256.as_str(),
            self.catalog_root_sha256.as_str(),
            self.k1_deficit_snapshot_root_sha256.as_str(),
            self.epistemic_registry_root_sha256.as_str(),
            self.fixture_exclusion_root_sha256.as_str(),
            self.candidate_root_sha256.as_str(),
            self.candidate_structural_root_sha256.as_str(),
            self.source_neutral_topology_root_sha256.as_str(),
            self.semantic_novelty_signature_root_sha256.as_str(),
            self.evidence_manifest_root_sha256.as_str(),
            self.readiness_receipt_root_sha256.as_str(),
        ];
        let capture_generation_valid = if self.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1 {
            self.capture_generation_root_sha256.is_empty()
        } else {
            self.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2
                && valid_nonzero_sha256(&self.capture_generation_root_sha256)
        };
        if !capture_generation_valid
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.generation_sequence == 0
            || self.scheduler_schema.is_empty()
            || self.generator_schema.is_empty()
            || self.support_watermark > self.contract_watermark
            || self.future_min_sequence != self.contract_watermark.saturating_add(1)
            || self.selected_at_unix == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.freeze_root_sha256 != self.expected_root()?
        {
            return Err("k1_natural_candidate_freeze_invalid");
        }
        Ok(())
    }

    pub(crate) fn expected_root(&self) -> Result<String, &'static str> {
        if self.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2 {
            return canonical_json_sha256(&CandidateFreezeDigestV2 {
                schema: K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V2,
                generation_sequence: self.generation_sequence,
                catalog_root_sha256: &self.catalog_root_sha256,
                k1_deficit_snapshot_root_sha256: &self.k1_deficit_snapshot_root_sha256,
                epistemic_registry_revision: self.epistemic_registry_revision,
                epistemic_registry_root_sha256: &self.epistemic_registry_root_sha256,
                fixture_exclusion_root_sha256: &self.fixture_exclusion_root_sha256,
                candidate_root_sha256: &self.candidate_root_sha256,
                capture_generation_root_sha256: &self.capture_generation_root_sha256,
                candidate_structural_root_sha256: &self.candidate_structural_root_sha256,
                source_neutral_topology_root_sha256: &self.source_neutral_topology_root_sha256,
                semantic_novelty_signature_root_sha256: &self
                    .semantic_novelty_signature_root_sha256,
                consequence_type: self.consequence_type,
                evidence_manifest_root_sha256: &self.evidence_manifest_root_sha256,
                generator_schema: &self.generator_schema,
                readiness_receipt_root_sha256: &self.readiness_receipt_root_sha256,
                scoring_tuple: &self.scoring_tuple,
                scheduler_schema: &self.scheduler_schema,
                budget: self.budget,
                support_watermark: self.support_watermark,
                contract_watermark: self.contract_watermark,
                future_min_sequence: self.future_min_sequence,
                selected_at_unix: self.selected_at_unix,
                authority_ready: false,
                phase_mutation_allowed: false,
            });
        }
        canonical_json_sha256(&CandidateFreezeDigestV1 {
            schema: K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1,
            generation_sequence: self.generation_sequence,
            catalog_root_sha256: &self.catalog_root_sha256,
            k1_deficit_snapshot_root_sha256: &self.k1_deficit_snapshot_root_sha256,
            epistemic_registry_revision: self.epistemic_registry_revision,
            epistemic_registry_root_sha256: &self.epistemic_registry_root_sha256,
            fixture_exclusion_root_sha256: &self.fixture_exclusion_root_sha256,
            candidate_root_sha256: &self.candidate_root_sha256,
            candidate_structural_root_sha256: &self.candidate_structural_root_sha256,
            source_neutral_topology_root_sha256: &self.source_neutral_topology_root_sha256,
            semantic_novelty_signature_root_sha256: &self.semantic_novelty_signature_root_sha256,
            consequence_type: self.consequence_type,
            evidence_manifest_root_sha256: &self.evidence_manifest_root_sha256,
            generator_schema: &self.generator_schema,
            readiness_receipt_root_sha256: &self.readiness_receipt_root_sha256,
            scoring_tuple: &self.scoring_tuple,
            scheduler_schema: &self.scheduler_schema,
            budget: self.budget,
            support_watermark: self.support_watermark,
            contract_watermark: self.contract_watermark,
            future_min_sequence: self.future_min_sequence,
            selected_at_unix: self.selected_at_unix,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
    }
}

impl K1IdentificationFreezeV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        candidate_freeze: &K1NaturalCandidateFreezeV1,
        support_manifest_root_sha256: String,
        generator_schema: String,
        mut initial_semantic_class_roots_sha256: Vec<String>,
        semantic_quotient_root_sha256: String,
        probe_policy_root_sha256: String,
        prediction_schema: String,
    ) -> Result<Self, &'static str> {
        candidate_freeze.validate()?;
        canonical_roots(&mut initial_semantic_class_roots_sha256)?;
        if initial_semantic_class_roots_sha256.is_empty()
            || initial_semantic_class_roots_sha256.len() > K1_VERSION_SPACE_MAX_CLASSES_V1
        {
            return Err("k1_initial_version_space_size_invalid");
        }
        let initial_version_space_root_sha256 =
            version_space_root(&initial_semantic_class_roots_sha256)?;
        let mut freeze = Self {
            schema: K1_IDENTIFICATION_FREEZE_SCHEMA_V1.to_owned(),
            freeze_root_sha256: String::new(),
            candidate_freeze_root_sha256: candidate_freeze.freeze_root_sha256.clone(),
            support_manifest_root_sha256,
            generator_schema,
            initial_version_space_root_sha256,
            initial_semantic_class_roots_sha256,
            semantic_quotient_root_sha256,
            probe_policy_root_sha256,
            prediction_schema,
            budget: candidate_freeze.budget,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        freeze.freeze_root_sha256 = freeze.expected_root()?;
        freeze.validate()?;
        Ok(freeze)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.budget.validate()?;
        let roots = [
            self.freeze_root_sha256.as_str(),
            self.candidate_freeze_root_sha256.as_str(),
            self.support_manifest_root_sha256.as_str(),
            self.initial_version_space_root_sha256.as_str(),
            self.semantic_quotient_root_sha256.as_str(),
            self.probe_policy_root_sha256.as_str(),
        ];
        if self.schema != K1_IDENTIFICATION_FREEZE_SCHEMA_V1
            || !roots.into_iter().all(valid_nonzero_sha256)
            || self.generator_schema.is_empty()
            || self.prediction_schema.is_empty()
            || self.initial_semantic_class_roots_sha256.is_empty()
            || self.initial_semantic_class_roots_sha256.len() > K1_VERSION_SPACE_MAX_CLASSES_V1
            || !canonical_root_slice(&self.initial_semantic_class_roots_sha256)
            || self.initial_version_space_root_sha256
                != version_space_root(&self.initial_semantic_class_roots_sha256)?
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.freeze_root_sha256 != self.expected_root()?
        {
            return Err("k1_identification_freeze_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_IDENTIFICATION_FREEZE_SCHEMA_V1,
            self.candidate_freeze_root_sha256.as_str(),
            self.support_manifest_root_sha256.as_str(),
            self.generator_schema.as_str(),
            self.initial_version_space_root_sha256.as_str(),
            &self.initial_semantic_class_roots_sha256,
            self.semantic_quotient_root_sha256.as_str(),
            self.probe_policy_root_sha256.as_str(),
            self.prediction_schema.as_str(),
            self.budget,
            false,
            false,
        ))
    }
}
