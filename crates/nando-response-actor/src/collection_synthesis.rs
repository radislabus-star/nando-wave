use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AtomValueType, CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, OutputGraphSegment, OutputValueCandidate, OutputValueSource,
    ProjectStatusMapping, ResponseExecutionStatus, ResponseOperation, ResponseProgram,
    ResponseRenderSegment, ResponseScalarLiteral, ResponseValueSelector, ValueProjectionFormat,
    VerifierProgram, build_output_graph, execute_response,
};

const MAX_SEARCH_ROWS: usize = 1_024;
const MAX_CANDIDATES: usize = 16_384;

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
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResponseCoverageDiagnostic {
    pub response_bytes: usize,
    pub dynamic_bytes: usize,
    pub request_dynamic_bytes: usize,
    pub tool_dynamic_bytes: usize,
    pub matching_selectors: usize,
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
        let Some(value) = execute_response(&program, "", &example.provider_payload).response else {
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
    }
}

pub fn enumerate_source_neutral_collection_programs(
    example: &CollectionSynthesisExample,
) -> Result<CollectionVersionSpace, &'static str> {
    let first = latest_output_json(&example.provider_payload)?;
    let candidates = expand_output_renderers(enumerate_candidates(&first)?, example);
    let candidates_enumerated = candidates.len();
    let mut exact_checks = 0_usize;
    let mut semantic_classes = BTreeMap::new();
    for program in candidates {
        if !is_source_neutral_collection_program(&program) {
            continue;
        }
        exact_checks = exact_checks.saturating_add(1);
        let execution = execute_response(&program, "", &example.provider_payload);
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
    })
}

pub fn enumerate_source_neutral_response_programs(
    example: &CollectionSynthesisExample,
) -> Result<CollectionVersionSpace, &'static str> {
    let mut candidates = Vec::new();
    let selectors = learned_selector_candidates(&example.provider_payload);
    if let Ok(root) = latest_output_json(&example.provider_payload)
        && let Ok(collection) = enumerate_candidates(&root)
    {
        candidates.extend(compose_collection_render_sequence_candidates(
            example,
            &collection,
            &selectors,
        ));
        candidates.extend(expand_output_renderers(collection, example));
    }
    let mut policy_rejected_exact_matches = 0_usize;
    let mut policy_rejection_reasons = BTreeMap::<String, usize>::new();
    for selector in &selectors {
        let value_type = selector_value_type(selector);
        for format in [
            ValueProjectionFormat::PlainText,
            ValueProjectionFormat::CanonicalJson,
        ] {
            candidates.extend(expand_output_renderers(
                vec![ResponseProgram::project_selected_value(
                    selector.clone(),
                    format,
                    "completed",
                )],
                example,
            ));
        }
        if value_type == AtomValueType::Integer {
            for mapping in [
                ProjectStatusMapping::ZeroIsSuccess,
                ProjectStatusMapping::ZeroIsPass,
                ProjectStatusMapping::ZeroIsOk,
                ProjectStatusMapping::ZeroIsTrue,
            ] {
                candidates.push(ResponseProgram::project_status(
                    selector.clone(),
                    mapping,
                    "completed",
                ));
            }
        }
    }
    if let Some(program) = compose_render_sequence_candidate(example, &selectors) {
        if let Err(reason) = program.validate() {
            policy_rejected_exact_matches = policy_rejected_exact_matches.saturating_add(1);
            *policy_rejection_reasons
                .entry(reason.to_owned())
                .or_default() += 1;
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
        if !is_learned_bounded_response_program(&program) {
            continue;
        }
        exact_checks = exact_checks.saturating_add(1);
        let match_quality = response_program_match_quality(&program, example);
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
    })
}

