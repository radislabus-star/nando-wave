mod controls;
mod evaluator;
mod evidence;
mod report;

pub use evaluator::evaluate_phase_ranking_v3;
pub use evidence::export_runtime_phase_control_evidence_v3;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum PhaseControlV3 {
    Full,
    NoPhase,
    ShuffledPhase,
    MagnitudeOnly,
    MatchedRandomCenter,
}

impl PhaseControlV3 {
    pub const ALL: [Self; 5] = [
        Self::Full,
        Self::NoPhase,
        Self::ShuffledPhase,
        Self::MagnitudeOnly,
        Self::MatchedRandomCenter,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PhaseSelectionVerdictV3 {
    Selected,
    AbstainStructuralBoundary,
    AbstainAmbiguousAction,
    AbstainTie,
    AbstainNoCandidate,
    AbstainCoherenceFloor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PhaseGainVerdictV3 {
    Measured,
    WatchNoSearchGain,
    NotEvaluated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseAttemptScoreV3 {
    mapping_sha256: String,
    physical_action_sha256: String,
    phase_trace_sha256: String,
    score_fixed: i64,
    coherence_fixed: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseControlReportV3 {
    control: PhaseControlV3,
    scores: Box<[PhaseAttemptScoreV3]>,
    action_classes: usize,
    exact_action_checks: usize,
    selected_physical_action_sha256: Option<String>,
    winner_score_fixed: Option<i64>,
    runner_up_score_fixed: Option<i64>,
    winner_coherence_fixed: Option<i64>,
    runner_up_coherence_fixed: Option<i64>,
    verdict: PhaseSelectionVerdictV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhaseRankingReportV3 {
    report_sha256: String,
    index_sha256: String,
    request_view_sha256: String,
    controls: Box<[PhaseControlReportV3]>,
    full_phase_search_gain: usize,
    full_phase_applicability_gain: usize,
    gain_verdict: PhaseGainVerdictV3,
    phase_trace_informative: bool,
    action_changes_from_structural_result: usize,
}

#[cfg(test)]
mod tests;
