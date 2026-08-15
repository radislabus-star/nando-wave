use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
    require_composition_root_v1,
};
use super::model::{
    K2_INQUIRY_BASELINES_SCHEMA_V1, K2_INQUIRY_EVALUATION_SCHEMA_V1, K2_INQUIRY_MAX_COST_UNITS_V1,
    K2_INQUIRY_MAX_PROTOCOL_BYTES_V1, K2_INQUIRY_MAX_RISK_UNITS_V1,
    K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1, K2_INQUIRY_PRECOMMIT_SCHEMA_V1,
    K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1, K2InquiryBaselineDecisionV1,
    K2InquiryBaselineKindV1, K2InquiryBaselineRequestV1, K2InquiryBaselineSurvivorsV1,
    K2InquiryBaselinesV1, K2InquiryEligibilityReasonV1, K2InquiryEligibilityV1,
    K2InquiryObservationModeV1, K2InquiryOutcomeVerificationReceiptV1,
    K2InquiryOutcomeVerificationRequestV1, K2InquiryPredictionV1, K2InquiryProbeEvaluationV1,
    K2InquiryProbeV1, K2InquiryPublicCaseV1, K2InquirySelectionPrecommitV1,
    K2InquirySelectionVerificationReceiptV1, K2InquirySelectorRequestV1,
    K2InquiryVerifierCommandV1, K2InquiryVerifierReceiptV1,
};

