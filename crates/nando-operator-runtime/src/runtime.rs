use std::collections::{BTreeMap, BTreeSet};

use nando_core::wave::{phase_margin_to_micro, phase_vector_from_atom_ids};
use serde_json::Value;

use nando_operator_kernel::{
    AtomValueType, CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, MAX_PROJECT_STATUS_CODE,
    ProjectStatusMapping, RequestTemplateMarker, ResponseAdapterWaveRoute, ResponseArgument,
    ResponseConsensusVariant, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, SemanticRole, ValueProjectionFormat,
};

mod selection;
mod structural_json_roles;

#[cfg(test)]
use selection::latest_turn_output_scalar_from_end;
pub use selection::{
    ExtractedScalar, ObservedRoleCandidate, ObservedSourceClass,
    canonical_request_ordinal_selector, immediate_selected_scalar, immediate_tool_output_value,
    immediate_unique_scalar, observed_request_ordinal_roles, output_text_parts, parse_scalar_text,
    structural_output_selectors_for_field_hint, structural_output_selectors_for_teacher_value,
};
use selection::{
    active_turn_output_value, identifier_tokens, immediate_selected_scalar_with_request,
    request_mentions_identifier, runtime_embedded_json_objects,
};
pub use structural_json_roles::{
    ObservedJsonScalarRole, ObservedScalarRoleClass, observed_continuation_handle_role,
    observed_json_scalar_roles,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseExecutionStatus {
    Executed,
    Abstain,
    VerifyFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseExecution {
    pub status: ResponseExecutionStatus,
    pub reason: String,
    pub response: Option<String>,
    pub verification_receipt_id: Option<String>,
}

const RUNTIME_OUTPUT_SCALAR_BUDGET: usize = 2_048;

#[doc(hidden)]
pub type ExternalResponseValidator =
    dyn Fn(&ResponseProgram, &str, &Value, &str) -> Result<(), String>;

impl ResponseExecution {
    #[doc(hidden)]
    pub fn rejected(status: ResponseExecutionStatus, reason: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason.into(),
            response: None,
            verification_receipt_id: None,
        }
    }
}

pub fn execute_response_unverified(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
) -> ResponseExecution {
    execute_response_with_external_validator(
        program,
        request_text,
        provider_payload,
        &|_, _, _, _| Ok(()),
    )
}

#[doc(hidden)]
pub fn execute_response_with_external_validator(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    validator: &ExternalResponseValidator,
) -> ResponseExecution {
    if let Err(reason) = program.validate() {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::Abstain,
            format!("invalid_program:{reason}"),
        );
    }
    let response = match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave,
        } => execute_unique_consensus(
            variants,
            adapter_wave.as_ref(),
            request_text,
            provider_payload,
            validator,
        ),
        ResponseOperation::AdvancePlan { function_name } => {
            execute_advance_plan(provider_payload, function_name)
        }
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => execute_function_call_from_roles(provider_payload, function_name, selector, arguments),
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => execute_custom_tool_call_from_roles(
            provider_payload,
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        ),
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            ..
        } => project_selected_value_with_request(request_text, provider_payload, selector, *format)
            .and_then(|computed| {
                apply_value_renderer_with_request(
                    request_text,
                    provider_payload,
                    computed,
                    renderer,
                )
            }),
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            ..
        } => project_status(provider_payload, selector, *mapping).and_then(|computed| {
            apply_value_renderer_with_request(request_text, provider_payload, computed, renderer)
        }),
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            max_items,
            ..
        } => execute_compose_collection(provider_payload, steps, *format, renderer, *max_items),
        ResponseOperation::CopyAfterPrefix {
            prefixes,
            trim,
            allow_multiline,
        } => execute_copy(request_text, prefixes, *trim, *allow_multiline),
        ResponseOperation::TestResultSummary {
            required_intent_phrases,
            forbidden_intent_terms,
        } => execute_test_summary(
            request_text,
            provider_payload,
            required_intent_phrases,
            forbidden_intent_terms,
        ),
        ResponseOperation::WaitOnYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait(provider_payload, function_name, *yield_time_ms, *max_tokens),
        ResponseOperation::WaitOnAnyYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait_any(provider_payload, function_name, *yield_time_ms, *max_tokens),
        ResponseOperation::WaitOnYieldedSurfaces {
            surfaces,
            function_name,
            yield_time_ms,
            max_tokens,
        } => execute_wait_surface(
            provider_payload,
            surfaces,
            function_name,
            *yield_time_ms,
            *max_tokens,
        ),
    };
    let Ok(response) = response else {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::Abstain,
            response.err().unwrap_or("abstain"),
        );
    };
    if response.is_empty() || response.len() > program.max_output_bytes {
        return ResponseExecution::rejected(ResponseExecutionStatus::Abstain, "output_budget");
    }
    if let Err(error) = validator(program, request_text, provider_payload, &response) {
        return ResponseExecution::rejected(
            ResponseExecutionStatus::VerifyFailed,
            format!("verification:{error}"),
        );
    }
    ResponseExecution {
        status: ResponseExecutionStatus::Executed,
        reason: "executed".to_owned(),
        response: Some(response),
        verification_receipt_id: None,
    }
}

