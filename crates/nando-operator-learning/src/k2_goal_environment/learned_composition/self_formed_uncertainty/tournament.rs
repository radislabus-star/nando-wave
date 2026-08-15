use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    K2_INQUIRY_MAX_COST_UNITS_V1, K2_INQUIRY_MAX_RISK_UNITS_V1, K2CompositionErrorV1,
    K2CompositionResultV1, K2InquiryBaselineKindV1, K2InquiryBaselineRequestV1,
    K2InquiryBaselinesV1, K2InquiryObservationModeV1, K2InquiryProbeV1, K2InquiryPublicCaseV1,
    K2InquirySelectorRequestV1, evaluate_inquiry_baselines_v1, require_composition_root_v1,
    select_model_guided_probe_v1,
};
use super::{
    K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1, K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1,
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1,
    K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1, K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1,
    K2UncertaintyDirectScoreV1, K2UncertaintyDirectWinnerV1, K2UncertaintyEligibilityDispositionV1,
    K2UncertaintyLearnerResponseV1, K2UncertaintyProbeOutputV1, K2UncertaintyPublicCaseV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintySafetyDispositionV1,
    K2UncertaintyTournamentStepKindV1, K2UncertaintyTournamentStepV1, K2UncertaintyTournamentV1,
    denied_authority_v1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyBaselineTournamentV1 {
    pub kind: K2InquiryBaselineKindV1,
    pub requests: Vec<K2InquiryBaselineRequestV1>,
    pub outcomes: Vec<K2InquiryBaselinesV1>,
    pub selected_probe_root_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyTournamentArtifactsV1 {
    pub steps: Vec<K2UncertaintyTournamentStepV1>,
    pub tournament: K2UncertaintyTournamentV1,
    pub baselines: Vec<K2UncertaintyBaselineTournamentV1>,
}

#[allow(clippy::too_many_arguments)]
pub fn run_self_formed_tournament_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    probe_output: &K2UncertaintyProbeOutputV1,
    split_commitment_root_sha256: &str,
    selector_source_sha256: &str,
    selector_executable_sha256: &str,
    baseline_executable_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyTournamentArtifactsV1> {
    public_case.validate()?;
    learner_response.validate()?;
    probe_output.validate()?;
    for root in [
        split_commitment_root_sha256,
        selector_source_sha256,
        selector_executable_sha256,
        baseline_executable_sha256,
    ] {
        require_composition_root_v1(root)?;
    }
    if selector_source_sha256 != K2_UNCERTAINTY_SELECTOR_SOURCE_SHA256_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_selector_source_drift",
        ));
    }
    if public_case.vocabulary.case_id_sha256 != learner_response.model_set.case_id_sha256
        || public_case.vocabulary.case_id_sha256 != probe_output.frontier.case_id_sha256
        || learner_response.model_set.model_set_root_sha256
            != probe_output.frontier.model_set_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_input_binding_invalid",
        ));
    }

    let representatives = representative_dispositions_v1(probe_output)?;
    let probes = representatives
        .iter()
        .map(|(root, disposition)| (root.clone(), disposition.probe.clone()))
        .collect::<BTreeMap<_, _>>();
    let direct_winner = direct_winner_v1(probe_output, &representatives)?;
    let (steps, tournament_winner_probe_root_sha256) = selector_tournament_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        selector_executable_sha256,
        &probe_output.frontier.frontier_root_sha256,
        &probes,
    )?;
    if tournament_winner_probe_root_sha256 != direct_winner.selected_probe_root_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_direct_winner_mismatch",
        ));
    }

    let mut tournament = K2UncertaintyTournamentV1 {
        schema: K2_UNCERTAINTY_TOURNAMENT_SCHEMA_V1.to_owned(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
        frontier_root_sha256: probe_output.frontier.frontier_root_sha256.clone(),
        representative_count: probes.len() as u64,
        selector_source_sha256: selector_source_sha256.to_owned(),
        selector_executable_sha256: selector_executable_sha256.to_owned(),
        step_roots_sha256: steps
            .iter()
            .map(|step| step.step_root_sha256.clone())
            .collect(),
        request_count: steps.len() as u64,
        adapted_prediction_count: (steps.len() * K2_UNCERTAINTY_SELECTOR_PROBES_V1 * 4) as u64,
        tournament_winner_probe_root_sha256,
        direct_winner,
        authority: denied_authority_v1(),
        tournament_root_sha256: String::new(),
    };
    tournament.reseal()?;

    let mut baselines = vec![passive_baseline_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        baseline_executable_sha256,
        &probes,
    )?];
    for kind in [
        K2InquiryBaselineKindV1::StableHash,
        K2InquiryBaselineKindV1::CheapestFirst,
        K2InquiryBaselineKindV1::ExplicitHeuristic,
    ] {
        baselines.push(baseline_tournament_v1(
            kind,
            public_case,
            learner_response,
            split_commitment_root_sha256,
            baseline_executable_sha256,
            &probes,
        )?);
    }
    baselines.sort_by_key(|baseline| baseline.kind);

    Ok(K2UncertaintyTournamentArtifactsV1 {
        steps,
        tournament,
        baselines,
    })
}