pub fn verify_inquiry_selection_v1(
    verifier_executable_sha256: String,
    selector_request: &K2InquirySelectorRequestV1,
    precommit: &K2InquirySelectionPrecommitV1,
) -> K2CompositionResultV1<K2InquirySelectionVerificationReceiptV1> {
    require_composition_root_v1(&verifier_executable_sha256)?;
    let expected = verifier_reconstruct_precommit_v1(selector_request)?;
    if &expected != precommit {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_selection_verification_mismatch",
        ));
    }
    let prediction_count = precommit
        .evaluations
        .iter()
        .map(|evaluation| evaluation.predictions.len() as u64)
        .sum();
    let mut receipt = K2InquirySelectionVerificationReceiptV1 {
        schema: K2_INQUIRY_SELECTION_VERIFICATION_SCHEMA_V1.to_owned(),
        verifier_executable_sha256,
        public_case_root_sha256: selector_request.public_case.case_root_sha256.clone(),
        precommit_root_sha256: precommit.precommit_root_sha256.clone(),
        selected_probe_root_sha256: precommit.selected_probe_root_sha256.clone(),
        prediction_count,
        selection_verified: true,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

pub fn verify_inquiry_outcome_v1(
    request: &K2InquiryOutcomeVerificationRequestV1,
) -> K2CompositionResultV1<K2InquiryOutcomeVerificationReceiptV1> {
    verifier_validate_outcome_request_v1(request)?;
    let expected_selection = verify_inquiry_selection_v1(
        request.verifier_executable_sha256.clone(),
        &request.selector_request,
        &request.precommit,
    )?;
    if expected_selection != request.selection_verification {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_selection_receipt_mismatch",
        ));
    }
    let expected_baselines = verifier_reconstruct_baselines_v1(&request.baseline_request)?;
    if expected_baselines != request.baselines {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_baseline_verification_mismatch",
        ));
    }

    let mut observation = request.observation.clone();
    observation.post_manifest.validate()?;
    observation.authority.validate()?;
    observation.reseal()?;
    if observation != request.observation
        || request.observation.selected_probe_root_sha256
            != request.precommit.selected_probe_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_observation_receipt_invalid",
        ));
    }

    let selected_evaluation = request
        .precommit
        .evaluations
        .iter()
        .find(|evaluation| {
            evaluation.probe_root_sha256 == request.precommit.selected_probe_root_sha256
        })
        .ok_or(K2CompositionErrorV1::Invalid(
            "inquiry_selected_evaluation_missing",
        ))?;
    if !selected_evaluation.eligibility.eligible {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_selected_probe_ineligible",
        ));
    }
    let surviving_model_roots_sha256 = selected_evaluation
        .predictions
        .iter()
        .filter(|prediction| {
            prediction.observable_outcome_root_sha256
                == request.observation.observable_outcome_root_sha256
        })
        .map(|prediction| prediction.model_root_sha256.clone())
        .collect::<Vec<_>>();
    if surviving_model_roots_sha256.is_empty() {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_observation_contradicts_all_models",
        ));
    }

    let true_model = request
        .selector_request
        .public_case
        .model(&request.private_true_model_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "inquiry_true_model_not_in_public_set",
        ))?;
    let mut baseline_survivors = Vec::new();
    for decision in &request.baselines.decisions {
        let (survivors, cost_units) = match &decision.selected_probe_root_sha256 {
            None => (request.selector_request.public_case.models.len() as u64, 0),
            Some(probe_root) => {
                let probe = request
                    .selector_request
                    .public_case
                    .probe(probe_root)
                    .ok_or(K2CompositionErrorV1::Invalid(
                        "inquiry_baseline_probe_missing",
                    ))?;
                let outcome = verifier_predict_v1(true_model, probe)?;
                let evaluation =
                    verifier_evaluate_probe_v1(&request.selector_request.public_case, probe)?;
                let survivors = evaluation
                    .predictions
                    .iter()
                    .filter(|prediction| {
                        prediction.observable_outcome_root_sha256
                            == outcome.observable_outcome_root_sha256
                    })
                    .count() as u64;
                (survivors, probe.cost_units)
            }
        };
        let result_root_sha256 = composition_root_v1(&(
            "nando.k2-inquiry-baseline-survivors.v1",
            decision.kind,
            &decision.selected_probe_root_sha256,
            survivors,
            cost_units,
        ))?;
        baseline_survivors.push(K2InquiryBaselineSurvivorsV1 {
            kind: decision.kind,
            selected_probe_root_sha256: decision.selected_probe_root_sha256.clone(),
            survivors,
            cost_units,
            result_root_sha256,
        });
    }
    baseline_survivors.sort_by_key(|baseline| baseline.kind);

    let mut oracle = request
        .selector_request
        .public_case
        .probes
        .iter()
        .filter(|probe| {
            verifier_probe_eligibility_v1(&request.selector_request.public_case, probe)
                .is_ok_and(|eligibility| eligibility.eligible)
        })
        .map(|probe| {
            let outcome = verifier_predict_v1(true_model, probe)?;
            let evaluation =
                verifier_evaluate_probe_v1(&request.selector_request.public_case, probe)?;
            let survivors = evaluation
                .predictions
                .iter()
                .filter(|prediction| {
                    prediction.observable_outcome_root_sha256
                        == outcome.observable_outcome_root_sha256
                })
                .count() as u64;
            Ok((probe, survivors))
        })
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    oracle.sort_by(
        |(left_probe, left_survivors), (right_probe, right_survivors)| {
            left_survivors
                .cmp(right_survivors)
                .then_with(|| left_probe.cost_units.cmp(&right_probe.cost_units))
                .then_with(|| {
                    left_probe
                        .probe_root_sha256
                        .cmp(&right_probe.probe_root_sha256)
                })
        },
    );
    let (oracle_probe, oracle_survivors) = oracle.first().ok_or(K2CompositionErrorV1::Invalid(
        "inquiry_oracle_no_eligible_probe",
    ))?;
    let complete_prediction_count = request
        .precommit
        .evaluations
        .iter()
        .map(|evaluation| evaluation.predictions.len() as u64)
        .sum();
    let mut receipt = K2InquiryOutcomeVerificationReceiptV1 {
        schema: super::model::K2_INQUIRY_OUTCOME_VERIFICATION_SCHEMA_V1.to_owned(),
        verifier_executable_sha256: request.verifier_executable_sha256.clone(),
        verification_request_root_sha256: request.request_root_sha256.clone(),
        public_case_root_sha256: request
            .selector_request
            .public_case
            .case_root_sha256
            .clone(),
        selected_probe_root_sha256: request.precommit.selected_probe_root_sha256.clone(),
        surviving_model_roots_sha256,
        baseline_survivors,
        oracle_probe_root_sha256: oracle_probe.probe_root_sha256.clone(),
        oracle_survivors: *oracle_survivors,
        selector_matches_oracle: request.precommit.selected_probe_root_sha256
            == oracle_probe.probe_root_sha256,
        complete_prediction_count,
        forbidden_probe_executions: 0,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.reseal()?;
    Ok(receipt)
}

