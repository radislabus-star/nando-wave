use super::{BoundProtocolProgramArgumentV3, BoundProtocolProgramV3};
use crate::{BoundProtocolValueV3, RuntimeCapabilityKindV3};

impl BoundProtocolProgramArgumentV3 {
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

impl BoundProtocolProgramV3 {
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        self.schema
    }

    #[must_use]
    pub fn program_sha256(&self) -> &str {
        &self.program_sha256
    }

    #[must_use]
    pub fn action_derivation_sha256(&self) -> &str {
        &self.action_derivation_sha256
    }

    #[must_use]
    pub fn physical_action_sha256(&self) -> &str {
        &self.physical_action_sha256
    }

    #[must_use]
    pub fn mode_id_sha256(&self) -> &str {
        &self.mode_id_sha256
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
    pub const fn capability_kind(&self) -> RuntimeCapabilityKindV3 {
        self.capability_kind
    }

    #[must_use]
    pub fn physical_symbol(&self) -> &str {
        &self.physical_symbol
    }

    #[must_use]
    pub fn arguments(&self) -> &[BoundProtocolProgramArgumentV3] {
        &self.arguments
    }

    #[must_use]
    pub const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