/// Presents a captured pre-action payload as the provider envelope expected by
/// response programs. Existing complete envelopes stay borrowed; direct tool
/// values are wrapped deterministically without using the teacher response.
#[doc(hidden)]
pub fn provider_payload_view<'a>(
    request_text: &str,
    provider_payload: &'a Value,
) -> Result<std::borrow::Cow<'a, Value>, &'static str> {
    if request_text.len() > 16_384 {
        return Err("provider_view_request_budget");
    }
    if let Some(input) = provider_payload.get("input").and_then(Value::as_array) {
        if request_text.is_empty()
            || input
                .iter()
                .any(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        {
            return Ok(std::borrow::Cow::Borrowed(provider_payload));
        }
        let mut owned = provider_payload.clone();
        owned
            .get_mut("input")
            .and_then(Value::as_array_mut)
            .ok_or("provider_view_input_missing")?
            .insert(
                0,
                serde_json::json!({
                    "type": "message",
                    "role": "user",
                    "content": request_text,
                }),
            );
        return Ok(std::borrow::Cow::Owned(owned));
    }
    let output = serde_json::to_string(provider_payload)
        .map_err(|_| "provider_view_payload_serialization")?;
    if output.len() > 65_536 {
        return Err("provider_view_payload_budget");
    }
    let mut input = Vec::with_capacity(2);
    if !request_text.is_empty() {
        input.push(serde_json::json!({
            "type": "message",
            "role": "user",
            "content": request_text,
        }));
    }
    input.push(serde_json::json!({
        "type": "function_call_output",
        "output": output,
    }));
    Ok(std::borrow::Cow::Owned(serde_json::json!({"input": input})))
}

fn execute_unique_consensus(
    variants: &[ResponseConsensusVariant],
    adapter_wave: Option<&nando_operator_kernel::ResponseAdapterWaveConsensus>,
    request_text: &str,
    provider_payload: &Value,
    validator: &ExternalResponseValidator,
) -> Result<String, &'static str> {
    let layout =
        actor_structural_layout_sha256(provider_payload).map_err(|_| "unique_consensus_layout")?;
    let request_atoms = self::request_text(provider_payload)
        .map(|text| crate::request_phase_atom_ids(&text))
        .unwrap_or_default();
    let mut applicable = variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| {
            (variant.allowed_layout_sha256.is_empty()
                || variant.allowed_layout_sha256.binary_search(&layout).is_ok())
                && variant
                    .required_request_atom_ids
                    .iter()
                    .all(|atom| request_atoms.binary_search(atom).is_ok())
        })
        .map(|(index, variant)| {
            let margin = adapter_wave
                .and_then(|wave| wave.routes.get(index))
                .and_then(|route| {
                    actor_adapter_wave_margin(&variant.program, provider_payload, route)
                })
                .unwrap_or(i64::MIN);
            (index, variant, margin)
        })
        .collect::<Vec<_>>();
    if let Some(wave) = adapter_wave {
        applicable.retain(|(_, _, margin)| *margin != i64::MIN);
        applicable.sort_unstable_by(|left, right| {
            right.2.cmp(&left.2).then_with(|| left.0.cmp(&right.0))
        });
        let Some(best_margin) = applicable.first().map(|value| value.2) else {
            return Err("adapter_wave_no_candidate");
        };
        let tied = applicable
            .iter()
            .filter(|value| value.2 == best_margin)
            .count();
        if tied > usize::from(wave.exact_budget) {
            return Err("adapter_wave_tie_budget");
        }
        applicable.retain(|value| value.2 == best_margin);
    }
    let responses = applicable
        .into_iter()
        .filter_map(|(_, variant, _)| {
            let execution = execute_response_with_external_validator(
                &variant.program,
                request_text,
                provider_payload,
                validator,
            );
            (execution.status == ResponseExecutionStatus::Executed)
                .then_some(execution.response)
                .flatten()
        })
        .collect::<BTreeSet<_>>();
    if responses.len() != 1 {
        return Err(if responses.is_empty() {
            "unique_consensus_no_applicable_variant"
        } else {
            "unique_consensus_disagreement"
        });
    }
    responses
        .into_iter()
        .next()
        .ok_or("unique_consensus_no_applicable_variant")
}

fn actor_adapter_wave_margin(
    program: &ResponseProgram,
    provider_payload: &Value,
    route: &ResponseAdapterWaveRoute,
) -> Option<i64> {
    let atoms = actor_adapter_phase_atom_ids(program, provider_payload);
    adapter_wave_margin_from_atoms(&atoms, route)
}

#[doc(hidden)]
pub fn adapter_wave_margin_from_atoms(
    atoms: &[u64],
    route: &ResponseAdapterWaveRoute,
) -> Option<i64> {
    if atoms.is_empty() {
        return None;
    }
    let anchor_matches = route
        .anchor_atom_ids
        .iter()
        .any(|atom| atoms.binary_search(atom).is_ok());
    let fingerprint_matches = route
        .positive_fingerprint_ids
        .binary_search(&actor_adapter_wave_atom_fingerprint(atoms))
        .is_ok();
    if (!route.anchor_atom_ids.is_empty() || !route.positive_fingerprint_ids.is_empty())
        && !anchor_matches
        && !fingerprint_matches
    {
        return None;
    }
    let query = phase_vector_from_atom_ids(atoms.iter().copied(), usize::from(route.cells));
    let score = |center: &[i32]| {
        phase_margin_to_micro(
            query
                .iter()
                .zip(center.chunks_exact(2))
                .map(|(query, center)| {
                    query.re * f64::from(center[0]) / 1_000_000.0
                        + query.im * f64::from(center[1]) / 1_000_000.0
                })
                .sum::<f64>()
                / f64::from(route.cells),
        )
        .ok()
    };
    std::iter::once((&route.center_delta_micro, route.threshold_micro))
        .chain(
            route
                .subcenters
                .iter()
                .map(|center| (&center.center_delta_micro, center.threshold_micro)),
        )
        .filter_map(|(center, threshold)| {
            score(center)
                .filter(|margin| *margin >= threshold)
                .map(|margin| margin.saturating_sub(threshold))
        })
        .max()
}

