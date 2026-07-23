use std::collections::BTreeSet;

use nando_core::wave::{
    Commitment256, OPERATOR_PAGE32_BYTES, OPERATOR_PAGE32_HEADER_BYTES, OperatorCircuit,
    OperatorCircuitRelation, OperatorPage32, OperatorPage32Error, OperatorRelationCell,
    PhaseCenterCell, RoleGraph, StructuralRoleSignature, TernaryRelationState, TransformOp8,
};
use nando_operator_kernel::{CollectionOutputRenderer, ResponseProgram};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::RuntimeOperatorSpec;

pub const CRYSTALLIZED_REGISTRY_SCHEMA_V3: &str = "nando.crystallized-registry.v3";
pub const CRYSTALLIZED_REGISTRY_MAX_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeOperatorArtifact {
    page: OperatorPage32,
    relation_program: OperatorCircuit,
    role_graph: RoleGraph,
    transform_program: Box<[TransformOp8]>,
    renderer: CollectionOutputRenderer,
    actor_template: ResponseProgram,
}

impl RuntimeOperatorArtifact {
    #[must_use]
    pub fn new(
        page: OperatorPage32,
        relation_program: OperatorCircuit,
        role_graph: RoleGraph,
        transform_program: Box<[TransformOp8]>,
        renderer: CollectionOutputRenderer,
        actor_template: ResponseProgram,
    ) -> Self {
        Self {
            page,
            relation_program,
            role_graph,
            transform_program,
            renderer,
            actor_template,
        }
    }

    #[must_use]
    pub const fn page(&self) -> &OperatorPage32 {
        &self.page
    }

    #[must_use]
    pub const fn relation_program(&self) -> &OperatorCircuit {
        &self.relation_program
    }

    #[must_use]
    pub const fn role_graph(&self) -> &RoleGraph {
        &self.role_graph
    }

    #[must_use]
    pub fn transform_program(&self) -> &[TransformOp8] {
        &self.transform_program
    }

    #[must_use]
    pub const fn renderer(&self) -> &CollectionOutputRenderer {
        &self.renderer
    }

    #[must_use]
    pub const fn actor_template(&self) -> &ResponseProgram {
        &self.actor_template
    }

    #[must_use]
    pub fn spec(&self) -> RuntimeOperatorSpec<'_> {
        RuntimeOperatorSpec::new(
            &self.role_graph,
            &self.relation_program,
            &self.transform_program,
            &self.actor_template,
            Some(&self.page),
        )
    }

    #[must_use]
    pub fn unpaged_spec(&self) -> RuntimeOperatorSpec<'_> {
        RuntimeOperatorSpec::new(
            &self.role_graph,
            &self.relation_program,
            &self.transform_program,
            &self.actor_template,
            None,
        )
    }

    #[must_use]
    pub fn execution_equivalent(&self, other: &Self) -> bool {
        let (Ok(left_header), Ok(right_header)) = (self.page.header(), other.page.header()) else {
            return false;
        };
        let stable_header_matches = left_header.schema_version == right_header.schema_version
            && left_header.role_count == right_header.role_count
            && left_header.relation_plane_count == right_header.relation_plane_count
            && left_header.transform_count == right_header.transform_count
            && left_header.composition_node_count == right_header.composition_node_count
            && left_header.renderer_instruction_count == right_header.renderer_instruction_count
            && left_header.flags == right_header.flags
            && left_header.circuit_fingerprint64 == right_header.circuit_fingerprint64
            && left_header.verifier_binding_fingerprint64
                == right_header.verifier_binding_fingerprint64
            && left_header.role_signature_fingerprint64
                == right_header.role_signature_fingerprint64
            && left_header.payload_fingerprint64 == right_header.payload_fingerprint64;
        stable_header_matches
            && self.page.as_bytes()[OPERATOR_PAGE32_HEADER_BYTES..]
                == other.page.as_bytes()[OPERATOR_PAGE32_HEADER_BYTES..]
            && self.relation_program == other.relation_program
            && self.role_graph == other.role_graph
            && self.transform_program == other.transform_program
            && self.renderer == other.renderer
            && self.actor_template == other.actor_template
    }
}

