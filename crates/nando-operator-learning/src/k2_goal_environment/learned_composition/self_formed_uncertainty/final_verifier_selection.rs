use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::super::{
    K2_INQUIRY_EVALUATION_SCHEMA_V1, K2_INQUIRY_PRECOMMIT_SCHEMA_V1,
    K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1, K2CompositionAuthorityBoundaryV1,
    K2CompositionErrorV1, K2CompositionResultV1, K2InquiryBaselineDecisionV1,
    K2InquiryBaselineKindV1, K2InquiryEligibilityReasonV1, K2InquiryEligibilityV1,
    K2InquiryPredictionV1, K2InquiryProbeEvaluationV1, K2InquiryPublicCaseV1,
    K2InquirySelectionPrecommitV1, K2InquirySelectionVerificationReceiptV1,
    K2InquirySelectorRequestV1,
};
use super::final_verifier_frontier::IndependentFrontierV1;
use super::{
    K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1, K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1,
    K2_UNCERTAINTY_SELECTOR_PROBES_V1, K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1,
    K2UncertaintyBaselineSummaryV1, K2UncertaintyCasePreverificationV1, K2UncertaintyDirectScoreV1,
    K2UncertaintyDirectWinnerV1, K2UncertaintyLearnerResponseV1, K2UncertaintyPublicCaseV1,
    K2UncertaintyRawProbeDispositionV1, K2UncertaintyTournamentStepKindV1,
    K2UncertaintyTournamentStepV1, denied_authority_v1, uncertainty_root_v1,
};

pub(super) struct IndependentSelectionCountsV1 {
    pub representative_count: u64,
    pub tournament_requests: u64,
    pub adapted_predictions: u64,
}

pub(super) fn verify_selection_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
    frontier: &IndependentFrontierV1,
    frontier_root_sha256: &str,
    split_commitment_root_sha256: &str,
    preverification: &K2UncertaintyCasePreverificationV1,
) -> K2CompositionResultV1<IndependentSelectionCountsV1> {
    let direct = independent_direct_winner_v1(public_case, frontier_root_sha256, frontier)?;
    if direct != preverification.tournament.direct_winner {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_direct_winner_mismatch",
        ));
    }
    let steps = independent_tournament_v1(
        public_case,
        learned,
        frontier,
        frontier_root_sha256,
        split_commitment_root_sha256,
        &preverification.tournament.selector_executable_sha256,
    )?;
    let step_roots = steps
        .iter()
        .map(|step| step.step_root_sha256.clone())
        .collect::<Vec<_>>();
    let winner = steps
        .last()
        .map(|step| step.retained_probe_root_sha256.as_str())
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_tournament_empty",
        ))?;
    if step_roots != preverification.tournament.step_roots_sha256
        || winner
            != preverification
                .tournament
                .tournament_winner_probe_root_sha256
        || winner != direct.selected_probe_root_sha256
        || steps.len() as u64 != preverification.tournament.request_count
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_tournament_mismatch",
        ));
    }
    let expected_receipts = steps
        .iter()
        .map(|step| {
            let mut receipt = K2InquirySelectionVerificationReceiptV1 {
                schema: K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1.to_owned(),
                verifier_executable_sha256: preverification
                    .selection_verifier_executable_sha256
                    .clone(),
                public_case_root_sha256: step.request.public_case.case_root_sha256.clone(),
                precommit_root_sha256: step.precommit.precommit_root_sha256.clone(),
                selected_probe_root_sha256: step.retained_probe_root_sha256.clone(),
                prediction_count: (K2_UNCERTAINTY_SELECTOR_PROBES_V1 * 4) as u64,
                selection_verified: true,
                authority: denied_authority_v1(),
                receipt_root_sha256: String::new(),
            };
            receipt.reseal()?;
            Ok(receipt)
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    if expected_receipts != preverification.step_verifications {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_preverification_mismatch",
        ));
    }
    let baseline =
        independent_baseline_summary_v1(public_case, frontier, &preverification.baseline_summary)?;
    if baseline != preverification.baseline_summary {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_baseline_mismatch",
        ));
    }
    let adapted_predictions = (steps.len() * K2_UNCERTAINTY_SELECTOR_PROBES_V1 * 4) as u64;
    if adapted_predictions != preverification.tournament.adapted_prediction_count {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_final_adapted_prediction_count_mismatch",
        ));
    }
    Ok(IndependentSelectionCountsV1 {
        representative_count: frontier.representatives.len() as u64,
        tournament_requests: steps.len() as u64,
        adapted_predictions,
    })
}

