use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AtomValueType, CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, OutputGraphSegment, OutputValueCandidate, OutputValueSource,
    ProjectStatusMapping, RequestTemplateMarker, ResponseExecution, ResponseExecutionStatus,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseScalarLiteral,
    ResponseValueSelector, ValueProjectionFormat, VerifierProgram, build_output_graph,
    execute_response,
};

const MAX_SEARCH_ROWS: usize = 1_024;
const MAX_CANDIDATES: usize = 16_384;
const MAX_TURN_COLLECTION_OUTPUTS: usize = 16;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CollectionSynthesisExample {
    pub provider_payload: Value,
    pub expected_response: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesizedCollectionProgram {
    pub program: ResponseProgram,
    pub verifier: VerifierProgram,
    pub exact_checks: usize,
    pub candidates_enumerated: usize,
    pub description_length_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionVersionSpace {
    pub programs: Vec<ResponseProgram>,
    pub exact_checks: usize,
    pub candidates_enumerated: usize,
    pub policy_rejected_exact_matches: usize,
    pub policy_rejection_reasons: BTreeMap<String, usize>,
    pub canonical_rejection_reasons: BTreeMap<String, usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResponseCoverageDiagnostic {
    pub response_bytes: usize,
    pub dynamic_bytes: usize,
    pub request_dynamic_bytes: usize,
    pub tool_dynamic_bytes: usize,
    pub matching_selectors: usize,
    pub exact_surface_required: bool,
}

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
            || example.expected_response.len() <= canonical_max_bytes.saturating_add(512)
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
            || example.expected_response.len() <= 512 + 32;
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
            <= 512
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
                if reason == "unsafe_render_sequence_static_text"
                    && let Ok(canonical) = canonical_direct_response_program(&program)
                {
                    structurally_aligned_canonical
                        .insert(serde_json::to_vec(&canonical).unwrap_or_default());
                    if response_program_match_quality_with_alignment(&canonical, example, true) == 0
                    {
                        *canonical_rejection_reasons
                            .entry(response_program_match_rejection_reason(&canonical, example))
                            .or_default() += 1;
                    }
                    candidates.push(canonical);
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
        <= 512;
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
            selector: ResponseValueSelector::RequestReferencedJsonField { .. },
            ..
        }
        | ResponseOperation::ProjectStatus {
            selector: ResponseValueSelector::RequestReferencedJsonField { .. },
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
        <= 512;
    compose_render_sequence_candidates(example, &selectors, include_surface_renderer)
        .into_iter()
        .filter(|surface| {
            surface.validate() == Err("unsafe_render_sequence_static_text")
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
    let mut selectors = [
        AtomValueType::String,
        AtomValueType::Integer,
        AtomValueType::Boolean,
    ]
    .into_iter()
    .flat_map(|value_type| {
        [
            ResponseValueSelector::UniqueScalar { value_type },
            ResponseValueSelector::UniqueTurnScalar { value_type },
        ]
    })
    .collect::<Vec<_>>();
    selectors.push(ResponseValueSelector::CommandOutputBody);
    selectors.push(ResponseValueSelector::RequestLastToken);
    selectors.push(ResponseValueSelector::RequestUniqueLiteral);
    selectors.extend([
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::String,
        },
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Integer,
        },
        ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Boolean,
        },
    ]);
    for ordinal in 0..16_u16 {
        selectors.extend([
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::String,
            },
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::Integer,
            },
            ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                ordinal,
                value_type: AtomValueType::Boolean,
            },
        ]);
    }
    let Some(items) = payload.get("input").and_then(Value::as_array) else {
        return selectors;
    };
    let outputs = items
        .iter()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .filter_map(|item| item.get("output"))
        .collect::<Vec<_>>();
    let Some(latest_output) = outputs.last().copied() else {
        return selectors;
    };
    if let Some(value) = unique_embedded_json_output(latest_output) {
        let mut fields = Vec::new();
        collect_scalar_field_candidates(&value, 0, &mut fields);
        selectors.extend(
            fields
                .into_iter()
                .map(|(field, value_type)| ResponseValueSelector::JsonField { field, value_type }),
        );
        let mut ordinals = BTreeMap::new();
        let mut ordinal_selectors = Vec::new();
        collect_scalar_ordinal_candidates(&value, 0, &mut ordinals, &mut ordinal_selectors);
        selectors.extend(ordinal_selectors);
    }
    let mut turn_fields = Vec::new();
    for output in &outputs {
        if let Some(value) = unique_embedded_json_output(output) {
            collect_scalar_field_candidates(&value, 0, &mut turn_fields);
        }
    }
    selectors.extend(turn_fields.into_iter().flat_map(|(field, value_type)| {
        [
            ResponseValueSelector::UniqueTurnJsonField {
                field: field.clone(),
                value_type,
            },
            ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type },
        ]
    }));
    for (output_index, output) in outputs.iter().enumerate() {
        let Ok(output_ordinal) = u16::try_from(output_index.saturating_add(1)) else {
            break;
        };
        if output_index < 16 {
            for value_type in [
                AtomValueType::String,
                AtomValueType::Integer,
                AtomValueType::Boolean,
            ] {
                for scalar_ordinal in 0_u16..16 {
                    selectors.push(ResponseValueSelector::TurnOutputScalarOrdinal {
                        output_ordinal,
                        scalar_ordinal,
                        value_type,
                    });
                    if output_index.saturating_add(1) == outputs.len() {
                        selectors.push(ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                            scalar_ordinal,
                            value_type,
                        });
                        selectors.push(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                            reverse_ordinal: scalar_ordinal,
                            value_type,
                        });
                    }
                }
            }
        }
        let Some(output_text) = bounded_collection_output_text(output) else {
            continue;
        };
        for (line_index, line) in output_text.lines().enumerate().take(256) {
            if !line.is_empty()
                && line.len() <= 512
                && let Ok(line_index) = u16::try_from(line_index)
            {
                selectors.push(ResponseValueSelector::TurnOutputLine {
                    output_ordinal,
                    line_index,
                    value_type: AtomValueType::String,
                });
                if output_index.saturating_add(1) == outputs.len() {
                    selectors.push(ResponseValueSelector::LatestTurnOutputLine {
                        line_index,
                        value_type: AtomValueType::String,
                    });
                }
            }
            if output_index.saturating_add(1) != outputs.len() {
                continue;
            }
            for delimiter in [':', '='] {
                let Some(offset) = line.find(delimiter) else {
                    continue;
                };
                let split = offset.saturating_add(delimiter.len_utf8());
                let whitespace = line[split..]
                    .len()
                    .saturating_sub(line[split..].trim_start().len());
                let prefix_end = split.saturating_add(whitespace);
                if prefix_end == 0 || prefix_end > 128 || prefix_end >= line.len() {
                    continue;
                }
                let candidate = line[prefix_end..].trim();
                if let Some(value_type) = scalar_text_type(candidate) {
                    selectors.push(ResponseValueSelector::ContentLinePrefix {
                        prefix: line[..prefix_end].to_owned(),
                        value_type,
                    });
                }
            }
        }
    }
    selectors.sort();
    selectors.dedup();
    selectors
}

