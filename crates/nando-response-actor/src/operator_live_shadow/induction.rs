//! Source-neutral live-trace extraction and bounded actor-hypothesis induction.
//!
//! This compiler produces candidates only; it does not grant runtime authority.

use super::*;

/// Converts one completed, verified trace into source-neutral circuit evidence.
/// The teacher response selects a hypothesis only after the action; runtime
/// binding remains restricted to the pre-action provider payload.
pub fn extract_live_scalar_circuit_sample(
    transition: &TeacherTransition,
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    if !transition.outcome.verifier.accepted {
        return Err(LiveScalarShadowBlocker::TeacherRejected);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(LiveScalarShadowBlocker::MissingParityCase)?;
    let payload_bytes = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::PayloadSerializationFailed)?;
    if payload_bytes.len() > LIVE_SCALAR_MAX_PROVIDER_PAYLOAD_BYTES
        || parity.expected_response.len() > 4 * 1024
    {
        return Err(LiveScalarShadowBlocker::PayloadTooLarge);
    }
    let synthesis_payload =
        synthesis_payload_with_request(&parity.request_text, &parity.provider_payload)?;
    let example = CollectionSynthesisExample {
        provider_payload: synthesis_payload.clone(),
        expected_response: parity.expected_response.clone(),
    };
    let version_space = enumerate_source_neutral_response_programs(&example).map_err(|error| {
        if error == "collection_candidate_budget" {
            LiveScalarShadowBlocker::CandidateBudgetExhausted
        } else {
            LiveScalarShadowBlocker::ProgramEnumerationFailed
        }
    })?;
    let teacher_programs = teacher_action_programs(transition, parity, &synthesis_payload);
    let teacher_programs = match teacher_programs {
        Ok(programs) => programs,
        Err(blocker) if version_space.programs.is_empty() => return Err(blocker),
        Err(_) => Vec::new(),
    };
    if version_space.programs.is_empty() && teacher_programs.is_empty() {
        return Err(LiveScalarShadowBlocker::EmptyVersionSpace);
    }
    let exact_count = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_count_program(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let exact_status = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_status_program(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let exact_filters = version_space
        .programs
        .iter()
        .flat_map(|program| {
            derive_exact_filter_programs(
                program,
                &parity.request_text,
                &synthesis_payload,
                &parity.expected_response,
            )
        })
        .collect::<Vec<_>>();
    let exact_filter = exact_filters
        .iter()
        .min_by_key(|program| serde_cbor::to_vec(program).unwrap_or_default());
    let mut scalar_programs = version_space
        .programs
        .iter()
        .filter_map(|program| {
            derive_exact_scalar_program(
                program,
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
        })
        .filter_map(project_scalar_program)
        .collect::<Vec<_>>();
    scalar_programs.sort_by(|left, right| left.0.cmp(&right.0));
    let rich_exact = version_space
        .programs
        .iter()
        .find(|program| rich_scalar_program_roles(program).is_some_and(|roles| roles.len() > 1));
    let selected_template = if let Some(program) = teacher_programs.first() {
        program.clone()
    } else if let Some(program) = &exact_count {
        program.clone()
    } else if let Some(program) = &exact_status {
        program.clone()
    } else if let Some(program) = exact_filter {
        program.clone()
    } else if let Some(program) = rich_exact {
        program.clone()
    } else if let Some((_, selector, _, _, renderer)) = scalar_programs.first() {
        ResponseProgram::project_selected_value(selector.clone(), scalar_programs[0].3, "completed")
            .with_value_renderer(renderer.clone())
    } else {
        version_space
            .programs
            .iter()
            .find(|program| rich_scalar_program_roles(program).is_some())
            .cloned()
            .ok_or_else(|| classify_exact_program_blocker(&version_space.programs))?
    };
    let exact_typed = exact_count.as_ref().or(exact_status.as_ref());
    let mut actor_hypotheses = if !teacher_programs.is_empty() {
        teacher_programs
            .iter()
            .filter_map(|program| {
                canonicalize_scalar_program_roles(
                    program,
                    &parity.request_text,
                    &parity.provider_payload,
                )
            })
            .collect()
    } else if !exact_filters.is_empty() {
        exact_filters
            .iter()
            .filter_map(|program| {
                canonicalize_scalar_program_roles(
                    program,
                    &parity.request_text,
                    &parity.provider_payload,
                )
            })
            .collect()
    } else if let Some(program) = exact_typed {
        vec![
            canonicalize_scalar_program_roles(
                program,
                &parity.request_text,
                &parity.provider_payload,
            )
            .ok_or(LiveScalarShadowBlocker::ExactTypedCanonicalizationFailed)?,
        ]
    } else {
        canonical_rich_actor_hypotheses(
            &version_space.programs,
            &parity.request_text,
            &parity.provider_payload,
            &parity.expected_response,
        )?
    };
    let clean_actor = (exact_typed.is_none() && exact_filters.is_empty())
        .then(|| {
            derive_clean_ordinal_actor(
                &parity.request_text,
                &parity.provider_payload,
                &parity.expected_response,
            )
        })
        .flatten();
    if let Some(actor) = &clean_actor {
        actor_hypotheses.push(actor.clone());
    }
    if actor_hypotheses.is_empty() {
        actor_hypotheses.push(
            canonicalize_scalar_program_roles(
                &selected_template,
                &parity.request_text,
                &parity.provider_payload,
            )
            .ok_or(LiveScalarShadowBlocker::SelectedTemplateCanonicalizationFailed)?,
        );
    }
    actor_hypotheses = expand_actor_hypothesis_set(
        actor_hypotheses,
        &parity.request_text,
        &parity.provider_payload,
        &parity.expected_response,
    )?;
    let canonical_candidates = if let Some(clean_actor) = clean_actor {
        expand_actor_hypothesis_set(
            vec![clean_actor],
            &parity.request_text,
            &parity.provider_payload,
            &parity.expected_response,
        )?
    } else {
        actor_hypotheses.clone()
    };
    let canonical_program = canonical_candidates
        .iter()
        .max_by_key(|program| rich_scalar_program_roles(program).map_or(0, |roles| roles.len()))
        .cloned()
        .ok_or(LiveScalarShadowBlocker::CanonicalCandidateMissing)?;
    finish_live_scalar_circuit_sample(
        transition,
        parity,
        payload_bytes,
        canonical_program,
        actor_hypotheses,
    )
}

pub(super) fn reextract_live_scalar_circuit_sample(
    transition: &TeacherTransition,
    support_actor_hypotheses: &[ResponseProgram],
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    if !transition.outcome.verifier.accepted {
        return Err(LiveScalarShadowBlocker::TeacherRejected);
    }
    let parity = transition
        .runtime_parity_case
        .as_ref()
        .ok_or(LiveScalarShadowBlocker::MissingParityCase)?;
    let payload_bytes = serde_json::to_vec(&parity.provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::PayloadSerializationFailed)?;
    if payload_bytes.len() > LIVE_SCALAR_MAX_PROVIDER_PAYLOAD_BYTES
        || parity.expected_response.len() > 4 * 1024
    {
        return Err(LiveScalarShadowBlocker::PayloadTooLarge);
    }
    let provider_view =
        crate::runtime::provider_payload_view(&parity.request_text, &parity.provider_payload)
            .map_err(|_| LiveScalarShadowBlocker::ProgramEnumerationFailed)?;
    let actor_hypotheses = support_actor_hypotheses
        .iter()
        .filter(|program| {
            execute_response(program, &parity.request_text, provider_view.as_ref())
                .response
                .as_deref()
                .is_some_and(|response| {
                    response == parity.expected_response
                        || crate::online_admission::responses_match_after_execution_budget_normalization(
                            response,
                            &parity.expected_response,
                        )
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    let canonical_program = actor_hypotheses
        .iter()
        .max_by_key(|program| rich_scalar_program_roles(program).map_or(0, |roles| roles.len()))
        .cloned()
        .ok_or(LiveScalarShadowBlocker::NoExactSourceNeutralProgram)?;
    finish_live_scalar_circuit_sample(
        transition,
        parity,
        payload_bytes,
        canonical_program,
        actor_hypotheses,
    )
}

fn finish_live_scalar_circuit_sample(
    transition: &TeacherTransition,
    parity: &crate::RuntimeParityCase,
    payload_bytes: Vec<u8>,
    canonical_program: ResponseProgram,
    actor_hypotheses: Vec<ResponseProgram>,
) -> Result<LiveScalarCircuitSample, LiveScalarShadowBlocker> {
    // The relation circuit and executable actor must be derived from the same
    // canonical winner program; otherwise runtime roles cannot match support.
    let roles = rich_scalar_program_roles(&canonical_program)
        .ok_or_else(|| classify_exact_program_blocker(&actor_hypotheses))?;

    let lineage_sha256 = parse_commitment(&transition.before.session_id_sha256)
        .or_else(|| parse_commitment(&transition.before.client_intent_id_sha256))
        .ok_or(LiveScalarShadowBlocker::InvalidCommitment)?;
    let surface_sha256 = digest_parts(
        b"nando.live-scalar-surface.v2",
        &[
            transition.before.frame_id_sha256.as_bytes(),
            transition.before.extractor_version.as_bytes(),
            &payload_bytes,
        ],
    );
    let observed = observed_rich_scalar_surface(
        &parity.request_text,
        &parity.provider_payload,
        &roles,
        program_transform_opcode(&canonical_program)
            .ok_or(LiveScalarShadowBlocker::UnsupportedTransformOpcode)?,
        program_transform_flags(&canonical_program)
            .ok_or(LiveScalarShadowBlocker::UnsupportedTransformFlags)?,
        program_has_filter_count(&canonical_program),
        &transition.before.frame_id_sha256,
        lineage_sha256,
        surface_sha256,
    )?;
    let raw_input_sha256 = digest_parts(
        b"nando.live-scalar-raw-input.v1",
        &[parity.request_text.as_bytes(), &payload_bytes],
    );
    let law_shape = structural_scalar_law_shape(
        &parity.request_text,
        &parity.provider_payload,
        &parity.expected_response,
    )
    .or_else(|| {
        (roles.len() == 1
            || program_transform_opcode(&canonical_program)
                == Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE))
        .then(|| source_neutral_scalar_program_shape(&canonical_program))
        .flatten()
    })
    .ok_or(LiveScalarShadowBlocker::LawShapeMissing)?;
    let law_sha256 = digest_parts(b"nando.live-scalar-law.v4", &[&law_shape]);
    Ok(LiveScalarCircuitSample {
        bundle: observed.bundle,
        anchors: observed.anchors,
        actor_template: canonical_program,
        actor_hypotheses: actor_hypotheses.into_boxed_slice(),
        request_text: parity.request_text.clone(),
        provider_payload: parity.provider_payload.clone(),
        expected_response: parity.expected_response.clone(),
        raw_input_sha256,
        extractor_version: extractor_version(&transition.before.extractor_version),
        law_sha256,
    })
}

fn teacher_action_programs(
    transition: &TeacherTransition,
    parity: &crate::RuntimeParityCase,
    provider_payload: &Value,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let frame = transition.as_training_relation_frame();
    let synthesized = synthesize_response_operator(std::slice::from_ref(&frame)).map_err(
        |error| match error {
            crate::SynthesisError::EmptySupport => {
                LiveScalarShadowBlocker::TeacherSynthesisEmptySupport
            }
            crate::SynthesisError::AmbiguousRoles => {
                LiveScalarShadowBlocker::TeacherSynthesisAmbiguousRoles
            }
            crate::SynthesisError::InconsistentRoleFamily => {
                LiveScalarShadowBlocker::TeacherSynthesisInconsistentRoleFamily
            }
            crate::SynthesisError::MissingPendingState => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingPendingState
            }
            crate::SynthesisError::MissingCompletionState => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingCompletionState
            }
            crate::SynthesisError::MissingUniqueHandle => {
                LiveScalarShadowBlocker::TeacherSynthesisMissingUniqueHandle
            }
            crate::SynthesisError::NoConsistentProgram => {
                LiveScalarShadowBlocker::TeacherSynthesisNoConsistentProgram
            }
            crate::SynthesisError::AmbiguousPrograms => {
                LiveScalarShadowBlocker::TeacherSynthesisAmbiguousPrograms
            }
        },
    )?;
    let program = canonicalize_call_execution_arguments(synthesized.candidate.program);
    if !matches!(
        &program.operation,
        ResponseOperation::FunctionCallFromRoles { .. }
            | ResponseOperation::CustomToolCallFromRoles { .. }
    ) {
        return Err(LiveScalarShadowBlocker::TeacherProgramNotCall);
    }
    let selector_type = rich_scalar_program_roles(&program)
        .and_then(|roles| roles.first().map(|role| selector_value_type(&role.0)))
        .ok_or(LiveScalarShadowBlocker::RoleTypeInferenceFailed)?;
    let mut candidates = vec![program.clone()];
    let teacher_alignment = match teacher_call_role_value(&program, &parity.expected_response) {
        Some(teacher_value) => {
            match crate::runtime::structural_output_selectors_for_teacher_value(
                provider_payload,
                &teacher_value,
                selector_type,
            ) {
                Ok(selectors) => {
                    candidates.extend(
                        selectors
                            .into_iter()
                            .filter_map(|selector| replace_call_selector(&program, selector)),
                    );
                    LiveScalarShadowBlocker::TeacherProgramRoleValueCandidateMismatch
                }
                Err(_) => teacher_role_value_location(
                    &parity.request_text,
                    provider_payload,
                    &teacher_value,
                ),
            }
        }
        None => LiveScalarShadowBlocker::TeacherProgramRoleValueUnavailable,
    };
    if let ResponseOperation::FunctionCallFromRoles { selector, .. }
    | ResponseOperation::CustomToolCallFromRoles { selector, .. } = &program.operation
        && let Some((field, value_type)) = teacher_field_selector_hint(selector)
        && let Ok(selectors) = crate::runtime::structural_output_selectors_for_field_hint(
            provider_payload,
            field,
            value_type,
        )
    {
        candidates.extend(
            selectors
                .into_iter()
                .filter_map(|selector| replace_call_selector(&program, selector)),
        );
    }
    candidates.extend(
        crate::collection_synthesis::learned_selector_candidates(provider_payload)
            .into_iter()
            .filter(|selector| {
                selector_value_type(selector) == selector_type
                    && teacher_call_selector_is_structural(selector)
            })
            .filter_map(|selector| replace_call_selector(&program, selector)),
    );
    // Collection synthesis intentionally caps broad ordinal expansion at 16.
    // Tool-call outputs can contain larger diagnostic objects, while the
    // continuation role can remain outside the broad 16-scalar search.
    // Extend only this typed call version space, preferring tail positions
    // where continuation handles commonly occur, without using field names.
    for ordinal in 16_u16..64 {
        for selector in [
            ResponseValueSelector::LatestTurnOutputScalarFromEnd {
                reverse_ordinal: ordinal,
                value_type: selector_type,
            },
            ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                scalar_ordinal: ordinal,
                value_type: selector_type,
            },
        ] {
            if let Some(candidate) = replace_call_selector(&program, selector) {
                candidates.push(candidate);
            }
        }
    }
    let mut exact = BTreeMap::new();
    let mut evaluated = BTreeSet::new();
    let mut first_execution = None;
    let mut first_structural_response = None;
    for candidate in candidates {
        let candidate_key = serde_cbor::to_vec(&candidate)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        if !evaluated.insert(candidate_key.clone()) {
            continue;
        }
        if evaluated.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::CandidateBudgetExhausted);
        }
        let execution = execute_response(&candidate, &parity.request_text, provider_payload);
        first_execution.get_or_insert_with(|| (candidate.clone(), execution.clone()));
        if let Some(response) = &execution.response {
            first_structural_response.get_or_insert_with(|| (candidate.clone(), response.clone()));
        }
        if !execution.response.as_deref().is_some_and(|response| {
            response == parity.expected_response
                || crate::online_admission::responses_match_after_execution_budget_normalization(
                    response,
                    &parity.expected_response,
                )
        }) {
            continue;
        }
        exact.entry(candidate_key).or_insert(candidate);
        // One surface may expose the same semantic role through many physical
        // paths. Preserve the bounded adapter version space here; the support
        // intersection must shrink it to the 64-variant authority limit.
        if exact.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
    }
    if exact.is_empty() {
        if let Some((candidate, response)) = first_structural_response {
            let blocker = teacher_program_parity_blocker(
                &parity.expected_response,
                &response,
                Some(&candidate),
            );
            return Err(dynamic_role_alignment_blocker(blocker, teacher_alignment));
        }
        let (candidate, execution) =
            first_execution.ok_or(LiveScalarShadowBlocker::TeacherProgramRuntimeAbstain)?;
        return Err(match execution.response {
            Some(response) => dynamic_role_alignment_blocker(
                teacher_program_parity_blocker(
                    &parity.expected_response,
                    &response,
                    Some(&candidate),
                ),
                teacher_alignment,
            ),
            None => teacher_program_runtime_blocker(&execution.reason),
        });
    }
    Ok(exact.into_values().collect())
}

fn canonicalize_call_execution_arguments(mut program: ResponseProgram) -> ResponseProgram {
    let arguments = match &mut program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return program,
    };
    arguments.retain(|argument| match argument {
        ResponseArgument::Integer { name, .. } => {
            !crate::teacher_join::is_execution_budget_argument(name)
        }
        ResponseArgument::String { name, value } => !(name == "chars" && value.is_empty()),
        _ => true,
    });
    program
}