pub fn run_inquiry_verifier_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_INQUIRY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_inquiry_verifier_stdin"))?;
    let command: K2InquiryVerifierCommandV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_inquiry_verifier"))?;
    let executable_sha256 = composition_sha256_file_v1(&executable)?;
    let receipt = match command {
        K2InquiryVerifierCommandV1::VerifySelection {
            verifier_executable_sha256,
            selector_request,
            precommit,
        } => {
            if executable_sha256 != verifier_executable_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "inquiry_verifier_executable_mismatch",
                ));
            }
            K2InquiryVerifierReceiptV1::Selection {
                value: verify_inquiry_selection_v1(
                    verifier_executable_sha256,
                    &selector_request,
                    &precommit,
                )?,
            }
        }
        K2InquiryVerifierCommandV1::VerifyOutcome { request } => {
            if executable_sha256 != request.verifier_executable_sha256 {
                return Err(K2CompositionErrorV1::Invalid(
                    "inquiry_verifier_executable_mismatch",
                ));
            }
            K2InquiryVerifierReceiptV1::Outcome {
                value: verify_inquiry_outcome_v1(&request)?,
            }
        }
    };
    std::io::stdout()
        .write_all(&composition_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_verifier_stdout"))
}

fn verifier_validate_outcome_request_v1(
    request: &K2InquiryOutcomeVerificationRequestV1,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&request.verifier_executable_sha256)?;
    require_composition_root_v1(&request.private_true_model_root_sha256)?;
    request.selector_request.validate()?;
    request.baseline_request.validate()?;
    request.authority.validate()?;
    if request.selector_request.public_case != request.baseline_request.public_case {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_verifier_public_input_mismatch",
        ));
    }
    let expected = composition_root_v1(&(
        K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1,
        &request.verifier_executable_sha256,
        &request.selector_request,
        &request.precommit,
        &request.selection_verification,
        &request.baseline_request,
        &request.baselines,
        &request.observation,
        &request.private_true_model_root_sha256,
        &request.authority,
    ))?;
    if request.schema != K2_INQUIRY_OUTCOME_VERIFICATION_REQUEST_SCHEMA_V1
        || expected != request.request_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_outcome_verification_request_invalid",
        ));
    }
    Ok(())
}

fn verifier_reconstruct_precommit_v1(
    request: &K2InquirySelectorRequestV1,
) -> K2CompositionResultV1<K2InquirySelectionPrecommitV1> {
    request.validate()?;
    let mut evaluations = request
        .public_case
        .probes
        .iter()
        .map(|probe| verifier_evaluate_probe_v1(&request.public_case, probe))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    evaluations.sort_by(|left, right| left.probe_root_sha256.cmp(&right.probe_root_sha256));
    let mut eligible = evaluations
        .iter()
        .filter(|evaluation| evaluation.eligibility.eligible)
        .collect::<Vec<_>>();
    eligible
        .sort_by(|left, right| verifier_compare_evaluations_v1(&request.public_case, left, right));
    let selected = eligible.first().ok_or(K2CompositionErrorV1::Invalid(
        "inquiry_verifier_no_eligible_probe",
    ))?;
    let selected_probe = request
        .public_case
        .probe(&selected.probe_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "inquiry_verifier_selected_probe_missing",
        ))?;
    let exact_best_ties = eligible
        .iter()
        .filter(|candidate| {
            let probe = request
                .public_case
                .probe(&candidate.probe_root_sha256)
                .expect("validated verifier probe");
            candidate.minimax_eliminated == selected.minimax_eliminated
                && candidate.pair_separation == selected.pair_separation
                && probe.risk_units == selected_probe.risk_units
                && probe.cost_units == selected_probe.cost_units
        })
        .count() as u64;
    let selected_probe_root_sha256 = selected.probe_root_sha256.clone();
    drop(eligible);
    let mut precommit = K2InquirySelectionPrecommitV1 {
        schema: K2_INQUIRY_PRECOMMIT_SCHEMA_V1.to_owned(),
        selector_request_root_sha256: request.request_root_sha256.clone(),
        public_case_root_sha256: request.public_case.case_root_sha256.clone(),
        evaluations,
        selected_probe_root_sha256,
        exact_best_ties,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        precommit_root_sha256: String::new(),
    };
    precommit.reseal()?;
    Ok(precommit)
}

