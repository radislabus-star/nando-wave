mod derivation;
mod grounding;
mod report;

pub use grounding::ground_protocol_actions_v3;

use nando_operator_kernel::BoundProtocolActionV3;

pub const F5D_MAX_ACTION_DERIVATIONS_V3: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionDerivationVerdictV3 {
    Bound,
    MissingCapability,
    AmbiguousCapabilityTopology,
    MissingRoleValue,
    AmbiguousRoleValue,
    UnsupportedSourceRole,
    InvalidAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityGroundingVerdictV3 {
    Complete,
    RejectIndexMismatch,
    AbstainNoStructuralMapping,
    AbstainMissingCapability,
    AbstainAmbiguousCapability,
    AbstainAmbiguousAction,
    AbstainRoleValue,
    AbstainBudgetExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingActionAttemptV3 {
    mode_id_sha256: String,
    mapping_sha256: String,
    runtime_source_role_id: u16,
    phase_fit_fixed: i64,
    phase_components_fixed: Box<[nando_core::wave::RuntimeRelationPhaseComponent]>,
    capability_id: Option<u16>,
    verdict: ActionDerivationVerdictV3,
    semantic_action_sha256: Option<String>,
    physical_action_sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BoundProtocolActionOutcomeV3 {
    index_sha256: String,
    request_view_sha256: String,
    attempts: Box<[MappingActionAttemptV3]>,
    actions: Box<[BoundProtocolActionV3]>,
    structural_mappings: usize,
    action_derivations: usize,
    semantic_action_classes: usize,
    physical_action_classes: usize,
    verdict: CapabilityGroundingVerdictV3,
}

#[derive(Clone, Debug)]
pub struct BoundProtocolActionSetV3 {
    index_sha256: String,
    request_view_sha256: String,
    attempts: Box<[MappingActionAttemptV3]>,
    action: BoundProtocolActionV3,
    structural_mappings: usize,
    action_derivations: usize,
}

#[cfg(test)]
mod tests;