fn representative_dispositions_v1(
    probe_output: &K2UncertaintyProbeOutputV1,
) -> K2CompositionResultV1<BTreeMap<String, K2UncertaintyRawProbeDispositionV1>> {
    let expected = probe_output
        .frontier
        .representative_probe_roots_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut representatives = BTreeMap::new();
    for disposition in probe_output
        .pages
        .iter()
        .flat_map(|page| &page.dispositions)
    {
        let root = &disposition.probe.probe_root_sha256;
        if expected.contains(root)
            && representatives
                .insert(root.clone(), disposition.clone())
                .is_some()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_tournament_duplicate_representative",
            ));
        }
    }
    if representatives.len() != expected.len()
        || representatives.keys().cloned().collect::<BTreeSet<_>>() != expected
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_representative_coverage_invalid",
        ));
    }
    Ok(representatives)
}

fn direct_winner_v1(
    probe_output: &K2UncertaintyProbeOutputV1,
    representatives: &BTreeMap<String, K2UncertaintyRawProbeDispositionV1>,
) -> K2CompositionResultV1<K2UncertaintyDirectWinnerV1> {
    let mut scores = Vec::with_capacity(representatives.len());
    for (root, disposition) in representatives {
        let partition_sizes =
            partition_sizes_v1(disposition.equivalence_key.pairwise_outcome_equal);
        let largest_partition = partition_sizes.first().copied().unwrap_or_default();
        let model_count = 4_u64;
        let minimax_eliminated = model_count.saturating_sub(largest_partition);
        let pair_separation = model_count.saturating_mul(model_count).saturating_sub(
            partition_sizes
                .iter()
                .map(|size| size.saturating_mul(*size))
                .sum(),
        );
        let eligible = disposition.eligibility == K2UncertaintyEligibilityDispositionV1::Eligible
            && disposition.safety == K2UncertaintySafetyDispositionV1::Pass
            && disposition.probe.reversible
            && disposition.probe.observation_mode == K2InquiryObservationModeV1::ExactImmediate
            && disposition.probe.risk_units <= K2_INQUIRY_MAX_RISK_UNITS_V1
            && disposition.probe.cost_units <= K2_INQUIRY_MAX_COST_UNITS_V1;
        let mut score = K2UncertaintyDirectScoreV1 {
            schema: K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1.to_owned(),
            probe_root_sha256: root.clone(),
            eligible,
            minimax_eliminated,
            pair_separation,
            risk_units: disposition.probe.risk_units,
            cost_units: disposition.probe.cost_units,
            score_root_sha256: String::new(),
        };
        score.reseal()?;
        scores.push(score);
    }
    scores.sort_by(|left, right| left.probe_root_sha256.cmp(&right.probe_root_sha256));
    let selected_probe_root_sha256 = scores
        .iter()
        .filter(|score| score.eligible)
        .min_by(|left, right| compare_scores_v1(left, right))
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_direct_winner_missing",
        ))?
        .probe_root_sha256
        .clone();
    let mut winner = K2UncertaintyDirectWinnerV1 {
        schema: K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1.to_owned(),
        case_id_sha256: probe_output.frontier.case_id_sha256.clone(),
        frontier_root_sha256: probe_output.frontier.frontier_root_sha256.clone(),
        scores,
        selected_probe_root_sha256,
        direct_winner_root_sha256: String::new(),
    };
    winner.reseal()?;
    Ok(winner)
}

fn partition_sizes_v1(equal: [bool; 6]) -> Vec<u64> {
    let mut parent = [0_usize, 1, 2, 3];
    for ((left, right), same) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]
        .into_iter()
        .zip(equal)
    {
        if same {
            let left_root = find_root_v1(&mut parent, left);
            let right_root = find_root_v1(&mut parent, right);
            parent[right_root] = left_root;
        }
    }
    let mut sizes = BTreeMap::<usize, u64>::new();
    for model in 0..4 {
        let root = find_root_v1(&mut parent, model);
        *sizes.entry(root).or_default() += 1;
    }
    let mut values = sizes.into_values().collect::<Vec<_>>();
    values.sort_unstable_by(|left, right| right.cmp(left));
    values
}

