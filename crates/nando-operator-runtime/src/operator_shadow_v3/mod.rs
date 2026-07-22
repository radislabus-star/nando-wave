mod actor;
mod bytecode;
mod execution;
mod report;

pub use execution::execute_bound_protocol_shadow_v3;

use nando_operator_kernel::BoundProtocolProgramV3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatorShadowVerdictV3 {
    Complete,
    AbstainUnsupportedCapability,
    AbstainProgramBudget,
    AbstainActor,
    AbstainVm,
    AbstainUnknownOpcode,
    ParityMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorShadowExecutionReceiptV3 {
    receipt_sha256: String,
    program_sha256: Option<String>,
    action_derivation_sha256: String,
    physical_action_sha256: String,
    mode_id_sha256: String,
    request_view_sha256: String,
    mapping_sha256: String,
    bytecode_sha256: Option<String>,
    actor_output_sha256: Option<String>,
    vm_output_sha256: Option<String>,
    actor_output_bytes: usize,
    vm_output_bytes: usize,
    verdict: OperatorShadowVerdictV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorShadowExecutionV3 {
    program: Option<BoundProtocolProgramV3>,
    bytecode: Option<Box<[u8]>>,
    actor_output: Option<String>,
    vm_output: Option<String>,
    receipt: OperatorShadowExecutionReceiptV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActorExecutionErrorV3 {
    UnsupportedCapability,
    DuplicateArgument,
    OutputBudget,
    Serialization,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BytecodeCompileErrorV3 {
    UnsupportedCapability,
    ProgramBudget,
    InvalidCommitment,
}

#[cfg(test)]
mod tests;
