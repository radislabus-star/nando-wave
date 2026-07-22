use super::{
    ActionDerivationVerdictV3, BoundProtocolActionOutcomeV3, BoundProtocolActionSetV3,
    CapabilityGroundingVerdictV3, MappingActionAttemptV3,
};

impl MappingActionAttemptV3 {
    #[must_use]
    pub fn mode_id_sha256(&self) -> &str {
        &self.mode_id_sha256
    }

    #[must_use]
    pub fn mapping_sha256(&self) -> &str {
        &self.mapping_sha256
    }

    #[must_use]
    pub const fn runtime_source_role_id(&self) -> u16 {
        self.runtime_source_role_id
    }

    #[must_use]
    pub const fn phase_fit_fixed(&self) -> i64 {
        self.phase_fit_fixed
    }

    #[must_use]
    pub const fn capability_id(&self) -> Option<u16> {
        self.capability_id
    }

    #[must_use]
    pub const fn verdict(&self) -> ActionDerivationVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub fn semantic_action_sha256(&self) -> Option<&str> {
        self.semantic_action_sha256.as_deref()
    }

    #[must_use]
    pub fn physical_action_sha256(&self) -> Option<&str> {
        self.physical_action_sha256.as_deref()
    }
}

impl BoundProtocolActionOutcomeV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn attempts(&self) -> &[MappingActionAttemptV3] {
        &self.attempts
    }

    #[must_use]
    pub fn actions(&self) -> &[nando_operator_kernel::BoundProtocolActionV3] {
        &self.actions
    }

    #[must_use]
    pub const fn structural_mappings(&self) -> usize {
        self.structural_mappings
    }

    #[must_use]
    pub const fn action_derivations(&self) -> usize {
        self.action_derivations
    }

    #[must_use]
    pub const fn semantic_action_classes(&self) -> usize {
        self.semantic_action_classes
    }

    #[must_use]
    pub const fn physical_action_classes(&self) -> usize {
        self.physical_action_classes
    }

    #[must_use]
    pub const fn verdict(&self) -> CapabilityGroundingVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub fn into_complete(self) -> Option<BoundProtocolActionSetV3> {
        if self.verdict != CapabilityGroundingVerdictV3::Complete || self.actions.len() != 1 {
            return None;
        }
        let mut actions = self.actions.into_vec();
        Some(BoundProtocolActionSetV3 {
            index_sha256: self.index_sha256,
            request_view_sha256: self.request_view_sha256,
            attempts: self.attempts,
            action: actions.pop()?,
            structural_mappings: self.structural_mappings,
            action_derivations: self.action_derivations,
        })
    }
}

impl BoundProtocolActionSetV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn attempts(&self) -> &[MappingActionAttemptV3] {
        &self.attempts
    }

    #[must_use]
    pub const fn action(&self) -> &nando_operator_kernel::BoundProtocolActionV3 {
        &self.action
    }

    #[must_use]
    pub const fn structural_mappings(&self) -> usize {
        self.structural_mappings
    }

    #[must_use]
    pub const fn action_derivations(&self) -> usize {
        self.action_derivations
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
