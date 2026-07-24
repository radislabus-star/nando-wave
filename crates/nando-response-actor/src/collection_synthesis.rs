use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

pub use nando_operator_learning::{
    CollectionSynthesisExample, CollectionVersionSpace, ResponseCoverageDiagnostic,
    SynthesizedCollectionProgram,
};

use crate::{
    AtomValueType, CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, MAX_RESPONSE_STATIC_TEXT_BYTES, OutputGraphSegment, OutputValueCandidate,
    OutputValueSource, ProjectStatusMapping, RequestTemplateMarker, ResponseExecution,
    ResponseExecutionStatus, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseScalarLiteral, ResponseValueSelector, ValueProjectionFormat, VerifierProgram,
    build_output_graph, collection_static_text_rejection_reason, execute_response,
};

const MAX_SEARCH_ROWS: usize = 1_024;
const MAX_CANDIDATES: usize = 16_384;
const MAX_TURN_COLLECTION_OUTPUTS: usize = 16;

fn execute_example(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> ResponseExecution {
    let request_text = synthesis_request_text(&example.provider_payload).unwrap_or_default();
    execute_response(program, &request_text, &example.provider_payload)
}

fn render_policy_rejected_sequence(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Option<String> {
    let segments = match &program.operation {
        ResponseOperation::ProjectSelectedValue {
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        }
        | ResponseOperation::ProjectStatus {
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        }
        | ResponseOperation::ComposeCollection {
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        } => segments.clone(),
        _ => return None,
    };
    let mut direct = program.clone();
    match &mut direct.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => {
            *renderer = CollectionOutputRenderer::Direct;
        }
        _ => return None,
    }
    let primary = execute_example(&direct, example).response?;
    let mut output = String::new();
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => output.push_str(&text),
            ResponseRenderSegment::Primary => output.push_str(&primary),
            ResponseRenderSegment::Selected { selector, format } => {
                let selected =
                    ResponseProgram::project_selected_value(selector, format, "completed");
                output.push_str(&execute_example(&selected, example).response?);
            }
        }
        if output.len() > program.max_output_bytes {
            return None;
        }
    }
    Some(output)
}

#[must_use]
pub fn diagnose_response_dynamic_coverage(
    example: &CollectionSynthesisExample,
) -> ResponseCoverageDiagnostic {
    let mut dynamic = vec![false; example.expected_response.len()];
    let mut request_dynamic = vec![false; example.expected_response.len()];
    let mut tool_dynamic = vec![false; example.expected_response.len()];
    let mut matching_selectors = 0_usize;
    for selector in learned_selector_candidates(&example.provider_payload) {
        let program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let Some(value) = execute_example(&program, example).response else {
            continue;
        };
        if value.is_empty() || value.len() > example.expected_response.len() {
            continue;
        }
        let request_derived = matches!(
            selector,
            ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral
        );
        let mut matched = false;
        for (offset, _) in example.expected_response.match_indices(&value) {
            let end = offset.saturating_add(value.len()).min(dynamic.len());
            dynamic[offset..end].fill(true);
            if request_derived {
                request_dynamic[offset..end].fill(true);
            } else {
                tool_dynamic[offset..end].fill(true);
            }
            matched = true;
        }
        matching_selectors = matching_selectors.saturating_add(usize::from(matched));
    }
    ResponseCoverageDiagnostic {
        response_bytes: example.expected_response.len(),
        dynamic_bytes: dynamic.into_iter().filter(|covered| *covered).count(),
        request_dynamic_bytes: request_dynamic
            .into_iter()
            .filter(|covered| *covered)
            .count(),
        tool_dynamic_bytes: tool_dynamic.into_iter().filter(|covered| *covered).count(),
        matching_selectors,
        exact_surface_required: synthesis_request_text(&example.provider_payload)
            .is_some_and(|request| request_requires_exact_surface(&request)),
    }
}

pub fn enumerate_source_neutral_collection_programs(
    example: &CollectionSynthesisExample,
) -> Result<CollectionVersionSpace, &'static str> {
    let candidates = expand_output_renderers(
        enumerate_collection_source_candidates(&example.provider_payload)?,
        example,
    );
    let candidates_enumerated = candidates.len();
    let mut exact_checks = 0_usize;
    let mut semantic_classes = BTreeMap::new();
    for program in candidates {
        if !is_source_neutral_collection_program(&program) {
            continue;
        }
        exact_checks = exact_checks.saturating_add(1);
        let execution = execute_example(&program, example);
        if execution.status != ResponseExecutionStatus::Executed
            || execution.response.as_deref() != Some(example.expected_response.as_str())
            || program.validate().is_err()
        {
            continue;
        }
        let key = collection_semantic_key(&program)?;
        semantic_classes.entry(key).or_insert(program);
    }
    let programs = semantic_classes.into_values().collect::<Vec<_>>();
    Ok(CollectionVersionSpace {
        programs,
        exact_checks,
        candidates_enumerated,
        policy_rejected_exact_matches: 0,
        policy_rejection_reasons: BTreeMap::new(),
        static_text_rejection_reasons: BTreeMap::new(),
        canonical_rejection_reasons: BTreeMap::new(),
    })
}

pub fn enumerate_source_neutral_response_programs(
    example: &CollectionSynthesisExample,
) -> Result<CollectionVersionSpace, &'static str> {
    enumerate_source_neutral_response_programs_with_coverage(example, None)
}

pub(crate) fn enumerate_source_neutral_response_programs_with_coverage(
    example: &CollectionSynthesisExample,
    coverage: Option<ResponseCoverageDiagnostic>,
) -> Result<CollectionVersionSpace, &'static str> {
    let mut candidates = Vec::new();
    let renderer_context = OutputRendererContext::from_example(example);
    let selectors = learned_selector_candidates(&example.provider_payload);
    let observed_dynamic_values = canonical_render_values(example, &selectors)
        .unwrap_or_default()
        .into_iter()
        .map(|candidate| candidate.rendered)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if let Ok(collection) = enumerate_collection_source_candidates(&example.provider_payload) {
        candidates.extend(compose_collection_render_sequence_candidates(
            example,
            &collection,
            &selectors,
        ));
        candidates.extend(expand_output_renderers_with_context(
            collection,
            example,
            &renderer_context,
        ));
    }
    let mut policy_rejected_exact_matches = 0_usize;
    let mut policy_rejection_reasons = BTreeMap::<String, usize>::new();
    let mut static_text_rejection_reasons = BTreeMap::<String, usize>::new();
    let mut canonical_rejection_reasons = BTreeMap::<String, usize>::new();
    let mut structurally_aligned_canonical = BTreeSet::<Vec<u8>>::new();
    for selector in &selectors {
        let value_type = selector_value_type(selector);
        let plain_program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let plain_response = execute_example(&plain_program, example)
            .response
            .filter(|computed| !computed.is_empty());
        if let Some(computed) = plain_response.as_deref() {
            candidates.extend(expand_output_renderer_with_computed(
                plain_program,
                computed,
                example,
                &renderer_context,
            ));
        }
        let canonical_max_bytes = plain_response.as_ref().map_or(0, |plain| {
            if value_type == AtomValueType::String {
                plain.len().saturating_mul(6).saturating_add(2)
            } else {
                plain.len()
            }
        });
        if renderer_context.allow_canonical_direct
            || !renderer_context.request_markers.is_empty()
            || example.expected_response.len()
                <= canonical_max_bytes.saturating_add(MAX_RESPONSE_STATIC_TEXT_BYTES)
        {
            let canonical_program = ResponseProgram::project_selected_value(
                selector.clone(),
                ValueProjectionFormat::CanonicalJson,
                "completed",
            );
            if value_type == AtomValueType::String {
                if let Some(computed) = execute_example(&canonical_program, example)
                    .response
                    .filter(|computed| !computed.is_empty())
                {
                    candidates.extend(expand_output_renderer_with_computed(
                        canonical_program,
                        &computed,
                        example,
                        &renderer_context,
                    ));
                }
            } else if let Some(computed) = plain_response.as_deref() {
                candidates.extend(expand_output_renderer_with_computed(
                    canonical_program,
                    computed,
                    example,
                    &renderer_context,
                ));
            }
        }
        let status_may_match = renderer_context.allow_canonical_direct
            || !renderer_context.request_markers.is_empty()
            || example.expected_response.len() <= MAX_RESPONSE_STATIC_TEXT_BYTES + 32;
        if value_type == AtomValueType::Integer && status_may_match {
            for mapping in [
                ProjectStatusMapping::ZeroIsSuccess,
                ProjectStatusMapping::ZeroIsPass,
                ProjectStatusMapping::ZeroIsOk,
                ProjectStatusMapping::ZeroIsTrue,
            ] {
                candidates.extend(expand_output_renderers_with_context(
                    vec![ResponseProgram::project_status(
                        selector.clone(),
                        mapping,
                        "completed",
                    )],
                    example,
                    &renderer_context,
                ));
            }
        }
    }
    let selector_sequence_possible = coverage.is_none_or(|coverage| {
        coverage
            .response_bytes
            .saturating_sub(coverage.dynamic_bytes)
            <= MAX_RESPONSE_STATIC_TEXT_BYTES
    });
    for program in
        compose_render_sequence_candidates(example, &selectors, selector_sequence_possible)
    {
        if let Err(reason) = program.validate() {
            let rejected_exact = render_policy_rejected_sequence(&program, example)
                .is_some_and(|response| response == example.expected_response);
            if rejected_exact {
                policy_rejected_exact_matches = policy_rejected_exact_matches.saturating_add(1);
                *policy_rejection_reasons
                    .entry(reason.to_owned())
                    .or_default() += 1;
                if reason == "unsafe_render_sequence_static_text" {
                    if let Some(detail) = response_program_static_text_rejection_reason(&program) {
                        *static_text_rejection_reasons.entry(detail).or_default() += 1;
                    }
                    if let Ok(canonical) = canonical_direct_response_program(&program) {
                        structurally_aligned_canonical
                            .insert(serde_json::to_vec(&canonical).unwrap_or_default());
                        if response_program_match_quality_with_alignment(&canonical, example, true)
                            == 0
                        {
                            *canonical_rejection_reasons
                                .entry(response_program_match_rejection_reason(&canonical, example))
                                .or_default() += 1;
                        }
                        candidates.push(canonical);
                    }
                }
            }
        }
        candidates.push(program);
    }
    candidates.sort_by_key(|program| serde_json::to_vec(program).unwrap_or_default());
    candidates.dedup();
    if candidates.len() > MAX_CANDIDATES {
        return Err("collection_candidate_budget");
    }
    let candidates_enumerated = candidates.len();
    let mut exact_checks = 0_usize;
    let mut semantic_classes = BTreeMap::new();
    for program in candidates {
        if response_program_static_renderer_captures_dynamic_value(
            &program,
            &observed_dynamic_values,
        ) {
            if execute_example(&program, example).response.as_deref()
                == Some(example.expected_response.as_str())
            {
                policy_rejected_exact_matches = policy_rejected_exact_matches.saturating_add(1);
                *policy_rejection_reasons
                    .entry("static_renderer_captures_dynamic_value".to_owned())
                    .or_default() += 1;
            }
            continue;
        }
        if !is_learned_bounded_response_program(&program) {
            continue;
        }
        exact_checks = exact_checks.saturating_add(1);
        let structurally_aligned = structurally_aligned_canonical
            .contains(&serde_json::to_vec(&program).unwrap_or_default());
        let match_quality =
            response_program_match_quality_with_alignment(&program, example, structurally_aligned);
        if match_quality == 0 {
            continue;
        }
        let entry = semantic_classes
            .entry(response_semantic_key(&program)?)
            .or_insert((match_quality, program.clone()));
        if match_quality > entry.0 {
            *entry = (match_quality, program);
        }
    }
    let mut programs = semantic_classes.into_values().collect::<Vec<_>>();
    programs.sort_by(|left, right| {
        right.0.cmp(&left.0).then_with(|| {
            serde_json::to_vec(&left.1)
                .unwrap_or_default()
                .cmp(&serde_json::to_vec(&right.1).unwrap_or_default())
        })
    });
    Ok(CollectionVersionSpace {
        programs: programs.into_iter().map(|(_, program)| program).collect(),
        exact_checks,
        candidates_enumerated,
        policy_rejected_exact_matches,
        policy_rejection_reasons,
        static_text_rejection_reasons,
        canonical_rejection_reasons,
    })
}

