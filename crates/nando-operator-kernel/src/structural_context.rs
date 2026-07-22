use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    BindingCallLineageV1, BindingCapabilityClassV1, BindingCompletionStateV1,
    BindingRequestRelationV1, BindingSourceEventClassV1, BindingValueTypeV1, canonical_json_sha256,
    sha256_bytes,
};

pub const CANONICAL_RUNTIME_STRUCTURAL_VIEW_SCHEMA_V3: &str =
    "nando.canonical-runtime-structural-view.v3";
pub const CANONICAL_RUNTIME_REQUEST_VIEW_SCHEMA_V3: &str =
    "nando.canonical-runtime-request-view.v3";
pub const RUNTIME_CONTEXT_EXTRACTION_RECEIPT_SCHEMA_V3: &str =
    "nando.runtime-context-extraction-receipt.v3";

pub const MAX_STRUCTURAL_JSON_NODES_V3: usize = 16_384;
pub const MAX_STRUCTURAL_TEXT_BYTES_V3: usize = 256 * 1024;
pub const MAX_STRUCTURAL_RECENT_EVENTS_V3: usize = 32;
pub const MAX_STRUCTURAL_ROLE_CANDIDATES_V3: usize = 8_192;
pub const MAX_STRUCTURAL_RELATIONS_V3: usize = 65_536;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuralExtractionBudgetV3 {
    pub max_json_nodes: usize,
    pub max_text_bytes: usize,
    pub max_recent_events: usize,
    pub max_role_candidates: usize,
    pub max_relations: usize,
}

