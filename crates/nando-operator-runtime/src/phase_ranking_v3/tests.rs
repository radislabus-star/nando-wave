use nando_core::wave::RuntimeRelationPhaseComponent;
use serde_json::json;

use super::controls::score_phase_components_v3;
use super::evaluator::select_action_class;
use super::*;
use crate::mode_to_role_v3::tests::fixtures::{
    artifact, mentioned_string_selector, request_payload, runtime_context,
};
use crate::{
    CapabilityGroundingVerdictV3, bind_structural_modes_v3, compile_structural_dispatch_index_v3,
    ground_protocol_actions_v3,
};

fn outcome(output: serde_json::Value) -> crate::BoundProtocolActionOutcomeV3 {
    outcome_from_payload(request_payload(output))
}

fn outcome_from_payload(payload: serde_json::Value) -> crate::BoundProtocolActionOutcomeV3 {
    let artifacts = [artifact(301, mentioned_string_selector())];
    let request = runtime_context("continue CellA17 then CellB18", &payload);
    let index = compile_structural_dispatch_index_v3(&artifacts).expect("index");
    let dispatch = index.dispatch(&request);
    let binding = bind_structural_modes_v3(&index, &request, &dispatch)
        .into_complete()
        .expect("binding");
    ground_protocol_actions_v3(&index, &request, &binding)
}

#[test]
fn real_phase_trace_recomputes_the_existing_binder_score() {
    let outcome = outcome(json!({"handle": "CellA17"}));
    assert_eq!(outcome.verdict(), CapabilityGroundingVerdictV3::Complete);
    for attempt in outcome.attempts() {
        if attempt.verdict() == crate::ActionDerivationVerdictV3::Bound {
            assert_eq!(
                score_phase_components_v3(attempt.phase_components_fixed(), PhaseControlV3::Full),
                attempt.phase_fit_fixed()
            );
        }
    }
}

#[test]
fn complete_structural_action_survives_every_control_without_authority() {
    let outcome = outcome(json!({"handle": "CellA17"}));
    let report = evaluate_phase_ranking_v3(&outcome);

    assert_eq!(report.controls().len(), PhaseControlV3::ALL.len());
    assert!(
        report
            .controls()
            .iter()
            .all(|control| control.verdict() == PhaseSelectionVerdictV3::Selected)
    );
    assert_eq!(report.action_changes_from_structural_result(), 0);
    assert_eq!(report.full_phase_search_gain(), 0);
    assert_eq!(report.gain_verdict(), PhaseGainVerdictV3::WatchNoSearchGain);
    assert!(!report.execution_authority());
}

#[test]
fn phase_never_rescues_ambiguous_structural_actions() {
    let outcome = outcome(json!({"first": "CellA17", "second": "CellB18"}));
    assert_eq!(
        outcome.verdict(),
        CapabilityGroundingVerdictV3::AbstainAmbiguousAction
    );
    let report = evaluate_phase_ranking_v3(&outcome);

    assert!(report.controls().iter().all(|control| {
        control.verdict() == PhaseSelectionVerdictV3::AbstainAmbiguousAction
            && control.selected_physical_action_sha256().is_none()
    }));
    assert_eq!(report.gain_verdict(), PhaseGainVerdictV3::NotEvaluated);
}

#[test]
fn missing_structural_action_remains_blocked_for_every_control() {
    let mut payload = request_payload(json!({"handle": "CellA17"}));
    payload["tools"] = json!([]);
    let outcome = outcome_from_payload(payload);
    let report = evaluate_phase_ranking_v3(&outcome);

    assert!(report.controls().iter().all(|control| {
        control.verdict() == PhaseSelectionVerdictV3::AbstainStructuralBoundary
            && control.scores().is_empty()
    }));
}

#[test]
fn distinct_action_tie_is_always_abstain() {
    let report = select_action_class(
        PhaseControlV3::Full,
        vec![
            PhaseAttemptScoreV3 {
                mapping_sha256: "a".repeat(64),
                physical_action_sha256: "b".repeat(64),
                phase_trace_sha256: "e".repeat(64),
                score_fixed: 7,
            },
            PhaseAttemptScoreV3 {
                mapping_sha256: "c".repeat(64),
                physical_action_sha256: "d".repeat(64),
                phase_trace_sha256: "f".repeat(64),
                score_fixed: 7,
            },
        ],
    );
    assert_eq!(report.verdict(), PhaseSelectionVerdictV3::AbstainTie);
    assert_eq!(report.action_classes(), 2);
    assert!(report.selected_physical_action_sha256().is_none());
}

#[test]
fn controls_use_phase_components_instead_of_the_aggregate_score() {
    let components = test_phase_components();
    let full = score_phase_components_v3(&components, PhaseControlV3::Full);
    let no_phase = score_phase_components_v3(&components, PhaseControlV3::NoPhase);
    let shuffled = score_phase_components_v3(&components, PhaseControlV3::ShuffledPhase);

    assert_eq!(full, 2_000_000_000);
    assert_eq!(no_phase, 0);
    assert_ne!(shuffled, full);
    assert!(
        RuntimeRelationPhaseComponent::try_from_fixed(
            0,
            0,
            1,
            (1_000_000_001, 0),
            (1_000_000_000, 0),
        )
        .is_none()
    );
}

fn test_phase_components() -> [RuntimeRelationPhaseComponent; 2] {
    // Test-only construction stays inside nando-core's public immutable value contract.
    [
        RuntimeRelationPhaseComponent::try_from_fixed(
            0,
            0,
            1,
            (1_000_000_000, 0),
            (1_000_000_000, 0),
        )
        .expect("valid fixed phase component"),
        RuntimeRelationPhaseComponent::try_from_fixed(
            1,
            0,
            2,
            (0, 1_000_000_000),
            (0, 1_000_000_000),
        )
        .expect("valid fixed phase component"),
    ]
}