pub(crate) fn enumerate_source_neutral_structural_response_programs(
    example: &CollectionSynthesisExample,
) -> Result<Vec<ResponseProgram>, &'static str> {
    let selectors = learned_selector_candidates(&example.provider_payload);
    let coverage = diagnose_response_dynamic_coverage(example);
    let include_surface_renderer = coverage
        .response_bytes
        .saturating_sub(coverage.dynamic_bytes)
        <= MAX_RESPONSE_STATIC_TEXT_BYTES;
    let mut candidates =
        compose_render_sequence_candidates(example, &selectors, include_surface_renderer);
    candidates.sort_by_key(|program| serde_json::to_vec(program).unwrap_or_default());
    candidates.dedup();

    let mut semantic_classes = BTreeMap::new();
    for program in candidates {
        let dynamic_roles = match &program.operation {
            ResponseOperation::ProjectSelectedValue {
                renderer: CollectionOutputRenderer::RenderSequence { segments },
                ..
            } => segments
                .iter()
                .filter(|segment| !matches!(segment, ResponseRenderSegment::Static { .. }))
                .count(),
            _ => 0,
        };
        if dynamic_roles < 2
            || !is_learned_bounded_response_program(&program)
            || response_program_match_quality(&program, example) == 0
        {
            continue;
        }
        semantic_classes
            .entry(response_semantic_key(&program)?)
            .or_insert(program);
    }
    Ok(semantic_classes.into_values().collect())
}

#[must_use]
pub fn response_program_matches_example(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    response_program_match_quality(program, example) > 0
}

/// Returns whether a completed teacher trace can train this structural law.
///
/// A structurally aligned canonical response may pass here even when its text
/// differs from the teacher response. This is not admission authority: frozen
/// support and future must independently prove the complete teacher output.
#[must_use]
pub fn response_program_authority_matches_example(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    let quality = response_program_match_quality(program, example);
    if quality == 2 {
        return true;
    }
    if quality != 1 {
        return false;
    }
    let Some(response) = execute_example(program, example).response else {
        return false;
    };
    if response_program_has_structurally_grounded_partial_authority(program) {
        return true;
    }
    mentioned_tool_scalars(example)
        .iter()
        .all(|scalar| contains_token(&response, scalar))
}

fn response_program_has_structurally_grounded_partial_authority(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue {
            selector:
                ResponseValueSelector::RequestReferencedJsonField { .. }
                | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. },
            ..
        }
        | ResponseOperation::ProjectStatus {
            selector:
                ResponseValueSelector::RequestReferencedJsonField { .. }
                | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. },
            ..
        }
        | ResponseOperation::ComposeCollection { .. } => true,
        ResponseOperation::UniqueConsensus {
            adapter_wave: Some(_),
            ..
        } => {
            is_source_neutral_response_program(program)
                && is_privacy_safe_online_response_program(program)
                && is_learned_bounded_response_program(program)
        }
        ResponseOperation::UniqueConsensus { variants, .. } => {
            !variants.is_empty()
                && variants.iter().all(|variant| {
                    response_program_has_structurally_grounded_partial_authority(&variant.program)
                })
        }
        _ => false,
    }
}

#[must_use]
pub fn response_program_exactly_matches_example(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    response_program_match_quality(program, example) == 2
}

fn response_program_match_quality(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> u8 {
    // 0 = incompatible, 1 = structurally aligned teacher law, 2 = exact output.
    let quality = response_program_match_quality_with_alignment(program, example, false);
    if quality == 0 && response_program_has_structural_output_alignment(program, example) {
        1
    } else {
        quality
    }
}

fn response_program_match_quality_with_alignment(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
    structurally_aligned: bool,
) -> u8 {
    let execution = execute_example(program, example);
    if execution.status != ResponseExecutionStatus::Executed {
        return 0;
    }
    let Some(response) = execution.response.as_deref() else {
        return 0;
    };
    if response == example.expected_response {
        return 2;
    }
    if let ResponseOperation::UniqueConsensus { variants, .. } = &program.operation {
        return u8::from(variants.iter().any(|variant| {
            let variant_execution = execute_example(&variant.program, example);
            variant_execution.status == ResponseExecutionStatus::Executed
                && variant_execution.response.as_deref() == Some(response)
                && response_program_match_quality(&variant.program, example) == 1
        }));
    }
    let direct_derived = matches!(
        &program.operation,
        ResponseOperation::ProjectStatus {
            renderer: CollectionOutputRenderer::Direct,
            ..
        } | ResponseOperation::ComposeCollection {
            renderer: CollectionOutputRenderer::Direct,
            ..
        }
    );
    let direct_projection = matches!(
        &program.operation,
        ResponseOperation::ProjectSelectedValue {
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            ..
        }
    );
    let canonical_dynamic_sequence = matches!(
        &program.operation,
        ResponseOperation::ProjectSelectedValue {
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::RenderSequence { segments },
            ..
        } if segments
            .iter()
            .filter(|segment| matches!(
                segment,
                ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. }
            ))
            .count() >= 2
            && segments.iter().all(|segment| match segment {
                ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => true,
                ResponseRenderSegment::Static { text } => text == "\n",
            })
    );
    if !direct_derived && !direct_projection && !canonical_dynamic_sequence {
        return u8::from(structurally_aligned);
    }
    if response.is_empty()
        || response.len() > 512
        || synthesis_request_text(&example.provider_payload)
            .is_some_and(|request| request_requires_exact_surface(&request))
    {
        return 0;
    }
    if canonical_dynamic_sequence {
        let values = response.split('\n').collect::<Vec<_>>();
        let mentioned = mentioned_tool_scalars(example);
        if !(2..=16).contains(&values.len())
            || values.iter().any(|value| value.is_empty())
            || mentioned.is_empty()
            || mentioned
                .iter()
                .any(|scalar| !values.contains(&scalar.as_str()))
        {
            return 0;
        }
        if values
            .iter()
            .any(|value| !contains_token(&example.expected_response, value))
            && !structurally_aligned
        {
            return 0;
        }
        return 1;
    }
    if token_occurrences(&example.expected_response, response) != 1 {
        return u8::from(structurally_aligned);
    }
    if direct_projection
        && !tool_outputs(&example.provider_payload)
            .into_iter()
            .filter_map(bounded_collection_output_text)
            .any(|output| contains_token(&output, response))
    {
        return 0;
    }
    1
}

fn response_program_has_structural_output_alignment(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    let selectors = learned_selector_candidates(&example.provider_payload);
    let coverage = diagnose_response_dynamic_coverage(example);
    let include_surface_renderer = coverage
        .response_bytes
        .saturating_sub(coverage.dynamic_bytes)
        <= MAX_RESPONSE_STATIC_TEXT_BYTES;
    compose_render_sequence_candidates(example, &selectors, include_surface_renderer)
        .into_iter()
        .filter(|surface| {
            (response_program_requires_static_frame_transfer(surface)
                || surface.validate() == Err("unsafe_render_sequence_static_text"))
                && render_policy_rejected_sequence(surface, example)
                    .is_some_and(|response| response == example.expected_response)
        })
        .filter_map(|surface| canonical_direct_response_program(&surface).ok())
        .any(|canonical| canonical == *program)
}

