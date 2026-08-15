use std::cmp::Ordering;
use std::io::{Read, Write};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_bytes_v1, composition_decode_v1, composition_root_v1, composition_sha256_file_v1,
};
use super::model::{
    K2_INQUIRY_BASELINES_SCHEMA_V1, K2_INQUIRY_MAX_COST_UNITS_V1, K2_INQUIRY_MAX_PROTOCOL_BYTES_V1,
    K2_INQUIRY_MAX_RISK_UNITS_V1, K2InquiryBaselineDecisionV1, K2InquiryBaselineKindV1,
    K2InquiryBaselineRequestV1, K2InquiryBaselinesV1, K2InquiryObservationModeV1, K2InquiryProbeV1,
    K2InquiryPublicCaseV1,
};

pub fn evaluate_inquiry_baselines_v1(
    request: &K2InquiryBaselineRequestV1,
) -> K2CompositionResultV1<K2InquiryBaselinesV1> {
    request.validate()?;
    let eligible = request
        .public_case
        .probes
        .iter()
        .filter(|probe| baseline_probe_eligible_v1(&request.public_case, probe))
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_baseline_no_eligible_probe",
        ));
    }

    let stable = eligible
        .iter()
        .min_by_key(|probe| &probe.probe_root_sha256)
        .expect("nonempty eligible probes");
    let cheapest = eligible
        .iter()
        .min_by(|left, right| baseline_compare_cheapest_v1(left, right))
        .expect("nonempty eligible probes");
    let heuristic = eligible
        .iter()
        .min_by(|left, right| baseline_compare_heuristic_v1(left, right))
        .expect("nonempty eligible probes");

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

pub fn run_inquiry_baseline_process_v1() -> K2CompositionResultV1<()> {
    let mut input = Vec::new();
    std::io::stdin()
        .take((K2_INQUIRY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| K2CompositionErrorV1::Io("read_inquiry_baseline_stdin"))?;
    let request: K2InquiryBaselineRequestV1 = composition_decode_v1(&input)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_inquiry_baseline"))?;
    if composition_sha256_file_v1(&executable)? != request.baseline_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "inquiry_baseline_executable_mismatch",
        ));
    }
    let outcome = evaluate_inquiry_baselines_v1(&request)?;
    std::io::stdout()
        .write_all(&composition_bytes_v1(&outcome)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_inquiry_baseline_stdout"))
}

fn baseline_decision_v1(
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

fn baseline_probe_eligible_v1(case: &K2InquiryPublicCaseV1, probe: &K2InquiryProbeV1) -> bool {
    probe.reversible
        && probe.observation_mode == K2InquiryObservationModeV1::ExactImmediate
        && probe.risk_units <= K2_INQUIRY_MAX_RISK_UNITS_V1
        && probe.cost_units <= K2_INQUIRY_MAX_COST_UNITS_V1
        && case
            .models
            .iter()
            .all(|model| model.effect(&probe.action_id_sha256).is_some())
}

fn baseline_compare_cheapest_v1(left: &&K2InquiryProbeV1, right: &&K2InquiryProbeV1) -> Ordering {
    left.cost_units
        .cmp(&right.cost_units)
        .then_with(|| left.risk_units.cmp(&right.risk_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

fn baseline_compare_heuristic_v1(left: &&K2InquiryProbeV1, right: &&K2InquiryProbeV1) -> Ordering {
    let left_score = baseline_heuristic_score_v1(left);
    let right_score = baseline_heuristic_score_v1(right);
    right_score
        .cmp(&left_score)
        .then_with(|| left.risk_units.cmp(&right.risk_units))
        .then_with(|| left.cost_units.cmp(&right.cost_units))
        .then_with(|| left.probe_root_sha256.cmp(&right.probe_root_sha256))
}

fn baseline_heuristic_score_v1(probe: &K2InquiryProbeV1) -> u64 {
    u64::from(probe.applicability_hint) * 4
        + u64::from(probe.dependency_hint) * 2
        + u64::from(probe.cleanup_hint)
}
