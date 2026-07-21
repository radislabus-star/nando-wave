use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    ExecutableProtocolModeArtifactV3, ExecutableProtocolModeErrorV3, ProtocolCapabilityKindV3,
    ProtocolFacetEvidenceInputV3, ProtocolFacetPayloadV3,
};
use crate::effect_law_v3::{EFFECT_LAW_ACTION_PHASE_V3, EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3};
use crate::{
    AtomValueType, BindingValueTypeV1, CanonicalEffectLawV3, PROTOCOL_FACET_SCHEMA_V3,
    ProtocolModeSetV2, ProtocolModeV2, RelationAtom, canonical_json_bytes, canonical_json_sha256,
    valid_nonzero_sha256,
};
use nando_operator_kernel::{
    build_executable_protocol_mode_artifact_v3, build_executable_protocol_mode_v3,
    build_protocol_facet_payload_v3, expected_protocol_arguments_v3,
    validate_executable_effect_law_payload_v3, validate_executable_protocol_mode_source_v3,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ProtocolFacetSourceWireV3 {
    schema: String,
    physical_atoms: Vec<Value>,
    root_sha256: String,
}

pub fn compile_executable_protocol_mode_artifact_v3(
    mode_set: &ProtocolModeSetV2,
    effect_law: &CanonicalEffectLawV3,
    facet_evidence: Vec<ProtocolFacetEvidenceInputV3>,
) -> Result<ExecutableProtocolModeArtifactV3, ExecutableProtocolModeErrorV3> {
    validate_executable_protocol_mode_source_v3(mode_set)?;
    let effect_law_payload = serde_json::to_value(effect_law)
        .map_err(|_| ExecutableProtocolModeErrorV3::Serialization)?;
    let effect_law_payload_root_sha256 = hash(&effect_law_payload)?;
    validate_executable_effect_law_payload_v3(
        &effect_law_payload,
        &effect_law_payload_root_sha256,
        mode_set,
    )?;

    let mut evidence_by_mode = BTreeMap::new();
    for input in facet_evidence {
        if !valid_nonzero_sha256(&input.mode_id_sha256)
            || evidence_by_mode
                .insert(input.mode_id_sha256, input.canonical_facet_bytes)
                .is_some()
        {
            return Err(ExecutableProtocolModeErrorV3::UnexpectedFacetEvidence);
        }
    }

    let mut modes = Vec::with_capacity(mode_set.modes.len());
    for mode in &mode_set.modes {
        let source_bytes = evidence_by_mode
            .remove(&mode.mode_id_sha256)
            .ok_or(ExecutableProtocolModeErrorV3::MissingFacetEvidence)?;
        let source_facet = parse_source_facet(&source_bytes, mode)?;
        let payload = compile_facet_payload(mode, source_facet)?;
        modes.push(build_executable_protocol_mode_v3(mode, payload)?);
    }
    if !evidence_by_mode.is_empty() {
        return Err(ExecutableProtocolModeErrorV3::UnexpectedFacetEvidence);
    }
    build_executable_protocol_mode_artifact_v3(mode_set, effect_law_payload, modes)
}

fn parse_source_facet(
    bytes: &[u8],
    mode: &ProtocolModeV2,
) -> Result<ProtocolFacetSourceWireV3, ExecutableProtocolModeErrorV3> {
    let source: ProtocolFacetSourceWireV3 = serde_json::from_slice(bytes)
        .map_err(|_| ExecutableProtocolModeErrorV3::InvalidFacetEvidence)?;
    if source.schema != PROTOCOL_FACET_SCHEMA_V3
        || source.physical_atoms.is_empty()
        || source.physical_atoms.len() > EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3
        || source.root_sha256 != mode.protocol_facet_root_sha256
        || hash(&(PROTOCOL_FACET_SCHEMA_V3, &source.physical_atoms))? != source.root_sha256
        || canonical_json_bytes(&source)
            .map_err(|_| ExecutableProtocolModeErrorV3::Serialization)?
            != bytes
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidFacetEvidence);
    }
    let atom_keys = source
        .physical_atoms
        .iter()
        .map(|atom| {
            canonical_json_bytes(atom).map_err(|_| ExecutableProtocolModeErrorV3::Serialization)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if atom_keys.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ExecutableProtocolModeErrorV3::InvalidFacetEvidence);
    }
    Ok(source)
}

