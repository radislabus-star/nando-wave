use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::binding_evidence_adjudication::{
    AcceptedBindingLawEvidenceV2, BindingAdjudicationErrorV1, BindingTrialEvidenceLabelV2,
    PhysicalTrialOutcomeV2, TrustedResolvedBindingRowV2,
};
use crate::{CanonicalEffectLawV3, canonical_json_sha256};

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
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
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
    pub protocol_facet_root_sha256: String,
    pub effect_invariant_root_sha256: String,
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
        pretty_json_bytes(self)
    }
}

pub fn compile_protocol_modes_for_effect_law_v3(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law: &CanonicalEffectLawV3,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    let effect_law_id = effect_law
        .effect_law_id()
        .map_err(|_| BindingProtocolCompilerErrorV2::InvalidDigest)?;
    let candidates = generate_protocol_mode_candidates_for_effect_law_v3(
        evidence,
        effect_law,
        effect_law_id.as_str(),
    )?;
    compile_protocol_modes_internal_v2(
        evidence,
        effect_law_id.as_str(),
        Some(effect_law.action_equivalence_root_sha256()),
        candidates,
        budget,
    )
}

#[cfg(test)]
pub(crate) fn compile_protocol_modes_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law_id_sha256: &str,
    candidates: Vec<BoundedProtocolModeCandidateV2>,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    compile_protocol_modes_internal_v2(evidence, effect_law_id_sha256, None, candidates, budget)
}