fn actor_adapter_wave_atom_fingerprint(atoms: &[u64]) -> u64 {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
    for byte in atoms.iter().flat_map(|atom| atom.to_le_bytes()) {
        fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    fingerprint
}

#[doc(hidden)]
pub fn actor_adapter_phase_atom_ids(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    if matches!(
        program.operation,
        ResponseOperation::FunctionCallFromRoles { .. }
            | ResponseOperation::CustomToolCallFromRoles { .. }
    ) {
        return crate::response_program_grounded_routing_atom_ids(program, provider_payload);
    }
    let mut atoms = Vec::new();
    let mut identifiers = Vec::<String>::new();
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => {
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            actor_selector_adapter_atoms(selector, provider_payload, &mut atoms, &mut identifiers);
            if execute_function_call_from_roles(
                provider_payload,
                function_name,
                selector,
                arguments,
            )
            .is_ok()
            {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => {
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            actor_selector_adapter_atoms(selector, provider_payload, &mut atoms, &mut identifiers);
            if execute_custom_tool_call_from_roles(
                provider_payload,
                custom_tool_name,
                inner_tool_name,
                selector,
                arguments,
                projection,
            )
            .is_ok()
            {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::ProjectSelectedValue { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:project"));
            actor_selector_adapter_atoms(selector, provider_payload, &mut atoms, &mut identifiers);
        }
        ResponseOperation::ProjectStatus { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:status"));
            actor_selector_adapter_atoms(selector, provider_payload, &mut atoms, &mut identifiers);
        }
        ResponseOperation::ComposeCollection { steps, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:collection"));
            for step in steps {
                match step {
                    CollectionProgramStep::SelectTurnOutput { output_ordinal } => {
                        atoms.push(crate::stable_atom_id(&format!(
                            "adapter:collection_output_ordinal:{output_ordinal}"
                        )));
                    }
                    CollectionProgramStep::SelectField { field }
                    | CollectionProgramStep::FilterFieldEquals { field, .. }
                    | CollectionProgramStep::ProjectField { field } => {
                        identifiers.push(field.clone());
                    }
                    CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                        selector,
                        ..
                    } => actor_selector_adapter_atoms(
                        selector,
                        provider_payload,
                        &mut atoms,
                        &mut identifiers,
                    ),
                    CollectionProgramStep::SelectOnlyArrayField
                    | CollectionProgramStep::FilterUniqueFieldEquals { .. }
                    | CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { .. }
                    | CollectionProgramStep::ProjectUniqueFieldByType { .. }
                    | CollectionProgramStep::ProjectOnlyNonFilterField
                    | CollectionProgramStep::AggregateUniqueIntegerField { .. }
                    | CollectionProgramStep::Count => {}
                }
            }
            if execute_compose_collection_from_program(program, provider_payload) {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        _ => return Vec::new(),
    }
    let request_tokens = request_text(provider_payload)
        .map(|text| identifier_tokens(&text))
        .unwrap_or_default();
    atoms.extend(actor_adapter_request_lexical_atoms(&request_tokens));
    let mentioned = identifiers
        .iter()
        .filter(|identifier| request_mentions_identifier(&request_tokens, identifier))
        .count();
    let relation = if identifiers.is_empty() {
        "none_available"
    } else if mentioned == 0 {
        "none_mentioned"
    } else if mentioned == identifiers.len() {
        "all_mentioned"
    } else {
        "some_mentioned"
    };
    atoms.push(crate::stable_atom_id(&format!(
        "adapter:request_identifier_relation:{relation}"
    )));
    atoms.extend(crate::response_pre_action_context_atom_ids(
        provider_payload,
    ));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn actor_adapter_request_lexical_atoms(tokens: &[String]) -> Vec<u64> {
    let bounded = tokens
        .iter()
        .filter(|token| !token.is_empty() && token.len() <= 64)
        .take(32)
        .collect::<Vec<_>>();
    let mut atoms = bounded
        .iter()
        .map(|token| crate::stable_atom_id(&format!("adapter:request_unigram:{token}")))
        .collect::<Vec<_>>();
    atoms.extend(bounded.windows(2).map(|window| {
        crate::stable_atom_id(&format!(
            "adapter:request_bigram:{}:{}",
            window[0], window[1]
        ))
    }));
    atoms
}

fn execute_compose_collection_from_program(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> bool {
    let ResponseOperation::ComposeCollection {
        steps,
        format,
        renderer,
        max_items,
        ..
    } = &program.operation
    else {
        return false;
    };
    execute_compose_collection(provider_payload, steps, *format, renderer, *max_items).is_ok()
}

fn actor_selector_adapter_atoms(
    selector: &ResponseValueSelector,
    provider_payload: &Value,
    atoms: &mut Vec<u64>,
    identifiers: &mut Vec<String>,
) {
    let (family, position, value_type) = match selector {
        ResponseValueSelector::ContinuationHandle { value_type } => {
            ("continuation_handle", None, Some(value_type))
        }
        ResponseValueSelector::UniqueScalar { value_type } => {
            ("unique_scalar", None, Some(value_type))
        }
        ResponseValueSelector::UniqueTurnScalar { value_type } => {
            ("unique_turn_scalar", None, Some(value_type))
        }
        ResponseValueSelector::ContentLinePrefix { value_type, .. } => {
            ("line_prefix", None, Some(value_type))
        }
        ResponseValueSelector::JsonField { field, value_type } => {
            identifiers.push(field.clone());
            ("json_field", None, Some(value_type))
        }
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => ("json_ordinal", Some(u64::from(*ordinal)), Some(value_type)),
        ResponseValueSelector::UniqueTurnJsonField { field, value_type } => {
            identifiers.push(field.clone());
            ("turn_json_field", None, Some(value_type))
        }
        ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            identifiers.push(field.clone());
            ("active_turn_json_field", None, Some(value_type))
        }
        ResponseValueSelector::RequestReferencedJsonField { value_type } => {
            ("request_referenced", None, Some(value_type))
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => (
            "request_referenced_ordinal",
            Some(u64::from(*ordinal)),
            Some(value_type),
        ),
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => (
            "turn_output_line",
            Some((u64::from(*output_ordinal) << 16) | u64::from(*line_index)),
            Some(value_type),
        ),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal,
            scalar_ordinal,
            value_type,
        } => (
            "turn_output_scalar",
            Some((u64::from(*output_ordinal) << 16) | u64::from(*scalar_ordinal)),
            Some(value_type),
        ),
        ResponseValueSelector::LatestTurnOutputLine {
            line_index,
            value_type,
        } => (
            "latest_line",
            Some(u64::from(*line_index)),
            Some(value_type),
        ),
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => (
            "latest_scalar",
            Some(u64::from(*scalar_ordinal)),
            Some(value_type),
        ),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => (
            "latest_scalar_from_end",
            Some(u64::from(*reverse_ordinal)),
            Some(value_type),
        ),
        ResponseValueSelector::CommandOutputBody => ("command_body", None, None),
        ResponseValueSelector::RequestLastToken => ("request_last_token", None, None),
        ResponseValueSelector::RequestUniqueLiteral => ("request_unique_literal", None, None),
    };
    atoms.push(crate::stable_atom_id(&format!(
        "adapter:selector_family:{family}"
    )));
    if let Some(position) = position {
        atoms.push(crate::stable_atom_id(&format!(
            "adapter:position:{position}"
        )));
    }
    if let Some(value_type) = value_type {
        atoms.push(crate::stable_atom_id(&format!(
            "adapter:value_type:{}",
            adapter_value_type_name(*value_type)
        )));
    }
    if let Ok(selected) = immediate_selected_scalar(provider_payload, selector) {
        atoms.push(crate::stable_atom_id("adapter:executes"));
        if identifiers.is_empty()
            && let Some(identifier) =
                actor_unique_output_key_for_scalar(provider_payload, &selected.value)
        {
            identifiers.push(identifier);
        }
    }
}

fn actor_unique_output_key_for_scalar(
    provider_payload: &Value,
    selected: &Value,
) -> Option<String> {
    let mut identifiers = BTreeSet::new();
    let input = provider_payload.get("input")?.as_array()?;
    for item in input {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        if let Some(text) = output.as_str() {
            for object in runtime_embedded_json_objects(text) {
                actor_collect_scalar_keys(&Value::Object(object), selected, &mut identifiers, 0);
            }
        } else {
            actor_collect_scalar_keys(output, selected, &mut identifiers, 0);
        }
        if identifiers.len() > 1 {
            return None;
        }
    }
    (identifiers.len() == 1)
        .then(|| identifiers.into_iter().next())
        .flatten()
}

fn actor_collect_scalar_keys(
    value: &Value,
    selected: &Value,
    identifiers: &mut BTreeSet<String>,
    depth: usize,
) {
    if depth > 8 || identifiers.len() > 1 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if value == selected {
                    identifiers.insert(key.clone());
                }
                actor_collect_scalar_keys(value, selected, identifiers, depth.saturating_add(1));
            }
        }
        Value::Array(values) => {
            for value in values {
                actor_collect_scalar_keys(value, selected, identifiers, depth.saturating_add(1));
            }
        }
        _ => {}
    }
}

const fn adapter_value_type_name(value_type: AtomValueType) -> &'static str {
    match value_type {
        AtomValueType::String => "string",
        AtomValueType::Integer => "integer",
        AtomValueType::Boolean => "boolean",
        AtomValueType::Identifier => "identifier",
        AtomValueType::Collection => "collection",
    }
}

#[doc(hidden)]
pub fn actor_structural_layout_sha256(value: &Value) -> Result<String, &'static str> {
    // Layout guards describe the current observation, not the entire provider
    // transcript. History length must not fragment equivalent tool outcomes.
    let output = immediate_tool_output_value(value).ok_or("immediate_tool_output_missing")?;
    crate::canonical_json_sha256(&actor_structural_layout(output))
}

fn actor_structural_layout(value: &Value) -> Value {
    match value {
        Value::Null => Value::String("null".to_owned()),
        Value::Bool(_) => Value::String("bool".to_owned()),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Value::String("integer".to_owned())
        }
        Value::Number(_) => Value::String("number".to_owned()),
        Value::String(value) => serde_json::from_str::<Value>(value)
            .ok()
            .filter(|parsed| !matches!(parsed, Value::String(_)))
            .map_or_else(
                || Value::String("string".to_owned()),
                |parsed| actor_structural_layout(&parsed),
            ),
        Value::Array(values) => Value::Array(values.iter().map(actor_structural_layout).collect()),
        Value::Object(values) => {
            let mut shapes = values
                .iter()
                .map(|(key, value)| {
                    Value::Array(vec![
                        Value::String(crate::sha256_bytes(key.as_bytes())),
                        actor_structural_layout(value),
                    ])
                })
                .collect::<Vec<_>>();
            shapes.sort_by_cached_key(|shape| serde_json::to_vec(shape).unwrap_or_default());
            Value::Array(shapes)
        }
    }
}