fn verifier_evaluate_probe_v1(
    case: &K2InquiryPublicCaseV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<K2InquiryProbeEvaluationV1> {
    let eligibility = verifier_probe_eligibility_v1(case, probe)?;
    let mut predictions = case
        .models
        .iter()
        .map(|model| verifier_predict_v1(model, probe))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    predictions.sort_by(|left, right| left.model_root_sha256.cmp(&right.model_root_sha256));
    let mut groups = BTreeMap::<String, u64>::new();
    for prediction in &predictions {
        *groups
            .entry(prediction.observable_outcome_root_sha256.clone())
            .or_default() += 1;
    }
    let mut partition_sizes = groups.into_values().collect::<Vec<_>>();
    partition_sizes.sort_unstable_by(|left, right| right.cmp(left));
    let largest_partition = partition_sizes.first().copied().unwrap_or_default();
    let model_count = case.models.len() as u64;
    let minimax_eliminated = model_count.saturating_sub(largest_partition);
    let pair_separation = model_count.saturating_mul(model_count).saturating_sub(
        partition_sizes
            .iter()
            .map(|size| size.saturating_mul(*size))
            .sum(),
    );
    let mut evaluation = K2InquiryProbeEvaluationV1 {
        schema: K2_INQUIRY_EVALUATION_SCHEMA_V1.to_owned(),
        probe_root_sha256: probe.probe_root_sha256.clone(),
        eligibility,
        predictions,
        partition_sizes,
        largest_partition,
        minimax_eliminated,
        pair_separation,
        evaluation_root_sha256: String::new(),
    };
    evaluation.reseal()?;
    Ok(evaluation)
}

fn verifier_predict_v1(
    model: &super::model::K2InquiryWorldModelV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<K2InquiryPredictionV1> {
    let (applied, reason, post) = match model.effect(&probe.action_id_sha256) {
        Some(effect) => verifier_apply_effect_v1(&probe.initial_manifest, effect)?,
        None => (
            false,
            "unknown_action".to_owned(),
            probe.initial_manifest.clone(),
        ),
    };
    K2InquiryPredictionV1::seal(
        model.model_root_sha256.clone(),
        probe.probe_root_sha256.clone(),
        applied,
        reason,
        post,
        probe.observation_mode,
    )
}

fn verifier_probe_eligibility_v1(
    case: &K2InquiryPublicCaseV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<K2InquiryEligibilityV1> {
    let reason = if !probe.reversible {
        K2InquiryEligibilityReasonV1::NonReversible
    } else if probe.observation_mode == K2InquiryObservationModeV1::Ambiguous {
        K2InquiryEligibilityReasonV1::AmbiguousObservation
    } else if probe.observation_mode == K2InquiryObservationModeV1::Delayed {
        K2InquiryEligibilityReasonV1::DelayedObservation
    } else if case
        .models
        .iter()
        .any(|model| model.effect(&probe.action_id_sha256).is_none())
    {
        K2InquiryEligibilityReasonV1::UnknownAction
    } else if probe.risk_units > K2_INQUIRY_MAX_RISK_UNITS_V1 {
        K2InquiryEligibilityReasonV1::RiskBudgetExceeded
    } else if probe.cost_units > K2_INQUIRY_MAX_COST_UNITS_V1 {
        K2InquiryEligibilityReasonV1::CostBudgetExceeded
    } else {
        K2InquiryEligibilityReasonV1::Eligible
    };
    K2InquiryEligibilityV1::seal(reason)
}

fn verifier_apply_effect_v1(
    manifest: &K2CompositionTreeManifestV1,
    effect: &K2CompositionLearnedEffectV1,
) -> K2CompositionResultV1<(bool, String, K2CompositionTreeManifestV1)> {
    let mut entries = manifest
        .entries
        .iter()
        .cloned()
        .map(|entry| (entry.path.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let (applied, reason) = match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => match entries.get(source_path).cloned() {
            Some(source) => {
                entries.insert(
                    target_path.clone(),
                    K2CompositionFileEntryV1 {
                        path: target_path.clone(),
                        content_sha256: source.content_sha256,
                        byte_len: source.byte_len,
                    },
                );
                (true, "applied".to_owned())
            }
            None => (false, "copy_source_missing".to_owned()),
        },
        K2CompositionLearnedEffectV1::RemoveFile { path } => {
            if entries.remove(path).is_some() {
                (true, "applied".to_owned())
            } else {
                (false, "remove_path_missing".to_owned())
            }
        }
    };
    let post = K2CompositionTreeManifestV1::seal_entries(entries.into_values().collect())?;
    Ok((applied, reason, post))
}

fn verifier_compare_evaluations_v1(
    case: &K2InquiryPublicCaseV1,
    left: &K2InquiryProbeEvaluationV1,
    right: &K2InquiryProbeEvaluationV1,
) -> Ordering {
    let left_probe = case
        .probe(&left.probe_root_sha256)
        .expect("validated left verifier probe");
    let right_probe = case
        .probe(&right.probe_root_sha256)
        .expect("validated right verifier probe");
    right
        .minimax_eliminated
        .cmp(&left.minimax_eliminated)
        .then_with(|| right.pair_separation.cmp(&left.pair_separation))
        .then_with(|| left_probe.risk_units.cmp(&right_probe.risk_units))
        .then_with(|| left_probe.cost_units.cmp(&right_probe.cost_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

fn verifier_reconstruct_baselines_v1(
    request: &K2InquiryBaselineRequestV1,
) -> K2CompositionResultV1<K2InquiryBaselinesV1> {
    request.validate()?;
    let mut eligible = request
        .public_case
        .probes
        .iter()
        .filter(|probe| {
            verifier_probe_eligibility_v1(&request.public_case, probe)
                .is_ok_and(|eligibility| eligibility.eligible)
        })
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_verifier_no_baseline_probe",
        ));
    }
    let stable_probe_root_sha256 = eligible
        .iter()
        .min_by_key(|probe| &probe.probe_root_sha256)
        .expect("nonempty verifier baseline probes")
        .probe_root_sha256
        .clone();
    eligible.sort_by(|left, right| {
        left.cost_units
            .cmp(&right.cost_units)
            .then_with(|| left.risk_units.cmp(&right.risk_units))
            .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
    });
    let cheapest_probe_root_sha256 = eligible[0].probe_root_sha256.clone();
    eligible.sort_by(|left, right| {
        verifier_heuristic_score_v1(right)
            .cmp(&verifier_heuristic_score_v1(left))
            .then_with(|| left.risk_units.cmp(&right.risk_units))
            .then_with(|| left.cost_units.cmp(&right.cost_units))
            .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
    });
    let heuristic_probe_root_sha256 = eligible[0].probe_root_sha256.clone();
    let mut decisions = vec![
        verifier_baseline_decision_v1(K2InquiryBaselineKindV1::Passive, None)?,
        verifier_baseline_decision_v1(
            K2InquiryBaselineKindV1::StableHash,
            Some(stable_probe_root_sha256),
        )?,
        verifier_baseline_decision_v1(
            K2InquiryBaselineKindV1::CheapestFirst,
            Some(cheapest_probe_root_sha256),
        )?,
        verifier_baseline_decision_v1(
            K2InquiryBaselineKindV1::ExplicitHeuristic,
            Some(heuristic_probe_root_sha256),
        )?,
    ];
    decisions.sort_by_key(|decision| decision.kind);
    let mut baselines = K2InquiryBaselinesV1 {
        schema: K2_INQUIRY_BASELINES_SCHEMA_V1.to_owned(),
        baseline_request_root_sha256: request.request_root_sha256.clone(),
        public_case_root_sha256: request.public_case.case_root_sha256.clone(),
        decisions,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        baselines_root_sha256: String::new(),
    };
    baselines.reseal()?;
    Ok(baselines)
}

fn verifier_baseline_decision_v1(
    kind: K2InquiryBaselineKindV1,
    selected_probe_root_sha256: Option<String>,
) -> K2CompositionResultV1<K2InquiryBaselineDecisionV1> {
    let decision_root_sha256 = composition_root_v1(&(
        "nando.k2-inquiry-baseline-decision.v1",
        kind,
        &selected_probe_root_sha256,
    ))?;
    Ok(K2InquiryBaselineDecisionV1 {
        kind,
        selected_probe_root_sha256,
        decision_root_sha256,
    })
}

fn verifier_heuristic_score_v1(probe: &K2InquiryProbeV1) -> u64 {
    u64::from(probe.applicability_hint) * 4
        + u64::from(probe.dependency_hint) * 2
        + u64::from(probe.cleanup_hint)
}
