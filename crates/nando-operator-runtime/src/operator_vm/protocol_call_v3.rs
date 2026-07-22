use std::collections::BTreeMap;

use nando_operator_kernel::{
    BOUND_PROTOCOL_PROGRAM_MAX_BYTECODE_BYTES_V3, BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3,
    BoundProtocolProgramV3, PROTOCOL_VM_HEADER_BYTES_V3, PROTOCOL_VM_MAGIC_V3,
    PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3, PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3,
    PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3, PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3,
    PROTOCOL_VM_OPCODE_BEGIN_CALL_V3, PROTOCOL_VM_OPCODE_EMIT_V3, PROTOCOL_VM_VERSION_V3,
};
use serde_json::{Value, json};

use super::OperatorVmError;

pub fn execute_protocol_vm_bytecode_v3(
    program: &BoundProtocolProgramV3,
    bytecode: &[u8],
) -> Result<String, OperatorVmError> {
    if bytecode.len() < PROTOCOL_VM_HEADER_BYTES_V3
        || bytecode.len() > BOUND_PROTOCOL_PROGRAM_MAX_BYTECODE_BYTES_V3
        || bytecode[..8] != PROTOCOL_VM_MAGIC_V3
        || read_u16(bytecode, 8)? != PROTOCOL_VM_VERSION_V3
    {
        return Err(OperatorVmError::InvalidProgram);
    }
    let instruction_count = usize::from(read_u16(bytecode, 10)?);
    let output_budget =
        usize::try_from(read_u32(bytecode, 12)?).map_err(|_| OperatorVmError::OutputBudget)?;
    if instruction_count < 2
        || output_budget == 0
        || output_budget > BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3
        || bytecode[16..48] != decode_sha256(program.action_derivation_sha256())?
        || bytecode[48..80] != decode_sha256(program.program_sha256())?
    {
        return Err(OperatorVmError::InvalidProgram);
    }

    let mut cursor = PROTOCOL_VM_HEADER_BYTES_V3;
    expect_opcode(bytecode, &mut cursor, PROTOCOL_VM_OPCODE_BEGIN_CALL_V3)?;
    if read_u8(bytecode, &mut cursor)? != 1 {
        return Err(OperatorVmError::UnsupportedOpcode);
    }
    let symbol = read_text(bytecode, &mut cursor)?;
    if !valid_physical_name(&symbol) {
        return Err(OperatorVmError::InvalidProgram);
    }
    let mut arguments = BTreeMap::<String, Value>::new();
    for ordinal in 0..instruction_count.saturating_sub(2) {
        let opcode = read_u8(bytecode, &mut cursor)?;
        let encoded_ordinal = usize::from(read_u16_at_cursor(bytecode, &mut cursor)?);
        let _source_role_id = read_u16_at_cursor(bytecode, &mut cursor)?;
        if encoded_ordinal != ordinal {
            return Err(OperatorVmError::InvalidProgram);
        }
        let name = read_text(bytecode, &mut cursor)?;
        if !valid_physical_name(&name) {
            return Err(OperatorVmError::InvalidProgram);
        }
        let value = read_value(bytecode, &mut cursor, opcode)?;
        if arguments.insert(name, value).is_some() {
            return Err(OperatorVmError::AmbiguousResponse);
        }
    }
    expect_opcode(bytecode, &mut cursor, PROTOCOL_VM_OPCODE_EMIT_V3)?;
    if cursor != bytecode.len() {
        return Err(OperatorVmError::InvalidProgram);
    }
    let output = serde_json::to_string(&json!({
        "name": symbol,
        "arguments": arguments,
    }))
    .map_err(|_| OperatorVmError::InvalidProgram)?;
    if output.is_empty() || output.len() > output_budget {
        return Err(OperatorVmError::OutputBudget);
    }
    Ok(output)
}