fn find_root_v1(parent: &mut [usize; 4], value: usize) -> usize {
    if parent[value] != value {
        let next = parent[value];
        parent[value] = find_root_v1(parent, next);
    }
    parent[value]
}

fn compare_scores_v1(
    left: &K2UncertaintyDirectScoreV1,
    right: &K2UncertaintyDirectScoreV1,
) -> Ordering {
    right
        .minimax_eliminated
        .cmp(&left.minimax_eliminated)
        .then_with(|| right.pair_separation.cmp(&left.pair_separation))
        .then_with(|| left.risk_units.cmp(&right.risk_units))
        .then_with(|| left.cost_units.cmp(&right.cost_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

fn selector_tournament_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    selector_executable_sha256: &str,
    frontier_root_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
) -> K2CompositionResultV1<(Vec<K2UncertaintyTournamentStepV1>, String)> {
    let mut active = probes.keys().cloned().collect::<Vec<_>>();
    let mut eliminated = Vec::new();
    let mut steps = Vec::new();
    while active.len() > K2_UNCERTAINTY_SELECTOR_PROBES_V1 {
        let submitted = active[..K2_UNCERTAINTY_SELECTOR_PROBES_V1].to_vec();
        let request = selector_request_v1(
            public_case,
            learner_response,
            split_commitment_root_sha256,
            selector_executable_sha256,
            probes,
            &submitted,
        )?;
        let precommit = select_model_guided_probe_v1(&request)?;
        let retained = precommit.selected_probe_root_sha256.clone();
        let removed = submitted
            .iter()
            .filter(|root| *root != &retained)
            .cloned()
            .collect::<Vec<_>>();
        eliminated.extend(removed.iter().cloned());
        active.drain(..K2_UNCERTAINTY_SELECTOR_PROBES_V1);
        active.insert(0, retained.clone());
        let step = K2UncertaintyTournamentStepV1 {
            schema: K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1.to_owned(),
            case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
            frontier_root_sha256: frontier_root_sha256.to_owned(),
            step_sequence: steps.len() as u64,
            kind: K2UncertaintyTournamentStepKindV1::Reduction,
            active_probe_roots_sha256: submitted,
            filler_probe_roots_sha256: Vec::new(),
            request,
            precommit,
            retained_probe_root_sha256: retained,
            eliminated_probe_roots_sha256: removed,
            step_root_sha256: String::new(),
        };
        steps.push(step);
    }
    let filler_count = K2_UNCERTAINTY_SELECTOR_PROBES_V1 - active.len();
    let mut sorted_eliminated = eliminated.clone();
    sorted_eliminated.sort();
    let fillers = sorted_eliminated
        .into_iter()
        .take(filler_count)
        .collect::<Vec<_>>();
    if fillers.len() != filler_count {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_fillers_missing",
        ));
    }
    let mut final_roots = active.clone();
    final_roots.extend(fillers.iter().cloned());
    final_roots.sort();
    let request = selector_request_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        selector_executable_sha256,
        probes,
        &final_roots,
    )?;
    let precommit = select_model_guided_probe_v1(&request)?;
    let winner = precommit.selected_probe_root_sha256.clone();
    if !active.contains(&winner) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_tournament_filler_won",
        ));
    }
    let mut final_step = K2UncertaintyTournamentStepV1 {
        schema: K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1.to_owned(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
        frontier_root_sha256: String::new(),
        step_sequence: steps.len() as u64,
        kind: K2UncertaintyTournamentStepKindV1::Final,
        active_probe_roots_sha256: active,
        filler_probe_roots_sha256: fillers,
        request,
        precommit,
        retained_probe_root_sha256: winner.clone(),
        eliminated_probe_roots_sha256: Vec::new(),
        step_root_sha256: String::new(),
    };
    for step in &mut steps {
        step.reseal()?;
    }
    final_step.frontier_root_sha256 = frontier_root_sha256.to_owned();
    final_step.reseal()?;
    steps.push(final_step);
    Ok((steps, winner))
}

fn selector_request_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    selector_executable_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
    roots: &[String],
) -> K2CompositionResultV1<K2InquirySelectorRequestV1> {
    let case = adapted_public_case_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        probes,
        roots,
    )?;
    K2InquirySelectorRequestV1::seal(selector_executable_sha256.to_owned(), case)
}

