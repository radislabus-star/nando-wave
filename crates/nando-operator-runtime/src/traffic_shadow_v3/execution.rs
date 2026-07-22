use nando_operator_kernel::BoundProtocolActionV3;

use super::TrafficShadowReceiptV3;

pub struct TrafficShadowExecutionV3 {
    receipt: TrafficShadowReceiptV3,
    actor_action: Option<BoundProtocolActionV3>,
    actor_output: Option<String>,
}

impl TrafficShadowExecutionV3 {
    pub(super) fn receipt_only(receipt: TrafficShadowReceiptV3) -> Self {
        Self {
            receipt,
            actor_action: None,
            actor_output: None,
        }
    }

    pub(super) fn complete(
        receipt: TrafficShadowReceiptV3,
        actor_action: BoundProtocolActionV3,
        actor_output: String,
    ) -> Self {
        Self {
            receipt,
            actor_action: Some(actor_action),
            actor_output: Some(actor_output),
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
    pub fn into_receipt(self) -> TrafficShadowReceiptV3 {
        self.receipt
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}
