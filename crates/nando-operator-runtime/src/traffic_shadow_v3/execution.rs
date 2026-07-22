use nando_operator_kernel::{BoundProtocolActionV3, RuntimePhaseControlEvidenceV3};

use super::TrafficShadowReceiptV3;

pub struct TrafficShadowExecutionV3 {
    receipt: TrafficShadowReceiptV3,
    actor_action: Option<BoundProtocolActionV3>,
    actor_output: Option<String>,
    phase_control_evidence: Option<RuntimePhaseControlEvidenceV3>,
}

impl TrafficShadowExecutionV3 {
    pub(super) fn receipt_only(receipt: TrafficShadowReceiptV3) -> Self {
        Self {
            receipt,
            actor_action: None,
            actor_output: None,
            phase_control_evidence: None,
        }
    }

    pub(super) fn receipt_with_phase(
        receipt: TrafficShadowReceiptV3,
        phase_control_evidence: RuntimePhaseControlEvidenceV3,
    ) -> Self {
        Self {
            receipt,
            actor_action: None,
            actor_output: None,
            phase_control_evidence: Some(phase_control_evidence),
        }
    }

    pub(super) fn complete(
        receipt: TrafficShadowReceiptV3,
        actor_action: BoundProtocolActionV3,
        actor_output: String,
        phase_control_evidence: RuntimePhaseControlEvidenceV3,
    ) -> Self {
        Self {
            receipt,
            actor_action: Some(actor_action),
            actor_output: Some(actor_output),
            phase_control_evidence: Some(phase_control_evidence),
        }
    }

    #[must_use]
    pub const fn receipt(&self) -> &TrafficShadowReceiptV3 {
        &self.receipt
    }

    #[must_use]
    pub const fn actor_action(&self) -> Option<&BoundProtocolActionV3> {
        self.actor_action.as_ref()
    }

    #[must_use]
    pub fn actor_output(&self) -> Option<&str> {
        self.actor_output.as_deref()
    }

    #[must_use]
    pub const fn phase_control_evidence(&self) -> Option<&RuntimePhaseControlEvidenceV3> {
        self.phase_control_evidence.as_ref()
    }

    #[must_use]
    pub fn into_receipt(self) -> TrafficShadowReceiptV3 {
        self.receipt
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
