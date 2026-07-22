use serde::Serialize;

use super::{
    BOUND_PROTOCOL_ACTION_SCHEMA_V3, BoundProtocolActionErrorV3, BoundProtocolActionInputV3,
    BoundProtocolActionV3, BoundProtocolArgumentV3, BoundProtocolValueV3,
    MAX_BOUND_PROTOCOL_ARGUMENTS_V3,
};
use crate::{canonical_json_sha256, valid_nonzero_sha256};

#[derive(Serialize)]
struct SemanticArgumentDigestV3<'a> {
    argument_ordinal: u16,
    source_role_id: u16,
    value: &'a BoundProtocolValueV3,
}

#[derive(Serialize)]
struct PhysicalArgumentDigestV3<'a> {
    argument_ordinal: u16,
    physical_name: &'a str,
    value: &'a BoundProtocolValueV3,
}

pub fn build_bound_protocol_action_v3(
    mut input: BoundProtocolActionInputV3,
) -> Result<BoundProtocolActionV3, BoundProtocolActionErrorV3> {
    validate_input(&mut input)?;
    let argument_inputs = std::mem::take(&mut input.arguments);
    let arguments = argument_inputs
        .into_iter()
        .map(|argument| BoundProtocolArgumentV3 {
            argument_ordinal: argument.argument_ordinal,
            source_role_id: argument.source_role_id,
            physical_name: argument.physical_name,
            value: argument.value,
        })
        .collect::<Vec<_>>();
    let semantic_action_sha256 = semantic_action_digest(&input, &arguments)?;
    let physical_action_sha256 =
        physical_action_digest(&input, &arguments, &semantic_action_sha256)?;
    let derivation_sha256 = derivation_digest(&input, &physical_action_sha256)?;

    Ok(BoundProtocolActionV3 {
        index_sha256: input.index_sha256,
        artifact_root_sha256: input.artifact_root_sha256,
        mode_id_sha256: input.mode_id_sha256,
        executable_mode_root_sha256: input.executable_mode_root_sha256,
        payload_root_sha256: input.payload_root_sha256,
        effect_law_id_sha256: input.effect_law_id_sha256,
        action_class_root_sha256: input.action_class_root_sha256,
        request_view_sha256: input.request_view_sha256,
        mapping_sha256: input.mapping_sha256,
        capability_id: input.capability_id,
        capability_kind: input.capability_kind,
        physical_symbol: input.physical_symbol,
        arguments: arguments.into_boxed_slice(),
        semantic_action_sha256,
        physical_action_sha256,
        derivation_sha256,
    })
}

fn validate_input(
    input: &mut BoundProtocolActionInputV3,
) -> Result<(), BoundProtocolActionErrorV3> {
    let commitments = [
        input.index_sha256.as_str(),
        input.artifact_root_sha256.as_str(),
        input.mode_id_sha256.as_str(),
        input.executable_mode_root_sha256.as_str(),
        input.payload_root_sha256.as_str(),
        input.effect_law_id_sha256.as_str(),
        input.action_class_root_sha256.as_str(),
        input.request_view_sha256.as_str(),
        input.mapping_sha256.as_str(),
    ];
    if commitments
        .iter()
        .any(|commitment| !valid_nonzero_sha256(commitment))
    {
        return Err(BoundProtocolActionErrorV3::InvalidCommitment);
    }
    if !valid_physical_name(&input.physical_symbol) {
        return Err(BoundProtocolActionErrorV3::InvalidPhysicalSymbol);
    }
    if input.arguments.is_empty() || input.arguments.len() > MAX_BOUND_PROTOCOL_ARGUMENTS_V3 {
        return Err(BoundProtocolActionErrorV3::InvalidArgument);
    }
    input
        .arguments
        .sort_by_key(|argument| argument.argument_ordinal);
    for (expected_ordinal, argument) in input.arguments.iter().enumerate() {
        if usize::from(argument.argument_ordinal) != expected_ordinal
            || !valid_physical_name(&argument.physical_name)
        {
            return Err(BoundProtocolActionErrorV3::InvalidArgument);
        }
    }
    if input.arguments.windows(2).any(|pair| {
        pair[0].source_role_id == pair[1].source_role_id
            || pair[0].physical_name == pair[1].physical_name
    }) {
        return Err(BoundProtocolActionErrorV3::InvalidArgument);
    }
    Ok(())
}

fn semantic_action_digest(
    input: &BoundProtocolActionInputV3,
    arguments: &[BoundProtocolArgumentV3],
) -> Result<String, BoundProtocolActionErrorV3> {
    let arguments = arguments
        .iter()
        .map(|argument| SemanticArgumentDigestV3 {
            argument_ordinal: argument.argument_ordinal,
            source_role_id: argument.source_role_id,
            value: &argument.value,
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        "nando.bound-protocol-semantic-action.v3",
        input.effect_law_id_sha256.as_str(),
        input.action_class_root_sha256.as_str(),
        input.capability_kind,
        arguments,
    ))
    .map_err(|_| BoundProtocolActionErrorV3::Serialization)
}

fn physical_action_digest(
    input: &BoundProtocolActionInputV3,
    arguments: &[BoundProtocolArgumentV3],
    semantic_action_sha256: &str,
) -> Result<String, BoundProtocolActionErrorV3> {
    let arguments = arguments
        .iter()
        .map(|argument| PhysicalArgumentDigestV3 {
            argument_ordinal: argument.argument_ordinal,
            physical_name: &argument.physical_name,
            value: &argument.value,
        })
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        "nando.bound-protocol-physical-action.v3",
        semantic_action_sha256,
        input.capability_kind,
        input.physical_symbol.as_str(),
        arguments,
    ))
    .map_err(|_| BoundProtocolActionErrorV3::Serialization)
}

fn derivation_digest(
    input: &BoundProtocolActionInputV3,
    physical_action_sha256: &str,
) -> Result<String, BoundProtocolActionErrorV3> {
    canonical_json_sha256(&(
        BOUND_PROTOCOL_ACTION_SCHEMA_V3,
        input.index_sha256.as_str(),
        input.artifact_root_sha256.as_str(),
        input.mode_id_sha256.as_str(),
        input.executable_mode_root_sha256.as_str(),
        input.payload_root_sha256.as_str(),
        input.request_view_sha256.as_str(),
        input.mapping_sha256.as_str(),
        input.capability_id,
        physical_action_sha256,
    ))
    .map_err(|_| BoundProtocolActionErrorV3::Serialization)
}

fn valid_physical_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}