fn dynamic_role_alignment_blocker(
    blocker: LiveScalarShadowBlocker,
    alignment: LiveScalarShadowBlocker,
) -> LiveScalarShadowBlocker {
    if matches!(
        blocker,
        LiveScalarShadowBlocker::TeacherProgramDynamicRoleNumericMismatch
            | LiveScalarShadowBlocker::TeacherProgramDynamicRoleStringMismatch
    ) {
        alignment
    } else {
        blocker
    }
}

fn teacher_call_role_value(program: &ResponseProgram, expected_response: &str) -> Option<Value> {
    let response = serde_json::from_str::<Value>(expected_response).ok()?;
    let arguments = if let Some(input) = response.get("input").and_then(Value::as_str) {
        call_input_arguments(input)?
    } else {
        response.get("arguments")?.clone()
    };
    let arguments = arguments.as_object()?;
    let role_name = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => {
            arguments.iter().find_map(|argument| match argument {
                ResponseArgument::Role { name, .. } => Some(name.as_str()),
                _ => None,
            })?
        }
        _ => return None,
    };
    arguments.get(role_name).cloned()
}

fn teacher_role_value_location(
    request_text: &str,
    provider_payload: &Value,
    teacher_value: &Value,
) -> LiveScalarShadowBlocker {
    let Some(text) = scalar_value_text(teacher_value) else {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueNotObserved;
    };
    if contains_bounded_token(request_text, &text) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInRequestText;
    }
    if payload_contains_exact_scalar(provider_payload, teacher_value) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInPayloadScalar;
    }
    if payload_contains_text_token(provider_payload, &text) {
        return LiveScalarShadowBlocker::TeacherProgramRoleValueInPayloadText;
    }
    LiveScalarShadowBlocker::TeacherProgramRoleValueAbsentFromPayload
}

