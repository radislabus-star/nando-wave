use std::cmp::Ordering;

use serde::Serialize;

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, K2InquiryObservationModeV1};
use super::final_verifier_frontier::IndependentFrontierV1;
use super::{
    K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1, K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1,
    K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_MODELS_V1,
    K2_UNCERTAINTY_MAX_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
    K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1, K2_UNCERTAINTY_MAX_RISK_UNITS_V1,
    K2UncertaintyCasePreverificationV2, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosurePlanV1,
    K2UncertaintyCompletionCandidateV1, K2UncertaintyEligibilityDispositionV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintySafetyDispositionV1, denied_authority_v1,
    uncertainty_root_v1,
};

pub(super) struct IndependentClosureV2 {
    pub candidate_count: u64,
    pub joint_pairwise_comparisons: u64,
}

pub(super) fn verify_closure_v2(
    frontier: &IndependentFrontierV1,
    case: &K2UncertaintyCasePreverificationV2,
) -> K2CompositionResultV1<IndependentClosureV2> {
    let planner = &case.closure_verification_request.planner_request;
    let representatives = frontier
        .representatives
        .values()
        .cloned()
        .collect::<Vec<_>>();
    if planner.representatives != representatives
        || planner.first_probe_root_sha256
            != case
                .selection_preverification
                .tournament
                .tournament_winner_probe_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_closure_representatives_mismatch",
        ));
    }
    let reconstructed = independent_census_v2(planner)?;
    if reconstructed != case.closure_verification_request.planner_census {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_closure_census_mismatch",
        ));
    }
    let receipt = &case.closure_verification_receipt;
    let comparisons =
        reconstructed
            .candidate_count
            .checked_mul(6)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_comparison_count_overflow",
            ))?;
    if receipt.planner_census_root_sha256 != reconstructed.census_root_sha256
        || receipt.reconstructed_census_root_sha256 != reconstructed.census_root_sha256
        || receipt.candidate_denominator_root_sha256
            != reconstructed.candidate_denominator_root_sha256
        || receipt.candidate_count != reconstructed.candidate_count
        || receipt.joint_pairwise_comparison_count != comparisons
        || receipt.disposition != reconstructed.disposition
        || receipt.selected_second_probe_root_sha256
            != reconstructed.selected_second_probe_root_sha256
        || !receipt.census_verified
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_closure_receipt_mismatch",
        ));
    }
    let expected_plan = independent_plan_v2(planner, &reconstructed, receipt)?;
    if case.closure_plan.as_ref() != Some(&expected_plan) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_closure_plan_mismatch",
        ));
    }
    Ok(IndependentClosureV2 {
        candidate_count: reconstructed.candidate_count,
        joint_pairwise_comparisons: comparisons,
    })
}