fn execute_advance_plan(
    provider_payload: &Value,
    function_name: &str,
) -> Result<String, &'static str> {
    let output = immediate_tool_output_value(provider_payload)
        .ok_or("plan_immediate_tool_output_missing")?;
    if !actor_explicit_tool_success(output) {
        return Err("plan_tool_success_missing");
    }
    let mut plan = actor_latest_plan(provider_payload, function_name)?;
    let active = actor_validate_canonical_plan(&plan)?;
    let current = plan
        .get_mut(active)
        .and_then(Value::as_object_mut)
        .ok_or("plan_active_step_missing")?;
    current.insert("status".to_owned(), Value::String("completed".to_owned()));
    if let Some(next) = plan.get_mut(active.saturating_add(1)) {
        let next = next.as_object_mut().ok_or("plan_next_step_invalid")?;
        next.insert("status".to_owned(), Value::String("in_progress".to_owned()));
    }
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": {"plan": plan},
    }))
    .map_err(|_| "plan_serialization")
}

fn actor_latest_plan(
    provider_payload: &Value,
    function_name: &str,
) -> Result<Vec<Value>, &'static str> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or("plan_input_missing")?;
    let call = items
        .iter()
        .rev()
        .skip(1)
        .find(|item| {
            item.get("type").and_then(Value::as_str) == Some("function_call")
                && item.get("name").and_then(Value::as_str) == Some(function_name)
        })
        .ok_or("plan_previous_call_missing")?;
    let arguments = call
        .get("arguments")
        .ok_or("plan_previous_arguments_missing")?;
    let parsed;
    let arguments = if let Some(arguments) = arguments.as_str() {
        parsed = serde_json::from_str::<Value>(arguments)
            .map_err(|_| "plan_previous_arguments_invalid")?;
        &parsed
    } else {
        arguments
    };
    arguments
        .get("plan")
        .and_then(Value::as_array)
        .filter(|plan| !plan.is_empty() && plan.len() <= 32)
        .cloned()
        .ok_or("plan_previous_state_missing")
}

fn actor_validate_canonical_plan(plan: &[Value]) -> Result<usize, &'static str> {
    let mut active = None;
    for (index, step) in plan.iter().enumerate() {
        let step = step.as_object().ok_or("plan_step_not_object")?;
        if step.len() != 2 {
            return Err("plan_step_schema_mismatch");
        }
        let text = step
            .get("step")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty() && text.len() <= 1_024)
            .ok_or("plan_step_text_invalid")?;
        if text.chars().any(char::is_control) {
            return Err("plan_step_text_control");
        }
        let status = step
            .get("status")
            .and_then(Value::as_str)
            .ok_or("plan_step_status_missing")?;
        match status {
            "completed" if active.is_none() => {}
            "in_progress" if active.is_none() => active = Some(index),
            "pending" if active.is_some() => {}
            _ => return Err("plan_status_order_invalid"),
        }
    }
    active.ok_or("plan_active_step_ambiguous")
}

#[doc(hidden)]
pub fn advance_plan_runtime_state(
    provider_payload: &Value,
    function_name: &str,
) -> Option<(u16, u16, u16)> {
    let output = immediate_tool_output_value(provider_payload)?;
    if !actor_explicit_tool_success(output) {
        return None;
    }
    let plan = actor_latest_plan(provider_payload, function_name).ok()?;
    let active_index = actor_validate_canonical_plan(&plan).ok()?;
    Some((
        u16::try_from(plan.len()).ok()?,
        u16::try_from(active_index).ok()?,
        u16::try_from(active_index).ok()?,
    ))
}

fn actor_explicit_tool_success(output: &Value) -> bool {
    match output {
        Value::Object(object) => {
            if let Some(exit_code) = object.get("exit_code") {
                return exit_code.as_i64() == Some(0);
            }
            if let Some(ok) = object.get("ok") {
                return ok.as_bool() == Some(true);
            }
            if let Some(status) = object.get("status").and_then(Value::as_str) {
                return matches!(
                    status.to_ascii_lowercase().as_str(),
                    "success" | "succeeded" | "pass" | "passed" | "ok" | "completed"
                );
            }
            object
                .get("result")
                .is_some_and(actor_explicit_tool_success)
        }
        Value::String(text) if !text.is_empty() && text.len() <= 131_072 => {
            if let Ok(decoded) = serde_json::from_str::<Value>(text) {
                return actor_explicit_tool_success(&decoded);
            }
            matches!(
                text.trim().to_ascii_lowercase().as_str(),
                "success" | "succeeded" | "pass" | "passed" | "ok" | "completed"
            ) || actor_transport_exit_success(text)
        }
        Value::Array(parts) if parts.len() == 1 => parts[0]
            .get("text")
            .is_some_and(actor_explicit_tool_success),
        _ => false,
    }
}

fn actor_transport_exit_success(text: &str) -> bool {
    const PREFIX: &str = "Process exited with code ";
    let mut exit_code = None;
    for line in text.lines() {
        let Some(raw_code) = line.strip_prefix(PREFIX) else {
            continue;
        };
        if exit_code.is_some()
            || raw_code.is_empty()
            || !raw_code.bytes().all(|byte| byte.is_ascii_digit())
        {
            return false;
        }
        let Ok(code) = raw_code.parse::<u16>() else {
            return false;
        };
        exit_code = Some(code);
    }
    exit_code == Some(0)
}