fn response_program_match_rejection_reason(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> String {
    let execution = execute_example(program, example);
    if execution.status != ResponseExecutionStatus::Executed {
        return format!("execution:{}", execution.reason);
    }
    let Some(response) = execution.response.as_deref() else {
        return "missing_response".to_owned();
    };
    if response.is_empty() {
        return "empty_response".to_owned();
    }
    if response.len() > 512 {
        return "response_over_512".to_owned();
    }
    if synthesis_request_text(&example.provider_payload)
        .is_some_and(|request| request_requires_exact_surface(&request))
    {
        return "exact_surface_required".to_owned();
    }
    if let ResponseOperation::ProjectSelectedValue {
        renderer: CollectionOutputRenderer::RenderSequence { segments },
        ..
    } = &program.operation
        && segments.iter().all(|segment| match segment {
            ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => true,
            ResponseRenderSegment::Static { text } => text == "\n",
        })
    {
        let values = response.split('\n').collect::<Vec<_>>();
        if !(2..=16).contains(&values.len()) {
            return "dynamic_value_count".to_owned();
        }
        if values.iter().any(|value| value.is_empty()) {
            return "empty_dynamic_value".to_owned();
        }
        if values
            .iter()
            .any(|value| !contains_token(&example.expected_response, value))
        {
            return "dynamic_value_missing_from_teacher".to_owned();
        }
        let mentioned = mentioned_tool_scalars(example);
        if mentioned.is_empty() {
            return "no_teacher_mentioned_tool_scalar".to_owned();
        }
        if mentioned
            .iter()
            .any(|scalar| !values.contains(&scalar.as_str()))
        {
            return "missing_teacher_mentioned_tool_scalar".to_owned();
        }
    }
    if token_occurrences(&example.expected_response, response) != 1 {
        return "projected_value_not_unique_in_teacher".to_owned();
    }
    "unsupported_partial_shape".to_owned()
}

fn mentioned_tool_scalars(example: &CollectionSynthesisExample) -> Vec<String> {
    let mut scalars = Vec::new();
    for output in tool_outputs(&example.provider_payload) {
        if let Some(value) = unique_embedded_json_output(output) {
            collect_json_scalars(&value, &mut scalars);
        }
    }
    scalars.sort();
    scalars.dedup();
    scalars.retain(|scalar| contains_token(&example.expected_response, scalar));
    scalars.sort_by(|left, right| {
        example
            .expected_response
            .find(left)
            .cmp(&example.expected_response.find(right))
            .then_with(|| left.cmp(right))
    });
    scalars
}

fn tool_outputs(payload: &Value) -> Vec<&Value> {
    payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output"))
        .collect()
}

fn collect_json_scalars(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Null => output.push("null".to_owned()),
        Value::Bool(value) => output.push(value.to_string()),
        Value::Number(value) => output.push(value.to_string()),
        Value::String(value) if !value.is_empty() && value.len() <= 128 => {
            output.push(value.clone());
        }
        Value::Array(values) => {
            for value in values {
                collect_json_scalars(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_json_scalars(value, output);
            }
        }
        Value::String(_) => {}
    }
}

fn contains_token(haystack: &str, needle: &str) -> bool {
    haystack.match_indices(needle).any(|(start, _)| {
        let end = start.saturating_add(needle.len());
        let left_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        let right_ok = haystack[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        left_ok && right_ok
    })
}

fn token_occurrences(haystack: &str, needle: &str) -> usize {
    haystack
        .match_indices(needle)
        .filter(|(start, _)| {
            let end = start.saturating_add(needle.len());
            let left_ok = haystack[..*start]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_alphanumeric() && character != '_');
            let right_ok = haystack[end..]
                .chars()
                .next()
                .is_none_or(|character| !character.is_alphanumeric() && character != '_');
            left_ok && right_ok
        })
        .count()
}

fn request_requires_exact_surface(request: &str) -> bool {
    let lower = request.to_lowercase();
    let literal_markers = [
        "exactly",
        "verbatim",
        "reply only",
        "respond only",
        "answer only",
        "ровно",
        "дословно",
        "ответь только",
        "верни только",
        "выведи только",
    ];
    let structured_surface_markers = [
        "return json",
        "respond with json",
        "reply with json",
        "output json",
        "json only",
        "valid json",
        "as json",
        "in json format",
        "json schema",
        "return yaml",
        "yaml only",
        "as yaml",
        "в формате json",
        "в json-формате",
        "ответ в json",
        "верни json",
        "выведи json",
        "только json",
        "в формате yaml",
        "только yaml",
        "формат ответа",
        "соблюдай формат",
        "по этой схеме",
        "по следующей схеме",
    ];
    literal_markers
        .iter()
        .chain(structured_surface_markers.iter())
        .any(|term| lower.contains(term))
}

fn compose_render_sequence_candidates(
    example: &CollectionSynthesisExample,
    selectors: &[ResponseValueSelector],
    include_surface_renderer: bool,
) -> Vec<ResponseProgram> {
    let Some(resolved) = canonical_render_values(example, selectors) else {
        return Vec::new();
    };
    if resolved.is_empty() {
        return Vec::new();
    }
    let mut programs = canonical_dynamic_sequence_candidates(example, &resolved);
    if !include_surface_renderer {
        return programs;
    }
    let Ok(graph) = build_output_graph(&example.expected_response, resolved) else {
        return programs;
    };
    let dynamic_count = graph
        .segments
        .iter()
        .filter(|segment| matches!(segment, OutputGraphSegment::RuntimeValue { .. }))
        .count();
    if dynamic_count == 0 {
        return Vec::new();
    }
    let primary_selectors = graph
        .segments
        .iter()
        .filter_map(|segment| match segment {
            OutputGraphSegment::RuntimeValue { sources, .. } => Some(sources),
            OutputGraphSegment::Static { .. } => None,
        })
        .flatten()
        .filter_map(|source| match source {
            OutputValueSource::Selector(selector) => Some(selector.clone()),
            OutputValueSource::Primary => None,
        })
        .collect::<BTreeSet<_>>();
    for primary_selector in primary_selectors.into_iter().take(64) {
        let mut segments = Vec::new();
        let mut complete = true;
        for segment in &graph.segments {
            match segment {
                OutputGraphSegment::Static { text } => {
                    segments.push(ResponseRenderSegment::Static { text: text.clone() });
                }
                OutputGraphSegment::RuntimeValue { sources, .. } => {
                    if sources.iter().any(|source| {
                        matches!(source, OutputValueSource::Selector(selector) if selector == &primary_selector)
                    }) {
                        segments.push(ResponseRenderSegment::Primary);
                    } else if let Some(selector) = sources.iter().find_map(|source| match source {
                        OutputValueSource::Selector(selector) => Some(selector.clone()),
                        OutputValueSource::Primary => None,
                    }) {
                        segments.push(ResponseRenderSegment::Selected {
                            selector,
                            format: ValueProjectionFormat::PlainText,
                        });
                    } else {
                        complete = false;
                        break;
                    }
                }
            }
        }
        if complete {
            programs.push(
                ResponseProgram::project_selected_value(
                    primary_selector,
                    ValueProjectionFormat::PlainText,
                    "completed",
                )
                .with_value_renderer(CollectionOutputRenderer::RenderSequence { segments }),
            );
        }
    }
    programs
}

fn canonical_dynamic_sequence_candidates(
    example: &CollectionSynthesisExample,
    resolved: &[OutputValueCandidate],
) -> Vec<ResponseProgram> {
    let mentioned = mentioned_tool_scalars(example);
    if !(2..=16).contains(&mentioned.len()) {
        return Vec::new();
    }
    let mut selectors_by_value = BTreeMap::<&str, BTreeSet<ResponseValueSelector>>::new();
    for candidate in resolved {
        let OutputValueSource::Selector(selector) = &candidate.source else {
            continue;
        };
        if mentioned.iter().any(|value| value == &candidate.rendered)
            && token_occurrences(&example.expected_response, &candidate.rendered) == 1
        {
            selectors_by_value
                .entry(candidate.rendered.as_str())
                .or_default()
                .insert(selector.clone());
        }
    }
    if mentioned
        .iter()
        .any(|value| !selectors_by_value.contains_key(value.as_str()))
    {
        return Vec::new();
    }

    let mut programs = Vec::new();
    for (primary_index, primary_value) in mentioned.iter().enumerate() {
        for primary_selector in selectors_by_value[primary_value.as_str()].iter() {
            let mut segments = Vec::with_capacity(mentioned.len() * 2 - 1);
            for (index, value) in mentioned.iter().enumerate() {
                if index > 0 {
                    segments.push(ResponseRenderSegment::Static {
                        text: "\n".to_owned(),
                    });
                }
                if index == primary_index {
                    segments.push(ResponseRenderSegment::Primary);
                } else {
                    let selector = selectors_by_value[value.as_str()]
                        .iter()
                        .next()
                        .expect("all mentioned values have a selector")
                        .clone();
                    segments.push(ResponseRenderSegment::Selected {
                        selector,
                        format: ValueProjectionFormat::PlainText,
                    });
                }
            }
            programs.push(
                ResponseProgram::project_selected_value(
                    primary_selector.clone(),
                    ValueProjectionFormat::PlainText,
                    "completed",
                )
                .with_value_renderer(CollectionOutputRenderer::RenderSequence { segments }),
            );
            if programs.len() == 64 {
                return programs;
            }
        }
    }
    programs
}

fn compose_collection_render_sequence_candidates(
    example: &CollectionSynthesisExample,
    programs: &[ResponseProgram],
    selectors: &[ResponseValueSelector],
) -> Vec<ResponseProgram> {
    let Some(selected_values) = canonical_render_values(example, selectors) else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for program in programs {
        let Some(computed) = execute_example(program, example).response else {
            continue;
        };
        if computed.is_empty() || !example.expected_response.contains(&computed) {
            continue;
        }
        let mut values = Vec::with_capacity(selected_values.len().saturating_add(1));
        values.push(OutputValueCandidate {
            source: OutputValueSource::Primary,
            rendered: computed,
        });
        values.extend(selected_values.iter().cloned());
        let Ok(graph) = build_output_graph(&example.expected_response, values) else {
            continue;
        };
        if graph.source_ambiguous {
            continue;
        }
        let dynamic_count = graph
            .segments
            .iter()
            .filter(|segment| matches!(segment, OutputGraphSegment::RuntimeValue { .. }))
            .count();
        let contains_primary = graph.segments.iter().any(|segment| {
            matches!(
                segment,
                OutputGraphSegment::RuntimeValue { sources, .. }
                    if sources == &[OutputValueSource::Primary]
            )
        });
        if dynamic_count < 2 || !contains_primary {
            continue;
        }
        let mut segments = Vec::with_capacity(graph.segments.len());
        let mut valid = true;
        for segment in graph.segments {
            match segment {
                OutputGraphSegment::Static { text } => {
                    segments.push(ResponseRenderSegment::Static { text });
                }
                OutputGraphSegment::RuntimeValue { sources, .. } => match sources.as_slice() {
                    [OutputValueSource::Primary] => segments.push(ResponseRenderSegment::Primary),
                    [OutputValueSource::Selector(selector)] => {
                        segments.push(ResponseRenderSegment::Selected {
                            selector: selector.clone(),
                            format: ValueProjectionFormat::PlainText,
                        });
                    }
                    _ => {
                        valid = false;
                        break;
                    }
                },
            }
        }
        if valid {
            output.push(
                program.clone().with_collection_renderer(
                    CollectionOutputRenderer::RenderSequence { segments },
                ),
            );
        }
    }
    output
}

fn canonical_render_values(
    example: &CollectionSynthesisExample,
    selectors: &[ResponseValueSelector],
) -> Option<Vec<OutputValueCandidate>> {
    let mut by_value = BTreeMap::<String, Vec<ResponseValueSelector>>::new();
    for selector in selectors {
        let program = ResponseProgram::project_selected_value(
            selector.clone(),
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let Some(value) = execute_example(&program, example).response else {
            continue;
        };
        if (value.len() < 2 && selector_value_type(selector) == AtomValueType::String)
            || !example.expected_response.contains(&value)
        {
            continue;
        }
        by_value.entry(value).or_default().push(selector.clone());
    }
    let mut values = Vec::new();
    for (rendered, selectors) in by_value {
        for selector in selectors.into_iter().take(64) {
            values.push(OutputValueCandidate {
                source: OutputValueSource::Selector(selector),
                rendered: rendered.clone(),
            });
        }
    }
    (!values.is_empty()).then_some(values)
}

pub(crate) fn learned_selector_candidates(payload: &Value) -> Vec<ResponseValueSelector> {
    nando_operator_runtime::selector_candidates(payload)
}

const fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
    match selector {
        ResponseValueSelector::ContinuationHandle { value_type }
        | ResponseValueSelector::UniqueScalar { value_type }
        | ResponseValueSelector::UniqueTurnScalar { value_type }
        | ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | ResponseValueSelector::JsonField { value_type, .. }
        | ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | ResponseValueSelector::RequestReferencedJsonField { value_type }
        | ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | ResponseValueSelector::TurnOutputLine { value_type, .. }
        | ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => *value_type,
        ResponseValueSelector::CommandOutputBody
        | ResponseValueSelector::RequestLastToken
        | ResponseValueSelector::RequestUniqueLiteral => AtomValueType::String,
    }
}

#[must_use]
pub fn is_source_neutral_collection_program(program: &ResponseProgram) -> bool {
    let crate::ResponseOperation::ComposeCollection { steps, .. } = &program.operation else {
        return false;
    };
    steps.iter().all(|step| match step {
        CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. } => {
            is_source_neutral_request_selector(selector)
        }
        CollectionProgramStep::SelectTurnOutput { .. }
        | CollectionProgramStep::SelectOnlyArrayField
        | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
        | CollectionProgramStep::ProjectUniqueFieldByType { .. }
        | CollectionProgramStep::ProjectOnlyNonFilterField
        | CollectionProgramStep::AggregateUniqueIntegerField { .. }
        | CollectionProgramStep::Count => true,
        _ => false,
    })
}

