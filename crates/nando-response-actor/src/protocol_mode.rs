use std::collections::{BTreeMap, BTreeSet};

mod selector;

pub use nando_operator_kernel::{
    BindingProtocolCompileVerdictV2, BindingProtocolCompilerErrorV2,
    BoundedProtocolModeCandidateV2, PROTOCOL_MODE_SET_SCHEMA_V2, ProtocolArgumentRoleSchemaV2,
    ProtocolArgumentRoleV2, ProtocolCapabilityContractV2, ProtocolConstantContractV2,
    ProtocolModeCompilerBudgetV2, ProtocolModeProgramV2, ProtocolModeSetV2, ProtocolModeV2,
    ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2, ProtocolSourceRoleSchemaV2,
    ProtocolSourceRoleV2, ProtocolStructuralGuardV2, ProtocolTemporalCardinalityContractV2,
    ProtocolValueContractV2,
};
use nando_operator_kernel::{
    is_protocol_mode_sha256 as is_sha256, protocol_mode_from_candidate_v2,
    protocol_mode_set_digest_v2, validate_protocol_mode_budget_v2 as validate_budget_v2,
    validate_protocol_mode_candidate_v2 as validate_candidate_v2, validate_protocol_mode_set_v2,
};

use crate::binding_evidence_adjudication::{
    AcceptedBindingLawEvidenceV2, BindingTrialEvidenceLabelV2,
};
#[cfg(test)]
use crate::binding_evidence_adjudication::{PhysicalTrialOutcomeV2, TrustedResolvedBindingRowV2};
use crate::{CanonicalEffectLawV3, FrozenCandidateRelationGraphV1};

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
