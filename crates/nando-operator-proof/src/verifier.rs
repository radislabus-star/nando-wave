use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use nando_core::wave::{phase_margin_to_micro, phase_vector_from_atom_ids};
use nando_operator_kernel::{
    AtomValueType, CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, MAX_RESPONSE_RENDER_DYNAMIC_SEGMENTS,
    MAX_RESPONSE_RENDER_SEGMENTS, MAX_RESPONSE_STATIC_TEXT_BYTES, MAX_UNIQUE_CONSENSUS_VARIANTS,
    ProjectStatusMapping, RequestTemplateMarker, ResponseAdapterWaveRoute, ResponseArgument,
    ResponseOperation, ResponseProgram, ResponseRenderSegment, ResponseValueSelector, SemanticRole,
    ValueProjectionFormat, VerifierProgram, canonical_json_sha256,
    canonical_response_value_selector, sha256_bytes,
};
use serde_json::Value;

mod collection;
mod input;
mod selection;

use collection::{
    independently_apply_value_renderer, independently_apply_value_renderer_with_request,
    independently_execute_collection, independently_safe_collection_renderer,
};
use input::independently_request_text;
use selection::{
    independently_embedded_json_objects, independently_format_selected_value,
    independently_identifier_tokens, independently_latest_tool_output,
    independently_project_status, independently_request_mentions_identifier,
    independently_select_scalar, independently_select_scalar_with_request, sha256_scalar,
};

use crate::surface::{
    classify_test_output, immediate_function_output, immediate_yielded_build_or_test,
    immediate_yielded_surface, latest_function_output_text, request_phase_atom_ids,
    response_pre_action_context_atom_ids, stable_atom_id, yielded_cell_id,
};
use crate::verifier_program::source_neutral_verifier_for_program;

const MAX_VERIFIER_OUTPUT_BYTES: usize = 16_384;
const MAX_VERIFIER_SCALARS: usize = 64;
const MAX_VERIFIER_DEPTH: usize = 8;
const MAX_VERIFIER_PROJECT_STATUS_CODE: u64 = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
struct VerifierScalar {
    value: Value,
    value_type: AtomValueType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResponseVerificationError(pub &'static str);

impl fmt::Display for ResponseVerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ResponseVerificationError {}

pub fn verify_response(
    program: &ResponseProgram,
    request_text: &str,
    provider_payload: &Value,
    candidate: &str,
) -> Result<(), ResponseVerificationError> {
    let expected = match &program.operation {
        ResponseOperation::UniqueConsensus {
            variants,
            adapter_wave,
        } => {
            let layout = verifier_structural_layout_sha256(provider_payload)?;
            let request_atoms = independently_request_text(provider_payload)
                .map(|text| request_phase_atom_ids(&text))
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
                        .as_ref()
                        .and_then(|wave| wave.routes.get(index))
                        .and_then(|route| {
                            independently_adapter_wave_margin(
                                &variant.program,
                                provider_payload,
                                route,
                            )
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
                    return Err(ResponseVerificationError("adapter_wave_no_candidate"));
                };
                let tied = applicable
                    .iter()
                    .filter(|value| value.2 == best_margin)
                    .count();
                if tied > usize::from(wave.exact_budget) {
                    return Err(ResponseVerificationError("adapter_wave_tie_budget"));
                }
                applicable.retain(|value| value.2 == best_margin);
            }
            let expected = applicable
                .into_iter()
                .filter_map(|(_, variant, _)| {
                    independently_expected_response_variant(&variant.program, provider_payload).ok()
                })
                .collect::<BTreeSet<_>>();
            if expected.len() != 1 {
                return Err(ResponseVerificationError("unique_consensus_disagreement"));
            }
            expected
                .into_iter()
                .next()
                .ok_or(ResponseVerificationError("unique_consensus_no_match"))?
        }
        ResponseOperation::AdvancePlan { function_name } => {
            let expected = independently_expected_plan_call(provider_payload, function_name)?;
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
        ResponseOperation::FunctionCallFromRoles {
            function_name,
            selector,
            arguments,
        } => {
            let scalar = independently_select_scalar(provider_payload, selector)?;
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
                            verifier_role_value(&scalar.value, *value_type)?,
                        );
                    }
                    ResponseArgument::Role { .. } => {
                        return Err(ResponseVerificationError("unsupported_verifier_role"));
                    }
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
            let expected = serde_json::json!({
                "name": function_name,
                "arguments": projected,
            });
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        } => {
            let expected = expected_custom_tool_call(
                provider_payload,
                custom_tool_name,
                inner_tool_name,
                selector,
                arguments,
                projection,
            )?;
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            ..
        } => {
            let selected =
                independently_select_scalar_with_request(request_text, provider_payload, selector)?;
            let computed = independently_format_selected_value(&selected, *format)?;
            independently_apply_value_renderer_with_request(
                request_text,
                provider_payload,
                computed,
                renderer,
            )?
        }
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            ..
        } => {
            let computed = independently_project_status(provider_payload, selector, *mapping)?;
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)?
        }
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            max_items,
            ..
        } => independently_execute_collection(
            provider_payload,
            steps,
            *format,
            renderer,
            *max_items,
        )?,
        ResponseOperation::CopyAfterPrefix { prefixes, .. } => {
            independently_extract(request_text, prefixes)?.to_owned()
        }
        ResponseOperation::TestResultSummary {
            required_intent_phrases,
            forbidden_intent_terms,
        } => {
            let lower = request_text.to_ascii_lowercase();
            if !required_intent_phrases
                .iter()
                .any(|phrase| lower.contains(&phrase.to_ascii_lowercase()))
            {
                return Err(ResponseVerificationError("test_intent_missing"));
            }
            if forbidden_intent_terms
                .iter()
                .any(|term| lower.contains(&term.to_ascii_lowercase()))
            {
                return Err(ResponseVerificationError("broad_intent"));
            }
            let output = latest_function_output_text(provider_payload)
                .ok_or(ResponseVerificationError("tool_output_missing"))?;
            classify_test_output(&output)
                .map_err(ResponseVerificationError)?
                .to_owned()
        }
        ResponseOperation::WaitOnYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => {
            if !immediate_yielded_build_or_test(provider_payload) {
                return Err(ResponseVerificationError("build_or_test_guard_mismatch"));
            }
            let output = immediate_function_output(provider_payload)
                .ok_or(ResponseVerificationError("immediate_tool_output_missing"))?;
            let cell_id = output
                .strip_prefix("Script running with cell ID ")
                .and_then(|tail| tail.split_whitespace().next())
                .ok_or(ResponseVerificationError("cell_id_missing"))?;
            let expected = serde_json::json!({
                "name": function_name,
                "arguments": {
                    "cell_id": cell_id,
                    "yield_time_ms": yield_time_ms,
                    "max_tokens": max_tokens,
                }
            });
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
        ResponseOperation::WaitOnAnyYieldedCell {
            function_name,
            yield_time_ms,
            max_tokens,
        } => {
            let output = immediate_function_output(provider_payload)
                .ok_or(ResponseVerificationError("immediate_tool_output_missing"))?;
            let cell_id = yielded_cell_id(output).map_err(ResponseVerificationError)?;
            let expected = serde_json::json!({
                "name": function_name,
                "arguments": {
                    "cell_id": cell_id,
                    "yield_time_ms": yield_time_ms,
                    "max_tokens": max_tokens,
                }
            });
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
        ResponseOperation::WaitOnYieldedSurfaces {
            surfaces,
            function_name,
            yield_time_ms,
            max_tokens,
        } => {
            let surface = immediate_yielded_surface(provider_payload)
                .ok_or(ResponseVerificationError("yielded_surface_missing"))?;
            if !surfaces.iter().any(|allowed| allowed == surface) {
                return Err(ResponseVerificationError("yielded_surface_guard_mismatch"));
            }
            let output = immediate_function_output(provider_payload)
                .ok_or(ResponseVerificationError("immediate_tool_output_missing"))?;
            let cell_id = yielded_cell_id(output).map_err(ResponseVerificationError)?;
            let expected = serde_json::json!({
                "name": function_name,
                "arguments": {
                    "cell_id": cell_id,
                    "yield_time_ms": yield_time_ms,
                    "max_tokens": max_tokens,
                }
            });
            return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
                Ok(())
            } else {
                Err(ResponseVerificationError("response_mismatch"))
            };
        }
    };
    if candidate == expected {
        Ok(())
    } else {
        Err(ResponseVerificationError("response_mismatch"))
    }
}

