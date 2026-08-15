use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionFileEntryV1,
    K2CompositionLearnedEffectV1, K2CompositionResultV1, K2CompositionTreeManifestV1,
    composition_bytes_v1, composition_decode_v1, composition_sha256_file_v1,
};
use super::model::{
    K2_INQUIRY_MAX_COST_UNITS_V1, K2_INQUIRY_MAX_PROTOCOL_BYTES_V1, K2_INQUIRY_MAX_RISK_UNITS_V1,
    K2_INQUIRY_PRECOMMIT_SCHEMA_V1, K2InquiryEligibilityReasonV1, K2InquiryEligibilityV1,
    K2InquiryPredictionV1, K2InquiryProbeEvaluationV1, K2InquiryProbeV1, K2InquiryPublicCaseV1,
    K2InquirySelectionPrecommitV1, K2InquirySelectorRequestV1,
};

pub fn select_model_guided_probe_v1(
    request: &K2InquirySelectorRequestV1,
) -> K2CompositionResultV1<K2InquirySelectionPrecommitV1> {
    request.validate()?;
    let mut evaluations = request
        .public_case
        .probes
        .iter()
        .map(|probe| selector_evaluate_probe_v1(&request.public_case, probe))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    evaluations.sort_by(|left, right| left.probe_root_sha256.cmp(&right.probe_root_sha256));

    let mut eligible = evaluations
        .iter()
        .filter(|evaluation| evaluation.eligibility.eligible)
        .collect::<Vec<_>>();
    eligible
        .sort_by(|left, right| selector_compare_evaluations_v1(&request.public_case, left, right));
    let selected = eligible
        .first()
        .ok_or(K2CompositionErrorV1::Invalid("inquiry_no_eligible_probe"))?;
    let selected_probe = request
        .public_case
        .probe(&selected.probe_root_sha256)
        .ok_or(K2CompositionErrorV1::Invalid(
            "inquiry_selected_probe_missing",
        ))?;
    let exact_best_ties = eligible
        .iter()
        .filter(|candidate| {
            let probe = request
                .public_case
                .probe(&candidate.probe_root_sha256)
                .expect("validated evaluation probe");
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

pub fn run_inquiry_selector_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_INQUIRY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_inquiry_selector_stdin"))?;
    let request: K2InquirySelectorRequestV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_inquiry_selector"))?;
    if composition_sha256_file_v1(&executable)? != request.selector_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_selector_executable_mismatch",
        ));
    }
    let precommit = select_model_guided_probe_v1(&request)?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&precommit)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_selector_stdout"))
}

pub(crate) fn selector_probe_eligibility_v1(
    case: &K2InquiryPublicCaseV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<K2InquiryEligibilityV1> {
    let reason = if !probe.reversible {
        K2InquiryEligibilityReasonV1::NonReversible
    } else if probe.observation_mode == super::model::K2InquiryObservationModeV1::Ambiguous {
        K2InquiryEligibilityReasonV1::AmbiguousObservation
    } else if probe.observation_mode == super::model::K2InquiryObservationModeV1::Delayed {
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

fn selector_evaluate_probe_v1(
    case: &K2InquiryPublicCaseV1,
    probe: &K2InquiryProbeV1,
) -> K2CompositionResultV1<K2InquiryProbeEvaluationV1> {
    let eligibility = selector_probe_eligibility_v1(case, probe)?;
    let mut predictions = case
        .models
        .iter()
        .map(|model| {
            let (applied, reason, post) = match model.effect(&probe.action_id_sha256) {
                Some(effect) => selector_apply_effect_v1(&probe.initial_manifest, effect)?,
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
        })
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
        schema: super::model::K2_INQUIRY_EVALUATION_SCHEMA_V1.to_owned(),
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

fn selector_compare_evaluations_v1(
    case: &K2InquiryPublicCaseV1,
    left: &K2InquiryProbeEvaluationV1,
    right: &K2InquiryProbeEvaluationV1,
) -> Ordering {
    let left_probe = case
        .probe(&left.probe_root_sha256)
        .expect("validated left probe");
    let right_probe = case
        .probe(&right.probe_root_sha256)
        .expect("validated right probe");
    right
        .minimax_eliminated
        .cmp(&left.minimax_eliminated)
        .then_with(|| right.pair_separation.cmp(&left.pair_separation))
        .then_with(|| left_probe.risk_units.cmp(&right_probe.risk_units))
        .then_with(|| left_probe.cost_units.cmp(&right_probe.cost_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

fn selector_apply_effect_v1(
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