// These selectors describe a structural position or type. Field names and JSON
// pointers are deliberately excluded so a learned filter survives renaming.
fn source_neutral_selector(selector: &ResponseValueSelector) -> bool {
    matches!(
        selector,
        ResponseValueSelector::UniqueScalar { .. }
            | ResponseValueSelector::UniqueTurnScalar { .. }
            | ResponseValueSelector::RequestReferencedJsonField { .. }
            | ResponseValueSelector::TurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
            | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. }
            | ResponseValueSelector::LatestTurnOutputLine { .. }
            | ResponseValueSelector::RequestLastToken
            | ResponseValueSelector::RequestUniqueLiteral
    )
}

#[must_use]
pub(crate) fn is_source_neutral_request_selector(selector: &ResponseValueSelector) -> bool {
    nando_operator_runtime::is_source_neutral_request_selector(selector)
}

#[must_use]
pub fn is_source_neutral_response_program(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave: Some(_),
        } => {
            program.validate().is_ok()
                && !variants.is_empty()
                && variants.iter().all(|variant| {
                    !matches!(
                        variant.program.operation,
                        ResponseOperation::UniqueConsensus { .. }
                    ) && variant.program.validate().is_ok()
                })
        }
        ResponseOperation::UniqueConsensus { variants, .. } => {
            !variants.is_empty()
                && variants
                    .iter()
                    .all(|variant| is_source_neutral_response_program(&variant.program))
        }
        ResponseOperation::ProjectSelectedValue {
            selector, renderer, ..
        }
        | ResponseOperation::ProjectStatus {
            selector, renderer, ..
        } => source_neutral_selector(selector) && response_renderer_is_surface_neutral(renderer),
        ResponseOperation::ComposeCollection { renderer, .. } => {
            is_source_neutral_collection_program(program)
                && response_renderer_is_surface_neutral(renderer)
        }
        _ => false,
    }
}

/// Returns whether a learned program can enter transfer-bound identification.
///
/// A response frame may contain stable prose, but it remains surface-bound
/// until adaptive future evidence proves the same frame over new dynamic
/// values. This predicate grants no package or execution authority.
#[must_use]
pub fn is_transfer_bound_response_program(program: &ResponseProgram) -> bool {
    if program.validate().is_err() || !is_learned_bounded_response_program(program) {
        return false;
    }
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            !variants.is_empty()
                && variants
                    .iter()
                    .all(|variant| is_transfer_bound_response_program(&variant.program))
        }
        ResponseOperation::ProjectSelectedValue { selector, .. }
        | ResponseOperation::ProjectStatus { selector, .. } => source_neutral_selector(selector),
        ResponseOperation::ComposeCollection { .. } => {
            is_source_neutral_collection_program(program)
        }
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => true,
        _ => false,
    }
}

fn response_renderer_is_surface_neutral(renderer: &CollectionOutputRenderer) -> bool {
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RequestTemplate { .. } => true,
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            static_renderer_text_is_surface_neutral(prefix)
                && static_renderer_text_is_surface_neutral(suffix)
        }
        CollectionOutputRenderer::RenderSequence { segments } => segments.iter().all(|segment| {
            !matches!(
                segment,
                ResponseRenderSegment::Static { text }
                    if !static_renderer_text_is_surface_neutral(text)
            )
        }),
    }
}

fn response_program_static_renderer_captures_dynamic_value(
    program: &ResponseProgram,
    observed_dynamic_values: &BTreeSet<String>,
) -> bool {
    let renderer = match &program.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return false,
    };
    let captures = |text: &str| {
        observed_dynamic_values
            .iter()
            .any(|value| contains_token(text, value))
    };
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RequestTemplate { .. } => {
            false
        }
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            captures(prefix) || captures(suffix)
        }
        CollectionOutputRenderer::RenderSequence { segments } => segments.iter().any(
            |segment| matches!(segment, ResponseRenderSegment::Static { text } if captures(text)),
        ),
    }
}

fn response_program_static_text_rejection_reason(program: &ResponseProgram) -> Option<String> {
    let renderer = match &program.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return None,
    };
    let static_text = match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RequestTemplate { .. } => None,
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            Some(format!("{prefix}{suffix}"))
        }
        CollectionOutputRenderer::RenderSequence { segments } => Some(
            segments
                .iter()
                .filter_map(|segment| match segment {
                    ResponseRenderSegment::Static { text } => Some(text.as_str()),
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => None,
                })
                .collect::<String>(),
        ),
    }?;
    let reason = collection_static_text_rejection_reason(&static_text, "")?;
    if reason != "byte_budget" {
        return Some(reason.to_owned());
    }
    let bucket = match static_text.len() {
        0..=1_024 => "byte_budget_0513_1024",
        1_025..=2_048 => "byte_budget_1025_2048",
        2_049..=3_072 => "byte_budget_2049_3072",
        3_073..=4_096 => "byte_budget_3073_4096",
        _ => "byte_budget_over_4096",
    };
    Some(bucket.to_owned())
}

