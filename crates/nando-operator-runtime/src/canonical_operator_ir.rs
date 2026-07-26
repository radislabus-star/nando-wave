use std::collections::BTreeSet;

use nando_core::wave::{
    CandidateOperatorBlueprint, OperatorCircuit, OperatorCircuitRelation, OperatorRelationCell,
    PhaseCenterCell, RoleGraph, StructuralRoleSignature, TernaryRelationState, TransformOp8,
};
use nando_operator_kernel::{
    CanonicalOperatorCompositionEdgeV1, CanonicalOperatorIrErrorV1, CanonicalOperatorIrV1,
    CanonicalOperatorRelationV1, CanonicalOperatorRoleV1, CanonicalOperatorTransformV1,
    CollectionOutputRenderer, ResponseProgram,
};

use crate::{CompiledRuntimeProgram, RuntimeProgramCompileError, compile_runtime_program};

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledCanonicalOperatorIrV1 {
    role_graph: RoleGraph,
    relation_program: OperatorCircuit,
    transform_program: Box<[TransformOp8]>,
    composition_edges: Box<[CanonicalOperatorCompositionEdgeV1]>,
    renderer: CollectionOutputRenderer,
    actor_template: ResponseProgram,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalOperatorIrCompileErrorV1 {
    InvalidIr(CanonicalOperatorIrErrorV1),
    Program(RuntimeProgramCompileError),
    InvalidRoleGraph,
    InvalidCircuit,
    InvalidRelationState,
    InvalidTransformOrder,
}

impl CompiledCanonicalOperatorIrV1 {
    #[must_use]
    pub const fn role_graph(&self) -> &RoleGraph {
        &self.role_graph
    }

    #[must_use]
    pub const fn relation_program(&self) -> &OperatorCircuit {
        &self.relation_program
    }

    #[must_use]
    pub fn transform_program(&self) -> &[TransformOp8] {
        &self.transform_program
    }

    #[must_use]
    pub fn composition_edges(&self) -> &[CanonicalOperatorCompositionEdgeV1] {
        &self.composition_edges
    }

    #[must_use]
    pub const fn renderer(&self) -> &CollectionOutputRenderer {
        &self.renderer
    }

    #[must_use]
    pub const fn actor_template(&self) -> &ResponseProgram {
        &self.actor_template
    }
}

pub fn canonical_operator_ir_from_response_program_v1(
    program: &ResponseProgram,
    verifier_contract_sha256: String,
) -> Result<CanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    let compiled =
        compile_runtime_program(program).map_err(CanonicalOperatorIrCompileErrorV1::Program)?;
    canonical_operator_ir_from_compiled_program_v1(
        &compiled,
        program.clone(),
        verifier_contract_sha256,
    )
}

pub fn canonical_operator_ir_from_blueprint_v1(
    blueprint: &CandidateOperatorBlueprint,
    renderer: CollectionOutputRenderer,
    actor_template: ResponseProgram,
    verifier_contract_sha256: String,
) -> Result<CanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    canonical_operator_ir_from_parts_v1(
        blueprint.role_graph(),
        blueprint.relation_program(),
        blueprint.transform_program(),
        renderer,
        actor_template,
        verifier_contract_sha256,
    )
}

pub fn canonical_operator_ir_from_runtime_artifact_v1(
    artifact: &crate::RuntimeOperatorArtifact,
    verifier_contract_sha256: String,
) -> Result<CanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    canonical_operator_ir_from_parts_v1(
        artifact.role_graph(),
        artifact.relation_program(),
        artifact.transform_program(),
        artifact.renderer().clone(),
        artifact.actor_template().clone(),
        verifier_contract_sha256,
    )
}

