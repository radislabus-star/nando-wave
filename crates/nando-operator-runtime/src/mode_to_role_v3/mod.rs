mod binding;
mod compiler;
mod constraint;
mod dispatch;
mod dispatch_index;
mod encoding;
mod feature_codec;
mod report;

pub use binding::bind_structural_modes_v3;
pub use compiler::compile_structural_dispatch_index_v3;

use nando_core::wave::{OperatorCircuit, RoleGraph};
use nando_operator_kernel::{
    BindingValueTypeV1, ProtocolCapabilityArgumentV3, ProtocolCapabilityKindV3,
    RuntimeCapabilityKindV3,
};

use self::constraint::CompiledConstraintV3;
use self::dispatch_index::StructuralDispatchBitIndexV3;

pub const F5C_MAX_INDEXED_MODES_V3: usize = 2_048;
pub const F5C_MAX_DISPATCHED_MODES_V3: usize = 32;
pub const F5C_MAX_MAPPINGS_PER_MODE_V3: usize = 64;
pub const F5C_MAX_MAPPING_EVALUATIONS_V3: usize = 2_048;
pub const F5C_MAX_SOURCE_CANDIDATE_EVALUATIONS_V3: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModeToRoleErrorV3 {
    InvalidArtifact,
    DuplicateArtifact,
    DuplicateMode,
    UnsupportedSelector,
    InvalidSelector,
    InvalidGraph,
    IndexBudgetExhausted,
    Serialization,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralDispatchVerdictV3 {
    Complete,
    AbstainDispatchExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralBindingVerdictV3 {
    Complete,
    RejectIndexMismatch,
    AbstainDispatchExhausted,
    AbstainBindingExhausted,
    AbstainBudgetExhausted,
}

#[derive(Clone, Debug)]
pub struct CompiledProtocolModeV3 {
    artifact_root_sha256: String,
    mode_id_sha256: String,
    executable_mode_root_sha256: String,
    payload_root_sha256: String,
    effect_law_id_sha256: String,
    action_class_root_sha256: String,
    source_value_type: BindingValueTypeV1,
    source_roles: Box<[(u16, BindingValueTypeV1)]>,
    capability_kind: ProtocolCapabilityKindV3,
    capability_argument_types: Box<[BindingValueTypeV1]>,
    capability_arguments: Box<[ProtocolCapabilityArgumentV3]>,
    role_graph: RoleGraph,
    relation_program: OperatorCircuit,
    constraints: Box<[CompiledConstraintV3]>,
}

#[derive(Clone, Debug)]
pub struct StructuralDispatchIndexV3 {
    index_sha256: String,
    modes: Box<[CompiledProtocolModeV3]>,
    dispatch_bits: StructuralDispatchBitIndexV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralDispatchReportV3 {
    index_sha256: String,
    mode_indices: Box<[usize]>,
    matched_mode_count: usize,
    verdict: StructuralDispatchVerdictV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStructuralMappingV3 {
    runtime_source_role_id: u16,
    local_to_canonical: Box<[u8]>,
    phase_fit_fixed: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModeStructuralBindingReportV3 {
    mode_id_sha256: String,
    mappings: Box<[RuntimeStructuralMappingV3]>,
    source_candidate_evaluations: usize,
    phase_winner_count: usize,
    phase_runner_up_fit_fixed: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralBindingOutcomeV3 {
    index_sha256: String,
    request_view_sha256: String,
    mode_reports: Box<[ModeStructuralBindingReportV3]>,
    source_candidate_evaluations: usize,
    mapping_evaluations: usize,
    verdict: StructuralBindingVerdictV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteRuntimeRoleBindingReportV3 {
    index_sha256: String,
    request_view_sha256: String,
    mode_reports: Box<[ModeStructuralBindingReportV3]>,
    source_candidate_evaluations: usize,
    mapping_evaluations: usize,
}

impl CompiledProtocolModeV3 {
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
    pub const fn source_value_type(&self) -> BindingValueTypeV1 {
        self.source_value_type
    }

    #[must_use]
    pub fn source_role_type(&self, source_role_id: u16) -> Option<BindingValueTypeV1> {
        self.source_roles
            .iter()
            .find_map(|(role_id, value_type)| (*role_id == source_role_id).then_some(*value_type))
    }

    #[must_use]
    pub const fn capability_kind(&self) -> ProtocolCapabilityKindV3 {
        self.capability_kind
    }

    #[must_use]
    pub fn capability_argument_types(&self) -> &[BindingValueTypeV1] {
        &self.capability_argument_types
    }

    #[must_use]
    pub fn capability_arguments(&self) -> &[ProtocolCapabilityArgumentV3] {
        &self.capability_arguments
    }

    #[must_use]
    pub const fn role_graph(&self) -> &RoleGraph {
        &self.role_graph
    }

    #[must_use]
    pub const fn relation_program(&self) -> &OperatorCircuit {
        &self.relation_program
    }

    #[must_use]
    pub const fn runtime_capability_kind(&self) -> RuntimeCapabilityKindV3 {
        match self.capability_kind {
            ProtocolCapabilityKindV3::Function => RuntimeCapabilityKindV3::Function,
            ProtocolCapabilityKindV3::CustomTool => RuntimeCapabilityKindV3::Custom,
        }
    }
}

impl StructuralDispatchIndexV3 {
    #[must_use]
    pub fn index_sha256(&self) -> &str {
        &self.index_sha256
    }

    #[must_use]
    pub fn modes(&self) -> &[CompiledProtocolModeV3] {
        &self.modes
    }
}

#[cfg(test)]
pub(crate) mod tests;