impl StructuralExtractionBudgetV3 {
    pub fn validate(self) -> Result<Self, StructuralExtractionErrorV3> {
        if self.max_json_nodes == 0
            || self.max_json_nodes > MAX_STRUCTURAL_JSON_NODES_V3
            || self.max_text_bytes == 0
            || self.max_text_bytes > MAX_STRUCTURAL_TEXT_BYTES_V3
            || self.max_recent_events == 0
            || self.max_recent_events > MAX_STRUCTURAL_RECENT_EVENTS_V3
            || self.max_role_candidates == 0
            || self.max_role_candidates > MAX_STRUCTURAL_ROLE_CANDIDATES_V3
            || self.max_relations == 0
            || self.max_relations > MAX_STRUCTURAL_RELATIONS_V3
        {
            return Err(StructuralExtractionErrorV3::InvalidBudget);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralExtractionScopeV3 {
    FrozenEvidence,
    PreActionRuntime,
}

impl StructuralExtractionScopeV3 {
    fn excludes_root_key(self, key: &str) -> bool {
        self == Self::PreActionRuntime
            && matches!(
                key,
                "action"
                    | "metadata"
                    | "model"
                    | "teacher"
                    | "teacher_response"
                    | "expected_action"
                    | "state_after"
                    | "target_patch"
                    | "tools"
                    | "verifier_receipt"
            )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct StructuralContextV3 {
    pub call_shape_count: u16,
    pub capability_count: u16,
    pub completion_state: BindingCompletionStateV1,
    pub temporal_relation_count: u16,
    pub cardinality_relation_count: u16,
    pub topology_neighborhood_root_sha256: String,
}

impl StructuralContextV3 {
    pub fn validate(&self) -> Result<(), StructuralExtractionErrorV3> {
        if !is_sha256(&self.topology_neighborhood_root_sha256) {
            return Err(StructuralExtractionErrorV3::InvalidContext);
        }
        Ok(())
    }

    fn capability_class(&self) -> BindingCapabilityClassV1 {
        match self.capability_count.max(self.call_shape_count) {
            0 => BindingCapabilityClassV1::None,
            1 => BindingCapabilityClassV1::Single,
            _ => BindingCapabilityClassV1::Multiple,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct StructuralCandidateFeaturesV3 {
    pub source_event_class: BindingSourceEventClassV1,
    pub call_lineage: BindingCallLineageV1,
    pub capability_class: BindingCapabilityClassV1,
    pub temporal_distance: u16,
    pub completion_state: BindingCompletionStateV1,
    pub event_candidate_cardinality: u16,
    pub value_type: BindingValueTypeV1,
    pub request_relation: BindingRequestRelationV1,
    pub topology_neighborhood_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralCandidateObservationV3 {
    pub source_role_id: u16,
    pub value_sha256: String,
    /// Ephemeral normalized values from the bounded walk. Canonical views and
    /// durable receipts deliberately exclude these bytes.
    pub normalized_values: Box<[String]>,
    pub features: StructuralCandidateFeaturesV3,
    pub occurrence_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalStructuralSourceBindingV3 {
    pub source_role_id: u16,
    pub canonical_role_id: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralRelationKindV3 {
    SameValue,
    SameCallLineage,
    SameTopologyNeighborhood,
    AdjacentTemporalDistance,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StructuralSourceRelationV3 {
    pub left_source_role_id: u16,
    pub right_source_role_id: u16,
    pub relation: StructuralRelationKindV3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralExtractionV3 {
    pub candidates: Vec<StructuralCandidateObservationV3>,
    pub relations: Vec<StructuralSourceRelationV3>,
    pub json_nodes_visited: usize,
    pub text_bytes_visited: usize,
    pub candidates_before_budget: usize,
    pub candidate_budget_exhausted: bool,
    pub relation_budget_exhausted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalStructuralRoleV3 {
    pub role_id: u16,
    pub features: StructuralCandidateFeaturesV3,
    pub occurrence_count: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalStructuralRelationV3 {
    pub left_role_id: u16,
    pub right_role_id: u16,
    pub relation: StructuralRelationKindV3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalRuntimeStructuralViewV3 {
    pub schema: String,
    pub structural_view_sha256: String,
    pub context: StructuralContextV3,
    pub roles: Vec<CanonicalStructuralRoleV3>,
    pub relations: Vec<CanonicalStructuralRelationV3>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProjectionV3 {
    Responses,
    ChatCompletions,
    TransitionApi,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCapabilityKindV3 {
    Function,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RuntimeCapabilityDescriptorV3 {
    pub capability_id: u16,
    pub kind: RuntimeCapabilityKindV3,
    pub argument_types: Vec<BindingValueTypeV1>,
    pub required_arity: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalRuntimeRequestViewV3 {
    pub schema: String,
    pub request_view_sha256: String,
    pub projection: RuntimeProjectionV3,
    pub request_relation_atoms: Vec<u64>,
    pub structural: CanonicalRuntimeStructuralViewV3,
    pub capabilities: Vec<RuntimeCapabilityDescriptorV3>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeContextExtractionVerdictV3 {
    Complete,
    AbstainBudgetExhausted,
    AbstainInvalidRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtractionReceiptV3 {
    pub schema: String,
    pub receipt_sha256: String,
    pub request_sha256: String,
    pub request_view_sha256: Option<String>,
    pub projection: RuntimeProjectionV3,
    pub verdict: RuntimeContextExtractionVerdictV3,
    pub json_nodes_visited: usize,
    pub text_bytes_visited: usize,
    pub role_candidates: usize,
    pub relations: usize,
    pub advertised_capabilities: usize,
    pub extraction_count: u8,
    pub teacher_or_action_fields_consumed: u8,
    pub raw_payloads_persisted: u8,
    pub execution_authority: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralExtractionErrorV3 {
    InvalidBudget,
    InvalidContext,
    InvalidDigest,
    InvalidSource,
    BudgetExhausted,
    Serialization,
}

#[derive(Clone)]
struct RawCandidate {
    normalized: String,
    value_sha256: String,
    value_type: BindingValueTypeV1,
    event_key: u32,
    temporal_distance: u16,
}

#[derive(Default)]
struct EventEvidence {
    anchors: BTreeSet<String>,
    event_class: Option<BindingSourceEventClassV1>,
    topology_neighborhood_root_sha256: Option<String>,
}

struct ExtractionState {
    budget: StructuralExtractionBudgetV3,
    scope: StructuralExtractionScopeV3,
    request_tokens: BTreeSet<String>,
    request_present: bool,
    json_nodes_visited: usize,
    text_bytes_visited: usize,
    next_event_key: u32,
    raw_candidates: Vec<RawCandidate>,
    events: BTreeMap<u32, EventEvidence>,
    event_candidate_values: BTreeMap<u32, BTreeSet<String>>,
    candidate_events: BTreeMap<String, BTreeSet<u32>>,
    stopped: bool,
}

#[derive(Clone)]
struct EventContext {
    event_key: u32,
    temporal_distance: u16,
}

#[derive(Clone, Default)]
struct ShapeStats {
    strings: usize,
    structured: usize,
    scalars: usize,
}

impl ShapeStats {
    fn merge(&mut self, other: &Self) {
        self.strings = self.strings.saturating_add(other.strings);
        self.structured = self.structured.saturating_add(other.structured);
        self.scalars = self.scalars.saturating_add(other.scalars);
    }

    fn event_class(&self) -> BindingSourceEventClassV1 {
        match (
            self.strings > 0,
            self.structured > 1,
            self.scalars > self.strings,
        ) {
            (true, false, false) => BindingSourceEventClassV1::Textual,
            (false, true, _) => BindingSourceEventClassV1::Structured,
            (true, true, _) => BindingSourceEventClassV1::Mixed,
            (false, false, true) => BindingSourceEventClassV1::Scalar,
            _ => BindingSourceEventClassV1::Unknown,
        }
    }
}

struct NodeSummary {
    shape: String,
    stats: ShapeStats,
    leading_singleton_collapsed_stats: ShapeStats,
}

impl NodeSummary {
    fn stopped() -> Self {
        Self {
            shape: "budget_exhausted".to_owned(),
            stats: ShapeStats::default(),
            leading_singleton_collapsed_stats: ShapeStats::default(),
        }
    }
}

pub fn extract_structural_surface_v3(
    request_text: &str,
    payload: &Value,
    context: StructuralContextV3,
    budget: StructuralExtractionBudgetV3,
    scope: StructuralExtractionScopeV3,
) -> Result<StructuralExtractionV3, StructuralExtractionErrorV3> {
    let budget = budget.validate()?;
    context.validate()?;
    let request_tokens = tokenize_candidate_text(request_text)
        .into_iter()
        .map(|(token, _)| token)
        .collect();
    let mut state = ExtractionState {
        budget,
        scope,
        request_tokens,
        request_present: !request_text.trim().is_empty(),
        json_nodes_visited: 0,
        text_bytes_visited: 0,
        next_event_key: 1,
        raw_candidates: Vec::new(),
        events: BTreeMap::new(),
        event_candidate_values: BTreeMap::new(),
        candidate_events: BTreeMap::new(),
        stopped: false,
    };
    visit_value(payload, None, 0, &mut state);

    let candidates_before_budget = state.raw_candidates.len();
    let mut candidates = materialize_candidates(&context, &state);
    let candidate_budget_exhausted = state.stopped || candidates.len() > budget.max_role_candidates;
    candidates.truncate(budget.max_role_candidates);
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.source_role_id =
            u16::try_from(index).map_err(|_| StructuralExtractionErrorV3::BudgetExhausted)?;
    }
    let (relations, relation_budget_exhausted) =
        source_relations(&candidates, budget.max_relations);
    Ok(StructuralExtractionV3 {
        candidates,
        relations,
        json_nodes_visited: state.json_nodes_visited,
        text_bytes_visited: state.text_bytes_visited,
        candidates_before_budget,
        candidate_budget_exhausted,
        relation_budget_exhausted,
    })
}

pub fn canonicalize_runtime_structural_view_v3(
    context: StructuralContextV3,
    extraction: &StructuralExtractionV3,
) -> Result<CanonicalRuntimeStructuralViewV3, StructuralExtractionErrorV3> {
    canonicalize_runtime_structural_projection_v3(context, extraction)
        .map(|projection| projection.0)
}

pub fn canonicalize_runtime_structural_projection_v3(
    context: StructuralContextV3,
    extraction: &StructuralExtractionV3,
) -> Result<
    (
        CanonicalRuntimeStructuralViewV3,
        Box<[CanonicalStructuralSourceBindingV3]>,
    ),
    StructuralExtractionErrorV3,
> {
    context.validate()?;
    if extraction.candidate_budget_exhausted || extraction.relation_budget_exhausted {
        return Err(StructuralExtractionErrorV3::BudgetExhausted);
    }
    let mut ordered = extraction.candidates.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.features
            .cmp(&right.features)
            .then_with(|| left.occurrence_count.cmp(&right.occurrence_count))
            .then_with(|| left.value_sha256.cmp(&right.value_sha256))
            .then_with(|| left.source_role_id.cmp(&right.source_role_id))
    });
    let mut source_to_canonical = BTreeMap::new();
    let mut roles = Vec::with_capacity(ordered.len());
    let mut source_bindings = Vec::with_capacity(ordered.len());
    for (index, source) in ordered.into_iter().enumerate() {
        let role_id =
            u16::try_from(index).map_err(|_| StructuralExtractionErrorV3::BudgetExhausted)?;
        if source_to_canonical
            .insert(source.source_role_id, role_id)
            .is_some()
        {
            return Err(StructuralExtractionErrorV3::InvalidSource);
        }
        source_bindings.push(CanonicalStructuralSourceBindingV3 {
            source_role_id: source.source_role_id,
            canonical_role_id: role_id,
        });
        roles.push(CanonicalStructuralRoleV3 {
            role_id,
            features: source.features.clone(),
            occurrence_count: source.occurrence_count,
        });
    }
    let relations = extraction
        .relations
        .iter()
        .map(|edge| {
            let left = source_to_canonical
                .get(&edge.left_source_role_id)
                .copied()
                .ok_or(StructuralExtractionErrorV3::InvalidSource)?;
            let right = source_to_canonical
                .get(&edge.right_source_role_id)
                .copied()
                .ok_or(StructuralExtractionErrorV3::InvalidSource)?;
            Ok(CanonicalStructuralRelationV3 {
                left_role_id: left.min(right),
                right_role_id: left.max(right),
                relation: edge.relation,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let relations = canonicalize_relation_components(relations, &roles);
    let mut view = CanonicalRuntimeStructuralViewV3 {
        schema: CANONICAL_RUNTIME_STRUCTURAL_VIEW_SCHEMA_V3.to_owned(),
        structural_view_sha256: String::new(),
        context,
        roles,
        relations,
    };
    view.structural_view_sha256 = structural_view_digest(&view)?;
    source_bindings.sort_by_key(|binding| binding.canonical_role_id);
    Ok((view, source_bindings.into_boxed_slice()))
}

fn canonicalize_relation_components(
    relations: Vec<CanonicalStructuralRelationV3>,
    roles: &[CanonicalStructuralRoleV3],
) -> Vec<CanonicalStructuralRelationV3> {
    let mut adjacency = BTreeMap::<StructuralRelationKindV3, BTreeMap<u16, BTreeSet<u16>>>::new();
    let mut output = BTreeSet::new();
    let mut has_temporal_adjacency = false;
    for relation in relations {
        if relation.relation == StructuralRelationKindV3::AdjacentTemporalDistance {
            has_temporal_adjacency = true;
            continue;
        }
        adjacency
            .entry(relation.relation)
            .or_default()
            .entry(relation.left_role_id)
            .or_default()
            .insert(relation.right_role_id);
        adjacency
            .entry(relation.relation)
            .or_default()
            .entry(relation.right_role_id)
            .or_default()
            .insert(relation.left_role_id);
    }
    for (kind, graph) in adjacency {
        let mut visited = BTreeSet::new();
        for start in graph.keys().copied() {
            if !visited.insert(start) {
                continue;
            }
            let mut frontier = vec![start];
            let mut component = BTreeSet::from([start]);
            while let Some(role) = frontier.pop() {
                for neighbor in graph.get(&role).into_iter().flatten().copied() {
                    if visited.insert(neighbor) {
                        component.insert(neighbor);
                        frontier.push(neighbor);
                    }
                }
            }
            let Some(center) = component.first().copied() else {
                continue;
            };
            for role in component.into_iter().skip(1) {
                output.insert(CanonicalStructuralRelationV3 {
                    left_role_id: center,
                    right_role_id: role,
                    relation: kind,
                });
            }
        }
    }
    if has_temporal_adjacency {
        let mut temporal_groups = BTreeMap::<u16, BTreeSet<u16>>::new();
        for role in roles {
            temporal_groups
                .entry(role.features.temporal_distance)
                .or_default()
                .insert(role.role_id);
        }
        let groups = temporal_groups.values().collect::<Vec<_>>();
        for pair in groups.windows(2) {
            if let (Some(left), Some(right)) = (pair[0].first().copied(), pair[1].first().copied())
            {
                output.insert(CanonicalStructuralRelationV3 {
                    left_role_id: left.min(right),
                    right_role_id: left.max(right),
                    relation: StructuralRelationKindV3::AdjacentTemporalDistance,
                });
            }
        }
    }
    output.into_iter().collect()
}

pub fn build_canonical_runtime_request_view_v3(
    projection: RuntimeProjectionV3,
    mut request_relation_atoms: Vec<u64>,
    structural: CanonicalRuntimeStructuralViewV3,
    mut capabilities: Vec<RuntimeCapabilityDescriptorV3>,
) -> Result<CanonicalRuntimeRequestViewV3, StructuralExtractionErrorV3> {
    if structural.structural_view_sha256 != structural_view_digest(&structural)? {
        return Err(StructuralExtractionErrorV3::InvalidSource);
    }
    request_relation_atoms.sort_unstable();
    request_relation_atoms.dedup();
    capabilities.sort();
    if capabilities
        .iter()
        .enumerate()
        .any(|(index, capability)| capability.capability_id as usize != index)
    {
        return Err(StructuralExtractionErrorV3::InvalidSource);
    }
    let mut view = CanonicalRuntimeRequestViewV3 {
        schema: CANONICAL_RUNTIME_REQUEST_VIEW_SCHEMA_V3.to_owned(),
        request_view_sha256: String::new(),
        projection,
        request_relation_atoms,
        structural,
        capabilities,
    };
    view.request_view_sha256 = request_view_digest(&view)?;
    Ok(view)
}

#[allow(clippy::too_many_arguments)]
pub fn build_extraction_receipt_v3(
    request_sha256: String,
    request_view_sha256: Option<String>,
    projection: RuntimeProjectionV3,
    verdict: RuntimeContextExtractionVerdictV3,
    json_nodes_visited: usize,
    text_bytes_visited: usize,
    role_candidates: usize,
    relations: usize,
    advertised_capabilities: usize,
) -> Result<ExtractionReceiptV3, StructuralExtractionErrorV3> {
    if !is_sha256(&request_sha256)
        || request_view_sha256
            .as_deref()
            .is_some_and(|root| !is_sha256(root))
    {
        return Err(StructuralExtractionErrorV3::InvalidDigest);
    }
    let mut receipt = ExtractionReceiptV3 {
        schema: RUNTIME_CONTEXT_EXTRACTION_RECEIPT_SCHEMA_V3.to_owned(),
        receipt_sha256: String::new(),
        request_sha256,
        request_view_sha256,
        projection,
        verdict,
        json_nodes_visited,
        text_bytes_visited,
        role_candidates,
        relations,
        advertised_capabilities,
        extraction_count: 1,
        teacher_or_action_fields_consumed: 0,
        raw_payloads_persisted: 0,
        execution_authority: false,
    };
    receipt.receipt_sha256 = extraction_receipt_digest(&receipt)?;
    Ok(receipt)
}

pub fn validate_extraction_receipt_v3(
    receipt: &ExtractionReceiptV3,
) -> Result<(), StructuralExtractionErrorV3> {
    if receipt.schema != RUNTIME_CONTEXT_EXTRACTION_RECEIPT_SCHEMA_V3
        || !is_sha256(&receipt.request_sha256)
        || receipt
            .request_view_sha256
            .as_deref()
            .is_some_and(|root| !is_sha256(root))
        || receipt.extraction_count != 1
        || receipt.teacher_or_action_fields_consumed != 0
        || receipt.raw_payloads_persisted != 0
        || receipt.execution_authority
        || receipt.receipt_sha256 != extraction_receipt_digest(receipt)?
    {
        return Err(StructuralExtractionErrorV3::InvalidSource);
    }
    Ok(())
}

fn visit_value(
    value: &Value,
    event: Option<EventContext>,
    depth: usize,
    state: &mut ExtractionState,
) -> NodeSummary {
    // Candidate evidence, event shape, and event class share this node budget.
    // A separate recursive shape pass would make the runtime bound dishonest.
    if state.stopped || state.json_nodes_visited >= state.budget.max_json_nodes {
        state.stopped = true;
        return NodeSummary::stopped();
    }
    state.json_nodes_visited += 1;
    match value {
        Value::Object(values) => {
            let mut children = Vec::new();
            let mut stats = ShapeStats {
                structured: 1,
                ..ShapeStats::default()
            };
            for (key, child) in values {
                if depth == 0 && state.scope.excludes_root_key(key) {
                    continue;
                }
                let child = visit_value(child, event.clone(), depth.saturating_add(1), state);
                stats.merge(&child.stats);
                children.push(child);
                if state.stopped {
                    break;
                }
            }
            let leading_singleton_collapsed_stats = if children.len() == 1 {
                children[0].leading_singleton_collapsed_stats.clone()
            } else {
                stats.clone()
            };
            let mut shapes = children
                .into_iter()
                .map(|child| child.shape)
                .collect::<Vec<_>>();
            shapes.sort();
            shapes.dedup();
            let shape = if shapes.len() == 1 {
                shapes.pop().unwrap_or_default()
            } else {
                format!("object{{{}}}", shapes.join(","))
            };
            NodeSummary {
                shape,
                stats,
                leading_singleton_collapsed_stats,
            }
        }
        Value::Array(values) if event.is_none() => {
            let count = values.len();
            let start = count.saturating_sub(state.budget.max_recent_events);
            let mut children = Vec::new();
            let mut stats = ShapeStats {
                structured: 1,
                ..ShapeStats::default()
            };
            for (index, child) in values.iter().enumerate().skip(start) {
                let event_key = state.next_event_key;
                state.next_event_key = state.next_event_key.saturating_add(1);
                state.events.entry(event_key).or_default();
                let event = EventContext {
                    event_key,
                    temporal_distance: u16::try_from(count.saturating_sub(index + 1))
                        .unwrap_or(u16::MAX),
                };
                let child = visit_value(child, Some(event), depth.saturating_add(1), state);
                let evidence = state.events.entry(event_key).or_default();
                evidence.event_class = Some(child.leading_singleton_collapsed_stats.event_class());
                evidence.topology_neighborhood_root_sha256 =
                    Some(sha256_bytes(child.shape.as_bytes()));
                stats.merge(&child.stats);
                children.push(child.shape);
                if state.stopped {
                    break;
                }
            }
            children.sort();
            NodeSummary {
                shape: format!("array[{}]", children.join(",")),
                leading_singleton_collapsed_stats: stats.clone(),
                stats,
            }
        }
        Value::Array(values) => {
            let mut children = Vec::new();
            let mut stats = ShapeStats {
                structured: 1,
                ..ShapeStats::default()
            };
            for child in values {
                let child = visit_value(child, event.clone(), depth.saturating_add(1), state);
                stats.merge(&child.stats);
                children.push(child.shape);
                if state.stopped {
                    break;
                }
            }
            children.sort();
            NodeSummary {
                shape: format!("array[{}]", children.join(",")),
                leading_singleton_collapsed_stats: stats.clone(),
                stats,
            }
        }
        Value::String(text) => {
            let remaining = state
                .budget
                .max_text_bytes
                .saturating_sub(state.text_bytes_visited);
            if remaining == 0 {
                state.stopped = true;
                return NodeSummary::stopped();
            }
            let bounded = if text.len() > remaining {
                state.stopped = true;
                &text[..nearest_char_boundary(text, remaining)]
            } else {
                text.as_str()
            };
            state.text_bytes_visited = state.text_bytes_visited.saturating_add(bounded.len());
            record_anchor(bounded, event.as_ref(), state);
            for (token, value_type) in tokenize_candidate_text(bounded) {
                add_raw_candidate(&token, value_type, event.as_ref(), state);
                if value_type == BindingValueTypeV1::Integer {
                    add_raw_candidate(&token, BindingValueTypeV1::String, event.as_ref(), state);
                }
            }
            for digits in bounded
                .split(|character: char| !character.is_ascii_digit())
                .filter(|digits| !digits.is_empty())
            {
                add_raw_candidate(digits, BindingValueTypeV1::Integer, event.as_ref(), state);
                add_raw_candidate(digits, BindingValueTypeV1::String, event.as_ref(), state);
            }
            let embedded = serde_json::from_str::<Value>(bounded).ok();
            if let Some(embedded) = embedded.as_ref() {
                visit_value(embedded, event, depth.saturating_add(1), state);
            }
            let stats = ShapeStats {
                strings: 1,
                scalars: 1,
                ..ShapeStats::default()
            };
            let shape = if embedded.is_some() {
                "string:embedded_json"
            } else if bounded.contains('\n') {
                "string:multiline"
            } else if bounded.chars().any(char::is_whitespace) {
                "string:text"
            } else {
                "string:scalar"
            };
            NodeSummary {
                shape: shape.to_owned(),
                leading_singleton_collapsed_stats: stats.clone(),
                stats,
            }
        }
        Value::Number(number) => {
            let token = number.to_string();
            record_anchor(&token, event.as_ref(), state);
            if number.is_i64() || number.is_u64() {
                add_raw_candidate(&token, BindingValueTypeV1::Integer, event.as_ref(), state);
            }
            let stats = ShapeStats {
                scalars: 1,
                ..ShapeStats::default()
            };
            NodeSummary {
                shape: if number.is_i64() || number.is_u64() {
                    "integer"
                } else {
                    "number"
                }
                .to_owned(),
                leading_singleton_collapsed_stats: stats.clone(),
                stats,
            }
        }
        Value::Bool(value) => {
            let token = value.to_string();
            record_anchor(&token, event.as_ref(), state);
            add_raw_candidate(&token, BindingValueTypeV1::Boolean, event.as_ref(), state);
            let stats = ShapeStats {
                scalars: 1,
                ..ShapeStats::default()
            };
            NodeSummary {
                shape: "bool".to_owned(),
                leading_singleton_collapsed_stats: stats.clone(),
                stats,
            }
        }
        Value::Null => NodeSummary {
            shape: "null".to_owned(),
            stats: ShapeStats::default(),
            leading_singleton_collapsed_stats: ShapeStats::default(),
        },
    }
}

fn add_raw_candidate(
    token: &str,
    value_type: BindingValueTypeV1,
    event: Option<&EventContext>,
    state: &mut ExtractionState,
) {
    if state.raw_candidates.len() >= state.budget.max_role_candidates.saturating_mul(8) {
        state.stopped = true;
        return;
    }
    let value_sha256 = match value_type {
        BindingValueTypeV1::Integer => token
            .parse::<u64>()
            .ok()
            .and_then(|value| canonical_json_sha256(&value).ok()),
        BindingValueTypeV1::Boolean => token
            .parse::<bool>()
            .ok()
            .and_then(|value| canonical_json_sha256(&value).ok()),
        BindingValueTypeV1::String | BindingValueTypeV1::Identifier => {
            canonical_json_sha256(&token).ok()
        }
    };
    let Some(value_sha256) = value_sha256 else {
        return;
    };
    let event_key = event.map_or(0, |event| event.event_key);
    state
        .event_candidate_values
        .entry(event_key)
        .or_default()
        .insert(value_sha256.clone());
    state
        .candidate_events
        .entry(value_sha256.clone())
        .or_default()
        .insert(event_key);
    state.raw_candidates.push(RawCandidate {
        normalized: token.to_owned(),
        value_sha256,
        value_type,
        event_key,
        temporal_distance: event.map_or(u16::MAX, |event| event.temporal_distance),
    });
}

fn record_anchor(text: &str, event: Option<&EventContext>, state: &mut ExtractionState) {
    let Some(event) = event else {
        return;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.len() > 128 || trimmed.chars().any(char::is_whitespace) {
        return;
    }
    state
        .events
        .entry(event.event_key)
        .or_default()
        .anchors
        .insert(sha256_bytes(trimmed.as_bytes()));
}

fn materialize_candidates(
    context: &StructuralContextV3,
    state: &ExtractionState,
) -> Vec<StructuralCandidateObservationV3> {
    let mut anchor_events = BTreeMap::<String, BTreeSet<u32>>::new();
    for (event_key, event) in &state.events {
        for anchor in &event.anchors {
            anchor_events
                .entry(anchor.clone())
                .or_default()
                .insert(*event_key);
        }
    }
    let mut grouped =
        BTreeMap::<(String, StructuralCandidateFeaturesV3), (usize, BTreeSet<String>)>::new();
    for raw in &state.raw_candidates {
        let same_value_events = state
            .candidate_events
            .get(&raw.value_sha256)
            .map_or(0, BTreeSet::len);
        let shared_anchor = state.events.get(&raw.event_key).is_some_and(|event| {
            event.anchors.iter().any(|anchor| {
                anchor_events
                    .get(anchor)
                    .is_some_and(|events| events.len() > 1)
            })
        });
        let call_lineage = if raw.event_key == 0 {
            BindingCallLineageV1::Unknown
        } else if same_value_events > 1 {
            BindingCallLineageV1::SameValueAcrossEvents
        } else if shared_anchor {
            BindingCallLineageV1::SharedOpaqueAnchor
        } else {
            BindingCallLineageV1::Unlinked
        };
        let request_relation = if !state.request_present {
            BindingRequestRelationV1::RequestAbsent
        } else if state.request_tokens.contains(&raw.normalized) {
            BindingRequestRelationV1::Mentioned
        } else {
            BindingRequestRelationV1::NotMentioned
        };
        let event_evidence = state.events.get(&raw.event_key);
        let features = StructuralCandidateFeaturesV3 {
            source_event_class: event_evidence
                .and_then(|event| event.event_class)
                .unwrap_or(BindingSourceEventClassV1::Unknown),
            call_lineage,
            capability_class: context.capability_class(),
            temporal_distance: raw.temporal_distance,
            completion_state: context.completion_state,
            event_candidate_cardinality: u16::try_from(
                state
                    .event_candidate_values
                    .get(&raw.event_key)
                    .map_or(0, BTreeSet::len),
            )
            .unwrap_or(u16::MAX),
            value_type: raw.value_type,
            request_relation,
            topology_neighborhood_root_sha256: event_evidence
                .and_then(|event| event.topology_neighborhood_root_sha256.clone())
                .unwrap_or_else(|| sha256_bytes(b"root-scalar")),
        };
        let entry = grouped
            .entry((raw.value_sha256.clone(), features))
            .or_default();
        entry.0 = entry.0.saturating_add(1);
        entry.1.insert(raw.normalized.clone());
    }
    grouped
        .into_iter()
        .enumerate()
        .map(
            |(index, ((value_sha256, features), (occurrences, normalized_values)))| {
                StructuralCandidateObservationV3 {
                    source_role_id: u16::try_from(index).unwrap_or(u16::MAX),
                    value_sha256,
                    normalized_values: normalized_values.into_iter().collect(),
                    features,
                    occurrence_count: u16::try_from(occurrences).unwrap_or(u16::MAX),
                }
            },
        )
        .collect()
}

fn source_relations(
    candidates: &[StructuralCandidateObservationV3],
    max_relations: usize,
) -> (Vec<StructuralSourceRelationV3>, bool) {
    let mut relations = BTreeSet::new();
    add_group_relations(
        candidates,
        |candidate| candidate.value_sha256.clone(),
        StructuralRelationKindV3::SameValue,
        &mut relations,
    );
    add_group_relations(
        candidates,
        |candidate| format!("{:?}", candidate.features.call_lineage),
        StructuralRelationKindV3::SameCallLineage,
        &mut relations,
    );
    add_group_relations(
        candidates,
        |candidate| candidate.features.topology_neighborhood_root_sha256.clone(),
        StructuralRelationKindV3::SameTopologyNeighborhood,
        &mut relations,
    );
    let mut temporal = candidates.iter().collect::<Vec<_>>();
    temporal.sort_by(|left, right| {
        left.features
            .temporal_distance
            .cmp(&right.features.temporal_distance)
            .then_with(|| left.source_role_id.cmp(&right.source_role_id))
    });
    for pair in temporal.windows(2) {
        if pair[0].features.temporal_distance != pair[1].features.temporal_distance {
            relations.insert(source_edge(
                pair[0],
                pair[1],
                StructuralRelationKindV3::AdjacentTemporalDistance,
            ));
        }
    }
    let exhausted = relations.len() > max_relations;
    let mut relations = relations.into_iter().collect::<Vec<_>>();
    relations.truncate(max_relations);
    (relations, exhausted)
}

fn add_group_relations<F>(
    candidates: &[StructuralCandidateObservationV3],
    key: F,
    relation: StructuralRelationKindV3,
    output: &mut BTreeSet<StructuralSourceRelationV3>,
) where
    F: Fn(&StructuralCandidateObservationV3) -> String,
{
    let mut groups = BTreeMap::<String, Vec<&StructuralCandidateObservationV3>>::new();
    for candidate in candidates {
        groups.entry(key(candidate)).or_default().push(candidate);
    }
    for candidates in groups.values_mut() {
        candidates.sort_by_key(|candidate| candidate.source_role_id);
        if let Some(first) = candidates.first().copied() {
            for candidate in candidates.iter().copied().skip(1) {
                output.insert(source_edge(first, candidate, relation));
            }
        }
    }
}

fn source_edge(
    left: &StructuralCandidateObservationV3,
    right: &StructuralCandidateObservationV3,
    relation: StructuralRelationKindV3,
) -> StructuralSourceRelationV3 {
    StructuralSourceRelationV3 {
        left_source_role_id: left.source_role_id.min(right.source_role_id),
        right_source_role_id: left.source_role_id.max(right.source_role_id),
        relation,
    }
}

fn tokenize_candidate_text(text: &str) -> Vec<(String, BindingValueTypeV1)> {
    let has_structure = text.chars().any(char::is_whitespace)
        || text.contains('{')
        || text.contains('[')
        || text.contains(':');
    let mut output = BTreeSet::new();
    for token in text
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '/'))
        })
        .map(|token| token.trim_matches(|character: char| matches!(character, ':' | '/' | '.')))
        .filter(|token| !token.is_empty())
    {
        if token.len() > 256 || token.len() < 2 {
            continue;
        }
        if token.bytes().all(|byte| byte.is_ascii_digit()) {
            if token.parse::<u64>().is_ok() {
                output.insert((token.to_owned(), BindingValueTypeV1::Integer));
            }
            continue;
        }
        let contains_digit = token.bytes().any(|byte| byte.is_ascii_digit());
        let contains_upper = token.bytes().any(|byte| byte.is_ascii_uppercase());
        let contains_lower = token.bytes().any(|byte| byte.is_ascii_lowercase());
        let opaque = token.len() >= 4
            && (contains_digit || (contains_upper && contains_lower))
            && token.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
            });
        if opaque || (has_structure && token.len() >= 12 && token.contains(['-', '_'])) {
            output.insert((token.to_owned(), BindingValueTypeV1::Identifier));
            output.insert((token.to_owned(), BindingValueTypeV1::String));
        }
    }
    output.into_iter().collect()
}

fn structural_view_digest(
    view: &CanonicalRuntimeStructuralViewV3,
) -> Result<String, StructuralExtractionErrorV3> {
    let mut material = view.clone();
    material.structural_view_sha256.clear();
    canonical_json_sha256(&material).map_err(|_| StructuralExtractionErrorV3::Serialization)
}

fn request_view_digest(
    view: &CanonicalRuntimeRequestViewV3,
) -> Result<String, StructuralExtractionErrorV3> {
    let mut material = view.clone();
    material.request_view_sha256.clear();
    canonical_json_sha256(&material).map_err(|_| StructuralExtractionErrorV3::Serialization)
}

fn extraction_receipt_digest(
    receipt: &ExtractionReceiptV3,
) -> Result<String, StructuralExtractionErrorV3> {
    let mut material = receipt.clone();
    material.receipt_sha256.clear();
    canonical_json_sha256(&material).map_err(|_| StructuralExtractionErrorV3::Serialization)
}

fn nearest_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
