use super::{
    CompleteRuntimeRoleBindingReportV3, ModeStructuralBindingReportV3, RuntimeStructuralMappingV3,
    StructuralBindingOutcomeV3, StructuralBindingVerdictV3, StructuralDispatchReportV3,
    StructuralDispatchVerdictV3,
};

impl StructuralDispatchReportV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn mode_indices(&self) -> &[usize] {
        &self.mode_indices
    }

    #[must_use]
    pub const fn matched_mode_count(&self) -> usize {
        self.matched_mode_count
    }

    #[must_use]
    pub const fn verdict(&self) -> StructuralDispatchVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl RuntimeStructuralMappingV3 {
    #[must_use]
    pub const fn runtime_source_role_id(&self) -> u16 {
        self.runtime_source_role_id
    }

    #[must_use]
    pub fn local_to_canonical(&self) -> &[u8] {
        &self.local_to_canonical
    }

    #[must_use]
    pub const fn phase_fit_fixed(&self) -> i64 {
        self.phase_fit_fixed
    }

    #[must_use]
    pub fn phase_components_fixed(&self) -> &[nando_core::wave::RuntimeRelationPhaseComponent] {
        &self.phase_components_fixed
    }
}

impl ModeStructuralBindingReportV3 {
    #[must_use]
    pub fn mode_id_sha256(&self) -> &str {
        &self.mode_id_sha256
    }

    #[must_use]
    pub fn mappings(&self) -> &[RuntimeStructuralMappingV3] {
        &self.mappings
    }

    #[must_use]
    pub fn phase_winner_mappings(&self) -> &[RuntimeStructuralMappingV3] {
        &self.mappings[..self.phase_winner_count]
    }

    #[must_use]
    pub const fn source_candidate_evaluations(&self) -> usize {
        self.source_candidate_evaluations
    }

    #[must_use]
    pub fn mapping_evaluations(&self) -> usize {
        self.mappings.len()
    }

    #[must_use]
    pub const fn phase_winner_count(&self) -> usize {
        self.phase_winner_count
    }

    #[must_use]
    pub const fn phase_runner_up_fit_fixed(&self) -> Option<i64> {
        self.phase_runner_up_fit_fixed
    }

    #[must_use]
    pub fn phase_margin_fixed(&self) -> Option<i64> {
        let winner = self.mappings.first()?.phase_fit_fixed;
        self.phase_runner_up_fit_fixed
            .map(|runner_up| winner.saturating_sub(runner_up))
    }
}

impl StructuralBindingOutcomeV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn mode_reports(&self) -> &[ModeStructuralBindingReportV3] {
        &self.mode_reports
    }

    #[must_use]
    pub const fn source_candidate_evaluations(&self) -> usize {
        self.source_candidate_evaluations
    }

    #[must_use]
    pub const fn mapping_evaluations(&self) -> usize {
        self.mapping_evaluations
    }

    #[must_use]
    pub const fn verdict(&self) -> StructuralBindingVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub fn into_complete(self) -> Option<CompleteRuntimeRoleBindingReportV3> {
        if self.verdict != StructuralBindingVerdictV3::Complete {
            return None;
        }
        Some(CompleteRuntimeRoleBindingReportV3 {
            index_sha256: self.index_sha256,
            request_view_sha256: self.request_view_sha256,
            mode_reports: self.mode_reports,
            source_candidate_evaluations: self.source_candidate_evaluations,
            mapping_evaluations: self.mapping_evaluations,
        })
    }
}

impl CompleteRuntimeRoleBindingReportV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn mode_reports(&self) -> &[ModeStructuralBindingReportV3] {
        &self.mode_reports
    }

    #[must_use]
    pub const fn source_candidate_evaluations(&self) -> usize {
        self.source_candidate_evaluations
    }

    #[must_use]
    pub const fn mapping_evaluations(&self) -> usize {
        self.mapping_evaluations
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
