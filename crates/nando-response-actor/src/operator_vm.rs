use std::collections::BTreeSet;

use nando_core::wave::{OperatorPage32, TransformOp8};
use serde_json::Value;

use crate::crystallized_operator::{
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE,
};
use crate::runtime::project_selected_value_with_request;
use crate::{
    CollectionOutputRenderer, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, ValueProjectionFormat,
};

const OPERATOR_VM_MAX_OUTPUT_BYTES: usize = 16_384;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperatorVmError {
    InvalidPage,
    InvalidProgram,
    MissingOperand,
    UnsupportedOpcode,
    UnsupportedRenderer,
    ProjectionFailed,
    AmbiguousResponse,
    OutputBudget,
}

/// Executes crystallized transform bytecode. Runtime selectors are operands
/// produced by role grounding; selectors embedded in the legacy actor are not
/// consulted and therefore cannot override the circuit-selected source roles.
pub(crate) fn execute_operator_page(
    page: &OperatorPage32,
    selectors: &[ResponseValueSelector],
    renderer_program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Result<String, OperatorVmError> {
    page.validate().map_err(|_| OperatorVmError::InvalidPage)?;
    let transforms = decode_transforms(page)?;
    if transforms.len() != selectors.len() {
        return Err(OperatorVmError::MissingOperand);
    }

    let mut values = Vec::with_capacity(transforms.len());
    for (transform, selector) in transforms.iter().zip(selectors) {
        if transform.opcode != TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR {
            return Err(OperatorVmError::UnsupportedOpcode);
        }
        let format = if transform.flags == 0 {
            ValueProjectionFormat::PlainText
        } else if transform.flags == TRANSFORM_FLAG_CANONICAL_JSON {
            ValueProjectionFormat::CanonicalJson
        } else {
            return Err(OperatorVmError::InvalidProgram);
        };
        values.push(
            project_selected_value_with_request(request_text, provider_payload, selector, format)
                .map_err(|_| OperatorVmError::ProjectionFailed)?,
        );
    }

    let response = render_program(renderer_program, &values, &transforms)?;
    if response.is_empty()
        || response.len() > renderer_program.max_output_bytes
        || response.len() > OPERATOR_VM_MAX_OUTPUT_BYTES
    {
        return Err(OperatorVmError::OutputBudget);
    }
    Ok(response)
}

fn decode_transforms(page: &OperatorPage32) -> Result<Vec<TransformOp8>, OperatorVmError> {
    let count = usize::from(
        page.header()
            .map_err(|_| OperatorVmError::InvalidPage)?
            .transform_count,
    );
    if count == 0 {
        return Err(OperatorVmError::InvalidProgram);
    }
    let mut transforms = (0..count)
        .map(|index| page.transform(index).ok_or(OperatorVmError::InvalidPage))
        .collect::<Result<Vec<_>, _>>()?;
    transforms.sort_by_key(|transform| transform.parameter >> 8);

    let output = transforms[0].output;
    let mut sources = BTreeSet::new();
    for (index, transform) in transforms.iter().enumerate() {
        if transform.source_b != TRANSFORM_ROLE_NONE
            || transform.output != output
            || transform.output == transform.source_a
            || usize::from(transform.parameter >> 8) != index
            || !sources.insert(transform.source_a)
        {
            return Err(OperatorVmError::InvalidProgram);
        }
    }
    Ok(transforms)
}

fn render_program(
    program: &ResponseProgram,
    values: &[String],
    transforms: &[TransformOp8],
) -> Result<String, OperatorVmError> {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. } => {
            render_values(renderer, values, transforms)
        }
        ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut responses = BTreeSet::new();
            for variant in variants {
                if let Ok(response) = render_program(&variant.program, values, transforms) {
                    responses.insert(response);
                }
            }
            if responses.len() != 1 {
                return Err(OperatorVmError::AmbiguousResponse);
            }
            responses
                .into_iter()
                .next()
                .ok_or(OperatorVmError::AmbiguousResponse)
        }
        _ => Err(OperatorVmError::UnsupportedRenderer),
    }
}

