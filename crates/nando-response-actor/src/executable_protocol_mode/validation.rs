use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use super::{
    EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3, ExecutableProtocolModeArtifactV3,
    ExecutableProtocolModeErrorV3, FACET_COMPILER_VERSION_V3, MAX_EXECUTABLE_MODES_V3,
    PROTOCOL_FACET_PAYLOAD_SCHEMA_V3, ProtocolCapabilityArgumentV3, ProtocolDefaultSemanticsV3,
    ProtocolFacetPayloadV3, ProtocolPhysicalSymbolSourceV3,
};
use crate::{
    BindingProtocolCompileVerdictV2, CANONICAL_EFFECT_LAW_SCHEMA_V3, ProtocolModeSetV2,
    ProtocolModeV2, canonical_json_sha256, valid_nonzero_sha256,
};

#[derive(Serialize)]
struct ProtocolFacetPayloadDigest<'a> {
    schema: &'a str,
    compiler_version: u16,
    source_protocol_facet_root_sha256: &'a str,
    capability_kind: super::ProtocolCapabilityKindV3,
    physical_symbol_source: ProtocolPhysicalSymbolSourceV3,
    arguments: &'a [ProtocolCapabilityArgumentV3],
    default_semantics: ProtocolDefaultSemanticsV3,
    effect_law_id_sha256: &'a str,
    relation_identity_sha256: &'a str,
    effect_invariant_root_sha256: &'a str,
    action_class_root_sha256: &'a str,
    source_role_schema_root_sha256: &'a str,
    selector_program_root_sha256: &'a str,
    observed_emitted_types_root_sha256: &'a str,
    legacy_capability_contract_root_sha256: &'a str,
    argument_role_schema_root_sha256: &'a str,
    constant_contract_root_sha256: &'a str,
    structural_guard_root_sha256: &'a str,
    temporal_cardinality_contract_root_sha256: &'a str,
}

pub(super) fn validate_source_mode_set(
    mode_set: &ProtocolModeSetV2,
) -> Result<(), ExecutableProtocolModeErrorV3> {
    let bytes = mode_set
        .canonical_bytes()
        .map_err(|_| ExecutableProtocolModeErrorV3::InvalidModeSet)?;
    if ProtocolModeSetV2::from_canonical_bytes(&bytes)
        .map_err(|_| ExecutableProtocolModeErrorV3::InvalidModeSet)?
        != *mode_set
        || mode_set.verdict != BindingProtocolCompileVerdictV2::ProtocolModeSet
        || mode_set.modes.is_empty()
        || mode_set.modes.len() > MAX_EXECUTABLE_MODES_V3
        || mode_set.production_admissible
        || mode_set.execution_authority
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidModeSet);
    }
    Ok(())
}

pub(super) fn validate_effect_law_payload(
    payload: &Value,
    payload_root_sha256: &str,
    mode_set: &ProtocolModeSetV2,
) -> Result<(), ExecutableProtocolModeErrorV3> {
    let object = payload
        .as_object()
        .ok_or(ExecutableProtocolModeErrorV3::InvalidEffectLaw)?;
    let expected_fields = BTreeSet::from([
        "schema",
        "ir_version",
        "dictionary_root_sha256",
        "quotient_hypothesis_root_sha256",
        "topology_nodes",
        "topology_edges",
        "relation_program",
        "effect_invariant_root_sha256",
        "preserved_frame_root_sha256",
        "action_equivalence_root_sha256",
    ]);
    if object.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_fields
        || object.get("schema").and_then(Value::as_str) != Some(CANONICAL_EFFECT_LAW_SCHEMA_V3)
        || object
            .get("ir_version")
            .and_then(Value::as_u64)
            .is_none_or(|version| version == 0)
        || object
            .get("topology_nodes")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        || object
            .get("relation_program")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        || payload_root_sha256 != mode_set.effect_law_id_sha256
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidEffectLaw);
    }
    let invariant = object
        .get("effect_invariant_root_sha256")
        .and_then(Value::as_str)
        .ok_or(ExecutableProtocolModeErrorV3::InvalidEffectLaw)?;
    let action_class = object
        .get("action_equivalence_root_sha256")
        .and_then(Value::as_str)
        .ok_or(ExecutableProtocolModeErrorV3::InvalidEffectLaw)?;
    if !valid_nonzero_sha256(invariant)
        || !valid_nonzero_sha256(action_class)
        || mode_set.modes.iter().any(|mode| {
            mode.effect_invariant_root_sha256 != invariant
                || mode.action_class_root_sha256 != action_class
        })
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidEffectLaw);
    }
    Ok(())
}

