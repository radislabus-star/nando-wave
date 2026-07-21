#[cfg(test)]
use nando_core::wave::{OperatorPage32, TransformOp8};
#[cfg(test)]
use nando_operator_kernel::{
    OPERATOR_RENDERER_EMIT as RENDERER_EMIT, OPERATOR_RENDERER_VERSION as RENDERER_VERSION,
};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
use crate::{
    CollectionOutputRenderer, ResponseRenderSegment, ResponseValueSelector,
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_OK,
    TRANSFORM_VALUE_COLLECTION, ValueProjectionFormat,
};

pub(crate) use crate::operator_vm_compiler::{
    encode_renderer_program, encode_typed_actor_renderer_program,
};
pub(crate) use nando_operator_runtime::execute_operator_page_with_actor;
#[cfg(test)]
pub(crate) use nando_operator_runtime::{OperatorVmError, execute_operator_page};

#[cfg(test)]
mod tests {
    use nando_core::wave::{
        OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES,
        OPERATOR_PAGE32_RENDERER_BYTES, OperatorPage32Metadata, StructuralRole16,
        TernaryOperatorCube32,
    };
    use serde_json::json;

    use super::*;
    use crate::{
        AtomValueType, ResponseArgument, ResponseProgram, SemanticRole, TRANSFORM_VALUE_IDENTIFIER,
    };

