use std::collections::BTreeMap;

use nando_core::wave::RuntimeRelationPhaseComponent;
use nando_operator_kernel::{RUNTIME_PHASE_APPLICABILITY_FLOOR_FIXED_V3, sha256_bytes};

use super::controls::{coherence_phase_components_v3, score_phase_components_v3};
use super::{
    PhaseAttemptScoreV3, PhaseControlReportV3, PhaseControlV3, PhaseGainVerdictV3,
    PhaseRankingReportV3, PhaseSelectionVerdictV3,
};
use crate::{
    ActionDerivationVerdictV3, BoundProtocolActionOutcomeV3, CapabilityGroundingVerdictV3,
};

#[must_use]
pub fn evaluate_phase_ranking_v3(outcome: &BoundProtocolActionOutcomeV3) -> PhaseRankingReportV3 {
    let controls = if outcome.verdict() == CapabilityGroundingVerdictV3::Complete
        && outcome.actions().len() == 1
    {
        PhaseControlV3::ALL
            .into_iter()
            .map(|control| evaluate_complete_control(outcome, control))
            .collect::<Vec<_>>()
    } else {
        let verdict = blocker_verdict(outcome.verdict());
        PhaseControlV3::ALL
            .into_iter()
            .map(|control| blocked_control(control, verdict))
            .collect::<Vec<_>>()
    };
    let full = &controls[0];
    let no_phase = &controls[1];
    let full_phase_search_gain = no_phase
        .exact_action_checks
        .saturating_sub(full.exact_action_checks);
    let full_phase_applicability_gain = controls
        .iter()
        .skip(1)
        .filter(|control| {
            control.selected_physical_action_sha256.as_deref()
                != full.selected_physical_action_sha256.as_deref()
        })
        .count();
    let evaluated = full.verdict == PhaseSelectionVerdictV3::Selected;
    let gain_verdict = if !evaluated {
        PhaseGainVerdictV3::NotEvaluated
    } else if full_phase_search_gain > 0 || full_phase_applicability_gain > 0 {
        PhaseGainVerdictV3::Measured
    } else {
        PhaseGainVerdictV3::WatchNoSearchGain
    };
    let phase_trace_informative = controls
        .iter()
        .skip(1)
        .any(|control| control_scores(control) != control_scores(full));
    let action_changes_from_structural_result = controls
        .iter()
        .filter(|control| {
            control.selected_physical_action_sha256.as_deref()
                != full.selected_physical_action_sha256.as_deref()
        })
        .count();
    let report_sha256 = report_digest(
        outcome.index_sha256(),
        outcome.request_view_sha256(),
        &controls,
        gain_verdict,
    );
    PhaseRankingReportV3 {
        report_sha256,
        index_sha256: outcome.index_sha256().to_owned(),
        request_view_sha256: outcome.request_view_sha256().to_owned(),
        controls: controls.into_boxed_slice(),
        full_phase_search_gain,
        full_phase_applicability_gain,
        gain_verdict,
        phase_trace_informative,
        action_changes_from_structural_result,
    }
}

fn evaluate_complete_control(
    outcome: &BoundProtocolActionOutcomeV3,
    control: PhaseControlV3,
) -> PhaseControlReportV3 {
    let scores = outcome
        .attempts()
        .iter()
        .filter(|attempt| attempt.verdict() == ActionDerivationVerdictV3::Bound)
        .filter_map(|attempt| {
            Some(PhaseAttemptScoreV3 {
                mapping_sha256: attempt.mapping_sha256().to_owned(),
                physical_action_sha256: attempt.physical_action_sha256()?.to_owned(),
                phase_trace_sha256: phase_trace_digest(attempt.phase_components_fixed()),
                score_fixed: score_phase_components_v3(attempt.phase_components_fixed(), control),
                coherence_fixed: coherence_phase_components_v3(
                    attempt.phase_components_fixed(),
                    control,
                ),
            })
        })
        .collect::<Vec<_>>();
    select_action_class(control, scores)
}

