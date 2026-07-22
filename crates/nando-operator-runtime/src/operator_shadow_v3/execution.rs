use nando_operator_kernel::{
    BoundProtocolActionV3, BoundProtocolProgramErrorV3, compile_bound_protocol_program_v3,
    sha256_bytes,
};

use super::actor::execute_actor_program_v3;
use super::bytecode::compile_vm_bytecode_v3;
use super::{
    ActorExecutionErrorV3, BytecodeCompileErrorV3, OperatorShadowExecutionReceiptV3,
    OperatorShadowExecutionV3, OperatorShadowVerdictV3,
};
use crate::{OperatorVmError, execute_protocol_vm_bytecode_v3};

#[must_use]
pub fn execute_bound_protocol_shadow_v3(
    action: &BoundProtocolActionV3,
) -> OperatorShadowExecutionV3 {
    let program = match compile_bound_protocol_program_v3(action) {
        Ok(program) => program,
        Err(error) => return compile_failure(action, error),
    };
    let actor_output = match execute_actor_program_v3(&program) {
        Ok(output) => output,
        Err(error) => return actor_failure(action, program, error),
    };
    let bytecode = match compile_vm_bytecode_v3(&program) {
        Ok(bytecode) => bytecode,
        Err(error) => return bytecode_failure(action, program, actor_output, error),
    };
    let vm_output = match execute_protocol_vm_bytecode_v3(&program, &bytecode) {
        Ok(output) => output,
        Err(error) => return vm_failure(action, program, bytecode, actor_output, error),
    };
    let actor_output_sha256 = sha256_bytes(actor_output.as_bytes());
    let vm_output_sha256 = sha256_bytes(vm_output.as_bytes());
    let verdict = if actor_output.as_bytes() == vm_output.as_bytes() {
        OperatorShadowVerdictV3::Complete
    } else {
        OperatorShadowVerdictV3::ParityMismatch
    };
    let receipt = receipt(
        action,
        Some(&program),
        Some(&bytecode),
        Some(&actor_output_sha256),
        Some(&vm_output_sha256),
        actor_output.len(),
        vm_output.len(),
        verdict,
    );
    OperatorShadowExecutionV3 {
        program: Some(program),
        bytecode: Some(bytecode),
        actor_output: Some(actor_output),
        vm_output: Some(vm_output),
        receipt,
    }
}

fn compile_failure(
    action: &BoundProtocolActionV3,
    error: BoundProtocolProgramErrorV3,
) -> OperatorShadowExecutionV3 {
    let verdict = match error {
        BoundProtocolProgramErrorV3::UnsupportedCapability => {
            OperatorShadowVerdictV3::AbstainUnsupportedCapability
        }
        BoundProtocolProgramErrorV3::ValueBudget => OperatorShadowVerdictV3::AbstainProgramBudget,
        BoundProtocolProgramErrorV3::Serialization => OperatorShadowVerdictV3::AbstainActor,
    };
    empty(action, verdict)
}

fn actor_failure(
    action: &BoundProtocolActionV3,
    program: nando_operator_kernel::BoundProtocolProgramV3,
    error: ActorExecutionErrorV3,
) -> OperatorShadowExecutionV3 {
    let verdict = match error {
        ActorExecutionErrorV3::OutputBudget => OperatorShadowVerdictV3::AbstainProgramBudget,
        _ => OperatorShadowVerdictV3::AbstainActor,
    };
    let receipt = receipt(action, Some(&program), None, None, None, 0, 0, verdict);
    OperatorShadowExecutionV3 {
        program: Some(program),
        bytecode: None,
        actor_output: None,
        vm_output: None,
        receipt,
    }
}

fn bytecode_failure(
    action: &BoundProtocolActionV3,
    program: nando_operator_kernel::BoundProtocolProgramV3,
    actor_output: String,
    error: BytecodeCompileErrorV3,
) -> OperatorShadowExecutionV3 {
    let verdict = match error {
        BytecodeCompileErrorV3::UnsupportedCapability => {
            OperatorShadowVerdictV3::AbstainUnsupportedCapability
        }
        BytecodeCompileErrorV3::ProgramBudget => OperatorShadowVerdictV3::AbstainProgramBudget,
        BytecodeCompileErrorV3::InvalidCommitment => OperatorShadowVerdictV3::AbstainVm,
    };
    let actor_hash = sha256_bytes(actor_output.as_bytes());
    let receipt = receipt(
        action,
        Some(&program),
        None,
        Some(&actor_hash),
        None,
        actor_output.len(),
        0,
        verdict,
    );
    OperatorShadowExecutionV3 {
        program: Some(program),
        bytecode: None,
        actor_output: Some(actor_output),
        vm_output: None,
        receipt,
    }
}

