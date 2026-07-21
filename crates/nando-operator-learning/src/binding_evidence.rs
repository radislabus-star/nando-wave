//! Label-blind pre-action binding evidence and bounded version-space search.
//!
//! This module is a proof-path owner. It does not execute a selector, compile a
//! protocol mode, or grant runtime authority. Expected bindings can only be
//! attached to an already frozen candidate graph.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub use nando_operator_kernel::binding::{
    BindingCallLineageV1, BindingCapabilityClassV1, BindingCompletionStateV1, BindingPredicateV1,
    BindingRequestRelationV1, BindingSourceEventClassV1, BindingValueTypeV1,
};

pub const PRE_ACTION_BINDING_SURFACE_SCHEMA_V1: &str = "nando.pre-action-binding-surface.v1";
pub const CANDIDATE_RELATION_GRAPH_SCHEMA_V1: &str = "nando.candidate-relation-graph.v1";
pub const FROZEN_CANDIDATE_RELATION_GRAPH_SCHEMA_V1: &str =
    "nando.frozen-candidate-relation-graph.v1";
pub const EXPECTED_BINDING_RECEIPT_SCHEMA_V1: &str = "nando.expected-binding-receipt.v1";
pub const BINDING_VERSION_SPACE_REPORT_SCHEMA_V1: &str = "nando.binding-version-space-report.v1.r1";

pub const MAX_BINDING_JSON_NODES_V1: usize = 16_384;
pub const MAX_BINDING_TEXT_BYTES_V1: usize = 256 * 1024;
pub const MAX_BINDING_RECENT_EVENTS_V1: usize = 32;
pub const MAX_BINDING_CANDIDATES_PER_ROW_V1: usize = 8_192;
pub const MAX_BINDING_RELATION_EDGES_PER_ROW_V1: usize = 65_536;
pub const MAX_BINDING_HYPOTHESES_V1: usize = 16_384;
pub const MAX_BINDING_PREDICATES_PER_HYPOTHESIS_V1: usize = 3;
pub const MAX_BINDING_REPORT_HYPOTHESES_V1: usize = 16_384;
pub const MAX_BINDING_REPORT_TIES_V1: usize = 16_384;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingEvidenceBudgetV1 {
    pub max_json_nodes: usize,
    pub max_text_bytes: usize,
    pub max_recent_events: usize,
    pub max_candidates_per_row: usize,
    pub max_relation_edges_per_row: usize,
    pub max_hypotheses: usize,
    pub max_predicates_per_hypothesis: usize,
    pub max_report_hypotheses: usize,
    pub max_report_ties: usize,
}

impl Default for BindingEvidenceBudgetV1 {
    fn default() -> Self {
        Self {
            max_json_nodes: MAX_BINDING_JSON_NODES_V1,
            max_text_bytes: MAX_BINDING_TEXT_BYTES_V1,
            max_recent_events: MAX_BINDING_RECENT_EVENTS_V1,
            max_candidates_per_row: MAX_BINDING_CANDIDATES_PER_ROW_V1,
            max_relation_edges_per_row: MAX_BINDING_RELATION_EDGES_PER_ROW_V1,
            max_hypotheses: MAX_BINDING_HYPOTHESES_V1,
            max_predicates_per_hypothesis: MAX_BINDING_PREDICATES_PER_HYPOTHESIS_V1,
            max_report_hypotheses: MAX_BINDING_REPORT_HYPOTHESES_V1,
            max_report_ties: MAX_BINDING_REPORT_TIES_V1,
        }
    }
}