fn scalar_value_text(value: &Value) -> Option<String> {
    match value {
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) if value.is_i64() || value.is_u64() => Some(value.to_string()),
        Value::String(value) if !value.is_empty() && value.len() <= 512 => Some(value.clone()),
        _ => None,
    }
}

fn payload_contains_exact_scalar(value: &Value, target: &Value) -> bool {
    if value == target {
        return true;
    }
    match value {
        Value::Array(values) => values
            .iter()
            .any(|value| payload_contains_exact_scalar(value, target)),
        Value::Object(values) => values
            .values()
            .any(|value| payload_contains_exact_scalar(value, target)),
        _ => false,
    }
}

fn payload_contains_text_token(value: &Value, target: &str) -> bool {
    match value {
        Value::String(value) => contains_bounded_token(value, target),
        Value::Array(values) => values
            .iter()
            .any(|value| payload_contains_text_token(value, target)),
        Value::Object(values) => values
            .values()
            .any(|value| payload_contains_text_token(value, target)),
        _ => false,
    }
}

fn contains_bounded_token(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || needle.len() > 512 || haystack.len() > 64 * 1024 {
        return false;
    }
    haystack.match_indices(needle).any(|(start, _)| {
        let end = start.saturating_add(needle.len());
        let left = haystack[..start].chars().next_back();
        let right = haystack[end..].chars().next();
        !left.is_some_and(is_identifier_character) && !right.is_some_and(is_identifier_character)
    })
}

fn is_identifier_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':' | '/')
}

// Field labels are post-action teacher hints only; crystallized actors retain
// the ordinal selector derived from the observed output, never this label.
pub(super) fn teacher_field_selector_hint(
    selector: &ResponseValueSelector,
) -> Option<(&str, AtomValueType)> {
    match selector {
        ResponseValueSelector::JsonField { field, value_type }
        | ResponseValueSelector::UniqueTurnJsonField { field, value_type }
        | ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            Some((field, *value_type))
        }
        _ => None,
    }
}

fn replace_call_selector(
    program: &ResponseProgram,
    selector: ResponseValueSelector,
) -> Option<ResponseProgram> {
    let mut candidate = program.clone();
    match &mut candidate.operation {
        ResponseOperation::FunctionCallFromRoles {
            selector: current, ..
        }
        | ResponseOperation::CustomToolCallFromRoles {
            selector: current, ..
        } => *current = selector,
        _ => return None,
    }
    Some(candidate)
}

fn teacher_call_selector_is_structural(selector: &ResponseValueSelector) -> bool {
    !matches!(
        selector,
        ResponseValueSelector::JsonField { .. }
            | ResponseValueSelector::UniqueTurnJsonField { .. }
            | ResponseValueSelector::UniqueActiveTurnJsonField { .. }
    )
}

fn teacher_program_runtime_blocker(reason: &str) -> LiveScalarShadowBlocker {
    if reason.starts_with("verification:") {
        return LiveScalarShadowBlocker::TeacherProgramVerificationFailed;
    }
    if reason.starts_with("invalid_program:") {
        return LiveScalarShadowBlocker::TeacherProgramInvalid;
    }
    match reason {
        "immediate_tool_output_missing" => {
            LiveScalarShadowBlocker::TeacherProgramImmediateToolOutputMissing
        }
        "selector_prefix_missing" => LiveScalarShadowBlocker::TeacherProgramSelectorPrefixMissing,
        "selector_prefix_ambiguous" => {
            LiveScalarShadowBlocker::TeacherProgramSelectorPrefixAmbiguous
        }
        "selector_type_mismatch" => LiveScalarShadowBlocker::TeacherProgramSelectorTypeMismatch,
        "selector_integer_parse"
        | "selector_boolean_parse"
        | "selector_collection_parse"
        | "selector_collection_unsupported"
        | "selector_scalar_unsupported" => {
            LiveScalarShadowBlocker::TeacherProgramSelectorParseFailed
        }
        "role_integer_parse"
        | "role_boolean_parse"
        | "role_string_parse"
        | "role_collection_unsupported"
        | "unsupported_runtime_role" => LiveScalarShadowBlocker::TeacherProgramRoleParseFailed,
        "scalar_output_budget"
        | "tool_output_not_text"
        | "output_part_cardinality"
        | "unsupported_output_part_type"
        | "output_part_text_missing" => LiveScalarShadowBlocker::TeacherProgramOutputTextInvalid,
        _ => LiveScalarShadowBlocker::TeacherProgramRuntimeAbstain,
    }
}

fn teacher_program_parity_blocker(
    expected: &str,
    actual: &str,
    program: Option<&ResponseProgram>,
) -> LiveScalarShadowBlocker {
    let (Ok(expected), Ok(actual)) = (
        serde_json::from_str::<Value>(expected),
        serde_json::from_str::<Value>(actual),
    ) else {
        return LiveScalarShadowBlocker::TeacherProgramParityMismatch;
    };
    let (Some(expected), Some(actual)) = (expected.as_object(), actual.as_object()) else {
        return LiveScalarShadowBlocker::TeacherProgramResponseShapeMismatch;
    };
    let expected_kind = expected.get("kind").or_else(|| expected.get("type"));
    let actual_kind = actual.get("kind").or_else(|| actual.get("type"));
    if expected.get("name") != actual.get("name") {
        return LiveScalarShadowBlocker::TeacherProgramSymbolMismatch;
    }
    if expected.get("input") != actual.get("input") {
        let (Some(expected_input), Some(actual_input)) = (
            expected.get("input").and_then(Value::as_str),
            actual.get("input").and_then(Value::as_str),
        ) else {
            return LiveScalarShadowBlocker::TeacherProgramInputMismatch;
        };
        if expected_input
            .chars()
            .filter(|character| !character.is_ascii_whitespace())
            .eq(actual_input
                .chars()
                .filter(|character| !character.is_ascii_whitespace()))
        {
            return LiveScalarShadowBlocker::TeacherProgramInputWhitespaceMismatch;
        }
        if wire_token_shape(expected_input) == wire_token_shape(actual_input) {
            if let Some(blocker) = program.and_then(|program| {
                classify_call_argument_mismatch(expected_input, actual_input, program)
            }) {
                return blocker;
            }
            return classify_wire_token_value_mismatch(expected_input, actual_input);
        }
        return LiveScalarShadowBlocker::TeacherProgramInputSyntaxShapeMismatch;
    }
    if expected_kind == actual_kind {
        return LiveScalarShadowBlocker::TeacherProgramWireEnvelopeMismatch;
    }
    LiveScalarShadowBlocker::TeacherProgramResponseShapeMismatch
}

