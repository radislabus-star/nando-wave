use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{OPERATOR_PAGE32_RENDERER_BYTES, OperatorPage32, TransformOp8};
use serde_json::Value;

use crate::crystallized_operator::{
    TRANSFORM_FLAG_CANONICAL_JSON, TRANSFORM_OPCODE_COUNT_COLLECTION,
    TRANSFORM_OPCODE_FILTER_REQUEST_VALUE, TRANSFORM_OPCODE_PROJECT_STATUS,
    TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE, TRANSFORM_STATUS_ZERO_IS_OK,
    TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS, TRANSFORM_STATUS_ZERO_IS_TRUE,
    TRANSFORM_VALUE_COLLECTION,
};
use crate::runtime::selected_value_with_request;
use crate::{
    CollectionOutputRenderer, MAX_PROJECT_STATUS_CODE, ResponseRenderSegment,
    ResponseValueSelector, ValueProjectionFormat,
};

const OPERATOR_VM_MAX_OUTPUT_BYTES: usize = 16_384;
const RENDERER_VERSION: u8 = 1;
const RENDERER_STATIC: u8 = 1;
const RENDERER_VALUE: u8 = 2;
const RENDERER_EMIT: u8 = 255;

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
    request_text: &str,
    provider_payload: &Value,
) -> Result<String, OperatorVmError> {
    page.validate().map_err(|_| OperatorVmError::InvalidPage)?;
    let transforms = decode_transforms(page)?;
    let produced_roles = transforms
        .iter()
        .map(|transform| transform.output)
        .collect::<BTreeSet<_>>();
    let mut external_roles = Vec::new();
    for transform in &transforms {
        for role in [transform.source_a, transform.source_b] {
            if role != TRANSFORM_ROLE_NONE
                && !produced_roles.contains(&role)
                && !external_roles.contains(&role)
            {
                external_roles.push(role);
            }
        }
    }
    if external_roles.len() != selectors.len() {
        return Err(OperatorVmError::MissingOperand);
    }
    let mut role_values = BTreeMap::<u8, Value>::new();
    for (role, selector) in external_roles.into_iter().zip(selectors) {
        let selected = selected_value_with_request(request_text, provider_payload, selector)
            .map_err(|_| OperatorVmError::ProjectionFailed)?;
        role_values.insert(role, selected.value);
    }
    let mut values = Vec::with_capacity(transforms.len());
    for transform in &transforms {
        let source_a = role_values
            .get(&transform.source_a)
            .cloned()
            .ok_or(OperatorVmError::MissingOperand)?;
        let output = match transform.opcode {
            TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR => {
                source_a
            }
            TRANSFORM_OPCODE_COUNT_COLLECTION
                if transform.flags == 0
                    && transform.parameter & 0x00ff == TRANSFORM_VALUE_COLLECTION =>
            {
                let count = match source_a {
                    Value::Array(items) => items.len(),
                    Value::Object(fields) => {
                        let mut arrays = fields.values().filter_map(Value::as_array);
                        let count = arrays
                            .next()
                            .ok_or(OperatorVmError::ProjectionFailed)?
                            .len();
                        if arrays.next().is_some() {
                            return Err(OperatorVmError::ProjectionFailed);
                        }
                        count
                    }
                    _ => return Err(OperatorVmError::ProjectionFailed),
                };
                Value::Number(serde_json::Number::from(count as u64))
            }
            TRANSFORM_OPCODE_PROJECT_STATUS => {
                let code = source_a
                    .as_u64()
                    .filter(|code| *code <= MAX_PROJECT_STATUS_CODE)
                    .ok_or(OperatorVmError::ProjectionFailed)?;
                Value::String(match (transform.flags, code == 0) {
                    (TRANSFORM_STATUS_ZERO_IS_SUCCESS, true) => "success".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_SUCCESS, false) => "failure".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_PASS, true) => "PASS".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_PASS, false) => "FAIL".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_OK, true) => "OK".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_OK, false) => "ERROR".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_TRUE, true) => "true".to_owned(),
                    (TRANSFORM_STATUS_ZERO_IS_TRUE, false) => "false".to_owned(),
                    _ => return Err(OperatorVmError::InvalidProgram),
                })
            }
            TRANSFORM_OPCODE_FILTER_REQUEST_VALUE => {
                let predicate = role_values
                    .get(&transform.source_b)
                    .ok_or(OperatorVmError::MissingOperand)?;
                let fields = source_a
                    .as_object()
                    .ok_or(OperatorVmError::ProjectionFailed)?;
                let mut arrays = fields.values().filter_map(Value::as_array);
                let rows = arrays.next().ok_or(OperatorVmError::ProjectionFailed)?;
                if arrays.next().is_some() || rows.is_empty() || rows.len() > 1_024 {
                    return Err(OperatorVmError::ProjectionFailed);
                }
                let first = rows[0]
                    .as_object()
                    .ok_or(OperatorVmError::ProjectionFailed)?;
                let mut matching_fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&predicate))
                });
                let field = matching_fields
                    .next()
                    .ok_or(OperatorVmError::ProjectionFailed)?;
                if matching_fields.next().is_some() {
                    return Err(OperatorVmError::AmbiguousResponse);
                }
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(field) == Some(predicate))
                        .cloned()
                        .collect(),
                )
            }
            _ => return Err(OperatorVmError::UnsupportedOpcode),
        };
        role_values.insert(transform.output, output.clone());
        values.push(render_transform_value(transform, &output)?);
    }

    let response = execute_renderer(page, &values)?;
    if response.is_empty() || response.len() > OPERATOR_VM_MAX_OUTPUT_BYTES {
        return Err(OperatorVmError::OutputBudget);
    }
    Ok(response)
}