impl BindingEvidenceBudgetV1 {
    fn validate(self) -> Result<Self, BindingEvidenceErrorV1> {
        if self.max_json_nodes == 0
            || self.max_json_nodes > MAX_BINDING_JSON_NODES_V1
            || self.max_text_bytes == 0
            || self.max_text_bytes > MAX_BINDING_TEXT_BYTES_V1
            || self.max_recent_events == 0
            || self.max_recent_events > MAX_BINDING_RECENT_EVENTS_V1
            || self.max_candidates_per_row == 0
            || self.max_candidates_per_row > MAX_BINDING_CANDIDATES_PER_ROW_V1
            || self.max_relation_edges_per_row == 0
            || self.max_relation_edges_per_row > MAX_BINDING_RELATION_EDGES_PER_ROW_V1
            || self.max_hypotheses == 0
            || self.max_hypotheses > MAX_BINDING_HYPOTHESES_V1
            || self.max_predicates_per_hypothesis == 0
            || self.max_predicates_per_hypothesis > MAX_BINDING_PREDICATES_PER_HYPOTHESIS_V1
            || self.max_report_hypotheses == 0
            || self.max_report_hypotheses > MAX_BINDING_REPORT_HYPOTHESES_V1
            || self.max_report_ties == 0
            || self.max_report_ties > MAX_BINDING_REPORT_TIES_V1
        {
            return Err(BindingEvidenceErrorV1::InvalidBudget);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreActionBindingContextV1 {
    pub call_shape_count: u16,
    pub capability_count: u16,
    pub completion_state: BindingCompletionStateV1,
    pub temporal_relation_count: u16,
    pub cardinality_relation_count: u16,
    pub topology_neighborhood_root_sha256: String,
}

impl PreActionBindingContextV1 {
    fn capability_class(&self) -> BindingCapabilityClassV1 {
        match self.capability_count.max(self.call_shape_count) {
            0 => BindingCapabilityClassV1::None,
            1 => BindingCapabilityClassV1::Single,
            _ => BindingCapabilityClassV1::Multiple,
        }
    }

    fn validate(&self) -> Result<(), BindingEvidenceErrorV1> {
        if !is_sha256(&self.topology_neighborhood_root_sha256) {
            return Err(BindingEvidenceErrorV1::InvalidContext);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BindingCandidateFeaturesV1 {
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingCandidateNodeV1 {
    pub candidate_id_sha256: String,
    /// Opaque value commitment used only to form action-equivalence classes
    /// after graph freeze. It is never a hypothesis predicate.
    pub action_equivalence_sha256: String,
    pub features: BindingCandidateFeaturesV1,
    pub occurrence_count: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreActionBindingSurfaceV1 {
    pub schema: String,
    pub row_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub context: PreActionBindingContextV1,
    pub candidates: Vec<BindingCandidateNodeV1>,
    pub json_nodes_visited: usize,
    pub text_bytes_visited: usize,
    pub candidates_before_budget: usize,
    pub candidate_budget_exhausted: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingCandidateRelationKindV1 {
    SameActionEquivalence,
    SameCallLineage,
    SameTopologyNeighborhood,
    AdjacentTemporalDistance,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BindingCandidateRelationEdgeV1 {
    pub left_candidate_id_sha256: String,
    pub right_candidate_id_sha256: String,
    pub relation: BindingCandidateRelationKindV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidateRelationGraphV1 {
    pub schema: String,
    pub row_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub context: PreActionBindingContextV1,
    pub nodes: Vec<BindingCandidateNodeV1>,
    pub edges: Vec<BindingCandidateRelationEdgeV1>,
    pub extraction_budget_exhausted: bool,
    pub relation_budget_exhausted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrozenCandidateRelationGraphV1 {
    pub schema: String,
    pub graph_root_sha256: String,
    pub graph: CandidateRelationGraphV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingBaselineOutcomeV1 {
    Exact,
    Wrong,
    Abstain,
    VerifyFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingEvaluationLabelV1 {
    Positive,
    ApplicabilityNegative,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedBindingReceiptV1 {
    pub schema: String,
    pub receipt_sha256: String,
    pub row_id_sha256: String,
    pub frozen_graph_root_sha256: String,
    pub label: BindingEvaluationLabelV1,
    pub expected_action_equivalence_sha256: Option<String>,
    pub baseline_outcome: BindingBaselineOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingHypothesisScoreV1 {
    pub hypothesis_id_sha256: String,
    pub predicates: Vec<BindingPredicateV1>,
    pub positive_rows_covered: usize,
    pub positive_rows_uncovered: usize,
    pub ambiguous_rows: usize,
    pub wrong_bindings: usize,
    pub negative_accepts: usize,
    pub selected_action_class_root_sha256: Option<String>,
    pub complete: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingRowAccountingV1 {
    pub row_id_sha256: String,
    pub graph_root_sha256: String,
    pub label: BindingEvaluationLabelV1,
    pub baseline_outcome: BindingBaselineOutcomeV1,
    pub candidate_count: usize,
    pub expected_candidate_count: usize,
    pub expected_observable: bool,
    pub extraction_budget_exhausted: bool,
    pub relation_budget_exhausted: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingTieV1 {
    pub row_id_sha256: String,
    pub expected_action_equivalence_sha256: String,
    pub competing_action_equivalence_sha256: Vec<String>,
    pub shared_feature_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingDistinguishingProbeV1 {
    pub row_id_sha256: String,
    pub tie_root_sha256: String,
    pub required_distinction: String,
    pub probe: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BindingVersionSpaceVerdictV1 {
    #[serde(rename = "BINDING_IDENTIFIABLE_CANDIDATE")]
    BindingIdentifiableCandidate,
    #[serde(rename = "INSUFFICIENT_BINDING_EVIDENCE")]
    InsufficientBindingEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BindingVersionSpaceReportV1 {
    pub schema: String,
    pub report_sha256: String,
    pub verdict: BindingVersionSpaceVerdictV1,
    pub frozen_denominator: usize,
    pub positive_rows: usize,
    pub applicability_negative_rows: usize,
    pub censored_unknown_rows: usize,
    pub censored_unknown_root_sha256: String,
    pub exceptional_rows: usize,
    pub exceptional_rows_accounted: usize,
    pub candidates_total: usize,
    pub candidates_max_per_row: usize,
    pub candidate_budget_exhausted_rows: usize,
    pub relation_budget_exhausted_rows: usize,
    pub hypotheses_evaluated: usize,
    pub hypothesis_budget_exhausted: bool,
    pub complete_hypotheses: usize,
    pub complete_action_equivalence_classes: usize,
    pub wrong_bindings: usize,
    pub negative_accepts: usize,
    pub identifiable_candidate: Option<BindingHypothesisScoreV1>,
    pub competing_hypotheses: Vec<BindingHypothesisScoreV1>,
    pub ties_total: usize,
    pub tie_budget_exhausted: bool,
    pub ties: Vec<BindingTieV1>,
    pub distinguishing_probes: Vec<BindingDistinguishingProbeV1>,
    pub row_accounting: Vec<BindingRowAccountingV1>,
    pub budget: BindingEvidenceBudgetV1,
    pub execution_authority: bool,
    pub protocol_mode_compiled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingEvidenceErrorV1 {
    InvalidBudget,
    InvalidContext,
    InvalidDigest,
    DuplicateRow,
    MissingReceipt,
    ForeignGraphReceipt,
    InvalidReceipt,
    InvalidCorpus,
    Serialization,
}

#[derive(Clone)]
struct RawCandidate {
    normalized: String,
    action_equivalence_sha256: String,
    value_type: BindingValueTypeV1,
    event_key: u32,
    temporal_distance: u16,
    event_class: BindingSourceEventClassV1,
    topology_neighborhood_root_sha256: String,
}

#[derive(Default)]
struct EventEvidence {
    anchors: BTreeSet<String>,
}

struct ExtractionState<'a> {
    budget: BindingEvidenceBudgetV1,
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
    _payload: &'a Value,
}

#[derive(Clone)]
struct EventContext {
    event_key: u32,
    temporal_distance: u16,
    event_class: BindingSourceEventClassV1,
    topology_neighborhood_root_sha256: String,
}

#[derive(Default)]
struct ShapeStats {
    strings: usize,
    structured: usize,
    scalars: usize,
}

impl PreActionBindingSurfaceV1 {
    pub fn capture(
        row_id_sha256: impl Into<String>,
        evidence_ref_sha256: impl Into<String>,
        request_text: &str,
        provider_payload: &Value,
        context: PreActionBindingContextV1,
        budget: BindingEvidenceBudgetV1,
    ) -> Result<Self, BindingEvidenceErrorV1> {
        let budget = budget.validate()?;
        context.validate()?;
        let row_id_sha256 = row_id_sha256.into();
        let evidence_ref_sha256 = evidence_ref_sha256.into();
        if !is_sha256(&row_id_sha256) || !is_sha256(&evidence_ref_sha256) {
            return Err(BindingEvidenceErrorV1::InvalidDigest);
        }
        let request_tokens = tokenize_candidate_text(request_text)
            .into_iter()
            .map(|(token, _)| token)
            .collect();
        let mut state = ExtractionState {
            budget,
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
            _payload: provider_payload,
        };
        visit_value(provider_payload, None, &mut state);

        let candidates_before_budget = state.raw_candidates.len();
        let mut nodes = materialize_candidates(&row_id_sha256, &context, &state);
        let candidate_budget_exhausted =
            state.stopped || nodes.len() > budget.max_candidates_per_row;
        nodes.sort_by(|left, right| left.candidate_id_sha256.cmp(&right.candidate_id_sha256));
        nodes.truncate(budget.max_candidates_per_row);
        Ok(Self {
            schema: PRE_ACTION_BINDING_SURFACE_SCHEMA_V1.to_owned(),
            row_id_sha256,
            evidence_ref_sha256,
            context,
            candidates: nodes,
            json_nodes_visited: state.json_nodes_visited,
            text_bytes_visited: state.text_bytes_visited,
            candidates_before_budget,
            candidate_budget_exhausted,
        })
    }

    pub fn candidate_relation_graph(
        self,
        budget: BindingEvidenceBudgetV1,
    ) -> Result<CandidateRelationGraphV1, BindingEvidenceErrorV1> {
        let budget = budget.validate()?;
        let mut edges = BTreeSet::new();
        add_group_edges(
            &self.candidates,
            |node| node.action_equivalence_sha256.clone(),
            BindingCandidateRelationKindV1::SameActionEquivalence,
            &mut edges,
        );
        add_group_edges(
            &self.candidates,
            |node| format!("{:?}", node.features.call_lineage),
            BindingCandidateRelationKindV1::SameCallLineage,
            &mut edges,
        );
        add_group_edges(
            &self.candidates,
            |node| node.features.topology_neighborhood_root_sha256.clone(),
            BindingCandidateRelationKindV1::SameTopologyNeighborhood,
            &mut edges,
        );
        let mut temporal = self.candidates.iter().collect::<Vec<_>>();
        temporal.sort_by(|left, right| {
            left.features
                .temporal_distance
                .cmp(&right.features.temporal_distance)
                .then_with(|| left.candidate_id_sha256.cmp(&right.candidate_id_sha256))
        });
        for pair in temporal.windows(2) {
            if pair[0].features.temporal_distance != pair[1].features.temporal_distance {
                edges.insert(edge(
                    pair[0],
                    pair[1],
                    BindingCandidateRelationKindV1::AdjacentTemporalDistance,
                ));
            }
        }
        let relation_budget_exhausted = edges.len() > budget.max_relation_edges_per_row;
        let mut edges = edges.into_iter().collect::<Vec<_>>();
        edges.truncate(budget.max_relation_edges_per_row);
        Ok(CandidateRelationGraphV1 {
            schema: CANDIDATE_RELATION_GRAPH_SCHEMA_V1.to_owned(),
            row_id_sha256: self.row_id_sha256,
            evidence_ref_sha256: self.evidence_ref_sha256,
            context: self.context,
            nodes: self.candidates,
            edges,
            extraction_budget_exhausted: self.candidate_budget_exhausted,
            relation_budget_exhausted,
        })
    }
}

impl CandidateRelationGraphV1 {
    pub fn freeze(self) -> Result<FrozenCandidateRelationGraphV1, BindingEvidenceErrorV1> {
        if self.schema != CANDIDATE_RELATION_GRAPH_SCHEMA_V1
            || !is_sha256(&self.row_id_sha256)
            || !is_sha256(&self.evidence_ref_sha256)
        {
            return Err(BindingEvidenceErrorV1::InvalidCorpus);
        }
        let graph_root_sha256 = sha256_json(&self)?;
        Ok(FrozenCandidateRelationGraphV1 {
            schema: FROZEN_CANDIDATE_RELATION_GRAPH_SCHEMA_V1.to_owned(),
            graph_root_sha256,
            graph: self,
        })
    }
}

impl ExpectedBindingReceiptV1 {
    /// Diagnostic-only label used to evaluate the frozen B1A graph. B1B
    /// scored evidence must use the externally pinned trusted-label envelope.
    pub fn positive(
        graph: &FrozenCandidateRelationGraphV1,
        expected_action_equivalence_sha256: impl Into<String>,
        baseline_outcome: BindingBaselineOutcomeV1,
    ) -> Result<Self, BindingEvidenceErrorV1> {
        let expected_action_equivalence_sha256 = expected_action_equivalence_sha256.into();
        if !is_sha256(&expected_action_equivalence_sha256) {
            return Err(BindingEvidenceErrorV1::InvalidDigest);
        }
        Self::new(
            graph,
            BindingEvaluationLabelV1::Positive,
            Some(expected_action_equivalence_sha256),
            baseline_outcome,
        )
    }

    pub fn applicability_negative(
        graph: &FrozenCandidateRelationGraphV1,
    ) -> Result<Self, BindingEvidenceErrorV1> {
        Self::new(
            graph,
            BindingEvaluationLabelV1::ApplicabilityNegative,
            None,
            BindingBaselineOutcomeV1::Exact,
        )
    }

    fn new(
        graph: &FrozenCandidateRelationGraphV1,
        label: BindingEvaluationLabelV1,
        expected_action_equivalence_sha256: Option<String>,
        baseline_outcome: BindingBaselineOutcomeV1,
    ) -> Result<Self, BindingEvidenceErrorV1> {
        let mut receipt = Self {
            schema: EXPECTED_BINDING_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_sha256: String::new(),
            row_id_sha256: graph.graph.row_id_sha256.clone(),
            frozen_graph_root_sha256: graph.graph_root_sha256.clone(),
            label,
            expected_action_equivalence_sha256,
            baseline_outcome,
        };
        receipt.receipt_sha256 = expected_receipt_digest(&receipt)?;
        Ok(receipt)
    }

    fn validate(
        &self,
        graph: &FrozenCandidateRelationGraphV1,
    ) -> Result<(), BindingEvidenceErrorV1> {
        if self.schema != EXPECTED_BINDING_RECEIPT_SCHEMA_V1
            || self.row_id_sha256 != graph.graph.row_id_sha256
            || self.frozen_graph_root_sha256 != graph.graph_root_sha256
        {
            return Err(BindingEvidenceErrorV1::ForeignGraphReceipt);
        }
        if self.label == BindingEvaluationLabelV1::Positive
            && self
                .expected_action_equivalence_sha256
                .as_deref()
                .is_none_or(|value| !is_sha256(value))
        {
            return Err(BindingEvidenceErrorV1::InvalidReceipt);
        }
        if self.label == BindingEvaluationLabelV1::ApplicabilityNegative
            && self.expected_action_equivalence_sha256.is_some()
        {
            return Err(BindingEvidenceErrorV1::InvalidReceipt);
        }
        if expected_receipt_digest(self)? != self.receipt_sha256 {
            return Err(BindingEvidenceErrorV1::InvalidReceipt);
        }
        Ok(())
    }
}

pub fn evaluate_binding_version_space_v1(
    mut graphs: Vec<FrozenCandidateRelationGraphV1>,
    mut receipts: Vec<ExpectedBindingReceiptV1>,
    mut censored_unknown_row_ids: Vec<String>,
    budget: BindingEvidenceBudgetV1,
) -> Result<BindingVersionSpaceReportV1, BindingEvidenceErrorV1> {
    let budget = budget.validate()?;
    graphs.sort_by(|left, right| left.graph.row_id_sha256.cmp(&right.graph.row_id_sha256));
    receipts.sort_by(|left, right| left.row_id_sha256.cmp(&right.row_id_sha256));
    censored_unknown_row_ids.sort();
    censored_unknown_row_ids.dedup();
    if censored_unknown_row_ids
        .iter()
        .any(|value| !is_sha256(value))
    {
        return Err(BindingEvidenceErrorV1::InvalidDigest);
    }

    let mut graph_by_row = BTreeMap::new();
    for graph in &graphs {
        if graph.schema != FROZEN_CANDIDATE_RELATION_GRAPH_SCHEMA_V1
            || sha256_json(&graph.graph)? != graph.graph_root_sha256
            || graph_by_row
                .insert(graph.graph.row_id_sha256.clone(), graph)
                .is_some()
        {
            return Err(BindingEvidenceErrorV1::DuplicateRow);
        }
    }
    let mut receipt_by_row = BTreeMap::new();
    for receipt in &receipts {
        if receipt_by_row
            .insert(receipt.row_id_sha256.clone(), receipt)
            .is_some()
        {
            return Err(BindingEvidenceErrorV1::DuplicateRow);
        }
        let graph = graph_by_row
            .get(&receipt.row_id_sha256)
            .ok_or(BindingEvidenceErrorV1::ForeignGraphReceipt)?;
        receipt.validate(graph)?;
    }
    if graph_by_row.len() != receipt_by_row.len() {
        return Err(BindingEvidenceErrorV1::MissingReceipt);
    }
    if censored_unknown_row_ids
        .iter()
        .any(|row| graph_by_row.contains_key(row))
    {
        return Err(BindingEvidenceErrorV1::InvalidCorpus);
    }

    let mut hypothesis_predicates = BTreeSet::<Vec<BindingPredicateV1>>::new();
    hypothesis_predicates.insert(Vec::new());
    let mut hypothesis_budget_exhausted = false;
    for receipt in &receipts {
        if receipt.label != BindingEvaluationLabelV1::Positive {
            continue;
        }
        let expected = receipt
            .expected_action_equivalence_sha256
            .as_deref()
            .ok_or(BindingEvidenceErrorV1::InvalidReceipt)?;
        let graph = graph_by_row[&receipt.row_id_sha256];
        for node in graph
            .graph
            .nodes
            .iter()
            .filter(|node| node.action_equivalence_sha256 == expected)
        {
            let atoms = binding_feature_predicates_v1(&node.features);
            add_predicate_subsets(
                &atoms,
                budget.max_predicates_per_hypothesis,
                budget.max_hypotheses,
                &mut hypothesis_predicates,
                &mut hypothesis_budget_exhausted,
            );
            if hypothesis_budget_exhausted {
                break;
            }
        }
        if hypothesis_budget_exhausted {
            break;
        }
    }

    let mut scores = hypothesis_predicates
        .into_iter()
        .map(|predicates| score_hypothesis(&predicates, &graphs, &receipt_by_row))
        .collect::<Result<Vec<_>, _>>()?;
    scores.sort_by(hypothesis_score_order);
    let complete = scores
        .iter()
        .filter(|score| score.complete)
        .cloned()
        .collect::<Vec<_>>();
    let complete_action_classes = complete
        .iter()
        .filter_map(|score| score.selected_action_class_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let identifiable = !hypothesis_budget_exhausted
        && graphs.iter().all(|graph| {
            !graph.graph.extraction_budget_exhausted && !graph.graph.relation_budget_exhausted
        })
        && complete_action_classes.len() == 1
        && !complete.is_empty();
    let identifiable_candidate = identifiable.then(|| complete[0].clone());

    let mut row_accounting = Vec::new();
    let mut exceptional_rows = 0_usize;
    let mut exceptional_rows_accounted = 0_usize;
    for graph in &graphs {
        let receipt = receipt_by_row[&graph.graph.row_id_sha256];
        let expected_candidate_count =
            receipt
                .expected_action_equivalence_sha256
                .as_ref()
                .map_or(0, |expected| {
                    graph
                        .graph
                        .nodes
                        .iter()
                        .filter(|node| &node.action_equivalence_sha256 == expected)
                        .count()
                });
        let exceptional = receipt.baseline_outcome != BindingBaselineOutcomeV1::Exact;
        exceptional_rows += usize::from(exceptional);
        exceptional_rows_accounted += usize::from(exceptional && expected_candidate_count > 0);
        row_accounting.push(BindingRowAccountingV1 {
            row_id_sha256: graph.graph.row_id_sha256.clone(),
            graph_root_sha256: graph.graph_root_sha256.clone(),
            label: receipt.label,
            baseline_outcome: receipt.baseline_outcome,
            candidate_count: graph.graph.nodes.len(),
            expected_candidate_count,
            expected_observable: expected_candidate_count > 0,
            extraction_budget_exhausted: graph.graph.extraction_budget_exhausted,
            relation_budget_exhausted: graph.graph.relation_budget_exhausted,
        });
    }

    let (ties, ties_total) = collect_ties(&graphs, &receipt_by_row, budget.max_report_ties);
    let distinguishing_probes = ties.iter().map(distinguishing_probe).collect();
    let best = scores.first().cloned();
    let wrong_bindings = identifiable_candidate
        .as_ref()
        .or(best.as_ref())
        .map_or(0, |score| score.wrong_bindings);
    let negative_accepts = identifiable_candidate
        .as_ref()
        .or(best.as_ref())
        .map_or(0, |score| score.negative_accepts);
    let positive_rows = receipts
        .iter()
        .filter(|receipt| receipt.label == BindingEvaluationLabelV1::Positive)
        .count();
    let applicability_negative_rows = receipts.len().saturating_sub(positive_rows);
    let candidates_total = graphs.iter().map(|graph| graph.graph.nodes.len()).sum();
    let candidates_max_per_row = graphs
        .iter()
        .map(|graph| graph.graph.nodes.len())
        .max()
        .unwrap_or(0);
    let mut report = BindingVersionSpaceReportV1 {
        schema: BINDING_VERSION_SPACE_REPORT_SCHEMA_V1.to_owned(),
        report_sha256: String::new(),
        verdict: if identifiable {
            BindingVersionSpaceVerdictV1::BindingIdentifiableCandidate
        } else {
            BindingVersionSpaceVerdictV1::InsufficientBindingEvidence
        },
        frozen_denominator: graphs.len(),
        positive_rows,
        applicability_negative_rows,
        censored_unknown_rows: censored_unknown_row_ids.len(),
        censored_unknown_root_sha256: sha256_json(&censored_unknown_row_ids)?,
        exceptional_rows,
        exceptional_rows_accounted,
        candidates_total,
        candidates_max_per_row,
        candidate_budget_exhausted_rows: graphs
            .iter()
            .filter(|graph| graph.graph.extraction_budget_exhausted)
            .count(),
        relation_budget_exhausted_rows: graphs
            .iter()
            .filter(|graph| graph.graph.relation_budget_exhausted)
            .count(),
        hypotheses_evaluated: scores.len(),
        hypothesis_budget_exhausted,
        complete_hypotheses: complete.len(),
        complete_action_equivalence_classes: complete_action_classes.len(),
        wrong_bindings,
        negative_accepts,
        identifiable_candidate,
        competing_hypotheses: scores
            .into_iter()
            .take(budget.max_report_hypotheses)
            .collect(),
        ties_total,
        tie_budget_exhausted: ties_total > ties.len(),
        ties,
        distinguishing_probes,
        row_accounting,
        budget,
        execution_authority: false,
        protocol_mode_compiled: false,
    };
    report.report_sha256 = binding_report_digest(&report)?;
    Ok(report)
}

fn visit_value(value: &Value, event: Option<EventContext>, state: &mut ExtractionState<'_>) {
    if state.stopped || state.json_nodes_visited >= state.budget.max_json_nodes {
        state.stopped = true;
        return;
    }
    state.json_nodes_visited += 1;
    match value {
        Value::Object(values) => {
            for child in values.values() {
                visit_value(child, event.clone(), state);
                if state.stopped {
                    break;
                }
            }
        }
        Value::Array(values) if event.is_none() => {
            let count = values.len();
            let start = count.saturating_sub(state.budget.max_recent_events);
            for (index, child) in values.iter().enumerate().skip(start) {
                let event_key = state.next_event_key;
                state.next_event_key = state.next_event_key.saturating_add(1);
                state.events.entry(event_key).or_default();
                let shape = canonical_shape(child, 0);
                let event = EventContext {
                    event_key,
                    temporal_distance: u16::try_from(count.saturating_sub(index + 1))
                        .unwrap_or(u16::MAX),
                    event_class: source_event_class(child),
                    topology_neighborhood_root_sha256: sha256_bytes(shape.as_bytes()),
                };
                visit_value(child, Some(event), state);
                if state.stopped {
                    break;
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                visit_value(child, event.clone(), state);
                if state.stopped {
                    break;
                }
            }
        }
        Value::String(text) => {
            let remaining = state
                .budget
                .max_text_bytes
                .saturating_sub(state.text_bytes_visited);
            if remaining == 0 {
                state.stopped = true;
                return;
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
            if let Ok(embedded) = serde_json::from_str::<Value>(bounded) {
                visit_value(&embedded, event, state);
            }
        }
        Value::Number(number) => {
            let token = number.to_string();
            record_anchor(&token, event.as_ref(), state);
            if number.is_i64() || number.is_u64() {
                add_raw_candidate(&token, BindingValueTypeV1::Integer, event.as_ref(), state);
            }
        }
        Value::Bool(value) => {
            let token = value.to_string();
            record_anchor(&token, event.as_ref(), state);
            add_raw_candidate(&token, BindingValueTypeV1::Boolean, event.as_ref(), state);
        }
        Value::Null => {}
    }
}

fn add_raw_candidate(
    token: &str,
    value_type: BindingValueTypeV1,
    event: Option<&EventContext>,
    state: &mut ExtractionState<'_>,
) {
    if state.raw_candidates.len() >= state.budget.max_candidates_per_row.saturating_mul(8) {
        state.stopped = true;
        return;
    }
    let normalized = token.to_owned();
    let action_equivalence_sha256 = match value_type {
        BindingValueTypeV1::Integer => token
            .parse::<u64>()
            .ok()
            .and_then(|value| sha256_json(&value).ok()),
        BindingValueTypeV1::Boolean => token
            .parse::<bool>()
            .ok()
            .and_then(|value| sha256_json(&value).ok()),
        BindingValueTypeV1::String | BindingValueTypeV1::Identifier => sha256_json(&token).ok(),
    };
    let Some(action_equivalence_sha256) = action_equivalence_sha256 else {
        return;
    };
    let event_key = event.map_or(0, |event| event.event_key);
    let raw = RawCandidate {
        normalized,
        action_equivalence_sha256: action_equivalence_sha256.clone(),
        value_type,
        event_key,
        temporal_distance: event.map_or(u16::MAX, |event| event.temporal_distance),
        event_class: event.map_or(BindingSourceEventClassV1::Unknown, |event| {
            event.event_class
        }),
        topology_neighborhood_root_sha256: event.map_or_else(
            || sha256_bytes(b"root-scalar"),
            |event| event.topology_neighborhood_root_sha256.clone(),
        ),
    };
    state
        .event_candidate_values
        .entry(event_key)
        .or_default()
        .insert(action_equivalence_sha256.clone());
    state
        .candidate_events
        .entry(action_equivalence_sha256)
        .or_default()
        .insert(event_key);
    state.raw_candidates.push(raw);
}

fn record_anchor(text: &str, event: Option<&EventContext>, state: &mut ExtractionState<'_>) {
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
    row_id_sha256: &str,
    context: &PreActionBindingContextV1,
    state: &ExtractionState<'_>,
) -> Vec<BindingCandidateNodeV1> {
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
        BTreeMap::<(String, BindingCandidateFeaturesV1), (usize, BTreeSet<String>)>::new();
    for raw in &state.raw_candidates {
        let same_value_events = state
            .candidate_events
            .get(&raw.action_equivalence_sha256)
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
        let features = BindingCandidateFeaturesV1 {
            source_event_class: raw.event_class,
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
            topology_neighborhood_root_sha256: raw.topology_neighborhood_root_sha256.clone(),
        };
        let entry = grouped
            .entry((raw.action_equivalence_sha256.clone(), features))
            .or_default();
        entry.0 = entry.0.saturating_add(1);
        entry.1.insert(raw.normalized.clone());
    }
    grouped
        .into_iter()
        .map(
            |((action_equivalence_sha256, features), (occurrences, _))| {
                let candidate_id_sha256 = sha256_json(&(
                    CANDIDATE_RELATION_GRAPH_SCHEMA_V1,
                    row_id_sha256,
                    &action_equivalence_sha256,
                    &features,
                ))
                .unwrap_or_else(|_| sha256_bytes(b"binding-candidate-serialization-error"));
                BindingCandidateNodeV1 {
                    candidate_id_sha256,
                    action_equivalence_sha256,
                    features,
                    occurrence_count: u16::try_from(occurrences).unwrap_or(u16::MAX),
                }
            },
        )
        .collect()
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

fn source_event_class(value: &Value) -> BindingSourceEventClassV1 {
    let mut stats = ShapeStats::default();
    collect_shape_stats(collapse_singleton_object(value), &mut stats, 0);
    match (
        stats.strings > 0,
        stats.structured > 1,
        stats.scalars > stats.strings,
    ) {
        (true, false, false) => BindingSourceEventClassV1::Textual,
        (false, true, _) => BindingSourceEventClassV1::Structured,
        (true, true, _) => BindingSourceEventClassV1::Mixed,
        (false, false, true) => BindingSourceEventClassV1::Scalar,
        _ => BindingSourceEventClassV1::Unknown,
    }
}

fn collapse_singleton_object(mut value: &Value) -> &Value {
    while let Value::Object(values) = value {
        if values.len() != 1 {
            break;
        }
        let Some(child) = values.values().next() else {
            break;
        };
        value = child;
    }
    value
}

fn collect_shape_stats(value: &Value, stats: &mut ShapeStats, depth: usize) {
    if depth > 32 {
        return;
    }
    match value {
        Value::Object(values) => {
            stats.structured = stats.structured.saturating_add(1);
            for value in values.values() {
                collect_shape_stats(value, stats, depth + 1);
            }
        }
        Value::Array(values) => {
            stats.structured = stats.structured.saturating_add(1);
            for value in values {
                collect_shape_stats(value, stats, depth + 1);
            }
        }
        Value::String(_) => {
            stats.strings = stats.strings.saturating_add(1);
            stats.scalars = stats.scalars.saturating_add(1);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            stats.scalars = stats.scalars.saturating_add(1);
        }
    }
}

fn canonical_shape(value: &Value, depth: usize) -> String {
    if depth > 32 {
        return "depth_limit".to_owned();
    }
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(_) => "bool".to_owned(),
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer".to_owned(),
        Value::Number(_) => "number".to_owned(),
        Value::String(value) => {
            if serde_json::from_str::<Value>(value).is_ok() {
                "string:embedded_json".to_owned()
            } else if value.contains('\n') {
                "string:multiline".to_owned()
            } else if value.chars().any(char::is_whitespace) {
                "string:text".to_owned()
            } else {
                "string:scalar".to_owned()
            }
        }
        Value::Array(values) => {
            let mut children = values
                .iter()
                .map(|value| canonical_shape(value, depth + 1))
                .collect::<Vec<_>>();
            children.sort();
            format!("array[{}]", children.join(","))
        }
        Value::Object(values) => {
            let mut children = values
                .values()
                .map(|value| canonical_shape(value, depth + 1))
                .collect::<Vec<_>>();
            children.sort();
            children.dedup();
            if children.len() == 1 {
                children.pop().unwrap_or_default()
            } else {
                format!("object{{{}}}", children.join(","))
            }
        }
    }
}

fn add_group_edges<F>(
    nodes: &[BindingCandidateNodeV1],
    key: F,
    relation: BindingCandidateRelationKindV1,
    edges: &mut BTreeSet<BindingCandidateRelationEdgeV1>,
) where
    F: Fn(&BindingCandidateNodeV1) -> String,
{
    let mut groups = BTreeMap::<String, Vec<&BindingCandidateNodeV1>>::new();
    for node in nodes {
        groups.entry(key(node)).or_default().push(node);
    }
    for nodes in groups.values_mut() {
        nodes.sort_by(|left, right| left.candidate_id_sha256.cmp(&right.candidate_id_sha256));
        if let Some(first) = nodes.first().copied() {
            for node in nodes.iter().copied().skip(1) {
                edges.insert(edge(first, node, relation));
            }
        }
    }
}

fn edge(
    left: &BindingCandidateNodeV1,
    right: &BindingCandidateNodeV1,
    relation: BindingCandidateRelationKindV1,
) -> BindingCandidateRelationEdgeV1 {
    let (left, right) = if left.candidate_id_sha256 <= right.candidate_id_sha256 {
        (left, right)
    } else {
        (right, left)
    };
    BindingCandidateRelationEdgeV1 {
        left_candidate_id_sha256: left.candidate_id_sha256.clone(),
        right_candidate_id_sha256: right.candidate_id_sha256.clone(),
        relation,
    }
}

#[doc(hidden)]
pub fn binding_feature_predicates_v1(
    features: &BindingCandidateFeaturesV1,
) -> Vec<BindingPredicateV1> {
    vec![
        BindingPredicateV1::SourceEventClass {
            value: features.source_event_class,
        },
        BindingPredicateV1::CallLineage {
            value: features.call_lineage,
        },
        BindingPredicateV1::CapabilityClass {
            value: features.capability_class,
        },
        BindingPredicateV1::TemporalDistance {
            value: features.temporal_distance,
        },
        BindingPredicateV1::CompletionState {
            value: features.completion_state,
        },
        BindingPredicateV1::EventCandidateCardinality {
            value: features.event_candidate_cardinality,
        },
        BindingPredicateV1::ValueType {
            value: features.value_type,
        },
        BindingPredicateV1::RequestRelation {
            value: features.request_relation,
        },
        BindingPredicateV1::TopologyNeighborhood {
            root_sha256: features.topology_neighborhood_root_sha256.clone(),
        },
    ]
}

fn add_predicate_subsets(
    atoms: &[BindingPredicateV1],
    max_depth: usize,
    max_hypotheses: usize,
    output: &mut BTreeSet<Vec<BindingPredicateV1>>,
    exhausted: &mut bool,
) {
    fn visit(
        atoms: &[BindingPredicateV1],
        start: usize,
        max_depth: usize,
        current: &mut Vec<BindingPredicateV1>,
        max_hypotheses: usize,
        output: &mut BTreeSet<Vec<BindingPredicateV1>>,
        exhausted: &mut bool,
    ) {
        if *exhausted || current.len() == max_depth {
            return;
        }
        for index in start..atoms.len() {
            current.push(atoms[index].clone());
            output.insert(current.clone());
            if output.len() >= max_hypotheses {
                *exhausted = true;
                current.pop();
                return;
            }
            visit(
                atoms,
                index + 1,
                max_depth,
                current,
                max_hypotheses,
                output,
                exhausted,
            );
            current.pop();
        }
    }
    visit(
        atoms,
        0,
        max_depth,
        &mut Vec::new(),
        max_hypotheses,
        output,
        exhausted,
    );
}

fn score_hypothesis(
    predicates: &[BindingPredicateV1],
    graphs: &[FrozenCandidateRelationGraphV1],
    receipts: &BTreeMap<String, &ExpectedBindingReceiptV1>,
) -> Result<BindingHypothesisScoreV1, BindingEvidenceErrorV1> {
    let mut positive_rows_covered = 0_usize;
    let mut positive_rows_uncovered = 0_usize;
    let mut ambiguous_rows = 0_usize;
    let mut wrong_bindings = 0_usize;
    let mut negative_accepts = 0_usize;
    let mut decisions = Vec::new();
    for graph in graphs {
        let receipt = receipts[&graph.graph.row_id_sha256];
        let classes = graph
            .graph
            .nodes
            .iter()
            .filter(|node| {
                predicates
                    .iter()
                    .all(|predicate| binding_predicate_matches_v1(predicate, node))
            })
            .map(|node| node.action_equivalence_sha256.clone())
            .collect::<BTreeSet<_>>();
        match receipt.label {
            BindingEvaluationLabelV1::ApplicabilityNegative => {
                negative_accepts += usize::from(!classes.is_empty());
            }
            BindingEvaluationLabelV1::Positive => {
                let expected = receipt
                    .expected_action_equivalence_sha256
                    .as_deref()
                    .ok_or(BindingEvidenceErrorV1::InvalidReceipt)?;
                match classes.len() {
                    0 => positive_rows_uncovered = positive_rows_uncovered.saturating_add(1),
                    1 if classes.contains(expected) => {
                        positive_rows_covered = positive_rows_covered.saturating_add(1);
                        decisions.push((graph.graph.row_id_sha256.as_str(), expected));
                    }
                    1 => wrong_bindings = wrong_bindings.saturating_add(1),
                    _ => ambiguous_rows = ambiguous_rows.saturating_add(1),
                }
            }
        }
    }
    let complete = positive_rows_uncovered == 0
        && ambiguous_rows == 0
        && wrong_bindings == 0
        && negative_accepts == 0;
    Ok(BindingHypothesisScoreV1 {
        hypothesis_id_sha256: sha256_json(&("binding-hypothesis-v1", predicates))?,
        predicates: predicates.to_vec(),
        positive_rows_covered,
        positive_rows_uncovered,
        ambiguous_rows,
        wrong_bindings,
        negative_accepts,
        selected_action_class_root_sha256: complete.then(|| sha256_json(&decisions)).transpose()?,
        complete,
    })
}

#[doc(hidden)]
pub fn binding_predicate_matches_v1(
    predicate: &BindingPredicateV1,
    node: &BindingCandidateNodeV1,
) -> bool {
    match predicate {
        BindingPredicateV1::SourceEventClass { value } => {
            &node.features.source_event_class == value
        }
        BindingPredicateV1::CallLineage { value } => &node.features.call_lineage == value,
        BindingPredicateV1::CapabilityClass { value } => &node.features.capability_class == value,
        BindingPredicateV1::TemporalDistance { value } => &node.features.temporal_distance == value,
        BindingPredicateV1::CompletionState { value } => &node.features.completion_state == value,
        BindingPredicateV1::EventCandidateCardinality { value } => {
            &node.features.event_candidate_cardinality == value
        }
        BindingPredicateV1::ValueType { value } => &node.features.value_type == value,
        BindingPredicateV1::RequestRelation { value } => &node.features.request_relation == value,
        BindingPredicateV1::TopologyNeighborhood { root_sha256 } => {
            &node.features.topology_neighborhood_root_sha256 == root_sha256
        }
    }
}

fn hypothesis_score_order(
    left: &BindingHypothesisScoreV1,
    right: &BindingHypothesisScoreV1,
) -> std::cmp::Ordering {
    right
        .complete
        .cmp(&left.complete)
        .then_with(|| right.positive_rows_covered.cmp(&left.positive_rows_covered))
        .then_with(|| left.wrong_bindings.cmp(&right.wrong_bindings))
        .then_with(|| left.negative_accepts.cmp(&right.negative_accepts))
        .then_with(|| left.ambiguous_rows.cmp(&right.ambiguous_rows))
        .then_with(|| {
            left.positive_rows_uncovered
                .cmp(&right.positive_rows_uncovered)
        })
        .then_with(|| right.predicates.len().cmp(&left.predicates.len()))
        .then_with(|| left.hypothesis_id_sha256.cmp(&right.hypothesis_id_sha256))
}

fn collect_ties(
    graphs: &[FrozenCandidateRelationGraphV1],
    receipts: &BTreeMap<String, &ExpectedBindingReceiptV1>,
    limit: usize,
) -> (Vec<BindingTieV1>, usize) {
    let mut ties = Vec::new();
    for graph in graphs {
        let receipt = receipts[&graph.graph.row_id_sha256];
        let Some(expected) = receipt.expected_action_equivalence_sha256.as_deref() else {
            continue;
        };
        let expected_features = graph
            .graph
            .nodes
            .iter()
            .filter(|node| node.action_equivalence_sha256 == expected)
            .map(|node| node.features.clone())
            .collect::<BTreeSet<_>>();
        for features in expected_features {
            let competitors = graph
                .graph
                .nodes
                .iter()
                .filter(|node| {
                    node.action_equivalence_sha256 != expected && node.features == features
                })
                .map(|node| node.action_equivalence_sha256.clone())
                .collect::<BTreeSet<_>>();
            if !competitors.is_empty() {
                ties.push(BindingTieV1 {
                    row_id_sha256: graph.graph.row_id_sha256.clone(),
                    expected_action_equivalence_sha256: expected.to_owned(),
                    competing_action_equivalence_sha256: competitors.into_iter().collect(),
                    shared_feature_root_sha256: sha256_json(&features)
                        .unwrap_or_else(|_| sha256_bytes(b"binding-tie-feature-error")),
                });
            }
        }
    }
    ties.sort_by(|left, right| {
        left.row_id_sha256.cmp(&right.row_id_sha256).then_with(|| {
            left.shared_feature_root_sha256
                .cmp(&right.shared_feature_root_sha256)
        })
    });
    let total = ties.len();
    ties.truncate(limit);
    (ties, total)
}

fn distinguishing_probe(tie: &BindingTieV1) -> BindingDistinguishingProbeV1 {
    let tie_root_sha256 =
        sha256_json(tie).unwrap_or_else(|_| sha256_bytes(b"binding-distinguishing-probe-error"));
    BindingDistinguishingProbeV1 {
        row_id_sha256: tie.row_id_sha256.clone(),
        tie_root_sha256,
        required_distinction: "expected_action_class_vs_competing_action_classes".to_owned(),
        probe: "Acquire a pre-action observable feature that separates the expected action-equivalence class from every competing class while holding the currently shared features constant."
            .to_owned(),
    }
}

fn expected_receipt_digest(
    receipt: &ExpectedBindingReceiptV1,
) -> Result<String, BindingEvidenceErrorV1> {
    sha256_json(&(
        receipt.schema.as_str(),
        receipt.row_id_sha256.as_str(),
        receipt.frozen_graph_root_sha256.as_str(),
        receipt.label,
        receipt.expected_action_equivalence_sha256.as_deref(),
        receipt.baseline_outcome,
    ))
}

fn binding_report_digest(
    report: &BindingVersionSpaceReportV1,
) -> Result<String, BindingEvidenceErrorV1> {
    let mut material = report.clone();
    material.report_sha256.clear();
    sha256_json(&material)
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

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingEvidenceErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|_| BindingEvidenceErrorV1::Serialization)
}

#[cfg(test)]
#[path = "binding_evidence_tests.rs"]
mod tests;
