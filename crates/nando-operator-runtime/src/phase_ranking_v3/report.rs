use super::{
    PhaseAttemptScoreV3, PhaseControlReportV3, PhaseControlV3, PhaseGainVerdictV3,
    PhaseRankingReportV3, PhaseSelectionVerdictV3,
};

impl PhaseAttemptScoreV3 {
    #[must_use]
    pub fn mapping_sha256(&self) -> &str {
        &self.mapping_sha256
    }

    #[must_use]
    pub fn physical_action_sha256(&self) -> &str {
        &self.physical_action_sha256
    }

    #[must_use]
    pub fn phase_trace_sha256(&self) -> &str {
        &self.phase_trace_sha256
    }

    #[must_use]
    pub const fn score_fixed(&self) -> i64 {
        self.score_fixed
    }
}

impl PhaseControlReportV3 {
    #[must_use]
    pub const fn control(&self) -> PhaseControlV3 {
        self.control
    }

    #[must_use]
    pub fn scores(&self) -> &[PhaseAttemptScoreV3] {
        &self.scores
    }

    #[must_use]
    pub const fn action_classes(&self) -> usize {
        self.action_classes
    }

    #[must_use]
    pub const fn exact_action_checks(&self) -> usize {
        self.exact_action_checks
    }

    #[must_use]
    pub fn selected_physical_action_sha256(&self) -> Option<&str> {
        self.selected_physical_action_sha256.as_deref()
    }

    #[must_use]
    pub const fn winner_score_fixed(&self) -> Option<i64> {
        self.winner_score_fixed
    }

    #[must_use]
    pub const fn runner_up_score_fixed(&self) -> Option<i64> {
        self.runner_up_score_fixed
    }

    #[must_use]
    pub const fn verdict(&self) -> PhaseSelectionVerdictV3 {
        self.verdict
    }
}

impl PhaseRankingReportV3 {
    #[must_use]
    pub fn report_sha256(&self) -> &str {
        &self.report_sha256
    }

    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn controls(&self) -> &[PhaseControlReportV3] {
        &self.controls
    }

    #[must_use]
    pub const fn full_phase_search_gain(&self) -> usize {
        self.full_phase_search_gain
    }

    #[must_use]
    pub const fn gain_verdict(&self) -> PhaseGainVerdictV3 {
        self.gain_verdict
    }

    #[must_use]
    pub const fn phase_trace_informative(&self) -> bool {
        self.phase_trace_informative
    }

    #[must_use]
    pub const fn action_changes_from_structural_result(&self) -> usize {
        self.action_changes_from_structural_result
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
