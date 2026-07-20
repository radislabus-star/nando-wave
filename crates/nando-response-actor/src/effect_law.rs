use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AtomValueType, EFFECT_GRAPH_SCHEMA_V1, EffectEdge, EffectGraph, EffectGraphCompleteness,
    EffectNode,
};

pub const CANONICAL_EFFECT_LAW_SCHEMA_V2: &str = "nando.canonical-effect-law.v2";

const MAX_EFFECT_LAW_NODES: usize = 32;
const MAX_EFFECT_LAW_EDGES: usize = 256;
const MAX_EFFECT_LAW_ROLES: usize = 32;
const MAX_EFFECT_LAW_CONSTANTS: usize = 32;
const MAX_EFFECT_LAW_PRECONDITIONS: usize = 64;
const MAX_EFFECT_LAW_POSTCONDITIONS: usize = 64;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKindV2 {
    ContinueActiveExecution,
    InjectInputAndContinue,
    TerminateExecution,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectRoleKindV2 {
    ActiveExecution,
    InputPayload,
    Capability,
    Outcome,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectRoleV2 {
    pub node: u16,
    pub kind: EffectRoleKindV2,
    pub value_type: AtomValueType,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemanticConstantV2 {
    pub owner: EffectRoleKindV2,
    pub value_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectPredicateKindV2 {
    Present,
    Unique,
    Active,
    Empty,
    NonEmpty,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectPredicateV2 {
    pub role: EffectRoleKindV2,
    pub predicate: EffectPredicateKindV2,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectPostconditionKindV2 {
    Continues,
    InputInjected,
    Terminated,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectPostconditionV2 {
    pub role: EffectRoleKindV2,
    pub condition: EffectPostconditionKindV2,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreservedFrameContractV2 {
    pub roles: Vec<EffectRoleKindV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedSemanticFacetsV2 {
    pub effect_kind: EffectKindV2,
    pub roles: Vec<EffectRoleV2>,
    pub semantic_constants: Vec<SemanticConstantV2>,
    pub preconditions: Vec<EffectPredicateV2>,
    pub postconditions: Vec<EffectPostconditionV2>,
    pub preserved_frame: PreservedFrameContractV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectTopologyV2 {
    schema: String,
    nodes: Vec<EffectNode>,
    edges: Vec<EffectEdge>,
    canonical_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectLawV2 {
    schema: String,
    topology: CanonicalEffectTopologyV2,
    effect_kind: EffectKindV2,
    roles: Vec<EffectRoleV2>,
    semantic_constants: Vec<SemanticConstantV2>,
    preconditions: Vec<EffectPredicateV2>,
    postconditions: Vec<EffectPostconditionV2>,
    preserved_frame: PreservedFrameContractV2,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EffectLawId(String);

#[derive(Deserialize)]
struct CanonicalEffectTopologyWireV2 {
    schema: String,
    nodes: Vec<EffectNode>,
    edges: Vec<EffectEdge>,
    canonical_sha256: String,
}

#[derive(Deserialize)]
struct CanonicalEffectLawWireV2 {
    schema: String,
    topology: CanonicalEffectTopologyWireV2,
    effect_kind: EffectKindV2,
    roles: Vec<EffectRoleV2>,
    semantic_constants: Vec<SemanticConstantV2>,
    preconditions: Vec<EffectPredicateV2>,
    postconditions: Vec<EffectPostconditionV2>,
    preserved_frame: PreservedFrameContractV2,
}

impl EffectLawId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectLawError {
    IncompleteTopology,
    InvalidTopology,
    OverBudget,
    InvalidRole,
    InvalidSemanticConstant,
    InvalidFacetContract,
    Serialization,
}

impl fmt::Display for EffectLawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::IncompleteTopology => "effect topology is not complete",
            Self::InvalidTopology => "effect topology is not canonical",
            Self::OverBudget => "effect law exceeds a bounded IR limit",
            Self::InvalidRole => "effect role does not reference a compatible canonical node",
            Self::InvalidSemanticConstant => "semantic constant is not a SHA-256 commitment",
            Self::InvalidFacetContract => "semantic facets are internally inconsistent",
            Self::Serialization => "canonical effect law serialization failed",
        })
    }
}

impl std::error::Error for EffectLawError {}

impl CanonicalEffectLawV2 {
    pub fn from_effect_graph(
        graph: &EffectGraph,
        mut facets: VerifiedSemanticFacetsV2,
    ) -> Result<Self, EffectLawError> {
        let topology = canonical_topology(graph)?;
        canonicalize_facets(&topology, &mut facets)?;
        Ok(Self {
            schema: CANONICAL_EFFECT_LAW_SCHEMA_V2.to_owned(),
            topology,
            effect_kind: facets.effect_kind,
            roles: facets.roles,
            semantic_constants: facets.semantic_constants,
            preconditions: facets.preconditions,
            postconditions: facets.postconditions,
            preserved_frame: facets.preserved_frame,
        })
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawError> {
        let wire: CanonicalEffectLawWireV2 =
            serde_json::from_slice(bytes).map_err(|_| EffectLawError::Serialization)?;
        if wire.schema != CANONICAL_EFFECT_LAW_SCHEMA_V2 {
            return Err(EffectLawError::InvalidFacetContract);
        }
        let graph = EffectGraph {
            schema: wire.topology.schema,
            nodes: wire.topology.nodes,
            edges: wire.topology.edges,
            completeness: EffectGraphCompleteness::Complete,
            canonical_sha256: Some(wire.topology.canonical_sha256),
            alignment_candidates: 0,
            canonical_permutations: 0,
        };
        let facets = VerifiedSemanticFacetsV2 {
            effect_kind: wire.effect_kind,
            roles: wire.roles,
            semantic_constants: wire.semantic_constants,
            preconditions: wire.preconditions,
            postconditions: wire.postconditions,
            preserved_frame: wire.preserved_frame,
        };
        let canonical = Self::from_effect_graph(&graph, facets)?;
        if canonical.canonical_bytes()? != bytes {
            return Err(EffectLawError::InvalidFacetContract);
        }
        Ok(canonical)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EffectLawError> {
        serde_json::to_vec(self).map_err(|_| EffectLawError::Serialization)
    }

    pub fn effect_law_id(&self) -> Result<EffectLawId, EffectLawError> {
        Ok(EffectLawId(format!(
            "{:x}",
            Sha256::digest(self.canonical_bytes()?)
        )))
    }

    #[must_use]
    pub fn topology(&self) -> &CanonicalEffectTopologyV2 {
        &self.topology
    }

    #[must_use]
    pub fn effect_kind(&self) -> EffectKindV2 {
        self.effect_kind
    }

    #[must_use]
    pub fn roles(&self) -> &[EffectRoleV2] {
        &self.roles
    }

    #[must_use]
    pub fn semantic_constants(&self) -> &[SemanticConstantV2] {
        &self.semantic_constants
    }

    #[must_use]
    pub fn preconditions(&self) -> &[EffectPredicateV2] {
        &self.preconditions
    }

    #[must_use]
    pub fn postconditions(&self) -> &[EffectPostconditionV2] {
        &self.postconditions
    }

    #[must_use]
    pub fn preserved_frame(&self) -> &PreservedFrameContractV2 {
        &self.preserved_frame
    }
}

impl CanonicalEffectTopologyV2 {
    #[must_use]
    pub fn nodes(&self) -> &[EffectNode] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[EffectEdge] {
        &self.edges
    }

    #[must_use]
    pub fn canonical_sha256(&self) -> &str {
        &self.canonical_sha256
    }
}

fn canonical_topology(graph: &EffectGraph) -> Result<CanonicalEffectTopologyV2, EffectLawError> {
    if graph.completeness != EffectGraphCompleteness::Complete {
        return Err(EffectLawError::IncompleteTopology);
    }
    if graph.schema != EFFECT_GRAPH_SCHEMA_V1
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_EFFECT_LAW_NODES
        || graph.edges.len() > MAX_EFFECT_LAW_EDGES
    {
        return Err(EffectLawError::InvalidTopology);
    }

    let mut nodes = graph.nodes.clone();
    nodes.sort();
    if nodes
        .iter()
        .enumerate()
        .any(|(index, node)| usize::from(node.index) != index)
    {
        return Err(EffectLawError::InvalidTopology);
    }
    let mut edges = graph.edges.clone();
    edges.sort();
    edges.dedup();
    if edges.len() != graph.edges.len()
        || edges.iter().any(|edge| {
            usize::from(edge.from) >= nodes.len() || usize::from(edge.to) >= nodes.len()
        })
    {
        return Err(EffectLawError::InvalidTopology);
    }

    let bytes = serde_json::to_vec(&(EFFECT_GRAPH_SCHEMA_V1, &nodes, &edges))
        .map_err(|_| EffectLawError::Serialization)?;
    let canonical_sha256 = format!("{:x}", Sha256::digest(bytes));
    if graph.canonical_sha256.as_deref() != Some(canonical_sha256.as_str()) {
        return Err(EffectLawError::InvalidTopology);
    }
    Ok(CanonicalEffectTopologyV2 {
        schema: EFFECT_GRAPH_SCHEMA_V1.to_owned(),
        nodes,
        edges,
        canonical_sha256,
    })
}

fn canonicalize_facets(
    topology: &CanonicalEffectTopologyV2,
    facets: &mut VerifiedSemanticFacetsV2,
) -> Result<(), EffectLawError> {
    if facets.roles.is_empty()
        || facets.roles.len() > MAX_EFFECT_LAW_ROLES
        || facets.semantic_constants.len() > MAX_EFFECT_LAW_CONSTANTS
        || facets.preconditions.len() > MAX_EFFECT_LAW_PRECONDITIONS
        || facets.postconditions.len() > MAX_EFFECT_LAW_POSTCONDITIONS
        || facets.preserved_frame.roles.len() > MAX_EFFECT_LAW_ROLES
    {
        return Err(EffectLawError::OverBudget);
    }

    for constant in &mut facets.semantic_constants {
        constant.value_sha256.make_ascii_lowercase();
    }
    facets.roles.sort();
    facets.semantic_constants.sort();
    facets.preconditions.sort();
    facets.postconditions.sort();
    facets.preserved_frame.roles.sort();
    if has_duplicates(&facets.roles)
        || has_duplicates(&facets.semantic_constants)
        || has_duplicates(&facets.preconditions)
        || has_duplicates(&facets.postconditions)
        || has_duplicates(&facets.preserved_frame.roles)
    {
        return Err(EffectLawError::InvalidFacetContract);
    }

    let mut role_kinds = BTreeSet::new();
    let mut role_nodes = BTreeSet::new();
    for role in &facets.roles {
        let Some(node) = topology.nodes.get(usize::from(role.node)) else {
            return Err(EffectLawError::InvalidRole);
        };
        if node.value_type != Some(role.value_type)
            || !role_kinds.insert(role.kind)
            || !role_nodes.insert(role.node)
        {
            return Err(EffectLawError::InvalidRole);
        }
    }
    if !role_kinds.contains(&EffectRoleKindV2::ActiveExecution)
        || facets
            .semantic_constants
            .iter()
            .any(|constant| !is_sha256(&constant.value_sha256))
    {
        return Err(if role_kinds.contains(&EffectRoleKindV2::ActiveExecution) {
            EffectLawError::InvalidSemanticConstant
        } else {
            EffectLawError::InvalidFacetContract
        });
    }

    let all_references_exist = facets
        .semantic_constants
        .iter()
        .map(|constant| constant.owner)
        .chain(facets.preconditions.iter().map(|item| item.role))
        .chain(facets.postconditions.iter().map(|item| item.role))
        .chain(facets.preserved_frame.roles.iter().copied())
        .all(|role| role_kinds.contains(&role));
    if !all_references_exist || !effect_contract_matches(facets) {
        return Err(EffectLawError::InvalidFacetContract);
    }
    Ok(())
}

fn effect_contract_matches(facets: &VerifiedSemanticFacetsV2) -> bool {
    let has_postcondition = |condition| {
        facets
            .postconditions
            .iter()
            .any(|item| item.condition == condition)
    };
    match facets.effect_kind {
        EffectKindV2::ContinueActiveExecution => {
            !facets
                .roles
                .iter()
                .any(|role| role.kind == EffectRoleKindV2::InputPayload)
                && has_postcondition(EffectPostconditionKindV2::Continues)
                && !has_postcondition(EffectPostconditionKindV2::InputInjected)
                && !has_postcondition(EffectPostconditionKindV2::Terminated)
        }
        EffectKindV2::InjectInputAndContinue => {
            has_postcondition(EffectPostconditionKindV2::Continues)
                && has_postcondition(EffectPostconditionKindV2::InputInjected)
                && !has_postcondition(EffectPostconditionKindV2::Terminated)
        }
        EffectKindV2::TerminateExecution => {
            has_postcondition(EffectPostconditionKindV2::Terminated)
                && !has_postcondition(EffectPostconditionKindV2::Continues)
                && !has_postcondition(EffectPostconditionKindV2::InputInjected)
        }
    }
}

fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    values.windows(2).any(|pair| pair[0] == pair[1])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectEdgeKind, EffectNodeKind, EffectOperationKind, EffectSource};

    fn topology(handle_type: AtomValueType) -> EffectGraph {
        let nodes = vec![
            EffectNode {
                index: 0,
                source: EffectSource::Observation,
                kind: EffectNodeKind::Scalar,
                value_type: Some(handle_type),
                unique: true,
                operation: None,
            },
            EffectNode {
                index: 1,
                source: EffectSource::Action,
                kind: EffectNodeKind::Scalar,
                value_type: Some(handle_type),
                unique: false,
                operation: None,
            },
            EffectNode {
                index: 2,
                source: EffectSource::Derived,
                kind: EffectNodeKind::Operation,
                value_type: None,
                unique: true,
                operation: Some(EffectOperationKind::Call),
            },
        ];
        let edges = vec![
            EffectEdge {
                from: 0,
                to: 1,
                kind: EffectEdgeKind::CopiedFrom,
            },
            EffectEdge {
                from: 1,
                to: 2,
                kind: EffectEdgeKind::ConsumedBy,
            },
        ];
        let bytes = serde_json::to_vec(&(EFFECT_GRAPH_SCHEMA_V1, &nodes, &edges))
            .expect("test topology serializes");
        EffectGraph {
            schema: EFFECT_GRAPH_SCHEMA_V1.to_owned(),
            nodes,
            edges,
            completeness: EffectGraphCompleteness::Complete,
            canonical_sha256: Some(format!("{:x}", Sha256::digest(bytes))),
            alignment_candidates: 1,
            canonical_permutations: 1,
        }
    }

    fn continuation(preserved: Vec<EffectRoleKindV2>) -> VerifiedSemanticFacetsV2 {
        VerifiedSemanticFacetsV2 {
            effect_kind: EffectKindV2::ContinueActiveExecution,
            roles: vec![EffectRoleV2 {
                node: 0,
                kind: EffectRoleKindV2::ActiveExecution,
                value_type: AtomValueType::Identifier,
            }],
            semantic_constants: Vec::new(),
            preconditions: vec![EffectPredicateV2 {
                role: EffectRoleKindV2::ActiveExecution,
                predicate: EffectPredicateKindV2::Active,
            }],
            postconditions: vec![EffectPostconditionV2 {
                role: EffectRoleKindV2::ActiveExecution,
                condition: EffectPostconditionKindV2::Continues,
            }],
            preserved_frame: PreservedFrameContractV2 { roles: preserved },
        }
    }

    fn inject_input() -> VerifiedSemanticFacetsV2 {
        let mut facets = continuation(Vec::new());
        facets.effect_kind = EffectKindV2::InjectInputAndContinue;
        facets.postconditions.push(EffectPostconditionV2 {
            role: EffectRoleKindV2::ActiveExecution,
            condition: EffectPostconditionKindV2::InputInjected,
        });
        facets
    }

    fn terminate() -> VerifiedSemanticFacetsV2 {
        let mut facets = continuation(Vec::new());
        facets.effect_kind = EffectKindV2::TerminateExecution;
        facets.postconditions = vec![EffectPostconditionV2 {
            role: EffectRoleKindV2::ActiveExecution,
            condition: EffectPostconditionKindV2::Terminated,
        }];
        facets
    }

    fn law(facets: VerifiedSemanticFacetsV2) -> CanonicalEffectLawV2 {
        CanonicalEffectLawV2::from_effect_graph(&topology(AtomValueType::Identifier), facets)
            .expect("test law is valid")
    }

    fn id(law: &CanonicalEffectLawV2) -> EffectLawId {
        law.effect_law_id().expect("test law has an identity")
    }

    fn bytes(law: &CanonicalEffectLawV2) -> Vec<u8> {
        law.canonical_bytes().expect("test law serializes")
    }

    #[test]
    fn wait_and_empty_write_stdin_share_the_effect_identity() {
        let wait = law(continuation(Vec::new()));
        let empty_write_stdin = law(continuation(Vec::new()));
        assert_eq!(id(&wait), id(&empty_write_stdin));
        assert_eq!(bytes(&wait), bytes(&empty_write_stdin));
    }

    #[test]
    fn empty_and_nonempty_input_have_different_effect_identities() {
        assert_ne!(id(&law(continuation(Vec::new()))), id(&law(inject_input())));
    }

    #[test]
    fn continuation_and_termination_have_different_effect_identities() {
        assert_ne!(id(&law(continuation(Vec::new()))), id(&law(terminate())));
    }

    #[test]
    fn transport_and_role_names_cannot_change_the_identity() {
        // Physical transport and role names are deliberately absent from the V2 IR.
        let direct_transport = law(continuation(Vec::new()));
        let wrapped_transport_with_renamed_role = law(continuation(Vec::new()));
        assert_eq!(
            id(&direct_transport),
            id(&wrapped_transport_with_renamed_role)
        );
    }

    #[test]
    fn changed_preserved_frame_changes_the_identity() {
        assert_ne!(
            id(&law(continuation(Vec::new()))),
            id(&law(continuation(vec![EffectRoleKindV2::ActiveExecution])))
        );
    }

    #[test]
    fn incomplete_or_ambiguous_topology_has_no_effect_law_id() {
        for completeness in [
            EffectGraphCompleteness::Ambiguous,
            EffectGraphCompleteness::InsufficientEvidence,
        ] {
            let mut graph = topology(AtomValueType::Identifier);
            graph.completeness = completeness;
            assert_eq!(
                CanonicalEffectLawV2::from_effect_graph(&graph, continuation(Vec::new())),
                Err(EffectLawError::IncompleteTopology)
            );
        }
    }

    #[test]
    fn canonical_restart_serialization_is_byte_identical() {
        let original = law(continuation(vec![EffectRoleKindV2::ActiveExecution]));
        let canonical_bytes = bytes(&original);
        let restored = CanonicalEffectLawV2::from_canonical_bytes(&canonical_bytes)
            .expect("canonical bytes restore");
        assert_eq!(canonical_bytes, bytes(&restored));
        assert_eq!(id(&original), id(&restored));
    }

    #[test]
    fn facet_order_and_digest_case_are_canonical() {
        let mut left = inject_input();
        left.semantic_constants.push(SemanticConstantV2 {
            owner: EffectRoleKindV2::ActiveExecution,
            value_sha256: "AB".repeat(32),
        });
        let mut right = left.clone();
        right.postconditions.reverse();
        right.semantic_constants[0]
            .value_sha256
            .make_ascii_lowercase();
        assert_eq!(id(&law(left)), id(&law(right)));
    }

    #[test]
    fn canonical_fingerprint_is_golden() {
        let effect_law_id = id(&law(continuation(Vec::new())));
        assert_eq!(
            effect_law_id.as_str(),
            "e47682b462e648fde1bcf896ad33fc0deaf780925cce93359468bdcc33448b7b"
        );
    }
}
