use serde::{Deserialize, Serialize};

pub const MULTI_SOURCE_MAX_ROLE_NODES_V1: usize = 32;
pub const MULTI_SOURCE_MAX_RELATION_EDGES_V1: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceTypeClassV1 {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceContainerClassV1 {
    Scalar,
    Sequence,
    Mapping,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceCardinalityClassV1 {
    Zero,
    One,
    Many,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceTemporalClassV1 {
    Historical,
    Latest,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiSourceRoleNodeV1 {
    pub local_role_id: u16,
    pub source_ordinal: u16,
    pub value_ordinal: u16,
    pub type_class: MultiSourceTypeClassV1,
    pub container_class: MultiSourceContainerClassV1,
    pub cardinality_class: MultiSourceCardinalityClassV1,
    pub temporal_class: MultiSourceTemporalClassV1,
    pub depth_bucket: u8,
    pub structural_flags: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceRelationKindV1 {
    Contains,
    Precedes,
    SameOutput,
    LatestOutput,
    RequestReferencesRole,
    CapabilityPermitsRole,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultiSourceRelationEdgeV1 {
    pub relation: MultiSourceRelationKindV1,
    pub source_role_id: u16,
    pub target_role_id: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum MultiSourceExtractionStatusV1 {
    Complete,
    Censored { reason: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreActionMultiSourceTopologyV1 {
    pub extraction_status: MultiSourceExtractionStatusV1,
    pub grounded_output_count: u16,
    pub output_part_count: u16,
    pub roles: Vec<MultiSourceRoleNodeV1>,
    pub relations: Vec<MultiSourceRelationEdgeV1>,
}

impl PreActionMultiSourceTopologyV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.roles.len() > MULTI_SOURCE_MAX_ROLE_NODES_V1
            || self.relations.len() > MULTI_SOURCE_MAX_RELATION_EDGES_V1
            || !self
                .roles
                .windows(2)
                .all(|pair| pair[0].local_role_id < pair[1].local_role_id)
            || !self.relations.windows(2).all(|pair| pair[0] < pair[1])
            || self.relations.iter().any(|edge| {
                !self
                    .roles
                    .iter()
                    .any(|role| role.local_role_id == edge.source_role_id)
                    || !self
                        .roles
                        .iter()
                        .any(|role| role.local_role_id == edge.target_role_id)
            })
        {
            return Err("multi_source_topology_invalid");
        }
        Ok(())
    }
}