fn adapted_public_case_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
    roots: &[String],
) -> K2CompositionResultV1<K2InquiryPublicCaseV1> {
    let selected = roots
        .iter()
        .map(|root| {
            probes
                .get(root)
                .cloned()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_tournament_probe_missing",
                ))
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    K2InquiryPublicCaseV1::seal(
        public_case.vocabulary.case_id_sha256.clone(),
        public_case.vocabulary.generator_schema_root_sha256.clone(),
        split_commitment_root_sha256.to_owned(),
        learner_response.world_models.clone(),
        selected,
    )
}

fn passive_baseline_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    baseline_executable_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
) -> K2CompositionResultV1<K2UncertaintyBaselineTournamentV1> {
    let roots = probes
        .keys()
        .take(K2_UNCERTAINTY_SELECTOR_PROBES_V1)
        .cloned()
        .collect::<Vec<_>>();
    let request = baseline_request_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        baseline_executable_sha256,
        probes,
        &roots,
    )?;
    let outcome = evaluate_inquiry_baselines_v1(&request)?;
    Ok(K2UncertaintyBaselineTournamentV1 {
        kind: K2InquiryBaselineKindV1::Passive,
        requests: vec![request],
        outcomes: vec![outcome],
        selected_probe_root_sha256: None,
    })
}

fn baseline_tournament_v1(
    kind: K2InquiryBaselineKindV1,
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    baseline_executable_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
) -> K2CompositionResultV1<K2UncertaintyBaselineTournamentV1> {
    let mut active = probes.keys().cloned().collect::<Vec<_>>();
    let mut eliminated = Vec::new();
    let mut requests = Vec::new();
    let mut outcomes = Vec::new();
    while active.len() > K2_UNCERTAINTY_SELECTOR_PROBES_V1 {
        let submitted = active[..K2_UNCERTAINTY_SELECTOR_PROBES_V1].to_vec();
        let request = baseline_request_v1(
            public_case,
            learner_response,
            split_commitment_root_sha256,
            baseline_executable_sha256,
            probes,
            &submitted,
        )?;
        let outcome = evaluate_inquiry_baselines_v1(&request)?;
        let retained = baseline_selected_v1(&outcome, kind)?;
        eliminated.extend(submitted.iter().filter(|root| *root != &retained).cloned());
        active.drain(..K2_UNCERTAINTY_SELECTOR_PROBES_V1);
        active.insert(0, retained);
        requests.push(request);
        outcomes.push(outcome);
    }
    let filler_count = K2_UNCERTAINTY_SELECTOR_PROBES_V1 - active.len();
    eliminated.sort();
    let fillers = eliminated
        .into_iter()
        .take(filler_count)
        .collect::<Vec<_>>();
    if fillers.len() != filler_count {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_baseline_fillers_missing",
        ));
    }
    let mut final_roots = active.clone();
    final_roots.extend(fillers);
    final_roots.sort();
    let request = baseline_request_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        baseline_executable_sha256,
        probes,
        &final_roots,
    )?;
    let outcome = evaluate_inquiry_baselines_v1(&request)?;
    let selected = baseline_selected_v1(&outcome, kind)?;
    if !active.contains(&selected) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_baseline_filler_won",
        ));
    }
    requests.push(request);
    outcomes.push(outcome);
    Ok(K2UncertaintyBaselineTournamentV1 {
        kind,
        requests,
        outcomes,
        selected_probe_root_sha256: Some(selected),
    })
}

fn baseline_request_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learner_response: &K2UncertaintyLearnerResponseV1,
    split_commitment_root_sha256: &str,
    baseline_executable_sha256: &str,
    probes: &BTreeMap<String, K2InquiryProbeV1>,
    roots: &[String],
) -> K2CompositionResultV1<K2InquiryBaselineRequestV1> {
    let case = adapted_public_case_v1(
        public_case,
        learner_response,
        split_commitment_root_sha256,
        probes,
        roots,
    )?;
    K2InquiryBaselineRequestV1::seal(baseline_executable_sha256.to_owned(), case)
}

fn baseline_selected_v1(
    outcome: &K2InquiryBaselinesV1,
    kind: K2InquiryBaselineKindV1,
) -> K2CompositionResultV1<String> {
    outcome
        .decisions
        .iter()
        .find(|decision| decision.kind == kind)
        .and_then(|decision| decision.selected_probe_root_sha256.clone())
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_baseline_selection_missing",
        ))
}