fn independent_census_v2(
    planner: &super::K2UncertaintyClosurePlannerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyClosureCensusV1> {
    let first = planner
        .representatives
        .iter()
        .find(|value| value.probe.probe_root_sha256 == planner.first_probe_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_first_probe_missing",
        ))?;
    let first_equal = independent_pairwise_equal_v2(first)?;
    let first_partition = independent_partition_v2(first_equal)?;
    let completion_required = first_partition.first().copied().unwrap_or_default() > 1;
    let representative_roots = planner
        .representatives
        .iter()
        .map(|value| value.probe.probe_root_sha256.clone())
        .collect::<Vec<_>>();
    let second_roots = if completion_required {
        representative_roots
            .iter()
            .filter(|root| *root != &planner.first_probe_root_sha256)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut candidates = Vec::with_capacity(second_roots.len());
    for second_root in &second_roots {
        let second = planner
            .representatives
            .iter()
            .find(|value| &value.probe.probe_root_sha256 == second_root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_second_probe_missing",
            ))?;
        candidates.push(independent_candidate_v2(planner, first, second)?);
    }
    candidates.sort_by(|left, right| {
        left.second_probe_root_sha256
            .cmp(&right.second_probe_root_sha256)
    });
    let candidate_roots = candidates
        .iter()
        .map(|value| value.candidate_root_sha256.clone())
        .collect::<Vec<_>>();
    let selected = candidates
        .iter()
        .filter(|value| value.eligible && value.joint_partition_sizes == [1_u64, 1, 1, 1])
        .min_by(|left, right| independent_candidate_order_v2(left, right));
    let disposition = if !completion_required {
        K2UncertaintyClosureDispositionV1::SingleProbe
    } else if selected.is_some() {
        K2UncertaintyClosureDispositionV1::TwoProbe
    } else {
        K2UncertaintyClosureDispositionV1::ClosureUnavailable
    };
    let selected_second = selected.map(|value| value.second_probe_root_sha256.clone());
    let selected_partition = selected.map(|value| value.joint_partition_sizes.clone());
    let membership_root = uncertainty_root_v1(&(
        "nando.k2-self-formed-completion-membership.v1",
        &representative_roots,
        &planner.first_probe_root_sha256,
        &second_roots,
    ))?;
    let denominator_root = uncertainty_root_v1(&(
        "nando.k2-self-formed-completion-denominator.v1",
        &candidate_roots,
    ))?;
    let mut census = K2UncertaintyClosureCensusV1 {
        schema: K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1.to_owned(),
        planner_request_root_sha256: planner.request_root_sha256.clone(),
        case_id_sha256: planner.case_id_sha256.clone(),
        frontier_root_sha256: planner.frontier_root_sha256.clone(),
        representative_probe_roots_sha256: representative_roots,
        representative_count: planner.representatives.len() as u64,
        first_tournament_root_sha256: planner.first_tournament_root_sha256.clone(),
        first_probe_root_sha256: planner.first_probe_root_sha256.clone(),
        first_pairwise_outcome_equal: first_equal,
        first_partition_sizes: first_partition,
        completion_required,
        second_probe_candidate_roots_sha256: second_roots,
        candidate_count: candidates.len() as u64,
        membership_root_sha256: membership_root,
        candidates,
        candidate_denominator_root_sha256: denominator_root,
        disposition,
        selected_second_probe_root_sha256: selected_second,
        selected_joint_partition_sizes: selected_partition,
        planner_executable_sha256: planner.planner_executable_sha256.clone(),
        authority: denied_authority_v1(),
        census_root_sha256: String::new(),
    };
    census.census_root_sha256 = independent_census_root_v2(&census)?;
    Ok(census)
}

fn independent_candidate_v2(
    planner: &super::K2UncertaintyClosurePlannerRequestV1,
    first: &K2UncertaintyRawProbeDispositionV1,
    second: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2UncertaintyCompletionCandidateV1> {
    let first_equal = independent_pairwise_equal_v2(first)?;
    let second_equal = independent_pairwise_equal_v2(second)?;
    let joint = std::array::from_fn(|index| first_equal[index] && second_equal[index]);
    let partition = independent_partition_v2(joint)?;
    let largest = partition.first().copied().unwrap_or_default();
    let within_pairs = partition
        .iter()
        .try_fold(0_u64, |sum, size| sum.checked_add(size.checked_mul(*size)?))
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_pair_separation_overflow",
        ))?;
    let pair_separation = 16_u64
        .checked_sub(within_pairs)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_pair_separation_invalid",
        ))?;
    let risk = independent_budget_v2(
        first.probe.risk_units,
        second.probe.risk_units,
        K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1,
        "self_formed_final_v2_risk_invalid",
    )?;
    let cost = independent_budget_v2(
        first.probe.cost_units,
        second.probe.cost_units,
        K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
        "self_formed_final_v2_cost_invalid",
    )?;
    let mut candidate = K2UncertaintyCompletionCandidateV1 {
        schema: K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1.to_owned(),
        case_id_sha256: planner.case_id_sha256.clone(),
        first_probe_root_sha256: first.probe.probe_root_sha256.clone(),
        second_probe_root_sha256: second.probe.probe_root_sha256.clone(),
        first_prediction_roots_sha256: independent_prediction_roots_v2(first)?,
        second_prediction_roots_sha256: independent_prediction_roots_v2(second)?,
        joint_pairwise_outcome_equal: joint,
        joint_partition_sizes: partition,
        joint_minimax_eliminated: 4_u64.checked_sub(largest).ok_or(
            K2CompositionErrorV1::Invalid("self_formed_final_v2_minimax_invalid"),
        )?,
        joint_pair_separation: pair_separation,
        cumulative_risk_units: risk,
        cumulative_cost_units: cost,
        eligible: independent_probe_eligible_v2(first) && independent_probe_eligible_v2(second),
        authority: denied_authority_v1(),
        candidate_root_sha256: String::new(),
    };
    candidate.candidate_root_sha256 = independent_candidate_root_v2(&candidate)?;
    Ok(candidate)
}