fn classify_call_argument_mismatch(
    expected: &str,
    actual: &str,
    program: &ResponseProgram,
) -> Option<LiveScalarShadowBlocker> {
    let expected = call_input_arguments(expected)?;
    let actual = call_input_arguments(actual)?;
    let expected = expected.as_object()?;
    let actual = actual.as_object()?;
    let differing = expected
        .iter()
        .filter(|(name, value)| actual.get(*name) != Some(*value))
        .map(|(name, value)| (name.as_str(), value))
        .collect::<Vec<_>>();
    if differing.len() != 1 || expected.len() != actual.len() {
        return None;
    }
    let (name, expected_value) = differing[0];
    let arguments = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return None,
    };
    arguments.iter().find_map(|argument| match argument {
        ResponseArgument::Role {
            name: argument_name,
            ..
        } if argument_name == name && expected_value.is_number() => {
            Some(LiveScalarShadowBlocker::TeacherProgramDynamicRoleNumericMismatch)
        }
        ResponseArgument::Role {
            name: argument_name,
            ..
        } if argument_name == name && expected_value.is_string() => {
            Some(LiveScalarShadowBlocker::TeacherProgramDynamicRoleStringMismatch)
        }
        ResponseArgument::Integer {
            name: argument_name,
            ..
        } if argument_name == name => {
            Some(LiveScalarShadowBlocker::TeacherProgramStaticIntegerMismatch)
        }
        ResponseArgument::String {
            name: argument_name,
            ..
        } if argument_name == name => {
            Some(LiveScalarShadowBlocker::TeacherProgramStaticStringMismatch)
        }
        _ => None,
    })
}

fn call_input_arguments(input: &str) -> Option<Value> {
    if let Ok(arguments) = serde_json::from_str::<Value>(input)
        && arguments.is_object()
    {
        return Some(arguments);
    }
    let tool = input.find("tools.")?;
    let arguments = input[tool..].find('(')?.saturating_add(tool + 1);
    serde_json::Deserializer::from_str(&input[arguments..])
        .into_iter::<Value>()
        .next()?
        .ok()
        .filter(Value::is_object)
}

fn classify_wire_token_value_mismatch(expected: &str, actual: &str) -> LiveScalarShadowBlocker {
    let expected = wire_tokens(expected);
    let actual = wire_tokens(actual);
    if expected.len() != actual.len() {
        return LiveScalarShadowBlocker::TeacherProgramInputTokenValueMismatch;
    }
    let mut numeric = 0_usize;
    let mut quoted = 0_usize;
    let mut identifiers = 0_usize;
    for ((expected_kind, expected_value), (actual_kind, actual_value)) in
        expected.iter().zip(&actual)
    {
        if expected_kind != actual_kind || expected_value == actual_value {
            continue;
        }
        match expected_kind {
            b'N' => numeric = numeric.saturating_add(1),
            b'Q' => quoted = quoted.saturating_add(1),
            b'A' => identifiers = identifiers.saturating_add(1),
            _ => return LiveScalarShadowBlocker::TeacherProgramInputMixedTokenMismatch,
        }
    }
    match (numeric, quoted, identifiers) {
        (1, 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputSingleNumericMismatch,
        (2.., 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputMultipleNumericMismatch,
        (0, 1.., 0) => LiveScalarShadowBlocker::TeacherProgramInputQuotedLiteralMismatch,
        (0, 0, 1..) => LiveScalarShadowBlocker::TeacherProgramInputIdentifierMismatch,
        (0, 0, 0) => LiveScalarShadowBlocker::TeacherProgramInputTokenValueMismatch,
        _ => LiveScalarShadowBlocker::TeacherProgramInputMixedTokenMismatch,
    }
}

fn wire_tokens(value: &str) -> Vec<(u8, String)> {
    let mut tokens = Vec::new();
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut index = 0_usize;
    while index < chars.len() {
        let (start, character) = chars[index];
        if character.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if character == '"' || character == '\'' {
            let quote = character;
            let mut escaped = false;
            let mut end = value.len();
            index += 1;
            while index < chars.len() {
                let (offset, next) = chars[index];
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    end = offset + next.len_utf8();
                    index += 1;
                    break;
                }
                index += 1;
            }
            tokens.push((b'Q', value[start..end].to_owned()));
            continue;
        }
        if character.is_ascii_digit() {
            index += 1;
            while index < chars.len() && chars[index].1.is_ascii_digit() {
                index += 1;
            }
            let end = chars.get(index).map_or(value.len(), |value| value.0);
            tokens.push((b'N', value[start..end].to_owned()));
            continue;
        }
        if character.is_ascii_alphabetic() || character == '_' {
            index += 1;
            while index < chars.len()
                && (chars[index].1.is_ascii_alphanumeric() || chars[index].1 == '_')
            {
                index += 1;
            }
            let end = chars.get(index).map_or(value.len(), |value| value.0);
            tokens.push((b'A', value[start..end].to_owned()));
            continue;
        }
        index += 1;
    }
    tokens
}

fn wire_token_shape(value: &str) -> Vec<u8> {
    let mut shape = Vec::with_capacity(value.len().min(4_096));
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_ascii_whitespace() {
            continue;
        }
        if character == '"' || character == '\'' {
            let quote = character;
            let mut escaped = false;
            for next in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if next == '\\' {
                    escaped = true;
                } else if next == quote {
                    break;
                }
            }
            shape.push(b'Q');
        } else if character.is_ascii_digit() {
            while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
                chars.next();
            }
            shape.push(b'N');
        } else if character.is_ascii_alphabetic() || character == '_' {
            while chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
            {
                chars.next();
            }
            shape.push(b'A');
        } else if character.is_ascii() {
            shape.push(character as u8);
        } else {
            shape.push(b'U');
        }
        if shape.len() >= 4_096 {
            break;
        }
    }
    shape
}

fn canonical_rich_actor_hypotheses(
    programs: &[ResponseProgram],
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let mut hypotheses = BTreeMap::new();
    for program in programs {
        if !rich_scalar_program_roles(program).is_some_and(|roles| roles.len() > 1) {
            continue;
        }
        let Some(canonical) =
            canonicalize_scalar_program_roles(program, request_text, provider_payload)
        else {
            continue;
        };
        let Some(roles) = rich_scalar_program_roles(&canonical) else {
            continue;
        };
        let distinct = roles
            .iter()
            .map(|(selector, _)| selector)
            .collect::<BTreeSet<_>>();
        if distinct.len() != roles.len()
            || roles.iter().any(|(selector, _)| {
                !matches!(
                    selector,
                    ResponseValueSelector::RequestReferencedJsonFieldOrdinal { .. }
                )
            })
            || execute_response(&canonical, request_text, provider_payload)
                .response
                .as_deref()
                != Some(expected_response)
        {
            continue;
        }
        let key = serde_cbor::to_vec(&canonical)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        hypotheses.entry(key).or_insert(canonical);
        if hypotheses.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
    }
    Ok(hypotheses.into_values().collect())
}

fn derive_clean_ordinal_actor(
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let observed =
        crate::runtime::observed_request_ordinal_roles(request_text, provider_payload).ok()?;
    let mut selectors_by_value = BTreeMap::<String, Vec<ResponseValueSelector>>::new();
    for role in observed {
        let value = execute_response(
            &ResponseProgram::project_selected_value(
                role.selector.clone(),
                ValueProjectionFormat::PlainText,
                "completed",
            ),
            request_text,
            provider_payload,
        )
        .response?;
        if value.is_empty() {
            return None;
        }
        selectors_by_value
            .entry(value)
            .or_default()
            .push(role.selector);
    }
    let mut spans = selectors_by_value
        .keys()
        .flat_map(|value| {
            expected_response
                .match_indices(value)
                .map(move |(start, _)| (start, start.saturating_add(value.len()), value.clone()))
        })
        .collect::<Vec<_>>();
    spans.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    let mut selected_spans = Vec::new();
    let mut cursor = 0_usize;
    for span in spans {
        if span.0 < cursor {
            continue;
        }
        cursor = span.1;
        selected_spans.push(span);
    }
    if selected_spans.is_empty()
        || selectors_by_value
            .keys()
            .any(|value| !selected_spans.iter().any(|span| &span.2 == value))
    {
        return None;
    }
    let primary_selector = selectors_by_value
        .get(&selected_spans[0].2)?
        .first()?
        .clone();
    let primary_format = ValueProjectionFormat::PlainText;
    let mut segments = Vec::new();
    let mut rendered_until = 0_usize;
    for (start, end, value) in selected_spans {
        if start > rendered_until {
            segments.push(ResponseRenderSegment::Static {
                text: expected_response[rendered_until..start].to_owned(),
            });
        }
        let selector = selectors_by_value.get(&value)?.first()?.clone();
        if selector == primary_selector {
            segments.push(ResponseRenderSegment::Primary);
        } else {
            segments.push(ResponseRenderSegment::Selected {
                selector,
                format: ValueProjectionFormat::PlainText,
            });
        }
        rendered_until = end;
    }
    if rendered_until < expected_response.len() {
        segments.push(ResponseRenderSegment::Static {
            text: expected_response[rendered_until..].to_owned(),
        });
    }
    let actor =
        ResponseProgram::project_selected_value(primary_selector, primary_format, "completed")
            .with_value_renderer(CollectionOutputRenderer::RenderSequence { segments });
    (execute_response(&actor, request_text, provider_payload)
        .response
        .as_deref()
        == Some(expected_response))
    .then_some(actor)
}