#[must_use]
pub fn response_program_matches_example(
    program: &ResponseProgram,
    example: &CollectionSynthesisExample,
) -> bool {
    response_program_match_quality(program, example) > 0
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
    let execution = execute_response(program, "", &example.provider_payload);
    if execution.status != ResponseExecutionStatus::Executed {
        return 0;
    }
    let Some(response) = execution.response.as_deref() else {
        return 0;
    };
    if response == example.expected_response {
        return 2;
    }
    let ResponseOperation::ProjectSelectedValue {
        format: ValueProjectionFormat::PlainText,
        renderer: CollectionOutputRenderer::Direct,
        ..
    } = &program.operation
    else {
        return 0;
    };
    if response.is_empty()
        || response.len() > 128
        || !contains_token(&example.expected_response, response)
    {
        return 0;
    }
    let Some(output) = latest_tool_output(&example.provider_payload) else {
        return 0;
    };
    let Some(value) = unique_embedded_json_output(output) else {
        return 0;
    };
    let mut scalars = Vec::new();
    collect_json_scalars(&value, &mut scalars);
    scalars.sort();
    scalars.dedup();
    u8::from(
        scalars.into_iter().any(|scalar| {
            scalar == response && contains_token(&example.expected_response, &scalar)
        }),
    )
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

fn compose_render_sequence_candidate(
    example: &CollectionSynthesisExample,
    selectors: &[ResponseValueSelector],
) -> Option<ResponseProgram> {
    let resolved = selectors
        .iter()
        .filter_map(|selector| {
            let program = ResponseProgram::project_selected_value(
                selector.clone(),
                ValueProjectionFormat::PlainText,
                "completed",
            );
            let value = execute_response(&program, "", &example.provider_payload).response?;
            ((value.len() >= 2 || selector_value_type(selector) != AtomValueType::String)
                && example.expected_response.contains(&value))
            .then(|| OutputValueCandidate {
                source: OutputValueSource::Selector(selector.clone()),
                rendered: value,
            })
        })
        .collect::<Vec<_>>();
    if resolved.is_empty() {
        return None;
    }
    let graph = build_output_graph(&example.expected_response, resolved).ok()?;
    if graph.source_ambiguous {
        return None;
    }
    let dynamic_count = graph
        .segments
        .iter()
        .filter(|segment| matches!(segment, OutputGraphSegment::RuntimeValue { .. }))
        .count();
    if dynamic_count < 2 {
        return None;
    }
    let primary_selector = graph.segments.iter().find_map(|segment| match segment {
        OutputGraphSegment::RuntimeValue { sources, .. } => sources.iter().find_map(|source| {
            let OutputValueSource::Selector(selector) = source else {
                return None;
            };
            Some(selector.clone())
        }),
        OutputGraphSegment::Static { .. } => None,
    })?;
    let mut segments = Vec::new();
    for segment in graph.segments {
        match segment {
            OutputGraphSegment::Static { text } => {
                segments.push(ResponseRenderSegment::Static { text });
            }
            OutputGraphSegment::RuntimeValue { sources, .. } => {
                let selector = sources.into_iter().find_map(|source| {
                    let OutputValueSource::Selector(selector) = source else {
                        return None;
                    };
                    Some(selector)
                })?;
                if selector == primary_selector {
                    segments.push(ResponseRenderSegment::Primary);
                } else {
                    segments.push(ResponseRenderSegment::Selected {
                        selector,
                        format: ValueProjectionFormat::PlainText,
                    });
                }
            }
        }
    }
    Some(
        ResponseProgram::project_selected_value(
            primary_selector,
            ValueProjectionFormat::PlainText,
            "completed",
        )
        .with_value_renderer(CollectionOutputRenderer::RenderSequence { segments }),
    )
}

fn compose_collection_render_sequence_candidates(
    example: &CollectionSynthesisExample,
    programs: &[ResponseProgram],
    selectors: &[ResponseValueSelector],
) -> Vec<ResponseProgram> {
    let selected_values = selectors
        .iter()
        .filter_map(|selector| {
            let program = ResponseProgram::project_selected_value(
                selector.clone(),
                ValueProjectionFormat::PlainText,
                "completed",
            );
            let rendered = execute_response(&program, "", &example.provider_payload).response?;
            example
                .expected_response
                .contains(&rendered)
                .then(|| OutputValueCandidate {
                    source: OutputValueSource::Selector(selector.clone()),
                    rendered,
                })
        })
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    for program in programs {
        let Some(computed) = execute_response(program, "", &example.provider_payload).response
        else {
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

fn learned_selector_candidates(payload: &Value) -> Vec<ResponseValueSelector> {
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
        | ResponseValueSelector::TurnOutputLine { value_type, .. } => *value_type,
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
            CollectionProgramStep::SelectOnlyArrayField
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
        ResponseOperation::ProjectSelectedValue { selector, .. }
        | ResponseOperation::ProjectStatus { selector, .. } => {
            matches!(
                selector,
                ResponseValueSelector::UniqueScalar { .. }
                    | ResponseValueSelector::UniqueTurnScalar { .. }
            )
        }
        ResponseOperation::ComposeCollection { .. } => {
            is_source_neutral_collection_program(program)
        }
        _ => false,
    }
}

#[must_use]
pub fn is_privacy_safe_online_response_program(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. } => program.validate().is_ok(),
        ResponseOperation::ComposeCollection { renderer, .. } => {
            is_source_neutral_collection_program(program)
                && match renderer {
                    CollectionOutputRenderer::Direct
                    | CollectionOutputRenderer::RenderTemplate { .. }
                    | CollectionOutputRenderer::RenderSequence { .. } => program.validate().is_ok(),
                }
        }
        _ => false,
    }
}

#[must_use]
pub fn is_learned_bounded_response_program(program: &ResponseProgram) -> bool {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::ProjectStatus { .. } => true,
        ResponseOperation::ComposeCollection { steps, .. } => steps.iter().all(|step| {
            matches!(
                step,
                CollectionProgramStep::SelectOnlyArrayField
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
            completion_state,
        } => Ok(VerifierProgram::ProjectStatus {
            selector: selector.clone(),
            mapping: *mapping,
            completion_state: completion_state.clone(),
            require_unique_value: true,
        }),
        ResponseOperation::ComposeCollection { .. } => collection_verifier_for_program(program),
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
    let first = latest_output_json(&support[0].provider_payload)?;
    let candidates = expand_output_renderers(enumerate_candidates(&first)?, &support[0]);
    let candidates_enumerated = candidates.len();
    let mut exact_checks = 0_usize;
    let mut survivors = Vec::new();
    for program in candidates {
        let mut consistent = true;
        for example in support {
            exact_checks = exact_checks.saturating_add(1);
            let execution = execute_response(&program, "", &example.provider_payload);
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
                } => *format,
                _ => ValueProjectionFormat::PlainText,
            };
            serde_json::to_vec(&(selector, effective_format, renderer, completion_state))
                .map_err(|_| "response_semantic_key_encode")
        }
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            completion_state,
        } => serde_json::to_vec(&(selector, mapping, completion_state))
            .map_err(|_| "response_semantic_key_encode"),
        _ => Err("response_semantic_key_kind"),
    }
}