fn execute_compose_collection(
    provider_payload: &Value,
    steps: &[CollectionProgramStep],
    format: ValueProjectionFormat,
    renderer: &CollectionOutputRenderer,
    max_items: usize,
) -> Result<String, &'static str> {
    let (output, transform_steps) = match steps.first() {
        Some(CollectionProgramStep::SelectTurnOutput { output_ordinal }) => (
            active_turn_output_value(provider_payload, Some(*output_ordinal))?,
            &steps[1..],
        ),
        _ => (
            immediate_tool_output_value(provider_payload).ok_or("immediate_tool_output_missing")?,
            steps,
        ),
    };
    let mut value = collection_json_from_value(output)?;
    let mut filter_field = None::<String>;
    for step in transform_steps {
        value = match step {
            CollectionProgramStep::SelectTurnOutput { .. } => {
                return Err("collection_output_selector_position");
            }
            CollectionProgramStep::SelectOnlyArrayField => {
                let object = value.as_object().ok_or("collection_select_not_object")?;
                let mut arrays = object.values().filter(|candidate| candidate.is_array());
                let selected = arrays.next().cloned().ok_or("collection_select_missing")?;
                if arrays.next().is_some() {
                    return Err("collection_select_ambiguous");
                }
                selected
            }
            CollectionProgramStep::SelectField { field } => value
                .as_object()
                .and_then(|object| object.get(field))
                .cloned()
                .ok_or("collection_select_missing")?,
            CollectionProgramStep::FilterFieldEquals {
                field,
                value: expected,
            } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| {
                            row.as_object().and_then(|object| object.get(field))
                                == Some(&expected.as_json())
                        })
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEquals { value: expected } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let expected = expected.as_json();
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_filter_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_filter_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_filter_field_ambiguous");
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue { value_type } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let (field, expected) =
                    request_grounded_collection_value(provider_payload, rows, *value_type)?;
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector,
                value_type,
            } => {
                let rows = value.as_array().ok_or("collection_filter_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let selected = immediate_selected_scalar(provider_payload, selector)?;
                if collection_atom_value_type(selected.value_type) != Some(*value_type) {
                    return Err("collection_filter_value_type");
                }
                let expected = selected.value;
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_filter_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_filter_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_filter_field_ambiguous");
                }
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::ProjectField { field } => {
                if let Some(rows) = value.as_array() {
                    if rows.len() > max_items {
                        return Err("collection_item_budget");
                    }
                    let projected = rows
                        .iter()
                        .map(|row| {
                            row.as_object()
                                .and_then(|object| object.get(field))
                                .cloned()
                                .ok_or("collection_project_missing")
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Value::Array(projected)
                } else {
                    value
                        .as_object()
                        .and_then(|object| object.get(field))
                        .cloned()
                        .ok_or("collection_project_missing")?
                }
            }
            CollectionProgramStep::ProjectUniqueFieldByType { value_type } => {
                let rows = value.as_array().ok_or("collection_project_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_project_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.get(*field)
                            .is_some_and(|value| collection_scalar_type(value) == Some(*value_type))
                    })
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_project_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_project_field_ambiguous");
                }
                Value::Array(
                    rows.iter()
                        .map(|row| row.get(&field).cloned().ok_or("collection_project_missing"))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::ProjectOnlyNonFilterField => {
                let excluded = filter_field
                    .as_deref()
                    .ok_or("collection_filter_role_missing")?;
                let rows = value.as_array().ok_or("collection_project_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_project_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    field.as_str() != excluded
                        && rows.iter().all(|row| {
                            row.as_object()
                                .is_some_and(|object| object.contains_key(*field))
                        })
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_project_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_project_field_ambiguous");
                }
                Value::Array(
                    rows.iter()
                        .map(|row| row.get(&field).cloned().ok_or("collection_project_missing"))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::AggregateUniqueIntegerField { operation } => {
                let rows = value.as_array().ok_or("collection_aggregate_not_array")?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err("collection_item_budget");
                }
                let first = rows[0]
                    .as_object()
                    .ok_or("collection_aggregate_row_not_object")?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter()
                        .all(|row| row.get(*field).and_then(Value::as_i64).is_some())
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or("collection_aggregate_field_missing")?;
                if fields.next().is_some() {
                    return Err("collection_aggregate_field_ambiguous");
                }
                let values = rows
                    .iter()
                    .map(|row| {
                        row.get(&field)
                            .and_then(Value::as_i64)
                            .ok_or("collection_aggregate_value_missing")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let aggregate = match operation {
                    CollectionAggregateOperation::Sum => values
                        .into_iter()
                        .try_fold(0_i64, i64::checked_add)
                        .ok_or("collection_aggregate_overflow")?,
                    CollectionAggregateOperation::Min => values
                        .into_iter()
                        .min()
                        .ok_or("collection_aggregate_empty")?,
                    CollectionAggregateOperation::Max => values
                        .into_iter()
                        .max()
                        .ok_or("collection_aggregate_empty")?,
                };
                Value::from(aggregate)
            }
            CollectionProgramStep::Count => {
                let count = value
                    .as_array()
                    .map(Vec::len)
                    .or_else(|| value.as_object().map(serde_json::Map::len))
                    .ok_or("collection_count_unsupported")?;
                Value::from(u64::try_from(count).map_err(|_| "collection_count_overflow")?)
            }
        };
    }
    let computed = match format {
        ValueProjectionFormat::CanonicalJson => {
            serde_json::to_string(&value).map_err(|_| "collection_serialization")
        }
        ValueProjectionFormat::PlainText => match value {
            Value::String(text) if !text.contains(['\n', '\r']) => Ok(text),
            Value::String(_) => Err("collection_multiline"),
            Value::Bool(_) | Value::Number(_) | Value::Null => Ok(value.to_string()),
            Value::Array(_) | Value::Object(_) => Err("collection_plain_text_non_scalar"),
        },
    }?;
    apply_value_renderer(provider_payload, computed, renderer)
}

fn request_grounded_collection_value(
    provider_payload: &Value,
    rows: &[Value],
    value_type: CollectionScalarType,
) -> Result<(String, Value), &'static str> {
    let request = request_text(provider_payload).ok_or("collection_request_text_missing")?;
    let mut matches = BTreeMap::<Vec<u8>, (String, Value)>::new();
    for row in rows {
        let object = row.as_object().ok_or("collection_filter_row_not_object")?;
        for (field, value) in object {
            if collection_value_type(value) == Some(value_type)
                && request_contains_collection_value(&request, value)
            {
                let key = serde_json::to_vec(&(field, value))
                    .map_err(|_| "collection_request_value_encode")?;
                matches.insert(key, (field.clone(), value.clone()));
            }
        }
    }
    if matches.len() != 1 {
        return Err("collection_request_value_ambiguous");
    }
    matches
        .into_values()
        .next()
        .ok_or("collection_request_value_missing")
}

fn request_text(provider_payload: &Value) -> Option<String> {
    let mut parts = Vec::new();
    for item in provider_payload.get("input")?.as_array()? {
        if item.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        match item.get("content") {
            Some(Value::String(text)) if !text.is_empty() => parts.push(text.as_str()),
            Some(Value::Array(content)) => {
                parts.extend(
                    content
                        .iter()
                        .filter_map(|part| part.get("text").and_then(Value::as_str)),
                );
            }
            _ => {}
        }
    }
    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn collection_value_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        _ => None,
    }
}

const fn collection_atom_value_type(value_type: AtomValueType) -> Option<CollectionScalarType> {
    match value_type {
        AtomValueType::String | AtomValueType::Identifier => Some(CollectionScalarType::String),
        AtomValueType::Integer => Some(CollectionScalarType::Integer),
        AtomValueType::Boolean => Some(CollectionScalarType::Boolean),
        AtomValueType::Collection => None,
    }
}

fn request_contains_collection_value(request: &str, value: &Value) -> bool {
    let needle = match value {
        Value::String(value) if !value.is_empty() && value.len() <= 128 => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return false,
    };
    request.match_indices(&needle).any(|(start, _)| {
        let end = start.saturating_add(needle.len());
        let left_ok = request[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        let right_ok = request[end..]
            .chars()
            .next()
            .is_none_or(|character| !character.is_alphanumeric() && character != '_');
        left_ok && right_ok
    })
}

fn apply_output_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    match renderer {
        CollectionOutputRenderer::Direct => Ok(computed),
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            Ok(format!("{prefix}{computed}{suffix}"))
        }
        CollectionOutputRenderer::RenderSequence { .. } => {
            Err("collection_render_sequence_unsupported")
        }
        CollectionOutputRenderer::RequestTemplate { marker } => {
            apply_request_template(provider_payload, computed, *marker)
        }
    }
}