fn collect_scalar_ordinal_candidates(
    value: &Value,
    depth: usize,
    ordinals: &mut BTreeMap<AtomValueType, u16>,
    output: &mut Vec<ResponseValueSelector>,
) {
    if depth > 8 || output.len() >= 64 {
        return;
    }
    if let Some(value_type) = atom_value_type(value) {
        let ordinal = ordinals.entry(value_type).or_default();
        output.push(ResponseValueSelector::JsonScalarOrdinal {
            ordinal: *ordinal,
            value_type,
        });
        *ordinal = ordinal.saturating_add(1);
        return;
    }
    match value {
        Value::Object(object) => {
            for value in object.values() {
                collect_scalar_ordinal_candidates(value, depth.saturating_add(1), ordinals, output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_ordinal_candidates(value, depth.saturating_add(1), ordinals, output);
            }
        }
        _ => {}
    }
}

fn collect_scalar_field_candidates(
    value: &Value,
    depth: usize,
    output: &mut Vec<(String, AtomValueType)>,
) {
    if depth > 8 || output.len() >= 256 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if let Some(value_type) = atom_value_type(value) {
                    output.push((field.clone(), value_type));
                }
                collect_scalar_field_candidates(value, depth.saturating_add(1), output);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_scalar_field_candidates(value, depth.saturating_add(1), output);
            }
        }
        _ => {}
    }
}

fn atom_value_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::String(_) => Some(AtomValueType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(AtomValueType::Integer),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}

fn scalar_text_type(value: &str) -> Option<AtomValueType> {
    let parsed =
        serde_json::from_str::<Value>(value).unwrap_or_else(|_| Value::String(value.to_owned()));
    atom_value_type(&parsed)
}

const fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type }
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
    steps.iter().all(|step| {
        matches!(
            step,
            CollectionProgramStep::SelectTurnOutput { .. }
                | CollectionProgramStep::SelectOnlyArrayField
                | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                | CollectionProgramStep::ProjectUniqueFieldByType { .. }
                | CollectionProgramStep::ProjectOnlyNonFilterField
                | CollectionProgramStep::AggregateUniqueIntegerField { .. }
                | CollectionProgramStep::Count
        )
    })
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
        } => {
            matches!(
                selector,
                ResponseValueSelector::UniqueScalar { .. }
                    | ResponseValueSelector::UniqueTurnScalar { .. }
                    | ResponseValueSelector::RequestReferencedJsonField { .. }
                    | ResponseValueSelector::TurnOutputScalarOrdinal { .. }
                    | ResponseValueSelector::LatestTurnOutputScalarOrdinal { .. }
                    | ResponseValueSelector::LatestTurnOutputScalarFromEnd { .. }
                    | ResponseValueSelector::LatestTurnOutputLine { .. }
            ) && response_renderer_is_surface_neutral(renderer)
        }
        ResponseOperation::ComposeCollection { renderer, .. } => {
            is_source_neutral_collection_program(program)
                && response_renderer_is_surface_neutral(renderer)
        }
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
        | ResponseOperation::ProjectStatus { .. } => program.validate().is_ok(),
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
        | ResponseOperation::ProjectStatus { .. } => true,
        ResponseOperation::ComposeCollection { steps, .. } => steps.iter().all(|step| {
            matches!(
                step,
                CollectionProgramStep::SelectTurnOutput { .. }
                    | CollectionProgramStep::SelectOnlyArrayField
                    | CollectionProgramStep::SelectField { .. }
                    | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
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
        if prefix.len().saturating_add(suffix.len()) <= 512 {
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
    match output {
        Value::String(text) if !text.is_empty() && text.len() <= 65_536 => Some(text.clone()),
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            let mut output = String::new();
            for part in parts {
                if !matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) {
                    return None;
                }
                let text = part.get("text").and_then(Value::as_str)?;
                let next_len = output
                    .len()
                    .checked_add(text.len())?
                    .checked_add(usize::from(!output.is_empty()))?;
                if text.is_empty() || next_len > 65_536 {
                    return None;
                }
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(text);
            }
            Some(output)
        }
        _ => None,
    }
}

fn unique_embedded_json_output(output: &Value) -> Option<Value> {
    bounded_collection_output_text(output)?;
    match output {
        Value::String(text) => unique_embedded_json_object(text),
        Value::Array(_) => {
            let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
            collect_embedded_json_objects(output, &mut candidates, 0);
            (candidates.len() == 1)
                .then(|| candidates.into_values().next())
                .flatten()
        }
        _ => None,
    }
}

fn unique_embedded_json_object(text: &str) -> Option<Value> {
    unique_embedded_json_object_at_depth(text, 0)
}

fn unique_embedded_json_object_at_depth(text: &str, depth: usize) -> Option<Value> {
    if depth > 4 {
        return None;
    }
    let mut sources = vec![text.trim().to_owned()];
    if let Some((_, output)) = text.rsplit_once("\nOutput:\n") {
        sources.push(output.trim().to_owned());
    }
    let mut fenced = None::<String>;
    for line in text.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if let Some(value) = &mut fenced {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
        } else if trimmed == "```" || trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if trimmed.starts_with(['{', '[']) {
            sources.push(trimmed.to_owned());
        }
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for source in sources {
        let Ok(value) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        collect_embedded_json_objects(&value, &mut candidates, depth);
    }
    (candidates.len() == 1)
        .then(|| candidates.into_values().next())
        .flatten()
}

fn collect_embedded_json_objects(
    value: &Value,
    output: &mut BTreeMap<Vec<u8>, Value>,
    depth: usize,
) {
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            if depth < 4 {
                for text in object.values().filter_map(Value::as_str) {
                    if let Some(child) = unique_embedded_json_object_at_depth(text, depth + 1)
                        && let Ok(key) = serde_json::to_vec(&child)
                    {
                        encoded_children.insert(key, child);
                    }
                }
            }
            if encoded_children.len() == 1 {
                output.extend(encoded_children);
                return;
            }
            if let Ok(key) = serde_json::to_vec(value) {
                output.insert(key, value.clone());
            }
        }
        Value::Array(parts) => {
            let content_parts = !parts.is_empty()
                && parts.iter().all(|part| {
                    part.get("text").and_then(Value::as_str).is_some()
                        && matches!(
                            part.get("type").and_then(Value::as_str),
                            Some("text" | "input_text" | "output_text")
                        )
                });
            if !content_parts {
                let wrapped = serde_json::json!({"items": value.clone()});
                if let Ok(key) = serde_json::to_vec(&wrapped) {
                    output.insert(key, wrapped);
                }
                return;
            }
            for part in parts {
                let Some(text) = part.get("text").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(value) = unique_embedded_json_object_at_depth(text, depth + 1)
                    && let Ok(key) = serde_json::to_vec(&value)
                {
                    output.insert(key, value);
                }
            }
        }
        _ => {}
    }
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
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{ResponseOperation, verify_response_independently};

    fn example(collection: &str, predicate: &str, value: &str) -> CollectionSynthesisExample {
        let output = json!({
            (collection): [
                {(predicate):"keep", (value):3},
                {(predicate):"drop", (value):4},
                {(predicate):"keep", (value):5}
            ]
        });
        CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":output.to_string()}]
            }),
            expected_response: "[3,5]".to_owned(),
        }
    }

    #[test]
    fn count_is_synthesized_from_an_earlier_turn_output_with_runtime_parity() {
        let support = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":"How many rows?"},
                    {"type":"function_call_output","output":"{\"rows\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"},
                    {"type":"function_call_output","output":"command completed"}
                ]
            }),
            expected_response: "3".to_owned(),
        };
        let synthesized = synthesize_collection_program(&[support.clone()]).expect("synthesis");
        let ResponseOperation::ComposeCollection { steps, .. } = &synthesized.program.operation
        else {
            panic!("expected collection program");
        };
        assert!(matches!(
            steps.first(),
            Some(CollectionProgramStep::SelectTurnOutput { output_ordinal: 1 })
        ));
        assert!(matches!(steps.last(), Some(CollectionProgramStep::Count)));

        let execution = execute_response(&synthesized.program, "", &support.provider_payload);
        assert_eq!(execution.response.as_deref(), Some("3"));
        assert!(
            verify_response_independently(&synthesized.verifier, &support.provider_payload, "3")
                .is_ok()
        );

        let swapped = json!({
            "input":[
                {"type":"message","role":"user","content":"How many rows?"},
                {"type":"function_call_output","output":"command completed"},
                {"type":"function_call_output","output":"{\"rows\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"}
            ]
        });
        assert!(
            execute_response(&synthesized.program, "", &swapped)
                .response
                .is_none()
        );
        assert!(verify_response_independently(&synthesized.verifier, &swapped, "3").is_err());
    }

    #[test]
    fn embedded_json_collection_has_actor_verifier_runtime_parity() {
        let mut support = [example("rows", "kind", "value")];
        support[0].provider_payload = json!({
            "input":[{
                "type":"function_call_output",
                "output":"command output\n```json\n{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}\n```\ncompleted"
            }]
        });
        let synthesized =
            synthesize_collection_program(&[example("rows", "kind", "value")]).expect("synthesis");
        let execution = execute_response(&synthesized.program, "", &support[0].provider_payload);
        assert_eq!(execution.response.as_deref(), Some("[3,5]"));
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &support[0].provider_payload,
                "[3,5]"
            )
            .is_ok()
        );
    }

    #[test]
    fn line_embedded_json_collection_is_synthesized_with_runtime_parity() {
        let mut support = [example("rows", "kind", "value")];
        support[0].provider_payload = json!({
            "input":[{
                "type":"function_call_output",
                "output":"progress\n{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}\ncompleted"
            }]
        });
        let synthesized = synthesize_collection_program(&support).expect("synthesis");
        let execution = execute_response(&synthesized.program, "", &support[0].provider_payload);
        assert_eq!(execution.response.as_deref(), Some("[3,5]"));
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &support[0].provider_payload,
                "[3,5]"
            )
            .is_ok()
        );
    }

    #[test]
    fn latest_turn_output_scalar_ignores_earlier_outputs_with_runtime_parity() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"function_call_output","output":"{\"old\":99}"},
                    {"type":"message","role":"user","content":"Return the latest total"},
                    {"type":"function_call_output","output":"{\"noise\":11}"},
                    {"type":"function_call_output","output":"{\"total\":7}"}
                ]
            }),
            expected_response: "7".to_owned(),
        };
        let version_space =
            enumerate_source_neutral_response_programs(&example).expect("version space");
        let program = version_space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                            scalar_ordinal: 0,
                            value_type: AtomValueType::Integer,
                        },
                        ..
                    }
                )
            })
            .expect("latest output program");
        assert_eq!(
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some("7")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
    }

    #[test]
    fn generic_version_space_discovers_rendered_scalar_projection() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"total\":7}"}]
            }),
            expected_response: "Total: 7.".to_owned(),
        };
        let version_space =
            enumerate_source_neutral_response_programs(&example).expect("version space");
        assert!(version_space.exact_checks > 0);
        let program = version_space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue { .. }
                )
            })
            .expect("scalar projection");
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert_eq!(
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some("Total: 7.")
        );
        assert!(
            verify_response_independently(
                &verifier,
                &example.provider_payload,
                &example.expected_response,
            )
            .is_ok()
        );
    }

    #[test]
    fn evidence_only_projection_discards_unverified_prose_but_keeps_grounded_scalar() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"count\":7}"}]
            }),
            expected_response: "There are 7 matching rows.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("version space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        format: ValueProjectionFormat::PlainText,
                        renderer: CollectionOutputRenderer::Direct,
                        ..
                    }
                ) && execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some("7")
            })
            .expect("evidence-only projection");
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
    }

    #[test]
    fn evidence_only_projection_retains_ambiguity_for_cegis() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"count\":7,\"failed\":2}"}]
            }),
            expected_response: "There are 7 rows and 2 failures.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("version space");
        let projected = space
            .programs
            .iter()
            .filter(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        renderer: CollectionOutputRenderer::Direct,
                        ..
                    }
                )
            })
            .filter_map(|program| execute_response(program, "", &example.provider_payload).response)
            .collect::<BTreeSet<_>>();
        assert_eq!(projected, BTreeSet::from(["2".to_owned(), "7".to_owned()]));
    }

    #[test]
    fn recursive_json_field_projection_keeps_actor_verifier_parity() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"outer\":{\"result\":7}}"}]
            }),
            expected_response: "The verified result is 7.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("version space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::JsonField { field, .. },
                        renderer: CollectionOutputRenderer::Direct,
                        ..
                    } if field == "result"
                )
            })
            .expect("recursive field program");
        assert_eq!(
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some("7")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &example.provider_payload, "7").is_ok());
        let ambiguous = json!({
            "input":[{"type":"function_call_output","output":"{\"left\":{\"result\":7},\"right\":{\"result\":8}}"}]
        });
        assert_eq!(
            execute_response(program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());
    }

    #[test]
    fn json_scalar_ordinal_transfers_across_field_names() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"a\":2,\"z\":7}"}]
            }),
            expected_response: "7".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("version space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::JsonScalarOrdinal {
                            ordinal: 1,
                            value_type: AtomValueType::Integer,
                        },
                        renderer: CollectionOutputRenderer::Direct,
                        ..
                    }
                )
            })
            .expect("ordinal program");
        let future = json!({
            "input":[{"type":"function_call_output","output":"{\"alpha\":3,\"omega\":7}"}]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("7")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "7").is_ok());
    }

    #[test]
    fn generic_version_space_discovers_status_projection() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"0"}]
            }),
            expected_response: "success".to_owned(),
        };
        let version_space =
            enumerate_source_neutral_response_programs(&example).expect("version space");
        assert!(
            version_space.programs.iter().any(|program| matches!(
                &program.operation,
                ResponseOperation::ProjectStatus { .. }
            ))
        );
    }

    #[test]
    fn learned_adapter_version_space_discovers_json_field_and_line_prefix() {
        let json_example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"Wall time: 0.1 seconds\nOutput:\n[{\"type\":\"text\",\"text\":\"{\\\"noise\\\":1,\\\"result\\\":42}\"}]"}]
            }),
            expected_response: "42".to_owned(),
        };
        let json_space =
            enumerate_source_neutral_response_programs(&json_example).expect("json space");
        assert!(json_space.programs.iter().any(|program| matches!(
            &program.operation,
            ResponseOperation::ProjectSelectedValue {
                selector: ResponseValueSelector::JsonField { field, .. },
                ..
            } if field == "result"
        )));

        let line_example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"noise: 1\ncount: 42"}]
            }),
            expected_response: "42".to_owned(),
        };
        let line_space =
            enumerate_source_neutral_response_programs(&line_example).expect("line space");
        assert!(line_space.programs.iter().any(|program| matches!(
            &program.operation,
            ResponseOperation::ProjectSelectedValue {
                selector: ResponseValueSelector::ContentLinePrefix { prefix, .. },
                ..
            } if prefix == "count: "
        )));
    }

    #[test]
    fn learned_adapter_composes_multiple_grounded_scalars() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"passed\":12,\"failed\":0}"}]
            }),
            expected_response: "Passed: 12; failed: 0.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("composed space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        renderer: CollectionOutputRenderer::RenderSequence { .. },
                        ..
                    }
                )
            })
            .expect("render sequence");
        assert!(is_privacy_safe_online_response_program(program));
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert_eq!(
            execute_response(program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some(example.expected_response.as_str())
        );
        assert!(
            verify_response_independently(
                &verifier,
                &example.provider_payload,
                &example.expected_response,
            )
            .is_ok()
        );
    }

    #[test]
    fn canonical_renderer_handles_more_than_eight_dynamic_values() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":
                    "{\"a\":11,\"b\":12,\"c\":13,\"d\":14,\"e\":15,\"f\":16,\"g\":17,\"h\":18,\"i\":19}"}]
            }),
            expected_response: "Observed values: 11, 12, 13, 14, 15, 16, 17, 18, 19.".to_owned(),
        };
        let expected = "11\n12\n13\n14\n15\n16\n17\n18\n19";
        let space = enumerate_source_neutral_response_programs(&example).expect("bounded space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some(expected)
            })
            .expect("canonical nine-value renderer");
        assert!(response_program_authority_matches_example(
            program, &example
        ));
        assert!(is_privacy_safe_online_response_program(program));
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(
            verify_response_independently(&verifier, &example.provider_payload, expected).is_ok()
        );
    }

    #[test]
    fn teacher_prose_trains_canonical_multi_value_response() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"passed\":12,\"failed\":0}"}]
            }),
            expected_response: "The run completed with 12 passing checks and 0 failures."
                .to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("composed space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some("12\n0")
            })
            .expect("canonical multi-value response");
        assert!(is_privacy_safe_online_response_program(program));
        assert!(!response_program_exactly_matches_example(program, &example));
        assert!(response_program_authority_matches_example(
            program, &example
        ));
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(
            verify_response_independently(&verifier, &example.provider_payload, "12\n0").is_ok()
        );
    }

    #[test]
    fn learned_adapter_composes_plain_text_scalar_ordinals() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Report the verified counts"}]},
                    {"type":"function_call_output","output":"ok=3 failed=2"}
                ]
            }),
            expected_response: "success: 3; failed: 2.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("ordinal space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::TurnOutputScalarOrdinal { .. },
                        renderer: CollectionOutputRenderer::RenderSequence { .. },
                        ..
                    }
                ) && execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some(example.expected_response.as_str())
            })
            .expect("plain text ordinal render sequence");
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(
            verify_response_independently(
                &verifier,
                &example.provider_payload,
                &example.expected_response,
            )
            .is_ok()
        );
    }

    #[test]
    fn all_observation_inverse_synthesis_transfers_across_renamed_fields() {
        let support = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the verified result"}]},
                    {"type":"function_call_output","call_id":"a","output":"{\"total\":7}"},
                    {"type":"function_call_output","call_id":"b","output":"{\"status\":\"ok\"}"}
                ]
            }),
            expected_response: "Total: 7; status: ok.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&support).expect("version space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::UniqueTurnScalar { .. },
                        renderer: CollectionOutputRenderer::RenderSequence { segments },
                        ..
                    } if segments.iter().any(|segment| matches!(
                        segment,
                        ResponseRenderSegment::Selected {
                            selector: ResponseValueSelector::UniqueTurnScalar { .. },
                            ..
                        }
                    ))
                )
            })
            .expect("field-neutral multi-output program");
        let future = json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the verified result"}]},
                {"type":"function_call_output","call_id":"x","output":"{\"items\":9}"},
                {"type":"function_call_output","call_id":"y","output":"{\"state\":\"ready\"}"}
            ]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("Total: 9; status: ready.")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(
            verify_response_independently(&verifier, &future, "Total: 9; status: ready.").is_ok()
        );
    }

    #[test]
    fn unique_turn_scalar_abstains_on_same_type_collision() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueTurnScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let ambiguous = json!({
            "input":[
                {"type":"function_call_output","output":"{\"left\":7}"},
                {"type":"function_call_output","output":"{\"right\":8}"}
            ]
        });
        assert_eq!(
            execute_response(&program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        assert!(verify_response_independently(&verifier, &ambiguous, "7").is_err());
    }

    #[test]
    fn request_grounded_filter_transfers_without_literal_or_field_names() {
        let support = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Select keep values"}]},
                    {"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\",\"value\":3},{\"kind\":\"drop\",\"value\":4},{\"kind\":\"keep\",\"value\":5}]}"}
                ]
            }),
            expected_response: "[3,5]".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&support).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ComposeCollection { steps, .. }
                        if steps.iter().any(|step| matches!(
                            step,
                            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        ))
                )
            })
            .expect("request-grounded program");
        let future = json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Return yes measurements"}]},
                {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"measurement\":11},{\"class\":\"no\",\"measurement\":12},{\"class\":\"yes\",\"measurement\":13}]}"}
            ]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("[11,13]")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "[11,13]").is_ok());
        let ambiguous = json!({
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"Return yes and no measurements"}]},
                {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"measurement\":11},{\"class\":\"no\",\"measurement\":12}]}"}
            ]
        });
        assert_eq!(
            execute_response(program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &ambiguous, "[11]").is_err());
    }

    #[test]
    fn bounded_sequence_allows_two_values_and_repeated_primary() {
        for expected in ["78", "77"] {
            let example = CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
                }),
                expected_response: expected.to_owned(),
            };
            let space = enumerate_source_neutral_response_programs(&example).expect("space");
            let selectors = learned_selector_candidates(&example.provider_payload);
            let resolved = selectors
                .iter()
                .map(|selector| {
                    (
                        selector.clone(),
                        execute_response(
                            &ResponseProgram::project_selected_value(
                                selector.clone(),
                                ValueProjectionFormat::PlainText,
                                "completed",
                            ),
                            "",
                            &example.provider_payload,
                        )
                        .response,
                    )
                })
                .collect::<Vec<_>>();
            let direct_candidate = compose_render_sequence_candidates(&example, &selectors, true)
                .into_iter()
                .find(|program| {
                    execute_response(program, "", &example.provider_payload)
                        .response
                        .as_deref()
                        == Some(expected)
                })
                .unwrap_or_else(|| panic!("direct sequence candidate: {resolved:?}"));
            assert_eq!(direct_candidate.validate(), Ok(()));
            assert_eq!(
                execute_response(&direct_candidate, "", &example.provider_payload)
                    .response
                    .as_deref(),
                Some(expected)
            );
            let rejection_reasons = space.policy_rejection_reasons.clone();
            let program = space
                .programs
                .iter()
                .find(|program| {
                    matches!(
                        &program.operation,
                        ResponseOperation::ProjectSelectedValue {
                            renderer: CollectionOutputRenderer::RenderSequence { .. },
                            ..
                        }
                    )
                })
                .unwrap_or_else(|| panic!("sequence: {rejection_reasons:?}"));
            let verifier = source_neutral_verifier_for_program(program).expect("verifier");
            assert_eq!(
                execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref(),
                Some(expected)
            );
            assert!(
                verify_response_independently(
                    &verifier,
                    &example.provider_payload,
                    &example.expected_response,
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn policy_rejection_counts_only_exact_candidates() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
            }),
            expected_response: "This answer is not derivable from either scalar.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        assert!(space.programs.is_empty());
        assert_eq!(space.policy_rejected_exact_matches, 0);
        assert!(space.policy_rejection_reasons.is_empty());
    }

    #[test]
    fn single_dynamic_value_renderer_is_exact_but_surface_bound() {
        let support = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"ok\":3}"}]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&support).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                response_program_exactly_matches_example(program, &support)
                    && matches!(
                        &program.operation,
                        ResponseOperation::ProjectSelectedValue {
                            selector: ResponseValueSelector::UniqueScalar { .. }
                                | ResponseValueSelector::UniqueTurnScalar { .. },
                            renderer: CollectionOutputRenderer::RenderSequence { segments },
                            ..
                        } if segments.iter().filter(|segment| matches!(
                            segment,
                            ResponseRenderSegment::Primary
                                | ResponseRenderSegment::Selected { .. }
                        )).count() == 1
                    )
            })
            .expect("single dynamic renderer");
        assert!(is_privacy_safe_online_response_program(program));
        assert!(!is_source_neutral_response_program(program));

        let future = json!({
            "input":[{"type":"function_call_output","output":"{\"done\":4}"}]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("Успешных записей: 4")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "Успешных записей: 4").is_ok());
    }

    #[test]
    fn policy_rejection_retains_exact_unsafe_sequence() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"{\"left\":7,\"right\":8}"}]
            }),
            expected_response: "Customer Alice has 7 of 8.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        assert!(
            space
                .programs
                .iter()
                .all(|program| !response_program_exactly_matches_example(program, &example))
        );
        assert!(space.policy_rejected_exact_matches > 0);
        assert!(
            space
                .policy_rejection_reasons
                .contains_key("unsafe_render_sequence_static_text")
        );
    }

    #[test]
    fn unsafe_mixed_request_tool_surface_retains_canonical_dynamic_law() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Alice"}]},
                    {"type":"function_call_output","output":"{\"count\":7}"}
                ]
            }),
            expected_response: "Customer Alice has 7.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        let canonical = space
            .programs
            .iter()
            .find(|program| {
                execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some("Alice\n7")
            })
            .expect("canonical request and tool law");
        assert!(is_privacy_safe_online_response_program(canonical));
        assert!(!is_source_neutral_response_program(canonical));
        let verifier = source_neutral_verifier_for_program(canonical).expect("verifier");
        assert!(
            verify_response_independently(&verifier, &example.provider_payload, "Alice\n7",)
                .is_ok()
        );
    }

    #[test]
    fn command_output_body_selector_ignores_envelope_metadata() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"function_call_output","output":"[{\"type\":\"input_text\",\"text\":\"Script completed\\nWall time 0.4 seconds\\nOutput:\\n\"},{\"type\":\"input_text\",\"text\":\"alpha\\n\"}]"}]
            }),
            expected_response: "Result: alpha.".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::CommandOutputBody,
                        ..
                    }
                )
            })
            .expect("command output projection");
        let future = json!({
            "input":[{"type":"function_call_output","output":"[{\"type\":\"input_text\",\"text\":\"Script completed\\nWall time 8.1 seconds\\nOutput:\\n\"},{\"type\":\"input_text\",\"text\":\"beta\\n\"}]"}]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("Result: beta.")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "Result: beta.").is_ok());
        let missing_marker = json!({
            "input":[{"type":"function_call_output","output":"beta"}]
        });
        assert_eq!(
            execute_response(program, "", &missing_marker).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn request_last_token_projects_literal_constraint_without_stored_literal() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Do the work. End only CAPTURE_COMPLETE."}]}]
            }),
            expected_response: "CAPTURE_COMPLETE".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::RequestLastToken,
                        ..
                    }
                )
            })
            .expect("request literal projection");
        let future = json!({
            "input":[{"type":"message","role":"user","content":"Run another task. Reply only FUTURE_OK!"}]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("FUTURE_OK")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "FUTURE_OK").is_ok());
        assert!(verify_response_independently(&verifier, &future, "WRONG").is_err());
    }

    #[test]
    fn request_unique_literal_projects_quoted_constraint() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[{"type":"message","role":"user","content":"Reply with `ALPHA VALUE` and then stop"}]
            }),
            expected_response: "ALPHA VALUE".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&example).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectSelectedValue {
                        selector: ResponseValueSelector::RequestUniqueLiteral,
                        ..
                    }
                )
            })
            .expect("quoted literal projection");
        let future = json!({
            "input":[{"type":"message","role":"user","content":[{"type":"input_text","text":"Return 'BETA VALUE' without commentary"}]}]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("BETA VALUE")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "BETA VALUE").is_ok());
        let ambiguous = json!({
            "input":[{"type":"message","role":"user","content":"Choose `ONE` or `TWO`"}]
        });
        assert_eq!(
            execute_response(program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn aggregate_and_filtered_aggregate_transfer_across_surfaces() {
        let support = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":"Sum keep rows"},
                    {"type":"function_call_output","output":"{\"rows\":[{\"kind\":\"keep\",\"amount\":3},{\"kind\":\"drop\",\"amount\":4},{\"kind\":\"keep\",\"amount\":5}]}"}
                ]
            }),
            expected_response: "8".to_owned(),
        };
        let space = enumerate_source_neutral_response_programs(&support).expect("space");
        let program = space
            .programs
            .iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ComposeCollection { steps, .. }
                        if steps.iter().any(|step| matches!(
                            step,
                            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                        )) && steps.iter().any(|step| matches!(
                            step,
                            CollectionProgramStep::AggregateUniqueIntegerField {
                                operation: CollectionAggregateOperation::Sum
                            }
                        ))
                )
            })
            .expect("filtered aggregate");
        let future = json!({
            "input":[
                {"type":"message","role":"user","content":"Total yes entries"},
                {"type":"function_call_output","output":"{\"items\":[{\"class\":\"yes\",\"metric\":11},{\"class\":\"no\",\"metric\":12},{\"class\":\"yes\",\"metric\":13}]}"}
            ]
        });
        assert_eq!(
            execute_response(program, "", &future).response.as_deref(),
            Some("24")
        );
        let verifier = source_neutral_verifier_for_program(program).expect("verifier");
        assert!(verify_response_independently(&verifier, &future, "24").is_ok());
    }

    #[test]
    fn embedded_json_collection_abstains_when_two_candidates_differ() {
        let synthesized =
            synthesize_collection_program(&[example("rows", "kind", "value")]).expect("synthesis");
        let payload = json!({
            "input":[{
                "type":"function_call_output",
                "output":"{\"rows\":[{\"kind\":\"keep\",\"value\":3}]}\n{\"rows\":[{\"kind\":\"keep\",\"value\":9}]}"
            }]
        });
        assert_ne!(
            execute_response(&synthesized.program, "", &payload).status,
            ResponseExecutionStatus::Executed
        );
        assert!(verify_response_independently(&synthesized.verifier, &payload, "[3]").is_err());
    }

    #[test]
    fn synthesis_discovers_role_based_filter_projection_across_renamed_layouts() {
        let support = vec![
            example("rows", "kind", "value"),
            example("entries", "tag", "amount"),
        ];
        let synthesized = synthesize_collection_program(&support).expect("synthesis");
        assert!(synthesized.exact_checks > 0);
        assert!(matches!(
            synthesized.program.operation,
            ResponseOperation::ComposeCollection { .. }
        ));
        let heldout = example("records", "marker", "score");
        let execution = execute_response(&synthesized.program, "", &heldout.provider_payload);
        assert_eq!(execution.status, ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("[3,5]"));
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &heldout.provider_payload,
                "[3,5]"
            )
            .is_ok()
        );
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &heldout.provider_payload,
                "[3,4,5]"
            )
            .is_err()
        );
    }

    #[test]
    fn synthesis_discovers_typed_output_renderer_across_renamed_layouts() {
        let mut support = vec![
            example("rows", "kind", "value"),
            example("entries", "tag", "amount"),
        ];
        support[0].expected_response = "Selected values: [3,5].".to_owned();
        support[1].provider_payload = json!({
            "input":[{"type":"function_call_output","output":
                "{\"entries\":[{\"tag\":\"keep\",\"amount\":7},{\"tag\":\"drop\",\"amount\":8},{\"tag\":\"keep\",\"amount\":9}]}"
            }]
        });
        support[1].expected_response = "Selected values: [7,9].".to_owned();
        let synthesized = synthesize_collection_program(&support).expect("template synthesis");
        assert!(matches!(
            &synthesized.program.operation,
            ResponseOperation::ComposeCollection {
                renderer: CollectionOutputRenderer::RenderTemplate { prefix, suffix },
                ..
            } if prefix == "Selected values: " && suffix == "."
        ));
        let heldout = example("records", "marker", "score");
        let execution = execute_response(&synthesized.program, "", &heldout.provider_payload);
        assert_eq!(
            execution.response.as_deref(),
            Some("Selected values: [3,5].")
        );
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &heldout.provider_payload,
                "Selected values: [3,5]."
            )
            .is_ok()
        );
        assert!(
            verify_response_independently(
                &synthesized.verifier,
                &heldout.provider_payload,
                "[3,5]"
            )
            .is_err()
        );
    }

    #[test]
    fn role_based_adapter_abstains_when_collection_or_projection_is_ambiguous() {
        let support = vec![
            example("rows", "kind", "value"),
            example("entries", "tag", "amount"),
        ];
        let synthesized = synthesize_collection_program(&support).expect("synthesis");
        let ambiguous = json!({
            "input":[{"type":"function_call_output","output":json!({
                "left":[{"kind":"keep","value":3}],
                "right":[{"tag":"keep","amount":3}]
            }).to_string()}]
        });
        assert_eq!(
            execute_response(&synthesized.program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn synthesis_discovers_filter_count_without_operator_label() {
        let mut support = vec![
            example("rows", "kind", "value"),
            example("entries", "tag", "amount"),
        ];
        for example in &mut support {
            example.expected_response = "2".to_owned();
        }
        let synthesized = synthesize_collection_program(&support).expect("count synthesis");
        let ResponseOperation::ComposeCollection { steps, format, .. } =
            &synthesized.program.operation
        else {
            panic!("collection program");
        };
        assert_eq!(*format, ValueProjectionFormat::PlainText);
        assert!(matches!(steps.last(), Some(CollectionProgramStep::Count)));
        let heldout = example("records", "marker", "score");
        assert_eq!(
            execute_response(&synthesized.program, "", &heldout.provider_payload)
                .response
                .as_deref(),
            Some("2")
        );
    }

    #[test]
    fn synthesis_unwraps_structural_json_from_text_part_arrays() {
        let wrapped = |wrapper_field: &str, collection_field: &str, values: &[i64]| {
            let mut inner = serde_json::Map::new();
            inner.insert(
                collection_field.to_owned(),
                Value::Array(
                    values
                        .iter()
                        .copied()
                        .map(|value| json!({"value":value}))
                        .collect(),
                ),
            );
            let mut envelope = serde_json::Map::new();
            envelope.insert("sequence".to_owned(), Value::from(values.len() as u64));
            envelope.insert(
                wrapper_field.to_owned(),
                Value::String(Value::Object(inner).to_string()),
            );
            CollectionSynthesisExample {
                provider_payload: json!({
                    "input":[{
                        "type":"function_call_output",
                        "output":[
                            {"type":"input_text","text":"Script completed\nOutput:\n"},
                            {"type":"input_text","text":Value::Object(envelope).to_string()}
                        ]
                    }]
                }),
                expected_response: values.len().to_string(),
            }
        };
        let support = vec![
            wrapped("payload_blob", "alpha", &[1, 2]),
            wrapped("result_text", "beta", &[4, 5, 6]),
        ];
        let synthesized = synthesize_collection_program(&support).expect("count synthesis");
        let heldout = wrapped("encoded_value", "gamma", &[7, 8, 9, 10]);

        assert_eq!(
            execute_response(&synthesized.program, "", &heldout.provider_payload)
                .response
                .as_deref(),
            Some("4")
        );
        assert!(
            verify_response_independently(&synthesized.verifier, &heldout.provider_payload, "4")
                .is_ok()
        );
    }

    #[test]
    fn synthesis_discovers_direct_projection_by_structural_scalar_type() {
        let mut support = vec![
            example("rows", "kind", "value"),
            example("entries", "tag", "amount"),
        ];
        for example in &mut support {
            example.expected_response = "[3,4,5]".to_owned();
        }
        let synthesized = synthesize_collection_program(&support).expect("direct projection");
        let heldout = example("records", "marker", "score");
        assert_eq!(
            execute_response(&synthesized.program, "", &heldout.provider_payload)
                .response
                .as_deref(),
            Some("[3,4,5]")
        );
        let ambiguous = serde_json::json!({
            "input":[{"type":"function_call_output","output":serde_json::json!({
                "records":[
                    {"marker":"keep","score":3,"rank":30},
                    {"marker":"drop","score":4,"rank":40}
                ]
            }).to_string()}]
        });
        assert_eq!(
            execute_response(&synthesized.program, "", &ambiguous).status,
            ResponseExecutionStatus::Abstain
        );
    }

    #[test]
    fn strict_synthesis_refuses_top_one_when_multiple_programs_survive() {
        let support = [example("rows", "kind", "value")];
        assert_eq!(
            synthesize_unique_collection_program(&support),
            Err("collection_ambiguous_programs")
        );
    }

    #[test]
    fn source_neutral_version_space_contains_no_field_names_or_observed_literals() {
        let mut input = example("private_rows", "client_kind", "customer_amount");
        input.expected_response = "3".to_owned();
        let version_space =
            enumerate_source_neutral_collection_programs(&input).expect("version space");
        assert!(!version_space.programs.is_empty());
        for program in version_space.programs {
            assert!(is_source_neutral_collection_program(&program));
            let encoded = serde_json::to_string(&program).expect("program json");
            for private in ["private_rows", "client_kind", "customer_amount", "keep"] {
                assert!(!encoded.contains(private), "program leaked {private}");
            }
        }
    }

    #[test]
    fn dynamic_coverage_reports_literal_tool_derived_upper_bound_without_storing_text() {
        let example = CollectionSynthesisExample {
            provider_payload: serde_json::json!({
                "input":[{
                    "type":"function_call_output",
                    "output":"picked: 42\ntotal: 99"
                }]
            }),
            expected_response: "Selected 42 of 99".to_owned(),
        };
        let diagnostic = diagnose_response_dynamic_coverage(&example);
        assert_eq!(diagnostic.response_bytes, 17);
        assert_eq!(diagnostic.dynamic_bytes, 4);
        assert_eq!(diagnostic.request_dynamic_bytes, 0);
        assert_eq!(diagnostic.tool_dynamic_bytes, 4);
        assert!(diagnostic.matching_selectors >= 2);
    }

    #[test]
    fn semantic_canonical_projection_requires_one_unambiguous_value_and_flexible_surface() {
        let program = ResponseProgram::project_selected_value(
            ResponseValueSelector::UniqueTurnScalar {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        let canonical = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Сколько успешных записей?"}]},
                    {"type":"function_call_output","output":"{\"ok\":3}"}
                ]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        assert!(response_program_matches_example(&program, &canonical));
        assert!(!response_program_exactly_matches_example(
            &program, &canonical
        ));

        let ambiguous = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Сколько записей?"}]},
                    {"type":"function_call_output","output":"{\"ok\":3,\"failed\":2}"}
                ]
            }),
            expected_response: "Успешных: 3, ошибочных: 2".to_owned(),
        };
        assert!(!response_program_matches_example(&program, &ambiguous));
        assert!(!response_program_authority_matches_example(
            &program, &ambiguous
        ));

        let exact_surface = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Ответь ровно: Успешных записей: 3"}]},
                    {"type":"function_call_output","output":"{\"ok\":3}"}
                ]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        assert!(!response_program_matches_example(&program, &exact_surface));

        let json_discussion = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Прочитай JSON и скажи, сколько успешных записей"}]},
                    {"type":"function_call_output","output":"{\"ok\":3}"}
                ]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        assert!(response_program_matches_example(&program, &json_discussion));

        let json_only = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Верни только JSON"}]},
                    {"type":"function_call_output","output":"{\"ok\":3}"}
                ]
            }),
            expected_response: "Успешных записей: 3".to_owned(),
        };
        assert!(!response_program_matches_example(&program, &json_only));
    }

    #[test]
    fn request_grounded_projection_has_partial_teacher_authority_without_ordinal_leak() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Return the ok field"}]},
                    {"type":"function_call_output","output":"{\"ok\":3,\"failed\":2}"}
                ]
            }),
            expected_response: "ok=3; failed=2".to_owned(),
        };
        let grounded = ResponseProgram::project_selected_value(
            ResponseValueSelector::RequestReferencedJsonField {
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        assert!(response_program_matches_example(&grounded, &example));
        assert!(response_program_authority_matches_example(
            &grounded, &example
        ));
        let verifier = source_neutral_verifier_for_program(&grounded).expect("verifier");
        assert!(verify_response_independently(&verifier, &example.provider_payload, "3").is_ok());

        let ordinal = ResponseProgram::project_selected_value(
            ResponseValueSelector::JsonScalarOrdinal {
                ordinal: 0,
                value_type: AtomValueType::Integer,
            },
            ValueProjectionFormat::PlainText,
            "completed",
        );
        assert!(response_program_matches_example(&ordinal, &example));
        assert!(!response_program_authority_matches_example(
            &ordinal, &example
        ));
    }

    #[test]
    fn semantic_status_inside_teacher_prose_keeps_actor_verifier_parity() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input":[
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Did the command succeed?"}]},
                    {"type":"function_call_output","output":"{\"exit_code\":0}"}
                ]
            }),
            expected_response: "Status: success.".to_owned(),
        };
        let direct = ResponseProgram::project_status(
            ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::Integer,
            },
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        assert_eq!(
            execute_response(&direct, "", &example.provider_payload)
                .response
                .as_deref(),
            Some("success")
        );
        let rendered = direct.with_status_renderer(CollectionOutputRenderer::RenderTemplate {
            prefix: "Status: ".to_owned(),
            suffix: ".".to_owned(),
        });
        assert_eq!(rendered.validate(), Ok(()));
        assert_eq!(
            execute_response(&rendered, "", &example.provider_payload)
                .response
                .as_deref(),
            Some(example.expected_response.as_str())
        );
        let program = enumerate_source_neutral_response_programs(&example)
            .expect("programs")
            .programs
            .into_iter()
            .find(|program| {
                matches!(
                    &program.operation,
                    ResponseOperation::ProjectStatus {
                        mapping: ProjectStatusMapping::ZeroIsSuccess,
                        renderer: CollectionOutputRenderer::RenderTemplate { .. },
                        ..
                    }
                ) && execute_response(program, "", &example.provider_payload)
                    .response
                    .as_deref()
                    == Some(example.expected_response.as_str())
            })
            .expect("semantic status program");
        assert!(response_program_matches_example(&program, &example));
        assert!(response_program_exactly_matches_example(&program, &example));
        assert_eq!(
            execute_response(&program, "", &example.provider_payload)
                .response
                .as_deref(),
            Some(example.expected_response.as_str())
        );
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        assert!(
            verify_response_independently(
                &verifier,
                &example.provider_payload,
                &example.expected_response,
            )
            .is_ok()
        );
        assert!(
            verify_response_independently(&verifier, &example.provider_payload, "failure").is_err()
        );
    }

    #[test]
    fn long_irreducible_response_skips_impossible_scalar_surface_search() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input": [
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize it"}]},
                    {"type":"function_call_output","output":"{\"value\":3,\"ok\":true}"}
                ]
            }),
            expected_response: "unrelated prose ".repeat(300),
        };
        let coverage = diagnose_response_dynamic_coverage(&example);
        assert!(
            coverage
                .response_bytes
                .saturating_sub(coverage.dynamic_bytes)
                > 512
        );

        let version_space =
            enumerate_source_neutral_response_programs_with_coverage(&example, Some(coverage))
                .expect("bounded version space");

        assert!(version_space.programs.is_empty());
        assert_eq!(version_space.candidates_enumerated, 0);
        assert_eq!(version_space.exact_checks, 0);
    }

    #[test]
    fn long_teacher_prose_preserves_canonical_multi_scalar_law() {
        let example = CollectionSynthesisExample {
            provider_payload: json!({
                "input": [
                    {"type":"message","role":"user","content":[{"type":"input_text","text":"Summarize the result"}]},
                    {"type":"function_call_output","output":"{\"value\":3,\"ok\":true}"}
                ]
            }),
            expected_response: format!("{} value 3 and true", "x".repeat(600)),
        };
        let coverage = diagnose_response_dynamic_coverage(&example);
        assert!(
            coverage
                .response_bytes
                .saturating_sub(coverage.dynamic_bytes)
                > 512
        );

        let program =
            enumerate_source_neutral_response_programs_with_coverage(&example, Some(coverage))
                .expect("bounded version space")
                .programs
                .into_iter()
                .find(|program| response_program_authority_matches_example(program, &example))
                .expect("canonical multi-scalar law");

        assert!(!response_program_exactly_matches_example(
            &program, &example
        ));
        let response = execute_example(&program, &example)
            .response
            .expect("canonical response");
        assert!(response.lines().any(|value| value == "3"));
        assert!(response.lines().any(|value| value == "true"));
        let verifier = source_neutral_verifier_for_program(&program).expect("verifier");
        assert!(
            verify_response_independently(&verifier, &example.provider_payload, &response).is_ok()
        );
    }

    #[test]
    fn law_quotient_separates_semantic_law_from_physical_selector_adapter() {
        let project = |selector| {
            ResponseProgram::project_selected_value(
                selector,
                ValueProjectionFormat::PlainText,
                "completed",
            )
        };
        let named_alpha = project(ResponseValueSelector::JsonField {
            field: "alpha".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let named_beta = project(ResponseValueSelector::JsonField {
            field: "renamed_beta".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let ordinal_zero = project(ResponseValueSelector::JsonScalarOrdinal {
            ordinal: 0,
            value_type: AtomValueType::Integer,
        });
        let ordinal_one = project(ResponseValueSelector::JsonScalarOrdinal {
            ordinal: 1,
            value_type: AtomValueType::Integer,
        });
        let from_end = project(ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal: 0,
            value_type: AtomValueType::Integer,
        });
        let request_referenced = project(ResponseValueSelector::RequestReferencedJsonField {
            value_type: AtomValueType::Integer,
        });
        let selected_beta =
            named_alpha
                .clone()
                .with_value_renderer(CollectionOutputRenderer::RenderSequence {
                    segments: vec![
                        ResponseRenderSegment::Static {
                            text: "Result: ".to_owned(),
                        },
                        ResponseRenderSegment::Selected {
                            selector: ResponseValueSelector::JsonField {
                                field: "renamed_beta".to_owned(),
                                value_type: AtomValueType::Integer,
                            },
                            format: ValueProjectionFormat::PlainText,
                        },
                    ],
                });

        assert_eq!(
            response_law_key(&named_alpha).expect("alpha law"),
            response_law_key(&named_beta).expect("renamed law")
        );
        let named_law = response_law_key(&named_alpha).expect("named law");
        let distinct = [ordinal_zero, ordinal_one, from_end, request_referenced]
            .iter()
            .map(|program| response_law_key(program).expect("structural law"))
            .collect::<BTreeSet<_>>();
        assert_eq!(distinct, BTreeSet::from([named_law]));
        let canonical_selected =
            canonical_direct_response_program(&selected_beta).expect("selected law");
        assert_eq!(canonical_selected, named_beta);
        let boolean = project(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Boolean,
        });
        assert_ne!(
            response_law_key(&named_alpha).expect("integer law"),
            response_law_key(&boolean).expect("boolean law")
        );

        let count_from = |output_ordinal| {
            ResponseProgram::compose_collection(
                vec![
                    CollectionProgramStep::SelectTurnOutput { output_ordinal },
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ],
                ValueProjectionFormat::PlainText,
                "completed",
            )
        };
        let first_output = count_from(1);
        let second_output = count_from(2);
        assert_ne!(first_output, second_output);
        assert_eq!(
            response_law_key(&first_output).expect("first output law"),
            response_law_key(&second_output).expect("second output law")
        );
    }

    #[test]
    fn law_quotient_requires_consensus_variants_to_share_one_law() {
        let project = |selector| {
            ResponseProgram::project_selected_value(
                selector,
                ValueProjectionFormat::PlainText,
                "completed",
            )
        };
        let integer_alpha = project(ResponseValueSelector::JsonField {
            field: "alpha".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let integer_beta = project(ResponseValueSelector::JsonField {
            field: "beta".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let boolean = project(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Boolean,
        });
        let variant = |program| crate::ResponseConsensusVariant {
            program,
            allowed_layout_sha256: Vec::new(),
            required_request_atom_ids: Vec::new(),
        };
        let unanimous = ResponseProgram::unique_consensus(vec![
            variant(integer_alpha.clone()),
            variant(integer_beta),
        ]);
        assert_eq!(
            response_law_key(&unanimous).expect("unanimous consensus law"),
            response_law_key(&integer_alpha).expect("integer law")
        );

        let mixed =
            ResponseProgram::unique_consensus(vec![variant(integer_alpha), variant(boolean)]);
        assert_eq!(
            response_law_key(&mixed),
            Err("response_law_key_consensus_mixed")
        );
    }
}
