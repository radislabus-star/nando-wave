use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ResponseProgram;

pub const INGRESS_EVENT_SCHEMA: &str = "nando.response-ingress-event.v1";
pub const RELATION_FRAME_SCHEMA: &str = "nando.response-relation-frame.v1";
pub const ROLE_HYPOTHESIS_SCHEMA: &str = "nando.response-role-hypothesis.v1";
pub const PROGRAM_CANDIDATE_SCHEMA: &str = "nando.response-program-candidate.v1";
pub const VERIFIER_RECEIPT_SCHEMA: &str = "nando.response-verifier-receipt.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficClass {
    Ordinary,
    Controlled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct IngressEvent {
    pub schema: String,
    pub event_id_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub parent_event_id_sha256: Option<String>,
    pub observed_at_unix_nanos: u64,
    pub traffic_class: TrafficClass,
    pub request_shape_sha256: String,
    pub evidence_ref_sha256: Option<String>,
    pub input_tokens: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationAtom {
    TypedSlot {
        slot_id: u16,
        value_type: AtomValueType,
        source: AtomSource,
        value_sha256: String,
    },
    SlotEquality {
        left_slot: u16,
        right_slot: u16,
    },
    UniqueSlot {
        slot_id: u16,
    },
    ObservationSelector {
        slot_id: u16,
        selector: ResponseValueSelector,
    },
    ObservationCallShape {
        value: String,
    },
    ActionFunction {
        value: String,
    },
    ActionCustomTool {
        value: String,
    },
    ActionInnerTool {
        value: String,
    },
    ActionRoleArgument {
        name: String,
        slot_id: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value_type: Option<AtomValueType>,
    },
    ActionIntegerArgument {
        name: String,
        value: u64,
    },
    ActionStringArgument {
        name: String,
        value: String,
    },
    ActionBooleanArgument {
        name: String,
        value: bool,
    },
    PlanState {
        step_count: u16,
        completed_count: u16,
        active_index: u16,
    },
    ActionPlanAdvance,
    ActionResultProjection {
        output_field: String,
        continuation_field: String,
        continuation_prefix: String,
    },
    ActionOutputProjection {
        output_field: String,
    },
    ActionJsonResultProjection,
    ActionValueProjection {
        format: crate::ValueProjectionFormat,
        #[serde(
            default,
            skip_serializing_if = "crate::CollectionOutputRenderer::is_direct"
        )]
        renderer: crate::CollectionOutputRenderer,
    },
    ActionStatusProjection {
        mapping: crate::ProjectStatusMapping,
    },
    CollectionShape {
        array_fields: u16,
        row_fields: u16,
    },
    RequestPhaseAtom {
        atom_id: u64,
    },
    ClientCapabilityAtom {
        atom_id: u64,
    },
    ReconstructedClientCapabilityAtom {
        atom_id: u64,
    },
    ToolKind {
        value: String,
    },
    OutputStatus {
        value: String,
    },
    TypedEquality {
        left_role: String,
        right_role: String,
    },
    Cardinality {
        role: String,
        count: u32,
    },
    TemporalEdge {
        predecessor: String,
        successor: String,
    },
    ResponseShape {
        value: String,
    },
    CompletionState {
        value: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomValueType {
    String,
    Integer,
    Boolean,
    Identifier,
    Collection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponseValueSelector {
    UniqueScalar {
        value_type: AtomValueType,
    },
    UniqueTurnScalar {
        value_type: AtomValueType,
    },
    ContentLinePrefix {
        prefix: String,
        value_type: AtomValueType,
    },
    JsonField {
        field: String,
        value_type: AtomValueType,
    },
    JsonScalarOrdinal {
        ordinal: u16,
        value_type: AtomValueType,
    },
    UniqueTurnJsonField {
        field: String,
        value_type: AtomValueType,
    },
    UniqueActiveTurnJsonField {
        field: String,
        value_type: AtomValueType,
    },
    RequestReferencedJsonField {
        value_type: AtomValueType,
    },
    RequestReferencedJsonFieldOrdinal {
        ordinal: u16,
        value_type: AtomValueType,
    },
    TurnOutputLine {
        output_ordinal: u16,
        line_index: u16,
        value_type: AtomValueType,
    },
    TurnOutputScalarOrdinal {
        output_ordinal: u16,
        scalar_ordinal: u16,
        value_type: AtomValueType,
    },
    LatestTurnOutputLine {
        line_index: u16,
        value_type: AtomValueType,
    },
    LatestTurnOutputScalarOrdinal {
        scalar_ordinal: u16,
        value_type: AtomValueType,
    },
    LatestTurnOutputScalarFromEnd {
        reverse_ordinal: u16,
        value_type: AtomValueType,
    },
    CommandOutputBody,
    RequestLastToken,
    RequestUniqueLiteral,
}

pub(crate) fn canonical_response_value_selector(selector: &ResponseValueSelector) -> String {
    serde_json::to_string(selector).expect("response value selector serializes")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AtomSource {
    Request,
    Observation,
    Action,
    Outcome,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationFrame {
    pub schema: String,
    pub frame_id_sha256: String,
    pub event_id_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub observed_at_unix_nanos: u64,
    #[serde(default)]
    pub estimated_input_tokens: u64,
    pub extractor_version: String,
    #[serde(default)]
    pub verifier_label: Option<bool>,
    pub atoms: Vec<RelationAtom>,
    pub evidence_ref_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    Selector,
    SourceValue,
    TargetValue,
    RecordOrCellId,
    StatusOrResult,
    ContinuationHandle,
    Collection,
    OrderingKey,
    FormatTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleHypothesis {
    pub schema: String,
    pub hypothesis_id_sha256: String,
    pub frame_family_id: u64,
    pub bindings: BTreeMap<SemanticRole, usize>,
    pub competing_binding_count: usize,
    pub description_length_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuardCandidate {
    pub required_atom_indices: Vec<usize>,
    pub forbidden_atom_indices: Vec<usize>,
    pub require_unique_selector: bool,
    pub max_evidence_age_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResponseProgramCandidate {
    pub schema: String,
    pub candidate_id_sha256: String,
    pub role_hypothesis_id_sha256: String,
    pub program: ResponseProgram,
    pub guard: GuardCandidate,
    pub phase_rank: u32,
    pub exact_checks: u32,
    pub description_length_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierReceipt {
    pub schema: String,
    pub verifier_id: String,
    pub verifier_version: String,
    pub candidate_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub output_sha256: String,
    pub verified_at_unix_nanos: u64,
    pub accepted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierConsensusVariant {
    pub verifier: VerifierProgram,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_layout_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_request_atom_ids: Vec<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerifierProgram {
    UniqueConsensus {
        variants: Vec<VerifierConsensusVariant>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        adapter_wave: Option<crate::program::ResponseAdapterWaveConsensus>,
    },
    AdvancePlan {
        function_name: String,
        require_explicit_tool_success: bool,
        require_canonical_plan: bool,
    },
    FunctionCallFromRoles {
        function_name: String,
        selector: ResponseValueSelector,
        role_arguments: BTreeMap<String, SemanticRole>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        role_argument_types: BTreeMap<String, AtomValueType>,
        integer_arguments: BTreeMap<String, u64>,
        #[serde(default)]
        string_arguments: BTreeMap<String, String>,
        #[serde(default)]
        boolean_arguments: BTreeMap<String, bool>,
        require_pending_state: bool,
        require_unique_handle: bool,
    },
    CustomToolCallFromRoles {
        custom_tool_name: String,
        inner_tool_name: String,
        selector: ResponseValueSelector,
        arguments: Vec<crate::ResponseArgument>,
        projection: crate::CustomToolResultProjection,
        require_pending_state: bool,
        require_unique_handle: bool,
    },
    ProjectSelectedValue {
        selector: ResponseValueSelector,
        format: crate::ValueProjectionFormat,
        #[serde(
            default,
            skip_serializing_if = "crate::CollectionOutputRenderer::is_direct"
        )]
        renderer: crate::CollectionOutputRenderer,
        completion_state: String,
        require_unique_value: bool,
    },
    ContinueHandle {
        require_observation_action_equality: bool,
        require_pending_state: bool,
        require_unique_handle: bool,
    },
    ProjectStatus {
        selector: ResponseValueSelector,
        mapping: crate::ProjectStatusMapping,
        #[serde(
            default,
            skip_serializing_if = "crate::CollectionOutputRenderer::is_direct"
        )]
        renderer: crate::CollectionOutputRenderer,
        completion_state: String,
        require_unique_value: bool,
    },
    ComposeCollection {
        steps: Vec<crate::CollectionProgramStep>,
        format: crate::ValueProjectionFormat,
        #[serde(
            default,
            skip_serializing_if = "crate::CollectionOutputRenderer::is_direct"
        )]
        renderer: crate::CollectionOutputRenderer,
        completion_state: String,
        max_items: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrozenSplitError {
    EmptySupport,
    EmptyFuture,
    FutureNotAfterBoundary,
    SessionLeakage,
    IntentLeakage,
}

pub fn validate_frozen_future_split(
    support: &[RelationFrame],
    future: &[RelationFrame],
    boundary_unix_nanos: u64,
) -> Result<(), FrozenSplitError> {
    if support.is_empty() {
        return Err(FrozenSplitError::EmptySupport);
    }
    if future.is_empty() {
        return Err(FrozenSplitError::EmptyFuture);
    }
    if support
        .iter()
        .any(|row| row.observed_at_unix_nanos > boundary_unix_nanos)
        || future
            .iter()
            .any(|row| row.observed_at_unix_nanos <= boundary_unix_nanos)
    {
        return Err(FrozenSplitError::FutureNotAfterBoundary);
    }
    let support_sessions = support
        .iter()
        .map(|row| row.session_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if future
        .iter()
        .any(|row| support_sessions.contains(row.session_id_sha256.as_str()))
    {
        return Err(FrozenSplitError::SessionLeakage);
    }
    let support_intents = support
        .iter()
        .map(|row| row.client_intent_id_sha256.as_str())
        .collect::<BTreeSet<_>>();
    if future
        .iter()
        .any(|row| support_intents.contains(row.client_intent_id_sha256.as_str()))
    {
        return Err(FrozenSplitError::IntentLeakage);
    }
    Ok(())
}