fn expand_actor_hypothesis_set(
    seeds: Vec<ResponseProgram>,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let mut hypotheses = BTreeMap::new();
    for seed in seeds {
        let seed_key = serde_cbor::to_vec(&seed)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        hypotheses.entry(seed_key).or_insert(seed.clone());
        if repeated_primary_slots(&seed) == 0 {
            continue;
        }
        for hypothesis in bounded_ordinal_role_permutations(
            &seed,
            request_text,
            provider_payload,
            expected_response,
        )? {
            let key = serde_cbor::to_vec(&hypothesis)
                .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
            hypotheses.entry(key).or_insert(hypothesis);
            if hypotheses.len() > TEACHER_CALL_SELECTOR_BUDGET {
                return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
            }
        }
    }
    Ok(hypotheses.into_values().collect())
}

pub(super) fn bounded_ordinal_role_permutations(
    seed: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Result<Vec<ResponseProgram>, LiveScalarShadowBlocker> {
    let role_types = scalar_program_role_slot_types(seed)
        .ok_or(LiveScalarShadowBlocker::RoleTypeInferenceFailed)?;
    let observed = crate::runtime::observed_request_ordinal_roles(request_text, provider_payload)
        .map_err(|_| LiveScalarShadowBlocker::ObservedRoleExtractionFailed)?;
    let candidates = observed
        .into_iter()
        .map(|role| role.selector)
        .collect::<Vec<_>>();
    let mut assignments = Vec::new();
    enumerate_ordinal_assignments(
        &role_types,
        &candidates,
        0,
        &mut vec![false; candidates.len()],
        &mut Vec::new(),
        &mut assignments,
    )?;
    let mut programs = BTreeMap::new();
    for assignment in assignments {
        let Some(program) = replace_scalar_program_selectors(seed, &assignment) else {
            continue;
        };
        if execute_response(&program, request_text, provider_payload)
            .response
            .as_deref()
            != Some(expected_response)
        {
            continue;
        }
        let key = serde_cbor::to_vec(&program)
            .map_err(|_| LiveScalarShadowBlocker::HypothesisEncodingFailed)?;
        programs.entry(key).or_insert(program);
    }
    Ok(programs.into_values().collect())
}

fn enumerate_ordinal_assignments(
    role_types: &[AtomValueType],
    candidates: &[ResponseValueSelector],
    slot: usize,
    used: &mut [bool],
    current: &mut Vec<ResponseValueSelector>,
    output: &mut Vec<Vec<ResponseValueSelector>>,
) -> Result<(), LiveScalarShadowBlocker> {
    if slot == role_types.len() {
        output.push(current.clone());
        if output.len() > TEACHER_CALL_SELECTOR_BUDGET {
            return Err(LiveScalarShadowBlocker::HypothesisBudgetExhausted);
        }
        return Ok(());
    }
    for (index, candidate) in candidates.iter().enumerate() {
        if used[index] || selector_value_type(candidate) != role_types[slot] {
            continue;
        }
        used[index] = true;
        current.push(candidate.clone());
        enumerate_ordinal_assignments(role_types, candidates, slot + 1, used, current, output)?;
        current.pop();
        used[index] = false;
    }
    Ok(())
}

fn replace_scalar_program_selectors(
    seed: &ResponseProgram,
    selectors: &[ResponseValueSelector],
) -> Option<ResponseProgram> {
    let mut program = seed.clone();
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        ..
    } = &mut program.operation
    else {
        return None;
    };
    let mut replacements = selectors.iter();
    *selector = replacements.next()?.clone();
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        let mut primary_seen = false;
        for segment in segments {
            match segment {
                ResponseRenderSegment::Primary if primary_seen => {
                    *segment = ResponseRenderSegment::Selected {
                        selector: replacements.next()?.clone(),
                        format: *format,
                    };
                }
                ResponseRenderSegment::Primary => primary_seen = true,
                ResponseRenderSegment::Selected { selector, .. } => {
                    *selector = replacements.next()?.clone();
                }
                ResponseRenderSegment::Static { .. } => {}
            }
        }
    }
    replacements.next().is_none().then_some(program)
}

pub(super) fn scalar_program_role_slot_types(
    program: &ResponseProgram,
) -> Option<Vec<AtomValueType>> {
    let ResponseOperation::ProjectSelectedValue {
        selector, renderer, ..
    } = &program.operation
    else {
        return None;
    };
    let primary_type = selector_value_type(selector);
    let mut role_types = vec![primary_type];
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        let mut primary_seen = false;
        for segment in segments {
            match segment {
                ResponseRenderSegment::Primary if primary_seen => role_types.push(primary_type),
                ResponseRenderSegment::Primary => primary_seen = true,
                ResponseRenderSegment::Selected { selector, .. } => {
                    role_types.push(selector_value_type(selector));
                }
                ResponseRenderSegment::Static { .. } => {}
            }
        }
    }
    (role_types.len() <= 16).then_some(role_types)
}

fn repeated_primary_slots(program: &ResponseProgram) -> usize {
    let ResponseOperation::ProjectSelectedValue { renderer, .. } = &program.operation else {
        return 0;
    };
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return 0;
    };
    segments
        .iter()
        .filter(|segment| matches!(segment, ResponseRenderSegment::Primary))
        .count()
        .saturating_sub(1)
}

fn structural_scalar_law_shape(
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<Vec<u8>> {
    let observed =
        crate::runtime::observed_request_ordinal_roles(request_text, provider_payload).ok()?;
    let mut values = observed
        .iter()
        .filter_map(|role| {
            execute_response(
                &ResponseProgram::project_selected_value(
                    role.selector.clone(),
                    ValueProjectionFormat::PlainText,
                    "completed",
                ),
                request_text,
                provider_payload,
            )
            .response
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() != observed.len() {
        return None;
    }
    values.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    values.dedup();
    let mut dynamic = vec![false; expected_response.len()];
    for value in values {
        for (offset, _) in expected_response.match_indices(&value) {
            let end = offset.saturating_add(value.len());
            if end <= dynamic.len() && !dynamic[offset..end].iter().any(|marked| *marked) {
                dynamic[offset..end].fill(true);
            }
        }
    }
    if !dynamic.iter().any(|marked| *marked) {
        return None;
    }
    let mut shape = vec![4, u8::try_from(observed.len()).ok()?];
    shape.extend(observed.iter().map(|role| value_type_tag(role.value_type)));
    let mut previous = None;
    for marked in dynamic {
        if previous != Some(marked) {
            shape.push(u8::from(marked));
            previous = Some(marked);
        }
    }
    Some(shape)
}

pub(super) fn source_neutral_scalar_program_shape(program: &ResponseProgram) -> Option<Vec<u8>> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            arguments,
            selector,
            ..
        } => {
            let arguments = arguments
                .iter()
                .filter(|argument| semantic_call_argument(argument))
                .collect::<Vec<_>>();
            let arguments = serde_cbor::to_vec(&arguments).ok()?;
            let mut shape = vec![10, value_type_tag(selector_value_type(selector))];
            shape.extend_from_slice(&digest_parts(
                b"nando.live-function-call-law.v1",
                &[function_name.as_bytes(), &arguments],
            ));
            return Some(shape);
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            arguments,
            projection,
            selector,
            ..
        } => {
            let arguments = arguments
                .iter()
                .filter(|argument| semantic_call_argument(argument))
                .collect::<Vec<_>>();
            let arguments = serde_cbor::to_vec(&arguments).ok()?;
            let projection = serde_cbor::to_vec(projection).ok()?;
            let mut shape = vec![11, value_type_tag(selector_value_type(selector))];
            shape.extend_from_slice(&digest_parts(
                b"nando.live-custom-tool-law.v1",
                &[
                    custom_tool_name.as_bytes(),
                    inner_tool_name.as_bytes(),
                    &arguments,
                    &projection,
                ],
            ));
            return Some(shape);
        }
        _ => {}
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        renderer,
        completion_state,
        ..
    } = &program.operation
        && let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { value_type, .. },
            tail @ ..,
        ] = steps.as_slice()
    {
        if completion_state != "completed" || !matches!(tail, [] | [CollectionProgramStep::Count]) {
            return None;
        }
        let mut shape = vec![
            if tail.is_empty() { 8 } else { 9 },
            collection_scalar_tag(*value_type),
        ];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    if let ResponseOperation::ProjectStatus {
        mapping,
        renderer,
        completion_state,
        ..
    } = &program.operation
    {
        if completion_state != "completed" {
            return None;
        }
        let mut shape = vec![7, status_mapping_flags(*mapping)? as u8];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        format,
        renderer,
        completion_state,
        ..
    } = &program.operation
    {
        if completion_state != "completed"
            || steps.as_slice()
                != [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ]
        {
            return None;
        }
        let mut shape = vec![6, u8::from(*format == ValueProjectionFormat::CanonicalJson)];
        shape.push(match renderer {
            CollectionOutputRenderer::Direct => 0,
            CollectionOutputRenderer::RenderTemplate { .. } => 1,
            _ => return None,
        });
        return Some(shape);
    }
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut shape = vec![
        5,
        value_type_tag(selector_value_type(selector)),
        u8::from(*format == ValueProjectionFormat::CanonicalJson),
    ];
    match renderer {
        CollectionOutputRenderer::Direct => shape.push(0),
        CollectionOutputRenderer::RenderTemplate { .. } => shape.push(1),
        CollectionOutputRenderer::RenderSequence { segments } => {
            shape.push(2);
            shape.extend_from_slice(&(segments.len() as u16).to_le_bytes());
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { .. } => shape.push(0),
                    ResponseRenderSegment::Primary => shape.push(1),
                    ResponseRenderSegment::Selected { selector, format } => {
                        shape.extend_from_slice(&[
                            2,
                            value_type_tag(selector_value_type(selector)),
                            u8::from(*format == ValueProjectionFormat::CanonicalJson),
                        ]);
                    }
                }
            }
        }
        CollectionOutputRenderer::RequestTemplate { marker } => {
            shape.extend_from_slice(&[3, *marker as u8]);
        }
    }
    Some(shape)
}