fn decode_sha256(value: &str) -> Result<[u8; 32], OperatorVmError> {
    if value.len() != 64 {
        return Err(OperatorVmError::InvalidProgram);
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        bytes[index] = (hex(pair[0])? << 4) | hex(pair[1])?;
    }
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(OperatorVmError::InvalidProgram);
    }
    Ok(bytes)
}

fn hex(value: u8) -> Result<u8, OperatorVmError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(OperatorVmError::InvalidProgram),
    }
}

fn valid_physical_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}

fn read_value(bytecode: &[u8], cursor: &mut usize, opcode: u8) -> Result<Value, OperatorVmError> {
    match opcode {
        PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3 | PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3 => {
            Ok(Value::String(read_text(bytecode, cursor)?))
        }
        PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3 => {
            let end = cursor
                .checked_add(8)
                .filter(|end| *end <= bytecode.len())
                .ok_or(OperatorVmError::InvalidProgram)?;
            let value = u64::from_le_bytes(
                bytecode[*cursor..end]
                    .try_into()
                    .map_err(|_| OperatorVmError::InvalidProgram)?,
            );
            *cursor = end;
            Ok(Value::from(value))
        }
        PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3 => match read_u8(bytecode, cursor)? {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            _ => Err(OperatorVmError::InvalidProgram),
        },
        _ => Err(OperatorVmError::UnsupportedOpcode),
    }
}

fn expect_opcode(bytecode: &[u8], cursor: &mut usize, expected: u8) -> Result<(), OperatorVmError> {
    let actual = read_u8(bytecode, cursor)?;
    if actual == expected {
        Ok(())
    } else if matches!(
        actual,
        PROTOCOL_VM_OPCODE_BEGIN_CALL_V3
            | PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3
            | PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3
            | PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3
            | PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3
            | PROTOCOL_VM_OPCODE_EMIT_V3
    ) {
        Err(OperatorVmError::InvalidProgram)
    } else {
        Err(OperatorVmError::UnsupportedOpcode)
    }
}

fn read_text(bytecode: &[u8], cursor: &mut usize) -> Result<String, OperatorVmError> {
    let len = usize::from(read_u16_at_cursor(bytecode, cursor)?);
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= bytecode.len())
        .ok_or(OperatorVmError::InvalidProgram)?;
    let value = std::str::from_utf8(&bytecode[*cursor..end])
        .map_err(|_| OperatorVmError::InvalidProgram)?
        .to_owned();
    if value.is_empty() {
        return Err(OperatorVmError::InvalidProgram);
    }
    *cursor = end;
    Ok(value)
}

fn read_u8(bytecode: &[u8], cursor: &mut usize) -> Result<u8, OperatorVmError> {
    let value = *bytecode
        .get(*cursor)
        .ok_or(OperatorVmError::InvalidProgram)?;
    *cursor = cursor.saturating_add(1);
    Ok(value)
}

fn read_u16_at_cursor(bytecode: &[u8], cursor: &mut usize) -> Result<u16, OperatorVmError> {
    let value = read_u16(bytecode, *cursor)?;
    *cursor = cursor.saturating_add(2);
    Ok(value)
}

fn read_u16(bytecode: &[u8], offset: usize) -> Result<u16, OperatorVmError> {
    let end = offset
        .checked_add(2)
        .filter(|end| *end <= bytecode.len())
        .ok_or(OperatorVmError::InvalidProgram)?;
    Ok(u16::from_le_bytes(
        bytecode[offset..end]
            .try_into()
            .map_err(|_| OperatorVmError::InvalidProgram)?,
    ))
}

fn read_u32(bytecode: &[u8], offset: usize) -> Result<u32, OperatorVmError> {
    let end = offset
        .checked_add(4)
        .filter(|end| *end <= bytecode.len())
        .ok_or(OperatorVmError::InvalidProgram)?;
    Ok(u32::from_le_bytes(
        bytecode[offset..end]
            .try_into()
            .map_err(|_| OperatorVmError::InvalidProgram)?,
    ))
}
