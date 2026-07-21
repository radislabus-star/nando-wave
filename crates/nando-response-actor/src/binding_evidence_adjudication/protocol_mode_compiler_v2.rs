use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::binding_law_evidence_v2::AcceptedBindingLawEvidenceV2;
use super::canonical::{is_sha256, pretty_json_bytes, sha256_json};
use super::trusted_resolver_v2::BindingTrialEvidenceLabelV2;
use super::wire::BindingAdjudicationErrorV1;

pub const PROTOCOL_MODE_SET_SCHEMA_V2: &str = "nando.protocol-mode-set.v2";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingProtocolCompileVerdictV2 {
    ProtocolModeSet,
    Abstain,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolModeCompilerBudgetV2 {
    pub max_candidates: usize,
    pub max_surviving_modes: usize,
}

impl Default for ProtocolModeCompilerBudgetV2 {
    fn default() -> Self {
        Self {
            max_candidates: 128,
            max_surviving_modes: 32,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedProtocolModeCandidateV2 {
    pub candidate_id_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub source_role_schema_root_sha256: String,
    pub selector_program_root_sha256: String,
    pub observed_emitted_types_root_sha256: String,
    pub capability_protocol_root_sha256: String,
    pub argument_role_schema_root_sha256: String,
    pub constant_contract_root_sha256: String,
    pub structural_guard_root_sha256: String,
    pub temporal_cardinality_contract_root_sha256: String,
    pub action_class_root_sha256: String,
    pub covers_positive_rows_sha256: Vec<String>,
    pub accepts_negative_rows_sha256: Vec<String>,
    pub wrong_action_rows_sha256: Vec<String>,
    pub verify_failed_rows_sha256: Vec<String>,
    pub search_exhausted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeV2 {
    pub mode_id_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub source_role_schema_root_sha256: String,
    pub selector_program_root_sha256: String,
    pub observed_emitted_types_root_sha256: String,
    pub capability_protocol_root_sha256: String,
    pub argument_role_schema_root_sha256: String,
    pub constant_contract_root_sha256: String,
    pub structural_guard_root_sha256: String,
    pub temporal_cardinality_contract_root_sha256: String,
    pub action_class_root_sha256: String,
    pub covered_positive_rows_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolModeSetV2 {
    pub schema: String,
    pub mode_set_sha256: String,
    pub verdict: BindingProtocolCompileVerdictV2,
    pub binding_capability_root_sha256: String,
    pub effect_law_id_sha256: String,
    pub relation_identity_sha256: String,
    pub modes: Vec<ProtocolModeV2>,
    pub positive_rows: usize,
    pub positive_rows_covered: usize,
    pub wrong_actions: usize,
    pub verify_failed: usize,
    pub negative_accepts: usize,
    pub search_exhausted: bool,
    pub action_equivalence_classes: usize,
    pub all_surviving_covers_action_equivalent: bool,
    pub production_admissible: bool,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingProtocolCompilerErrorV2 {
    InvalidDigest,
    InvalidBudget,
    InvalidCandidate,
    Serialization,
}

impl From<BindingAdjudicationErrorV1> for BindingProtocolCompilerErrorV2 {
    fn from(value: BindingAdjudicationErrorV1) -> Self {
        match value {
            BindingAdjudicationErrorV1::Serialization => Self::Serialization,
            BindingAdjudicationErrorV1::InvalidDigest => Self::InvalidDigest,
            _ => Self::InvalidCandidate,
        }
    }
}

impl ProtocolModeSetV2 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BindingProtocolCompilerErrorV2> {
        pretty_json_bytes(self).map_err(BindingProtocolCompilerErrorV2::from)
    }
}

pub fn compile_protocol_modes_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law_id_sha256: &str,
    candidates: Vec<BoundedProtocolModeCandidateV2>,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    validate_budget_v2(budget)?;
    if !is_sha256(effect_law_id_sha256) || candidates.is_empty() {
        return Err(BindingProtocolCompilerErrorV2::InvalidDigest);
    }
    let search_exhausted = candidates.len() > budget.max_candidates
        || candidates
            .iter()
            .any(|candidate| candidate.search_exhausted);
    let candidates = candidates
        .into_iter()
        .take(budget.max_candidates)
        .collect::<Vec<_>>();
    let positive_rows = evidence
        .rows()
        .iter()
        .filter(|row| row.evidence_label == BindingTrialEvidenceLabelV2::Positive)
        .map(|row| row.frozen_row_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut wrong_actions = 0_usize;
    let mut verify_failed = 0_usize;
    let mut negative_accepts = 0_usize;
    let mut surviving = Vec::new();
    for candidate in candidates {
        validate_candidate_v2(&candidate)?;
        if candidate.effect_law_id_sha256 != effect_law_id_sha256
            || candidate.relation_identity_sha256 != evidence.relation_identity_sha256()
        {
            continue;
        }
        wrong_actions += candidate.wrong_action_rows_sha256.len();
        verify_failed += candidate.verify_failed_rows_sha256.len();
        let accepted_negative_rows = candidate.accepts_negative_rows_sha256.len();
        negative_accepts += accepted_negative_rows;
        let covered = candidate
            .covers_positive_rows_sha256
            .iter()
            .filter(|row| positive_rows.contains(*row))
            .cloned()
            .collect::<BTreeSet<_>>();
        if covered == positive_rows
            && candidate.wrong_action_rows_sha256.is_empty()
            && candidate.verify_failed_rows_sha256.is_empty()
            && accepted_negative_rows == 0
        {
            surviving.push(protocol_mode_from_candidate_v2(&candidate, covered)?);
        }
    }
    surviving.sort_by(|left, right| left.mode_id_sha256.cmp(&right.mode_id_sha256));
    let too_many_survivors = surviving.len() > budget.max_surviving_modes;
    surviving.truncate(budget.max_surviving_modes);
    let action_classes = surviving
        .iter()
        .map(|mode| mode.action_class_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let action_equivalence_classes = action_classes.len();
    let positive_rows_covered = surviving
        .iter()
        .flat_map(|mode| mode.covered_positive_rows_sha256.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();
    let safe_unique = !search_exhausted
        && !too_many_survivors
        && wrong_actions == 0
        && verify_failed == 0
        && negative_accepts == 0
        && !surviving.is_empty()
        && action_equivalence_classes == 1
        && positive_rows_covered == positive_rows.len();
    let verdict = if safe_unique {
        BindingProtocolCompileVerdictV2::ProtocolModeSet
    } else {
        BindingProtocolCompileVerdictV2::Abstain
    };
    if verdict == BindingProtocolCompileVerdictV2::Abstain {
        surviving.clear();
    }
    let mut set = ProtocolModeSetV2 {
        schema: PROTOCOL_MODE_SET_SCHEMA_V2.to_owned(),
        mode_set_sha256: String::new(),
        verdict,
        binding_capability_root_sha256: evidence.capability_root_sha256().to_owned(),
        effect_law_id_sha256: effect_law_id_sha256.to_owned(),
        relation_identity_sha256: evidence.relation_identity_sha256().to_owned(),
        modes: surviving,
        positive_rows: positive_rows.len(),
        positive_rows_covered,
        wrong_actions,
        verify_failed,
        negative_accepts,
        search_exhausted: search_exhausted || too_many_survivors,
        action_equivalence_classes,
        all_surviving_covers_action_equivalent: action_equivalence_classes <= 1,
        production_admissible: safe_unique && evidence.production_admissible(),
        execution_authority: false,
    };
    set.mode_set_sha256 = protocol_mode_set_digest_v2(&set)?;
    Ok(set)
}

fn validate_budget_v2(
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    if budget.max_candidates == 0
        || budget.max_candidates > 4096
        || budget.max_surviving_modes == 0
        || budget.max_surviving_modes > 512
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidBudget);
    }
    Ok(())
}

fn validate_candidate_v2(
    candidate: &BoundedProtocolModeCandidateV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    let roots = [
        candidate.candidate_id_sha256.as_str(),
        candidate.effect_law_id_sha256.as_str(),
        candidate.relation_identity_sha256.as_str(),
        candidate.source_role_schema_root_sha256.as_str(),
        candidate.selector_program_root_sha256.as_str(),
        candidate.observed_emitted_types_root_sha256.as_str(),
        candidate.capability_protocol_root_sha256.as_str(),
        candidate.argument_role_schema_root_sha256.as_str(),
        candidate.constant_contract_root_sha256.as_str(),
        candidate.structural_guard_root_sha256.as_str(),
        candidate.temporal_cardinality_contract_root_sha256.as_str(),
        candidate.action_class_root_sha256.as_str(),
    ];
    if roots.into_iter().any(|root| !is_sha256(root))
        || candidate
            .covers_positive_rows_sha256
            .iter()
            .chain(candidate.accepts_negative_rows_sha256.iter())
            .chain(candidate.wrong_action_rows_sha256.iter())
            .chain(candidate.verify_failed_rows_sha256.iter())
            .any(|root| !is_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidDigest);
    }
    Ok(())
}

fn protocol_mode_from_candidate_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    covered: BTreeSet<String>,
) -> Result<ProtocolModeV2, BindingProtocolCompilerErrorV2> {
    let covered_positive_rows_sha256 = covered.into_iter().collect::<Vec<_>>();
    let mut mode = ProtocolModeV2 {
        mode_id_sha256: String::new(),
        effect_law_id_sha256: candidate.effect_law_id_sha256.clone(),
        relation_identity_sha256: candidate.relation_identity_sha256.clone(),
        source_role_schema_root_sha256: candidate.source_role_schema_root_sha256.clone(),
        selector_program_root_sha256: candidate.selector_program_root_sha256.clone(),
        observed_emitted_types_root_sha256: candidate.observed_emitted_types_root_sha256.clone(),
        capability_protocol_root_sha256: candidate.capability_protocol_root_sha256.clone(),
        argument_role_schema_root_sha256: candidate.argument_role_schema_root_sha256.clone(),
        constant_contract_root_sha256: candidate.constant_contract_root_sha256.clone(),
        structural_guard_root_sha256: candidate.structural_guard_root_sha256.clone(),
        temporal_cardinality_contract_root_sha256: candidate
            .temporal_cardinality_contract_root_sha256
            .clone(),
        action_class_root_sha256: candidate.action_class_root_sha256.clone(),
        covered_positive_rows_sha256,
    };
    mode.mode_id_sha256 = protocol_mode_digest_v2(&mode)?;
    Ok(mode)
}

fn protocol_mode_digest_v2(
    mode: &ProtocolModeV2,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    sha256_json(&(
        mode.effect_law_id_sha256.as_str(),
        mode.relation_identity_sha256.as_str(),
        mode.source_role_schema_root_sha256.as_str(),
        mode.selector_program_root_sha256.as_str(),
        mode.observed_emitted_types_root_sha256.as_str(),
        mode.capability_protocol_root_sha256.as_str(),
        mode.argument_role_schema_root_sha256.as_str(),
        mode.constant_contract_root_sha256.as_str(),
        mode.structural_guard_root_sha256.as_str(),
        mode.temporal_cardinality_contract_root_sha256.as_str(),
        mode.action_class_root_sha256.as_str(),
        &mode.covered_positive_rows_sha256,
    ))
    .map_err(BindingProtocolCompilerErrorV2::from)
}

fn protocol_mode_set_digest_v2(
    set: &ProtocolModeSetV2,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    sha256_json(&(
        set.schema.as_str(),
        set.verdict,
        set.binding_capability_root_sha256.as_str(),
        set.effect_law_id_sha256.as_str(),
        set.relation_identity_sha256.as_str(),
        &set.modes,
        set.positive_rows,
        set.positive_rows_covered,
        set.wrong_actions,
        set.verify_failed,
        set.negative_accepts,
        set.search_exhausted,
        set.action_equivalence_classes,
        set.all_surviving_covers_action_equivalent,
        set.production_admissible,
        set.execution_authority,
    ))
    .map_err(BindingProtocolCompilerErrorV2::from)
}
