use std::cmp::Ordering;
use std::io::{Read, Write};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryObservationModeV1,
    composition_sha256_file_v1,
};
use super::{
    K2_UNCERTAINTY_CLOSURE_CENSUS_SCHEMA_V1, K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1,
    K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1, K2_UNCERTAINTY_CONFIRM_MODELS_V1,
    K2_UNCERTAINTY_MAX_COST_UNITS_V1, K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
    K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1, K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1,
    K2_UNCERTAINTY_MAX_RISK_UNITS_V1, K2UncertaintyClosureCensusV1,
    K2UncertaintyClosureDispositionV1, K2UncertaintyClosureVerificationReceiptV1,
    K2UncertaintyClosureVerificationRequestV1, K2UncertaintyCompletionCandidateV1,
    K2UncertaintyEligibilityDispositionV1, K2UncertaintyRawProbeDispositionV1,
    K2UncertaintySafetyDispositionV1, closure_census_root_v1, completion_candidate_root_v1,
    denied_authority_v1, uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

pub fn verify_self_formed_closure_independently_v1(
    request: &K2UncertaintyClosureVerificationRequestV1,
) -> K2CompositionResultV1<K2UncertaintyClosureVerificationReceiptV1> {
    request.validate()?;
    let reconstructed = independently_reconstruct_census_v1(request)?;
    if reconstructed != request.planner_census {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_census_mismatch",
        ));
    }
    let mut receipt = K2UncertaintyClosureVerificationReceiptV1 {
        schema: K2_UNCERTAINTY_CLOSURE_VERIFICATION_RECEIPT_SCHEMA_V1.to_owned(),
        verifier_executable_sha256: request.verifier_executable_sha256.clone(),
        verification_request_root_sha256: request.request_root_sha256.clone(),
        case_id_sha256: request.planner_request.case_id_sha256.clone(),
        planner_census_root_sha256: request.planner_census.census_root_sha256.clone(),
        reconstructed_census_root_sha256: reconstructed.census_root_sha256,
        candidate_denominator_root_sha256: reconstructed.candidate_denominator_root_sha256,
        candidate_count: reconstructed.candidate_count,
        joint_pairwise_comparison_count: reconstructed.candidate_count.checked_mul(6).ok_or(
            K2CompositionErrorV1::Invalid("self_formed_closure_verification_comparison_overflow"),
        )?,
        disposition: reconstructed.disposition,
        selected_second_probe_root_sha256: reconstructed.selected_second_probe_root_sha256,
        census_verified: true,
        authority: denied_authority_v1(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

pub fn run_self_formed_closure_verifier_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_closure_verifier_stdin"))?;
    let request: K2UncertaintyClosureVerificationRequestV1 = uncertainty_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_closure_verifier"))?;
    if composition_sha256_file_v1(&executable)? != request.verifier_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verifier_executable_mismatch",
        ));
    }
    let receipt = verify_self_formed_closure_independently_v1(&request)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_closure_verifier_stdout"))
}

fn independently_reconstruct_census_v1(
    request: &K2UncertaintyClosureVerificationRequestV1,
) -> K2CompositionResultV1<K2UncertaintyClosureCensusV1> {
    let planner = &request.planner_request;
    let first = planner
        .representatives
        .iter()
        .find(|value| value.probe.probe_root_sha256 == planner.first_probe_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_first_probe_missing",
        ))?;
    let first_equal = independent_pairwise_equal_v1(first)?;
    let first_partition = independent_partition_sizes_v1(first_equal)?;
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
    for root in &second_roots {
        let second = planner
            .representatives
            .iter()
            .find(|value| &value.probe.probe_root_sha256 == root)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_closure_verification_second_probe_missing",
            ))?;
        candidates.push(independent_candidate_v1(planner, first, second)?);
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
        .min_by(|left, right| independent_candidate_order_v1(left, right));
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
    census.census_root_sha256 = closure_census_root_v1(&census)?;
    Ok(census)
}