fn independent_direct_winner_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    frontier_root_sha256: &str,
    frontier: &IndependentFrontierV1,
) -> K2CompositionResultV1<K2UncertaintyDirectWinnerV1> {
    let mut scores = frontier
        .representatives
        .values()
        .map(independent_direct_score_v1)
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    scores.sort_by(|left, right| left.probe_root_sha256.cmp(&right.probe_root_sha256));
    let selected = scores
        .iter()
        .filter(|score| score.eligible)
        .min_by(|left, right| compare_direct_scores_v1(left, right))
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_final_direct_winner_missing",
        ))?
        .probe_root_sha256
        .clone();
    let mut value = K2UncertaintyDirectWinnerV1 {
        schema: K2_UNCERTAINTY_DIRECT_WINNER_SCHEMA_V1.to_owned(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
        frontier_root_sha256: frontier_root_sha256.to_owned(),
        scores,
        selected_probe_root_sha256: selected,
        direct_winner_root_sha256: String::new(),
    };
    value.reseal()?;
    Ok(value)
}

fn independent_direct_score_v1(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2UncertaintyDirectScoreV1> {
    let partition_sizes = partition_sizes_v1(disposition.equivalence_key.pairwise_outcome_equal);
    let largest = partition_sizes.first().copied().unwrap_or_default();
    let pair_separation = 16_u64.saturating_sub(
        partition_sizes
            .iter()
            .map(|size| size.saturating_mul(*size))
            .sum(),
    );
    let mut score = K2UncertaintyDirectScoreV1 {
        schema: K2_UNCERTAINTY_DIRECT_SCORE_SCHEMA_V1.to_owned(),
        probe_root_sha256: disposition.probe.probe_root_sha256.clone(),
        eligible: disposition.probe.reversible
            && disposition.probe.observation_mode
                == super::super::K2InquiryObservationModeV1::ExactImmediate
            && disposition.probe.risk_units <= 10
            && disposition.probe.cost_units <= 10,
        minimax_eliminated: 4_u64.saturating_sub(largest),
        pair_separation,
        risk_units: disposition.probe.risk_units,
        cost_units: disposition.probe.cost_units,
        score_root_sha256: String::new(),
    };
    score.reseal()?;
    Ok(score)
}

fn independent_tournament_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
    frontier: &IndependentFrontierV1,
    frontier_root_sha256: &str,
    split_commitment_root_sha256: &str,
    selector_executable_sha256: &str,
) -> K2CompositionResultV1<Vec<K2UncertaintyTournamentStepV1>> {
    let mut active = frontier.representatives.keys().cloned().collect::<Vec<_>>();
    let mut eliminated = Vec::new();
    let mut steps = Vec::new();
    while active.len() > K2_UNCERTAINTY_SELECTOR_PROBES_V1 {
        let submitted = active[..K2_UNCERTAINTY_SELECTOR_PROBES_V1].to_vec();
        let (request, precommit) = independent_precommit_v1(
            public_case,
            learned,
            frontier,
            split_commitment_root_sha256,
            selector_executable_sha256,
            &submitted,
        )?;
        let retained = precommit.selected_probe_root_sha256.clone();
        let removed = submitted
            .iter()
            .filter(|root| *root != &retained)
            .cloned()
            .collect::<Vec<_>>();
        eliminated.extend(removed.iter().cloned());
        active.drain(..K2_UNCERTAINTY_SELECTOR_PROBES_V1);
        active.insert(0, retained.clone());
        let mut step = K2UncertaintyTournamentStepV1 {
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
        step.reseal()?;
        steps.push(step);
    }
    eliminated.sort();
    let fillers = eliminated
        .into_iter()
        .take(K2_UNCERTAINTY_SELECTOR_PROBES_V1 - active.len())
        .collect::<Vec<_>>();
    let mut submitted = active.clone();
    submitted.extend(fillers.iter().cloned());
    submitted.sort();
    let (request, precommit) = independent_precommit_v1(
        public_case,
        learned,
        frontier,
        split_commitment_root_sha256,
        selector_executable_sha256,
        &submitted,
    )?;
    let retained = precommit.selected_probe_root_sha256.clone();
    let mut step = K2UncertaintyTournamentStepV1 {
        schema: K2_UNCERTAINTY_TOURNAMENT_STEP_SCHEMA_V1.to_owned(),
        case_id_sha256: public_case.vocabulary.case_id_sha256.clone(),
        frontier_root_sha256: frontier_root_sha256.to_owned(),
        step_sequence: steps.len() as u64,
        kind: K2UncertaintyTournamentStepKindV1::Final,
        active_probe_roots_sha256: active,
        filler_probe_roots_sha256: fillers,
        request,
        precommit,
        retained_probe_root_sha256: retained,
        eliminated_probe_roots_sha256: Vec::new(),
        step_root_sha256: String::new(),
    };
    step.reseal()?;
    steps.push(step);
    Ok(steps)
}

fn independent_precommit_v1(
    public_case: &K2UncertaintyPublicCaseV1,
    learned: &K2UncertaintyLearnerResponseV1,
    frontier: &IndependentFrontierV1,
    split_commitment_root_sha256: &str,
    selector_executable_sha256: &str,
    roots: &[String],
) -> K2CompositionResultV1<(K2InquirySelectorRequestV1, K2InquirySelectionPrecommitV1)> {
    let probes = roots
        .iter()
        .map(|root| {
            frontier
                .representatives
                .get(root)
                .map(|disposition| disposition.probe.clone())
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_final_tournament_probe_missing",
                ))
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let case = K2InquiryPublicCaseV1::seal(
        public_case.vocabulary.case_id_sha256.clone(),
        public_case.vocabulary.generator_schema_root_sha256.clone(),
        split_commitment_root_sha256.to_owned(),
        learned.world_models.clone(),
        probes,
    )?;
    let request = K2InquirySelectorRequestV1::seal(selector_executable_sha256.to_owned(), case)?;
    let mut evaluations = roots
        .iter()
        .map(|root| {
            independent_evaluation_v1(
                frontier
                    .representatives
                    .get(root)
                    .expect("checked representative"),
            )
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    evaluations.sort_by(|left, right| left.probe_root_sha256.cmp(&right.probe_root_sha256));
    let mut eligible = evaluations
        .iter()
        .filter(|evaluation| evaluation.eligibility.eligible)
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        let left_probe = request
            .public_case
            .probe(&left.probe_root_sha256)
            .expect("probe");
        let right_probe = request
            .public_case
            .probe(&right.probe_root_sha256)
            .expect("probe");
        right
            .minimax_eliminated
            .cmp(&left.minimax_eliminated)
            .then_with(|| right.pair_separation.cmp(&left.pair_separation))
            .then_with(|| left_probe.risk_units.cmp(&right_probe.risk_units))
            .then_with(|| left_probe.cost_units.cmp(&right_probe.cost_units))
            .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
    });
    let selected = eligible.first().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_final_no_eligible_probe",
    ))?;
    let selected_probe = request
        .public_case
        .probe(&selected.probe_root_sha256)
        .expect("selected probe");
    let exact_best_ties = eligible
        .iter()
        .filter(|candidate| {
            let probe = request
                .public_case
                .probe(&candidate.probe_root_sha256)
                .expect("candidate probe");
            candidate.minimax_eliminated == selected.minimax_eliminated
                && candidate.pair_separation == selected.pair_separation
                && probe.risk_units == selected_probe.risk_units
                && probe.cost_units == selected_probe.cost_units
        })
        .count() as u64;
    let selected_root = selected.probe_root_sha256.clone();
    drop(eligible);
    let mut precommit = K2InquirySelectionPrecommitV1 {
        schema: K2_INQUIRY_PRECOMMIT_SCHEMA_V1.to_owned(),
        selector_request_root_sha256: request.request_root_sha256.clone(),
        public_case_root_sha256: request.public_case.case_root_sha256.clone(),
        evaluations,
        selected_probe_root_sha256: selected_root,
        exact_best_ties,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        precommit_root_sha256: String::new(),
    };
    precommit.reseal()?;
    Ok((request, precommit))
}

