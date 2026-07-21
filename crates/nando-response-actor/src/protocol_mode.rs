use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

mod selector;

pub use selector::{
    ProtocolArgumentRoleSchemaV2, ProtocolArgumentRoleV2, ProtocolCapabilityContractV2,
    ProtocolConstantContractV2, ProtocolModeProgramV2, ProtocolRoleCardinalityV2,
    ProtocolSelectorProgramV2, ProtocolSourceRoleSchemaV2, ProtocolSourceRoleV2,
    ProtocolStructuralGuardV2, ProtocolTemporalCardinalityContractV2, ProtocolValueContractV2,
};

use crate::binding_evidence_adjudication::{
    AcceptedBindingLawEvidenceV2, BindingAdjudicationErrorV1, BindingTrialEvidenceLabelV2,
};
#[cfg(test)]
use crate::binding_evidence_adjudication::{PhysicalTrialOutcomeV2, TrustedResolvedBindingRowV2};
use crate::{CanonicalEffectLawV3, FrozenCandidateRelationGraphV1, canonical_json_sha256};

pub const PROTOCOL_MODE_SET_SCHEMA_V2: &str = "nando.protocol-mode-set.v2.f4r2";

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
            max_candidates: 512,
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
    pub program: ProtocolModeProgramV2,
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
    pub program: ProtocolModeProgramV2,
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
    InvalidGraphView,
    InvalidModeSet,
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

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, BindingProtocolCompilerErrorV2> {
        let set: Self = serde_json::from_slice(bytes)
            .map_err(|_| BindingProtocolCompilerErrorV2::InvalidModeSet)?;
        if set.canonical_bytes()? != bytes {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        validate_protocol_mode_set_v2(&set)?;
        Ok(set)
    }
}

pub fn compile_protocol_modes_for_effect_law_v3(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law: &CanonicalEffectLawV3,
    graph_views: &[FrozenCandidateRelationGraphV1],
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    let effect_law_id = effect_law
        .effect_law_id()
        .map_err(|_| BindingProtocolCompilerErrorV2::InvalidDigest)?;
    let graphs = selector::index_frozen_graph_views_v2(evidence, graph_views)?;
    let generation = selector::generate_protocol_mode_candidates_v2(
        evidence,
        effect_law,
        effect_law_id.as_str(),
        &graphs,
        budget,
    )?;
    compile_protocol_modes_internal_v2(
        evidence,
        effect_law_id.as_str(),
        Some(effect_law.action_equivalence_root_sha256()),
        generation.candidates,
        budget,
        ProtocolMatrixSourceV2::FrozenGraphs(&graphs),
        generation.search_exhausted,
    )
}

#[cfg(test)]
pub(crate) fn compile_protocol_modes_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law_id_sha256: &str,
    candidates: Vec<BoundedProtocolModeCandidateV2>,
    budget: ProtocolModeCompilerBudgetV2,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    compile_protocol_modes_internal_v2(
        evidence,
        effect_law_id_sha256,
        None,
        candidates,
        budget,
        ProtocolMatrixSourceV2::FacetControl,
        false,
    )
}

enum ProtocolMatrixSourceV2<'a> {
    FrozenGraphs(&'a BTreeMap<String, &'a FrozenCandidateRelationGraphV1>),
    #[cfg(test)]
    FacetControl,
}