pub(super) fn validate_artifact(
    artifact: &ExecutableProtocolModeArtifactV3,
) -> Result<(), ExecutableProtocolModeErrorV3> {
    validate_source_mode_set(&artifact.source_mode_set)?;
    if artifact.schema != EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3
        || artifact.compiler_version != FACET_COMPILER_VERSION_V3
        || artifact.production_admissible
        || artifact.execution_authority
        || artifact.effect_law_payload_root_sha256 != hash(&artifact.effect_law_payload)?
        || artifact.artifact_sha256 != artifact_digest(artifact)?
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
    }
    validate_effect_law_payload(
        &artifact.effect_law_payload,
        &artifact.effect_law_payload_root_sha256,
        &artifact.source_mode_set,
    )?;
    if artifact.modes.len() != artifact.source_mode_set.modes.len()
        || artifact
            .modes
            .windows(2)
            .any(|pair| pair[0].source_mode_id_sha256 >= pair[1].source_mode_id_sha256)
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
    }
    for (entry, mode) in artifact
        .modes
        .iter()
        .zip(artifact.source_mode_set.modes.iter())
    {
        if entry.source_mode_id_sha256 != mode.mode_id_sha256
            || entry.source_physical_program_set_root_sha256
                != hash(&mode.program.capability_contract.physical_program_ids_sha256)?
            || entry.executable_mode_root_sha256
                != executable_mode_digest(
                    entry.source_mode_id_sha256.as_str(),
                    entry.source_physical_program_set_root_sha256.as_str(),
                    entry.payload.payload_root_sha256.as_str(),
                )?
        {
            return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
        }
        validate_facet_payload(&entry.payload, mode)?;
    }
    Ok(())
}

pub(super) fn validate_facet_payload(
    payload: &ProtocolFacetPayloadV3,
    mode: &ProtocolModeV2,
) -> Result<(), ExecutableProtocolModeErrorV3> {
    if payload.schema != PROTOCOL_FACET_PAYLOAD_SCHEMA_V3
        || payload.compiler_version != FACET_COMPILER_VERSION_V3
        || payload.default_semantics != ProtocolDefaultSemanticsV3::NoImplicitDefaults
        || payload.physical_symbol_source
            != ProtocolPhysicalSymbolSourceV3::CurrentAdvertisedCapabilitySurface
        || payload.source_protocol_facet_root_sha256 != mode.protocol_facet_root_sha256
        || payload.effect_law_id_sha256 != mode.effect_law_id_sha256
        || payload.relation_identity_sha256 != mode.relation_identity_sha256
        || payload.effect_invariant_root_sha256 != mode.effect_invariant_root_sha256
        || payload.action_class_root_sha256 != mode.action_class_root_sha256
        || payload.source_role_schema_root_sha256 != mode.source_role_schema_root_sha256
        || payload.selector_program_root_sha256 != mode.selector_program_root_sha256
        || payload.observed_emitted_types_root_sha256 != mode.observed_emitted_types_root_sha256
        || payload.legacy_capability_contract_root_sha256 != mode.capability_protocol_root_sha256
        || payload.argument_role_schema_root_sha256 != mode.argument_role_schema_root_sha256
        || payload.constant_contract_root_sha256 != mode.constant_contract_root_sha256
        || payload.structural_guard_root_sha256 != mode.structural_guard_root_sha256
        || payload.temporal_cardinality_contract_root_sha256
            != mode.temporal_cardinality_contract_root_sha256
        || payload.payload_root_sha256 != facet_payload_digest(payload)?
        || payload.arguments != expected_arguments(mode)?
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
    }
    Ok(())
}

