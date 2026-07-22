mod builder;
mod report;

pub use builder::build_bound_protocol_action_v3;

use serde::Serialize;

use crate::{BindingValueTypeV1, RuntimeCapabilityKindV3};

pub const BOUND_PROTOCOL_ACTION_SCHEMA_V3: &str = "nando.bound-protocol-action.v3";
pub const MAX_BOUND_PROTOCOL_ARGUMENTS_V3: usize = 32;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum BoundProtocolValueV3 {
    String(String),
    Integer(u64),
    Boolean(bool),
    Identifier(String),
}

impl BoundProtocolValueV3 {
    #[must_use]
    pub const fn value_type(&self) -> BindingValueTypeV1 {
        match self {
            Self::String(_) => BindingValueTypeV1::String,
            Self::Integer(_) => BindingValueTypeV1::Integer,
            Self::Boolean(_) => BindingValueTypeV1::Boolean,
            Self::Identifier(_) => BindingValueTypeV1::Identifier,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolArgumentV3 {
    argument_ordinal: u16,
    source_role_id: u16,
    physical_name: String,
    value: BoundProtocolValueV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolActionV3 {
    index_sha256: String,
    artifact_root_sha256: String,
    mode_id_sha256: String,
    executable_mode_root_sha256: String,
    payload_root_sha256: String,
    effect_law_id_sha256: String,
    action_class_root_sha256: String,
    request_view_sha256: String,
    mapping_sha256: String,
    capability_id: u16,
    capability_kind: RuntimeCapabilityKindV3,
    physical_symbol: String,
    arguments: Box<[BoundProtocolArgumentV3]>,
    semantic_action_sha256: String,
    physical_action_sha256: String,
    derivation_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolArgumentInputV3 {
    pub argument_ordinal: u16,
    pub source_role_id: u16,
    pub physical_name: String,
    pub value: BoundProtocolValueV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundProtocolActionInputV3 {
    pub index_sha256: String,
    pub artifact_root_sha256: String,
    pub mode_id_sha256: String,
    pub executable_mode_root_sha256: String,
    pub payload_root_sha256: String,
    pub effect_law_id_sha256: String,
    pub action_class_root_sha256: String,
    pub request_view_sha256: String,
    pub mapping_sha256: String,
    pub capability_id: u16,
    pub capability_kind: RuntimeCapabilityKindV3,
    pub physical_symbol: String,
    pub arguments: Vec<BoundProtocolArgumentInputV3>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundProtocolActionErrorV3 {
    InvalidCommitment,
    InvalidPhysicalSymbol,
    InvalidArgument,
    Serialization,
}
