use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{OperatorPage32, TransformOp8};
use nando_operator_kernel::{
    MAX_PROJECT_STATUS_CODE, OPERATOR_RENDERER_EMIT, OPERATOR_RENDERER_STATIC,
    OPERATOR_RENDERER_TYPED_ACTOR, OPERATOR_RENDERER_VALUE, OPERATOR_RENDERER_VERSION,
    ResponseOperation, ResponseProgram, ResponseValueSelector, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_OPCODE_COUNT_COLLECTION, TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
    TRANSFORM_OPCODE_PROJECT_STATUS, TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR, TRANSFORM_ROLE_NONE,
    TRANSFORM_STATUS_ZERO_IS_OK, TRANSFORM_STATUS_ZERO_IS_PASS, TRANSFORM_STATUS_ZERO_IS_SUCCESS,
    TRANSFORM_STATUS_ZERO_IS_TRUE, TRANSFORM_VALUE_COLLECTION, ValueProjectionFormat,
};
use serde_json::Value;

use crate::{ResponseExecutionStatus, selected_value_with_request};

const OPERATOR_VM_MAX_OUTPUT_BYTES: usize = 16_384;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub enum OperatorVmError {
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
#[doc(hidden)]
pub fn execute_operator_page(
    page: &OperatorPage32,
    selectors: &[ResponseValueSelector],
    request_text: &str,
    provider_payload: &Value,
) -> Result<String, OperatorVmError> {
    execute_operator_page_internal(page, selectors, request_text, provider_payload, None)
}

#[doc(hidden)]
pub fn execute_operator_page_with_actor(
    page: &OperatorPage32,
    selectors: &[ResponseValueSelector],
    request_text: &str,
    provider_payload: &Value,
    actor: &ResponseProgram,
) -> Result<String, OperatorVmError> {
    execute_operator_page_internal(page, selectors, request_text, provider_payload, Some(actor))
}

fn execute_operator_page_internal(
    page: &OperatorPage32,
    selectors: &[ResponseValueSelector],
    request_text: &str,
    provider_payload: &Value,
    actor: Option<&ResponseProgram>,
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
            TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR => source_a,
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
                    }) && rows.iter().any(|row| row.get(*field) == Some(predicate))
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

    let response = if renderer_uses_typed_actor(page)? {
        execute_typed_actor_renderer(
            page,
            actor.ok_or(OperatorVmError::UnsupportedRenderer)?,
            request_text,
            provider_payload,
        )?
    } else {
        execute_renderer(page, &values)?
    };
    if response.is_empty() || response.len() > OPERATOR_VM_MAX_OUTPUT_BYTES {
        return Err(OperatorVmError::OutputBudget);
    }
    Ok(response)
}

fn render_transform_value(
    transform: &TransformOp8,
    value: &Value,
) -> Result<String, OperatorVmError> {
    if transform.opcode == TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR {
        return match transform_format(transform.flags)? {
            ValueProjectionFormat::PlainText => match value {
                Value::String(text) if !text.contains(['\n', '\r']) => Ok(text.clone()),
                Value::Bool(value) => Ok(value.to_string()),
                Value::Number(value) => Ok(value.to_string()),
                _ => Err(OperatorVmError::ProjectionFailed),
            },
            ValueProjectionFormat::CanonicalJson => {
                serde_json::to_string(value).map_err(|_| OperatorVmError::ProjectionFailed)
            }
        };
    }
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) => Ok(number.to_string()),
        Value::Array(_) | Value::Object(_) if transform.flags == TRANSFORM_FLAG_CANONICAL_JSON => {
            serde_json::to_string(value).map_err(|_| OperatorVmError::ProjectionFailed)
        }
        _ => Err(OperatorVmError::ProjectionFailed),
    }
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
        if transform.output == transform.source_a || usize::from(transform.parameter >> 8) != index
        {
            return Err(OperatorVmError::InvalidProgram);
        }
        if transform.source_b != TRANSFORM_ROLE_NONE
            && (transform.source_b == transform.output || transform.source_b == transform.source_a)
        {
            return Err(OperatorVmError::InvalidProgram);
        }
        for source in [transform.source_a, transform.source_b] {
            if source != TRANSFORM_ROLE_NONE
                && outputs
                    .get(&source)
                    .is_some_and(|producer| *producer >= index)
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
        || program[0] != OPERATOR_RENDERER_VERSION
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
            OPERATOR_RENDERER_STATIC => {
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
            OPERATOR_RENDERER_VALUE => {
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
            OPERATOR_RENDERER_EMIT => {
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

fn renderer_uses_typed_actor(page: &OperatorPage32) -> Result<bool, OperatorVmError> {
    let header = page.header().map_err(|_| OperatorVmError::InvalidPage)?;
    let program = page.renderer_program();
    if program[0] != OPERATOR_RENDERER_VERSION {
        return Err(OperatorVmError::InvalidProgram);
    }
    Ok(header.renderer_instruction_count == 2
        && program[1] == 2
        && program[2] == OPERATOR_RENDERER_TYPED_ACTOR
        && program[3] == OPERATOR_RENDERER_EMIT)
}

fn execute_typed_actor_renderer(
    page: &OperatorPage32,
    actor: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Result<String, OperatorVmError> {
    if !renderer_uses_typed_actor(page)?
        || !matches!(
            &actor.operation,
            ResponseOperation::FunctionCallFromRoles { .. }
                | ResponseOperation::CustomToolCallFromRoles { .. }
        )
    {
        return Err(OperatorVmError::UnsupportedRenderer);
    }
    let execution = crate::execute_response_unverified(actor, request_text, provider_payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return Err(OperatorVmError::ProjectionFailed);
    }
    execution.response.ok_or(OperatorVmError::ProjectionFailed)
}

fn transform_format(flags: u16) -> Result<ValueProjectionFormat, OperatorVmError> {
    match flags {
        0 => Ok(ValueProjectionFormat::PlainText),
        TRANSFORM_FLAG_CANONICAL_JSON => Ok(ValueProjectionFormat::CanonicalJson),
        _ => Err(OperatorVmError::InvalidProgram),
    }
}
