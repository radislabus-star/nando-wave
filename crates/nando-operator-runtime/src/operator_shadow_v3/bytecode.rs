use nando_operator_kernel::{
    BOUND_PROTOCOL_PROGRAM_MAX_BYTECODE_BYTES_V3, BoundProtocolProgramV3, BoundProtocolValueV3,
    PROTOCOL_VM_HEADER_BYTES_V3, PROTOCOL_VM_MAGIC_V3, PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3,
    PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3, PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3,
    PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3, PROTOCOL_VM_OPCODE_BEGIN_CALL_V3,
    PROTOCOL_VM_OPCODE_EMIT_V3, PROTOCOL_VM_VERSION_V3, RuntimeCapabilityKindV3,
};

use super::BytecodeCompileErrorV3;

pub(super) fn compile_vm_bytecode_v3(
    program: &BoundProtocolProgramV3,
) -> Result<Box<[u8]>, BytecodeCompileErrorV3> {
    if program.capability_kind() != RuntimeCapabilityKindV3::Function {
        return Err(BytecodeCompileErrorV3::UnsupportedCapability);
    }
    let instruction_count = u16::try_from(program.arguments().len().saturating_add(2))
        .map_err(|_| BytecodeCompileErrorV3::ProgramBudget)?;
    let max_output_bytes = u32::try_from(program.max_output_bytes())
        .map_err(|_| BytecodeCompileErrorV3::ProgramBudget)?;
    let action_root = decode_sha256(program.action_derivation_sha256())?;
    let program_root = decode_sha256(program.program_sha256())?;
    let mut bytes = Vec::with_capacity(PROTOCOL_VM_HEADER_BYTES_V3 + 256);
    bytes.extend_from_slice(&PROTOCOL_VM_MAGIC_V3);
    bytes.extend_from_slice(&PROTOCOL_VM_VERSION_V3.to_le_bytes());
    bytes.extend_from_slice(&instruction_count.to_le_bytes());
    bytes.extend_from_slice(&max_output_bytes.to_le_bytes());
    bytes.extend_from_slice(&action_root);
    bytes.extend_from_slice(&program_root);

    bytes.push(PROTOCOL_VM_OPCODE_BEGIN_CALL_V3);
    bytes.push(1);
    push_text(&mut bytes, program.physical_symbol())?;
    for argument in program.arguments() {
        bytes.push(value_opcode(argument.value()));
        bytes.extend_from_slice(&argument.argument_ordinal().to_le_bytes());
        bytes.extend_from_slice(&argument.source_role_id().to_le_bytes());
        push_text(&mut bytes, argument.physical_name())?;
        push_value(&mut bytes, argument.value())?;
    }
    bytes.push(PROTOCOL_VM_OPCODE_EMIT_V3);
    if bytes.len() > BOUND_PROTOCOL_PROGRAM_MAX_BYTECODE_BYTES_V3 {
        return Err(BytecodeCompileErrorV3::ProgramBudget);
    }
    Ok(bytes.into_boxed_slice())
}

fn value_opcode(value: &BoundProtocolValueV3) -> u8 {
    match value {
        BoundProtocolValueV3::String(_) => PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3,
        BoundProtocolValueV3::Integer(_) => PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3,
        BoundProtocolValueV3::Boolean(_) => PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3,
        BoundProtocolValueV3::Identifier(_) => PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3,
    }
}

fn push_value(
    bytes: &mut Vec<u8>,
    value: &BoundProtocolValueV3,
) -> Result<(), BytecodeCompileErrorV3> {
    match value {
        BoundProtocolValueV3::String(value) | BoundProtocolValueV3::Identifier(value) => {
            push_text(bytes, value)
        }
        BoundProtocolValueV3::Integer(value) => {
            bytes.extend_from_slice(&value.to_le_bytes());
            Ok(())
        }
        BoundProtocolValueV3::Boolean(value) => {
            bytes.push(u8::from(*value));
            Ok(())
        }
    }
}

fn push_text(bytes: &mut Vec<u8>, value: &str) -> Result<(), BytecodeCompileErrorV3> {
    let len = u16::try_from(value.len()).map_err(|_| BytecodeCompileErrorV3::ProgramBudget)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn decode_sha256(value: &str) -> Result<[u8; 32], BytecodeCompileErrorV3> {
    if value.len() != 64 {
        return Err(BytecodeCompileErrorV3::InvalidCommitment);
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex(pair[0])? << 4) | hex(pair[1])?;
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(BytecodeCompileErrorV3::InvalidCommitment);
    }
    Ok(bytes)
}

fn hex(value: u8) -> Result<u8, BytecodeCompileErrorV3> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(BytecodeCompileErrorV3::InvalidCommitment),
    }
}
