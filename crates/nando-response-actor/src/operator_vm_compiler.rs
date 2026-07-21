use std::collections::BTreeSet;

use nando_core::wave::{OPERATOR_PAGE32_RENDERER_BYTES, TransformOp8};
use nando_operator_kernel::{
    OPERATOR_RENDERER_EMIT, OPERATOR_RENDERER_STATIC, OPERATOR_RENDERER_TYPED_ACTOR,
    OPERATOR_RENDERER_VALUE, OPERATOR_RENDERER_VERSION,
};

use crate::{
    CollectionOutputRenderer, ResponseRenderSegment, TRANSFORM_FLAG_CANONICAL_JSON,
    TRANSFORM_ROLE_NONE, ValueProjectionFormat,
};

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
    ) && unique_sink_transform(transforms).is_err()
    {
        return Err(crate::CrystallizedOperatorError::UnsupportedTransformProgram);
    }
    instructions.push(OPERATOR_RENDERER_EMIT);
    instruction_count = instruction_count.saturating_add(1);
    if instructions.len().saturating_add(2) > OPERATOR_PAGE32_RENDERER_BYTES
        || instruction_count == 0
    {
        return Err(crate::CrystallizedOperatorError::RendererMismatch);
    }
    let mut encoded = [0_u8; OPERATOR_PAGE32_RENDERER_BYTES];
    encoded[0] = OPERATOR_RENDERER_VERSION;
    encoded[1] = instruction_count;
    encoded[2..2 + instructions.len()].copy_from_slice(&instructions);
    Ok((encoded, instruction_count))
}

pub(crate) fn encode_typed_actor_renderer_program(
    transforms: &[TransformOp8],
) -> Result<([u8; OPERATOR_PAGE32_RENDERER_BYTES], u8), crate::CrystallizedOperatorError> {
    if transforms.len() != 1 || unique_sink_transform(transforms).is_err() {
        return Err(crate::CrystallizedOperatorError::UnsupportedTransformProgram);
    }
    let mut encoded = [0_u8; OPERATOR_PAGE32_RENDERER_BYTES];
    encoded[..4].copy_from_slice(&[
        OPERATOR_RENDERER_VERSION,
        2,
        OPERATOR_RENDERER_TYPED_ACTOR,
        OPERATOR_RENDERER_EMIT,
    ]);
    Ok((encoded, 2))
}

fn unique_sink_transform(
    transforms: &[TransformOp8],
) -> Result<usize, crate::CrystallizedOperatorError> {
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
    instructions.push(OPERATOR_RENDERER_STATIC);
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
    instructions.extend_from_slice(&[OPERATOR_RENDERER_VALUE, index]);
    Ok(())
}

fn transform_format(flags: u16) -> Result<ValueProjectionFormat, ()> {
    match flags {
        0 => Ok(ValueProjectionFormat::PlainText),
        TRANSFORM_FLAG_CANONICAL_JSON => Ok(ValueProjectionFormat::CanonicalJson),
        _ => Err(()),
    }
}