#[derive(Serialize)]
enum SemanticRenderAtomV1<'a> {
    Static(&'a str),
    Role {
        selector: Vec<u8>,
        format: ValueProjectionFormat,
    },
}

pub(super) fn source_neutral_multi_role_program_shape(
    program: &ResponseProgram,
) -> Option<Vec<u8>> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return source_neutral_scalar_program_shape(program);
    };
    if completion_state != "completed" {
        return None;
    }
    let role = |selector: &ResponseValueSelector, format| {
        Some(SemanticRenderAtomV1::Role {
            selector: semantic_selector_shape(selector)?,
            format,
        })
    };
    let atoms = match renderer {
        CollectionOutputRenderer::Direct => vec![role(selector, *format)?],
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => vec![
            SemanticRenderAtomV1::Static(prefix),
            role(selector, *format)?,
            SemanticRenderAtomV1::Static(suffix),
        ],
        CollectionOutputRenderer::RenderSequence { segments } => segments
            .iter()
            .map(|segment| match segment {
                ResponseRenderSegment::Static { text } => Some(SemanticRenderAtomV1::Static(text)),
                ResponseRenderSegment::Primary => role(selector, *format),
                ResponseRenderSegment::Selected { selector, format } => role(selector, *format),
            })
            .collect::<Option<Vec<_>>>()?,
        CollectionOutputRenderer::RequestTemplate { marker } => {
            return serde_cbor::to_vec(&(
                "nando.live-multi-role-behavior.v1",
                completion_state,
                "request_template",
                marker,
                semantic_selector_shape(selector)?,
                format,
            ))
            .ok();
        }
    };
    serde_cbor::to_vec(&("nando.live-multi-role-behavior.v1", completion_state, atoms)).ok()
}

fn semantic_selector_shape(selector: &ResponseValueSelector) -> Option<Vec<u8>> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => serde_cbor::to_vec(&("request_json_field_ordinal", ordinal, value_type)).ok(),
        ResponseValueSelector::RequestLastToken => {
            serde_cbor::to_vec(&("request_last_token", AtomValueType::String)).ok()
        }
        ResponseValueSelector::RequestUniqueLiteral => {
            serde_cbor::to_vec(&("request_unique_literal", AtomValueType::String)).ok()
        }
        _ => serde_cbor::to_vec(selector).ok(),
    }
}

fn semantic_call_argument(argument: &ResponseArgument) -> bool {
    match argument {
        ResponseArgument::Integer { name, .. } => {
            !crate::teacher_join::is_execution_budget_argument(name)
        }
        ResponseArgument::String { name, value } => !(name == "chars" && value.is_empty()),
        _ => true,
    }
}

fn synthesis_payload_with_request(
    request_text: &str,
    provider_payload: &Value,
) -> Result<Value, LiveScalarShadowBlocker> {
    crate::runtime::provider_payload_view(request_text, provider_payload)
        .map(std::borrow::Cow::into_owned)
        .map_err(|error| match error {
            "provider_view_request_budget" => LiveScalarShadowBlocker::RequestTextInvalid,
            "provider_view_input_missing" => LiveScalarShadowBlocker::ProviderInputMissing,
            "provider_view_payload_budget" => LiveScalarShadowBlocker::PayloadTooLarge,
            _ => LiveScalarShadowBlocker::PayloadSerializationFailed,
        })
}

pub(super) fn rich_scalar_program_roles(
    program: &ResponseProgram,
) -> Option<Vec<(ResponseValueSelector, ValueProjectionFormat)>> {
    if let ResponseOperation::FunctionCallFromRoles { selector, .. }
    | ResponseOperation::CustomToolCallFromRoles { selector, .. } = &program.operation
    {
        return Some(vec![(selector.clone(), ValueProjectionFormat::PlainText)]);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        completion_state,
        ..
    } = &program.operation
        && let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector,
                value_type: _,
            },
            tail @ ..,
        ] = steps.as_slice()
    {
        return (completion_state == "completed"
            && matches!(tail, [] | [CollectionProgramStep::Count]))
        .then(|| {
            vec![
                (
                    ResponseValueSelector::UniqueScalar {
                        value_type: AtomValueType::Collection,
                    },
                    ValueProjectionFormat::CanonicalJson,
                ),
                (selector.clone(), ValueProjectionFormat::CanonicalJson),
            ]
        });
    }
    if let ResponseOperation::ProjectStatus {
        selector,
        completion_state,
        ..
    } = &program.operation
    {
        return (completion_state == "completed")
            .then(|| vec![(selector.clone(), ValueProjectionFormat::PlainText)]);
    }
    if let ResponseOperation::ComposeCollection {
        steps,
        format,
        completion_state,
        ..
    } = &program.operation
    {
        return (completion_state == "completed"
            && steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ])
        .then(|| {
            vec![(
                ResponseValueSelector::UniqueScalar {
                    value_type: AtomValueType::Collection,
                },
                *format,
            )]
        });
    }
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let mut roles = Vec::new();
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            let role = match segment {
                ResponseRenderSegment::Primary => Some((selector.clone(), *format)),
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
    } else {
        roles.push((selector.clone(), *format));
    }
    (roles.len() <= 16).then_some(roles)
}