fn render_values(
    renderer: &CollectionOutputRenderer,
    values: &[String],
    transforms: &[TransformOp8],
) -> Result<String, OperatorVmError> {
    match renderer {
        CollectionOutputRenderer::Direct => match values {
            [value] => Ok(value.clone()),
            _ => Err(OperatorVmError::UnsupportedRenderer),
        },
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => match values {
            [value] => Ok(format!("{prefix}{value}{suffix}")),
            _ => Err(OperatorVmError::UnsupportedRenderer),
        },
        CollectionOutputRenderer::RenderSequence { segments } => {
            let mut output = String::new();
            let mut next_value = 0_usize;
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { text } => output.push_str(text),
                    ResponseRenderSegment::Primary => {
                        let value = values
                            .get(next_value)
                            .ok_or(OperatorVmError::MissingOperand)?;
                        output.push_str(value);
                        next_value += 1;
                    }
                    ResponseRenderSegment::Selected { format, .. } => {
                        let value = values
                            .get(next_value)
                            .ok_or(OperatorVmError::MissingOperand)?;
                        let expected = transform_format(
                            transforms
                                .get(next_value)
                                .ok_or(OperatorVmError::MissingOperand)?
                                .flags,
                        )?;
                        if *format != expected {
                            return Err(OperatorVmError::InvalidProgram);
                        }
                        output.push_str(value);
                        next_value += 1;
                    }
                }
                if output.len() > OPERATOR_VM_MAX_OUTPUT_BYTES {
                    return Err(OperatorVmError::OutputBudget);
                }
            }
            if next_value != values.len() {
                return Err(OperatorVmError::MissingOperand);
            }
            Ok(output)
        }
        CollectionOutputRenderer::RequestTemplate { .. } => {
            Err(OperatorVmError::UnsupportedRenderer)
        }
    }
}

fn transform_format(flags: u16) -> Result<ValueProjectionFormat, OperatorVmError> {
    match flags {
        0 => Ok(ValueProjectionFormat::PlainText),
        TRANSFORM_FLAG_CANONICAL_JSON => Ok(ValueProjectionFormat::CanonicalJson),
        _ => Err(OperatorVmError::InvalidProgram),
    }
}

#[cfg(test)]
mod tests {
    use nando_core::wave::{
        OPERATOR_PAGE32_COMPOSITION_BYTES, OPERATOR_PAGE32_PHASE_BYTES,
        OPERATOR_PAGE32_RENDERER_BYTES, OperatorPage32Metadata, StructuralRole16,
        TernaryOperatorCube32,
    };
    use serde_json::json;

    use super::*;
    use crate::AtomValueType;

    fn page(transforms: &[TransformOp8]) -> OperatorPage32 {
        OperatorPage32::build(
            OperatorPage32Metadata {
                generation: 1,
                circuit_fingerprint64: 1,
                verifier_binding_fingerprint64: 2,
                proof_lineage_fingerprint64: 3,
                role_signature_fingerprint64: 4,
                relation_plane_count: 1,
                composition_node_count: 0,
                renderer_instruction_count: 0,
                flags: 0,
            },
            &[0; OPERATOR_PAGE32_PHASE_BYTES],
            &[
                StructuralRole16::default(),
                StructuralRole16::default(),
                StructuralRole16::default(),
            ],
            &TernaryOperatorCube32::default(),
            transforms,
            &[0; OPERATOR_PAGE32_COMPOSITION_BYTES],
            &[0; OPERATOR_PAGE32_RENDERER_BYTES],
        )
        .expect("valid test page")
    }

    fn transform(source: u8, order: u16) -> TransformOp8 {
        TransformOp8 {
            opcode: TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR,
            output: 2,
            source_a: source,
            source_b: TRANSFORM_ROLE_NONE,
            parameter: (order << 8) | 1,
            flags: 0,
        }
    }

    #[test]
    fn page_transform_order_drives_rich_rendering() {
        let page = page(&[transform(0, 1), transform(1, 0)]);
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

        assert_eq!(
            execute_operator_page(
                &page,
                &selectors,
                &program,
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
    fn unknown_page_opcode_is_rejected() {
        let mut unknown = transform(0, 0);
        unknown.opcode = 255;
        let page = page(&[unknown]);
        let selector = ResponseValueSelector::JsonField {
            field: "total".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );

        assert_eq!(
            execute_operator_page(
                &page,
                &[selector],
                &program,
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
}