pub(super) fn select_action_class(
    control: PhaseControlV3,
    mut scores: Vec<PhaseAttemptScoreV3>,
) -> PhaseControlReportV3 {
    scores.sort_by(|left, right| {
        right
            .coherence_fixed
            .cmp(&left.coherence_fixed)
            .then_with(|| right.score_fixed.cmp(&left.score_fixed))
            .then_with(|| {
                left.physical_action_sha256
                    .cmp(&right.physical_action_sha256)
            })
            .then_with(|| left.mapping_sha256.cmp(&right.mapping_sha256))
    });
    let mut action_scores = BTreeMap::<String, (i64, i64)>::new();
    for score in &scores {
        action_scores
            .entry(score.physical_action_sha256.clone())
            .and_modify(|current| {
                *current = (*current).max((score.coherence_fixed, score.score_fixed));
            })
            .or_insert((score.coherence_fixed, score.score_fixed));
    }
    let action_classes = action_scores.len();
    let mut ranked = action_scores.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let winner_score_fixed = ranked.first().map(|(_, (_, score))| *score);
    let runner_up_score_fixed = ranked.get(1).map(|(_, (_, score))| *score);
    let winner_coherence_fixed = ranked.first().map(|(_, (coherence, _))| *coherence);
    let runner_up_coherence_fixed = ranked.get(1).map(|(_, (coherence, _))| *coherence);
    let tie =
        winner_coherence_fixed.is_some() && winner_coherence_fixed == runner_up_coherence_fixed;
    let (selected, verdict) = match ranked.first() {
        None => (None, PhaseSelectionVerdictV3::AbstainNoCandidate),
        Some(_) if tie => (None, PhaseSelectionVerdictV3::AbstainTie),
        Some((_, (coherence, _))) if *coherence < RUNTIME_PHASE_APPLICABILITY_FLOOR_FIXED_V3 => {
            (None, PhaseSelectionVerdictV3::AbstainCoherenceFloor)
        }
        Some((action, _)) => (Some(action.clone()), PhaseSelectionVerdictV3::Selected),
    };
    PhaseControlReportV3 {
        control,
        scores: scores.into_boxed_slice(),
        action_classes,
        exact_action_checks: action_classes,
        selected_physical_action_sha256: selected,
        winner_score_fixed,
        runner_up_score_fixed,
        winner_coherence_fixed,
        runner_up_coherence_fixed,
        verdict,
    }
}

fn blocked_control(
    control: PhaseControlV3,
    verdict: PhaseSelectionVerdictV3,
) -> PhaseControlReportV3 {
    PhaseControlReportV3 {
        control,
        scores: Box::new([]),
        action_classes: 0,
        exact_action_checks: 0,
        selected_physical_action_sha256: None,
        winner_score_fixed: None,
        runner_up_score_fixed: None,
        winner_coherence_fixed: None,
        runner_up_coherence_fixed: None,
        verdict,
    }
}

const fn blocker_verdict(verdict: CapabilityGroundingVerdictV3) -> PhaseSelectionVerdictV3 {
    match verdict {
        CapabilityGroundingVerdictV3::AbstainAmbiguousAction
        | CapabilityGroundingVerdictV3::AbstainAmbiguousCapability => {
            PhaseSelectionVerdictV3::AbstainAmbiguousAction
        }
        _ => PhaseSelectionVerdictV3::AbstainStructuralBoundary,
    }
}

fn control_scores(control: &PhaseControlReportV3) -> Vec<(&str, &str, i64)> {
    let mut scores = control
        .scores
        .iter()
        .map(|score| {
            (
                score.mapping_sha256.as_str(),
                score.physical_action_sha256.as_str(),
                score.score_fixed,
            )
        })
        .collect::<Vec<_>>();
    scores.sort_unstable();
    scores
}

fn phase_trace_digest(components: &[RuntimeRelationPhaseComponent]) -> String {
    let mut bytes = b"nando.runtime-relation-phase-trace.v3".to_vec();
    for component in components {
        bytes.push(component.plane());
        bytes.push(component.source_role());
        bytes.push(component.target_role());
        let (observed_re, observed_im) = component.observed_fixed();
        let (expected_re, expected_im) = component.expected_fixed();
        for value in [observed_re, observed_im, expected_re, expected_im] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    sha256_bytes(&bytes)
}

fn report_digest(
    index_sha256: &str,
    request_view_sha256: &str,
    controls: &[PhaseControlReportV3],
    gain_verdict: PhaseGainVerdictV3,
) -> String {
    let mut bytes = b"nando.phase-ranking-report.v3".to_vec();
    bytes.extend_from_slice(index_sha256.as_bytes());
    bytes.extend_from_slice(request_view_sha256.as_bytes());
    bytes.push(gain_verdict as u8);
    for control in controls {
        bytes.push(control.control as u8);
        bytes.push(control.verdict as u8);
        for score in &control.scores {
            bytes.extend_from_slice(score.mapping_sha256.as_bytes());
            bytes.extend_from_slice(score.physical_action_sha256.as_bytes());
            bytes.extend_from_slice(score.phase_trace_sha256.as_bytes());
            bytes.extend_from_slice(&score.score_fixed.to_le_bytes());
            bytes.extend_from_slice(&score.coherence_fixed.to_le_bytes());
        }
    }
    sha256_bytes(&bytes)
}