#[must_use]
pub fn response_program_requires_static_frame_transfer(program: &ResponseProgram) -> bool {
    let renderer = match &program.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        ResponseOperation::UniqueConsensus { variants, .. } => {
            return variants
                .iter()
                .any(|variant| response_program_requires_static_frame_transfer(&variant.program));
        }
        _ => return false,
    };
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RequestTemplate { .. } => {
            false
        }
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => prefix
            .chars()
            .chain(suffix.chars())
            .any(char::is_alphanumeric),
        CollectionOutputRenderer::RenderSequence { segments } => segments.iter().any(|segment| {
            matches!(
                segment,
                ResponseRenderSegment::Static { text }
                    if text.chars().any(char::is_alphanumeric)
            )
        }),
    }
}

pub fn response_program_dynamic_value_root_sha256(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> Result<Option<String>, &'static str> {
    if !response_program_requires_static_frame_transfer(program) {
        return Ok(None);
    }
    let renderer = match &program.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        ResponseOperation::UniqueConsensus { .. } => {
            return Err("static_frame_consensus_requires_variant_proof");
        }
        _ => return Err("static_frame_program_kind"),
    };
    let mut direct = program.clone();
    match &mut direct.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => {
            *renderer = CollectionOutputRenderer::Direct;
        }
        _ => return Err("static_frame_program_kind"),
    }
    let primary = execute_example(&direct, example)
        .response
        .ok_or("static_frame_primary_missing")?;
    let mut dynamic_values = Vec::new();
    match renderer {
        CollectionOutputRenderer::RenderTemplate { .. } => dynamic_values.push(primary),
        CollectionOutputRenderer::RenderSequence { segments } => {
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { .. } => {}
                    ResponseRenderSegment::Primary => dynamic_values.push(primary.clone()),
                    ResponseRenderSegment::Selected { selector, format } => {
                        let selected = ResponseProgram::project_selected_value(
                            selector.clone(),
                            *format,
                            "completed",
                        );
                        dynamic_values.push(
                            execute_example(&selected, example)
                                .response
                                .ok_or("static_frame_selected_value_missing")?,
                        );
                    }
                }
            }
        }
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RequestTemplate { .. } => {
            return Err("static_frame_renderer_missing");
        }
    }
    if dynamic_values.is_empty() || dynamic_values.iter().any(String::is_empty) {
        return Err("static_frame_dynamic_values_missing");
    }
    let observed = dynamic_values.iter().cloned().collect::<BTreeSet<_>>();
    if response_program_static_renderer_captures_dynamic_value(program, &observed) {
        return Err("static_frame_captures_dynamic_value");
    }
    crate::canonical_json_sha256(&("nando.static-frame-dynamic-values.v1", dynamic_values))
        .map(Some)
}

fn static_renderer_text_is_surface_neutral(value: &str) -> bool {
    value.chars().all(|character| !character.is_alphanumeric())
}

#[must_use]
pub fn is_privacy_safe_online_response_program(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave: Some(_),
        } => {
            program.validate().is_ok()
                && variants.iter().all(|variant| {
                    !matches!(
                        variant.program.operation,
                        ResponseOperation::UniqueConsensus { .. }
                    ) && variant.program.validate().is_ok()
                })
        }
        ResponseOperation::UniqueConsensus { variants, .. } => {
            program.validate().is_ok()
                && variants
                    .iter()
                    .all(|variant| is_privacy_safe_online_response_program(&variant.program))
        }
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. }
        | ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => program.validate().is_ok(),
        ResponseOperation::ComposeCollection { renderer, .. } => {
            is_source_neutral_collection_program(program)
                && match renderer {
                    CollectionOutputRenderer::Direct
                    | CollectionOutputRenderer::RenderTemplate { .. }
                    | CollectionOutputRenderer::RenderSequence { .. }
                    | CollectionOutputRenderer::RequestTemplate { .. } => {
                        program.validate().is_ok()
                    }
                }
        }
        _ => false,
    }
}

#[must_use]
pub fn is_learned_bounded_response_program(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            !variants.is_empty()
                && variants
                    .iter()
                    .all(|variant| is_learned_bounded_response_program(&variant.program))
        }
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. }
        | ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => true,
        ResponseOperation::ComposeCollection { steps, .. } => steps.iter().all(|step| {
            matches!(
                step,
                CollectionProgramStep::SelectTurnOutput { .. }
                    | CollectionProgramStep::SelectOnlyArrayField
                    | CollectionProgramStep::SelectField { .. }
                    | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                    | CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. }
                    | CollectionProgramStep::ProjectField { .. }
                    | CollectionProgramStep::ProjectUniqueFieldByType { .. }
                    | CollectionProgramStep::ProjectOnlyNonFilterField
                    | CollectionProgramStep::AggregateUniqueIntegerField { .. }
                    | CollectionProgramStep::Count
            )
        }),
        _ => false,
    }
}

pub fn collection_verifier_for_program(
    program: &ResponseProgram,
) -> Result<VerifierProgram, &'static str> {
    let crate::ResponseOperation::ComposeCollection {
        steps,
        format,
        renderer,
        completion_state,
        max_items,
    } = &program.operation
    else {
        return Err("collection_internal_program_kind");
    };
    Ok(VerifierProgram::ComposeCollection {
        steps: steps.clone(),
        format: *format,
        renderer: renderer.clone(),
        completion_state: completion_state.clone(),
        max_items: *max_items,
    })
}

pub fn source_neutral_verifier_for_program(
    program: &ResponseProgram,
) -> Result<VerifierProgram, &'static str> {
    match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave,
        } => Ok(VerifierProgram::UniqueConsensus {
            variants: variants
                .iter()
                .map(|variant| {
                    source_neutral_verifier_for_program(&variant.program).map(|verifier| {
                        crate::VerifierConsensusVariant {
                            verifier,
                            allowed_layout_sha256: variant.allowed_layout_sha256.clone(),
                            required_request_atom_ids: variant.required_request_atom_ids.clone(),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            adapter_wave: adapter_wave.clone(),
        }),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectSelectedValue {
            selector: selector.clone(),
            format: *format,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } => Ok(VerifierProgram::ProjectStatus {
            selector: selector.clone(),
            mapping: *mapping,
            renderer: renderer.clone(),
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ComposeCollection { .. } => collection_verifier_for_program(program),
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => {
            crate::synthesis::compile_independent_verifier(program)
                .map_err(|_| "source_neutral_call_verifier")
        }
        _ => Err("source_neutral_verifier_program_kind"),
    }
}

pub fn synthesize_collection_program(
    support: &[CollectionSynthesisExample],
) -> Result<SynthesizedCollectionProgram, &'static str> {
    synthesize_collection_program_with_policy(support, false)
}

pub fn synthesize_unique_collection_program(
    support: &[CollectionSynthesisExample],
) -> Result<SynthesizedCollectionProgram, &'static str> {
    synthesize_collection_program_with_policy(support, true)
}

fn synthesize_collection_program_with_policy(
    support: &[CollectionSynthesisExample],
    require_unique: bool,
) -> Result<SynthesizedCollectionProgram, &'static str> {
    if support.is_empty() {
        return Err("collection_support_empty");
    }
    let candidates = expand_output_renderers(
        enumerate_collection_source_candidates(&support[0].provider_payload)?,
        &support[0],
    );
    let candidates_enumerated = candidates.len();
    let mut exact_checks = 0_usize;
    let mut survivors = Vec::new();
    for program in candidates {
        let mut consistent = true;
        for example in support {
            exact_checks = exact_checks.saturating_add(1);
            let execution = execute_example(&program, example);
            if execution.status != ResponseExecutionStatus::Executed
                || execution.response.as_deref() != Some(example.expected_response.as_str())
            {
                consistent = false;
                break;
            }
        }
        if consistent {
            let bytes = serde_json::to_vec(&program).map_err(|_| "collection_program_encode")?;
            survivors.push((bytes.len(), bytes, program));
        }
    }
    survivors.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    let mut semantic_classes = BTreeMap::new();
    for survivor in survivors {
        let key = collection_semantic_key(&survivor.2)?;
        semantic_classes.entry(key).or_insert(survivor);
    }
    let mut survivors = semantic_classes.into_values().collect::<Vec<_>>();
    survivors.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
    if require_unique && survivors.len() > 1 {
        return Err("collection_ambiguous_programs");
    }
    let (description_length_bytes, _, program) = survivors
        .into_iter()
        .next()
        .ok_or("collection_no_consistent_program")?;
    let verifier = collection_verifier_for_program(&program)?;
    Ok(SynthesizedCollectionProgram {
        verifier,
        program,
        exact_checks,
        candidates_enumerated,
        description_length_bytes,
    })
}

fn collection_semantic_key(program: &ResponseProgram) -> Result<Vec<u8>, &'static str> {
    let crate::ResponseOperation::ComposeCollection {
        steps,
        format,
        renderer,
        completion_state,
        max_items,
    } = &program.operation
    else {
        return Err("collection_internal_program_kind");
    };
    let effective_format = if matches!(steps.last(), Some(CollectionProgramStep::Count)) {
        ValueProjectionFormat::PlainText
    } else {
        *format
    };
    serde_json::to_vec(&(
        steps,
        effective_format,
        renderer,
        completion_state,
        max_items,
    ))
    .map_err(|_| "collection_semantic_key_encode")
}

fn response_semantic_key(program: &ResponseProgram) -> Result<Vec<u8>, &'static str> {
    match &program.operation {
        ResponseOperation::ComposeCollection { .. } => collection_semantic_key(program),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
        } => {
            let effective_format = match selector {
                ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::String,
                }
                | ResponseValueSelector::UniqueTurnScalar {
                    value_type: AtomValueType::String,
                }
                | ResponseValueSelector::TurnOutputScalarOrdinal {
                    value_type: AtomValueType::String,
                    ..
                }
                | ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                    value_type: AtomValueType::String,
                    ..
                } => *format,
                _ => ValueProjectionFormat::PlainText,
            };
            serde_json::to_vec(&(selector, effective_format, renderer, completion_state))
                .map_err(|_| "response_semantic_key_encode")
        }
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
        } => serde_json::to_vec(&(selector, mapping, renderer, completion_state))
            .map_err(|_| "response_semantic_key_encode"),
        _ => Err("response_semantic_key_kind"),
    }
}

