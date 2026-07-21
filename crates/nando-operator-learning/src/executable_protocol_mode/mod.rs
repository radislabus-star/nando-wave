mod compiler;

pub use compiler::compile_executable_protocol_mode_artifact_v3;
pub use nando_operator_kernel::{
    EXECUTABLE_PROTOCOL_MODE_ARTIFACT_SCHEMA_V3, ExecutableProtocolModeArtifactV3,
    ExecutableProtocolModeErrorV3, ExecutableProtocolModeV3, PROTOCOL_FACET_PAYLOAD_SCHEMA_V3,
    ProtocolCapabilityArgumentV3, ProtocolCapabilityKindV3, ProtocolDefaultSemanticsV3,
    ProtocolFacetPayloadV3, ProtocolPhysicalSymbolSourceV3,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolFacetEvidenceInputV3 {
    pub mode_id_sha256: String,
    pub canonical_facet_bytes: Vec<u8>,
}