pub(super) fn facet_payload_digest(
    payload: &ProtocolFacetPayloadV3,
) -> Result<String, ExecutableProtocolModeErrorV3> {
    hash(&ProtocolFacetPayloadDigest {
        schema: payload.schema.as_str(),
        compiler_version: payload.compiler_version,
        source_protocol_facet_root_sha256: payload.source_protocol_facet_root_sha256.as_str(),
        capability_kind: payload.capability_kind,
        physical_symbol_source: payload.physical_symbol_source,
        arguments: &payload.arguments,
        default_semantics: payload.default_semantics,
        effect_law_id_sha256: payload.effect_law_id_sha256.as_str(),
        relation_identity_sha256: payload.relation_identity_sha256.as_str(),
        effect_invariant_root_sha256: payload.effect_invariant_root_sha256.as_str(),
        action_class_root_sha256: payload.action_class_root_sha256.as_str(),
        source_role_schema_root_sha256: payload.source_role_schema_root_sha256.as_str(),
        selector_program_root_sha256: payload.selector_program_root_sha256.as_str(),
        observed_emitted_types_root_sha256: payload.observed_emitted_types_root_sha256.as_str(),
        legacy_capability_contract_root_sha256: payload
            .legacy_capability_contract_root_sha256
            .as_str(),
        argument_role_schema_root_sha256: payload.argument_role_schema_root_sha256.as_str(),
        constant_contract_root_sha256: payload.constant_contract_root_sha256.as_str(),
        structural_guard_root_sha256: payload.structural_guard_root_sha256.as_str(),
        temporal_cardinality_contract_root_sha256: payload
            .temporal_cardinality_contract_root_sha256
            .as_str(),
    })
}

pub(super) fn expected_arguments(
    mode: &ProtocolModeV2,
) -> Result<Vec<ProtocolCapabilityArgumentV3>, ExecutableProtocolModeErrorV3> {
    let source_roles = mode
        .program
        .source_role_schema
        .roles
        .iter()
        .map(|role| (role.role_id, role.value_type))
        .collect::<BTreeMap<_, _>>();
    let mut arguments = mode
        .program
        .argument_role_schema
        .roles
        .iter()
        .map(|argument| {
            let value_type = source_roles
                .get(&argument.source_role_id)
                .copied()
                .ok_or(ExecutableProtocolModeErrorV3::UnsupportedFacetShape)?;
            Ok(ProtocolCapabilityArgumentV3 {
                argument_ordinal: argument.argument_ordinal,
                source_role_id: argument.source_role_id,
                value_type,
            })
        })
        .collect::<Result<Vec<_>, ExecutableProtocolModeErrorV3>>()?;
    arguments.sort();
    if arguments.is_empty()
        || arguments.windows(2).any(|pair| {
            pair[0].argument_ordinal == pair[1].argument_ordinal
                || pair[0].source_role_id == pair[1].source_role_id
        })
    {
        return Err(ExecutableProtocolModeErrorV3::UnsupportedFacetShape);
    }
    Ok(arguments)
}

pub(super) fn executable_mode_digest(
    source_mode_id_sha256: &str,
    source_physical_program_set_root_sha256: &str,
    payload_root_sha256: &str,
) -> Result<String, ExecutableProtocolModeErrorV3> {
    hash(&(
        EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3,
        FACET_COMPILER_VERSION_V3,
        source_mode_id_sha256,
        source_physical_program_set_root_sha256,
        payload_root_sha256,
    ))
}

pub(super) fn artifact_digest(
    artifact: &ExecutableProtocolModeArtifactV3,
) -> Result<String, ExecutableProtocolModeErrorV3> {
    hash(&(
        artifact.schema.as_str(),
        artifact.compiler_version,
        artifact.source_mode_set.mode_set_sha256.as_str(),
        artifact.effect_law_payload_root_sha256.as_str(),
        &artifact.modes,
        artifact.production_admissible,
        artifact.execution_authority,
    ))
}

pub(super) fn hash<T: Serialize>(value: &T) -> Result<String, ExecutableProtocolModeErrorV3> {
    canonical_json_sha256(value).map_err(|_| ExecutableProtocolModeErrorV3::Serialization)
}
