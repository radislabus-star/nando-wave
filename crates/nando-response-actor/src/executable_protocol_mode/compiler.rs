use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::validation::{
    artifact_digest, executable_mode_digest, expected_arguments, facet_payload_digest, hash,
    validate_artifact, validate_effect_law_payload, validate_facet_payload,
    validate_source_mode_set,
};
use super::{
    EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3, ExecutableProtocolModeArtifactV3,
    ExecutableProtocolModeErrorV3, ExecutableProtocolModeV3, FACET_COMPILER_VERSION_V3,
    PROTOCOL_FACET_PAYLOAD_SCHEMA_V3, ProtocolCapabilityKindV3, ProtocolDefaultSemanticsV3,
    ProtocolFacetEvidenceInputV3, ProtocolFacetPayloadV3, ProtocolPhysicalSymbolSourceV3,
};
use crate::effect_law_v3::{EFFECT_LAW_ACTION_PHASE_V3, EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3};
use crate::{
    AtomValueType, BindingValueTypeV1, CanonicalEffectLawV3, PROTOCOL_FACET_SCHEMA_V3,
    ProtocolModeSetV2, ProtocolModeV2, RelationAtom, canonical_json_bytes, valid_nonzero_sha256,
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
    validate_source_mode_set(mode_set)?;
    let effect_law_payload = serde_json::to_value(effect_law)
        .map_err(|_| ExecutableProtocolModeErrorV3::Serialization)?;
    let effect_law_payload_root_sha256 = hash(&effect_law_payload)?;
    validate_effect_law_payload(
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
        let source_physical_program_set_root_sha256 =
            hash(&mode.program.capability_contract.physical_program_ids_sha256)?;
        let executable_mode_root_sha256 = executable_mode_digest(
            mode.mode_id_sha256.as_str(),
            source_physical_program_set_root_sha256.as_str(),
            payload.payload_root_sha256.as_str(),
        )?;
        modes.push(ExecutableProtocolModeV3 {
            source_mode_id_sha256: mode.mode_id_sha256.clone(),
            source_physical_program_set_root_sha256,
            payload,
            executable_mode_root_sha256,
        });
    }
    if !evidence_by_mode.is_empty() {
        return Err(ExecutableProtocolModeErrorV3::UnexpectedFacetEvidence);
    }
    modes.sort_by(|left, right| left.source_mode_id_sha256.cmp(&right.source_mode_id_sha256));

    let mut artifact = ExecutableProtocolModeArtifactV3 {
        schema: EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3.to_owned(),
        compiler_version: FACET_COMPILER_VERSION_V3,
        artifact_sha256: String::new(),
        source_mode_set: mode_set.clone(),
        effect_law_payload,
        effect_law_payload_root_sha256,
        modes,
        production_admissible: false,
        execution_authority: false,
    };
    artifact.artifact_sha256 = artifact_digest(&artifact)?;
    validate_artifact(&artifact)?;
    Ok(artifact)
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

    let arguments = expected_arguments(mode)?;
    let mut expected_role_types = arguments
        .iter()
        .map(|argument| argument.value_type)
        .collect::<Vec<_>>();
    expected_role_types.sort();
    observed_role_types.sort();
    if observed_role_types != expected_role_types {
        return Err(ExecutableProtocolModeErrorV3::UnsupportedFacetShape);
    }

    // Physical symbols prove the class during cold compilation but never enter V3 payload bytes.
    let mut payload = ProtocolFacetPayloadV3 {
        schema: PROTOCOL_FACET_PAYLOAD_SCHEMA_V3.to_owned(),
        compiler_version: FACET_COMPILER_VERSION_V3,
        source_protocol_facet_root_sha256: source.root_sha256,
        capability_kind,
        physical_symbol_source: ProtocolPhysicalSymbolSourceV3::CurrentAdvertisedCapabilitySurface,
        arguments,
        default_semantics: ProtocolDefaultSemanticsV3::NoImplicitDefaults,
        effect_law_id_sha256: mode.effect_law_id_sha256.clone(),
        relation_identity_sha256: mode.relation_identity_sha256.clone(),
        effect_invariant_root_sha256: mode.effect_invariant_root_sha256.clone(),
        action_class_root_sha256: mode.action_class_root_sha256.clone(),
        source_role_schema_root_sha256: mode.source_role_schema_root_sha256.clone(),
        selector_program_root_sha256: mode.selector_program_root_sha256.clone(),
        observed_emitted_types_root_sha256: mode.observed_emitted_types_root_sha256.clone(),
        legacy_capability_contract_root_sha256: mode.capability_protocol_root_sha256.clone(),
        argument_role_schema_root_sha256: mode.argument_role_schema_root_sha256.clone(),
        constant_contract_root_sha256: mode.constant_contract_root_sha256.clone(),
        structural_guard_root_sha256: mode.structural_guard_root_sha256.clone(),
        temporal_cardinality_contract_root_sha256: mode
            .temporal_cardinality_contract_root_sha256
            .clone(),
        payload_root_sha256: String::new(),
    };
    payload.payload_root_sha256 = facet_payload_digest(&payload)?;
    validate_facet_payload(&payload, mode)?;
    Ok(payload)
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