pub(super) fn canonicalize_scalar_program_roles(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> Option<ResponseProgram> {
    if let ResponseOperation::FunctionCallFromRoles { .. }
    | ResponseOperation::CustomToolCallFromRoles { .. } = &program.operation
    {
        let mut canonical = program.clone();
        let selector = match &mut canonical.operation {
            ResponseOperation::FunctionCallFromRoles { selector, .. }
            | ResponseOperation::CustomToolCallFromRoles { selector, .. } => selector,
            _ => unreachable!(),
        };
        *selector = match selector {
            ResponseValueSelector::JsonField { .. }
            | ResponseValueSelector::UniqueTurnJsonField { .. }
            | ResponseValueSelector::UniqueActiveTurnJsonField { .. } => {
                ResponseValueSelector::UniqueScalar {
                    value_type: selector_value_type(selector),
                }
            }
            _ => selector.clone(),
        };
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE) {
        let mut canonical = program.clone();
        let ResponseOperation::ComposeCollection {
            steps, renderer, ..
        } = &mut canonical.operation
        else {
            return None;
        };
        let [
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { selector, .. },
            tail @ ..,
        ] = steps.as_mut_slice()
        else {
            return None;
        };
        if !matches!(tail, [] | [CollectionProgramStep::Count]) {
            return None;
        }
        *selector = canonical_role_selector(selector, request_text, provider_payload)?;
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_PROJECT_STATUS) {
        let mut canonical = program.clone();
        let ResponseOperation::ProjectStatus {
            selector, renderer, ..
        } = &mut canonical.operation
        else {
            return None;
        };
        *selector = canonical_role_selector(selector, request_text, provider_payload)?;
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    if program_transform_opcode(program) == Some(TRANSFORM_OPCODE_COUNT_COLLECTION) {
        let mut canonical = program.clone();
        let ResponseOperation::ComposeCollection { renderer, .. } = &mut canonical.operation else {
            return None;
        };
        *renderer = normalized_scalar_renderer(renderer)?;
        return Some(canonical);
    }
    let mut canonical = program.clone();
    let ResponseOperation::ProjectSelectedValue {
        selector,
        renderer,
        completion_state,
        ..
    } = &mut canonical.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    *selector = canonical_role_selector(selector, request_text, provider_payload)?;
    if let CollectionOutputRenderer::RenderSequence { segments } = renderer {
        for segment in segments {
            if let ResponseRenderSegment::Selected { selector, .. } = segment {
                *selector = canonical_role_selector(selector, request_text, provider_payload)?;
            }
        }
    }
    Some(canonical)
}

fn canonical_role_selector(
    selector: &ResponseValueSelector,
    request_text: &str,
    provider_payload: &Value,
) -> Option<ResponseValueSelector> {
    match selector {
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => Some(ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal: *ordinal,
            value_type: *value_type,
        }),
        // Preserve the structural origin of request-bound roles. Collapsing
        // these to UniqueScalar erases the circuit edge that distinguishes a
        // predicate supplied by the request from unrelated payload strings.
        ResponseValueSelector::RequestLastToken | ResponseValueSelector::RequestUniqueLiteral => {
            Some(selector.clone())
        }
        // Content-line selectors are learned structural anchors into tool
        // output, not surface field names. Collapsing one to UniqueScalar
        // loses the only path from a continuation marker to its bound role.
        ResponseValueSelector::ContentLinePrefix { .. } => Some(selector.clone()),
        ResponseValueSelector::JsonField { .. } => {
            crate::runtime::canonical_request_ordinal_selector(
                request_text,
                provider_payload,
                selector,
            )
            .ok()
            .flatten()
            .or(Some(ResponseValueSelector::UniqueScalar {
                value_type: selector_value_type(selector),
            }))
        }
        _ => Some(ResponseValueSelector::UniqueScalar {
            value_type: selector_value_type(selector),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observed_rich_scalar_surface(
    request_text: &str,
    provider_payload: &Value,
    program_roles: &[(ResponseValueSelector, ValueProjectionFormat)],
    transform_opcode: u8,
    transform_flags: u16,
    compose_count: bool,
    frame_id: &str,
    lineage_sha256: [u8; 32],
    surface_sha256: [u8; 32],
) -> Result<crate::RuntimeSurfaceEvidence, LiveScalarShadowBlocker> {
    let is_filter = transform_opcode == TRANSFORM_OPCODE_FILTER_REQUEST_VALUE;
    let role_count = if is_filter {
        program_roles
            .len()
            .saturating_add(if compose_count { 3 } else { 2 })
    } else {
        // Context + one source and one virtual result slot per transform.
        // The renderer composes those result slots into the final response.
        1_usize.saturating_add(program_roles.len().saturating_mul(2))
    };
    if !(3..=18).contains(&role_count) {
        return Err(LiveScalarShadowBlocker::InvalidBundle);
    }
    let semantic_to_local = local_role_permutation_n(frame_id, role_count);
    let context = semantic_to_local[0];
    let output = is_filter.then_some(semantic_to_local[role_count - 1]);
    let intermediate = (is_filter && compose_count).then_some(semantic_to_local[role_count - 2]);
    let mut roles = vec![StructuralRoleSignature::new(0, 0, 0, 0, Vec::new()); role_count];
    let planes = (0..program_roles.len())
        .map(|index| u8::try_from(index).unwrap_or(u8::MAX))
        .collect::<Vec<_>>();
    roles[usize::from(context)] = StructuralRoleSignature::new(5, 1, 0, 1, planes.clone());
    if let Some(intermediate) = intermediate {
        roles[usize::from(intermediate)] = StructuralRoleSignature::new(
            value_type_tag(AtomValueType::Collection),
            1,
            2,
            4,
            Vec::new(),
        );
    }
    if let Some(output) = output {
        roles[usize::from(output)] = StructuralRoleSignature::new(
            value_type_tag(if compose_count {
                AtomValueType::Integer
            } else {
                selector_value_type(&program_roles[0].0)
            }),
            1,
            2,
            4,
            Vec::new(),
        );
    }
    let mut relations = Vec::with_capacity(program_roles.len());
    let mut atoms = Vec::with_capacity(program_roles.len());
    let mut anchors = Vec::with_capacity(program_roles.len());
    for (index, (selector, format)) in program_roles.iter().enumerate() {
        let source = semantic_to_local[index + 1];
        let plane = u8::try_from(index).map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
        let value_type = selector_value_type(selector);
        let phase = if program_roles.len() == 1 {
            roles[usize::from(source)] =
                crate::crystallized_operator::runtime_role_signature_for_selector(selector, plane);
            let phase_atoms = [
                format!("scalar_type:{}", value_type_tag(value_type)),
                "cardinality:unique".to_owned(),
            ];
            phase_vector_from_atoms(phase_atoms.iter().map(String::as_str), 1)[0]
        } else {
            roles[usize::from(source)] =
                crate::crystallized_operator::runtime_multi_role_signature_for_selector(
                    selector, plane,
                );
            let phase_atom = format!("scalar_role:{index}:type:{}", value_type_tag(value_type));
            phase_vector_from_atoms([phase_atom.as_str()], 1)[0]
        };
        relations.push(LocalRelationFragment {
            plane,
            source_local_role: context,
            target_local_role: source,
            state: TernaryRelationState::Supported,
            phase_anchor: phase,
        });
        if !is_filter {
            let transform_output = semantic_to_local[1 + program_roles.len() + index];
            roles[usize::from(transform_output)] =
                StructuralRoleSignature::new(value_type_tag(value_type), 1, 2, 4, Vec::new());
            atoms.push(TypedProgramAtom {
                opcode: transform_opcode,
                output_local_role: transform_output,
                source_a_local_role: source,
                source_b_local_role: OPERATOR_ROLE_NONE,
                parameter: transform_parameter(value_type)
                    | (u16::try_from(index).unwrap_or(u16::MAX) << 8),
                flags: match transform_opcode {
                    TRANSFORM_OPCODE_PROJECT_STATUS => transform_flags,
                    TRANSFORM_OPCODE_COUNT_COLLECTION => 0,
                    _ if *format == ValueProjectionFormat::CanonicalJson => {
                        TRANSFORM_FLAG_CANONICAL_JSON
                    }
                    _ => 0,
                },
            });
        }
        anchors.push(RuntimeRoleAnchor {
            local_role: source,
            selector: selector.clone(),
            json_path_sha256: None,
        });
    }
    if is_filter {
        let predicate_type = selector_value_type(&program_roles[1].0);
        let output = output.ok_or(LiveScalarShadowBlocker::InvalidBundle)?;
        let filter_output = intermediate.unwrap_or(output);
        atoms.push(TypedProgramAtom {
            opcode: TRANSFORM_OPCODE_FILTER_REQUEST_VALUE,
            output_local_role: filter_output,
            source_a_local_role: semantic_to_local[1],
            source_b_local_role: semantic_to_local[2],
            parameter: transform_parameter(predicate_type),
            flags: transform_flags,
        });
        if compose_count {
            atoms.push(TypedProgramAtom {
                opcode: TRANSFORM_OPCODE_COUNT_COLLECTION,
                output_local_role: output,
                source_a_local_role: filter_output,
                source_b_local_role: OPERATOR_ROLE_NONE,
                parameter: (1 << 8) | transform_parameter(AtomValueType::Collection),
                flags: 0,
            });
        }
    }
    let bundle =
        SurfaceFragmentBundle::new(lineage_sha256, surface_sha256, roles, relations, atoms)
            .map_err(|_| LiveScalarShadowBlocker::InvalidBundle)?;
    Ok(crate::RuntimeSurfaceEvidence {
        bundle,
        request_text: request_text.to_owned(),
        provider_payload: provider_payload.clone(),
        anchors: anchors.into_boxed_slice(),
    })
}

fn project_scalar_program(
    program: ResponseProgram,
) -> Option<(
    Vec<u8>,
    ResponseValueSelector,
    AtomValueType,
    ValueProjectionFormat,
    CollectionOutputRenderer,
)> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
    } = &program.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let renderer = normalized_scalar_renderer(renderer)?;
    let value_type = selector_value_type(selector);
    let bytes = serde_json::to_vec(&program).ok()?;
    Some((bytes, selector.clone(), value_type, *format, renderer))
}

fn derive_exact_scalar_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ProjectSelectedValue {
        selector,
        format,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let direct = ResponseProgram::project_selected_value(
        selector.clone(),
        *format,
        completion_state.clone(),
    );
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_value_renderer(renderer);
    if derived.validate().is_err()
        || !is_privacy_safe_online_response_program(&derived)
        || execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            != Some(expected_response)
    {
        return None;
    }
    Some(derived)
}

fn derive_exact_count_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ComposeCollection {
        steps,
        format: _,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed"
        || steps.as_slice()
            != [
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ]
    {
        return None;
    }
    // Count emits a decimal integer, for which PlainText and CanonicalJson are
    // byte-identical. Freeze one representation so equivalent hypotheses do
    // not split the operator field or VM contract.
    let direct = ResponseProgram::compose_collection(
        steps.clone(),
        ValueProjectionFormat::PlainText,
        completion_state.clone(),
    );
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_collection_renderer(renderer);
    (derived.validate().is_ok()
        && is_privacy_safe_online_response_program(&derived)
        && execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            == Some(expected_response))
    .then_some(derived)
}

fn derive_exact_filter_programs(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Vec<ResponseProgram> {
    let ResponseOperation::ComposeCollection {
        steps,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return Vec::new();
    };
    let [
        CollectionProgramStep::SelectOnlyArrayField,
        CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type },
        tail @ ..,
    ] = steps.as_slice()
    else {
        return Vec::new();
    };
    if completion_state != "completed" || !matches!(tail, [] | [CollectionProgramStep::Count]) {
        return Vec::new();
    }
    let expected_type = collection_atom_type(*value_type);
    crate::collection_synthesis::learned_selector_candidates(provider_payload)
        .into_iter()
        .filter(|selector| selector_value_type(selector) == expected_type)
        // The broad hypothesis is explicitly request-conditioned. Letting an
        // equal payload scalar replace that role creates a second, spurious
        // circuit which cannot be separated while predicate and row agree.
        .filter(crate::collection_synthesis::is_source_neutral_request_selector)
        .filter_map(|selector| {
            let mut steps = vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                    selector,
                    value_type: *value_type,
                },
            ];
            if !tail.is_empty() {
                steps.push(CollectionProgramStep::Count);
            }
            let direct = ResponseProgram::compose_collection(
                steps,
                ValueProjectionFormat::CanonicalJson,
                completion_state.clone(),
            );
            if !is_source_neutral_response_program(&direct) {
                return None;
            }
            let computed = execute_response(&direct, request_text, provider_payload).response?;
            let renderer = infer_scalar_renderer(&computed, expected_response)?;
            let derived = direct.with_collection_renderer(renderer);
            (derived.validate().is_ok()
                && is_privacy_safe_online_response_program(&derived)
                && execute_response(&derived, request_text, provider_payload)
                    .response
                    .as_deref()
                    == Some(expected_response))
            .then_some(derived)
        })
        .collect()
}

fn derive_exact_status_program(
    candidate: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    expected_response: &str,
) -> Option<ResponseProgram> {
    let ResponseOperation::ProjectStatus {
        selector,
        mapping,
        completion_state,
        ..
    } = &candidate.operation
    else {
        return None;
    };
    if completion_state != "completed" {
        return None;
    }
    let direct =
        ResponseProgram::project_status(selector.clone(), *mapping, completion_state.clone());
    if !is_source_neutral_response_program(&direct) {
        return None;
    }
    let computed = execute_response(&direct, request_text, provider_payload).response?;
    let renderer = infer_scalar_renderer(&computed, expected_response)?;
    let derived = direct.with_status_renderer(renderer);
    (derived.validate().is_ok()
        && is_privacy_safe_online_response_program(&derived)
        && execute_response(&derived, request_text, provider_payload)
            .response
            .as_deref()
            == Some(expected_response))
    .then_some(derived)
}

fn infer_scalar_renderer(computed: &str, expected: &str) -> Option<CollectionOutputRenderer> {
    if computed == expected {
        return Some(CollectionOutputRenderer::Direct);
    }
    if computed.is_empty() {
        return None;
    }
    let mut matches = expected.match_indices(computed);
    let (offset, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(CollectionOutputRenderer::RenderTemplate {
        prefix: expected[..offset].to_owned(),
        suffix: expected[offset + computed.len()..].to_owned(),
    })
}

pub(super) fn normalized_scalar_renderer(
    renderer: &CollectionOutputRenderer,
) -> Option<CollectionOutputRenderer> {
    match renderer {
        CollectionOutputRenderer::Direct | CollectionOutputRenderer::RenderTemplate { .. } => {
            Some(renderer.clone())
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            let mut prefix = String::new();
            let mut suffix = String::new();
            let mut primary_seen = false;
            for segment in segments {
                match segment {
                    ResponseRenderSegment::Static { text } if primary_seen => {
                        suffix.push_str(text);
                    }
                    ResponseRenderSegment::Static { text } => prefix.push_str(text),
                    ResponseRenderSegment::Primary if !primary_seen => primary_seen = true,
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => {
                        return None;
                    }
                }
            }
            primary_seen.then_some(CollectionOutputRenderer::RenderTemplate { prefix, suffix })
        }
        CollectionOutputRenderer::RequestTemplate { .. } => None,
    }
}

fn classify_exact_program_blocker(programs: &[ResponseProgram]) -> LiveScalarShadowBlocker {
    if programs
        .iter()
        .any(|program| matches!(&program.operation, ResponseOperation::ProjectStatus { .. }))
    {
        LiveScalarShadowBlocker::ExactStatusProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ComposeCollection { .. }
        )
    }) {
        LiveScalarShadowBlocker::ExactCollectionProgram
    } else if programs.iter().any(|program| {
        matches!(
            &program.operation,
            ResponseOperation::ProjectSelectedValue { .. }
        )
    }) {
        LiveScalarShadowBlocker::UnsupportedRendererProgram
    } else {
        LiveScalarShadowBlocker::UnsupportedProgramKind
    }
}

