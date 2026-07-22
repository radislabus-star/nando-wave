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
use nando_operator_kernel::{
    CanonicalRuntimeStructuralViewV3, StructuralCandidateObservationV3, StructuralContextV3,
    StructuralExtractionBudgetV3, StructuralExtractionScopeV3, StructuralExtractionV3,
    StructuralRelationKindV3, StructuralSourceRelationV3, canonicalize_runtime_structural_view_v3,
    extract_structural_surface_v3,
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
        let extraction = extract_structural_surface_v3(
            request_text,
            provider_payload,
            StructuralContextV3 {
                call_shape_count: context.call_shape_count,
                capability_count: context.capability_count,
                completion_state: context.completion_state,
                temporal_relation_count: context.temporal_relation_count,
                cardinality_relation_count: context.cardinality_relation_count,
                topology_neighborhood_root_sha256: context
                    .topology_neighborhood_root_sha256
                    .clone(),
            },
            StructuralExtractionBudgetV3 {
                max_json_nodes: budget.max_json_nodes,
                max_text_bytes: budget.max_text_bytes,
                max_recent_events: budget.max_recent_events,
                max_role_candidates: budget.max_candidates_per_row,
                max_relations: budget.max_relation_edges_per_row,
            },
            StructuralExtractionScopeV3::FrozenEvidence,
        )
        .map_err(|_| BindingEvidenceErrorV1::InvalidCorpus)?;
        let mut nodes = extraction
            .candidates
            .into_iter()
            .map(|candidate| {
                let features = BindingCandidateFeaturesV1 {
                    source_event_class: candidate.features.source_event_class,
                    call_lineage: candidate.features.call_lineage,
                    capability_class: candidate.features.capability_class,
                    temporal_distance: candidate.features.temporal_distance,
                    completion_state: candidate.features.completion_state,
                    event_candidate_cardinality: candidate.features.event_candidate_cardinality,
                    value_type: candidate.features.value_type,
                    request_relation: candidate.features.request_relation,
                    topology_neighborhood_root_sha256: candidate
                        .features
                        .topology_neighborhood_root_sha256,
                };
                let candidate_id_sha256 = sha256_json(&(
                    CANDIDATE_RELATION_GRAPH_SCHEMA_V1,
                    &row_id_sha256,
                    &candidate.value_sha256,
                    &features,
                ))
                .unwrap_or_else(|_| sha256_bytes(b"binding-candidate-serialization-error"));
                BindingCandidateNodeV1 {
                    candidate_id_sha256,
                    action_equivalence_sha256: candidate.value_sha256,
                    features,
                    occurrence_count: candidate.occurrence_count,
                }
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.candidate_id_sha256.cmp(&right.candidate_id_sha256));
        nodes.truncate(budget.max_candidates_per_row);
        Ok(Self {
            schema: PRE_ACTION_BINDING_SURFACE_SCHEMA_V1.to_owned(),
            row_id_sha256,
            evidence_ref_sha256,
            context,
            candidates: nodes,
            json_nodes_visited: extraction.json_nodes_visited,
            text_bytes_visited: extraction.text_bytes_visited,
            candidates_before_budget: extraction.candidates_before_budget,
            candidate_budget_exhausted: extraction.candidate_budget_exhausted,
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

pub fn canonical_runtime_structural_view_v3_from_frozen_graph(
    frozen: &FrozenCandidateRelationGraphV1,
) -> Result<CanonicalRuntimeStructuralViewV3, BindingEvidenceErrorV1> {
    if frozen.schema != FROZEN_CANDIDATE_RELATION_GRAPH_SCHEMA_V1
        || sha256_json(&frozen.graph)? != frozen.graph_root_sha256
    {
        return Err(BindingEvidenceErrorV1::InvalidCorpus);
    }
    let source_ids = frozen
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            u16::try_from(index)
                .map(|source_id| (node.candidate_id_sha256.as_str(), source_id))
                .map_err(|_| BindingEvidenceErrorV1::InvalidCorpus)
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let candidates = frozen
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            Ok(StructuralCandidateObservationV3 {
                source_role_id: u16::try_from(index)
                    .map_err(|_| BindingEvidenceErrorV1::InvalidCorpus)?,
                value_sha256: node.action_equivalence_sha256.clone(),
                features: nando_operator_kernel::StructuralCandidateFeaturesV3 {
                    source_event_class: node.features.source_event_class,
                    call_lineage: node.features.call_lineage,
                    capability_class: node.features.capability_class,
                    temporal_distance: node.features.temporal_distance,
                    completion_state: node.features.completion_state,
                    event_candidate_cardinality: node.features.event_candidate_cardinality,
                    value_type: node.features.value_type,
                    request_relation: node.features.request_relation,
                    topology_neighborhood_root_sha256: node
                        .features
                        .topology_neighborhood_root_sha256
                        .clone(),
                },
                occurrence_count: node.occurrence_count,
            })
        })
        .collect::<Result<Vec<_>, BindingEvidenceErrorV1>>()?;
    let relations = frozen
        .graph
        .edges
        .iter()
        .map(|edge| {
            let left_source_role_id = source_ids
                .get(edge.left_candidate_id_sha256.as_str())
                .copied()
                .ok_or(BindingEvidenceErrorV1::InvalidCorpus)?;
            let right_source_role_id = source_ids
                .get(edge.right_candidate_id_sha256.as_str())
                .copied()
                .ok_or(BindingEvidenceErrorV1::InvalidCorpus)?;
            let relation = match edge.relation {
                BindingCandidateRelationKindV1::SameActionEquivalence => {
                    StructuralRelationKindV3::SameValue
                }
                BindingCandidateRelationKindV1::SameCallLineage => {
                    StructuralRelationKindV3::SameCallLineage
                }
                BindingCandidateRelationKindV1::SameTopologyNeighborhood => {
                    StructuralRelationKindV3::SameTopologyNeighborhood
                }
                BindingCandidateRelationKindV1::AdjacentTemporalDistance => {
                    StructuralRelationKindV3::AdjacentTemporalDistance
                }
            };
            Ok(StructuralSourceRelationV3 {
                left_source_role_id,
                right_source_role_id,
                relation,
            })
        })
        .collect::<Result<Vec<_>, BindingEvidenceErrorV1>>()?;
    let context = StructuralContextV3 {
        call_shape_count: frozen.graph.context.call_shape_count,
        capability_count: frozen.graph.context.capability_count,
        completion_state: frozen.graph.context.completion_state,
        temporal_relation_count: frozen.graph.context.temporal_relation_count,
        cardinality_relation_count: frozen.graph.context.cardinality_relation_count,
        topology_neighborhood_root_sha256: frozen
            .graph
            .context
            .topology_neighborhood_root_sha256
            .clone(),
    };
    canonicalize_runtime_structural_view_v3(
        context,
        &StructuralExtractionV3 {
            candidates,
            relations,
            json_nodes_visited: 0,
            text_bytes_visited: 0,
            candidates_before_budget: frozen.graph.nodes.len(),
            candidate_budget_exhausted: frozen.graph.extraction_budget_exhausted,
            relation_budget_exhausted: frozen.graph.relation_budget_exhausted,
        },
    )
    .map_err(|_| BindingEvidenceErrorV1::InvalidCorpus)
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