fn apply_value_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    apply_value_renderer_inner(None, provider_payload, computed, renderer)
}

fn apply_value_renderer_with_request(
    request_text: &str,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    apply_value_renderer_inner(Some(request_text), provider_payload, computed, renderer)
}

fn apply_value_renderer_inner(
    request_text: Option<&str>,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, &'static str> {
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return apply_output_renderer(provider_payload, computed, renderer);
    };
    let mut output = String::new();
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => output.push_str(text),
            ResponseRenderSegment::Primary => output.push_str(&computed),
            ResponseRenderSegment::Selected { selector, format } => {
                let selected = match request_text {
                    Some(request_text) => project_selected_value_with_request(
                        request_text,
                        provider_payload,
                        selector,
                        *format,
                    )?,
                    None => project_selected_value(provider_payload, selector, *format)?,
                };
                output.push_str(&selected);
            }
        }
        if output.len() > 16_384 {
            return Err("projection_output_budget");
        }
    }
    Ok(output)
}

fn apply_request_template(
    provider_payload: &Value,
    computed: String,
    marker: RequestTemplateMarker,
) -> Result<String, &'static str> {
    let request = request_text(provider_payload).ok_or("request_template_text_missing")?;
    let template = unique_request_template(&request, marker.token())?;
    let output = template.replacen(marker.token(), &computed, 1);
    if output.is_empty() || output.len() > 16_384 {
        return Err("request_template_output_budget");
    }
    Ok(output)
}

fn unique_request_template(request: &str, marker: &str) -> Result<String, &'static str> {
    let mut templates = BTreeMap::<String, ()>::new();
    for delimiter in ['`', '\'', '"'] {
        let parts = request.split(delimiter).collect::<Vec<_>>();
        for value in parts.iter().skip(1).step_by(2) {
            let value = value.trim();
            if !value.is_empty()
                && value.len() <= 512
                && !value.contains(['\n', '\r'])
                && value.matches(marker).count() == 1
            {
                templates.insert(value.to_owned(), ());
            }
        }
    }
    if templates.len() != 1 {
        return Err("request_template_cardinality");
    }
    templates
        .into_keys()
        .next()
        .ok_or("request_template_missing")
}

fn collection_json_from_value(output: &Value) -> Result<Value, &'static str> {
    let mut texts = Vec::new();
    let mut total_bytes = 0_usize;
    match output {
        Value::String(text) => texts.push(text.as_str()),
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            for part in parts {
                if !matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) {
                    return Err("collection_output_part_type");
                }
                texts.push(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or("collection_output_part_text")?,
                );
            }
        }
        _ => return Err("collection_output_not_text"),
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for text in texts {
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or("collection_input_budget")?;
        if text.is_empty() || total_bytes > 65_536 {
            return Err("collection_input_budget");
        }
        collect_runtime_collection_candidates(text, &mut candidates)?;
    }
    if candidates.len() != 1 {
        return Err(if candidates.is_empty() {
            "collection_input_not_json"
        } else {
            "collection_input_ambiguous"
        });
    }
    Ok(candidates.into_values().next().expect("one candidate"))
}

fn collect_runtime_collection_candidates(
    output: &str,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), &'static str> {
    let mut sources = vec![output.to_owned()];
    let mut fenced = None::<String>;
    for line in output.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if fenced.is_some() {
            let buffer = fenced.as_mut().expect("checked above");
            if !buffer.is_empty() {
                buffer.push('\n');
            }
            buffer.push_str(line);
        } else if trimmed == "```" || trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if trimmed.starts_with(['{', '[']) {
            sources.push(trimmed.to_owned());
        }
    }
    for source in sources {
        if source.is_empty() || source.len() > 16_384 {
            continue;
        }
        for object in runtime_embedded_json_objects(&source) {
            let value = Value::Object(object);
            if bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value).map_err(|_| "collection_serialization")?;
                candidates.insert(key, value);
            }
        }
        if let Ok(value @ Value::Array(_)) = serde_json::from_str::<Value>(&source)
            && !is_text_part_array(&value)
        {
            let value = serde_json::json!({"items": value});
            if bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value).map_err(|_| "collection_serialization")?;
                candidates.insert(key, value);
            }
        }
    }
    Ok(())
}

fn is_text_part_array(value: &Value) -> bool {
    value.as_array().is_some_and(|parts| {
        !parts.is_empty()
            && parts.iter().all(|part| {
                matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) && part.get("text").is_some_and(Value::is_string)
            })
    })
}

fn bounded_collection_root(value: &Value) -> bool {
    let Some(object) = value.as_object().filter(|object| object.len() <= 16) else {
        return false;
    };
    let mut arrays = object.values().filter_map(Value::as_array);
    let Some(rows) = arrays.next() else {
        return false;
    };
    if arrays.next().is_some() || rows.is_empty() || rows.len() > 1_024 {
        return false;
    }
    rows.iter().all(|row| {
        row.as_object().is_some_and(|fields| {
            !fields.is_empty()
                && fields.len() <= 16
                && fields.iter().all(|(name, value)| {
                    safe_collection_identifier(name) && safe_collection_scalar(value)
                })
        })
    })
}

fn safe_collection_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if value.len() > 64 || !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        && ![
            "auth",
            "cookie",
            "credential",
            "passwd",
            "password",
            "secret",
            "token",
            "api_key",
            "apikey",
            "private_key",
            "privatekey",
        ]
        .iter()
        .any(|private| lower.contains(private))
}

fn safe_collection_scalar(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::Number(number) => {
            number
                .as_i64()
                .is_some_and(|value| (-(1_i64 << 53)..=(1_i64 << 53)).contains(&value))
                || number.as_u64().is_some_and(|value| value <= (1_u64 << 53))
        }
        Value::String(text) => {
            text.len() <= 128
                && ![
                    "auth",
                    "cookie",
                    "credential",
                    "passwd",
                    "password",
                    "secret",
                    "token",
                    "api_key",
                    "apikey",
                    "private_key",
                    "privatekey",
                ]
                .iter()
                .any(|private| text.to_ascii_lowercase().contains(private))
        }
        Value::Array(_) | Value::Object(_) => false,
    }
}

fn collection_scalar_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        Value::Null | Value::Array(_) | Value::Object(_) | Value::Number(_) => None,
    }
}

#[doc(hidden)]
pub fn project_selected_value(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    format: ValueProjectionFormat,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    format_selected_scalar(selected, format)
}

#[doc(hidden)]
pub fn project_selected_value_with_request(
    request_text: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    format: ValueProjectionFormat,
) -> Result<String, &'static str> {
    let selected =
        immediate_selected_scalar_with_request(request_text, provider_payload, selector)?;
    format_selected_scalar(selected, format)
}

#[doc(hidden)]
pub fn selected_value_with_request(
    request_text: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<ExtractedScalar, &'static str> {
    immediate_selected_scalar_with_request(request_text, provider_payload, selector)
}