fn compile_facet_payload(
    mode: &ProtocolModeV2,
    source: ProtocolFacetSourceWireV3,
) -> Result<ProtocolFacetPayloadV3, ExecutableProtocolModeErrorV3> {
    if mode
        .program
        .constant_contract
        .semantic_constants_sha256
        .iter()
        .chain(
            mode.program
                .constant_contract
                .protocol_noop_constants_sha256
                .iter(),
        )
        .chain(
            mode.program
                .constant_contract
                .execution_budget_roots_sha256
                .iter(),
        )
        .chain(
            mode.program
                .constant_contract
                .transport_default_roots_sha256
                .iter(),
        )
        .next()
        .is_some()
    {
        return Err(ExecutableProtocolModeErrorV3::HashOnlyConstantCommitment);
    }

    let mut function_count = 0_usize;
    let mut custom_tool_count = 0_usize;
    let mut observed_role_types = Vec::new();
    for atom in &source.physical_atoms {
        if atom.get("phase").and_then(Value::as_u64) != Some(u64::from(EFFECT_LAW_ACTION_PHASE_V3))
        {
            continue;
        }
        let surface = atom
            .get("surface")
            .cloned()
            .ok_or(ExecutableProtocolModeErrorV3::InvalidFacetEvidence)?;
        let surface: RelationAtom = serde_json::from_value(surface)
            .map_err(|_| ExecutableProtocolModeErrorV3::InvalidFacetEvidence)?;
        match surface {
            RelationAtom::ActionFunction { .. } => function_count += 1,
            RelationAtom::ActionCustomTool { .. } => custom_tool_count += 1,
            RelationAtom::ActionRoleArgument { value_type, .. } => {
                let value_type = value_type
                    .and_then(binding_value_type)
                    .ok_or(ExecutableProtocolModeErrorV3::UnsupportedFacetShape)?;
                observed_role_types.push(value_type);
            }
            RelationAtom::ActionIntegerArgument { .. }
            | RelationAtom::ActionStringArgument { .. }
            | RelationAtom::ActionBooleanArgument { .. } => {
                // V2 stores only hashes for constants and has no executable ordinal binding.
                return Err(ExecutableProtocolModeErrorV3::UncommittedPhysicalConstant);
            }
            _ => {}
        }
    }
    let capability_kind = match (function_count, custom_tool_count) {
        (1, 0) => ProtocolCapabilityKindV3::Function,
        (0, 1) => ProtocolCapabilityKindV3::CustomTool,
        _ => return Err(ExecutableProtocolModeErrorV3::UnsupportedFacetShape),
    };

    let arguments = expected_protocol_arguments_v3(mode)?;
    let mut expected_role_types = arguments
        .iter()
        .map(|argument| argument.value_type())
        .collect::<Vec<_>>();
    expected_role_types.sort();
    observed_role_types.sort();
    if observed_role_types != expected_role_types {
        return Err(ExecutableProtocolModeErrorV3::UnsupportedFacetShape);
    }

    // Physical symbols prove the class during cold compilation but never enter payload bytes.
    build_protocol_facet_payload_v3(source.root_sha256, capability_kind, mode)
}

fn hash<T: Serialize>(value: &T) -> Result<String, ExecutableProtocolModeErrorV3> {
    canonical_json_sha256(value).map_err(|_| ExecutableProtocolModeErrorV3::Serialization)
}

fn binding_value_type(value: AtomValueType) -> Option<BindingValueTypeV1> {
    match value {
        AtomValueType::String => Some(BindingValueTypeV1::String),
        AtomValueType::Integer => Some(BindingValueTypeV1::Integer),
        AtomValueType::Boolean => Some(BindingValueTypeV1::Boolean),
        AtomValueType::Identifier => Some(BindingValueTypeV1::Identifier),
        AtomValueType::Collection => None,
    }
}