fn compile_protocol_modes_internal_v2(
    evidence: &AcceptedBindingLawEvidenceV2,
    effect_law_id_sha256: &str,
    expected_action_equivalence_root_sha256: Option<&str>,
    candidates: Vec<BoundedProtocolModeCandidateV2>,
    budget: ProtocolModeCompilerBudgetV2,
    matrix_source: ProtocolMatrixSourceV2<'_>,
    generation_search_exhausted: bool,
) -> Result<ProtocolModeSetV2, BindingProtocolCompilerErrorV2> {
    validate_budget_v2(budget)?;
    if !is_sha256(effect_law_id_sha256)
        || expected_action_equivalence_root_sha256.is_some_and(|root| !is_sha256(root))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidDigest);
    }
    let mut search_exhausted = generation_search_exhausted
        || candidates.len() > budget.max_candidates
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
    let mut rejected_wrong_action_rows = BTreeSet::new();
    let mut rejected_verify_failed_rows = BTreeSet::new();
    let mut rejected_negative_accept_rows = BTreeSet::new();
    let mut eligible = Vec::new();
    for candidate in candidates {
        validate_candidate_v2(&candidate)?;
        if candidate.effect_law_id_sha256 != effect_law_id_sha256
            || candidate.relation_identity_sha256 != evidence.relation_identity_sha256()
        {
            continue;
        }
        let derived = match &matrix_source {
            ProtocolMatrixSourceV2::FrozenGraphs(graphs) => selector::derive_candidate_matrix_v2(
                &candidate,
                evidence.rows(),
                graphs,
                expected_action_equivalence_root_sha256
                    .ok_or(BindingProtocolCompilerErrorV2::InvalidCandidate)?,
            )?,
            #[cfg(test)]
            ProtocolMatrixSourceV2::FacetControl => derive_facet_control_matrix_v2(
                &candidate,
                evidence.rows(),
                expected_action_equivalence_root_sha256,
            )?,
        };
        if derived.is_eligible() {
            eligible.push(protocol_mode_from_candidate_v2(
                &candidate,
                derived.covered_positive_rows_sha256,
            )?);
        } else {
            // Search may explore unsafe hypotheses; only eligible modes reach exact cover.
            rejected_wrong_action_rows.extend(derived.wrong_action_rows_sha256);
            rejected_verify_failed_rows.extend(derived.verify_failed_rows_sha256);
            rejected_negative_accept_rows.extend(derived.accepts_negative_rows_sha256);
        }
    }
    eligible = prune_dominated_modes_v2(eligible);
    eligible.sort_by(|left, right| {
        left.program
            .selector_program
            .predicates
            .len()
            .cmp(&right.program.selector_program.predicates.len())
            .then_with(|| left.mode_id_sha256.cmp(&right.mode_id_sha256))
    });
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
        wrong_actions: if safe_unique {
            0
        } else {
            rejected_wrong_action_rows.len()
        },
        verify_failed: if safe_unique {
            0
        } else {
            rejected_verify_failed_rows.len()
        },
        negative_accepts: if safe_unique {
            0
        } else {
            rejected_negative_accept_rows.len()
        },
        search_exhausted,
        action_equivalence_classes,
        all_surviving_covers_action_equivalent: all_complete_covers_action_equivalent,
        production_admissible: safe_unique && evidence.production_admissible(),
        execution_authority: false,
    };
    set.mode_set_sha256 = protocol_mode_set_digest_v2(&set)?;
    validate_protocol_mode_set_v2(&set)?;
    Ok(set)
}

pub(super) fn derived_mode_root_v2<T: Serialize>(
    label: &str,
    material: &T,
) -> Result<String, BindingProtocolCompilerErrorV2> {
    sha256_json(&(PROTOCOL_MODE_SET_SCHEMA_V2, label, material))
}

#[cfg(test)]
fn derive_facet_control_matrix_v2(
    candidate: &BoundedProtocolModeCandidateV2,
    rows: &[TrustedResolvedBindingRowV2],
    expected_action_equivalence_root_sha256: Option<&str>,
) -> Result<selector::DerivedProtocolModeMatrixV2, BindingProtocolCompilerErrorV2> {
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
    Ok(selector::DerivedProtocolModeMatrixV2 {
        covered_positive_rows_sha256,
        accepts_negative_rows_sha256,
        wrong_action_rows_sha256,
        verify_failed_rows_sha256,
    })
}

#[cfg(test)]
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
    selector::validate_program_v2(&candidate.program)?;
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
        || candidate.source_role_schema_root_sha256
            != derived_mode_root_v2("source-role-schema", &candidate.program.source_role_schema)?
        || candidate.selector_program_root_sha256
            != derived_mode_root_v2("selector-program", &candidate.program.selector_program)?
        || candidate.observed_emitted_types_root_sha256
            != derived_mode_root_v2("observed-emitted-types", &candidate.program.value_contract)?
        || candidate.capability_protocol_root_sha256
            != derived_mode_root_v2(
                "capability-protocol",
                &candidate.program.capability_contract,
            )?
        || candidate.argument_role_schema_root_sha256
            != derived_mode_root_v2(
                "argument-role-schema",
                &candidate.program.argument_role_schema,
            )?
        || candidate.constant_contract_root_sha256
            != derived_mode_root_v2("constant-contract", &candidate.program.constant_contract)?
        || candidate.structural_guard_root_sha256
            != derived_mode_root_v2("structural-guard", &candidate.program.structural_guard)?
        || candidate.temporal_cardinality_contract_root_sha256
            != derived_mode_root_v2(
                "temporal-cardinality",
                &candidate.program.temporal_cardinality_contract,
            )?
        || candidate.protocol_facet_root_sha256
            != candidate
                .program
                .capability_contract
                .protocol_facet_root_sha256
        || candidate.relation_identity_sha256
            != candidate.program.structural_guard.relation_identity_sha256
        || candidate.effect_invariant_root_sha256
            != candidate
                .program
                .structural_guard
                .effect_invariant_root_sha256
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
        program: candidate.program.clone(),
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
        &mode.program,
        &mode.covered_positive_rows_sha256,
    ))
}

