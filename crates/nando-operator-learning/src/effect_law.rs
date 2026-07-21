use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nando_operator_kernel::{AtomValueType, RelationAtom};

use crate::{
    EFFECT_GRAPH_SCHEMA_V1, EffectEdgeKind, EffectGraph, EffectGraphBuilder,
    EffectGraphCompleteness, EffectNode, EffectNodeKind, EffectSource, TeacherTransition,
};

pub const CANONICAL_EFFECT_LAW_SCHEMA_V2: &str = "nando.canonical-effect-law.v2";
pub const EFFECT_OBSERVATION_SCHEMA_V2: &str = "nando.evidence-bound-effect-observation.v2";
pub const EFFECT_QUOTIENT_REPORT_SCHEMA_V2: &str = "nando.effect-quotient-report.v2";

pub const EFFECT_OPCODE_EQUAL: u16 = 1;
pub const EFFECT_OPCODE_COPY: u16 = 2;
pub const EFFECT_OPCODE_CONSUME: u16 = 3;
pub const EFFECT_OPCODE_PRODUCE: u16 = 4;
pub const EFFECT_OPCODE_REQUIRE: u16 = 5;
pub const EFFECT_OPCODE_ASSERT_CONSTANT: u16 = 6;
pub const EFFECT_OPCODE_PRESERVE: u16 = 7;
pub const EFFECT_OPCODE_COMPOSE: u16 = 8;

pub const EFFECT_VALUE_OPAQUE_SCALAR: u16 = 1;
pub const EFFECT_VALUE_COLLECTION: u16 = 2;
pub const EFFECT_VALUE_STRING: u16 = 3;
pub const EFFECT_VALUE_INTEGER: u16 = 4;
pub const EFFECT_VALUE_BOOLEAN: u16 = 5;
pub const EFFECT_VALUE_IDENTIFIER: u16 = 6;
pub const EFFECT_VALUE_OPERATION: u16 = 7;

pub const EFFECT_NODE_SCALAR: u16 = 1;
pub const EFFECT_NODE_COLLECTION: u16 = 2;
pub const EFFECT_NODE_OPERATION: u16 = 3;

pub const EFFECT_OPERATION_CALL: u16 = 1;
pub const EFFECT_OPERATION_PROJECT: u16 = 2;
pub const EFFECT_OPERATION_STATUS: u16 = 3;
pub const EFFECT_OPERATION_PLAN_ADVANCE: u16 = 4;

const MAX_EFFECT_LAW_NODES: usize = 32;
const MAX_EFFECT_LAW_EDGES: usize = 256;
const MAX_EFFECT_LAW_ROLES: usize = 32;
const MAX_EFFECT_LAW_CLAUSES: usize = 256;
const MAX_EFFECT_LAW_CONSTANTS: usize = 32;
const MAX_CANONICAL_PERMUTATIONS: usize = 16_384;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EffectOpcodeV2(u16);