pub(crate) fn canonical_direct_response_program(
    program: &ResponseProgram,
) -> Result<ResponseProgram, &'static str> {
    let mut canonical = program.clone();
    if let Some((selector, format, completion_state)) =
        single_selected_renderer_projection(&canonical.operation)
    {
        canonical.operation = ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer: CollectionOutputRenderer::Direct,
            completion_state,
        };
    } else {
        match &mut canonical.operation {
            ResponseOperation::ProjectSelectedValue { renderer, .. }
            | ResponseOperation::ProjectStatus { renderer, .. }
            | ResponseOperation::ComposeCollection { renderer, .. } => {
                *renderer = canonical_law_renderer(renderer);
            }
            _ => return Err("response_law_key_kind"),
        }
    }
    canonical
        .validate()
        .map_err(|_| "response_law_canonical_invalid")?;
    Ok(canonical)
}

fn single_selected_renderer_projection(
    operation: &ResponseOperation,
) -> Option<(ResponseValueSelector, ValueProjectionFormat, String)> {
    let (renderer, completion_state) = match operation {
        ResponseOperation::ProjectSelectedValue {
            renderer,
            completion_state,
            ..
        }
        | ResponseOperation::ProjectStatus {
            renderer,
            completion_state,
            ..
        }
        | ResponseOperation::ComposeCollection {
            renderer,
            completion_state,
            ..
        } => (renderer, completion_state),
        _ => return None,
    };
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return None;
    };
    let mut dynamic = segments
        .iter()
        .filter(|segment| !matches!(segment, ResponseRenderSegment::Static { .. }));
    let ResponseRenderSegment::Selected { selector, format } = dynamic.next()? else {
        return None;
    };
    dynamic
        .next()
        .is_none()
        .then(|| (selector.clone(), *format, completion_state.clone()))
}

pub(crate) fn response_law_key(program: &ResponseProgram) -> Result<Vec<u8>, &'static str> {
    response_law_key_with_policy(program, false)
}

fn response_law_key_with_policy(
    program: &ResponseProgram,
    include_renderer: bool,
) -> Result<Vec<u8>, &'static str> {
    if let ResponseOperation::UniqueConsensus { variants, .. } = &program.operation {
        let mut keys = variants
            .iter()
            .map(|variant| response_law_key_with_policy(&variant.program, include_renderer));
        let first = keys.next().ok_or("response_law_key_consensus_empty")??;
        for key in keys {
            if key? != first {
                return Err("response_law_key_consensus_mixed");
            }
        }
        return Ok(first);
    }
    let canonical = canonical_direct_response_program(program)?;
    let renderer = match &canonical.operation {
        ResponseOperation::ProjectSelectedValue { renderer, .. }
        | ResponseOperation::ProjectStatus { renderer, .. }
        | ResponseOperation::ComposeCollection { renderer, .. } => renderer,
        _ => return Err("response_law_key_kind"),
    };
    let mut law = match &canonical.operation {
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            completion_state,
            ..
        } => serde_json::json!({
            "kind": "project",
            "value_type": selector_value_type(selector),
            "format": format,
            "completion_state": completion_state,
        }),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            completion_state,
            ..
        } => serde_json::json!({
            "kind": "status",
            "value_type": selector_value_type(selector),
            "mapping": mapping,
            "completion_state": completion_state,
        }),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            completion_state,
            max_items,
            ..
        } => serde_json::json!({
            "kind": "collection",
            "steps": abstract_collection_steps(steps),
            "format": format,
            "completion_state": completion_state,
            "max_items": max_items,
        }),
        _ => return Err("response_law_key_kind"),
    };
    if include_renderer {
        let primary = match &canonical.operation {
            ResponseOperation::ProjectSelectedValue {
                selector, format, ..
            } => Some((selector, *format)),
            ResponseOperation::ProjectStatus { selector, .. } => {
                Some((selector, ValueProjectionFormat::PlainText))
            }
            _ => None,
        };
        law.as_object_mut()
            .ok_or("response_law_key_object")?
            .insert("renderer".to_owned(), renderer_law_shape(renderer, primary));
    }
    serde_json::to_vec(&law).map_err(|_| "response_law_key_encode")
}

fn canonical_law_renderer(renderer: &CollectionOutputRenderer) -> CollectionOutputRenderer {
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return CollectionOutputRenderer::Direct;
    };
    let mut dynamic = Vec::new();
    for segment in segments
        .iter()
        .filter(|segment| !matches!(segment, ResponseRenderSegment::Static { .. }))
    {
        if !dynamic.contains(segment) {
            dynamic.push(segment.clone());
        }
    }
    if dynamic.len() < 2 {
        return CollectionOutputRenderer::Direct;
    }
    let mut canonical = Vec::with_capacity(dynamic.len().saturating_mul(2).saturating_sub(1));
    for (index, segment) in dynamic.into_iter().enumerate() {
        if index > 0 {
            canonical.push(ResponseRenderSegment::Static {
                text: "\n".to_owned(),
            });
        }
        canonical.push(segment);
    }
    CollectionOutputRenderer::RenderSequence {
        segments: canonical,
    }
}

fn renderer_law_shape(
    renderer: &CollectionOutputRenderer,
    primary: Option<(&ResponseValueSelector, ValueProjectionFormat)>,
) -> serde_json::Value {
    match renderer {
        CollectionOutputRenderer::Direct => serde_json::Value::String("direct".to_owned()),
        CollectionOutputRenderer::RenderSequence { segments } => serde_json::Value::Array(
            segments
                .iter()
                .map(|segment| match segment {
                    ResponseRenderSegment::Static { .. } => {
                        serde_json::json!({"separator":"newline"})
                    }
                    ResponseRenderSegment::Primary => primary.map_or_else(
                        || serde_json::json!({"source":"collection"}),
                        |(selector, format)| {
                            serde_json::json!({
                                "source":selector_law_source(selector),
                                "value_type":selector_value_type(selector),
                                "format":format,
                            })
                        },
                    ),
                    ResponseRenderSegment::Selected { selector, format } => serde_json::json!({
                        "source":selector_law_source(selector),
                        "value_type":selector_value_type(selector),
                        "format":format,
                    }),
                })
                .collect(),
        ),
        CollectionOutputRenderer::RenderTemplate { .. }
        | CollectionOutputRenderer::RequestTemplate { .. } => {
            serde_json::Value::String("surface".to_owned())
        }
    }
}

fn selector_law_source(selector: &ResponseValueSelector) -> serde_json::Value {
    match selector {
        ResponseValueSelector::ContinuationHandle { .. } => {
            serde_json::json!({"domain":"observation","role":"continuation_handle"})
        }
        ResponseValueSelector::UniqueScalar { .. } => {
            serde_json::json!({"domain":"observation","role":"unique_scalar"})
        }
        ResponseValueSelector::UniqueTurnScalar { .. } => {
            serde_json::json!({"domain":"observation","role":"unique_turn_scalar"})
        }
        ResponseValueSelector::ContentLinePrefix { .. } => {
            serde_json::json!({"domain":"observation","role":"content_line_prefix"})
        }
        ResponseValueSelector::JsonField { .. } => {
            serde_json::json!({"domain":"observation","role":"json_named_field"})
        }
        ResponseValueSelector::JsonScalarOrdinal { ordinal, .. } => serde_json::json!({
            "domain":"observation","role":"json_scalar_ordinal","ordinal":ordinal
        }),
        ResponseValueSelector::UniqueTurnJsonField { .. } => {
            serde_json::json!({"domain":"observation","role":"unique_turn_json_field"})
        }
        ResponseValueSelector::UniqueActiveTurnJsonField { .. } => serde_json::json!({
            "domain":"observation","role":"unique_active_turn_json_field"
        }),
        ResponseValueSelector::RequestReferencedJsonField { .. } => serde_json::json!({
            "domain":"observation","role":"request_referenced_json_field"
        }),
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal, .. } => {
            serde_json::json!({
                "domain":"observation",
                "role":"request_referenced_json_field_ordinal",
                "ordinal":ordinal
            })
        }
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            ..
        } => serde_json::json!({
            "domain":"observation","role":"turn_output_line",
            "output_ordinal":output_ordinal,"line_index":line_index
        }),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal,
            scalar_ordinal,
            ..
        } => serde_json::json!({
            "domain":"observation","role":"turn_output_scalar_ordinal",
            "output_ordinal":output_ordinal,"scalar_ordinal":scalar_ordinal
        }),
        ResponseValueSelector::LatestTurnOutputLine { line_index, .. } => serde_json::json!({
            "domain":"observation","role":"latest_turn_output_line","line_index":line_index
        }),
        ResponseValueSelector::LatestTurnOutputScalarOrdinal { scalar_ordinal, .. } => {
            serde_json::json!({
                "domain":"observation","role":"latest_turn_output_scalar_ordinal",
                "scalar_ordinal":scalar_ordinal
            })
        }
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal, ..
        } => serde_json::json!({
            "domain":"observation","role":"latest_turn_output_scalar_from_end",
            "reverse_ordinal":reverse_ordinal
        }),
        ResponseValueSelector::CommandOutputBody => {
            serde_json::json!({"domain":"command_output","role":"body"})
        }
        ResponseValueSelector::RequestLastToken => {
            serde_json::json!({"domain":"request","role":"last_token"})
        }
        ResponseValueSelector::RequestUniqueLiteral => {
            serde_json::json!({"domain":"request","role":"unique_literal"})
        }
    }
}