fn independent_plan_v2(
    planner: &super::K2UncertaintyClosurePlannerRequestV1,
    census: &K2UncertaintyClosureCensusV1,
    receipt: &super::K2UncertaintyClosureVerificationReceiptV1,
) -> K2CompositionResultV1<K2UncertaintyClosurePlanV1> {
    let first = planner
        .representatives
        .iter()
        .find(|value| value.probe.probe_root_sha256 == planner.first_probe_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_plan_first_missing",
        ))?;
    let first_predictions = independent_prediction_roots_v2(first)?;
    let (probe_roots, prediction_roots, risk, cost) = match census.disposition {
        K2UncertaintyClosureDispositionV1::SingleProbe => (
            vec![planner.first_probe_root_sha256.clone()],
            vec![first_predictions],
            first.probe.risk_units,
            first.probe.cost_units,
        ),
        K2UncertaintyClosureDispositionV1::TwoProbe => {
            let second_root = census.selected_second_probe_root_sha256.as_ref().ok_or(
                K2CompositionErrorV1::Invalid("self_formed_final_v2_plan_second_missing"),
            )?;
            let candidate = census
                .candidates
                .iter()
                .find(|value| &value.second_probe_root_sha256 == second_root)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_final_v2_plan_candidate_missing",
                ))?;
            (
                vec![planner.first_probe_root_sha256.clone(), second_root.clone()],
                vec![
                    candidate.first_prediction_roots_sha256.clone(),
                    candidate.second_prediction_roots_sha256.clone(),
                ],
                candidate.cumulative_risk_units,
                candidate.cumulative_cost_units,
            )
        }
        K2UncertaintyClosureDispositionV1::ClosureUnavailable => {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_plan_unavailable",
            ));
        }
    };
    let mut plan = K2UncertaintyClosurePlanV1 {
        schema: K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1.to_owned(),
        case_id_sha256: planner.case_id_sha256.clone(),
        frontier_root_sha256: planner.frontier_root_sha256.clone(),
        first_tournament_root_sha256: planner.first_tournament_root_sha256.clone(),
        first_probe_root_sha256: planner.first_probe_root_sha256.clone(),
        first_partition_sizes: census.first_partition_sizes.clone(),
        completion_required: census.completion_required,
        candidate_denominator_root_sha256: census.candidate_denominator_root_sha256.clone(),
        closure_census_root_sha256: census.census_root_sha256.clone(),
        disposition: census.disposition,
        selected_second_probe_root_sha256: census.selected_second_probe_root_sha256.clone(),
        selected_joint_partition_sizes: census
            .selected_joint_partition_sizes
            .clone()
            .unwrap_or_else(|| census.first_partition_sizes.clone()),
        plan_length: probe_roots.len() as u64,
        ordered_probe_roots_sha256: probe_roots,
        ordered_prediction_roots_sha256: prediction_roots,
        cumulative_risk_units: risk,
        cumulative_cost_units: cost,
        planner_executable_sha256: planner.planner_executable_sha256.clone(),
        preverification_receipt_root_sha256: receipt.receipt_root_sha256.clone(),
        authority: denied_authority_v1(),
        plan_root_sha256: String::new(),
    };
    plan.plan_root_sha256 = independent_plan_root_v2(&plan)?;
    Ok(plan)
}

fn independent_pairwise_equal_v2(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<[bool; 6]> {
    if disposition.predictions.len() != K2_UNCERTAINTY_CONFIRM_MODELS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_prediction_count_invalid",
        ));
    }
    let outcomes = disposition
        .predictions
        .iter()
        .map(|value| value.observable_outcome_root_sha256.as_str())
        .collect::<Vec<_>>();
    Ok([
        outcomes[0] == outcomes[1],
        outcomes[0] == outcomes[2],
        outcomes[0] == outcomes[3],
        outcomes[1] == outcomes[2],
        outcomes[1] == outcomes[3],
        outcomes[2] == outcomes[3],
    ])
}