fn compile_protocol_modes_internal_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law_id_sha256: &str,
    expected_action_equivalence_root_sha256: Option<&str>,
    candidates: Vec<BoundedProtocolModeCandidateV2>,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    validate_budget_v2(budget)?;
    if !is_sha256(effect_law_id_sha256)
        || expected_action_equivalence_root_sha256.is_some_and(|root| !is_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidDigest);
    }
    let mut search_exhausted = candidates.len() > budget.max_candidates
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
    let mut wrong_action_rows = BTreeSet::new();
    let mut verify_failed_rows = BTreeSet::new();
    let mut negative_accept_rows = BTreeSet::new();
    let mut eligible = Vec::new();
    for candidate in candidates {
        validate_candidate_v2(&candidate)?;
        if candidate.effect_law_id_sha256 != effect_law_id_sha256
            || candidate.relation_identity_sha256 != evidence.relation_identity_sha256()
        {
            continue;
        }
        let derived = derive_candidate_matrix_v2(
            &candidate,
            evidence.rows(),
            expected_action_equivalence_root_sha256,
        )?;
        wrong_action_rows.extend(derived.wrong_action_rows_sha256.iter().cloned());
        verify_failed_rows.extend(derived.verify_failed_rows_sha256.iter().cloned());
        negative_accept_rows.extend(derived.accepts_negative_rows_sha256.iter().cloned());
        if derived.is_eligible() {
            eligible.push(protocol_mode_from_candidate_v2(
                &candidate,
                derived.covered_positive_rows_sha256,
            )?);
        }
    }
    eligible.sort_by(|left, right| left.mode_id_sha256.cmp(&right.mode_id_sha256));
    let (complete_covers, exact_cover_exhausted) =
        exact_cover_protocol_modes_v2(&positive_rows, &eligible, budget.max_surviving_modes);
    search_exhausted |= exact_cover_exhausted;
    let action_classes = complete_covers
        .iter()
        .flat_map(|cover| {
            cover
                .iter()
                .map(|index| eligible[*index].action_class_root_sha256.clone())
        })
        .collect::<BTreeSet<_>>();
    let action_equivalence_classes = action_classes.len();
    let all_complete_covers_action_equivalent =
        !complete_covers.is_empty() && action_equivalence_classes == 1;
    let positive_rows_covered = if complete_covers.is_empty() {
        eligible
            .iter()
            .flat_map(|mode| mode.covered_positive_rows_sha256.iter().cloned())
            .collect::<BTreeSet<_>>()
            .len()
    } else {
        complete_covers
            .iter()
            .flat_map(|cover| {
                cover.iter().flat_map(|index| {
                    eligible[*index]
                        .covered_positive_rows_sha256
                        .iter()
                        .cloned()
                })
            })
            .collect::<BTreeSet<_>>()
            .len()
    };
    let safe_unique = !search_exhausted
        && wrong_action_rows.is_empty()
        && verify_failed_rows.is_empty()
        && negative_accept_rows.is_empty()
        && all_complete_covers_action_equivalent
        && positive_rows_covered == positive_rows.len();
    let verdict = if safe_unique {
        BindingProtocolCompileVerdictV2::ProtocolModeSet
    } else {
        BindingProtocolCompileVerdictV2::Abstain
    };
    let mut selected_modes = if safe_unique {
        complete_covers
            .first()
            .into_iter()
            .flat_map(|cover| cover.iter().map(|index| eligible[*index].clone()))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    selected_modes.sort_by(|left, right| left.mode_id_sha256.cmp(&right.mode_id_sha256));
    let selected_positive_rows_covered = selected_modes
        .iter()
        .flat_map(|mode| mode.covered_positive_rows_sha256.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();
    let mut set = ProtocolModeSetV2 {
        schema: PROTOCOL_MODE_SET_SCHEMA_V2.to_owned(),
        mode_set_sha256: String::new(),
        verdict,
        binding_capability_root_sha256: evidence.capability_root_sha256().to_owned(),
        effect_law_id_sha256: effect_law_id_sha256.to_owned(),
        relation_identity_sha256: evidence.relation_identity_sha256().to_owned(),
        modes: selected_modes,
        positive_rows: positive_rows.len(),
        positive_rows_covered: if safe_unique {
            selected_positive_rows_covered
        } else {
            positive_rows_covered
        },
        wrong_actions: wrong_action_rows.len(),
        verify_failed: verify_failed_rows.len(),
        negative_accepts: negative_accept_rows.len(),
        search_exhausted,
        action_equivalence_classes,
        all_surviving_covers_action_equivalent: all_complete_covers_action_equivalent,
        production_admissible: safe_unique && evidence.production_admissible(),
        execution_authority: false,
    };
    set.mode_set_sha256 = protocol_mode_set_digest_v2(&set)?;
    Ok(set)
}

fn generate_protocol_mode_candidates_for_effect_law_v3(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law: &CanonicalEffectLawV3,
    effect_law_id_sha256: &str,
) -> Result<Vec<BoundedProtocolModeCandidateV2>, BindingProtocolCompilerErrorV2> {
    let mut rows_by_mode = BTreeMap::<(String, String), Vec<&TrustedResolvedBindingRowV2>>::new();
    for row in evidence.rows().iter().filter(|row| {
        row.relation_identity_sha256 == evidence.relation_identity_sha256()
            && row.effect_invariant_root_sha256 == effect_law.effect_invariant_root_sha256()
            && row.evidence_label == BindingTrialEvidenceLabelV2::Positive
            && row.trial_outcome == PhysicalTrialOutcomeV2::Pass
    }) {
        rows_by_mode
            .entry((
                row.protocol_facet_root_sha256.clone(),
                row.effect_invariant_root_sha256.clone(),
            ))
            .or_default()
            .push(row);
    }

    rows_by_mode
        .into_iter()
        .map(
            |((protocol_facet_root_sha256, effect_invariant_root_sha256), mut rows)| {
                rows.sort_by(|left, right| {
                    left.frozen_row_root_sha256
                        .cmp(&right.frozen_row_root_sha256)
                });
                let frozen_graph_roots = row_roots_v2(&rows, |row| &row.frozen_graph_root_sha256);
                let capture_roots = row_roots_v2(&rows, |row| &row.capture_root_sha256);
                let surface_roots = row_roots_v2(&rows, |row| &row.surface_root_sha256);
                let physical_program_ids =
                    row_roots_v2(&rows, |row| &row.physical_program_id_sha256);
                let actor_programs = row_roots_v2(&rows, |row| &row.actor_program_digest_sha256);
                let verifier_programs =
                    row_roots_v2(&rows, |row| &row.verifier_program_digest_sha256);
                let action_roots = row_roots_v2(&rows, |row| &row.candidate_action_digest_sha256);
                let delta_roots = row_roots_v2(&rows, |row| &row.observed_delta_root_sha256);
                Ok(BoundedProtocolModeCandidateV2 {
                    candidate_id_sha256: derived_mode_root_v2(
                        "candidate",
                        &(
                            effect_law_id_sha256,
                            evidence.relation_identity_sha256(),
                            protocol_facet_root_sha256.as_str(),
                            effect_invariant_root_sha256.as_str(),
                        ),
                    )?,
                    effect_law_id_sha256: effect_law_id_sha256.to_owned(),
                    relation_identity_sha256: evidence.relation_identity_sha256().to_owned(),
                    protocol_facet_root_sha256: protocol_facet_root_sha256.clone(),
                    effect_invariant_root_sha256: effect_invariant_root_sha256.clone(),
                    source_role_schema_root_sha256: derived_mode_root_v2(
                        "source-role-schema",
                        &(protocol_facet_root_sha256.as_str(), &frozen_graph_roots),
                    )?,
                    selector_program_root_sha256: derived_mode_root_v2(
                        "selector-program",
                        &(
                            evidence.relation_identity_sha256(),
                            protocol_facet_root_sha256.as_str(),
                            &surface_roots,
                        ),
                    )?,
                    observed_emitted_types_root_sha256: derived_mode_root_v2(
                        "observed-emitted-types",
                        &(effect_invariant_root_sha256.as_str(), &delta_roots),
                    )?,
                    capability_protocol_root_sha256: derived_mode_root_v2(
                        "capability-protocol",
                        &(&physical_program_ids, &actor_programs, &verifier_programs),
                    )?,
                    argument_role_schema_root_sha256: derived_mode_root_v2(
                        "argument-role-schema",
                        &(
                            evidence.relation_identity_sha256(),
                            protocol_facet_root_sha256.as_str(),
                            &action_roots,
                        ),
                    )?,
                    constant_contract_root_sha256: derived_mode_root_v2(
                        "constant-contract",
                        &(effect_invariant_root_sha256.as_str(), &delta_roots),
                    )?,
                    structural_guard_root_sha256: derived_mode_root_v2(
                        "structural-guard",
                        &(protocol_facet_root_sha256.as_str(), &frozen_graph_roots),
                    )?,
                    temporal_cardinality_contract_root_sha256: derived_mode_root_v2(
                        "temporal-cardinality",
                        &(protocol_facet_root_sha256.as_str(), &capture_roots),
                    )?,
                    action_class_root_sha256: effect_law
                        .action_equivalence_root_sha256()
                        .to_owned(),
                    covers_positive_rows_sha256: Vec::new(),
                    accepts_negative_rows_sha256: Vec::new(),
                    wrong_action_rows_sha256: Vec::new(),
                    verify_failed_rows_sha256: Vec::new(),
                    search_exhausted: false,
                })
            },
        )
        .collect()
}

fn row_roots_v2(
    rows: &[&TrustedResolvedBindingRowV2],
    read: fn(&TrustedResolvedBindingRowV2) -> &String,
) -> Vec<String> {
    rows.iter()
        .map(|row| read(row))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn derived_mode_root_v2<T: Serialize>(
    label: &str,
    material: &T,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    sha256_json(&(PROTOCOL_MODE_SET_SCHEMA_V2, label, material))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DerivedProtocolModeMatrixV2 {
    covered_positive_rows_sha256: BTreeSet<String>,
    accepts_negative_rows_sha256: BTreeSet<String>,
    wrong_action_rows_sha256: BTreeSet<String>,
    verify_failed_rows_sha256: BTreeSet<String>,
}

impl DerivedProtocolModeMatrixV2 {
    fn is_eligible(&self) -> bool {
        !self.covered_positive_rows_sha256.is_empty()
            && self.accepts_negative_rows_sha256.is_empty()
            && self.wrong_action_rows_sha256.is_empty()
            && self.verify_failed_rows_sha256.is_empty()
    }
}

fn derive_candidate_matrix_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    rows: &[TrustedResolvedBindingRowV2],
    expected_action_equivalence_root_sha256: Option<&str>,
) -> Result<DerivedProtocolModeMatrixV2, BindingProtocolCompilerErrorV2> {
    let mut covered_positive_rows_sha256 = BTreeSet::new();
    let mut accepts_negative_rows_sha256 = BTreeSet::new();
    let mut wrong_action_rows_sha256 = BTreeSet::new();
    let mut verify_failed_rows_sha256 = BTreeSet::new();
    for row in rows
        .iter()
        .filter(|row| protocol_mode_candidate_matches_row_v2(candidate, row))
    {
        match row.evidence_label {
            BindingTrialEvidenceLabelV2::Positive => match row.trial_outcome {
                PhysicalTrialOutcomeV2::Pass => {
                    if expected_action_equivalence_root_sha256
                        .is_some_and(|root| root != candidate.action_class_root_sha256)
                    {
                        wrong_action_rows_sha256.insert(row.frozen_row_root_sha256.clone());
                    } else {
                        covered_positive_rows_sha256.insert(row.frozen_row_root_sha256.clone());
                    }
                }
                PhysicalTrialOutcomeV2::Fail => {
                    verify_failed_rows_sha256.insert(row.frozen_row_root_sha256.clone());
                }
                PhysicalTrialOutcomeV2::Abstain => {
                    wrong_action_rows_sha256.insert(row.frozen_row_root_sha256.clone());
                }
                PhysicalTrialOutcomeV2::Censored => {}
            },
            BindingTrialEvidenceLabelV2::ApplicabilityNegative => {
                accepts_negative_rows_sha256.insert(row.frozen_row_root_sha256.clone());
            }
        }
    }
    Ok(DerivedProtocolModeMatrixV2 {
        covered_positive_rows_sha256,
        accepts_negative_rows_sha256,
        wrong_action_rows_sha256,
        verify_failed_rows_sha256,
    })
}

fn protocol_mode_candidate_matches_row_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    row: &TrustedResolvedBindingRowV2,
) -> bool {
    candidate.relation_identity_sha256 == row.relation_identity_sha256
        && candidate.protocol_facet_root_sha256 == row.protocol_facet_root_sha256
        && candidate.effect_invariant_root_sha256 == row.effect_invariant_root_sha256
}

fn exact_cover_protocol_modes_v2(
    positive_rows: &BTreeSet<String>,
    modes: &[ProtocolModeV2],
    max_complete_covers: usize,
) -> (Vec<Vec<usize>>, bool) {
    if positive_rows.is_empty() {
        return (Vec::new(), false);
    }
    let mut covers = Vec::new();
    let mut current = Vec::new();
    let mut covered = BTreeSet::new();
    let mut states = 0_usize;
    let exhausted = exact_cover_dfs_v2(
        positive_rows,
        modes,
        max_complete_covers,
        &mut covered,
        &mut current,
        &mut covers,
        &mut states,
    );
    covers.sort();
    (covers, exhausted)
}

fn exact_cover_dfs_v2(
    positive_rows: &BTreeSet<String>,
    modes: &[ProtocolModeV2],
    max_complete_covers: usize,
    covered: &mut BTreeSet<String>,
    current: &mut Vec<usize>,
    covers: &mut Vec<Vec<usize>>,
    states: &mut usize,
) -> bool {
    const MAX_EXACT_COVER_STATES_V2: usize = 65_536;
    *states += 1;
    if *states > MAX_EXACT_COVER_STATES_V2 || covers.len() > max_complete_covers {
        return true;
    }
    if covered == positive_rows {
        covers.push(current.clone());
        return covers.len() > max_complete_covers;
    }
    let Some(next_row) = positive_rows.iter().find(|row| !covered.contains(*row)) else {
        return false;
    };
    for index in 0..modes.len() {
        if current.contains(&index) {
            continue;
        }
        let mode_rows = modes[index]
            .covered_positive_rows_sha256
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if !mode_rows.contains(next_row) || !covered.is_disjoint(&mode_rows) {
            continue;
        }
        covered.extend(mode_rows.iter().cloned());
        current.push(index);
        if exact_cover_dfs_v2(
            positive_rows,
            modes,
            max_complete_covers,
            covered,
            current,
            covers,
            states,
        ) {
            return true;
        }
        current.pop();
        for row in mode_rows {
            covered.remove(&row);
        }
    }
    false
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
        candidate.protocol_facet_root_sha256.as_str(),
        candidate.effect_invariant_root_sha256.as_str(),
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
        protocol_facet_root_sha256: candidate.protocol_facet_root_sha256.clone(),
        effect_invariant_root_sha256: candidate.effect_invariant_root_sha256.clone(),
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
        mode.protocol_facet_root_sha256.as_str(),
        mode.effect_invariant_root_sha256.as_str(),
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
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, BindingProtocolCompilerErrorV2> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| BindingProtocolCompilerErrorV2::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingProtocolCompilerErrorV2> {
    canonical_json_sha256(value).map_err(|_| BindingProtocolCompilerErrorV2::Serialization)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