fn abstract_collection_step(step: &CollectionProgramStep) -> serde_json::Value {
    match step {
        CollectionProgramStep::SelectTurnOutput { .. } => {
            serde_json::json!({"step": "select_turn_output"})
        }
        CollectionProgramStep::SelectOnlyArrayField => {
            serde_json::json!({"step": "select_only_array_field"})
        }
        CollectionProgramStep::SelectField { .. } => {
            serde_json::json!({"step": "select_field_role"})
        }
        CollectionProgramStep::FilterUniqueFieldEquals { value } => serde_json::json!({
            "step": "filter_unique_field_equals",
            "value": value,
        }),
        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
            serde_json::json!({
                "step": "filter_unique_field_equals_request_value",
                "value_type": value_type,
            })
        }
        CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
            selector,
            value_type,
        } => serde_json::json!({
            "step": "filter_unique_field_equals_selected_value",
            "selector": selector_law_source(selector),
            "value_type": value_type,
        }),
        CollectionProgramStep::FilterFieldEquals { value, .. } => serde_json::json!({
            "step": "filter_field_role_equals",
            "value": value,
        }),
        CollectionProgramStep::ProjectField { .. } => {
            serde_json::json!({"step": "project_field_role"})
        }
        CollectionProgramStep::ProjectUniqueFieldByType { value_type } => serde_json::json!({
            "step": "project_unique_field_by_type",
            "value_type": value_type,
        }),
        CollectionProgramStep::ProjectOnlyNonFilterField => {
            serde_json::json!({"step": "project_only_non_filter_field"})
        }
        CollectionProgramStep::AggregateUniqueIntegerField { operation } => serde_json::json!({
            "step": "aggregate_unique_integer_field",
            "operation": operation,
        }),
        CollectionProgramStep::Count => serde_json::json!({"step": "count"}),
    }
}

fn abstract_collection_steps(steps: &[CollectionProgramStep]) -> Vec<serde_json::Value> {
    let mut abstracted = steps
        .iter()
        .map(abstract_collection_step)
        .collect::<Vec<_>>();
    if !matches!(
        steps.first(),
        Some(CollectionProgramStep::SelectTurnOutput { .. })
    ) {
        abstracted.insert(0, serde_json::json!({"step": "select_turn_output"}));
    }
    abstracted
}

fn expand_output_renderers(
    candidates: Vec<ResponseProgram>,
    example: &CollectionSynthesisExample,
) -> Vec<ResponseProgram> {
    let context = OutputRendererContext::from_example(example);
    expand_output_renderers_with_context(candidates, example, &context)
}

struct OutputRendererContext {
    allow_canonical_direct: bool,
    request_markers: Vec<RequestTemplateMarker>,
}

impl OutputRendererContext {
    fn from_example(example: &CollectionSynthesisExample) -> Self {
        Self {
            allow_canonical_direct: example.expected_response.len() <= 512
                && !synthesis_request_text(&example.provider_payload)
                    .is_some_and(|request| request_requires_exact_surface(&request)),
            request_markers: request_template_markers(&example.provider_payload),
        }
    }
}

fn expand_output_renderers_with_context(
    candidates: Vec<ResponseProgram>,
    example: &CollectionSynthesisExample,
    context: &OutputRendererContext,
) -> Vec<ResponseProgram> {
    let mut expanded = Vec::with_capacity(candidates.len().saturating_mul(2));
    for program in candidates {
        let execution = execute_example(&program, example);
        if let Some(computed) = execution
            .response
            .as_deref()
            .filter(|computed| !computed.is_empty())
        {
            expanded.extend(expand_output_renderer_with_computed(
                program, computed, example, context,
            ));
        }
    }
    if expanded.len() > 1 {
        expanded.sort_by(|left, right| {
            serde_json::to_vec(left)
                .unwrap_or_default()
                .cmp(&serde_json::to_vec(right).unwrap_or_default())
        });
        expanded.dedup();
    }
    expanded
}

fn expand_output_renderer_with_computed(
    program: ResponseProgram,
    computed: &str,
    example: &CollectionSynthesisExample,
    context: &OutputRendererContext,
) -> Vec<ResponseProgram> {
    let mut expanded = Vec::new();
    let mut matches = example.expected_response.match_indices(computed);
    if let Some((offset, _)) = matches.next()
        && matches.next().is_none()
        && example.expected_response != computed
    {
        let suffix_offset = offset.saturating_add(computed.len());
        let prefix = &example.expected_response[..offset];
        let suffix = &example.expected_response[suffix_offset..];
        if prefix.len().saturating_add(suffix.len()) <= MAX_RESPONSE_STATIC_TEXT_BYTES {
            let renderer = CollectionOutputRenderer::RenderTemplate {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
            };
            expanded.push(match &program.operation {
                ResponseOperation::ProjectSelectedValue { .. } => {
                    program.clone().with_value_renderer(renderer)
                }
                ResponseOperation::ProjectStatus { .. } => {
                    program.clone().with_status_renderer(renderer)
                }
                _ => program.clone().with_collection_renderer(renderer),
            });
        }
    }
    for marker in &context.request_markers {
        let renderer = CollectionOutputRenderer::RequestTemplate { marker: *marker };
        let rendered = match &program.operation {
            ResponseOperation::ProjectSelectedValue { .. } => {
                program.clone().with_value_renderer(renderer)
            }
            ResponseOperation::ProjectStatus { .. } => {
                program.clone().with_status_renderer(renderer)
            }
            _ => program.clone().with_collection_renderer(renderer),
        };
        if execute_example(&rendered, example).response.as_deref()
            == Some(example.expected_response.as_str())
        {
            expanded.push(rendered);
        }
    }
    if computed == example.expected_response || context.allow_canonical_direct {
        expanded.push(program);
    }
    expanded
}

fn request_template_markers(payload: &Value) -> Vec<RequestTemplateMarker> {
    let request = synthesis_request_text(payload).unwrap_or_default();
    [
        RequestTemplateMarker::BracedValue,
        RequestTemplateMarker::BracedResult,
        RequestTemplateMarker::BracedCount,
        RequestTemplateMarker::BracedStatus,
        RequestTemplateMarker::BracedItems,
        RequestTemplateMarker::DoubleBracedValue,
        RequestTemplateMarker::AngleValue,
        RequestTemplateMarker::AngleResult,
        RequestTemplateMarker::PercentS,
    ]
    .into_iter()
    .filter(|marker| request.contains(marker.token()))
    .collect()
}

fn synthesis_request_text(payload: &Value) -> Option<String> {
    let request = payload
        .get("input")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content"))
        .flat_map(|content| match content {
            Value::String(text) => vec![text.as_str()],
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!request.is_empty()).then_some(request)
}

fn latest_output_json(payload: &Value) -> Result<Value, &'static str> {
    let output = latest_tool_output(payload).ok_or("collection_support_output_missing")?;
    unique_embedded_json_output(output).ok_or("collection_support_output_not_json")
}

fn enumerate_collection_source_candidates(
    payload: &Value,
) -> Result<Vec<ResponseProgram>, &'static str> {
    let mut programs = Vec::new();
    if let Ok(root) = latest_output_json(payload)
        && let Ok(candidates) = enumerate_candidates(&root)
    {
        programs.extend(candidates);
    }
    for (output_ordinal, output) in active_turn_outputs(payload)
        .into_iter()
        .rev()
        .take(MAX_TURN_COLLECTION_OUTPUTS)
    {
        let Some(root) = unique_embedded_json_output(output) else {
            continue;
        };
        let Ok(candidates) = enumerate_candidates(&root) else {
            continue;
        };
        for mut program in candidates {
            let ResponseOperation::ComposeCollection { steps, .. } = &mut program.operation else {
                continue;
            };
            steps.insert(
                0,
                CollectionProgramStep::SelectTurnOutput { output_ordinal },
            );
            if program.validate().is_ok() {
                programs.push(program);
            }
        }
    }
    programs.sort_by_key(|program| serde_json::to_vec(program).unwrap_or_default());
    programs.dedup();
    if programs.is_empty() {
        return Err("collection_support_output_not_json");
    }
    if programs.len() > MAX_CANDIDATES {
        return Err("collection_candidate_budget");
    }
    Ok(programs)
}

fn active_turn_outputs(payload: &Value) -> Vec<(u16, &Value)> {
    let Some(items) = payload.get("input").and_then(Value::as_array) else {
        return Vec::new();
    };
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    items[turn_start..]
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output"))
        .enumerate()
        .filter_map(|(index, output)| {
            u16::try_from(index.saturating_add(1))
                .ok()
                .filter(|ordinal| *ordinal <= MAX_TURN_COLLECTION_OUTPUTS as u16)
                .map(|ordinal| (ordinal, output))
        })
        .collect()
}

fn latest_tool_output(payload: &Value) -> Option<&Value> {
    let item = payload.get("input")?.as_array()?.last()?;
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    )
    .then(|| item.get("output"))?
}

fn bounded_collection_output_text(output: &Value) -> Option<String> {
    nando_operator_runtime::bounded_output_text(output)
}

fn unique_embedded_json_output(output: &Value) -> Option<Value> {
    nando_operator_runtime::unique_embedded_json_output(output)
}

