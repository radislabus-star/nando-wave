use nando_core::wave::{
    OperatorCircuit, OperatorCircuitRelation, OperatorRelationCell, RoleGraph,
    StructuralRoleSignature, TernaryRelationState, TransformOp8, phase_vector_from_atoms,
};
use nando_operator_kernel::{
    AtomValueType, CollectionOutputRenderer, CollectionProgramStep, ProjectStatusMapping,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector,
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_OK,
    TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS, TRANSFORM_STATUS_ZERO_IS_TRUE,
    TRANSFORM_VALUE_BOOLEAN, TRANSFORM_VALUE_COLLECTION, TRANSFORM_VALUE_IDENTIFIER,
    TRANSFORM_VALUE_INTEGER, TRANSFORM_VALUE_STRING, ValueProjectionFormat,
};

use crate::{
    runtime_multi_role_signature_for_selector, runtime_role_signature_for_selector,
    selector_value_type,
};

const MAX_COMPILED_PROGRAM_ROLES: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledRuntimeProgram {
    role_graph: RoleGraph,
    relation_program: OperatorCircuit,
    transform_program: Box<[TransformOp8]>,
    renderer: CollectionOutputRenderer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeProgramCompileError {
    UnsupportedProgram,
    InvalidProgram,
    RoleBudgetExceeded,
    CircuitBuildFailed,
}

impl CompiledRuntimeProgram {
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
    pub const fn renderer(&self) -> &CollectionOutputRenderer {
        &self.renderer
    }
}

pub fn compile_runtime_program(
    program: &ResponseProgram,
) -> Result<CompiledRuntimeProgram, RuntimeProgramCompileError> {
    program
        .validate()
        .map_err(|_| RuntimeProgramCompileError::InvalidProgram)?;
    let shape = runtime_program_shape(program)?;
    if shape.roles.is_empty() || shape.roles.len() > MAX_COMPILED_PROGRAM_ROLES {
        return Err(RuntimeProgramCompileError::RoleBudgetExceeded);
    }

    let is_filter = shape.opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE;
    let role_count = if is_filter {
        1_usize
            .saturating_add(shape.roles.len())
            .saturating_add(if shape.compose_count { 2 } else { 1 })
    } else {
        1_usize.saturating_add(shape.roles.len().saturating_mul(2))
    };
    let role_count_u8 =
        u8::try_from(role_count).map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
    let planes = (0..shape.roles.len())
        .map(|index| {
            u8::try_from(index).map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); role_count];
    roles[0] = StructuralRoleSignature::new(5, 1, 0, 1, planes);
    let mut relations = Vec::with_capacity(shape.roles.len());
    for (index, (selector, _)) in shape.roles.iter().enumerate() {
        let source =
            u8::try_from(index + 1).map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
        let plane =
            u8::try_from(index).map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
        let value_type =
            selector_value_type(selector).ok_or(RuntimeProgramCompileError::InvalidProgram)?;
        roles[usize::from(source)] = if shape.roles.len() == 1 {
            runtime_role_signature_for_selector(selector, plane)
        } else {
            runtime_multi_role_signature_for_selector(selector, plane)
        };
        let phase_anchor = if shape.roles.len() == 1 {
            let phase_atoms = [
                format!("scalar_type:{}", runtime_value_type_tag(value_type)),
                "cardinality:unique".to_owned(),
            ];
            phase_vector_from_atoms(phase_atoms.iter().map(String::as_str), 1)[0]
        } else {
            let atom = format!(
                "scalar_role:{index}:type:{}",
                runtime_value_type_tag(value_type)
            );
            phase_vector_from_atoms([atom.as_str()], 1)[0]
        };
        relations.push(OperatorCircuitRelation {
            cell: OperatorRelationCell {
                plane,
                source_role: 0,
                target_role: source,
            },
            state: TernaryRelationState::Supported,
            phase_anchor,
        });
    }

    let (transforms, virtual_roles) = if is_filter {
        compile_filter_transforms(&shape, &mut roles)?
    } else {
        compile_projection_transforms(&shape, &mut roles)?
    };
    let role_graph = RoleGraph::from_canonical_roles(roles)
        .ok_or(RuntimeProgramCompileError::CircuitBuildFailed)?;
    let relation_program =
        OperatorCircuit::new_with_virtual_roles(role_count_u8, relations, &virtual_roles)
            .map_err(|_| RuntimeProgramCompileError::CircuitBuildFailed)?;
    Ok(CompiledRuntimeProgram {
        role_graph,
        relation_program,
        transform_program: transforms.into_boxed_slice(),
        renderer: shape.renderer,
    })
}

struct RuntimeProgramShape {
    roles: Vec<(ResponseValueSelector, ValueProjectionFormat)>,
    opcode: u8,
    flags: u16,
    compose_count: bool,
    renderer: CollectionOutputRenderer,
}

fn runtime_program_shape(
    program: &ResponseProgram,
) -> Result<RuntimeProgramShape, RuntimeProgramCompileError> {
    let (roles, opcode, flags, compose_count, renderer) = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. }
        | ResponseOperation::CustomToolCallFromRoles { selector, .. } => (
            vec![(selector.clone(), ValueProjectionFormat::PlainText)],
            TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            0,
            false,
            CollectionOutputRenderer::Direct,
        ),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } if completion_state == "completed" => (
            renderer_roles(selector, *format, renderer),
            TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            u16::from(*format == ValueProjectionFormat::CanonicalJson),
            false,
            renderer.clone(),
        ),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } if completion_state == "completed" => (
            vec![(selector.clone(), ValueProjectionFormat::PlainText)],
            TRANSFORM_OPCODE_PROJECT_STATUS,
            status_mapping_flags(*mapping),
            false,
            renderer.clone(),
        ),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            completion_state,
            ..
        } if completion_state == "completed"
            && steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ] =>
        {
            (
                vec![(
                    ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Collection,
                    },
                    *format,
                )],
                TRANSFORM_OPCODE_COUNT_COLLECTION,
                0,
                false,
                renderer.clone(),
            )
        }
        ResponseOperation::ComposeCollection {
            steps,
            renderer,
            completion_state,
            ..
        } if completion_state == "completed" => {
            let [
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                    selector,
                    value_type: _,
                },
                tail @ ..,
            ] = steps.as_slice()
            else {
                return Err(RuntimeProgramCompileError::UnsupportedProgram);
            };
            let compose_count = match tail {
                [] => false,
                [CollectionProgramStep::Count] => true,
                _ => return Err(RuntimeProgramCompileError::UnsupportedProgram),
            };
            (
                vec![
                    (
                        ResponseValueSelector::UniqueScalar {
                            value_type: AtomValueType::Collection,
                        },
                        ValueProjectionFormat::CanonicalJson,
                    ),
                    (selector.clone(), ValueProjectionFormat::CanonicalJson),
                ],
                TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
                TRANSFORM_FLAG_CANONICAL_JSON,
                compose_count,
                renderer.clone(),
            )
        }
        _ => return Err(RuntimeProgramCompileError::UnsupportedProgram),
    };
    Ok(RuntimeProgramShape {
        roles,
        opcode,
        flags,
        compose_count,
        renderer,
    })
}

