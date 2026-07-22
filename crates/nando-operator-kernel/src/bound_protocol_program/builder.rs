use serde::Serialize;

use super::{
    BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3, BOUND_PROTOCOL_PROGRAM_MAX_VALUE_BYTES_V3,
    BOUND_PROTOCOL_PROGRAM_SCHEMA_V3, BoundProtocolProgramArgumentV3, BoundProtocolProgramErrorV3,
    BoundProtocolProgramV3,
};
use crate::{
    BoundProtocolActionV3, BoundProtocolValueV3, RuntimeCapabilityKindV3, canonical_json_sha256,
};

#[derive(Serialize)]
struct ProgramArgumentDigestV3<'a> {
    argument_ordinal: u16,
    source_role_id: u16,
    physical_name: &'a str,
    value: &'a BoundProtocolValueV3,
}

pub fn compile_bound_protocol_program_v3(
    action: &BoundProtocolActionV3,
) -> Result<BoundProtocolProgramV3, BoundProtocolProgramErrorV3> {
    if action.capability_kind() != RuntimeCapabilityKindV3::Function {
        return Err(BoundProtocolProgramErrorV3::UnsupportedCapability);
    }
    let arguments = action
        .arguments()
        .iter()
        .map(|argument| {
            if value_bytes(argument.value()) > BOUND_PROTOCOL_PROGRAM_MAX_VALUE_BYTES_V3 {
                return Err(BoundProtocolProgramErrorV3::ValueBudget);
            }
            Ok(BoundProtocolProgramArgumentV3 {
                argument_ordinal: argument.argument_ordinal(),
                source_role_id: argument.source_role_id(),
                physical_name: argument.physical_name().to_owned(),
                value: argument.value().clone(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let program_sha256 = program_digest(action, &arguments)?;
    Ok(BoundProtocolProgramV3 {
        schema: BOUND_PROTOCOL_PROGRAM_SCHEMA_V3,
        program_sha256,
        action_derivation_sha256: action.derivation_sha256().to_owned(),
        physical_action_sha256: action.physical_action_sha256().to_owned(),
        mode_id_sha256: action.mode_id_sha256().to_owned(),
        request_view_sha256: action.request_view_sha256().to_owned(),
        mapping_sha256: action.mapping_sha256().to_owned(),
        capability_kind: action.capability_kind(),
        physical_symbol: action.physical_symbol().to_owned(),
        arguments,
        max_output_bytes: BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3,
    })
}

fn program_digest(
    action: &BoundProtocolActionV3,
    arguments: &[BoundProtocolProgramArgumentV3],
) -> Result<String, BoundProtocolProgramErrorV3> {
    let arguments = arguments
        .iter()
        .map(|argument| ProgramArgumentDigestV3 {
            argument_ordinal: argument.argument_ordinal,
            source_role_id: argument.source_role_id,
            physical_name: &argument.physical_name,
            value: &argument.value,
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        BOUND_PROTOCOL_PROGRAM_SCHEMA_V3,
        action.derivation_sha256(),
        action.physical_action_sha256(),
        action.mode_id_sha256(),
        action.request_view_sha256(),
        action.mapping_sha256(),
        action.capability_kind(),
        action.physical_symbol(),
        arguments,
        BOUND_PROTOCOL_PROGRAM_MAX_OUTPUT_BYTES_V3,
    ))
    .map_err(|_| BoundProtocolProgramErrorV3::Serialization)
}

fn value_bytes(value: &BoundProtocolValueV3) -> usize {
    match value {
        BoundProtocolValueV3::String(value) | BoundProtocolValueV3::Identifier(value) => {
            value.len()
        }
        BoundProtocolValueV3::Integer(_) => size_of::<u64>(),
        BoundProtocolValueV3::Boolean(_) => size_of::<bool>(),
    }
}