fn expand_output_renderers(
    candidates: Vec<ResponseProgram>,
    example: &CollectionSynthesisExample,
) -> Vec<ResponseProgram> {
    let mut expanded = Vec::with_capacity(candidates.len().saturating_mul(2));
    for program in candidates {
        let execution = execute_response(&program, "", &example.provider_payload);
        if let Some(computed) = execution
            .response
            .as_deref()
            .filter(|computed| !computed.is_empty())
        {
            let mut matches = example.expected_response.match_indices(computed);
            if let Some((offset, _)) = matches.next()
                && matches.next().is_none()
                && example.expected_response != computed
            {
                let suffix_offset = offset.saturating_add(computed.len());
                let prefix = &example.expected_response[..offset];
                let suffix = &example.expected_response[suffix_offset..];
                if prefix.len().saturating_add(suffix.len()) <= 1_024 {
                    let renderer = CollectionOutputRenderer::RenderTemplate {
                        prefix: prefix.to_owned(),
                        suffix: suffix.to_owned(),
                    };
                    expanded.push(match &program.operation {
                        ResponseOperation::ProjectSelectedValue { .. } => {
                            program.clone().with_value_renderer(renderer)
                        }
                        _ => program.clone().with_collection_renderer(renderer),
                    });
                }
            }
        }
        expanded.push(program);
    }
    expanded.sort_by(|left, right| {
        serde_json::to_vec(left)
            .unwrap_or_default()
            .cmp(&serde_json::to_vec(right).unwrap_or_default())
    });
    expanded.dedup();
    expanded
}

fn latest_output_json(payload: &Value) -> Result<Value, &'static str> {
    let output = latest_tool_output(payload).ok_or("collection_support_output_missing")?;
    unique_embedded_json_output(output).ok_or("collection_support_output_not_json")
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
        if trimmed == "```" {
            if let Some(value) = fenced.take() {
                sources.push(value);
            }
        } else if trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if let Some(value) = &mut fenced {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
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
            let direct_candidate = compose_render_sequence_candidate(&example, &selectors)
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
}