fn renderer_roles(
    primary: &ResponseValueSelector,
    format: ValueProjectionFormat,
    renderer: &CollectionOutputRenderer,
) -> Vec<(ResponseValueSelector, ValueProjectionFormat)> {
    let mut roles = Vec::new();
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            let role = match segment {
                ResponseRenderSegment::Primary => Some((primary.clone(), format)),
                ResponseRenderSegment::Selected { selector, format } => {
                    Some((selector.clone(), *format))
                }
                ResponseRenderSegment::Static { .. } => None,
            };
            if let Some(role) = role
                && !roles.contains(&role)
            {
                roles.push(role);
            }
        }
    }
    if roles.is_empty() {
        roles.push((primary.clone(), format));
    }
    roles
}

fn compile_projection_transforms(
    shape: &RuntimeProgramShape,
    roles: &mut [StructuralRoleSignature],
) -> Result<(Vec<TransformOp8>, Vec<u8>), RuntimeProgramCompileError> {
    let output_offset = 1 + shape.roles.len();
    let mut transforms = Vec::with_capacity(shape.roles.len());
    let mut virtual_roles = Vec::with_capacity(shape.roles.len());
    for (index, (selector, format)) in shape.roles.iter().enumerate() {
        let source =
            u8::try_from(index + 1).map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
        let output = u8::try_from(output_offset + index)
            .map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
        let value_type =
            selector_value_type(selector).ok_or(RuntimeProgramCompileError::InvalidProgram)?;
        roles[usize::from(output)] =
            StructuralRoleSignature::new(runtime_value_type_tag(value_type), 1, 2, 4, Vec::new());
        transforms.push(TransformOp8 {
            opcode: shape.opcode,
            output,
            source_a: source,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: transform_parameter(value_type)
                | (u16::try_from(index)
                    .map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?
                    << 8),
            flags: match shape.opcode {
                TRANSFORM_OPCODE_PROJECT_STATUS => shape.flags,
                TRANSFORM_OPCODE_COUNT_COLLECTION => 0,
                _ if *format == ValueProjectionFormat::CanonicalJson => {
                    TRANSFORM_FLAG_CANONICAL_JSON
                }
                _ => 0,
            },
        });
        virtual_roles.push(output);
    }
    Ok((transforms, virtual_roles))
}

