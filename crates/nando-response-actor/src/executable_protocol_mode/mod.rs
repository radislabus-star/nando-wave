mod compiler;
mod validation;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BindingValueTypeV1, ProtocolModeSetV2, canonical_json_bytes, valid_nonzero_sha256};

pub use compiler::compile_executable_protocol_mode_artifact_v3;

pub const PROTOCOL_FACET_PAYLOAD_SCHEMA_V3: &str = "nando.protocol-facet-payload.v3.f5a";
pub const EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3: &str =
    "nando.executable-protocol-mode-artifact.v3.f5a";

pub(super) const FACET_COMPILER_VERSION_V3: u16 = 1;
pub(super) const MAX_EXECUTABLE_MODES_V3: usize = 32;

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
    pub(super) argument_ordinal: u16,
    pub(super) source_role_id: u16,
    pub(super) value_type: BindingValueTypeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtocolFacetPayloadV3 {
    pub(super) schema: String,
    pub(super) compiler_version: u16,
    pub(super) source_protocol_facet_root_sha256: String,
    pub(super) capability_kind: ProtocolCapabilityKindV3,
    pub(super) physical_symbol_source: ProtocolPhysicalSymbolSourceV3,
    pub(super) arguments: Vec<ProtocolCapabilityArgumentV3>,
    pub(super) default_semantics: ProtocolDefaultSemanticsV3,
    pub(super) effect_law_id_sha256: String,
    pub(super) relation_identity_sha256: String,
    pub(super) effect_invariant_root_sha256: String,
    pub(super) action_class_root_sha256: String,
    pub(super) source_role_schema_root_sha256: String,
    pub(super) selector_program_root_sha256: String,
    pub(super) observed_emitted_types_root_sha256: String,
    pub(super) legacy_capability_contract_root_sha256: String,
    pub(super) argument_role_schema_root_sha256: String,
    pub(super) constant_contract_root_sha256: String,
    pub(super) structural_guard_root_sha256: String,
    pub(super) temporal_cardinality_contract_root_sha256: String,
    pub(super) payload_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableProtocolModeV3 {
    pub(super) source_mode_id_sha256: String,
    pub(super) source_physical_program_set_root_sha256: String,
    pub(super) payload: ProtocolFacetPayloadV3,
    pub(super) executable_mode_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableProtocolModeArtifactV3 {
    pub(super) schema: String,
    pub(super) compiler_version: u16,
    pub(super) artifact_sha256: String,
    pub(super) source_mode_set: ProtocolModeSetV2,
    pub(super) effect_law_payload: Value,
    pub(super) effect_law_payload_root_sha256: String,
    pub(super) modes: Vec<ExecutableProtocolModeV3>,
    pub(super) production_admissible: bool,
    pub(super) execution_authority: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolFacetEvidenceInputV3 {
    pub mode_id_sha256: String,
    pub canonical_facet_bytes: Vec<u8>,
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
        validation::validate_artifact(&artifact)?;
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