pub(super) fn selector_value_type(selector: &ResponseValueSelector) -> AtomValueType {
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

fn local_role_permutation_n(frame_id: &str, role_count: usize) -> Vec<u8> {
    let digest = Sha256::digest(frame_id.as_bytes());
    let mut roles = (0..role_count)
        .map(|role| u8::try_from(role).expect("bounded live role count"))
        .collect::<Vec<_>>();
    for index in (1..roles.len()).rev() {
        roles.swap(
            index,
            usize::from(digest[index % digest.len()]) % (index + 1),
        );
    }
    roles
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

pub(super) fn program_transform_opcode(program: &ResponseProgram) -> Option<u8> {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue { .. }
        | ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => {
            Some(TRANSFORM_OPCODE_PROJECT_UNIQUE_SCALAR)
        }
        ResponseOperation::ProjectStatus { .. } => Some(TRANSFORM_OPCODE_PROJECT_STATUS),
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    ..
                ]
            ) && matches!(steps.len(), 2 | 3) =>
        {
            Some(TRANSFORM_OPCODE_FILTER_REQUEST_VALUE)
        }
        ResponseOperation::ComposeCollection { steps, .. }
            if steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ] =>
        {
            Some(TRANSFORM_OPCODE_COUNT_COLLECTION)
        }
        _ => None,
    }
}

pub(super) fn program_transform_flags(program: &ResponseProgram) -> Option<u16> {
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => Some(0),
        ResponseOperation::ProjectSelectedValue { format, .. } => {
            Some(u16::from(*format == ValueProjectionFormat::CanonicalJson))
        }
        ResponseOperation::ProjectStatus { mapping, .. } => status_mapping_flags(*mapping),
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    ..
                ]
            ) && matches!(steps.len(), 2 | 3) =>
        {
            Some(TRANSFORM_FLAG_CANONICAL_JSON)
        }
        ResponseOperation::ComposeCollection { steps, .. }
            if steps.as_slice()
                == [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::Count,
                ] =>
        {
            Some(0)
        }
        _ => None,
    }
}

pub(super) fn program_has_filter_count(program: &ResponseProgram) -> bool {
    matches!(
        &program.operation,
        ResponseOperation::ComposeCollection { steps, .. }
            if matches!(
                steps.as_slice(),
                [
                    CollectionProgramStep::SelectOnlyArrayField,
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue { .. },
                    CollectionProgramStep::Count,
                ]
            )
    )
}

const fn collection_scalar_tag(value_type: CollectionScalarType) -> u8 {
    match value_type {
        CollectionScalarType::String => 1,
        CollectionScalarType::Integer => 2,
        CollectionScalarType::Boolean => 3,
    }
}

const fn collection_atom_type(value_type: CollectionScalarType) -> AtomValueType {
    match value_type {
        CollectionScalarType::String => AtomValueType::String,
        CollectionScalarType::Integer => AtomValueType::Integer,
        CollectionScalarType::Boolean => AtomValueType::Boolean,
    }
}

const fn status_mapping_flags(mapping: ProjectStatusMapping) -> Option<u16> {
    Some(match mapping {
        ProjectStatusMapping::ZeroIsSuccess => TRANSFORM_STATUS_ZERO_IS_SUCCESS,
        ProjectStatusMapping::ZeroIsPass => TRANSFORM_STATUS_ZERO_IS_PASS,
        ProjectStatusMapping::ZeroIsOk => TRANSFORM_STATUS_ZERO_IS_OK,
        ProjectStatusMapping::ZeroIsTrue => TRANSFORM_STATUS_ZERO_IS_TRUE,
    })
}

const fn value_type_tag(value_type: AtomValueType) -> u8 {
    match value_type {
        AtomValueType::String => 1,
        AtomValueType::Integer => 2,
        AtomValueType::Boolean => 3,
        AtomValueType::Identifier => 4,
        AtomValueType::Collection => 5,
    }
}

pub(super) fn parse_commitment(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, byte) in digest.iter_mut().enumerate() {
        let offset = index * 2;
        *byte = u8::from_str_radix(&value[offset..offset + 2], 16).ok()?;
    }
    (digest != [0; 32]).then_some(digest)
}

pub(super) fn commitment_hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn extractor_version(value: &str) -> u32 {
    let digest = Sha256::digest(value.as_bytes());
    u32::from_le_bytes(digest[..4].try_into().expect("fixed digest width"))
}
