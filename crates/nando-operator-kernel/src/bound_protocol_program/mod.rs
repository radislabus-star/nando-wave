mod builder;
mod report;

pub use builder::compile_bound_protocol_program_v3;

use crate::{BoundProtocolValueV3, RuntimeCapabilityKindV3};

pub const BOUND_PROTOCOL_PROGRAM_SCHEMA_V3: &str = "nando.bound-protocol-program.v3";
pub const BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3: usize = 16_384;
pub const BOUND_PROTOCOL_PROGRAM_MAX_BYTECODE_BYTES_V3: usize = 32_768;
pub const BOUND_PROTOCOL_PROGRAM_MAX_VALUE_BYTES_V3: usize = 16_384;

pub const PROTOCOL_VM_MAGIC_V3: [u8; 8] = *b"NWVMF5E3";
pub const PROTOCOL_VM_VERSION_V3: u16 = 3;
pub const PROTOCOL_VM_HEADER_BYTES_V3: usize = 80;
pub const PROTOCOL_VM_OPCODE_BEGIN_CALL_V3: u8 = 1;
pub const PROTOCOL_VM_OPCODE_ARGUMENT_STRING_V3: u8 = 2;
pub const PROTOCOL_VM_OPCODE_ARGUMENT_INTEGER_V3: u8 = 3;
pub const PROTOCOL_VM_OPCODE_ARGUMENT_BOOLEAN_V3: u8 = 4;
pub const PROTOCOL_VM_OPCODE_ARGUMENT_IDENTIFIER_V3: u8 = 5;
pub const PROTOCOL_VM_OPCODE_EMIT_V3: u8 = u8::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolProgramArgumentV3 {
    argument_ordinal: u16,
    source_role_id: u16,
    physical_name: String,
    value: BoundProtocolValueV3,
}

/// Request-owned typed actor program. It is deliberately not deserializable:
/// only the compiler from an opaque F5-D action can construct one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolProgramV3 {
    schema: &'static str,
    program_sha256: String,
    action_derivation_sha256: String,
    physical_action_sha256: String,
    mode_id_sha256: String,
    request_view_sha256: String,
    mapping_sha256: String,
    capability_kind: RuntimeCapabilityKindV3,
    physical_symbol: String,
    arguments: Box<[BoundProtocolProgramArgumentV3]>,
    max_output_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundProtocolProgramErrorV3 {
    UnsupportedCapability,
    ValueBudget,
    Serialization,
}