fn independent_partition_v2(equal: [bool; 6]) -> K2CompositionResultV1<Vec<u64>> {
    let pairs = [(0_usize, 1_usize), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let mut adjacency = [[false; 4]; 4];
    for (index, row) in adjacency.iter_mut().enumerate() {
        row[index] = true;
    }
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        adjacency[left][right] = same;
        adjacency[right][left] = same;
    }
    let mut components = [usize::MAX; 4];
    let mut component_count = 0_usize;
    for start in 0..4 {
        if components[start] != usize::MAX {
            continue;
        }
        let mut stack = vec![start];
        components[start] = component_count;
        while let Some(left) = stack.pop() {
            for right in 0..4 {
                if adjacency[left][right] && components[right] == usize::MAX {
                    components[right] = component_count;
                    stack.push(right);
                }
            }
        }
        component_count += 1;
    }
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        if (components[left] == components[right]) != same {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_equality_nontransitive",
            ));
        }
    }
    let mut sizes = vec![0_u64; component_count];
    for component in components {
        sizes[component] = sizes[component]
            .checked_add(1)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_final_v2_partition_overflow",
            ))?;
    }
    sizes.sort_by(|left, right| right.cmp(left));
    Ok(sizes)
}

fn independent_prediction_roots_v2(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<Vec<String>> {
    if disposition.predictions.len() != K2_UNCERTAINTY_CONFIRM_MODELS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_v2_prediction_count_invalid",
        ));
    }
    Ok(disposition
        .predictions
        .iter()
        .map(|value| value.prediction_root_sha256.clone())
        .collect())
}

fn independent_probe_eligible_v2(disposition: &K2UncertaintyRawProbeDispositionV1) -> bool {
    disposition.eligibility == K2UncertaintyEligibilityDispositionV1::Eligible
        && disposition.safety == K2UncertaintySafetyDispositionV1::Pass
        && disposition.probe.reversible
        && disposition.probe.observation_mode == K2InquiryObservationModeV1::ExactImmediate
        && disposition.probe.risk_units <= K2_UNCERTAINTY_MAX_RISK_UNITS_V1
        && disposition.probe.cost_units <= K2_UNCERTAINTY_MAX_COST_UNITS_V1
}

fn independent_budget_v2(
    first: u64,
    second: u64,
    limit: u64,
    reason: &'static str,
) -> K2CompositionResultV1<u64> {
    let total = first
        .checked_add(second)
        .ok_or(K2CompositionErrorV1::Invalid(reason))?;
    if total > limit {
        return Err(K2CompositionErrorV1::Invalid(reason));
    }
    Ok(total)
}

fn independent_candidate_order_v2(
    left: &K2UncertaintyCompletionCandidateV1,
    right: &K2UncertaintyCompletionCandidateV1,
) -> Ordering {
    right
        .joint_minimax_eliminated
        .cmp(&left.joint_minimax_eliminated)
        .then_with(|| right.joint_pair_separation.cmp(&left.joint_pair_separation))
        .then_with(|| left.cumulative_risk_units.cmp(&right.cumulative_risk_units))
        .then_with(|| left.cumulative_cost_units.cmp(&right.cumulative_cost_units))
        .then_with(|| {
            left.second_probe_root_sha256
                .cmp(&right.second_probe_root_sha256)
        })
}

fn independent_candidate_root_v2(
    value: &K2UncertaintyCompletionCandidateV1,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1,
        &value.case_id_sha256,
        &value.first_probe_root_sha256,
        &value.second_probe_root_sha256,
        &value.first_prediction_roots_sha256,
        &value.second_prediction_roots_sha256,
        value.joint_pairwise_outcome_equal,
        &value.joint_partition_sizes,
        value.joint_minimax_eliminated,
        value.joint_pair_separation,
        value.cumulative_risk_units,
        value.cumulative_cost_units,
        value.eligible,
        &value.authority,
    ))
}

