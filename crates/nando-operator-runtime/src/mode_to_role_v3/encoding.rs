use nando_core::wave::{
    LocalRelationFragment, OperatorCircuit, OperatorCircuitRelation, OperatorRelationCell,
    PhaseCenterCell, RoleGraph, StructuralRoleSignature, SurfaceFragmentBundle,
    TernaryRelationState,
};
use nando_operator_kernel::{BindingValueTypeV1, CanonicalStructuralRoleV3, ProtocolModeV2};
use sha2::{Digest, Sha256};

use super::constraint::{CompiledConstraintV3, compile_constraints_v3, observed_constraints_v3};
use super::feature_codec::value_type_tag_v3;
use super::{CompiledProtocolModeV3, ModeToRoleErrorV3};

const SOURCE_ROLE_CONSTRAINT_V3: u32 = 0xf5c0_0001;

pub(super) fn compile_mode_graph_v3(
    mode: &ProtocolModeV2,
) -> Result<(RoleGraph, OperatorCircuit, Box<[CompiledConstraintV3]>), ModeToRoleErrorV3> {
    let constraints = compile_constraints_v3(mode)?;
    let source_type = mode
        .program
        .source_role_schema
        .roles
        .first()
        .ok_or(ModeToRoleErrorV3::InvalidSelector)?
        .value_type;
    let planes = constraints
        .iter()
        .map(CompiledConstraintV3::plane)
        .collect::<Vec<_>>();
    let mut roles = Vec::with_capacity(constraints.len().saturating_add(1));
    roles.push(source_role_signature_v3(source_type, planes));
    roles.extend(constraints.iter().map(CompiledConstraintV3::signature));
    let role_graph =
        RoleGraph::from_canonical_roles(roles).ok_or(ModeToRoleErrorV3::InvalidGraph)?;
    let relation_program = constraint_circuit_v3(&constraints)?;
    Ok((role_graph, relation_program, constraints.into_boxed_slice()))
}

pub(super) fn runtime_candidate_bundle_v3(
    mode: &CompiledProtocolModeV3,
    candidate: &CanonicalStructuralRoleV3,
    request_sha256: &str,
) -> Result<SurfaceFragmentBundle, ModeToRoleErrorV3> {
    let planes = mode
        .constraints
        .iter()
        .map(CompiledConstraintV3::plane)
        .collect::<Vec<_>>();
    let mut roles = Vec::with_capacity(mode.constraints.len().saturating_add(1));
    let mut matched_observed = Vec::with_capacity(mode.constraints.len());
    roles.push(source_role_signature_v3(
        candidate.features.value_type,
        planes,
    ));
    let observed_constraints = observed_constraints_v3(&candidate.features)?;
    for constraint in &mode.constraints {
        let observed = observed_constraints
            .iter()
            .find(|observed| observed.kind == constraint.kind && observed.slot == constraint.slot)
            .ok_or(ModeToRoleErrorV3::InvalidGraph)?;
        roles.push(observed.signature());
        matched_observed.push(*observed);
    }
    let relations = constraint_relations_v3(&matched_observed)?;
    SurfaceFragmentBundle::new(
        commitment_v3(
            b"nando.f5c.runtime-lineage.v3",
            &[request_sha256.as_bytes(), mode.mode_id_sha256.as_bytes()],
        ),
        commitment_v3(
            b"nando.f5c.runtime-surface.v3",
            &[
                request_sha256.as_bytes(),
                mode.mode_id_sha256.as_bytes(),
                &candidate.role_id.to_le_bytes(),
            ],
        ),
        roles,
        relations,
        Vec::new(),
    )
    .map_err(|_| ModeToRoleErrorV3::InvalidGraph)
}

fn constraint_circuit_v3(
    constraints: &[CompiledConstraintV3],
) -> Result<OperatorCircuit, ModeToRoleErrorV3> {
    OperatorCircuit::new(
        u8::try_from(constraints.len().saturating_add(1))
            .map_err(|_| ModeToRoleErrorV3::InvalidGraph)?,
        constraints
            .iter()
            .enumerate()
            .map(|(index, constraint)| {
                Ok(OperatorCircuitRelation {
                    cell: OperatorRelationCell {
                        plane: constraint.plane(),
                        source_role: 0,
                        target_role: u8::try_from(index.saturating_add(1))
                            .map_err(|_| ModeToRoleErrorV3::InvalidGraph)?,
                    },
                    state: TernaryRelationState::Supported,
                    phase_anchor: constraint_phase_v3(constraint),
                })
            })
            .collect::<Result<Vec<_>, ModeToRoleErrorV3>>()?,
    )
    .map_err(|_| ModeToRoleErrorV3::InvalidGraph)
}

fn constraint_relations_v3(
    constraints: &[CompiledConstraintV3],
) -> Result<Vec<LocalRelationFragment>, ModeToRoleErrorV3> {
    constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| {
            Ok(LocalRelationFragment {
                plane: constraint.plane(),
                source_local_role: 0,
                target_local_role: u8::try_from(index.saturating_add(1))
                    .map_err(|_| ModeToRoleErrorV3::InvalidGraph)?,
                state: TernaryRelationState::Supported,
                phase_anchor: constraint_phase_v3(constraint),
            })
        })
        .collect()
}

fn source_role_signature_v3(
    value_type: BindingValueTypeV1,
    planes: Vec<u8>,
) -> StructuralRoleSignature {
    StructuralRoleSignature::new(
        value_type_tag_v3(value_type),
        1,
        0,
        SOURCE_ROLE_CONSTRAINT_V3,
        planes,
    )
}

fn constraint_phase_v3(constraint: &CompiledConstraintV3) -> PhaseCenterCell {
    const CENTERS: [PhaseCenterCell; 4] = [
        PhaseCenterCell { re: 1.0, im: 0.0 },
        PhaseCenterCell { re: 0.0, im: 1.0 },
        PhaseCenterCell { re: -1.0, im: 0.0 },
        PhaseCenterCell { re: 0.0, im: -1.0 },
    ];
    let mixed = u32::from(constraint.plane())
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add(u32::from(constraint.slot).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(constraint.expected.wrapping_mul(0xc2b2_ae35));
    CENTERS[(mixed as usize) % CENTERS.len()]
}

fn commitment_v3(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}
