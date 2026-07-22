use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BindingProtocolCompileVerdictV2, BindingValueTypeV1, CANONICAL_EFFECT_LAW_SCHEMA_V3,
    ProtocolModeSetV2, ProtocolModeV2, canonical_json_bytes, canonical_json_sha256,
    valid_nonzero_sha256,
};

pub const PROTOCOL_FACET_PAYLOAD_SCHEMA_V3: &str = "nando.protocol-facet-payload.v3.f5a";
pub const EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3: &str =
    "nando.executable-protocol-mode-artifact.v3.f5a";
pub const FACET_COMPILER_VERSION_V3: u16 = 1;
pub const MAX_EXECUTABLE_MODES_V3: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolCapabilityKindV3 {
    Function,
    CustomTool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolDefaultSemanticsV3 {
    NoImplicitDefaults,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolPhysicalSymbolSourceV3 {
    CurrentAdvertisedCapabilitySurface,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolCapabilityArgumentV3 {
    argument_ordinal: u16,
    source_role_id: u16,
    value_type: BindingValueTypeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolFacetPayloadV3 {
    schema: String,
    compiler_version: u16,
    source_protocol_facet_root_sha256: String,
    capability_kind: ProtocolCapabilityKindV3,
    physical_symbol_source: ProtocolPhysicalSymbolSourceV3,
    arguments: Vec<ProtocolCapabilityArgumentV3>,
    default_semantics: ProtocolDefaultSemanticsV3,
    effect_law_id_sha256: String,
    relation_identity_sha256: String,
    effect_invariant_root_sha256: String,
    action_class_root_sha256: String,
    source_role_schema_root_sha256: String,
    selector_program_root_sha256: String,
    observed_emitted_types_root_sha256: String,
    legacy_capability_contract_root_sha256: String,
    argument_role_schema_root_sha256: String,
    constant_contract_root_sha256: String,
    structural_guard_root_sha256: String,
    temporal_cardinality_contract_root_sha256: String,
    payload_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableProtocolModeV3 {
    source_mode_id_sha256: String,
    source_physical_program_set_root_sha256: String,
    payload: ProtocolFacetPayloadV3,
    executable_mode_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableProtocolModeArtifactV3 {
    schema: String,
    compiler_version: u16,
    artifact_sha256: String,
    source_mode_set: ProtocolModeSetV2,
    effect_law_payload: Value,
    effect_law_payload_root_sha256: String,
    modes: Vec<ExecutableProtocolModeV3>,
    production_admissible: bool,
    execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableProtocolModeErrorV3 {
    InvalidModeSet,
    InvalidEffectLaw,
    MissingFacetEvidence,
    UnexpectedFacetEvidence,
    InvalidFacetEvidence,
    UnsupportedFacetShape,
    UncommittedPhysicalConstant,
    HashOnlyConstantCommitment,
    InvalidArtifact,
    Serialization,
}

#[derive(Serialize)]
struct ProtocolFacetPayloadDigest<'a> {
    schema: &'a str,
    compiler_version: u16,
    source_protocol_facet_root_sha256: &'a str,
    capability_kind: ProtocolCapabilityKindV3,
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

impl ExecutableProtocolModeArtifactV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ExecutableProtocolModeErrorV3> {
        canonical_json_bytes(self).map_err(|_| ExecutableProtocolModeErrorV3::Serialization)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        expected_artifact_root_sha256: &str,
    ) -> Result<Self, ExecutableProtocolModeErrorV3> {
        let artifact: Self = serde_json::from_slice(bytes)
            .map_err(|_| ExecutableProtocolModeErrorV3::InvalidArtifact)?;
        if !valid_nonzero_sha256(expected_artifact_root_sha256)
            || artifact.artifact_sha256 != expected_artifact_root_sha256
        {
            return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
        }
        validate_executable_protocol_mode_artifact_v3(&artifact)?;
        if artifact.canonical_bytes()? != bytes {
            return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
        }
        Ok(artifact)
    }

    #[must_use]
    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }

    #[must_use]
    pub fn source_mode_set(&self) -> &ProtocolModeSetV2 {
        &self.source_mode_set
    }

    #[must_use]
    pub fn effect_law_payload(&self) -> &Value {
        &self.effect_law_payload
    }

    #[must_use]
    pub fn modes(&self) -> &[ExecutableProtocolModeV3] {
        &self.modes
    }

    #[must_use]
    pub const fn production_admissible(&self) -> bool {
        self.production_admissible
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }
}

impl ExecutableProtocolModeV3 {
    #[must_use]
    pub fn source_mode_id_sha256(&self) -> &str {
        &self.source_mode_id_sha256
    }

    #[must_use]
    pub fn payload(&self) -> &ProtocolFacetPayloadV3 {
        &self.payload
    }

    #[must_use]
    pub fn executable_mode_root_sha256(&self) -> &str {
        &self.executable_mode_root_sha256
    }
}

impl ProtocolFacetPayloadV3 {
    #[must_use]
    pub const fn capability_kind(&self) -> ProtocolCapabilityKindV3 {
        self.capability_kind
    }

    #[must_use]
    pub const fn physical_symbol_source(&self) -> ProtocolPhysicalSymbolSourceV3 {
        self.physical_symbol_source
    }

    #[must_use]
    pub fn arguments(&self) -> &[ProtocolCapabilityArgumentV3] {
        &self.arguments
    }

    #[must_use]
    pub fn payload_root_sha256(&self) -> &str {
        &self.payload_root_sha256
    }
}

impl ProtocolCapabilityArgumentV3 {
    #[must_use]
    pub const fn argument_ordinal(&self) -> u16 {
        self.argument_ordinal
    }

    #[must_use]
    pub const fn source_role_id(&self) -> u16 {
        self.source_role_id
    }

    #[must_use]
    pub const fn value_type(&self) -> BindingValueTypeV1 {
        self.value_type
    }
}

pub fn build_protocol_facet_payload_v3(
    source_protocol_facet_root_sha256: String,
    capability_kind: ProtocolCapabilityKindV3,
    mode: &ProtocolModeV2,
) -> Result<ProtocolFacetPayloadV3, ExecutableProtocolModeErrorV3> {
    let mut payload = ProtocolFacetPayloadV3 {
        schema: PROTOCOL_FACET_PAYLOAD_SCHEMA_V3.to_owned(),
        compiler_version: FACET_COMPILER_VERSION_V3,
        source_protocol_facet_root_sha256,
        capability_kind,
        physical_symbol_source: ProtocolPhysicalSymbolSourceV3::CurrentAdvertisedCapabilitySurface,
        arguments: expected_protocol_arguments_v3(mode)?,
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
    validate_protocol_facet_payload_v3(&payload, mode)?;
    Ok(payload)
}

pub fn build_executable_protocol_mode_v3(
    mode: &ProtocolModeV2,
    payload: ProtocolFacetPayloadV3,
) -> Result<ExecutableProtocolModeV3, ExecutableProtocolModeErrorV3> {
    validate_protocol_facet_payload_v3(&payload, mode)?;
    let source_physical_program_set_root_sha256 =
        hash(&mode.program.capability_contract.physical_program_ids_sha256)?;
    let executable_mode_root_sha256 = executable_mode_digest(
        &mode.mode_id_sha256,
        &source_physical_program_set_root_sha256,
        payload.payload_root_sha256(),
    )?;
    Ok(ExecutableProtocolModeV3 {
        source_mode_id_sha256: mode.mode_id_sha256.clone(),
        source_physical_program_set_root_sha256,
        payload,
        executable_mode_root_sha256,
    })
}

pub fn build_executable_protocol_mode_artifact_v3(
    mode_set: &ProtocolModeSetV2,
    effect_law_payload: Value,
    mut modes: Vec<ExecutableProtocolModeV3>,
) -> Result<ExecutableProtocolModeArtifactV3, ExecutableProtocolModeErrorV3> {
    validate_executable_protocol_mode_source_v3(mode_set)?;
    let effect_law_payload_root_sha256 = hash(&effect_law_payload)?;
    validate_executable_effect_law_payload_v3(
        &effect_law_payload,
        &effect_law_payload_root_sha256,
        mode_set,
    )?;
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
    validate_executable_protocol_mode_artifact_v3(&artifact)?;
    Ok(artifact)
}

pub fn validate_executable_protocol_mode_source_v3(
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

pub fn validate_executable_effect_law_payload_v3(
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

pub fn validate_executable_protocol_mode_artifact_v3(
    artifact: &ExecutableProtocolModeArtifactV3,
) -> Result<(), ExecutableProtocolModeErrorV3> {
    validate_executable_protocol_mode_source_v3(&artifact.source_mode_set)?;
    if artifact.schema != EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3
        || artifact.compiler_version != FACET_COMPILER_VERSION_V3
        || artifact.production_admissible
        || artifact.execution_authority
        || artifact.effect_law_payload_root_sha256 != hash(&artifact.effect_law_payload)?
        || artifact.artifact_sha256 != artifact_digest(artifact)?
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
    }
    validate_executable_effect_law_payload_v3(
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
        validate_protocol_facet_payload_v3(&entry.payload, mode)?;
    }
    Ok(())
}

pub fn validate_protocol_facet_payload_v3(
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
        || payload.arguments != expected_protocol_arguments_v3(mode)?
    {
        return Err(ExecutableProtocolModeErrorV3::InvalidArtifact);
    }
    Ok(())
}

pub fn expected_protocol_arguments_v3(
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

fn facet_payload_digest(
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

fn executable_mode_digest(
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

fn artifact_digest(
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

fn hash<T: Serialize>(value: &T) -> Result<String, ExecutableProtocolModeErrorV3> {
    canonical_json_sha256(value).map_err(|_| ExecutableProtocolModeErrorV3::Serialization)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BindingCompletionStateV1, BindingPredicateV1, ProtocolArgumentRoleSchemaV2,
        ProtocolArgumentRoleV2, ProtocolCapabilityContractV2, ProtocolConstantContractV2,
        ProtocolModeProgramV2, ProtocolRoleCardinalityV2, ProtocolSelectorProgramV2,
        ProtocolSourceRoleSchemaV2, ProtocolSourceRoleV2, ProtocolStructuralGuardV2,
        ProtocolTemporalCardinalityContractV2, ProtocolValueContractV2, derived_mode_root_v2,
    };

    fn root(label: &str) -> String {
        canonical_json_sha256(&label).expect("fixture root")
    }

    fn mode() -> ProtocolModeV2 {
        let selector_program = ProtocolSelectorProgramV2 {
            predicates: vec![BindingPredicateV1::ValueType {
                value: BindingValueTypeV1::Integer,
            }],
            max_action_classes: 1,
        };
        let selector_root =
            derived_mode_root_v2("selector-program", &selector_program).expect("selector root");
        ProtocolModeV2 {
            mode_id_sha256: root("mode"),
            effect_law_id_sha256: root("effect-law"),
            relation_identity_sha256: root("relation"),
            protocol_facet_root_sha256: root("facet"),
            effect_invariant_root_sha256: root("invariant"),
            source_role_schema_root_sha256: root("roles"),
            selector_program_root_sha256: selector_root.clone(),
            observed_emitted_types_root_sha256: root("types"),
            capability_protocol_root_sha256: root("capability"),
            argument_role_schema_root_sha256: root("arguments"),
            constant_contract_root_sha256: root("constants"),
            structural_guard_root_sha256: root("guard"),
            temporal_cardinality_contract_root_sha256: root("temporal"),
            action_class_root_sha256: root("action"),
            program: ProtocolModeProgramV2 {
                source_role_schema: ProtocolSourceRoleSchemaV2 {
                    roles: vec![ProtocolSourceRoleV2 {
                        role_id: 0,
                        value_type: BindingValueTypeV1::Integer,
                        cardinality: ProtocolRoleCardinalityV2::OneActionClass,
                    }],
                },
                selector_program,
                value_contract: ProtocolValueContractV2 {
                    observed: BindingValueTypeV1::Integer,
                    emitted: BindingValueTypeV1::Integer,
                },
                capability_contract: ProtocolCapabilityContractV2 {
                    protocol_facet_root_sha256: root("facet"),
                    physical_program_ids_sha256: vec![root("physical")],
                },
                argument_role_schema: ProtocolArgumentRoleSchemaV2 {
                    roles: vec![ProtocolArgumentRoleV2 {
                        argument_ordinal: 0,
                        source_role_id: 0,
                    }],
                },
                constant_contract: ProtocolConstantContractV2 {
                    semantic_constants_sha256: Vec::new(),
                    protocol_noop_constants_sha256: Vec::new(),
                    execution_budget_roots_sha256: Vec::new(),
                    transport_default_roots_sha256: Vec::new(),
                },
                structural_guard: ProtocolStructuralGuardV2 {
                    relation_identity_sha256: root("relation"),
                    effect_invariant_root_sha256: root("invariant"),
                    selector_program_root_sha256: selector_root,
                },
                temporal_cardinality_contract: ProtocolTemporalCardinalityContractV2 {
                    completion_states: vec![BindingCompletionStateV1::Completed],
                    temporal_distances: vec![0],
                    event_candidate_cardinalities: vec![1],
                    require_unique_action_class: true,
                },
            },
            covered_positive_rows_sha256: vec![root("row")],
        }
    }

    #[test]
    fn facet_builder_is_closed_and_deterministic() {
        let mode = mode();
        let payload = build_protocol_facet_payload_v3(
            mode.protocol_facet_root_sha256.clone(),
            ProtocolCapabilityKindV3::Function,
            &mode,
        )
        .expect("payload");
        assert_eq!(payload.arguments().len(), 1);
        assert_eq!(payload.arguments()[0].source_role_id(), 0);
        assert!(valid_nonzero_sha256(payload.payload_root_sha256()));
        assert_eq!(validate_protocol_facet_payload_v3(&payload, &mode), Ok(()));
    }
}
