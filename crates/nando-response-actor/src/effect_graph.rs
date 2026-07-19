use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AtomSource, AtomValueType, RelationAtom, TeacherTransition};

pub const EFFECT_GRAPH_SCHEMA_V1: &str = "nando.effect-graph.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectGraphPolicy {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_alignments: usize,
    pub max_canonical_permutations: usize,
}

impl Default for EffectGraphPolicy {
    fn default() -> Self {
        Self {
            max_nodes: 32,
            max_edges: 256,
            max_alignments: 4_096,
            max_canonical_permutations: 16_384,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectSource {
    Request,
    Observation,
    Action,
    Outcome,
    Derived,
}

impl From<AtomSource> for EffectSource {
    fn from(value: AtomSource) -> Self {
        match value {
            AtomSource::Request => Self::Request,
            AtomSource::Observation => Self::Observation,
            AtomSource::Action => Self::Action,
            AtomSource::Outcome => Self::Outcome,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectNodeKind {
    Scalar,
    Collection,
    Operation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOperationKind {
    Call,
    Project,
    Status,
    PlanAdvance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectEdgeKind {
    Equal,
    CopiedFrom,
    ConsumedBy,
    Produces,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectGraphCompleteness {
    Complete,
    Ambiguous,
    InsufficientEvidence,
    OverBudget,
    Invalid,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectNode {
    pub index: u16,
    pub source: EffectSource,
    pub kind: EffectNodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<AtomValueType>,
    pub unique: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<EffectOperationKind>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectEdge {
    pub from: u16,
    pub to: u16,
    pub kind: EffectEdgeKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectGraph {
    pub schema: String,
    pub nodes: Vec<EffectNode>,
    pub edges: Vec<EffectEdge>,
    pub completeness: EffectGraphCompleteness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_sha256: Option<String>,
    pub alignment_candidates: usize,
    pub canonical_permutations: usize,
}

impl EffectGraph {
    fn incomplete(completeness: EffectGraphCompleteness) -> Self {
        Self {
            schema: EFFECT_GRAPH_SCHEMA_V1.to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
            completeness,
            canonical_sha256: None,
            alignment_candidates: 0,
            canonical_permutations: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct DraftNode {
    source: EffectSource,
    kind: EffectNodeKind,
    value_type: Option<AtomValueType>,
    unique: bool,
    operation: Option<EffectOperationKind>,
    value_sha256: Option<String>,
}

impl DraftNode {
    fn public(&self, index: u16) -> EffectNode {
        EffectNode {
            index,
            source: self.source,
            kind: self.kind,
            value_type: self.value_type,
            unique: self.unique,
            operation: self.operation,
        }
    }

    fn color(
        &self,
    ) -> (
        EffectSource,
        EffectNodeKind,
        Option<AtomValueType>,
        bool,
        Option<EffectOperationKind>,
    ) {
        (
            self.source,
            self.kind,
            self.value_type,
            self.unique,
            self.operation,
        )
    }
}

#[derive(Clone, Debug)]
struct CanonicalGraph {
    nodes: Vec<EffectNode>,
    edges: Vec<EffectEdge>,
    bytes: Vec<u8>,
    permutations: usize,
}

#[derive(Default)]
pub struct EffectGraphBuilder {
    policy: EffectGraphPolicy,
}

impl EffectGraphBuilder {
    #[must_use]
    pub fn new(policy: EffectGraphPolicy) -> Self {
        Self { policy }
    }

    #[must_use]
    pub fn build(&self, transition: &TeacherTransition) -> EffectGraph {
        if !transition.outcome.verifier.accepted
            || transition.before.contains_teacher_atoms()
            || transition.runtime_parity_case.is_none()
        {
            return EffectGraph::incomplete(EffectGraphCompleteness::Invalid);
        }

        let mut slot_nodes = BTreeMap::<u16, usize>::new();
        let mut nodes = Vec::<DraftNode>::new();
        let atoms = transition
            .before
            .atoms
            .iter()
            .chain(transition.outcome.action.atoms.iter())
            .collect::<Vec<_>>();

        for atom in &atoms {
            if let RelationAtom::TypedSlot {
                slot_id,
                value_type,
                source,
                value_sha256,
            } = atom
            {
                let key = *slot_id;
                if let Some(index) = slot_nodes.get(&key).copied() {
                    let node = &nodes[index];
                    if node.source != EffectSource::from(*source)
                        || node.value_type != Some(*value_type)
                        || node.value_sha256.as_deref() != Some(value_sha256.as_str())
                    {
                        return EffectGraph::incomplete(EffectGraphCompleteness::Invalid);
                    }
                    continue;
                }
                let index = nodes.len();
                slot_nodes.insert(key, index);
                nodes.push(DraftNode {
                    source: EffectSource::from(*source),
                    kind: if *value_type == AtomValueType::Collection {
                        EffectNodeKind::Collection
                    } else {
                        EffectNodeKind::Scalar
                    },
                    value_type: Some(*value_type),
                    unique: false,
                    operation: None,
                    value_sha256: Some(value_sha256.clone()),
                });
            }
        }

        for atom in &atoms {
            if let RelationAtom::UniqueSlot { slot_id } = atom
                && let Some(index) = slot_nodes.get(slot_id).copied()
            {
                nodes[index].unique = true;
            }
        }

        let operation_kinds = operation_kinds(&atoms);
        let mut operation_nodes = BTreeMap::<EffectOperationKind, usize>::new();
        for operation in operation_kinds {
            operation_nodes.insert(operation, nodes.len());
            nodes.push(DraftNode {
                source: EffectSource::Derived,
                kind: EffectNodeKind::Operation,
                value_type: None,
                unique: true,
                operation: Some(operation),
                value_sha256: None,
            });
        }

        if nodes.is_empty() || nodes.len() > self.policy.max_nodes {
            return EffectGraph::incomplete(if nodes.is_empty() {
                EffectGraphCompleteness::InsufficientEvidence
            } else {
                EffectGraphCompleteness::OverBudget
            });
        }

        let mut base_edges = BTreeSet::<(usize, usize, EffectEdgeKind)>::new();
        for atom in &atoms {
            if let RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } = atom
                && let (Some(left), Some(right)) = (
                    slot_nodes.get(left_slot).copied(),
                    slot_nodes.get(right_slot).copied(),
                )
            {
                insert_symmetric_edge(&mut base_edges, left, right, EffectEdgeKind::Equal);
            }
        }

        let call_node = operation_nodes.get(&EffectOperationKind::Call).copied();
        let mut required_action_slots = BTreeSet::<usize>::new();
        for atom in &atoms {
            if let RelationAtom::ActionRoleArgument { slot_id, .. } = atom
                && let Some(slot) = slot_nodes.get(slot_id).copied()
            {
                required_action_slots.insert(slot);
                if let Some(operation) = call_node {
                    base_edges.insert((slot, operation, EffectEdgeKind::ConsumedBy));
                }
            }
        }

        for (index, node) in nodes.iter().enumerate() {
            if matches!(node.source, EffectSource::Action | EffectSource::Outcome)
                && node.kind != EffectNodeKind::Operation
            {
                required_action_slots.insert(index);
            }
        }

        if base_edges.len() > self.policy.max_edges {
            return EffectGraph::incomplete(EffectGraphCompleteness::OverBudget);
        }

        let mut choices = Vec::<(usize, Vec<usize>)>::new();
        for action_index in required_action_slots {
            if base_edges.iter().any(|(from, to, kind)| {
                *kind == EffectEdgeKind::Equal
                    && (*from == action_index || *to == action_index)
                    && is_pre_action(&nodes[if *from == action_index { *to } else { *from }])
            }) {
                continue;
            }
            let action = &nodes[action_index];
            let compatible = nodes
                .iter()
                .enumerate()
                .filter(|(_, candidate)| {
                    is_pre_action(candidate)
                        && candidate.value_type == action.value_type
                        && candidate.value_sha256 == action.value_sha256
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if compatible.is_empty() {
                return EffectGraph::incomplete(EffectGraphCompleteness::InsufficientEvidence);
            }
            choices.push((action_index, compatible));
        }

        let alignment_count = choices.iter().try_fold(1_usize, |count, (_, values)| {
            count.checked_mul(values.len())
        });
        let Some(alignment_count) = alignment_count else {
            return EffectGraph::incomplete(EffectGraphCompleteness::OverBudget);
        };
        if alignment_count > self.policy.max_alignments {
            return EffectGraph::incomplete(EffectGraphCompleteness::OverBudget);
        }

        let mut canonical = BTreeMap::<Vec<u8>, CanonicalGraph>::new();
        let mut selected = Vec::<usize>::with_capacity(choices.len());
        let mut permutations = 0_usize;
        let result = enumerate_alignments(
            &nodes,
            &base_edges,
            &choices,
            0,
            &mut selected,
            self.policy,
            &mut canonical,
            &mut permutations,
        );
        if result.is_err() {
            return EffectGraph::incomplete(EffectGraphCompleteness::OverBudget);
        }
        if canonical.len() != 1 {
            return EffectGraph {
                alignment_candidates: alignment_count,
                canonical_permutations: permutations,
                ..EffectGraph::incomplete(EffectGraphCompleteness::Ambiguous)
            };
        }
        let graph = canonical.into_values().next().expect("one canonical graph");
        let canonical_sha256 = format!("{:x}", Sha256::digest(&graph.bytes));
        EffectGraph {
            schema: EFFECT_GRAPH_SCHEMA_V1.to_owned(),
            nodes: graph.nodes,
            edges: graph.edges,
            completeness: EffectGraphCompleteness::Complete,
            canonical_sha256: Some(canonical_sha256),
            alignment_candidates: alignment_count,
            canonical_permutations: permutations.max(graph.permutations),
        }
    }
}

fn operation_kinds(atoms: &[&RelationAtom]) -> BTreeSet<EffectOperationKind> {
    atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionFunction { .. }
            | RelationAtom::ActionCustomTool { .. }
            | RelationAtom::ActionInnerTool { .. }
            | RelationAtom::ActionRoleArgument { .. }
            | RelationAtom::ActionIntegerArgument { .. }
            | RelationAtom::ActionStringArgument { .. }
            | RelationAtom::ActionBooleanArgument { .. } => Some(EffectOperationKind::Call),
            RelationAtom::ActionResultProjection { .. }
            | RelationAtom::ActionOutputProjection { .. }
            | RelationAtom::ActionJsonResultProjection
            | RelationAtom::ActionValueProjection { .. } => Some(EffectOperationKind::Project),
            RelationAtom::ActionStatusProjection { .. } => Some(EffectOperationKind::Status),
            RelationAtom::ActionPlanAdvance => Some(EffectOperationKind::PlanAdvance),
            _ => None,
        })
        .collect()
}

fn is_pre_action(node: &DraftNode) -> bool {
    matches!(
        node.source,
        EffectSource::Request | EffectSource::Observation
    )
}

fn insert_symmetric_edge(
    edges: &mut BTreeSet<(usize, usize, EffectEdgeKind)>,
    left: usize,
    right: usize,
    kind: EffectEdgeKind,
) {
    edges.insert((left.min(right), left.max(right), kind));
}

#[allow(clippy::too_many_arguments)]
fn enumerate_alignments(
    nodes: &[DraftNode],
    base_edges: &BTreeSet<(usize, usize, EffectEdgeKind)>,
    choices: &[(usize, Vec<usize>)],
    depth: usize,
    selected: &mut Vec<usize>,
    policy: EffectGraphPolicy,
    output: &mut BTreeMap<Vec<u8>, CanonicalGraph>,
    permutations: &mut usize,
) -> Result<(), ()> {
    if depth < choices.len() {
        for candidate in &choices[depth].1 {
            selected.push(*candidate);
            enumerate_alignments(
                nodes,
                base_edges,
                choices,
                depth + 1,
                selected,
                policy,
                output,
                permutations,
            )?;
            selected.pop();
        }
        return Ok(());
    }

    let mut edges = base_edges.clone();
    for ((action, _), source) in choices.iter().zip(selected.iter()) {
        edges.insert((*action, *source, EffectEdgeKind::CopiedFrom));
    }
    if edges.len() > policy.max_edges {
        return Err(());
    }
    let canonical = canonicalize(nodes, &edges, policy.max_canonical_permutations)?;
    *permutations = permutations.saturating_add(canonical.permutations);
    if *permutations > policy.max_canonical_permutations {
        return Err(());
    }
    output.entry(canonical.bytes.clone()).or_insert(canonical);
    Ok(())
}

fn canonicalize(
    nodes: &[DraftNode],
    edges: &BTreeSet<(usize, usize, EffectEdgeKind)>,
    max_permutations: usize,
) -> Result<CanonicalGraph, ()> {
    let mut groups = BTreeMap::<_, Vec<usize>>::new();
    for (index, node) in nodes.iter().enumerate() {
        groups.entry(node.color()).or_default().push(index);
    }
    let groups = groups.into_values().collect::<Vec<_>>();
    let permutation_count = groups.iter().try_fold(1_usize, |count, group| {
        count.checked_mul(factorial(group.len())?)
    });
    let Some(permutation_count) = permutation_count else {
        return Err(());
    };
    if permutation_count > max_permutations {
        return Err(());
    }

    let mut best = None::<CanonicalGraph>;
    let mut ordered = Vec::<usize>::with_capacity(nodes.len());
    enumerate_node_groups(nodes, edges, &groups, 0, &mut ordered, &mut best)?;
    let mut best = best.ok_or(())?;
    best.permutations = permutation_count;
    Ok(best)
}

fn enumerate_node_groups(
    nodes: &[DraftNode],
    edges: &BTreeSet<(usize, usize, EffectEdgeKind)>,
    groups: &[Vec<usize>],
    group_index: usize,
    ordered: &mut Vec<usize>,
    best: &mut Option<CanonicalGraph>,
) -> Result<(), ()> {
    if group_index == groups.len() {
        let mut old_to_new = vec![0_u16; nodes.len()];
        for (new, old) in ordered.iter().enumerate() {
            old_to_new[*old] = u16::try_from(new).map_err(|_| ())?;
        }
        let canonical_nodes = ordered
            .iter()
            .enumerate()
            .map(|(new, old)| {
                u16::try_from(new)
                    .map(|index| nodes[*old].public(index))
                    .map_err(|_| ())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut canonical_edges = edges
            .iter()
            .map(|(from, to, kind)| EffectEdge {
                from: old_to_new[*from],
                to: old_to_new[*to],
                kind: *kind,
            })
            .collect::<Vec<_>>();
        canonical_edges.sort();
        let bytes =
            serde_json::to_vec(&(EFFECT_GRAPH_SCHEMA_V1, &canonical_nodes, &canonical_edges))
                .map_err(|_| ())?;
        if best.as_ref().is_none_or(|current| bytes < current.bytes) {
            *best = Some(CanonicalGraph {
                nodes: canonical_nodes,
                edges: canonical_edges,
                bytes,
                permutations: 0,
            });
        }
        return Ok(());
    }

    let mut group = groups[group_index].clone();
    enumerate_permutations(&mut group, 0, &mut |permutation| {
        ordered.extend_from_slice(permutation);
        let result = enumerate_node_groups(nodes, edges, groups, group_index + 1, ordered, best);
        ordered.truncate(ordered.len().saturating_sub(permutation.len()));
        result
    })
}

fn enumerate_permutations(
    values: &mut [usize],
    index: usize,
    visit: &mut impl FnMut(&[usize]) -> Result<(), ()>,
) -> Result<(), ()> {
    if index == values.len() {
        return visit(values);
    }
    for candidate in index..values.len() {
        values.swap(index, candidate);
        enumerate_permutations(values, index + 1, visit)?;
        values.swap(index, candidate);
    }
    Ok(())
}

fn factorial(value: usize) -> Option<usize> {
    (2..=value).try_fold(1_usize, |product, next| product.checked_mul(next))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        RuntimeFrame, RuntimeParityCase, TeacherActionAst, TeacherOutcome, TeacherVerifierEvidence,
    };

    fn transition(
        request_slot: u16,
        action_slot: u16,
        function_name: &str,
        argument_name: &str,
        operation: Option<RelationAtom>,
        duplicate_request: bool,
    ) -> TeacherTransition {
        let secret = "private-customer-value";
        let mut before_atoms = vec![RelationAtom::TypedSlot {
            slot_id: request_slot,
            value_type: AtomValueType::Identifier,
            source: AtomSource::Observation,
            value_sha256: format!("{:x}", Sha256::digest(secret)),
        }];
        if duplicate_request {
            before_atoms.push(RelationAtom::TypedSlot {
                slot_id: request_slot + 1,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Request,
                value_sha256: format!("{:x}", Sha256::digest(secret)),
            });
            before_atoms.push(RelationAtom::UniqueSlot {
                slot_id: request_slot + 1,
            });
        }
        let mut action_atoms = vec![
            RelationAtom::TypedSlot {
                slot_id: action_slot,
                value_type: AtomValueType::Identifier,
                source: AtomSource::Action,
                value_sha256: format!("{:x}", Sha256::digest(secret)),
            },
            RelationAtom::ActionFunction {
                value: function_name.to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: argument_name.to_owned(),
                slot_id: action_slot,
                value_type: Some(AtomValueType::Identifier),
            },
        ];
        if let Some(operation) = operation {
            action_atoms.push(operation);
        }
        TeacherTransition {
            schema: "nando.teacher-transition.v1".to_owned(),
            before: RuntimeFrame {
                schema: "nando.runtime-frame.v1".to_owned(),
                frame_id_sha256: "frame".repeat(16),
                event_id_sha256: "event".repeat(16),
                client_intent_id_sha256: "intent".repeat(16),
                session_id_sha256: "session".repeat(16),
                observed_at_unix_nanos: 1,
                extractor_version: "test".to_owned(),
                atoms: before_atoms,
                evidence_ref_sha256: "evidence".repeat(8),
            },
            outcome: TeacherOutcome {
                schema: "nando.teacher-outcome.v1".to_owned(),
                action: TeacherActionAst {
                    signature_sha256: "signature".repeat(8),
                    action_symbol: function_name.to_owned(),
                    atoms: action_atoms,
                },
                verifier: TeacherVerifierEvidence {
                    accepted: true,
                    evidence_ref_sha256: "receipt".repeat(8),
                    output_digest_sha256: "output".repeat(8),
                },
                completed_at_unix_nanos: 2,
            },
            economics: None,
            runtime_parity_case: Some(RuntimeParityCase {
                evidence_ref_sha256: "parity".repeat(8),
                capture_receipt: None,
                request_text: String::new(),
                provider_payload: json!({"value": secret}),
                expected_response: "ok".to_owned(),
            }),
        }
    }

    #[test]
    fn alpha_and_wire_renaming_keep_byte_identical_effect() {
        let left =
            EffectGraphBuilder::default().build(&transition(1, 7, "wait", "cell_id", None, false));
        let right =
            EffectGraphBuilder::default().build(&transition(44, 91, "poll", "handle", None, false));
        assert_eq!(left.completeness, EffectGraphCompleteness::Complete);
        assert_eq!(left.canonical_sha256, right.canonical_sha256);
        assert_eq!(left.nodes, right.nodes);
        assert_eq!(left.edges, right.edges);
    }

    #[test]
    fn different_typed_effect_gets_a_different_digest() {
        let call = EffectGraphBuilder::default().build(&transition(1, 7, "a", "x", None, false));
        let projected = EffectGraphBuilder::default().build(&transition(
            1,
            7,
            "a",
            "x",
            Some(RelationAtom::ActionJsonResultProjection),
            false,
        ));
        assert_ne!(call.canonical_sha256, projected.canonical_sha256);
    }

    #[test]
    fn structurally_distinct_equal_sources_are_ambiguous() {
        let graph = EffectGraphBuilder::default().build(&transition(1, 7, "a", "x", None, true));
        assert_eq!(graph.completeness, EffectGraphCompleteness::Ambiguous);
        assert!(graph.canonical_sha256.is_none());
    }

    #[test]
    fn canonical_graph_does_not_serialize_raw_teacher_text() {
        let graph = EffectGraphBuilder::default().build(&transition(
            1,
            7,
            "secret-function",
            "secret-argument",
            None,
            false,
        ));
        let bytes = serde_json::to_string(&graph).expect("effect graph serializes");
        assert!(!bytes.contains("private-customer-value"));
        assert!(!bytes.contains("secret-function"));
        assert!(!bytes.contains("secret-argument"));
    }
}