fn vm_failure(
    action: &BoundProtocolActionV3,
    program: nando_operator_kernel::BoundProtocolProgramV3,
    bytecode: Box<[u8]>,
    actor_output: String,
    error: OperatorVmError,
) -> OperatorShadowExecutionV3 {
    let verdict = match error {
        OperatorVmError::UnsupportedOpcode => OperatorShadowVerdictV3::AbstainUnknownOpcode,
        OperatorVmError::OutputBudget => OperatorShadowVerdictV3::AbstainProgramBudget,
        _ => OperatorShadowVerdictV3::AbstainVm,
    };
    let actor_hash = sha256_bytes(actor_output.as_bytes());
    let receipt = receipt(
        action,
        Some(&program),
        Some(&bytecode),
        Some(&actor_hash),
        None,
        actor_output.len(),
        0,
        verdict,
    );
    OperatorShadowExecutionV3 {
        program: Some(program),
        bytecode: Some(bytecode),
        actor_output: Some(actor_output),
        vm_output: None,
        receipt,
    }
}

fn empty(
    action: &BoundProtocolActionV3,
    verdict: OperatorShadowVerdictV3,
) -> OperatorShadowExecutionV3 {
    OperatorShadowExecutionV3 {
        program: None,
        bytecode: None,
        actor_output: None,
        vm_output: None,
        receipt: receipt(action, None, None, None, None, 0, 0, verdict),
    }
}

#[allow(clippy::too_many_arguments)]
fn receipt(
    action: &BoundProtocolActionV3,
    program: Option<&nando_operator_kernel::BoundProtocolProgramV3>,
    bytecode: Option<&[u8]>,
    actor_output_sha256: Option<&str>,
    vm_output_sha256: Option<&str>,
    actor_output_bytes: usize,
    vm_output_bytes: usize,
    verdict: OperatorShadowVerdictV3,
) -> OperatorShadowExecutionReceiptV3 {
    let bytecode_sha256 = bytecode.map(sha256_bytes);
    let receipt_sha256 = receipt_digest(&[
        action.derivation_sha256(),
        program.map_or("none", |program| program.program_sha256()),
        bytecode_sha256.as_deref().unwrap_or("none"),
        actor_output_sha256.unwrap_or("none"),
        vm_output_sha256.unwrap_or("none"),
        verdict_label(verdict),
    ]);
    OperatorShadowExecutionReceiptV3 {
        receipt_sha256,
        program_sha256: program.map(|program| program.program_sha256().to_owned()),
        action_derivation_sha256: action.derivation_sha256().to_owned(),
        physical_action_sha256: action.physical_action_sha256().to_owned(),
        mode_id_sha256: action.mode_id_sha256().to_owned(),
        request_view_sha256: action.request_view_sha256().to_owned(),
        mapping_sha256: action.mapping_sha256().to_owned(),
        bytecode_sha256,
        actor_output_sha256: actor_output_sha256.map(str::to_owned),
        vm_output_sha256: vm_output_sha256.map(str::to_owned),
        actor_output_bytes,
        vm_output_bytes,
        verdict,
    }
}

fn receipt_digest(parts: &[&str]) -> String {
    let mut bytes = b"nando.operator-shadow-execution-receipt.v3".to_vec();
    for part in parts {
        bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
        bytes.extend_from_slice(part.as_bytes());
    }
    sha256_bytes(&bytes)
}

const fn verdict_label(verdict: OperatorShadowVerdictV3) -> &'static str {
    match verdict {
        OperatorShadowVerdictV3::Complete => "complete",
        OperatorShadowVerdictV3::AbstainUnsupportedCapability => "unsupported_capability",
        OperatorShadowVerdictV3::AbstainProgramBudget => "program_budget",
        OperatorShadowVerdictV3::AbstainActor => "actor",
        OperatorShadowVerdictV3::AbstainVm => "vm",
        OperatorShadowVerdictV3::AbstainUnknownOpcode => "unknown_opcode",
        OperatorShadowVerdictV3::ParityMismatch => "parity_mismatch",
    }
}
