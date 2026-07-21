use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCompletionStateV1 {
    Unresolved,
    Completed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCapabilityClassV1 {
    None,
    Single,
    Multiple,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingValueTypeV1 {
    String,
    Integer,
    Boolean,
    Identifier,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingSourceEventClassV1 {
    Textual,
    Structured,
    Mixed,
    Scalar,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCallLineageV1 {
    SameValueAcrossEvents,
    SharedOpaqueAnchor,
    Unlinked,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingRequestRelationV1 {
    Mentioned,
    NotMentioned,
    RequestAbsent,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BindingPredicateV1 {
    SourceEventClass { value: BindingSourceEventClassV1 },
    CallLineage { value: BindingCallLineageV1 },
    CapabilityClass { value: BindingCapabilityClassV1 },
    TemporalDistance { value: u16 },
    CompletionState { value: BindingCompletionStateV1 },
    EventCandidateCardinality { value: u16 },
    ValueType { value: BindingValueTypeV1 },
    RequestRelation { value: BindingRequestRelationV1 },
    TopologyNeighborhood { root_sha256: String },
}