fn prune_dominated_modes_v2(mut modes: Vec<ProtocolModeV2>) -> Vec<ProtocolModeV2> {
    modes.sort_by(|left, right| {
        left.program
            .selector_program
            .predicates
            .len()
            .cmp(&right.program.selector_program.predicates.len())
            .then_with(|| left.mode_id_sha256.cmp(&right.mode_id_sha256))
    });
    let mut retained = Vec::<ProtocolModeV2>::new();
    'candidate: for mode in modes {
        let mode_predicates = mode
            .program
            .selector_program
            .predicates
            .iter()
            .collect::<BTreeSet<_>>();
        for existing in &retained {
            if existing.protocol_facet_root_sha256 == mode.protocol_facet_root_sha256
                && existing.action_class_root_sha256 == mode.action_class_root_sha256
                && existing.covered_positive_rows_sha256 == mode.covered_positive_rows_sha256
            {
                let existing_predicates = existing
                    .program
                    .selector_program
                    .predicates
                    .iter()
                    .collect::<BTreeSet<_>>();
                if existing_predicates.is_subset(&mode_predicates) {
                    continue 'candidate;
                }
            }
        }
        retained.push(mode);
    }
    retained
}

fn validate_protocol_mode_set_v2(
    set: &ProtocolModeSetV2,
) -> Result<(), BindingProtocolCompilerErrorV2> {
    let mode_ids = set
        .modes
        .iter()
        .map(|mode| mode.mode_id_sha256.as_str())
        .collect::<Vec<_>>();
    let mode_ids_are_sorted = mode_ids.windows(2).all(|pair| pair[0] < pair[1]);
    if set.schema != PROTOCOL_MODE_SET_SCHEMA_V2
        || !is_sha256(&set.mode_set_sha256)
        || !is_sha256(&set.binding_capability_root_sha256)
        || !is_sha256(&set.effect_law_id_sha256)
        || !is_sha256(&set.relation_identity_sha256)
        || set.execution_authority
        || !mode_ids_are_sorted
        || set.positive_rows_covered > set.positive_rows
        || set.mode_set_sha256 != protocol_mode_set_digest_v2(set)?
        || (set.verdict == BindingProtocolCompileVerdictV2::Abstain && !set.modes.is_empty())
        || (set.verdict == BindingProtocolCompileVerdictV2::ProtocolModeSet
            && (set.modes.is_empty()
                || set.search_exhausted
                || set.action_equivalence_classes != 1
                || set.wrong_actions != 0
                || set.verify_failed != 0
                || set.negative_accepts != 0
                || set.positive_rows == 0
                || set.positive_rows_covered != set.positive_rows
                || !set.all_surviving_covers_action_equivalent))
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
    }
    let mut mode_ids = BTreeSet::new();
    let mut covered_rows = BTreeSet::new();
    let mut action_classes = BTreeSet::new();
    for mode in &set.modes {
        selector::validate_program_v2(&mode.program)?;
        let candidate = BoundedProtocolModeCandidateV2 {
            candidate_id_sha256: mode.mode_id_sha256.clone(),
            effect_law_id_sha256: mode.effect_law_id_sha256.clone(),
            relation_identity_sha256: mode.relation_identity_sha256.clone(),
            protocol_facet_root_sha256: mode.protocol_facet_root_sha256.clone(),
            effect_invariant_root_sha256: mode.effect_invariant_root_sha256.clone(),
            source_role_schema_root_sha256: mode.source_role_schema_root_sha256.clone(),
            selector_program_root_sha256: mode.selector_program_root_sha256.clone(),
            observed_emitted_types_root_sha256: mode.observed_emitted_types_root_sha256.clone(),
            capability_protocol_root_sha256: mode.capability_protocol_root_sha256.clone(),
            argument_role_schema_root_sha256: mode.argument_role_schema_root_sha256.clone(),
            constant_contract_root_sha256: mode.constant_contract_root_sha256.clone(),
            structural_guard_root_sha256: mode.structural_guard_root_sha256.clone(),
            temporal_cardinality_contract_root_sha256: mode
                .temporal_cardinality_contract_root_sha256
                .clone(),
            action_class_root_sha256: mode.action_class_root_sha256.clone(),
            program: mode.program.clone(),
            covers_positive_rows_sha256: Vec::new(),
            accepts_negative_rows_sha256: Vec::new(),
            wrong_action_rows_sha256: Vec::new(),
            verify_failed_rows_sha256: Vec::new(),
            search_exhausted: false,
        };
        validate_candidate_v2(&candidate)?;
        if mode.effect_law_id_sha256 != set.effect_law_id_sha256
            || mode.relation_identity_sha256 != set.relation_identity_sha256
            || mode.mode_id_sha256 != protocol_mode_digest_v2(mode)?
            || !mode_ids.insert(mode.mode_id_sha256.clone())
            || mode.covered_positive_rows_sha256.is_empty()
            || mode
                .covered_positive_rows_sha256
                .iter()
                .any(|root| !is_sha256(root))
        {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        if !mode
            .covered_positive_rows_sha256
            .windows(2)
            .all(|pair| pair[0] < pair[1])
            || mode
                .covered_positive_rows_sha256
                .iter()
                .any(|row| !covered_rows.insert(row.clone()))
        {
            return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
        }
        action_classes.insert(mode.action_class_root_sha256.clone());
    }
    if !set.modes.is_empty()
        && (covered_rows.len() != set.positive_rows_covered
            || action_classes.len() != set.action_equivalence_classes)
    {
        return Err(BindingProtocolCompilerErrorV2::InvalidModeSet);
    }
    Ok(())
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