fn format_selected_scalar(
    selected: ExtractedScalar,
    format: ValueProjectionFormat,
) -> Result<String, &'static str> {
    let projected = match format {
        ValueProjectionFormat::PlainText => match &selected.value {
            Value::String(text) if !text.contains(['\n', '\r']) => text.clone(),
            Value::String(_) => return Err("projection_multiline"),
            Value::Bool(_) | Value::Number(_) => selected.value.to_string(),
            _ => return Err("projection_non_scalar"),
        },
        ValueProjectionFormat::CanonicalJson => {
            serde_json::to_string(&selected.value).map_err(|_| "projection_serialization")?
        }
    };
    if projected.is_empty() || projected.len() > 16_384 {
        return Err("projection_output_budget");
    }
    Ok(projected)
}

#[doc(hidden)]
pub fn project_status(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    mapping: ProjectStatusMapping,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    if selected.value_type != AtomValueType::Integer {
        return Err("status_selector_type_mismatch");
    }
    let code = selected
        .value
        .as_u64()
        .filter(|code| *code <= MAX_PROJECT_STATUS_CODE)
        .ok_or("status_integer_out_of_bounds")?;
    let status = match mapping {
        ProjectStatusMapping::ZeroIsSuccess if code == 0 => "success",
        ProjectStatusMapping::ZeroIsSuccess => "failure",
        ProjectStatusMapping::ZeroIsPass if code == 0 => "PASS",
        ProjectStatusMapping::ZeroIsPass => "FAIL",
        ProjectStatusMapping::ZeroIsOk if code == 0 => "OK",
        ProjectStatusMapping::ZeroIsOk => "ERROR",
        ProjectStatusMapping::ZeroIsTrue if code == 0 => "true",
        ProjectStatusMapping::ZeroIsTrue => "false",
    };
    Ok(status.to_owned())
}

fn execute_function_call_from_roles(
    provider_payload: &Value,
    function_name: &str,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
) -> Result<String, &'static str> {
    let scalar = immediate_selected_scalar(provider_payload, selector)?;
    let mut projected = serde_json::Map::new();
    for argument in arguments {
        match argument {
            ResponseArgument::Role {
                name,
                role: SemanticRole::ContinuationHandle | SemanticRole::SourceValue,
                value_type,
            } => {
                projected.insert(
                    name.clone(),
                    runtime_role_value(&scalar.value, *value_type)?,
                );
            }
            ResponseArgument::Role { .. } => return Err("unsupported_runtime_role"),
            ResponseArgument::Integer { name, value } => {
                projected.insert(name.clone(), Value::from(*value));
            }
            ResponseArgument::String { name, value } => {
                projected.insert(name.clone(), Value::String(value.clone()));
            }
            ResponseArgument::Boolean { name, value } => {
                projected.insert(name.clone(), Value::Bool(*value));
            }
        }
    }
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": projected,
    }))
    .map_err(|_| "function_call_serialization")
}

fn execute_custom_tool_call_from_roles(
    provider_payload: &Value,
    custom_tool_name: &str,
    inner_tool_name: &str,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
    projection: &CustomToolResultProjection,
) -> Result<String, &'static str> {
    let selected = immediate_selected_scalar(provider_payload, selector)?;
    let projected = project_arguments(arguments, &selected)?;
    let arguments_json =
        serde_json::to_string(&projected).map_err(|_| "custom_tool_arguments_serialization")?;
    let source = match projection {
        CustomToolResultProjection::OutputField { output_field } => format!(
            "const r=await tools.{inner_tool_name}({arguments_json});text(r.{output_field});"
        ),
        CustomToolResultProjection::OutputAndContinuation {
            output_field,
            continuation_field,
            continuation_prefix,
        } => {
            let prefix = serde_json::to_string(continuation_prefix)
                .map_err(|_| "custom_tool_prefix_serialization")?;
            format!(
                "const r=await tools.{inner_tool_name}({arguments_json});text(r.{output_field});if(r.{continuation_field})text({prefix}+r.{continuation_field});"
            )
        }
        CustomToolResultProjection::JsonStringifyResult => format!(
            "const r=await tools.{inner_tool_name}({arguments_json});text(JSON.stringify(r));"
        ),
    };
    serde_json::to_string(&serde_json::json!({
        "kind": "custom_tool_call",
        "name": custom_tool_name,
        "input": source,
    }))
    .map_err(|_| "custom_tool_call_serialization")
}

fn project_arguments(
    arguments: &[ResponseArgument],
    selected: &ExtractedScalar,
) -> Result<serde_json::Map<String, Value>, &'static str> {
    let mut projected = serde_json::Map::new();
    for argument in arguments {
        match argument {
            ResponseArgument::Role {
                name,
                role: SemanticRole::ContinuationHandle | SemanticRole::SourceValue,
                value_type,
            } => {
                projected.insert(
                    name.clone(),
                    runtime_role_value(&selected.value, *value_type)?,
                );
            }
            ResponseArgument::Role { .. } => return Err("unsupported_runtime_role"),
            ResponseArgument::Integer { name, value } => {
                projected.insert(name.clone(), Value::from(*value));
            }
            ResponseArgument::String { name, value } => {
                projected.insert(name.clone(), Value::String(value.clone()));
            }
            ResponseArgument::Boolean { name, value } => {
                projected.insert(name.clone(), Value::Bool(*value));
            }
        }
    }
    Ok(projected)
}

fn runtime_role_value(
    value: &Value,
    value_type: Option<AtomValueType>,
) -> Result<Value, &'static str> {
    match value_type {
        None => Ok(value.clone()),
        Some(AtomValueType::Integer) => value
            .as_u64()
            .or_else(|| value.as_str()?.parse::<u64>().ok())
            .map(Value::from)
            .ok_or("role_integer_parse"),
        Some(AtomValueType::Boolean) => value
            .as_bool()
            .or_else(|| value.as_str()?.parse::<bool>().ok())
            .map(Value::from)
            .ok_or("role_boolean_parse"),
        Some(AtomValueType::String | AtomValueType::Identifier) => value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .ok_or("role_string_parse"),
        Some(AtomValueType::Collection) => Err("role_collection_unsupported"),
    }
}

fn execute_wait_surface(
    provider_payload: &Value,
    surfaces: &[String],
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let surface = immediate_yielded_surface(provider_payload).ok_or("yielded_surface_missing")?;
    if !surfaces.iter().any(|allowed| allowed == surface) {
        return Err("yielded_surface_guard_mismatch");
    }
    execute_wait_any(provider_payload, function_name, yield_time_ms, max_tokens)
}

fn execute_wait_any(
    provider_payload: &Value,
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let output =
        immediate_function_output(provider_payload).ok_or("immediate_tool_output_missing")?;
    let cell_id = yielded_cell_id(output)?;
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": {
            "cell_id": cell_id,
            "yield_time_ms": yield_time_ms,
            "max_tokens": max_tokens,
        }
    }))
    .map_err(|_| "wait_serialization")
}

#[doc(hidden)]
pub fn yielded_cell_id(output: &str) -> Result<&str, &'static str> {
    let tail = output
        .strip_prefix("Script running with cell ID ")
        .ok_or("running_cell_marker_missing")?;
    let cell_id = tail.split_whitespace().next().ok_or("cell_id_missing")?;
    if cell_id.is_empty()
        || !cell_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err("invalid_cell_id");
    }
    Ok(cell_id)
}

