use super::super::{K2CompositionErrorV1, K2CompositionResultV1};
use super::{
    K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1, K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1,
    K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1,
    K2UncertaintyClosureCensusV1, K2UncertaintyClosureDispositionV1,
    K2UncertaintyClosurePlannerRequestV1, K2UncertaintyCompletionCandidateV1,
    K2UncertaintyRawProbeDispositionV1, checked_plan_budget_v1, closure_partition_sizes_v1,
    closure_probe_eligible_v1, denied_authority_v1, prediction_roots_v1,
    representative_root_set_v1, uncertainty_root_v1,
};

pub fn plan_self_formed_uncertainty_closure_v1(
    request: &K2UncertaintyClosurePlannerRequestV1,
) -> K2CompositionResultV1<K2UncertaintyClosureCensusV1> {
    request.validate()?;
    let first = find_representative_v1(request, &request.first_probe_root_sha256)?;
    let first_equal = first.equivalence_key.pairwise_outcome_equal;
    let first_partition = closure_partition_sizes_v1(first_equal)?;
    let completion_required = first_partition.first().copied().unwrap_or_default() > 1;
    let representative_roots = representative_root_set_v1(request)
        .into_iter()
        .collect::<Vec<_>>();
    let second_roots = if completion_required {
        representative_roots
            .iter()
            .filter(|root| *root != &request.first_probe_root_sha256)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut candidates = Vec::with_capacity(second_roots.len());
    for second_root in &second_roots {
        let second = find_representative_v1(request, second_root)?;
        candidates.push(build_candidate_v1(request, first, second)?);
    }
    candidates.sort_by(|left, right| {
        left.second_probe_root_sha256
            .cmp(&right.second_probe_root_sha256)
    });
    let candidate_roots = candidates
        .iter()
        .map(|candidate| candidate.candidate_root_sha256.clone())
        .collect::<Vec<_>>();
    let selected = candidates
        .iter()
        .filter(|candidate| {
            candidate.eligible && candidate.joint_partition_sizes == [1_u64, 1, 1, 1]
        })
        .min_by(|left, right| super::compare_completion_candidates_v1(left, right));
    let disposition = if !completion_required {
        K2UncertaintyClosureDispositionV1::SingleProbe
    } else if selected.is_some() {
        K2UncertaintyClosureDispositionV1::TwoProbe
    } else {
        K2UncertaintyClosureDispositionV1::ClosureUnavailable
    };
    let selected_second_probe_root_sha256 =
        selected.map(|candidate| candidate.second_probe_root_sha256.clone());
    let selected_joint_partition_sizes =
        selected.map(|candidate| candidate.joint_partition_sizes.clone());
    let membership_root_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-completion-membership.v1",
        &representative_roots,
        &request.first_probe_root_sha256,
        &second_roots,
    ))?;
    let candidate_denominator_root_sha256 = uncertainty_root_v1(&(
        "nando.k2-self-formed-completion-denominator.v1",
        &candidate_roots,
    ))?;
    let mut census = K2UncertaintyClosureCensusV1 {
        schema: K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1.to_owned(),
        planner_request_root_sha256: request.request_root_sha256.clone(),
        case_id_sha256: request.case_id_sha256.clone(),
        frontier_root_sha256: request.frontier_root_sha256.clone(),
        representative_probe_roots_sha256: representative_roots,
        representative_count: request.representatives.len() as u64,
        first_tournament_root_sha256: request.first_tournament_root_sha256.clone(),
        first_probe_root_sha256: request.first_probe_root_sha256.clone(),
        first_pairwise_outcome_equal: first_equal,
        first_partition_sizes: first_partition,
        completion_required,
        second_probe_candidate_roots_sha256: second_roots,
        candidate_count: candidates.len() as u64,
        membership_root_sha256,
        candidates,
        candidate_denominator_root_sha256,
        disposition,
        selected_second_probe_root_sha256,
        selected_joint_partition_sizes,
        planner_executable_sha256: request.planner_executable_sha256.clone(),
        authority: denied_authority_v1(),
        census_root_sha256: String::new(),
    };
    census.reseal()?;
    Ok(census)
}

fn find_representative_v1<'a>(
    request: &'a K2UncertaintyClosurePlannerRequestV1,
    root: &str,
) -> K2CompositionResultV1<&'a K2UncertaintyRawProbeDispositionV1> {
    request
        .representatives
        .binary_search_by(|value| value.probe.probe_root_sha256.as_str().cmp(root))
        .ok()
        .map(|index| &request.representatives[index])
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_closure_representative_missing",
        ))
}

fn build_candidate_v1(
    request: &K2UncertaintyClosurePlannerRequestV1,
    first: &K2UncertaintyRawProbeDispositionV1,
    second: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2UncertaintyCompletionCandidateV1> {
    let joint = std::array::from_fn(|index| {
        first.equivalence_key.pairwise_outcome_equal[index]
            && second.equivalence_key.pairwise_outcome_equal[index]
    });
    let partition = closure_partition_sizes_v1(joint)?;
    let largest = partition.first().copied().unwrap_or_default();
    let pair_separation = 16_u64.saturating_sub(
        partition
            .iter()
            .map(|size| size.saturating_mul(*size))
            .sum(),
    );
    let cumulative_risk_units = checked_plan_budget_v1(
        first.probe.risk_units,
        second.probe.risk_units,
        K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1,
        "self_formed_completion_risk_budget_invalid",
    )?;
    let cumulative_cost_units = checked_plan_budget_v1(
        first.probe.cost_units,
        second.probe.cost_units,
        K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
        "self_formed_completion_cost_budget_invalid",
    )?;
    let mut candidate = K2UncertaintyCompletionCandidateV1 {
        schema: K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1.to_owned(),
        case_id_sha256: request.case_id_sha256.clone(),
        first_probe_root_sha256: first.probe.probe_root_sha256.clone(),
        second_probe_root_sha256: second.probe.probe_root_sha256.clone(),
        first_prediction_roots_sha256: prediction_roots_v1(first),
        second_prediction_roots_sha256: prediction_roots_v1(second),
        joint_pairwise_outcome_equal: joint,
        joint_partition_sizes: partition,
        joint_minimax_eliminated: 4_u64.saturating_sub(largest),
        joint_pair_separation: pair_separation,
        cumulative_risk_units,
        cumulative_cost_units,
        eligible: closure_probe_eligible_v1(first) && closure_probe_eligible_v1(second),
        authority: denied_authority_v1(),
        candidate_root_sha256: String::new(),
    };
    candidate.reseal()?;
    Ok(candidate)
}
