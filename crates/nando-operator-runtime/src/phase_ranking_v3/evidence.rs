use nando_operator_kernel::{
    RuntimePhaseControlEvidenceErrorV3, RuntimePhaseControlEvidenceInputV3,
    RuntimePhaseControlEvidenceV3, RuntimePhaseControlKindV3,
    RuntimePhaseControlObservationInputV3, RuntimePhaseSelectionV3,
    seal_runtime_phase_control_evidence_v3,
};

use super::{PhaseControlV3, PhaseRankingReportV3, PhaseSelectionVerdictV3};

pub fn export_runtime_phase_control_evidence_v3(
    report: &PhaseRankingReportV3,
) -> Result<RuntimePhaseControlEvidenceV3, RuntimePhaseControlEvidenceErrorV3> {
    let observations = report
        .controls()
        .iter()
        .map(|control| {
            Ok(RuntimePhaseControlObservationInputV3 {
                control: control_kind(control.control()),
                selection: selection(control.verdict()),
                exact_action_checks: u32::try_from(control.exact_action_checks())
                    .map_err(|_| RuntimePhaseControlEvidenceErrorV3::InvalidInput)?,
                selected_physical_action_sha256: control
                    .selected_physical_action_sha256()
                    .map(str::to_owned),
                winner_coherence_fixed: control.winner_coherence_fixed(),
                runner_up_coherence_fixed: control.runner_up_coherence_fixed(),
            })
        })
        .collect::<Result<Vec<_>, RuntimePhaseControlEvidenceErrorV3>>()?;
    seal_runtime_phase_control_evidence_v3(RuntimePhaseControlEvidenceInputV3 {
        index_sha256: report.index_sha256().to_owned(),
        request_view_sha256: report.request_view_sha256().to_owned(),
        report_sha256: report.report_sha256().to_owned(),
        observations,
    })
}

const fn control_kind(control: PhaseControlV3) -> RuntimePhaseControlKindV3 {
    match control {
        PhaseControlV3::Full => RuntimePhaseControlKindV3::Full,
        PhaseControlV3::NoPhase => RuntimePhaseControlKindV3::NoPhase,
        PhaseControlV3::ShuffledPhase => RuntimePhaseControlKindV3::ShuffledPhase,
        PhaseControlV3::MagnitudeOnly => RuntimePhaseControlKindV3::MagnitudeOnly,
        PhaseControlV3::MatchedRandomCenter => RuntimePhaseControlKindV3::MatchedRandomCenter,
    }
}

const fn selection(verdict: PhaseSelectionVerdictV3) -> RuntimePhaseSelectionV3 {
    match verdict {
        PhaseSelectionVerdictV3::Selected => RuntimePhaseSelectionV3::Selected,
        PhaseSelectionVerdictV3::AbstainStructuralBoundary => {
            RuntimePhaseSelectionV3::AbstainStructuralBoundary
        }
        PhaseSelectionVerdictV3::AbstainAmbiguousAction => {
            RuntimePhaseSelectionV3::AbstainAmbiguousAction
        }
        PhaseSelectionVerdictV3::AbstainTie => RuntimePhaseSelectionV3::AbstainTie,
        PhaseSelectionVerdictV3::AbstainNoCandidate => RuntimePhaseSelectionV3::AbstainNoCandidate,
        PhaseSelectionVerdictV3::AbstainCoherenceFloor => {
            RuntimePhaseSelectionV3::AbstainCoherenceFloor
        }
    }
}