fn execute_wait(
    provider_payload: &Value,
    function_name: &str,
    yield_time_ms: u64,
    max_tokens: u64,
) -> Result<String, &'static str> {
    let output =
        immediate_function_output(provider_payload).ok_or("immediate_tool_output_missing")?;
    if !immediate_yielded_build_or_test(provider_payload) {
        return Err("build_or_test_guard_mismatch");
    }
    let cell_id = yielded_cell_id(output)?;
    serde_json::to_string(&serde_json::json!({
        "name": function_name,
        "arguments": {
            "cell_id": cell_id,
            "yield_time_ms": yield_time_ms,
            "max_tokens": max_tokens,
        }
    }))
    .map_err(|_| "wait_serialization")
}

#[doc(hidden)]
pub fn immediate_function_output(payload: &Value) -> Option<&str> {
    let item = payload.get("input")?.as_array()?.last()?;
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    )
    .then(|| item.get("output").and_then(Value::as_str))?
}

#[doc(hidden)]
pub fn immediate_yielded_build_or_test(payload: &Value) -> bool {
    immediate_yielded_source(payload).is_some_and(|source| build_or_test_command(&source))
}

#[doc(hidden)]
pub fn immediate_yielded_surface(payload: &Value) -> Option<&'static str> {
    immediate_yielded_source(payload).map(|source| classify_yielded_surface(&source))
}

fn immediate_yielded_source(payload: &Value) -> Option<String> {
    let items = payload.get("input").and_then(Value::as_array)?;
    let output = items.last()?;
    let call_id = output.get("call_id").and_then(Value::as_str)?;
    items[..items.len().saturating_sub(1)]
        .iter()
        .rev()
        .find(|item| item.get("call_id").and_then(Value::as_str) == Some(call_id))
        .and_then(|item| item.get("arguments").or_else(|| item.get("input")))
        .map(|value| {
            value
                .as_str()
                .map_or_else(|| value.to_string(), str::to_owned)
        })
}

#[doc(hidden)]
pub fn classify_yielded_surface(source: &str) -> &'static str {
    let lower = source.to_ascii_lowercase();
    if build_or_test_command(source) {
        return "build_or_test";
    }
    if lower.contains("nando-live-transition-gate") {
        return "live_transition_gate";
    }
    if lower.contains("systemctl ") || lower.contains("journalctl ") {
        return "service_observation";
    }
    if lower.contains("curl ") || lower.contains("wget ") || lower.contains("ping ") {
        return "network_observation";
    }
    if lower.contains("git ") {
        return "version_control";
    }
    if lower.contains("python") {
        return "python_batch";
    }
    if lower.contains("nando-") {
        return "nando_ops";
    }
    if lower.contains("nginx") {
        return "nginx_ops";
    }
    if lower.contains("sleep ") || lower.contains("timeout ") {
        return "timed_wait";
    }
    if ["install ", "mkdir ", " cp ", " mv ", "chmod ", "chown "]
        .iter()
        .any(|term| lower.contains(term))
    {
        return "filesystem_mutation";
    }
    if ["tar ", "gzip ", "zstd ", "xz "]
        .iter()
        .any(|term| lower.contains(term))
    {
        return "archive_batch";
    }
    if lower.contains("sha256sum") || lower.contains("b2sum") {
        return "checksum_batch";
    }
    if lower.contains("ps ") || lower.contains("ss ") || lower.contains("lsof ") {
        return "process_observation";
    }
    if ["rg ", "find ", "sed ", "jq ", "ls "]
        .iter()
        .any(|tool| lower.contains(tool))
    {
        return "filesystem_observation";
    }
    if lower.contains("&&") || lower.contains(';') || lower.contains("set -") {
        return "shell_batch";
    }
    "generic_long_command"
}

#[doc(hidden)]
pub fn build_or_test_command(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    ((lower.contains("cargo ")
        || lower.contains("pytest")
        || lower.contains("unittest")
        || lower.contains("npm test")
        || lower.contains("pnpm test"))
        && ["test", "build", "check", "clippy", "bench"]
            .iter()
            .any(|term| lower.contains(term)))
        || lower.contains("graphify update")
        || lower.contains("rust-action-memory")
        || (lower.contains("find ") && lower.contains("xargs"))
        || lower.contains("apt-get ")
}

fn execute_copy(
    request_text: &str,
    prefixes: &[String],
    trim: bool,
    allow_multiline: bool,
) -> Result<String, &'static str> {
    let request = request_text.trim_start();
    let lower = request.to_ascii_lowercase();
    let mut matches = prefixes.iter().filter_map(|prefix| {
        let normalized = prefix.to_ascii_lowercase();
        lower.starts_with(&normalized).then_some(normalized.len())
    });
    let Some(offset) = matches.next() else {
        return Err("prefix_mismatch");
    };
    if matches.next().is_some() {
        return Err("ambiguous_prefix");
    }
    let mut output = request.get(offset..).ok_or("prefix_boundary")?;
    if trim {
        output = output.trim();
    }
    if output.is_empty() {
        return Err("empty_capture");
    }
    if !allow_multiline && (output.contains('\n') || output.contains('\r')) {
        return Err("multiline_capture");
    }
    Ok(output.to_owned())
}

fn execute_test_summary(
    request_text: &str,
    provider_payload: &Value,
    required: &[String],
    forbidden: &[String],
) -> Result<String, &'static str> {
    let request = request_text.to_ascii_lowercase();
    if !required
        .iter()
        .any(|phrase| request.contains(&phrase.to_ascii_lowercase()))
    {
        return Err("test_intent_missing");
    }
    if forbidden
        .iter()
        .any(|term| request.contains(&term.to_ascii_lowercase()))
    {
        return Err("broad_intent");
    }
    let output = latest_function_output_text(provider_payload).ok_or("tool_output_missing")?;
    classify_test_output(&output).map(str::to_owned)
}

#[doc(hidden)]
pub fn latest_function_output_text(payload: &Value) -> Option<String> {
    let output = payload
        .get("input")?
        .as_array()?
        .iter()
        .rev()
        .find(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })?
        .get("output")?;
    if let Some(text) = output.as_str() {
        return Some(text.to_owned());
    }
    output_text_parts(output).ok().map(|parts| parts.join("\n"))
}

#[doc(hidden)]
pub fn classify_test_output(output: &str) -> Result<&'static str, &'static str> {
    let lower = output.to_ascii_lowercase();
    if lower.contains("error[e") || lower.contains("could not compile") {
        Ok("Tests did not run because compilation failed.")
    } else if lower.contains("panicked at") || lower.contains("thread 'main' panicked") {
        Ok("Tests failed with a runtime panic.")
    } else if lower.contains("test result: failed")
        || lower.contains("failures:")
        || lower.contains(" ... failed")
    {
        Ok("Tests failed.")
    } else if lower.contains("test result: ok")
        || (lower.contains("0 failed") && lower.contains("passed"))
        || lower.contains("process exited with code 0")
        || lower.contains("\"exit_code\":0")
    {
        Ok("Validation passed.")
    } else {
        Err("test_result_ambiguous")
    }
}

#[cfg(test)]
#[path = "runtime_scalar_budget_tests.rs"]
mod runtime_scalar_budget_tests;
