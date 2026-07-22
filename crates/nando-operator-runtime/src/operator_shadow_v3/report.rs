use super::{OperatorShadowExecutionReceiptV3, OperatorShadowExecutionV3, OperatorShadowVerdictV3};

impl OperatorShadowExecutionReceiptV3 {
    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }

    #[must_use]
    pub fn program_sha256(&self) -> Option<&str> {
        self.program_sha256.as_deref()
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
    pub fn bytecode_sha256(&self) -> Option<&str> {
        self.bytecode_sha256.as_deref()
    }

    #[must_use]
    pub fn actor_output_sha256(&self) -> Option<&str> {
        self.actor_output_sha256.as_deref()
    }

    #[must_use]
    pub fn vm_output_sha256(&self) -> Option<&str> {
        self.vm_output_sha256.as_deref()
    }

    #[must_use]
    pub const fn actor_output_bytes(&self) -> usize {
        self.actor_output_bytes
    }

    #[must_use]
    pub const fn vm_output_bytes(&self) -> usize {
        self.vm_output_bytes
    }

    #[must_use]
    pub const fn verdict(&self) -> OperatorShadowVerdictV3 {
        self.verdict
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

impl OperatorShadowExecutionV3 {
    #[must_use]
    pub fn program(&self) -> Option<&nando_operator_kernel::BoundProtocolProgramV3> {
        self.program.as_ref()
    }

    #[must_use]
    pub fn bytecode(&self) -> Option<&[u8]> {
        self.bytecode.as_deref()
    }

    #[must_use]
    pub fn actor_output(&self) -> Option<&str> {
        self.actor_output.as_deref()
    }

    #[must_use]
    pub fn vm_output(&self) -> Option<&str> {
        self.vm_output.as_deref()
    }

    #[must_use]
    pub const fn receipt(&self) -> &OperatorShadowExecutionReceiptV3 {
        &self.receipt
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