fn render_transform_value(transform: &TransformOp8, value: &Value) -> Result<String, OperatorVmError> {
    if transform.opcode == TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR {
        return match transform_format(transform.flags)? {
            ValueProjectionFormat::PlainText => match value {
                Value::String(text) if !text.contains(['\n', '\r']) => Ok(text.clone()),
                Value::Bool(value) => Ok(value.to_string()),
                Value::Number(value) => Ok(value.to_string()),
                _ => Err(OperatorVmError::ProjectionFailed),
            },
            ValueProjectionFormat::CanonicalJson =>
                serde_json::to_string(value).map_err(|_| OperatorVmError::ProjectionFailed),
        };
    }
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Array(_) | Value::Object(_) if transform.flags == TRANSFORM_FLAG_CANONICAL_JSON =>
            serde_json::to_string(value).map_err(|_| OperatorVmError::ProjectionFailed),
        _ => Err(OperatorVmError::ProjectionFailed),
    }
}

pub(crate) fn encode_renderer_program(
    renderer: &CollectionOutputRenderer,
    transforms: &[TransformOp8],
) -> Result<([u8; OPERATOR_PAGE32_RENDERER_BYTES], u8), crate::CrystallizedOperatorError> {
    let mut instructions = Vec::new();
    let mut instruction_count = 0_u8;
    let mut next_value = 0_usize;
    match renderer {
        CollectionOutputRenderer::Direct => {
            push_value(&mut instructions, unique_sink_transform(transforms)?)?;
            instruction_count = instruction_count.saturating_add(1);
        }
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            instruction_count =
                instruction_count.saturating_add(push_static(&mut instructions, prefix)?);
            push_value(&mut instructions, unique_sink_transform(transforms)?)?;
            instruction_count = instruction_count.saturating_add(1);
            instruction_count =
                instruction_count.saturating_add(push_static(&mut instructions, suffix)?);
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { text } => {
                        instruction_count =
                            instruction_count.saturating_add(push_static(&mut instructions, text)?);
                    }
                    ResponseRenderSegment::Primary => {
                        push_value(&mut instructions, next_value)?;
                        instruction_count = instruction_count.saturating_add(1);
                        next_value = next_value.saturating_add(1);
                    }
                    ResponseRenderSegment::Selected { format, .. } => {
                        let transform = transforms
                            .get(next_value)
                            .ok_or(crate::CrystallizedOperatorError::UnsupportedTransformProgram)?;
                        if *format
                            != transform_format(transform.flags).map_err(|_| {
                                crate::CrystallizedOperatorError::UnsupportedTransformProgram
                            })?
                        {
                            return Err(
                                crate::CrystallizedOperatorError::UnsupportedTransformProgram,
                            );
                        }
                        push_value(&mut instructions, next_value)?;
                        instruction_count = instruction_count.saturating_add(1);
                        next_value = next_value.saturating_add(1);
                    }
                }
            }
            if next_value != transforms.len() {
                return Err(crate::CrystallizedOperatorError::UnsupportedTransformProgram);
            }
        }
        CollectionOutputRenderer::RequestTemplate { .. } => {
            return Err(crate::CrystallizedOperatorError::UnsupportedTransformProgram);
        }
    }
    if matches!(
        renderer,
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RenderTemplate { .. }
    ) && unique_sink_transform(transforms).is_err() {
        return Err(crate::CrystallizedOperatorError::UnsupportedTransformProgram);
    }
    instructions.push(RENDERER_EMIT);
    instruction_count = instruction_count.saturating_add(1);
    if instructions.len().saturating_add(2) > OPERATOR_PAGE32_RENDERER_BYTES
        || instruction_count == 0
    {
        return Err(crate::CrystallizedOperatorError::RendererMismatch);
    }
    let mut encoded = [0_u8; OPERATOR_PAGE32_RENDERER_BYTES];
    encoded[0] = RENDERER_VERSION;
    encoded[1] = instruction_count;
    encoded[2..2 + instructions.len()].copy_from_slice(&instructions);
    Ok((encoded, instruction_count))
}