/// Restart transport only. These fields carry proof commitments but grant no
/// authority; the proof owner must independently validate them after decode.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRestartParitySealData {
    pub winner_seal_sha256: Commitment256,
    pub actor_sha256: Commitment256,
    pub verifier_sha256: Commitment256,
    pub binding_receipts_root: Commitment256,
    pub execution_receipts_root: Commitment256,
    pub future_evidence_count: u32,
    pub future_lineage_count: u32,
    pub wrong_accepts: u32,
    pub seal_sha256: Commitment256,
}

/// Non-authoritative metadata transported beside the immutable runtime
/// artifact. Runtime validates structure and byte integrity, never proof truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeArtifactRestartMetadata {
    pub blueprint_sha256: Commitment256,
    pub candidate_set_sha256: Commitment256,
    pub support_root_sha256: Commitment256,
    pub future_evidence_root_sha256: Commitment256,
    pub future_lineage_root_sha256: Commitment256,
    pub winner_seal_sha256: Commitment256,
    pub actor_sha256: String,
    pub verifier_sha256: String,
    pub verified_future_lineages: Vec<Commitment256>,
    pub parity_seal: RuntimeRestartParitySealData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeArtifactRestartError {
    Encode,
    Decode,
    DigestMismatch,
    InvalidPage(OperatorPage32Error),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRuntimeOperatorArtifact {
    page: OperatorPage32,
    relation_program: OperatorCircuit,
    role_graph: RoleGraph,
    transform_program: Box<[TransformOp8]>,
    renderer: CollectionOutputRenderer,
    actor_template: Option<ResponseProgram>,
    metadata: RuntimeArtifactRestartMetadata,
}

impl DecodedRuntimeOperatorArtifact {
    pub fn finalize<E>(
        self,
        legacy_actor: impl FnOnce(
            &[TransformOp8],
            &CollectionOutputRenderer,
        ) -> Result<ResponseProgram, E>,
    ) -> Result<(RuntimeOperatorArtifact, RuntimeArtifactRestartMetadata), E> {
        let actor_template = match self.actor_template {
            Some(actor) => actor,
            None => legacy_actor(&self.transform_program, &self.renderer)?,
        };
        Ok((
            RuntimeOperatorArtifact::new(
                self.page,
                self.relation_program,
                self.role_graph,
                self.transform_program,
                self.renderer,
                actor_template,
            ),
            self.metadata,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CrystallizedRegistryV3 {
    schema: String,
    page_sha256: Commitment256,
    roles: Vec<RestartRole>,
    relations: Vec<RestartRelation>,
    renderer: CollectionOutputRenderer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actor_template: Option<ResponseProgram>,
    blueprint_sha256: Commitment256,
    candidate_set_sha256: Commitment256,
    support_root_sha256: Commitment256,
    future_evidence_root_sha256: Commitment256,
    future_lineage_root_sha256: Commitment256,
    winner_seal_sha256: Commitment256,
    actor_sha256: String,
    verifier_sha256: String,
    verified_future_lineages: Vec<Commitment256>,
    parity_seal: RestartParitySeal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartRole {
    type_class: u8,
    cardinality_class: u8,
    temporal_position: u8,
    constraint_mask: u32,
    neighboring_relation_planes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartRelation {
    plane: u8,
    source_role: u8,
    target_role: u8,
    state: i8,
    phase_re_bits: u64,
    phase_im_bits: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct RestartParitySeal {
    winner_seal_sha256: Commitment256,
    actor_sha256: Commitment256,
    verifier_sha256: Commitment256,
    binding_receipts_root: Commitment256,
    execution_receipts_root: Commitment256,
    #[serde(default)]
    future_evidence_count: u32,
    future_lineage_count: u32,
    wrong_accepts: u32,
    seal_sha256: Commitment256,
}

pub fn encode_runtime_artifact_registry(
    artifact: &RuntimeOperatorArtifact,
    metadata: &RuntimeArtifactRestartMetadata,
) -> Result<Box<[u8]>, RuntimeArtifactRestartError> {
    let registry = CrystallizedRegistryV3 {
        schema: CRYSTALLIZED_REGISTRY_SCHEMA_V3.to_owned(),
        page_sha256: Sha256::digest(artifact.page().as_bytes()).into(),
        roles: artifact
            .role_graph()
            .canonical_roles()
            .iter()
            .map(|role| RestartRole {
                type_class: role.type_class(),
                cardinality_class: role.cardinality_class(),
                temporal_position: role.temporal_position(),
                constraint_mask: role.constraint_mask(),
                neighboring_relation_planes: role.neighboring_relation_planes().to_vec(),
            })
            .collect(),
        relations: artifact
            .relation_program()
            .relations()
            .iter()
            .map(|relation| RestartRelation {
                plane: relation.cell.plane,
                source_role: relation.cell.source_role,
                target_role: relation.cell.target_role,
                state: relation.state as i8,
                phase_re_bits: relation.phase_anchor.re.to_bits(),
                phase_im_bits: relation.phase_anchor.im.to_bits(),
            })
            .collect(),
        renderer: artifact.renderer().clone(),
        actor_template: Some(artifact.actor_template().clone()),
        blueprint_sha256: metadata.blueprint_sha256,
        candidate_set_sha256: metadata.candidate_set_sha256,
        support_root_sha256: metadata.support_root_sha256,
        future_evidence_root_sha256: metadata.future_evidence_root_sha256,
        future_lineage_root_sha256: metadata.future_lineage_root_sha256,
        winner_seal_sha256: metadata.winner_seal_sha256,
        actor_sha256: metadata.actor_sha256.clone(),
        verifier_sha256: metadata.verifier_sha256.clone(),
        verified_future_lineages: metadata.verified_future_lineages.clone(),
        parity_seal: RestartParitySeal::from(&metadata.parity_seal),
    };
    let bytes = serde_cbor::to_vec(&registry).map_err(|_| RuntimeArtifactRestartError::Encode)?;
    if bytes.len() > CRYSTALLIZED_REGISTRY_MAX_BYTES {
        return Err(RuntimeArtifactRestartError::Encode);
    }
    Ok(bytes.into_boxed_slice())
}

pub fn decode_runtime_artifact_registry(
    page_bytes: &[u8],
    registry_cbor: &[u8],
) -> Result<DecodedRuntimeOperatorArtifact, RuntimeArtifactRestartError> {
    if page_bytes.len() != OPERATOR_PAGE32_BYTES
        || registry_cbor.len() > CRYSTALLIZED_REGISTRY_MAX_BYTES
    {
        return Err(RuntimeArtifactRestartError::Decode);
    }
    let page =
        OperatorPage32::from_bytes(page_bytes).map_err(RuntimeArtifactRestartError::InvalidPage)?;
    let registry: CrystallizedRegistryV3 =
        serde_cbor::from_slice(registry_cbor).map_err(|_| RuntimeArtifactRestartError::Decode)?;
    if registry.schema != CRYSTALLIZED_REGISTRY_SCHEMA_V3
        || registry.page_sha256 != Commitment256::from(Sha256::digest(page_bytes))
    {
        return Err(RuntimeArtifactRestartError::DigestMismatch);
    }
    let role_graph = RoleGraph::from_canonical_roles(
        registry
            .roles
            .into_iter()
            .map(|role| {
                StructuralRoleSignature::new(
                    role.type_class,
                    role.cardinality_class,
                    role.temporal_position,
                    role.constraint_mask,
                    role.neighboring_relation_planes,
                )
            })
            .collect(),
    )
    .ok_or(RuntimeArtifactRestartError::Decode)?;
    let header = page
        .header()
        .map_err(RuntimeArtifactRestartError::InvalidPage)?;
    let transform_program = (0..usize::from(header.transform_count))
        .map(|index| {
            page.transform(index)
                .ok_or(RuntimeArtifactRestartError::Decode)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let relations = registry
        .relations
        .into_iter()
        .map(restart_relation)
        .collect::<Result<Vec<_>, _>>()?;
    let observed_roles = relations
        .iter()
        .flat_map(|relation| [relation.cell.source_role, relation.cell.target_role])
        .collect::<BTreeSet<_>>();
    let virtual_roles = transform_program
        .iter()
        .map(|transform| transform.output)
        .filter(|role| !observed_roles.contains(role))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let relation_program =
        OperatorCircuit::new_with_virtual_roles(role_graph.role_count(), relations, &virtual_roles)
            .map_err(|_| RuntimeArtifactRestartError::Decode)?;
    if relation_program.fingerprint64() != header.circuit_fingerprint64
        || usize::from(header.role_count) != role_graph.canonical_roles().len()
    {
        return Err(RuntimeArtifactRestartError::DigestMismatch);
    }
    Ok(DecodedRuntimeOperatorArtifact {
        page,
        relation_program,
        role_graph,
        transform_program,
        renderer: registry.renderer,
        actor_template: registry.actor_template,
        metadata: RuntimeArtifactRestartMetadata {
            blueprint_sha256: registry.blueprint_sha256,
            candidate_set_sha256: registry.candidate_set_sha256,
            support_root_sha256: registry.support_root_sha256,
            future_evidence_root_sha256: registry.future_evidence_root_sha256,
            future_lineage_root_sha256: registry.future_lineage_root_sha256,
            winner_seal_sha256: registry.winner_seal_sha256,
            actor_sha256: registry.actor_sha256,
            verifier_sha256: registry.verifier_sha256,
            verified_future_lineages: registry.verified_future_lineages,
            parity_seal: registry.parity_seal.into(),
        },
    })
}

impl From<&RuntimeRestartParitySealData> for RestartParitySeal {
    fn from(seal: &RuntimeRestartParitySealData) -> Self {
        Self {
            winner_seal_sha256: seal.winner_seal_sha256,
            actor_sha256: seal.actor_sha256,
            verifier_sha256: seal.verifier_sha256,
            binding_receipts_root: seal.binding_receipts_root,
            execution_receipts_root: seal.execution_receipts_root,
            future_evidence_count: seal.future_evidence_count,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        }
    }
}

impl From<RestartParitySeal> for RuntimeRestartParitySealData {
    fn from(seal: RestartParitySeal) -> Self {
        let future_evidence_count = if seal.future_evidence_count == 0 {
            seal.future_lineage_count
        } else {
            seal.future_evidence_count
        };
        Self {
            winner_seal_sha256: seal.winner_seal_sha256,
            actor_sha256: seal.actor_sha256,
            verifier_sha256: seal.verifier_sha256,
            binding_receipts_root: seal.binding_receipts_root,
            execution_receipts_root: seal.execution_receipts_root,
            future_evidence_count,
            future_lineage_count: seal.future_lineage_count,
            wrong_accepts: seal.wrong_accepts,
            seal_sha256: seal.seal_sha256,
        }
    }
}

fn restart_relation(
    relation: RestartRelation,
) -> Result<OperatorCircuitRelation, RuntimeArtifactRestartError> {
    let phase_anchor = PhaseCenterCell {
        re: f64::from_bits(relation.phase_re_bits),
        im: f64::from_bits(relation.phase_im_bits),
    };
    if !phase_anchor.re.is_finite()
        || !phase_anchor.im.is_finite()
        || phase_anchor.re.hypot(phase_anchor.im) <= f64::EPSILON
    {
        return Err(RuntimeArtifactRestartError::Decode);
    }
    let state = match relation.state {
        -1 => TernaryRelationState::Opposed,
        1 => TernaryRelationState::Supported,
        _ => return Err(RuntimeArtifactRestartError::Decode),
    };
    Ok(OperatorCircuitRelation {
        cell: OperatorRelationCell {
            plane: relation.plane,
            source_role: relation.source_role,
            target_role: relation.target_role,
        },
        state,
        phase_anchor,
    })
}