fn independently_adapter_wave_margin(
    program: &ResponseProgram,
    provider_payload: &Value,
    route: &ResponseAdapterWaveRoute,
) -> Option<i64> {
    let atoms = independently_adapter_phase_atom_ids(program, provider_payload);
    independently_adapter_wave_margin_from_atoms(atoms, route)
}

fn independently_verifier_adapter_wave_margin(
    verifier: &VerifierProgram,
    provider_payload: &Value,
    route: &ResponseAdapterWaveRoute,
) -> Option<i64> {
    let atoms = independently_verifier_adapter_phase_atom_ids(verifier, provider_payload);
    independently_adapter_wave_margin_from_atoms(atoms, route)
}

fn independently_adapter_wave_margin_from_atoms(
    atoms: Vec<u64>,
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
        .binary_search(&independently_adapter_wave_atom_fingerprint(&atoms))
        .is_ok();
    if (!route.anchor_atom_ids.is_empty() || !route.positive_fingerprint_ids.is_empty())
        && !anchor_matches
        && !fingerprint_matches
    {
        return None;
    }
    let query = phase_vector_from_atom_ids(atoms, usize::from(route.cells));
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

fn independently_adapter_wave_atom_fingerprint(atoms: &[u64]) -> u64 {
    let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
    for byte in atoms.iter().flat_map(|atom| atom.to_le_bytes()) {
        fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    fingerprint
}

#[doc(hidden)]
pub fn independently_verifier_adapter_phase_atom_ids(
    verifier: &VerifierProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    if matches!(
        verifier,
        VerifierProgram::FunctionCallFromRoles { .. }
            | VerifierProgram::CustomToolCallFromRoles { .. }
    ) {
        return independently_call_grounded_routing_atom_ids(verifier, provider_payload);
    }
    let mut atoms = Vec::new();
    let mut identifiers = Vec::<String>::new();
    match verifier {
        VerifierProgram::FunctionCallFromRoles { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("function", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_verifier_variant(verifier, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        VerifierProgram::CustomToolCallFromRoles { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("custom", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_verifier_variant(verifier, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        VerifierProgram::ProjectSelectedValue { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:project"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        VerifierProgram::ProjectStatus { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:status"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        VerifierProgram::ComposeCollection { steps, .. } => {
            atoms.push(stable_atom_id("adapter:operation:collection"));
            for step in steps {
                match step {
                    CollectionProgramStep::SelectTurnOutput { output_ordinal } => {
                        atoms.push(stable_atom_id(&format!(
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
                    } => independently_selector_adapter_atoms(
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
            if independently_expected_verifier_variant(verifier, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        _ => return Vec::new(),
    }
    let request_tokens = independently_request_text(provider_payload)
        .map(|text| independently_identifier_tokens(&text))
        .unwrap_or_default();
    atoms.extend(independently_adapter_request_lexical_atoms(&request_tokens));
    let mentioned = identifiers
        .iter()
        .filter(|identifier| independently_request_mentions_identifier(&request_tokens, identifier))
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
    atoms.push(stable_atom_id(&format!(
        "adapter:request_identifier_relation:{relation}"
    )));
    atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

#[doc(hidden)]
pub fn independently_adapter_phase_atom_ids(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    if matches!(
        program.operation,
        ResponseOperation::FunctionCallFromRoles { .. }
            | ResponseOperation::CustomToolCallFromRoles { .. }
    ) {
        return source_neutral_verifier_for_program(program)
            .map(|verifier| {
                independently_call_grounded_routing_atom_ids(&verifier, provider_payload)
            })
            .unwrap_or_default();
    }
    let mut atoms = Vec::new();
    let mut identifiers = Vec::<String>::new();
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("function", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_response_variant(program, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("custom", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_response_variant(program, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::ProjectSelectedValue { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:project"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        ResponseOperation::ProjectStatus { selector, .. } => {
            atoms.push(stable_atom_id("adapter:operation:status"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        ResponseOperation::ComposeCollection { steps, .. } => {
            atoms.push(stable_atom_id("adapter:operation:collection"));
            for step in steps {
                match step {
                    CollectionProgramStep::SelectTurnOutput { output_ordinal } => {
                        atoms.push(stable_atom_id(&format!(
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
                    } => independently_selector_adapter_atoms(
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
            if independently_expected_response_variant(program, provider_payload).is_ok() {
                atoms.push(stable_atom_id("adapter:executes"));
            }
        }
        _ => return Vec::new(),
    }
    let request_tokens = independently_request_text(provider_payload)
        .map(|text| independently_identifier_tokens(&text))
        .unwrap_or_default();
    atoms.extend(independently_adapter_request_lexical_atoms(&request_tokens));
    let mentioned = identifiers
        .iter()
        .filter(|identifier| independently_request_mentions_identifier(&request_tokens, identifier))
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
    atoms.push(stable_atom_id(&format!(
        "adapter:request_identifier_relation:{relation}"
    )));
    atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn independently_call_grounded_routing_atom_ids(
    verifier: &VerifierProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    let (selector, completion) = match verifier {
        VerifierProgram::FunctionCallFromRoles {
            selector,
            require_pending_state,
            ..
        } => (
            selector,
            if *require_pending_state {
                "pending"
            } else {
                "completed"
            },
        ),
        VerifierProgram::CustomToolCallFromRoles { selector, .. } => (selector, "pending"),
        _ => return Vec::new(),
    };
    let Ok(scalar) = independently_select_scalar(provider_payload, selector) else {
        return Vec::new();
    };
    let selector_canonical = canonical_response_value_selector(selector);
    let mut atoms = vec![
        stable_atom_id("relation:tool_kind"),
        stable_atom_id(&format!("completion:{completion}")),
        stable_atom_id(&format!(
            "slot:{}:observation",
            independently_adapter_value_type_name(scalar.value_type)
        )),
        stable_atom_id("relation:unique_slot"),
        stable_atom_id(&format!("selector:{selector_canonical}")),
    ];
    if let Some((shape, tool_kind)) = independently_immediate_observation_metadata(provider_payload)
    {
        atoms.push(stable_atom_id(&format!("observation_call_shape:{shape}")));
        atoms.push(stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms.extend(response_pre_action_context_atom_ids(provider_payload));
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

fn independently_immediate_observation_metadata(
    provider_payload: &Value,
) -> Option<(String, String)> {
    let input = provider_payload.get("input")?.as_array()?;
    let (output_index, call_id) = input.iter().enumerate().rev().find_map(|(index, item)| {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            return None;
        }
        item.get("call_id")
            .and_then(Value::as_str)
            .map(|call_id| (index, call_id))
    })?;
    input[..output_index].iter().rev().find_map(|item| {
        let shape = item.get("type").and_then(Value::as_str)?;
        if !matches!(shape, "function_call" | "custom_tool_call")
            || item.get("call_id").and_then(Value::as_str) != Some(call_id)
        {
            return None;
        }
        let tool_kind = item.get("name").and_then(Value::as_str)?;
        Some((shape.to_owned(), tool_kind.to_owned()))
    })
}

fn independently_adapter_transport_relation_atom(
    expected: &str,
    provider_payload: &Value,
    atoms: &mut Vec<u64>,
) {
    let observed = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
        .and_then(|kind| match kind {
            "function_call_output" => Some("function"),
            "custom_tool_call_output" => Some("custom"),
            _ => None,
        });
    let relation = match observed {
        Some(observed) if observed == expected => "match",
        Some(_) => "mismatch",
        None => "missing",
    };
    atoms.push(stable_atom_id(&format!(
        "adapter:transport_relation:{relation}"
    )));
}

fn independently_adapter_request_lexical_atoms(tokens: &[String]) -> Vec<u64> {
    let bounded = tokens
        .iter()
        .filter(|token| !token.is_empty() && token.len() <= 64)
        .take(32)
        .collect::<Vec<_>>();
    let mut atoms = bounded
        .iter()
        .map(|token| stable_atom_id(&format!("adapter:request_unigram:{token}")))
        .collect::<Vec<_>>();
    atoms.extend(bounded.windows(2).map(|window| {
        stable_atom_id(&format!(
            "adapter:request_bigram:{}:{}",
            window[0], window[1]
        ))
    }));
    atoms
}

fn independently_selector_adapter_atoms(
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
    atoms.push(stable_atom_id(&format!("adapter:selector_family:{family}")));
    if let Some(position) = position {
        atoms.push(stable_atom_id(&format!("adapter:position:{position}")));
    }
    if let Some(value_type) = value_type {
        atoms.push(stable_atom_id(&format!(
            "adapter:value_type:{}",
            independently_adapter_value_type_name(*value_type)
        )));
    }
    if let Ok(selected) = independently_select_scalar(provider_payload, selector) {
        atoms.push(stable_atom_id("adapter:executes"));
        if identifiers.is_empty()
            && let Some(identifier) =
                independently_unique_output_key_for_scalar(provider_payload, &selected.value)
        {
            identifiers.push(identifier);
        }
    }
}

fn independently_unique_output_key_for_scalar(
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
            for object in independently_embedded_json_objects(text) {
                independently_collect_scalar_keys(
                    &Value::Object(object),
                    selected,
                    &mut identifiers,
                    0,
                );
            }
        } else {
            independently_collect_scalar_keys(output, selected, &mut identifiers, 0);
        }
        if identifiers.len() > 1 {
            return None;
        }
    }
    (identifiers.len() == 1)
        .then(|| identifiers.into_iter().next())
        .flatten()
}

fn independently_collect_scalar_keys(
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
                independently_collect_scalar_keys(
                    value,
                    selected,
                    identifiers,
                    depth.saturating_add(1),
                );
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_scalar_keys(
                    value,
                    selected,
                    identifiers,
                    depth.saturating_add(1),
                );
            }
        }
        _ => {}
    }
}

const fn independently_adapter_value_type_name(value_type: AtomValueType) -> &'static str {
    match value_type {
        AtomValueType::String => "string",
        AtomValueType::Integer => "integer",
        AtomValueType::Boolean => "boolean",
        AtomValueType::Identifier => "identifier",
        AtomValueType::Collection => "collection",
    }
}

fn independently_expected_response_variant(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    match &program.operation {
        ResponseOperation::ProjectSelectedValue {
            selector,
            format,
            renderer,
            ..
        } => {
            let selected = independently_select_scalar(provider_payload, selector)?;
            let computed = independently_format_selected_value(&selected, *format)?;
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)
        }
        ResponseOperation::ProjectStatus {
            selector,
            mapping,
            renderer,
            ..
        } => {
            let computed = independently_project_status(provider_payload, selector, *mapping)?;
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)
        }
        ResponseOperation::ComposeCollection {
            steps,
            format,
            renderer,
            max_items,
            ..
        } => {
            independently_execute_collection(provider_payload, steps, *format, renderer, *max_items)
        }
        ResponseOperation::FunctionCallFromRoles { .. }
        | ResponseOperation::CustomToolCallFromRoles { .. } => {
            let verifier =
                source_neutral_verifier_for_program(program).map_err(ResponseVerificationError)?;
            independently_expected_verifier_variant(&verifier, provider_payload)
        }
        _ => Err(ResponseVerificationError(
            "unique_consensus_variant_unsupported",
        )),
    }
}

fn independently_expected_verifier_variant(
    verifier: &VerifierProgram,
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    match verifier {
        VerifierProgram::ProjectSelectedValue {
            selector,
            format,
            renderer,
            completion_state,
            require_unique_value,
        } => {
            if !*require_unique_value
                || !matches!(completion_state.as_str(), "pending" | "completed")
            {
                return Err(ResponseVerificationError("value_projection_guard_missing"));
            }
            if !independently_safe_collection_renderer(renderer) {
                return Err(ResponseVerificationError("value_renderer_unsafe"));
            }
            let selected = independently_select_scalar(provider_payload, selector)?;
            let computed = independently_format_selected_value(&selected, *format)?;
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)
        }
        VerifierProgram::ProjectStatus {
            selector,
            mapping,
            renderer,
            completion_state,
            require_unique_value,
        } => {
            if !*require_unique_value
                || !matches!(completion_state.as_str(), "pending" | "completed")
            {
                return Err(ResponseVerificationError("status_projection_guard_missing"));
            }
            if !independently_safe_collection_renderer(renderer) {
                return Err(ResponseVerificationError("status_renderer_unsafe"));
            }
            let computed = independently_project_status(provider_payload, selector, *mapping)?;
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)
        }
        VerifierProgram::ComposeCollection {
            steps,
            format,
            renderer,
            completion_state,
            max_items,
        } => {
            if !matches!(completion_state.as_str(), "pending" | "completed") {
                return Err(ResponseVerificationError(
                    "collection_completion_guard_missing",
                ));
            }
            independently_execute_collection(provider_payload, steps, *format, renderer, *max_items)
        }
        VerifierProgram::FunctionCallFromRoles { .. }
        | VerifierProgram::CustomToolCallFromRoles { .. } => serde_json::to_string(
            &independently_expected_call_value(verifier, provider_payload)?,
        )
        .map_err(|_| ResponseVerificationError("call_response_encode")),
        _ => Err(ResponseVerificationError(
            "unique_consensus_verifier_variant_unsupported",
        )),
    }
}

fn independently_expected_call_value(
    verifier: &VerifierProgram,
    provider_payload: &Value,
) -> Result<Value, ResponseVerificationError> {
    match verifier {
        VerifierProgram::CustomToolCallFromRoles {
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
            require_pending_state,
            require_unique_handle,
        } => {
            if !*require_pending_state || !*require_unique_handle {
                return Err(ResponseVerificationError("custom_tool_guard_missing"));
            }
            expected_custom_tool_call(
                provider_payload,
                custom_tool_name,
                inner_tool_name,
                selector,
                arguments,
                projection,
            )
        }
        VerifierProgram::FunctionCallFromRoles {
            function_name,
            selector,
            role_arguments,
            role_argument_types,
            integer_arguments,
            string_arguments,
            boolean_arguments,
            require_pending_state,
            require_unique_handle,
        } => {
            let scalar = independently_select_scalar(provider_payload, selector)?;
            if *require_pending_state
                && !matches!(
                    selector,
                    ResponseValueSelector::ContentLinePrefix { .. }
                        | ResponseValueSelector::JsonField { .. }
                )
            {
                return Err(ResponseVerificationError("pending_selector_missing"));
            }
            if *require_unique_handle && scalar.value.is_null() {
                return Err(ResponseVerificationError("continuation_handle_missing"));
            }
            let mut arguments = serde_json::Map::new();
            for (name, role) in role_arguments {
                match role {
                    SemanticRole::ContinuationHandle | SemanticRole::SourceValue => {
                        arguments.insert(
                            name.clone(),
                            verifier_role_value(
                                &scalar.value,
                                role_argument_types.get(name).copied(),
                            )?,
                        );
                    }
                    _ => return Err(ResponseVerificationError("unsupported_verifier_role")),
                }
            }
            for (name, value) in integer_arguments {
                arguments.insert(name.clone(), Value::from(*value));
            }
            for (name, value) in string_arguments {
                arguments.insert(name.clone(), Value::String(value.clone()));
            }
            for (name, value) in boolean_arguments {
                arguments.insert(name.clone(), Value::Bool(*value));
            }
            Ok(serde_json::json!({
                "name": function_name,
                "arguments": arguments,
            }))
        }
        _ => Err(ResponseVerificationError("expected_call_program_kind")),
    }
}

pub fn verify_response_independently(
    verifier: &VerifierProgram,
    provider_payload: &Value,
    candidate: &str,
) -> Result<(), ResponseVerificationError> {
    let request = independently_request_text(provider_payload).unwrap_or_default();
    verify_response_independently_with_request(verifier, &request, provider_payload, candidate)
}

pub fn verify_response_independently_with_request(
    verifier: &VerifierProgram,
    request_text: &str,
    provider_payload: &Value,
    candidate: &str,
) -> Result<(), ResponseVerificationError> {
    if let VerifierProgram::UniqueConsensus {
        variants,
        adapter_wave,
    } = verifier
    {
        if !(1..=MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&variants.len()) {
            return Err(ResponseVerificationError(
                "unique_consensus_verifier_variant_count",
            ));
        }
        let layout = verifier_structural_layout_sha256(provider_payload)?;
        let request_atoms = independently_request_text(provider_payload)
            .map(|text| request_phase_atom_ids(&text))
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
                    .as_ref()
                    .and_then(|wave| wave.routes.get(index))
                    .and_then(|route| {
                        independently_verifier_adapter_wave_margin(
                            &variant.verifier,
                            provider_payload,
                            route,
                        )
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
                return Err(ResponseVerificationError("adapter_wave_no_candidate"));
            };
            let tied = applicable
                .iter()
                .filter(|value| value.2 == best_margin)
                .count();
            if tied > usize::from(wave.exact_budget) {
                return Err(ResponseVerificationError("adapter_wave_tie_budget"));
            }
            applicable.retain(|value| value.2 == best_margin);
        }
        let expected = applicable
            .into_iter()
            .filter_map(|(_, variant, _)| {
                independently_expected_verifier_variant(&variant.verifier, provider_payload).ok()
            })
            .collect::<BTreeSet<_>>();
        if expected.len() != 1 {
            return Err(ResponseVerificationError(
                "unique_consensus_verifier_disagreement",
            ));
        }
        return (expected.contains(candidate))
            .then_some(())
            .ok_or(ResponseVerificationError("response_mismatch"));
    }
    if let VerifierProgram::AdvancePlan {
        function_name,
        require_explicit_tool_success,
        require_canonical_plan,
    } = verifier
    {
        if !*require_explicit_tool_success || !*require_canonical_plan {
            return Err(ResponseVerificationError("plan_verifier_guard_missing"));
        }
        let expected = independently_expected_plan_call(provider_payload, function_name)?;
        return (serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected))
            .then_some(())
            .ok_or(ResponseVerificationError("response_mismatch"));
    }
    if let VerifierProgram::ComposeCollection {
        steps,
        format,
        renderer,
        completion_state,
        max_items,
    } = verifier
    {
        if !matches!(completion_state.as_str(), "pending" | "completed") {
            return Err(ResponseVerificationError(
                "collection_completion_guard_missing",
            ));
        }
        let expected = independently_execute_collection(
            provider_payload,
            steps,
            *format,
            renderer,
            *max_items,
        )?;
        return (candidate == expected)
            .then_some(())
            .ok_or(ResponseVerificationError("response_mismatch"));
    }
    if let VerifierProgram::ProjectStatus {
        selector,
        mapping,
        renderer,
        completion_state,
        require_unique_value,
    } = verifier
    {
        if !*require_unique_value || !matches!(completion_state.as_str(), "pending" | "completed") {
            return Err(ResponseVerificationError("status_projection_guard_missing"));
        }
        if !independently_safe_collection_renderer(renderer) {
            return Err(ResponseVerificationError("status_renderer_unsafe"));
        }
        let computed = independently_project_status(provider_payload, selector, *mapping)?;
        let expected =
            independently_apply_value_renderer(provider_payload, computed.to_owned(), renderer)?;
        return if candidate == expected {
            Ok(())
        } else {
            Err(ResponseVerificationError("response_mismatch"))
        };
    }
    if let VerifierProgram::ProjectSelectedValue {
        selector,
        format,
        renderer,
        completion_state,
        require_unique_value,
    } = verifier
    {
        if !*require_unique_value || !matches!(completion_state.as_str(), "pending" | "completed") {
            return Err(ResponseVerificationError("value_projection_guard_missing"));
        }
        if !independently_safe_collection_renderer(renderer) {
            return Err(ResponseVerificationError("value_renderer_unsafe"));
        }
        let selected =
            independently_select_scalar_with_request(request_text, provider_payload, selector)?;
        let computed = independently_format_selected_value(&selected, *format)?;
        let expected = independently_apply_value_renderer_with_request(
            request_text,
            provider_payload,
            computed.clone(),
            renderer,
        )?;
        if candidate != expected {
            return Err(ResponseVerificationError("response_mismatch"));
        }
        if matches!(renderer, CollectionOutputRenderer::RenderSequence { .. }) {
            return Ok(());
        }
        let candidate_value = match format {
            ValueProjectionFormat::PlainText => match selected.value_type {
                AtomValueType::String | AtomValueType::Identifier => Value::String(computed),
                AtomValueType::Integer | AtomValueType::Boolean => serde_json::from_str(&computed)
                    .map_err(|_| ResponseVerificationError("projection_type_mismatch"))?,
                AtomValueType::Collection => {
                    return Err(ResponseVerificationError("projection_non_scalar"));
                }
            },
            ValueProjectionFormat::CanonicalJson => serde_json::from_str(&computed)
                .map_err(|_| ResponseVerificationError("projection_format_mismatch"))?,
        };
        if candidate_value != selected.value {
            return Err(ResponseVerificationError("projection_equality_mismatch"));
        }
        let selected_hash = sha256_scalar(&selected.value)?;
        let candidate_hash = sha256_scalar(&candidate_value)?;
        return (selected_hash == candidate_hash)
            .then_some(())
            .ok_or(ResponseVerificationError("projection_hash_mismatch"));
    }
    if let VerifierProgram::CustomToolCallFromRoles {
        custom_tool_name,
        inner_tool_name,
        selector,
        arguments,
        projection,
        require_pending_state,
        require_unique_handle,
    } = verifier
    {
        if !*require_pending_state || !*require_unique_handle {
            return Err(ResponseVerificationError("custom_tool_guard_missing"));
        }
        let expected = expected_custom_tool_call(
            provider_payload,
            custom_tool_name,
            inner_tool_name,
            selector,
            arguments,
            projection,
        )?;
        return if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
            Ok(())
        } else {
            Err(ResponseVerificationError("response_mismatch"))
        };
    }
    let VerifierProgram::FunctionCallFromRoles {
        function_name,
        selector,
        role_arguments,
        role_argument_types,
        integer_arguments,
        string_arguments,
        boolean_arguments,
        require_pending_state,
        require_unique_handle,
    } = verifier
    else {
        return Err(ResponseVerificationError(
            "unsupported_independent_verifier",
        ));
    };
    let scalar = independently_select_scalar(provider_payload, selector)?;
    if *require_pending_state
        && !matches!(
            selector,
            ResponseValueSelector::ContentLinePrefix { .. }
                | ResponseValueSelector::JsonField { .. }
        )
    {
        return Err(ResponseVerificationError("pending_selector_missing"));
    }
    if *require_unique_handle && scalar.value.is_null() {
        return Err(ResponseVerificationError("continuation_handle_missing"));
    }
    let mut arguments = serde_json::Map::new();
    for (name, role) in role_arguments {
        match role {
            SemanticRole::ContinuationHandle => {
                arguments.insert(
                    name.clone(),
                    verifier_role_value(&scalar.value, role_argument_types.get(name).copied())?,
                );
            }
            SemanticRole::SourceValue => {
                arguments.insert(
                    name.clone(),
                    verifier_role_value(&scalar.value, role_argument_types.get(name).copied())?,
                );
            }
            _ => return Err(ResponseVerificationError("unsupported_verifier_role")),
        }
    }
    for (name, value) in integer_arguments {
        arguments.insert(name.clone(), Value::from(*value));
    }
    for (name, value) in string_arguments {
        arguments.insert(name.clone(), Value::String(value.clone()));
    }
    for (name, value) in boolean_arguments {
        arguments.insert(name.clone(), Value::Bool(*value));
    }
    let expected = serde_json::json!({
        "name": function_name,
        "arguments": arguments,
    });
    if serde_json::from_str::<Value>(candidate).ok().as_ref() == Some(&expected) {
        Ok(())
    } else {
        Err(ResponseVerificationError("response_mismatch"))
    }
}

fn verifier_structural_layout_sha256(value: &Value) -> Result<String, ResponseVerificationError> {
    // Re-extract the immediate observation independently. The verifier does
    // not inherit an actor-selected payload or irrelevant transcript history.
    let output = independently_latest_tool_output(value)?;
    canonical_json_sha256(&verifier_structural_layout(output)).map_err(ResponseVerificationError)
}

fn verifier_structural_layout(value: &Value) -> Value {
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
                |parsed| verifier_structural_layout(&parsed),
            ),
        Value::Array(values) => {
            Value::Array(values.iter().map(verifier_structural_layout).collect())
        }
        Value::Object(values) => {
            let mut shapes = values
                .iter()
                .map(|(key, value)| {
                    Value::Array(vec![
                        Value::String(sha256_bytes(key.as_bytes())),
                        verifier_structural_layout(value),
                    ])
                })
                .collect::<Vec<_>>();
            shapes.sort_by_cached_key(|shape| serde_json::to_vec(shape).unwrap_or_default());
            Value::Array(shapes)
        }
    }
}

fn independently_expected_plan_call(
    provider_payload: &Value,
    function_name: &str,
) -> Result<Value, ResponseVerificationError> {
    let input = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("plan_input_missing"))?;
    let immediate = input
        .last()
        .filter(|item| {
            matches!(
                item.get("type").and_then(Value::as_str),
                Some("function_call_output" | "custom_tool_call_output")
            )
        })
        .and_then(|item| item.get("output"))
        .ok_or(ResponseVerificationError(
            "plan_immediate_tool_output_missing",
        ))?;
    if !verifier_explicit_tool_success(immediate) {
        return Err(ResponseVerificationError("plan_tool_success_missing"));
    }
    let previous = input[..input.len().saturating_sub(1)]
        .iter()
        .rev()
        .find(|item| {
            item.get("type").and_then(Value::as_str) == Some("function_call")
                && item.get("name").and_then(Value::as_str) == Some(function_name)
        })
        .ok_or(ResponseVerificationError("plan_previous_call_missing"))?;
    let raw_arguments = previous
        .get("arguments")
        .ok_or(ResponseVerificationError("plan_previous_arguments_missing"))?;
    let decoded;
    let arguments = match raw_arguments.as_str() {
        Some(raw) => {
            decoded = serde_json::from_str::<Value>(raw)
                .map_err(|_| ResponseVerificationError("plan_previous_arguments_invalid"))?;
            &decoded
        }
        None => raw_arguments,
    };
    let mut plan = arguments
        .get("plan")
        .and_then(Value::as_array)
        .filter(|steps| !steps.is_empty() && steps.len() <= 32)
        .cloned()
        .ok_or(ResponseVerificationError("plan_previous_state_missing"))?;
    let mut active_index = None;
    for (index, value) in plan.iter().enumerate() {
        let step = value
            .as_object()
            .filter(|step| step.len() == 2)
            .ok_or(ResponseVerificationError("plan_step_schema_mismatch"))?;
        let text = step
            .get("step")
            .and_then(Value::as_str)
            .filter(|text| {
                !text.is_empty() && text.len() <= 1_024 && !text.chars().any(char::is_control)
            })
            .ok_or(ResponseVerificationError("plan_step_text_invalid"))?;
        let _ = text;
        let status = step
            .get("status")
            .and_then(Value::as_str)
            .ok_or(ResponseVerificationError("plan_step_status_missing"))?;
        match (active_index, status) {
            (None, "completed") => {}
            (None, "in_progress") => active_index = Some(index),
            (Some(_), "pending") => {}
            _ => return Err(ResponseVerificationError("plan_status_order_invalid")),
        }
    }
    let active_index =
        active_index.ok_or(ResponseVerificationError("plan_active_step_ambiguous"))?;
    plan[active_index]["status"] = Value::String("completed".to_owned());
    if let Some(next) = plan.get_mut(active_index.saturating_add(1)) {
        next["status"] = Value::String("in_progress".to_owned());
    }
    Ok(serde_json::json!({
        "name": function_name,
        "arguments": {"plan": plan},
    }))
}

fn verifier_explicit_tool_success(output: &Value) -> bool {
    if let Some(object) = output.as_object() {
        if object.contains_key("exit_code") {
            return object.get("exit_code").and_then(Value::as_i64) == Some(0);
        }
        if object.contains_key("ok") {
            return object.get("ok").and_then(Value::as_bool) == Some(true);
        }
        if let Some(status) = object.get("status").and_then(Value::as_str) {
            return ["success", "succeeded", "pass", "passed", "ok", "completed"]
                .contains(&status.to_ascii_lowercase().as_str());
        }
        return object
            .get("result")
            .is_some_and(verifier_explicit_tool_success);
    }
    if let Some(text) = output.as_str().filter(|text| text.len() <= 131_072) {
        if let Ok(decoded) = serde_json::from_str::<Value>(text) {
            return verifier_explicit_tool_success(&decoded);
        }
        return ["success", "succeeded", "pass", "passed", "ok", "completed"]
            .contains(&text.trim().to_ascii_lowercase().as_str())
            || verifier_transport_exit_success(text);
    }
    output.as_array().is_some_and(|parts| {
        parts.len() == 1
            && parts[0]
                .get("text")
                .is_some_and(verifier_explicit_tool_success)
    })
}

fn verifier_transport_exit_success(text: &str) -> bool {
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

fn expected_custom_tool_call(
    provider_payload: &Value,
    custom_tool_name: &str,
    inner_tool_name: &str,
    selector: &ResponseValueSelector,
    arguments: &[ResponseArgument],
    projection: &CustomToolResultProjection,
) -> Result<Value, ResponseVerificationError> {
    let scalar = independently_select_scalar(provider_payload, selector)?;
    let arguments = verified_arguments(arguments, &scalar)?;
    let arguments_json =
        serde_json::to_string(&arguments).map_err(|_| ResponseVerificationError("arguments"))?;
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
                .map_err(|_| ResponseVerificationError("prefix"))?;
            format!(
                "const r=await tools.{inner_tool_name}({arguments_json});text(r.{output_field});if(r.{continuation_field})text({prefix}+r.{continuation_field});"
            )
        }
        CustomToolResultProjection::JsonStringifyResult => format!(
            "const r=await tools.{inner_tool_name}({arguments_json});text(JSON.stringify(r));"
        ),
    };
    Ok(serde_json::json!({
        "kind": "custom_tool_call",
        "name": custom_tool_name,
        "input": source,
    }))
}

fn verified_arguments(
    arguments: &[ResponseArgument],
    scalar: &VerifierScalar,
) -> Result<serde_json::Map<String, Value>, ResponseVerificationError> {
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
                    verifier_role_value(&scalar.value, *value_type)?,
                );
            }
            ResponseArgument::Role { .. } => {
                return Err(ResponseVerificationError("unsupported_verifier_role"));
            }
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

fn verifier_role_value(
    value: &Value,
    value_type: Option<AtomValueType>,
) -> Result<Value, ResponseVerificationError> {
    match value_type {
        None => Ok(value.clone()),
        Some(AtomValueType::Integer) => value
            .as_u64()
            .or_else(|| value.as_str()?.parse::<u64>().ok())
            .map(Value::from)
            .ok_or(ResponseVerificationError("role_integer_parse")),
        Some(AtomValueType::Boolean) => value
            .as_bool()
            .or_else(|| value.as_str()?.parse::<bool>().ok())
            .map(Value::from)
            .ok_or(ResponseVerificationError("role_boolean_parse")),
        Some(AtomValueType::String | AtomValueType::Identifier) => value
            .as_str()
            .map(|value| Value::String(value.to_owned()))
            .ok_or(ResponseVerificationError("role_string_parse")),
        Some(AtomValueType::Collection) => {
            Err(ResponseVerificationError("role_collection_unsupported"))
        }
    }
}

fn independently_extract<'a>(
    request_text: &'a str,
    prefixes: &[String],
) -> Result<&'a str, ResponseVerificationError> {
    let request = request_text.trim_start();
    let lower = request.to_ascii_lowercase();
    let offsets = prefixes
        .iter()
        .filter_map(|prefix| {
            let normalized = prefix.to_ascii_lowercase();
            lower.starts_with(&normalized).then_some(normalized.len())
        })
        .collect::<Vec<_>>();
    if offsets.len() != 1 {
        return Err(ResponseVerificationError("prefix_cardinality"));
    }
    let output = request
        .get(offsets[0]..)
        .ok_or(ResponseVerificationError("prefix_boundary"))?
        .trim();
    if output.is_empty() || output.contains('\n') || output.contains('\r') {
        return Err(ResponseVerificationError("invalid_capture"));
    }
    Ok(output)
}