fn independent_evaluation_v1(
    disposition: &K2UncertaintyRawProbeDispositionV1,
) -> K2CompositionResultV1<K2InquiryProbeEvaluationV1> {
    let mut predictions = disposition
        .predictions
        .iter()
        .map(|prediction| {
            K2InquiryPredictionV1::seal(
                prediction.model_root_sha256.clone(),
                prediction.probe_root_sha256.clone(),
                prediction.transition_applied,
                prediction.transition_reason.clone(),
                prediction.predicted_post_manifest.clone(),
                disposition.probe.observation_mode,
            )
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    predictions.sort_by(|left, right| left.model_root_sha256.cmp(&right.model_root_sha256));
    let mut groups = BTreeMap::<String, u64>::new();
    for prediction in &predictions {
        *groups
            .entry(prediction.observable_outcome_root_sha256.clone())
            .or_default() += 1;
    }
    let mut partitions = groups.into_values().collect::<Vec<_>>();
    partitions.sort_unstable_by(|left, right| right.cmp(left));
    let largest = partitions.first().copied().unwrap_or_default();
    let mut evaluation = K2InquiryProbeEvaluationV1 {
        schema: K2_INQUIRY_EVALUATION_SCHEMA_V1.to_owned(),
        probe_root_sha256: disposition.probe.probe_root_sha256.clone(),
        eligibility: K2InquiryEligibilityV1::seal(K2InquiryEligibilityReasonV1::Eligible)?,
        predictions,
        partition_sizes: partitions.clone(),
        largest_partition: largest,
        minimax_eliminated: 4_u64.saturating_sub(largest),
        pair_separation: 16_u64.saturating_sub(
            partitions
                .iter()
                .map(|size| size.saturating_mul(*size))
                .sum(),
        ),
        evaluation_root_sha256: String::new(),
    };
    evaluation.reseal()?;
    Ok(evaluation)
}

fn independent_baseline_summary_v1(
    _public_case: &K2UncertaintyPublicCaseV1,
    frontier: &IndependentFrontierV1,
    observed: &K2UncertaintyBaselineSummaryV1,
) -> K2CompositionResultV1<K2UncertaintyBaselineSummaryV1> {
    let probes = frontier
        .representatives
        .values()
        .map(|disposition| &disposition.probe)
        .collect::<Vec<_>>();
    let stable = probes
        .iter()
        .min_by_key(|probe| &probe.probe_root_sha256)
        .expect("nonempty frontier");
    let cheapest = probes
        .iter()
        .min_by(|left, right| {
            left.cost_units
                .cmp(&right.cost_units)
                .then_with(|| left.risk_units.cmp(&right.risk_units))
                .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
        })
        .expect("nonempty frontier");
    let heuristic = probes
        .iter()
        .min_by(|left, right| {
            heuristic_score_v1(right)
                .cmp(&heuristic_score_v1(left))
                .then_with(|| left.risk_units.cmp(&right.risk_units))
                .then_with(|| left.cost_units.cmp(&right.cost_units))
                .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
        })
        .expect("nonempty frontier");
    let mut decisions = vec![
        baseline_decision_v1(K2InquiryBaselineKindV1::Passive, None)?,
        baseline_decision_v1(
            K2InquiryBaselineKindV1::StableHash,
            Some(stable.probe_root_sha256.clone()),
        )?,
        baseline_decision_v1(
            K2InquiryBaselineKindV1::CheapestFirst,
            Some(cheapest.probe_root_sha256.clone()),
        )?,
        baseline_decision_v1(
            K2InquiryBaselineKindV1::ExplicitHeuristic,
            Some(heuristic.probe_root_sha256.clone()),
        )?,
    ];
    decisions.sort_by_key(|decision| decision.kind);
    let mut summary = K2UncertaintyBaselineSummaryV1 {
        schema: super::K2_UNCERTAINTY_BASELINE_SUMMARY_SCHEMA_V1.to_owned(),
        case_id_sha256: observed.case_id_sha256.clone(),
        frontier_root_sha256: observed.frontier_root_sha256.clone(),
        baseline_source_sha256: observed.baseline_source_sha256.clone(),
        baseline_executable_sha256: observed.baseline_executable_sha256.clone(),
        decisions,
        authority: denied_authority_v1(),
        summary_root_sha256: String::new(),
    };
    summary.reseal()?;
    Ok(summary)
}

fn baseline_decision_v1(
    kind: K2InquiryBaselineKindV1,
    selected_probe_root_sha256: Option<String>,
) -> K2CompositionResultV1<K2InquiryBaselineDecisionV1> {
    Ok(K2InquiryBaselineDecisionV1 {
        kind,
        decision_root_sha256: uncertainty_root_v1(&(
            "nando.k2-inquiry-baseline-decision.v1",
            kind,
            &selected_probe_root_sha256,
        ))?,
        selected_probe_root_sha256,
    })
}

fn heuristic_score_v1(probe: &super::super::K2InquiryProbeV1) -> u64 {
    u64::from(probe.applicability_hint) * 4
        + u64::from(probe.dependency_hint) * 2
        + u64::from(probe.cleanup_hint)
}

fn compare_direct_scores_v1(
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
        parent[value] = find_root_v1(parent, parent[value]);
    }
    parent[value]
}