fn enumerate_candidates(root: &Value) -> Result<Vec<ResponseProgram>, &'static str> {
    if let Some(rows) = root.as_array() {
        return enumerate_root_array_candidates(rows);
    }
    let object = root
        .as_object()
        .ok_or("collection_support_root_not_object")?;
    let mut programs = Vec::new();
    let array_field_count = object.values().filter(|value| value.is_array()).count();
    for (collection_field, collection) in object {
        let Some(rows) = collection.as_array() else {
            continue;
        };
        if rows.is_empty() || rows.len() > MAX_SEARCH_ROWS {
            continue;
        }
        let select = CollectionProgramStep::SelectField {
            field: collection_field.clone(),
        };
        let structural_select = CollectionProgramStep::SelectOnlyArrayField;
        push_program(&mut programs, vec![select.clone()]);
        push_program(
            &mut programs,
            vec![select.clone(), CollectionProgramStep::Count],
        );
        let fields = common_row_fields(rows);
        if array_field_count == 1 {
            push_program(&mut programs, vec![structural_select.clone()]);
            push_program(
                &mut programs,
                vec![structural_select.clone(), CollectionProgramStep::Count],
            );
            for operation in [
                CollectionAggregateOperation::Sum,
                CollectionAggregateOperation::Min,
                CollectionAggregateOperation::Max,
            ] {
                push_program(
                    &mut programs,
                    vec![
                        structural_select.clone(),
                        CollectionProgramStep::AggregateUniqueIntegerField { operation },
                    ],
                );
            }
            for value_type in [
                CollectionScalarType::String,
                CollectionScalarType::Integer,
                CollectionScalarType::Boolean,
            ] {
                let request_filter =
                    CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type };
                push_program(
                    &mut programs,
                    vec![structural_select.clone(), request_filter.clone()],
                );
                push_program(
                    &mut programs,
                    vec![
                        structural_select.clone(),
                        request_filter.clone(),
                        CollectionProgramStep::Count,
                    ],
                );
                push_program(
                    &mut programs,
                    vec![
                        structural_select.clone(),
                        request_filter.clone(),
                        CollectionProgramStep::ProjectOnlyNonFilterField,
                    ],
                );
                for operation in [
                    CollectionAggregateOperation::Sum,
                    CollectionAggregateOperation::Min,
                    CollectionAggregateOperation::Max,
                ] {
                    push_program(
                        &mut programs,
                        vec![
                            structural_select.clone(),
                            request_filter.clone(),
                            CollectionProgramStep::AggregateUniqueIntegerField { operation },
                        ],
                    );
                }
            }
        }
        for field in &fields {
            push_program(
                &mut programs,
                vec![
                    select.clone(),
                    CollectionProgramStep::ProjectField {
                        field: field.clone(),
                    },
                ],
            );
            if array_field_count == 1
                && let Some(value_type) = common_field_scalar_type(rows, field)
                && fields
                    .iter()
                    .filter(|candidate| {
                        common_field_scalar_type(rows, candidate) == Some(value_type)
                    })
                    .count()
                    == 1
            {
                push_program(
                    &mut programs,
                    vec![
                        structural_select.clone(),
                        CollectionProgramStep::ProjectUniqueFieldByType { value_type },
                    ],
                );
            }
        }
        for predicate_field in &fields {
            for literal in observed_literals(rows, predicate_field) {
                let filter = CollectionProgramStep::FilterFieldEquals {
                    field: predicate_field.clone(),
                    value: literal,
                };
                push_program(&mut programs, vec![select.clone(), filter.clone()]);
                push_program(
                    &mut programs,
                    vec![select.clone(), filter.clone(), CollectionProgramStep::Count],
                );
                for projection_field in &fields {
                    push_program(
                        &mut programs,
                        vec![
                            select.clone(),
                            filter.clone(),
                            CollectionProgramStep::ProjectField {
                                field: projection_field.clone(),
                            },
                        ],
                    );
                }
                if array_field_count == 1 {
                    let structural_filter = CollectionProgramStep::FilterUniqueFieldEquals {
                        value: match &filter {
                            CollectionProgramStep::FilterFieldEquals { value, .. } => value.clone(),
                            _ => unreachable!(),
                        },
                    };
                    push_program(
                        &mut programs,
                        vec![structural_select.clone(), structural_filter.clone()],
                    );
                    push_program(
                        &mut programs,
                        vec![
                            structural_select.clone(),
                            structural_filter.clone(),
                            CollectionProgramStep::Count,
                        ],
                    );
                    push_program(
                        &mut programs,
                        vec![
                            structural_select.clone(),
                            structural_filter,
                            CollectionProgramStep::ProjectOnlyNonFilterField,
                        ],
                    );
                }
            }
        }
    }
    programs.sort_by(|left, right| {
        serde_json::to_vec(left)
            .unwrap_or_default()
            .cmp(&serde_json::to_vec(right).unwrap_or_default())
    });
    programs.dedup();
    if programs.len() > MAX_CANDIDATES {
        return Err("collection_candidate_budget");
    }
    Ok(programs)
}

fn enumerate_root_array_candidates(rows: &[Value]) -> Result<Vec<ResponseProgram>, &'static str> {
    if rows.is_empty() || rows.len() > MAX_SEARCH_ROWS {
        return Err("collection_support_root_array_budget");
    }
    let mut programs = Vec::new();
    push_program(&mut programs, vec![CollectionProgramStep::Count]);
    let fields = common_row_fields(rows);
    for operation in [
        CollectionAggregateOperation::Sum,
        CollectionAggregateOperation::Min,
        CollectionAggregateOperation::Max,
    ] {
        push_program(
            &mut programs,
            vec![CollectionProgramStep::AggregateUniqueIntegerField { operation }],
        );
    }
    for value_type in [
        CollectionScalarType::String,
        CollectionScalarType::Integer,
        CollectionScalarType::Boolean,
    ] {
        push_program(
            &mut programs,
            vec![CollectionProgramStep::ProjectUniqueFieldByType { value_type }],
        );
        let filter = CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type };
        push_program(&mut programs, vec![filter.clone()]);
        push_program(
            &mut programs,
            vec![filter.clone(), CollectionProgramStep::Count],
        );
        push_program(
            &mut programs,
            vec![
                filter.clone(),
                CollectionProgramStep::ProjectOnlyNonFilterField,
            ],
        );
        for operation in [
            CollectionAggregateOperation::Sum,
            CollectionAggregateOperation::Min,
            CollectionAggregateOperation::Max,
        ] {
            push_program(
                &mut programs,
                vec![
                    filter.clone(),
                    CollectionProgramStep::AggregateUniqueIntegerField { operation },
                ],
            );
        }
    }
    for field in &fields {
        for literal in observed_literals(rows, field) {
            let filter = CollectionProgramStep::FilterUniqueFieldEquals { value: literal };
            push_program(&mut programs, vec![filter.clone()]);
            push_program(
                &mut programs,
                vec![filter.clone(), CollectionProgramStep::Count],
            );
            push_program(
                &mut programs,
                vec![filter, CollectionProgramStep::ProjectOnlyNonFilterField],
            );
        }
    }
    programs.sort_by_key(|program| serde_json::to_vec(program).unwrap_or_default());
    programs.dedup();
    if programs.len() > MAX_CANDIDATES {
        return Err("collection_candidate_budget");
    }
    Ok(programs)
}

fn push_program(programs: &mut Vec<ResponseProgram>, steps: Vec<CollectionProgramStep>) {
    programs.push(ResponseProgram::compose_collection(
        steps.clone(),
        ValueProjectionFormat::CanonicalJson,
        "completed",
    ));
    if matches!(
        steps.last(),
        Some(
            CollectionProgramStep::Count
                | CollectionProgramStep::AggregateUniqueIntegerField { .. }
                | CollectionProgramStep::ProjectField { .. }
                | CollectionProgramStep::ProjectUniqueFieldByType { .. }
                | CollectionProgramStep::ProjectOnlyNonFilterField
        )
    ) {
        programs.push(ResponseProgram::compose_collection(
            steps,
            ValueProjectionFormat::PlainText,
            "completed",
        ));
    }
}

fn common_row_fields(rows: &[Value]) -> Vec<String> {
    let mut common = rows
        .first()
        .and_then(Value::as_object)
        .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    for row in rows.iter().skip(1) {
        let fields = row
            .as_object()
            .map(|object| object.keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        common = common.intersection(&fields).cloned().collect();
    }
    common.into_iter().collect()
}

fn observed_literals(rows: &[Value], field: &str) -> Vec<ResponseScalarLiteral> {
    rows.iter()
        .filter_map(|row| row.as_object()?.get(field))
        .filter_map(|value| match value {
            Value::String(value) if value.len() <= 128 => {
                Some(ResponseScalarLiteral::String(value.clone()))
            }
            Value::Number(value) => value.as_i64().map(ResponseScalarLiteral::Integer),
            Value::Bool(value) => Some(ResponseScalarLiteral::Boolean(*value)),
            Value::Null => Some(ResponseScalarLiteral::Null),
            Value::Array(_) | Value::Object(_) | Value::String(_) => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn common_field_scalar_type(rows: &[Value], field: &str) -> Option<CollectionScalarType> {
    let mut observed = rows.iter().map(|row| {
        let value = row.as_object()?.get(field)?;
        match value {
            Value::String(_) => Some(CollectionScalarType::String),
            Value::Number(number) if number.is_i64() || number.is_u64() => {
                Some(CollectionScalarType::Integer)
            }
            Value::Bool(_) => Some(CollectionScalarType::Boolean),
            _ => None,
        }
    });
    let first = observed.next()??;
    observed.all(|value| value == Some(first)).then_some(first)
}

#[cfg(test)]
#[path = "collection_synthesis_tests.rs"]
mod tests;