fn independent_candidate_v1(
    planner: &super::K2UncertaintyClosurePlannerRequestV1,
    first: &K2UncertaintyRawProbeDispositionV1,
    second: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2UncertaintyCompletionCandidateV1> {
    let first_equal = independent_pairwise_equal_v1(first)?;
    let second_equal = independent_pairwise_equal_v1(second)?;
    let joint = std::array::from_fn(|index| first_equal[index] && second_equal[index]);
    let partition = independent_partition_sizes_v1(joint)?;
    let largest = partition.first().copied().unwrap_or_default();
    let within_pairs = partition
        .iter()
        .try_fold(0_u64, |sum, size| sum.checked_add(size.checked_mul(*size)?));
    let pair_separation = 16_u64
        .checked_sub(within_pairs.ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_pair_separation_overflow",
        ))?)
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_pair_separation_invalid",
        ))?;
    let risk = independent_budget_v1(
        first.probe.risk_units,
        second.probe.risk_units,
        K2_UNCERTAINTY_MAX_PLAN_RISK_UNITS_V1,
        "self_formed_closure_verification_risk_invalid",
    )?;
    let cost = independent_budget_v1(
        first.probe.cost_units,
        second.probe.cost_units,
        K2_UNCERTAINTY_MAX_PLAN_COST_UNITS_V1,
        "self_formed_closure_verification_cost_invalid",
    )?;
    let mut candidate = K2UncertaintyCompletionCandidateV1 {
        schema: K2_UNCERTAINTY_COMPLETION_CANDIDATE_SCHEMA_V1.to_owned(),
        case_id_sha256: planner.case_id_sha256.clone(),
        first_probe_root_sha256: first.probe.probe_root_sha256.clone(),
        second_probe_root_sha256: second.probe.probe_root_sha256.clone(),
        first_prediction_roots_sha256: independent_prediction_roots_v1(first)?,
        second_prediction_roots_sha256: independent_prediction_roots_v1(second)?,
        joint_pairwise_outcome_equal: joint,
        joint_partition_sizes: partition,
        joint_minimax_eliminated: 4_u64.checked_sub(largest).ok_or(
            K2CompositionErrorV1::Invalid("self_formed_closure_verification_minimax_invalid"),
        )?,
        joint_pair_separation: pair_separation,
        cumulative_risk_units: risk,
        cumulative_cost_units: cost,
        eligible: independent_probe_eligible_v1(first) && independent_probe_eligible_v1(second),
        authority: denied_authority_v1(),
        candidate_root_sha256: String::new(),
    };
    candidate.candidate_root_sha256 = completion_candidate_root_v1(&candidate)?;
    Ok(candidate)
}

fn independent_pairwise_equal_v1(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<[bool; 6]> {
    if disposition.predictions.len() != K2_UNCERTAINTY_CONFIRM_MODELS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_prediction_count_invalid",
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

fn independent_partition_sizes_v1(equal: [bool; 6]) -> K2CompositionResultV1<Vec<u64>> {
    let pairs = [(0_usize, 1_usize), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    let mut adjacency = [[false; 4]; 4];
    for (index, row) in adjacency.iter_mut().enumerate() {
        row[index] = true;
    }
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        adjacency[left][right] = same;
        adjacency[right][left] = same;
    }
    let mut component = [usize::MAX; 4];
    let mut component_count = 0_usize;
    for start in 0..4 {
        if component[start] != usize::MAX {
            continue;
        }
        let mut stack = vec![start];
        component[start] = component_count;
        while let Some(left) = stack.pop() {
            for right in 0..4 {
                if adjacency[left][right] && component[right] == usize::MAX {
                    component[right] = component_count;
                    stack.push(right);
                }
            }
        }
        component_count += 1;
    }
    for ((left, right), same) in pairs.into_iter().zip(equal) {
        if (component[left] == component[right]) != same {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_closure_verification_equality_nontransitive",
            ));
        }
    }
    let mut sizes = vec![0_u64; component_count];
    for value in component {
        sizes[value] = sizes[value]
            .checked_add(1)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_closure_verification_partition_overflow",
            ))?;
    }
    sizes.sort_by(|left, right| right.cmp(left));
    Ok(sizes)
}

fn independent_prediction_roots_v1(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<Vec<String>> {
    if disposition.predictions.len() != K2_UNCERTAINTY_CONFIRM_MODELS_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_closure_verification_prediction_count_invalid",
        ));
    }
    Ok(disposition
        .predictions
        .iter()
        .map(|value| value.prediction_root_sha256.clone())
        .collect())
}

fn independent_probe_eligible_v1(disposition: &K2UncertaintyRawProbeDispositionV1) -> bool {
    disposition.eligibility == K2UncertaintyEligibilityDispositionV1::Eligible
        && disposition.safety == K2UncertaintySafetyDispositionV1::Pass
        && disposition.probe.reversible
        && disposition.probe.observation_mode == K2InquiryObservationModeV1::ExactImmediate
        && disposition.probe.risk_units <= K2_UNCERTAINTY_MAX_RISK_UNITS_V1
        && disposition.probe.cost_units <= K2_UNCERTAINTY_MAX_COST_UNITS_V1
}

fn independent_budget_v1(
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

fn independent_candidate_order_v1(
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