    fn page(transforms: &[TransformOp8], renderer: &CollectionOutputRenderer) -> OperatorPage32 {
        let (renderer_program, renderer_instruction_count) =
            encode_renderer_program(renderer, transforms).expect("encodable renderer");
        let mut composition = [0_u8; OPERATOR_PAGE32_COMPOSITION_BYTES];
        let mut composition_count = 0_u8;
        for (producer, produced) in transforms.iter().enumerate() {
            for (consumer, consumed) in transforms.iter().enumerate() {
                if producer != consumer
                    && (produced.output == consumed.source_a
                        || produced.output == consumed.source_b)
                {
                    let offset = usize::from(composition_count) * 2;
                    composition[offset] = producer as u8;
                    composition[offset + 1] = consumer as u8;
                    composition_count = composition_count.saturating_add(1);
                }
            }
        }
        let role_count = transforms
            .iter()
            .flat_map(|transform| [transform.output, transform.source_a, transform.source_b])
            .filter(|role| *role != TRANSFORM_ROLE_NONE)
            .max()
            .map_or(0, |role| usize::from(role) + 1);
        OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 1,
                circuit_fingerprint64: 1,
                verifier_binding_fingerprint64: 2,
                proof_lineage_fingerprint64: 3,
                role_signature_fingerprint64: 4,
                relation_plane_count: 1,
                composition_node_count: composition_count,
                renderer_instruction_count,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &vec![StructuralRole16::default(); role_count],
            &TernaryOperatorCube32::default(),
            transforms,
            &composition,
            &renderer_program,
        )
        .expect("valid test page")
    }

    fn typed_actor_page(transform: TransformOp8) -> OperatorPage32 {
        let (renderer, renderer_instruction_count) =
            encode_typed_actor_renderer_program(&[transform]).expect("typed actor renderer");
        OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 1,
                circuit_fingerprint64: 1,
                verifier_binding_fingerprint64: 2,
                proof_lineage_fingerprint64: 3,
                role_signature_fingerprint64: 4,
                relation_plane_count: 1,
                composition_node_count: 0,
                renderer_instruction_count,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &[StructuralRole16::default(); 3],
            &TernaryOperatorCube32::default(),
            &[transform],
            &[0; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &renderer,
        )
        .expect("valid typed actor page")
    }

    fn transform(source: u8, order: u16) -> TransformOp8 {
        TransformOp8 {
            opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            output: u8::try_from(order).expect("test transform order fits u8") + 2,
            source_a: source,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: (order << 8) | 1,
            flags: 0,
        }
    }

    fn count_transform(source: u8) -> TransformOp8 {
        TransformOp8 {
            opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
            output: 2,
            source_a: source,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: TRANSFORM_VALUE_COLLECTION,
            flags: 0,
        }
    }

    fn status_transform(source: u8, mapping: u16) -> TransformOp8 {
        TransformOp8 {
            opcode: TRANSFORM_OPCODE_PROJECT_STATUS,
            output: 2,
            source_a: source,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: 1,
            flags: mapping,
        }
    }

    fn filter_transform(collection: u8, predicate: u8) -> TransformOp8 {
        TransformOp8 {
            opcode: TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
            output: 2,
            source_a: collection,
            source_b: predicate,
            parameter: 0,
            flags: TRANSFORM_FLAG_CANONICAL_JSON,
        }
    }

    #[test]
    fn page_composes_filter_then_count_through_virtual_role() {
        let filter = filter_transform(0, 1);
        let count = TransformOp8 {
            opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
            output: 3,
            source_a: filter.output,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: (1 << 8) | TRANSFORM_VALUE_COLLECTION,
            flags: 0,
        };
        let selectors = [
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Collection,
            },
            ResponseValueSelector::RequestLastToken,
        ];
        let page = page(&[filter, count], &CollectionOutputRenderer::Direct);
        assert_eq!(
            execute_operator_page(
                &page,
                &selectors,
                "Filter active",
                &json!({
                    "input": [
                        {"type":"message", "role":"user", "content":"Filter active"},
                        {"type":"function_call_output", "output":"{\"items\":[{\"kind\":\"active\"},{\"kind\":\"idle\"}]}"}
                    ]
                }),
            ),
            Ok("1".to_owned())
        );
    }

    #[test]
    fn typed_actor_renderer_executes_bound_function_call() {
        let selector = ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Identifier,
        };
        let actor = ResponseProgram::function_call_from_roles(
            "continue_job",
            selector.clone(),
            vec![ResponseArgument::Role {
                name: "job_id".to_owned(),
                role: SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
        );
        let page = typed_actor_page(TransformOp8 {
            parameter: TRANSFORM_VALUE_IDENTIFIER,
            ..transform(0, 0)
        });
        let payload = json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"job\":\"task-991\"}"
            }]
        });

        let response = execute_operator_page_with_actor(
            &page,
            &[selector],
            "Continue the job",
            &payload,
            &actor,
        )
        .expect("typed actor VM execution");
        let call: Value = serde_json::from_str(&response).expect("function call JSON");
        assert_eq!(call["name"], "continue_job");
        assert_eq!(call["arguments"]["job_id"], "task-991");
        assert_eq!(
            execute_operator_page(
                &page,
                &[ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Identifier,
                }],
                "Continue the job",
                &payload,
            ),
            Err(OperatorVmError::UnsupportedRenderer)
        );
    }

    #[test]
    fn page_transform_order_drives_rich_rendering() {
        let selectors = [
            ResponseValueSelector::JsonField {
                field: "failed".to_owned(),
                value_type: AtomValueType::Integer,
            },
            ResponseValueSelector::JsonField {
                field: "total".to_owned(),
                value_type: AtomValueType::Integer,
            },
        ];
        let program = ResponseProgram::project_selected_value(
            selectors[0].clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_value_renderer(CollectionOutputRenderer::RenderSequence {
            segments: vec![
                ResponseRenderSegment::Static {
                    text: "failed=".to_owned(),
                },
                ResponseRenderSegment::Primary,
                ResponseRenderSegment::Static {
                    text: "; total=".to_owned(),
                },
                ResponseRenderSegment::Selected {
                    selector: selectors[1].clone(),
                    format: ValueProjectionFormat::PlainText,
                },
            ],
        });
        let renderer = match &program.operation {
            crate::ResponseOperation::ProjectSelectedValue { renderer, .. } => renderer,
            _ => unreachable!("projection program"),
        };
        let page = page(&[transform(0, 1), transform(1, 0)], renderer);

        assert_eq!(
            execute_operator_page(
                &page,
                &selectors,
                "Return failed then total",
                &json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "{\"total\":7,\"failed\":2}"
                    }]
                }),
            )
            .as_deref(),
            Ok("failed=2; total=7")
        );
    }

    #[test]
    fn page_count_transform_executes_collection_law() {
        let selector = ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Collection,
        };
        let page = page(
            &[count_transform(0)],
            &CollectionOutputRenderer::RenderTemplate {
                prefix: "Total records: ".to_owned(),
                suffix: ".".to_owned(),
            },
        );

        assert_eq!(
            execute_operator_page(
                &page,
                &[selector],
                "Count the records",
                &json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "[{\"id\":1},{\"id\":2},{\"id\":3}]"
                    }]
                }),
            )
            .as_deref(),
            Ok("Total records: 3.")
        );
    }

    #[test]
    fn page_count_transform_rejects_non_collection_operand() {
        let selector = ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Collection,
        };
        let page = page(&[count_transform(0)], &CollectionOutputRenderer::Direct);

        assert_eq!(
            execute_operator_page(
                &page,
                &[selector],
                "Count the records",
                &json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "3"
                    }]
                }),
            ),
            Err(OperatorVmError::ProjectionFailed)
        );
    }

    #[test]
    fn page_status_transform_executes_mapping_law() {
        let selector = ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        };
        let page = page(
            &[status_transform(0, TRANSFORM_STATUS_ZERO_IS_OK)],
            &CollectionOutputRenderer::RenderTemplate {
                prefix: "Status: ".to_owned(),
                suffix: ".".to_owned(),
            },
        );
        let payload = |code| {
            json!({
                "input": [{
                    "type": "function_call_output",
                    "output": format!("{{\"code\":{code}}}")
                }]
            })
        };

        assert_eq!(
            execute_operator_page(
                &page,
                std::slice::from_ref(&selector),
                "Check status",
                &payload(0)
            )
            .as_deref(),
            Ok("Status: OK.")
        );
        assert_eq!(
            execute_operator_page(&page, &[selector], "Check status", &payload(7)).as_deref(),
            Ok("Status: ERROR.")
        );
    }

    #[test]
    fn page_filter_transform_uses_both_bound_roles() {
        let selectors = [
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Collection,
            },
            ResponseValueSelector::RequestLastToken,
        ];
        let page = page(&[filter_transform(0, 1)], &CollectionOutputRenderer::Direct);
        let payload = json!({
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": "Filter active"
                },
                {
                    "type": "function_call_output",
                    "output": "[{\"renamed_kind\":\"active\",\"value\":3},{\"renamed_kind\":\"idle\",\"value\":5}]"
                }
            ]
        });

        assert_eq!(
            execute_operator_page(&page, &selectors, "Filter active", &payload).as_deref(),
            Ok("[{\"renamed_kind\":\"active\",\"value\":3}]")
        );
    }

    #[test]
    fn unknown_page_opcode_is_rejected() {
        let mut unknown = transform(0, 0);
        unknown.opcode = 255;
        let page = page(&[unknown], &CollectionOutputRenderer::Direct);
        let selector = ResponseValueSelector::JsonField {
            field: "total".to_owned(),
            value_type: AtomValueType::Integer,
        };
        assert_eq!(
            execute_operator_page(
                &page,
                &[selector],
                "",
                &json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "{\"total\":7}"
                    }]
                }),
            ),
            Err(OperatorVmError::UnsupportedOpcode)
        );
    }

    #[test]
    fn unknown_renderer_opcode_is_rejected() {
        let transform = transform(0, 0);
        let mut renderer = [0_u8; OPERATOR_PAGE32_RENDERER_BYTES];
        renderer[..4].copy_from_slice(&[RENDERER_VERSION, 2, 77, RENDERER_EMIT]);
        let page = OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 1,
                circuit_fingerprint64: 1,
                verifier_binding_fingerprint64: 2,
                proof_lineage_fingerprint64: 3,
                role_signature_fingerprint64: 4,
                relation_plane_count: 1,
                composition_node_count: 0,
                renderer_instruction_count: 2,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &[StructuralRole16::default(), StructuralRole16::default()],
            &TernaryOperatorCube32::default(),
            &[transform],
            &[0; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &renderer,
        )
        .expect("structurally valid page");
        let selector = ResponseValueSelector::JsonField {
            field: "total".to_owned(),
            value_type: AtomValueType::Integer,
        };

        assert_eq!(
            execute_operator_page(
                &page,
                &[selector],
                "",
                &json!({
                    "input": [{
                        "type": "function_call_output",
                        "output": "{\"total\":7}"
                    }]
                }),
            ),
            Err(OperatorVmError::UnsupportedRenderer)
        );
    }
}