fn independent_census_root_v2(
    value: &K2UncertaintyClosureCensusV1,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&IndependentCensusRootV2 {
        schema: K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1,
        planner_request_root_sha256: &value.planner_request_root_sha256,
        case_id_sha256: &value.case_id_sha256,
        frontier_root_sha256: &value.frontier_root_sha256,
        representative_probe_roots_sha256: &value.representative_probe_roots_sha256,
        representative_count: value.representative_count,
        first_tournament_root_sha256: &value.first_tournament_root_sha256,
        first_probe_root_sha256: &value.first_probe_root_sha256,
        first_pairwise_outcome_equal: value.first_pairwise_outcome_equal,
        first_partition_sizes: &value.first_partition_sizes,
        completion_required: value.completion_required,
        second_probe_candidate_roots_sha256: &value.second_probe_candidate_roots_sha256,
        candidate_count: value.candidate_count,
        membership_root_sha256: &value.membership_root_sha256,
        candidates: &value.candidates,
        candidate_denominator_root_sha256: &value.candidate_denominator_root_sha256,
        disposition: value.disposition,
        selected_second_probe_root_sha256: &value.selected_second_probe_root_sha256,
        selected_joint_partition_sizes: &value.selected_joint_partition_sizes,
        planner_executable_sha256: &value.planner_executable_sha256,
        authority: &value.authority,
    })
}

fn independent_plan_root_v2(value: &K2UncertaintyClosurePlanV1) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&IndependentPlanRootV2 {
        schema: K2_UNCERTAINTY_CLOSURE_PLAN_SCHEMA_V1,
        case_id_sha256: &value.case_id_sha256,
        frontier_root_sha256: &value.frontier_root_sha256,
        first_tournament_root_sha256: &value.first_tournament_root_sha256,
        first_probe_root_sha256: &value.first_probe_root_sha256,
        first_partition_sizes: &value.first_partition_sizes,
        completion_required: value.completion_required,
        candidate_denominator_root_sha256: &value.candidate_denominator_root_sha256,
        closure_census_root_sha256: &value.closure_census_root_sha256,
        disposition: value.disposition,
        selected_second_probe_root_sha256: &value.selected_second_probe_root_sha256,
        selected_joint_partition_sizes: &value.selected_joint_partition_sizes,
        plan_length: value.plan_length,
        ordered_probe_roots_sha256: &value.ordered_probe_roots_sha256,
        ordered_prediction_roots_sha256: &value.ordered_prediction_roots_sha256,
        cumulative_risk_units: value.cumulative_risk_units,
        cumulative_cost_units: value.cumulative_cost_units,
        planner_executable_sha256: &value.planner_executable_sha256,
        preverification_receipt_root_sha256: &value.preverification_receipt_root_sha256,
        authority: &value.authority,
    })
}

#[derive(Serialize)]
struct IndependentCensusRootV2<'a> {
    schema: &'static str,
    planner_request_root_sha256: &'a str,
    case_id_sha256: &'a str,
    frontier_root_sha256: &'a str,
    representative_probe_roots_sha256: &'a [String],
    representative_count: u64,
    first_tournament_root_sha256: &'a str,
    first_probe_root_sha256: &'a str,
    first_pairwise_outcome_equal: [bool; 6],
    first_partition_sizes: &'a [u64],
    completion_required: bool,
    second_probe_candidate_roots_sha256: &'a [String],
    candidate_count: u64,
    membership_root_sha256: &'a str,
    candidates: &'a [K2UncertaintyCompletionCandidateV1],
    candidate_denominator_root_sha256: &'a str,
    disposition: K2UncertaintyClosureDispositionV1,
    selected_second_probe_root_sha256: &'a Option<String>,
    selected_joint_partition_sizes: &'a Option<Vec<u64>>,
    planner_executable_sha256: &'a str,
    authority: &'a super::super::K2CompositionAuthorityBoundaryV1,
}

#[derive(Serialize)]
struct IndependentPlanRootV2<'a> {
    schema: &'static str,
    case_id_sha256: &'a str,
    frontier_root_sha256: &'a str,
    first_tournament_root_sha256: &'a str,
    first_probe_root_sha256: &'a str,
    first_partition_sizes: &'a [u64],
    completion_required: bool,
    candidate_denominator_root_sha256: &'a str,
    closure_census_root_sha256: &'a str,
    disposition: K2UncertaintyClosureDispositionV1,
    selected_second_probe_root_sha256: &'a Option<String>,
    selected_joint_partition_sizes: &'a [u64],
    plan_length: u64,
    ordered_probe_roots_sha256: &'a [String],
    ordered_prediction_roots_sha256: &'a [Vec<String>],
    cumulative_risk_units: u64,
    cumulative_cost_units: u64,
    planner_executable_sha256: &'a str,
    preverification_receipt_root_sha256: &'a str,
    authority: &'a super::super::K2CompositionAuthorityBoundaryV1,
}