fn compile_filter_transforms(
    shape: &RuntimeProgramShape,
    roles: &mut [StructuralRoleSignature],
) -> Result<(Vec<TransformOp8>, Vec<u8>), RuntimeProgramCompileError> {
    let output = u8::try_from(1 + shape.roles.len())
        .map_err(|_| RuntimeProgramCompileError::RoleBudgetExceeded)?;
    let predicate_type =
        selector_value_type(&shape.roles[1].0).ok_or(RuntimeProgramCompileError::InvalidProgram)?;
    let mut transforms = vec![TransformOp8 {
        opcode: TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
        output,
        source_a: 1,
        source_b: 2,
        parameter: transform_parameter(predicate_type),
        flags: shape.flags,
    }];
    roles[usize::from(output)] = StructuralRoleSignature::new(
        runtime_value_type_tag(AtomValueType::Collection),
        1,
        2,
        4,
        Vec::new(),
    );
    let mut virtual_roles = vec![output];
    if shape.compose_count {
        let count_output = output
            .checked_add(1)
            .ok_or(RuntimeProgramCompileError::RoleBudgetExceeded)?;
        roles[usize::from(count_output)] = StructuralRoleSignature::new(
            runtime_value_type_tag(AtomValueType::Integer),
            1,
            2,
            4,
            Vec::new(),
        );
        transforms.push(TransformOp8 {
            opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
            output: count_output,
            source_a: output,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: (1 << 8) | transform_parameter(AtomValueType::Collection),
            flags: 0,
        });
        virtual_roles.push(count_output);
    }
    Ok((transforms, virtual_roles))
}

const fn transform_parameter(value_type: AtomValueType) -> u16 {
    match value_type {
        AtomValueType::String => TRANSFORM_VALUE_STRING,
        AtomValueType::Integer => TRANSFORM_VALUE_INTEGER,
        AtomValueType::Boolean => TRANSFORM_VALUE_BOOLEAN,
        AtomValueType::Identifier => TRANSFORM_VALUE_IDENTIFIER,
        AtomValueType::Collection => TRANSFORM_VALUE_COLLECTION,
    }
}

const fn runtime_value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
}

const fn status_mapping_flags(mapping: ProjectStatusMapping) -> u16 {
    match mapping {
        ProjectStatusMapping::ZeroIsSuccess => TRANSFORM_STATUS_ZERO_IS_SUCCESS,
        ProjectStatusMapping::ZeroIsPass => TRANSFORM_STATUS_ZERO_IS_PASS,
        ProjectStatusMapping::ZeroIsOk => TRANSFORM_STATUS_ZERO_IS_OK,
        ProjectStatusMapping::ZeroIsTrue => TRANSFORM_STATUS_ZERO_IS_TRUE,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{RuntimeOperatorSpec, bind_pre_action_with_validator};

    #[test]
    fn request_selector_compiles_and_executes_through_bound_runtime() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::RequestLastToken,
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let compiled = compile_runtime_program(&program).expect("compiled request projection");
        assert_eq!(compiled.role_graph().role_count(), 3);
        assert_eq!(compiled.relation_program().relations().len(), 1);
        assert_eq!(compiled.transform_program().len(), 1);
        let payload = json!({
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "return alpha"}]
            }]
        });
        let bound = bind_pre_action_with_validator(
            RuntimeOperatorSpec::new(
                compiled.role_graph(),
                compiled.relation_program(),
                compiled.transform_program(),
                &program,
                None,
            ),
            "return alpha",
            &payload,
            |_, response| (response == "alpha").then_some(()).ok_or("wrong response"),
        )
        .expect("bound request role");
        assert_eq!(bound.execute_unverified().expect("VM response"), "alpha");
    }

    #[test]
    fn count_program_compiles_to_one_relation_and_one_transform() {
        let program = ResponseProgram::compose_collection(
            vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let compiled = compile_runtime_program(&program).expect("compiled count");
        assert_eq!(compiled.role_graph().role_count(), 3);
        assert_eq!(compiled.relation_program().relations().len(), 1);
        assert_eq!(
            compiled.transform_program()[0].opcode,
            TRANSFORM_OPCODE_COUNT_COLLECTION
        );
    }
}
