use super::{BoundProtocolActionV3, BoundProtocolArgumentV3, BoundProtocolValueV3};
use crate::RuntimeCapabilityKindV3;

impl BoundProtocolArgumentV3 {
    #[must_use]
    pub const fn argument_ordinal(&self) -> u16 {
        self.argument_ordinal
    }

    #[must_use]
    pub const fn source_role_id(&self) -> u16 {
        self.source_role_id
    }

    #[must_use]
    pub fn physical_name(&self) -> &str {
        &self.physical_name
    }

    #[must_use]
    pub const fn value(&self) -> &BoundProtocolValueV3 {
        &self.value
    }
}

impl BoundProtocolActionV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn artifact_root_sha256(&self) -> &str {
        &self.artifact_root_sha256
    }

    #[must_use]
    pub fn mode_id_sha256(&self) -> &str {
        &self.mode_id_sha256
    }

    #[must_use]
    pub fn executable_mode_root_sha256(&self) -> &str {
        &self.executable_mode_root_sha256
    }

    #[must_use]
    pub fn payload_root_sha256(&self) -> &str {
        &self.payload_root_sha256
    }

    #[must_use]
    pub fn effect_law_id_sha256(&self) -> &str {
        &self.effect_law_id_sha256
    }

    #[must_use]
    pub fn action_class_root_sha256(&self) -> &str {
        &self.action_class_root_sha256
    }

    #[must_use]
    pub fn request_view_sha256(&self) -> &str {
        &self.request_view_sha256
    }

    #[must_use]
    pub fn mapping_sha256(&self) -> &str {
        &self.mapping_sha256
    }

    #[must_use]
    pub const fn capability_id(&self) -> u16 {
        self.capability_id
    }

    #[must_use]
    pub const fn capability_kind(&self) -> RuntimeCapabilityKindV3 {
        self.capability_kind
    }

    #[must_use]
    pub fn physical_symbol(&self) -> &str {
        &self.physical_symbol
    }

    #[must_use]
    pub fn arguments(&self) -> &[BoundProtocolArgumentV3] {
        &self.arguments
    }

    #[must_use]
    pub fn semantic_action_sha256(&self) -> &str {
        &self.semantic_action_sha256
    }

    #[must_use]
    pub fn physical_action_sha256(&self) -> &str {
        &self.physical_action_sha256
    }

    #[must_use]
    pub fn derivation_sha256(&self) -> &str {
        &self.derivation_sha256
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