impl EffectOpcodeV2 {
    /// The law layer keeps the opcode space open. Runtime capability binding
    /// and admission, not this identity IR, decide whether an opcode executes.
    pub fn new(value: u16) -> Result<Self, EffectLawError> {
        (value != 0)
            .then_some(Self(value))
            .ok_or(EffectLawError::InvalidProgram)
    }

    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EffectValueTypeV2(u16);

impl EffectValueTypeV2 {
    pub fn new(value: u16) -> Result<Self, EffectLawError> {
        (value != 0)
            .then_some(Self(value))
            .ok_or(EffectLawError::InvalidRole)
    }

    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RoleRefV2(u16);

impl RoleRefV2 {
    #[must_use]
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RoleCardinalityV2 {
    pub min: u16,
    pub max: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectRoleV2 {
    pub role_id: RoleRefV2,
    pub node: u16,
    pub value_type: EffectValueTypeV2,
    pub cardinality: RoleCardinalityV2,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TypedConstantCommitmentV2 {
    pub value_type: EffectValueTypeV2,
    pub value_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectClauseV2 {
    pub opcode: EffectOpcodeV2,
    pub lhs: RoleRefV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rhs: Option<RoleRefV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constant: Option<TypedConstantCommitmentV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub argument_key_sha256: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreservedFrameContractV2 {
    pub roles: Vec<RoleRefV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectLawProgramV2 {
    pub roles: Vec<EffectRoleV2>,
    pub clauses: Vec<EffectClauseV2>,
    pub preserved_frame: PreservedFrameContractV2,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectLawDictionaryRootsV2 {
    pub opcode_dictionary_root: String,
    pub value_type_dictionary_root: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectTopologyNodeV2 {
    pub index: u16,
    pub source: EffectSource,
    pub node_kind_code: u16,
    pub value_type: EffectValueTypeV2,
    pub unique: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_code: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectTopologyEdgeV2 {
    pub from: u16,
    pub to: u16,
    pub opcode: EffectOpcodeV2,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectTopologyV2 {
    schema: String,
    nodes: Vec<EffectTopologyNodeV2>,
    edges: Vec<EffectTopologyEdgeV2>,
    canonical_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectLawV2 {
    schema: String,
    ir_version: u16,
    dictionary_roots: EffectLawDictionaryRootsV2,
    topology: CanonicalEffectTopologyV2,
    roles: Vec<EffectRoleV2>,
    clauses: Vec<EffectClauseV2>,
    preserved_frame: PreservedFrameContractV2,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EffectLawId(String);

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct PhysicalEffectArgumentV2 {
    pub argument_key_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_slot: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_node: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<EffectValueTypeV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constant: Option<TypedConstantCommitmentV2>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvidenceBoundEffectObservationV2 {
    pub schema: String,
    pub observation_sha256: String,
    pub transition_sha256: String,
    pub lineage_sha256: String,
    pub verifier_evidence_ref_sha256: String,
    pub runtime_parity_evidence_ref_sha256: String,
    pub runtime_parity_case_sha256: String,
    pub physical_graph: EffectGraph,
    pub arguments: Vec<PhysicalEffectArgumentV2>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalizedEffectLawV2 {
    law: CanonicalEffectLawV2,
    node_mapping: Vec<CanonicalNodeMappingEntryV2>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalNodeMappingEntryV2 {
    pub physical_node: u16,
    pub canonical_node: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObservationNodeMappingV2 {
    pub observation_sha256: String,
    pub nodes: Vec<CanonicalNodeMappingEntryV2>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProtocolModeDifferenceV2 {
    pub left_observation_sha256: String,
    pub right_observation_sha256: String,
    pub left_physical_topology_sha256: String,
    pub right_physical_topology_sha256: String,
    pub left_arguments_sha256: String,
    pub right_arguments_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawQuotientReportV2 {
    pub schema: String,
    pub observation_count: usize,
    pub independent_lineages: usize,
    pub observation_root_sha256: String,
    pub candidate: Option<CanonicalizedEffectLawV2>,
    pub observation_node_mappings: Vec<ObservationNodeMappingV2>,
    pub protocol_mode_candidates: Vec<ProtocolModeDifferenceV2>,
    pub blocker: Option<String>,
}

#[derive(Deserialize)]
struct CanonicalEffectTopologyWireV2 {
    schema: String,
    nodes: Vec<EffectTopologyNodeV2>,
    edges: Vec<EffectTopologyEdgeV2>,
    canonical_sha256: String,
}

#[derive(Deserialize)]
struct CanonicalEffectLawWireV2 {
    schema: String,
    ir_version: u16,
    dictionary_roots: EffectLawDictionaryRootsV2,
    topology: CanonicalEffectTopologyWireV2,
    roles: Vec<EffectRoleV2>,
    clauses: Vec<EffectClauseV2>,
    preserved_frame: PreservedFrameContractV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectLawError {
    IncompleteTopology,
    InvalidTopology,
    OverBudget,
    InvalidRole,
    InvalidConstant,
    InvalidProgram,
    InvalidEvidence,
    InsufficientIndependentEvidence,
    Serialization,
}

impl fmt::Display for EffectLawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::IncompleteTopology => "effect topology is not complete",
            Self::InvalidTopology => "effect topology is not canonical",
            Self::OverBudget => "effect law exceeds a bounded IR limit",
            Self::InvalidRole => "effect role does not reference a compatible canonical node",
            Self::InvalidConstant => "effect constant is not a SHA-256 commitment",
            Self::InvalidProgram => "effect relation program is invalid",
            Self::InvalidEvidence => "effect law evidence is not independently verified",
            Self::InsufficientIndependentEvidence => {
                "effect quotient requires multiple independent observations"
            }
            Self::Serialization => "canonical effect law serialization failed",
        })
    }
}

impl std::error::Error for EffectLawError {}

impl EffectLawId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl EffectLawDictionaryRootsV2 {
    pub fn new(
        opcode_dictionary_root: String,
        value_type_dictionary_root: String,
    ) -> Result<Self, EffectLawError> {
        if !is_sha256(&opcode_dictionary_root) || !is_sha256(&value_type_dictionary_root) {
            return Err(EffectLawError::InvalidProgram);
        }
        Ok(Self {
            opcode_dictionary_root: opcode_dictionary_root.to_ascii_lowercase(),
            value_type_dictionary_root: value_type_dictionary_root.to_ascii_lowercase(),
        })
    }

    pub fn builtin_v2() -> Result<Self, EffectLawError> {
        Self::new(
            sha256_json(&(
                "relation_opcodes",
                [
                    EFFECT_OPCODE_EQUAL,
                    EFFECT_OPCODE_COPY,
                    EFFECT_OPCODE_CONSUME,
                    EFFECT_OPCODE_PRODUCE,
                    EFFECT_OPCODE_REQUIRE,
                    EFFECT_OPCODE_ASSERT_CONSTANT,
                    EFFECT_OPCODE_PRESERVE,
                    EFFECT_OPCODE_COMPOSE,
                ],
                "physical_operation_classes",
                [
                    EFFECT_OPERATION_CALL,
                    EFFECT_OPERATION_PROJECT,
                    EFFECT_OPERATION_STATUS,
                    EFFECT_OPERATION_PLAN_ADVANCE,
                ],
            ))?,
            sha256_json(&[
                EFFECT_VALUE_OPAQUE_SCALAR,
                EFFECT_VALUE_COLLECTION,
                EFFECT_VALUE_STRING,
                EFFECT_VALUE_INTEGER,
                EFFECT_VALUE_BOOLEAN,
                EFFECT_VALUE_IDENTIFIER,
                EFFECT_VALUE_OPERATION,
            ])?,
        )
    }
}

impl CanonicalEffectLawV2 {
    pub fn from_unverified_program(
        graph: &EffectGraph,
        dictionary_roots: EffectLawDictionaryRootsV2,
        mut program: EffectLawProgramV2,
    ) -> Result<CanonicalizedEffectLawV2, EffectLawError> {
        let (topology, node_mapping) = canonical_topology(graph)?;
        remap_program_nodes(&node_mapping, &mut program)?;
        let law = Self::from_topology_and_program(topology, dictionary_roots, program)?;
        Ok(CanonicalizedEffectLawV2 { law, node_mapping })
    }

    fn from_topology_and_program(
        topology: CanonicalEffectTopologyV2,
        dictionary_roots: EffectLawDictionaryRootsV2,
        mut program: EffectLawProgramV2,
    ) -> Result<Self, EffectLawError> {
        validate_dictionary_roots(&dictionary_roots)?;
        canonicalize_program(&topology, &mut program)?;
        Ok(Self {
            schema: CANONICAL_EFFECT_LAW_SCHEMA_V2.to_owned(),
            ir_version: 2,
            dictionary_roots,
            topology,
            roles: program.roles,
            clauses: program.clauses,
            preserved_frame: program.preserved_frame,
        })
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawError> {
        let wire: CanonicalEffectLawWireV2 =
            serde_json::from_slice(bytes).map_err(|_| EffectLawError::Serialization)?;
        if wire.schema != CANONICAL_EFFECT_LAW_SCHEMA_V2 || wire.ir_version != 2 {
            return Err(EffectLawError::InvalidProgram);
        }
        let topology = topology_from_wire(wire.topology)?;
        let canonical = Self::from_topology_and_program(
            topology,
            wire.dictionary_roots,
            EffectLawProgramV2 {
                roles: wire.roles,
                clauses: wire.clauses,
                preserved_frame: wire.preserved_frame,
            },
        )?;
        if canonical.canonical_bytes()? != bytes {
            return Err(EffectLawError::InvalidProgram);
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
    pub fn roles(&self) -> &[EffectRoleV2] {
        &self.roles
    }

    #[must_use]
    pub fn clauses(&self) -> &[EffectClauseV2] {
        &self.clauses
    }

    #[must_use]
    pub fn preserved_frame(&self) -> &PreservedFrameContractV2 {
        &self.preserved_frame
    }

    #[must_use]
    pub fn dictionary_roots(&self) -> &EffectLawDictionaryRootsV2 {
        &self.dictionary_roots
    }
}

impl CanonicalEffectTopologyV2 {
    #[must_use]
    pub fn nodes(&self) -> &[EffectTopologyNodeV2] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[EffectTopologyEdgeV2] {
        &self.edges
    }

    #[must_use]
    pub fn canonical_sha256(&self) -> &str {
        &self.canonical_sha256
    }
}

impl CanonicalizedEffectLawV2 {
    #[must_use]
    pub fn law(&self) -> &CanonicalEffectLawV2 {
        &self.law
    }

    #[must_use]
    pub fn node_mapping(&self) -> &[CanonicalNodeMappingEntryV2] {
        &self.node_mapping
    }
}

pub fn observe_effect_transition_v2(
    transition: &TeacherTransition,
) -> Result<EvidenceBoundEffectObservationV2, EffectLawError> {
    if !transition.outcome.verifier.accepted
        || !is_sha256(&transition.before.frame_id_sha256)
        || !is_sha256(&transition.outcome.action.signature_sha256)
        || !is_sha256(&transition.outcome.verifier.evidence_ref_sha256)
        || !is_sha256(&transition.outcome.verifier.output_digest_sha256)
    {
        return Err(EffectLawError::InvalidEvidence);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(EffectLawError::InvalidEvidence)?;
    if !is_sha256(&parity.evidence_ref_sha256) {
        return Err(EffectLawError::InvalidEvidence);
    }
    let physical_graph = EffectGraphBuilder::default().build(transition);
    physical_graph
        .canonical_sha256
        .as_ref()
        .ok_or(EffectLawError::IncompleteTopology)?;
    let transition_sha256 = sha256_json(transition)?;
    let runtime_parity_case_sha256 = sha256_json(parity)?;
    let arguments = physical_arguments(transition, &physical_graph)?;
    let mut observation = EvidenceBoundEffectObservationV2 {
        schema: EFFECT_OBSERVATION_SCHEMA_V2.to_owned(),
        observation_sha256: String::new(),
        transition_sha256,
        lineage_sha256: transition.before.session_id_sha256.clone(),
        verifier_evidence_ref_sha256: transition.outcome.verifier.evidence_ref_sha256.clone(),
        runtime_parity_evidence_ref_sha256: parity.evidence_ref_sha256.clone(),
        runtime_parity_case_sha256,
        physical_graph,
        arguments,
    };
    observation.observation_sha256 = observation_digest(&observation)?;
    Ok(observation)
}

pub fn search_effect_law_quotient_v2(
    observations: &[EvidenceBoundEffectObservationV2],
    dictionary_roots: EffectLawDictionaryRootsV2,
) -> Result<EffectLawQuotientReportV2, EffectLawError> {
    if observations.len() < 2 {
        return Err(EffectLawError::InsufficientIndependentEvidence);
    }
    validate_dictionary_roots(&dictionary_roots)?;
    for observation in observations {
        validate_observation(observation)?;
    }
    let independent_lineages = observations
        .iter()
        .map(|item| item.lineage_sha256.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    if independent_lineages < 2 {
        return Err(EffectLawError::InsufficientIndependentEvidence);
    }

    let mut observation_ids = observations
        .iter()
        .map(|item| item.observation_sha256.clone())
        .collect::<Vec<_>>();
    observation_ids.sort();
    observation_ids.dedup();
    if observation_ids.len() != observations.len() {
        return Err(EffectLawError::InsufficientIndependentEvidence);
    }
    let observation_root_sha256 = sha256_json(&observation_ids)?;

    let mut candidates = Vec::with_capacity(observations.len());
    let mut observation_node_mappings = Vec::with_capacity(observations.len());
    for observation in observations {
        let (topology, node_mapping) = canonical_topology(&observation.physical_graph)?;
        let program = derive_relation_program(observation, &topology, &node_mapping)?;
        let law = CanonicalEffectLawV2::from_topology_and_program(
            topology,
            dictionary_roots.clone(),
            program,
        )?;
        observation_node_mappings.push(ObservationNodeMappingV2 {
            observation_sha256: observation.observation_sha256.clone(),
            nodes: node_mapping.clone(),
        });
        candidates.push(CanonicalizedEffectLawV2 { law, node_mapping });
    }

    let first_bytes = candidates[0].law.canonical_bytes()?;
    let all_equal = candidates.iter().skip(1).all(|candidate| {
        candidate
            .law
            .canonical_bytes()
            .is_ok_and(|bytes| bytes == first_bytes)
    });
    let mut protocol_mode_candidates = Vec::new();
    if !all_equal {
        let left = &observations[0];
        for (index, right) in observations.iter().enumerate().skip(1) {
            if candidates[0].law != candidates[index].law {
                protocol_mode_candidates.push(protocol_difference(left, right)?);
            }
        }
    }
    Ok(EffectLawQuotientReportV2 {
        schema: EFFECT_QUOTIENT_REPORT_SCHEMA_V2.to_owned(),
        observation_count: observations.len(),
        independent_lineages,
        observation_root_sha256,
        candidate: all_equal.then(|| candidates.remove(0)),
        observation_node_mappings,
        protocol_mode_candidates,
        blocker: (!all_equal).then(|| "no_invariant_exact_quotient".to_owned()),
    })
}

fn topology_from_wire(
    wire: CanonicalEffectTopologyWireV2,
) -> Result<CanonicalEffectTopologyV2, EffectLawError> {
    if wire.schema != CANONICAL_EFFECT_LAW_SCHEMA_V2 {
        return Err(EffectLawError::InvalidTopology);
    }
    let mut nodes = wire.nodes;
    let mut edges = wire.edges;
    nodes.sort();
    edges.sort();
    if nodes.is_empty()
        || nodes.len() > MAX_EFFECT_LAW_NODES
        || edges.len() > MAX_EFFECT_LAW_EDGES
        || has_duplicates(&nodes)
        || has_duplicates(&edges)
        || nodes
            .iter()
            .enumerate()
            .any(|(index, node)| usize::from(node.index) != index)
        || edges.iter().any(|edge| {
            usize::from(edge.from) >= nodes.len() || usize::from(edge.to) >= nodes.len()
        })
        || nodes.iter().any(|node| {
            (node.node_kind_code == EFFECT_NODE_OPERATION)
                != (node.operation_code.is_some()
                    && node.value_type.get() == EFFECT_VALUE_OPERATION)
        })
    {
        return Err(EffectLawError::InvalidTopology);
    }
    let canonical_sha256 = topology_digest(&nodes, &edges)?;
    if wire.canonical_sha256 != canonical_sha256 {
        return Err(EffectLawError::InvalidTopology);
    }
    Ok(CanonicalEffectTopologyV2 {
        schema: CANONICAL_EFFECT_LAW_SCHEMA_V2.to_owned(),
        nodes,
        edges,
        canonical_sha256,
    })
}

#[derive(Clone)]
struct DraftTopologyNode {
    physical_node: u16,
    source: EffectSource,
    node_kind_code: u16,
    value_type: EffectValueTypeV2,
    unique: bool,
    operation_code: Option<u16>,
}

type CanonicalTopologyCandidate = (
    Vec<u8>,
    Vec<EffectTopologyNodeV2>,
    Vec<EffectTopologyEdgeV2>,
    Vec<CanonicalNodeMappingEntryV2>,
);

impl DraftTopologyNode {
    fn color(&self) -> (EffectSource, u16, EffectValueTypeV2, bool, Option<u16>) {
        (
            self.source,
            self.node_kind_code,
            self.value_type,
            self.unique,
            self.operation_code,
        )
    }
}

fn canonical_topology(
    graph: &EffectGraph,
) -> Result<(CanonicalEffectTopologyV2, Vec<CanonicalNodeMappingEntryV2>), EffectLawError> {
    if graph.completeness != EffectGraphCompleteness::Complete {
        return Err(EffectLawError::IncompleteTopology);
    }
    if graph.schema != EFFECT_GRAPH_SCHEMA_V1
        || graph.nodes.is_empty()
        || graph.nodes.len() > MAX_EFFECT_LAW_NODES
        || graph.edges.len() > MAX_EFFECT_LAW_EDGES
        || !physical_topology_digest_matches(graph)?
    {
        return Err(EffectLawError::InvalidTopology);
    }

    let mut old_to_draft = BTreeMap::new();
    let mut draft_nodes = Vec::new();
    for node in &graph.nodes {
        let (node_kind_code, value_type, operation_code) = exact_node_fields(node)?;
        old_to_draft.insert(node.index, draft_nodes.len());
        draft_nodes.push(DraftTopologyNode {
            physical_node: node.index,
            source: node.source,
            node_kind_code,
            value_type,
            unique: node.unique,
            operation_code,
        });
    }
    if draft_nodes.is_empty() {
        return Err(EffectLawError::IncompleteTopology);
    }

    let mut draft_edges = BTreeSet::new();
    for edge in &graph.edges {
        let from = old_to_draft
            .get(&edge.from)
            .copied()
            .ok_or(EffectLawError::InvalidTopology)?;
        let to = old_to_draft
            .get(&edge.to)
            .copied()
            .ok_or(EffectLawError::InvalidTopology)?;
        draft_edges.insert((from, to, edge_opcode(edge.kind)?));
    }
    canonicalize_normalized_topology(&draft_nodes, &draft_edges)
}

fn physical_topology_digest_matches(graph: &EffectGraph) -> Result<bool, EffectLawError> {
    let bytes = serde_json::to_vec(&(EFFECT_GRAPH_SCHEMA_V1, &graph.nodes, &graph.edges))
        .map_err(|_| EffectLawError::Serialization)?;
    let digest = format!("{:x}", Sha256::digest(bytes));
    Ok(graph.canonical_sha256.as_deref() == Some(digest.as_str()))
}

fn exact_node_fields(
    node: &EffectNode,
) -> Result<(u16, EffectValueTypeV2, Option<u16>), EffectLawError> {
    match node.kind {
        EffectNodeKind::Scalar => Ok((
            EFFECT_NODE_SCALAR,
            exact_value_type(node.value_type.ok_or(EffectLawError::InvalidTopology)?)?,
            None,
        )),
        EffectNodeKind::Collection => Ok((
            EFFECT_NODE_COLLECTION,
            exact_value_type(node.value_type.ok_or(EffectLawError::InvalidTopology)?)?,
            None,
        )),
        EffectNodeKind::Operation => Ok((
            EFFECT_NODE_OPERATION,
            EffectValueTypeV2::new(EFFECT_VALUE_OPERATION)?,
            Some(
                match node.operation.ok_or(EffectLawError::InvalidTopology)? {
                    crate::EffectOperationKind::Call => EFFECT_OPERATION_CALL,
                    crate::EffectOperationKind::Project => EFFECT_OPERATION_PROJECT,
                    crate::EffectOperationKind::Status => EFFECT_OPERATION_STATUS,
                    crate::EffectOperationKind::PlanAdvance => EFFECT_OPERATION_PLAN_ADVANCE,
                },
            ),
        )),
    }
}

fn exact_value_type(value: AtomValueType) -> Result<EffectValueTypeV2, EffectLawError> {
    EffectValueTypeV2::new(match value {
        AtomValueType::String => EFFECT_VALUE_STRING,
        AtomValueType::Integer => EFFECT_VALUE_INTEGER,
        AtomValueType::Boolean => EFFECT_VALUE_BOOLEAN,
        AtomValueType::Identifier => EFFECT_VALUE_IDENTIFIER,
        AtomValueType::Collection => EFFECT_VALUE_COLLECTION,
    })
}

fn edge_opcode(kind: EffectEdgeKind) -> Result<EffectOpcodeV2, EffectLawError> {
    EffectOpcodeV2::new(match kind {
        EffectEdgeKind::Equal => EFFECT_OPCODE_EQUAL,
        EffectEdgeKind::CopiedFrom => EFFECT_OPCODE_COPY,
        EffectEdgeKind::ConsumedBy => EFFECT_OPCODE_CONSUME,
        EffectEdgeKind::Produces => EFFECT_OPCODE_PRODUCE,
    })
}

fn canonicalize_normalized_topology(
    nodes: &[DraftTopologyNode],
    edges: &BTreeSet<(usize, usize, EffectOpcodeV2)>,
) -> Result<(CanonicalEffectTopologyV2, Vec<CanonicalNodeMappingEntryV2>), EffectLawError> {
    let mut groups = BTreeMap::<_, Vec<usize>>::new();
    for (index, node) in nodes.iter().enumerate() {
        groups.entry(node.color()).or_default().push(index);
    }
    let groups = groups.into_values().collect::<Vec<_>>();
    let permutation_count = groups.iter().try_fold(1_usize, |count, group| {
        count.checked_mul(factorial(group.len())?)
    });
    if permutation_count.is_none_or(|count| count > MAX_CANONICAL_PERMUTATIONS) {
        return Err(EffectLawError::OverBudget);
    }

    let mut ordered = Vec::with_capacity(nodes.len());
    let mut best = None::<CanonicalTopologyCandidate>;
    enumerate_topology_groups(nodes, edges, &groups, 0, &mut ordered, &mut best)?;
    let (_, nodes, edges, node_mapping) = best.ok_or(EffectLawError::InvalidTopology)?;
    let canonical_sha256 = topology_digest(&nodes, &edges)?;
    Ok((
        CanonicalEffectTopologyV2 {
            schema: CANONICAL_EFFECT_LAW_SCHEMA_V2.to_owned(),
            nodes,
            edges,
            canonical_sha256,
        },
        node_mapping,
    ))
}

fn enumerate_topology_groups(
    nodes: &[DraftTopologyNode],
    edges: &BTreeSet<(usize, usize, EffectOpcodeV2)>,
    groups: &[Vec<usize>],
    group_index: usize,
    ordered: &mut Vec<usize>,
    best: &mut Option<CanonicalTopologyCandidate>,
) -> Result<(), EffectLawError> {
    if group_index < groups.len() {
        let mut group = groups[group_index].clone();
        return enumerate_permutations(&mut group, 0, &mut |permutation| {
            ordered.extend_from_slice(permutation);
            let result =
                enumerate_topology_groups(nodes, edges, groups, group_index + 1, ordered, best);
            ordered.truncate(ordered.len().saturating_sub(permutation.len()));
            result
        });
    }

    let mut old_to_new = vec![0_u16; nodes.len()];
    for (new, old) in ordered.iter().enumerate() {
        old_to_new[*old] = u16::try_from(new).map_err(|_| EffectLawError::OverBudget)?;
    }
    let canonical_nodes = ordered
        .iter()
        .enumerate()
        .map(|(new, old)| {
            Ok(EffectTopologyNodeV2 {
                index: u16::try_from(new).map_err(|_| EffectLawError::OverBudget)?,
                source: nodes[*old].source,
                node_kind_code: nodes[*old].node_kind_code,
                value_type: nodes[*old].value_type,
                unique: nodes[*old].unique,
                operation_code: nodes[*old].operation_code,
            })
        })
        .collect::<Result<Vec<_>, EffectLawError>>()?;
    let mut canonical_edges = edges
        .iter()
        .map(|(from, to, opcode)| EffectTopologyEdgeV2 {
            from: old_to_new[*from],
            to: old_to_new[*to],
            opcode: *opcode,
        })
        .collect::<Vec<_>>();
    canonical_edges.sort();
    let bytes = serde_json::to_vec(&(
        CANONICAL_EFFECT_LAW_SCHEMA_V2,
        &canonical_nodes,
        &canonical_edges,
    ))
    .map_err(|_| EffectLawError::Serialization)?;
    if best
        .as_ref()
        .is_none_or(|(current, _, _, _)| bytes < *current)
    {
        let mut node_mapping = nodes
            .iter()
            .enumerate()
            .map(|(old, node)| CanonicalNodeMappingEntryV2 {
                physical_node: node.physical_node,
                canonical_node: old_to_new[old],
            })
            .collect::<Vec<_>>();
        node_mapping.sort();
        *best = Some((bytes, canonical_nodes, canonical_edges, node_mapping));
    }
    Ok(())
}

fn enumerate_permutations(
    values: &mut [usize],
    index: usize,
    visit: &mut impl FnMut(&[usize]) -> Result<(), EffectLawError>,
) -> Result<(), EffectLawError> {
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

fn topology_digest(
    nodes: &[EffectTopologyNodeV2],
    edges: &[EffectTopologyEdgeV2],
) -> Result<String, EffectLawError> {
    let bytes = serde_json::to_vec(&(CANONICAL_EFFECT_LAW_SCHEMA_V2, nodes, edges))
        .map_err(|_| EffectLawError::Serialization)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn derive_relation_program(
    observation: &EvidenceBoundEffectObservationV2,
    topology: &CanonicalEffectTopologyV2,
    node_mapping: &[CanonicalNodeMappingEntryV2],
) -> Result<EffectLawProgramV2, EffectLawError> {
    let roles = topology
        .nodes
        .iter()
        .map(|node| EffectRoleV2 {
            role_id: RoleRefV2::new(node.index),
            node: node.index,
            value_type: node.value_type,
            cardinality: RoleCardinalityV2 {
                min: 1,
                max: if node.unique { 1 } else { u16::MAX },
            },
        })
        .collect::<Vec<_>>();
    let mut clauses = topology
        .edges
        .iter()
        .map(|edge| EffectClauseV2 {
            opcode: edge.opcode,
            lhs: RoleRefV2::new(edge.from),
            rhs: Some(RoleRefV2::new(edge.to)),
            constant: None,
            argument_key_sha256: None,
        })
        .collect::<Vec<_>>();
    let require_opcode = EffectOpcodeV2::new(EFFECT_OPCODE_REQUIRE)?;
    clauses.extend(roles.iter().map(|role| EffectClauseV2 {
        opcode: require_opcode,
        lhs: role.role_id,
        rhs: None,
        constant: None,
        argument_key_sha256: None,
    }));

    let constant_owner = topology
        .nodes
        .iter()
        .find(|node| node.operation_code == Some(EFFECT_OPERATION_CALL))
        .map(|node| RoleRefV2::new(node.index))
        .ok_or(EffectLawError::InvalidProgram);
    let constant_opcode = EffectOpcodeV2::new(EFFECT_OPCODE_ASSERT_CONSTANT)?;
    for argument in &observation.arguments {
        if let Some(physical_node) = argument.physical_node {
            let canonical_node = mapped_node(node_mapping, physical_node)?;
            clauses.push(EffectClauseV2 {
                opcode: require_opcode,
                lhs: RoleRefV2::new(canonical_node),
                rhs: None,
                constant: None,
                argument_key_sha256: Some(argument.argument_key_sha256.clone()),
            });
        }
        if let Some(constant) = argument.constant.clone() {
            clauses.push(EffectClauseV2 {
                opcode: constant_opcode,
                lhs: constant_owner?,
                rhs: None,
                constant: Some(constant),
                argument_key_sha256: Some(argument.argument_key_sha256.clone()),
            });
        }
    }

    let preserved_frame = PreservedFrameContractV2 {
        roles: topology
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    node.source,
                    EffectSource::Request | EffectSource::Observation
                )
            })
            .map(|node| RoleRefV2::new(node.index))
            .collect(),
    };
    Ok(EffectLawProgramV2 {
        roles,
        clauses,
        preserved_frame,
    })
}

fn physical_arguments(
    transition: &TeacherTransition,
    graph: &EffectGraph,
) -> Result<Vec<PhysicalEffectArgumentV2>, EffectLawError> {
    let mut arguments = Vec::new();
    for atom in &transition.outcome.action.atoms {
        let argument = match atom {
            RelationAtom::ActionRoleArgument {
                name,
                slot_id,
                value_type,
            } => Some(PhysicalEffectArgumentV2 {
                argument_key_sha256: argument_key(name),
                role_slot: Some(*slot_id),
                physical_node: Some(physical_node_for_action_slot(
                    transition,
                    graph,
                    *slot_id,
                    *value_type,
                )?),
                value_type: value_type.map(exact_value_type).transpose()?,
                constant: None,
            }),
            RelationAtom::ActionIntegerArgument { name, value } => Some(PhysicalEffectArgumentV2 {
                argument_key_sha256: argument_key(name),
                role_slot: None,
                physical_node: None,
                value_type: Some(EffectValueTypeV2::new(EFFECT_VALUE_INTEGER)?),
                constant: Some(typed_constant(EFFECT_VALUE_INTEGER, value)?),
            }),
            RelationAtom::ActionStringArgument { name, value } => Some(PhysicalEffectArgumentV2 {
                argument_key_sha256: argument_key(name),
                role_slot: None,
                physical_node: None,
                value_type: Some(EffectValueTypeV2::new(EFFECT_VALUE_STRING)?),
                constant: Some(typed_constant(EFFECT_VALUE_STRING, value)?),
            }),
            RelationAtom::ActionBooleanArgument { name, value } => Some(PhysicalEffectArgumentV2 {
                argument_key_sha256: argument_key(name),
                role_slot: None,
                physical_node: None,
                value_type: Some(EffectValueTypeV2::new(EFFECT_VALUE_BOOLEAN)?),
                constant: Some(typed_constant(EFFECT_VALUE_BOOLEAN, value)?),
            }),
            _ => None,
        };
        if let Some(argument) = argument {
            arguments.push(argument);
        }
    }
    arguments.sort();
    arguments.dedup();
    Ok(arguments)
}

fn physical_node_for_action_slot(
    transition: &TeacherTransition,
    graph: &EffectGraph,
    slot_id: u16,
    declared_type: Option<AtomValueType>,
) -> Result<u16, EffectLawError> {
    let observed_type = transition
        .outcome
        .action
        .atoms
        .iter()
        .find_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id: candidate,
                value_type,
                source: crate::AtomSource::Action,
                ..
            } if *candidate == slot_id => Some(*value_type),
            _ => None,
        })
        .ok_or(EffectLawError::InvalidEvidence)?;
    if declared_type.is_some_and(|value| value != observed_type) {
        return Err(EffectLawError::InvalidEvidence);
    }
    let candidates = graph
        .nodes
        .iter()
        .filter(|node| {
            node.source == EffectSource::Action
                && node.value_type == Some(observed_type)
                && node.kind
                    == if observed_type == AtomValueType::Collection {
                        EffectNodeKind::Collection
                    } else {
                        EffectNodeKind::Scalar
                    }
        })
        .map(|node| node.index)
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [physical_node] => Ok(*physical_node),
        _ => Err(EffectLawError::InvalidEvidence),
    }
}

fn argument_key(name: &str) -> String {
    format!("{:x}", Sha256::digest(name.as_bytes()))
}

fn typed_constant<T: Serialize>(
    value_type: u16,
    value: &T,
) -> Result<TypedConstantCommitmentV2, EffectLawError> {
    let value_type = EffectValueTypeV2::new(value_type)?;
    let bytes = serde_json::to_vec(&(CANONICAL_EFFECT_LAW_SCHEMA_V2, value_type, value))
        .map_err(|_| EffectLawError::Serialization)?;
    Ok(TypedConstantCommitmentV2 {
        value_type,
        value_sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

fn validate_dictionary_roots(roots: &EffectLawDictionaryRootsV2) -> Result<(), EffectLawError> {
    if !is_sha256(&roots.opcode_dictionary_root)
        || !is_sha256(&roots.value_type_dictionary_root)
        || roots.opcode_dictionary_root != roots.opcode_dictionary_root.to_ascii_lowercase()
        || roots.value_type_dictionary_root != roots.value_type_dictionary_root.to_ascii_lowercase()
    {
        return Err(EffectLawError::InvalidProgram);
    }
    Ok(())
}

fn validate_observation(
    observation: &EvidenceBoundEffectObservationV2,
) -> Result<(), EffectLawError> {
    if observation.schema != EFFECT_OBSERVATION_SCHEMA_V2
        || !is_sha256(&observation.transition_sha256)
        || !is_sha256(&observation.lineage_sha256)
        || !is_sha256(&observation.verifier_evidence_ref_sha256)
        || !is_sha256(&observation.runtime_parity_evidence_ref_sha256)
        || !is_sha256(&observation.runtime_parity_case_sha256)
        || !physical_topology_digest_matches(&observation.physical_graph)?
        || observation.arguments.iter().any(|argument| {
            !is_sha256(&argument.argument_key_sha256)
                || argument
                    .constant
                    .as_ref()
                    .is_some_and(|constant| !is_sha256(&constant.value_sha256))
                || argument.physical_node.is_some_and(|physical_node| {
                    !observation
                        .physical_graph
                        .nodes
                        .iter()
                        .any(|node| node.index == physical_node)
                })
        })
        || observation.observation_sha256 != observation_digest(observation)?
    {
        return Err(EffectLawError::InvalidEvidence);
    }
    Ok(())
}

fn observation_digest(
    observation: &EvidenceBoundEffectObservationV2,
) -> Result<String, EffectLawError> {
    sha256_json(&(
        observation.schema.as_str(),
        observation.transition_sha256.as_str(),
        observation.lineage_sha256.as_str(),
        observation.verifier_evidence_ref_sha256.as_str(),
        observation.runtime_parity_evidence_ref_sha256.as_str(),
        observation.runtime_parity_case_sha256.as_str(),
        &observation.physical_graph,
        &observation.arguments,
    ))
}

fn remap_program_nodes(
    mapping: &[CanonicalNodeMappingEntryV2],
    program: &mut EffectLawProgramV2,
) -> Result<(), EffectLawError> {
    for role in &mut program.roles {
        role.node = mapped_node(mapping, role.node)?;
    }
    Ok(())
}

fn mapped_node(
    mapping: &[CanonicalNodeMappingEntryV2],
    physical_node: u16,
) -> Result<u16, EffectLawError> {
    mapping
        .iter()
        .find(|item| item.physical_node == physical_node)
        .map(|item| item.canonical_node)
        .ok_or(EffectLawError::InvalidRole)
}

fn protocol_difference(
    left: &EvidenceBoundEffectObservationV2,
    right: &EvidenceBoundEffectObservationV2,
) -> Result<ProtocolModeDifferenceV2, EffectLawError> {
    Ok(ProtocolModeDifferenceV2 {
        left_observation_sha256: left.observation_sha256.clone(),
        right_observation_sha256: right.observation_sha256.clone(),
        left_physical_topology_sha256: left
            .physical_graph
            .canonical_sha256
            .clone()
            .ok_or(EffectLawError::InvalidTopology)?,
        right_physical_topology_sha256: right
            .physical_graph
            .canonical_sha256
            .clone()
            .ok_or(EffectLawError::InvalidTopology)?,
        left_arguments_sha256: sha256_json(&left.arguments)?,
        right_arguments_sha256: sha256_json(&right.arguments)?,
    })
}

fn canonicalize_program(
    topology: &CanonicalEffectTopologyV2,
    program: &mut EffectLawProgramV2,
) -> Result<(), EffectLawError> {
    if program.roles.is_empty()
        || program.roles.len() > MAX_EFFECT_LAW_ROLES
        || program.clauses.is_empty()
        || program.clauses.len() > MAX_EFFECT_LAW_CLAUSES
        || program
            .clauses
            .iter()
            .filter(|clause| clause.constant.is_some())
            .count()
            > MAX_EFFECT_LAW_CONSTANTS
    {
        return Err(EffectLawError::OverBudget);
    }

    let mut old_role_ids = BTreeSet::new();
    let mut role_nodes = BTreeSet::new();
    for role in &program.roles {
        let Some(node) = topology.nodes.get(usize::from(role.node)) else {
            return Err(EffectLawError::InvalidRole);
        };
        if node.index != role.node
            || node.value_type != role.value_type
            || role.cardinality.min == 0
            || role.cardinality.max < role.cardinality.min
            || !old_role_ids.insert(role.role_id)
            || !role_nodes.insert(role.node)
        {
            return Err(EffectLawError::InvalidRole);
        }
    }

    program.roles.sort_by_key(|role| {
        (
            role.node,
            role.value_type,
            role.cardinality.min,
            role.cardinality.max,
        )
    });
    let mut role_map = BTreeMap::new();
    for (index, role) in program.roles.iter_mut().enumerate() {
        let canonical =
            RoleRefV2::new(u16::try_from(index).map_err(|_| EffectLawError::OverBudget)?);
        role_map.insert(role.role_id, canonical);
        role.role_id = canonical;
    }

    for clause in &mut program.clauses {
        clause.lhs = *role_map
            .get(&clause.lhs)
            .ok_or(EffectLawError::InvalidProgram)?;
        if let Some(rhs) = clause.rhs.as_mut() {
            *rhs = *role_map.get(rhs).ok_or(EffectLawError::InvalidProgram)?;
        }
        if clause.opcode.get() == 0 {
            return Err(EffectLawError::InvalidProgram);
        }
        if let Some(constant) = clause.constant.as_mut() {
            constant.value_sha256.make_ascii_lowercase();
            if constant.value_type.get() == 0 || !is_sha256(&constant.value_sha256) {
                return Err(EffectLawError::InvalidConstant);
            }
        }
        if clause
            .argument_key_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
            || (clause.constant.is_some() && clause.argument_key_sha256.is_none())
        {
            return Err(EffectLawError::InvalidProgram);
        }
    }
    for role in &mut program.preserved_frame.roles {
        *role = *role_map.get(role).ok_or(EffectLawError::InvalidProgram)?;
    }
    program.clauses.sort();
    program.clauses.dedup();
    program.preserved_frame.roles.sort();
    program.preserved_frame.roles.dedup();
    Ok(())
}

fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    values.windows(2).any(|pair| pair[0] == pair[1])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, EffectLawError> {
    let bytes = serde_json::to_vec(value).map_err(|_| EffectLawError::Serialization)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
#[path = "effect_law_tests.rs"]
mod tests;