fn unique_sink_transform(transforms: &[TransformOp8]) -> Result<usize, crate::CrystallizedOperatorError> {
    let consumed = transforms
        .iter()
        .flat_map(|transform| [transform.source_a, transform.source_b])
        .filter(|role| *role != TRANSFORM_ROLE_NONE)
        .collect::<BTreeSet<_>>();
    let mut sinks = transforms
        .iter()
        .enumerate()
        .filter_map(|(index, transform)| (!consumed.contains(&transform.output)).then_some(index));
    let sink = sinks
        .next()
        .ok_or(crate::CrystallizedOperatorError::UnsupportedTransformProgram)?;
    sinks
        .next()
        .is_none()
        .then_some(sink)
        .ok_or(crate::CrystallizedOperatorError::UnsupportedTransformProgram)
}

fn push_static(
    instructions: &mut Vec<u8>,
    text: &str,
) -> Result<u8, crate::CrystallizedOperatorError> {
    if text.is_empty() {
        return Ok(0);
    }
    let bytes = text.as_bytes();
    let len = u8::try_from(bytes.len())
        .map_err(|_| crate::CrystallizedOperatorError::RendererMismatch)?;
    instructions.push(RENDERER_STATIC);
    instructions.push(len);
    instructions.extend_from_slice(bytes);
    Ok(1)
}

fn push_value(
    instructions: &mut Vec<u8>,
    index: usize,
) -> Result<(), crate::CrystallizedOperatorError> {
    let index =
        u8::try_from(index).map_err(|_| crate::CrystallizedOperatorError::RendererMismatch)?;
    instructions.extend_from_slice(&[RENDERER_VALUE, index]);
    Ok(())
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

    let mut outputs = BTreeMap::new();
    for (index, transform) in transforms.iter().enumerate() {
        if outputs.insert(transform.output, index).is_some() {
            return Err(OperatorVmError::InvalidProgram);
        }
    }
    for (index, transform) in transforms.iter().enumerate() {
        if transform.output == transform.source_a
            || usize::from(transform.parameter >> 8) != index
        {
            return Err(OperatorVmError::InvalidProgram);
        }
        if transform.source_b != TRANSFORM_ROLE_NONE
            && (transform.source_b == transform.output
                || transform.source_b == transform.source_a)
        {
            return Err(OperatorVmError::InvalidProgram);
        }
        for source in [transform.source_a, transform.source_b] {
            if source != TRANSFORM_ROLE_NONE
                && outputs.get(&source).is_some_and(|producer| *producer >= index)
            {
                return Err(OperatorVmError::InvalidProgram);
            }
        }
    }
    Ok(transforms)
}

fn execute_renderer(page: &OperatorPage32, values: &[String]) -> Result<String, OperatorVmError> {
    let header = page.header().map_err(|_| OperatorVmError::InvalidPage)?;
    let program = page.renderer_program();
    let instruction_count = usize::from(header.renderer_instruction_count);
    if instruction_count == 0
        || program[0] != RENDERER_VERSION
        || usize::from(program[1]) != instruction_count
    {
        return Err(OperatorVmError::InvalidProgram);
    }

    let mut cursor = 2_usize;
    let mut output = String::new();
    let mut used_values = BTreeSet::new();
    let mut emitted = false;
    for instruction_index in 0..instruction_count {
        let opcode = *program.get(cursor).ok_or(OperatorVmError::InvalidProgram)?;
        cursor = cursor.saturating_add(1);
        match opcode {
            RENDERER_STATIC => {
                if emitted {
                    return Err(OperatorVmError::InvalidProgram);
                }
                let len = usize::from(*program.get(cursor).ok_or(OperatorVmError::InvalidProgram)?);
                cursor = cursor.saturating_add(1);
                let end = cursor
                    .checked_add(len)
                    .filter(|end| *end <= program.len())
                    .ok_or(OperatorVmError::InvalidProgram)?;
                let text = std::str::from_utf8(&program[cursor..end])
                    .map_err(|_| OperatorVmError::InvalidProgram)?;
                output.push_str(text);
                cursor = end;
            }
            RENDERER_VALUE => {
                if emitted {
                    return Err(OperatorVmError::InvalidProgram);
                }
                let index =
                    usize::from(*program.get(cursor).ok_or(OperatorVmError::InvalidProgram)?);
                cursor = cursor.saturating_add(1);
                if !used_values.insert(index) {
                    return Err(OperatorVmError::AmbiguousResponse);
                }
                output.push_str(values.get(index).ok_or(OperatorVmError::MissingOperand)?);
            }
            RENDERER_EMIT => {
                if emitted || instruction_index + 1 != instruction_count {
                    return Err(OperatorVmError::InvalidProgram);
                }
                emitted = true;
            }
            _ => return Err(OperatorVmError::UnsupportedRenderer),
        }
        if output.len() > OPERATOR_VM_MAX_OUTPUT_BYTES {
            return Err(OperatorVmError::OutputBudget);
        }
    }
    if !emitted || program[cursor..].iter().any(|b| *b != 0) {
        return Err(OperatorVmError::InvalidProgram);
    }
    Ok(output)
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
    use crate::{AtomValueType, ResponseProgram};

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
            .flat_map(|transform| {
                [transform.output, transform.source_a, transform.source_b]
            })
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