pub fn compile_canonical_operator_ir_v1(
    ir: &CanonicalOperatorIrV1,
) -> Result<CompiledCanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    ir.validate()
        .map_err(CanonicalOperatorIrCompileErrorV1::InvalidIr)?;
    let roles = ir
        .roles()
        .iter()
        .map(|role| {
            StructuralRoleSignature::new(
                role.type_class,
                role.cardinality_class,
                role.temporal_position,
                role.constraint_mask,
                role.neighboring_relation_planes.clone(),
            )
        })
        .collect::<Vec<_>>();
    let role_graph = RoleGraph::from_canonical_roles(roles)
        .ok_or(CanonicalOperatorIrCompileErrorV1::InvalidRoleGraph)?;
    let relations = ir
        .relations()
        .iter()
        .map(|relation| {
            let state = match relation.state {
                -1 => TernaryRelationState::Opposed,
                1 => TernaryRelationState::Supported,
                _ => return Err(CanonicalOperatorIrCompileErrorV1::InvalidRelationState),
            };
            Ok(OperatorCircuitRelation {
                cell: OperatorRelationCell {
                    plane: relation.plane,
                    source_role: relation.source_role,
                    target_role: relation.target_role,
                },
                state,
                phase_anchor: PhaseCenterCell {
                    re: f64::from_bits(relation.phase_re_bits),
                    im: f64::from_bits(relation.phase_im_bits),
                },
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let transforms = ir
        .transforms()
        .iter()
        .map(|transform| TransformOp8 {
            opcode: transform.opcode,
            output: transform.output,
            source_a: transform.source_a,
            source_b: transform.source_b,
            parameter: transform.parameter,
            flags: transform.flags,
        })
        .collect::<Vec<_>>();
    let observed_roles = relations
        .iter()
        .flat_map(|relation| [relation.cell.source_role, relation.cell.target_role])
        .collect::<BTreeSet<_>>();
    let virtual_roles = transforms
        .iter()
        .map(|transform| transform.output)
        .filter(|role| !observed_roles.contains(role))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let relation_program =
        OperatorCircuit::new_with_virtual_roles(role_graph.role_count(), relations, &virtual_roles)
            .map_err(|_| CanonicalOperatorIrCompileErrorV1::InvalidCircuit)?;
    Ok(CompiledCanonicalOperatorIrV1 {
        role_graph,
        relation_program,
        transform_program: transforms.into_boxed_slice(),
        composition_edges: ir.composition_edges().into(),
        renderer: ir.renderer().clone(),
        actor_template: ir.actor_template().clone(),
    })
}

fn canonical_operator_ir_from_compiled_program_v1(
    compiled: &CompiledRuntimeProgram,
    actor_template: ResponseProgram,
    verifier_contract_sha256: String,
) -> Result<CanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    canonical_operator_ir_from_parts_v1(
        compiled.role_graph(),
        compiled.relation_program(),
        compiled.transform_program(),
        compiled.renderer().clone(),
        actor_template,
        verifier_contract_sha256,
    )
}

fn canonical_operator_ir_from_parts_v1(
    role_graph: &RoleGraph,
    relation_program: &OperatorCircuit,
    transform_program: &[TransformOp8],
    renderer: CollectionOutputRenderer,
    actor_template: ResponseProgram,
    verifier_contract_sha256: String,
) -> Result<CanonicalOperatorIrV1, CanonicalOperatorIrCompileErrorV1> {
    let roles = role_graph
        .canonical_roles()
        .iter()
        .map(|role| CanonicalOperatorRoleV1 {
            type_class: role.type_class(),
            cardinality_class: role.cardinality_class(),
            temporal_position: role.temporal_position(),
            constraint_mask: role.constraint_mask(),
            neighboring_relation_planes: role.neighboring_relation_planes().to_vec(),
        })
        .collect();
    let relations = relation_program
        .relations()
        .iter()
        .map(|relation| CanonicalOperatorRelationV1 {
            plane: relation.cell.plane,
            source_role: relation.cell.source_role,
            target_role: relation.cell.target_role,
            state: relation.state as i8,
            phase_re_bits: relation.phase_anchor.re.to_bits(),
            phase_im_bits: relation.phase_anchor.im.to_bits(),
        })
        .collect();
    let ordered_transforms = crate::ordered_role_transforms(transform_program)
        .map_err(|_| CanonicalOperatorIrCompileErrorV1::InvalidTransformOrder)?;
    let transforms = ordered_transforms
        .iter()
        .map(|transform| CanonicalOperatorTransformV1 {
            opcode: transform.opcode,
            output: transform.output,
            source_a: transform.source_a,
            source_b: transform.source_b,
            parameter: transform.parameter,
            flags: transform.flags,
        })
        .collect::<Vec<_>>();
    let composition_edges = transform_composition_edges(&transforms);
    CanonicalOperatorIrV1::new(
        roles,
        relations,
        transforms,
        composition_edges,
        renderer,
        actor_template,
        verifier_contract_sha256,
    )
    .map_err(CanonicalOperatorIrCompileErrorV1::InvalidIr)
}

fn transform_composition_edges(
    transforms: &[CanonicalOperatorTransformV1],
) -> Vec<CanonicalOperatorCompositionEdgeV1> {
    let mut edges = Vec::new();
    for (producer, produced) in transforms.iter().enumerate() {
        for (consumer, consumed) in transforms.iter().enumerate() {
            if producer != consumer
                && (produced.output == consumed.source_a || produced.output == consumed.source_b)
            {
                edges.push(CanonicalOperatorCompositionEdgeV1 {
                    producer_step: producer as u8,
                    consumer_step: consumer as u8,
                });
            }
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use nando_operator_kernel::{ResponseValueSelector, ValueProjectionFormat};

    use super::*;

    #[test]
    fn response_program_ir_roundtrip_preserves_compiled_runtime() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::RequestLastToken,
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let ir =
            canonical_operator_ir_from_response_program_v1(&program, "11".repeat(32)).expect("IR");
        let restored = compile_canonical_operator_ir_v1(&ir).expect("compiled IR");
        let direct = compile_runtime_program(&program).expect("direct compile");
        assert_eq!(restored.role_graph(), direct.role_graph());
        assert_eq!(restored.relation_program(), direct.relation_program());
        assert_eq!(restored.transform_program(), direct.transform_program());
        assert_eq!(restored.renderer(), direct.renderer());
        assert_eq!(restored.actor_template(), &program);
    }
}
