use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use nando_core::wave::{phase_margin_to_micro, phase_vector_from_atom_ids};
use serde_json::Value;

use crate::program::{
    CollectionAggregateOperation, CollectionOutputRenderer, CollectionProgramStep,
    CollectionScalarType, CustomToolResultProjection, ProjectStatusMapping, RequestTemplateMarker,
    ResponseAdapterWaveRoute, ResponseArgument, ResponseOperation, ResponseProgram,
    ResponseRenderSegment, ValueProjectionFormat,
};
use crate::runtime::{
    classify_test_output, immediate_function_output, immediate_yielded_build_or_test,
    immediate_yielded_surface, latest_function_output_text, yielded_cell_id,
};
use crate::{AtomValueType, ResponseValueSelector, SemanticRole, VerifierProgram};

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

fn independently_verifier_adapter_phase_atom_ids(
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
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("function", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_verifier_variant(verifier, provider_payload).is_ok() {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        VerifierProgram::CustomToolCallFromRoles { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("custom", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_verifier_variant(verifier, provider_payload).is_ok() {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        VerifierProgram::ProjectSelectedValue { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:project"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        VerifierProgram::ProjectStatus { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:status"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        VerifierProgram::ComposeCollection { steps, .. } => {
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
                atoms.push(crate::stable_atom_id("adapter:executes"));
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

fn independently_adapter_phase_atom_ids(
    program: &ResponseProgram,
    provider_payload: &Value,
) -> Vec<u64> {
    if matches!(
        program.operation,
        ResponseOperation::FunctionCallFromRoles { .. }
            | ResponseOperation::CustomToolCallFromRoles { .. }
    ) {
        return crate::source_neutral_verifier_for_program(program)
            .map(|verifier| {
                independently_call_grounded_routing_atom_ids(&verifier, provider_payload)
            })
            .unwrap_or_default();
    }
    let mut atoms = Vec::new();
    let mut identifiers = Vec::<String>::new();
    match &program.operation {
        ResponseOperation::FunctionCallFromRoles { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("function", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_response_variant(program, provider_payload).is_ok() {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::CustomToolCallFromRoles { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:call"));
            independently_adapter_transport_relation_atom("custom", provider_payload, &mut atoms);
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
            if independently_expected_response_variant(program, provider_payload).is_ok() {
                atoms.push(crate::stable_atom_id("adapter:executes"));
            }
        }
        ResponseOperation::ProjectSelectedValue { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:project"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
        }
        ResponseOperation::ProjectStatus { selector, .. } => {
            atoms.push(crate::stable_atom_id("adapter:operation:status"));
            independently_selector_adapter_atoms(
                selector,
                provider_payload,
                &mut atoms,
                &mut identifiers,
            );
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
                atoms.push(crate::stable_atom_id("adapter:executes"));
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
    let selector_canonical = crate::contracts::canonical_response_value_selector(selector);
    let mut atoms = vec![
        crate::stable_atom_id("relation:tool_kind"),
        crate::stable_atom_id(&format!("completion:{completion}")),
        crate::stable_atom_id(&format!(
            "slot:{}:observation",
            independently_adapter_value_type_name(scalar.value_type)
        )),
        crate::stable_atom_id("relation:unique_slot"),
        crate::stable_atom_id(&format!("selector:{selector_canonical}")),
    ];
    if let Some((shape, tool_kind)) = independently_immediate_observation_metadata(provider_payload)
    {
        atoms.push(crate::stable_atom_id(&format!(
            "observation_call_shape:{shape}"
        )));
        atoms.push(crate::stable_atom_id(&format!("tool_kind:{tool_kind}")));
    }
    atoms.extend(crate::response_pre_action_context_atom_ids(
        provider_payload,
    ));
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
    atoms.push(crate::stable_atom_id(&format!(
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

fn independently_selector_adapter_atoms(
    selector: &ResponseValueSelector,
    provider_payload: &Value,
    atoms: &mut Vec<u64>,
    identifiers: &mut Vec<String>,
) {
    let (family, position, value_type) = match selector {
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
            independently_adapter_value_type_name(*value_type)
        )));
    }
    if let Ok(selected) = independently_select_scalar(provider_payload, selector) {
        atoms.push(crate::stable_atom_id("adapter:executes"));
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
            let verifier = crate::source_neutral_verifier_for_program(program)
                .map_err(ResponseVerificationError)?;
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
        if !(1..=crate::program::MAX_UNIQUE_CONSENSUS_VARIANTS).contains(&variants.len()) {
            return Err(ResponseVerificationError(
                "unique_consensus_verifier_variant_count",
            ));
        }
        let layout = verifier_structural_layout_sha256(provider_payload)?;
        let request_atoms = independently_request_text(provider_payload)
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
            crate::ValueProjectionFormat::PlainText => match selected.value_type {
                crate::AtomValueType::String | crate::AtomValueType::Identifier => {
                    Value::String(computed)
                }
                crate::AtomValueType::Integer | crate::AtomValueType::Boolean => {
                    serde_json::from_str(&computed)
                        .map_err(|_| ResponseVerificationError("projection_type_mismatch"))?
                }
                crate::AtomValueType::Collection => {
                    return Err(ResponseVerificationError("projection_non_scalar"));
                }
            },
            crate::ValueProjectionFormat::CanonicalJson => serde_json::from_str(&computed)
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
    crate::canonical_json_sha256(&verifier_structural_layout(value))
        .map_err(ResponseVerificationError)
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
                        Value::String(crate::sha256_bytes(key.as_bytes())),
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

fn independently_execute_collection(
    provider_payload: &Value,
    steps: &[CollectionProgramStep],
    format: ValueProjectionFormat,
    renderer: &CollectionOutputRenderer,
    max_items: usize,
) -> Result<String, ResponseVerificationError> {
    let has_explicit_source = matches!(
        steps.first(),
        Some(CollectionProgramStep::SelectTurnOutput { .. })
    );
    if steps.is_empty()
        || steps.len().saturating_sub(usize::from(has_explicit_source)) > 8
        || max_items == 0
        || max_items > 4_096
    {
        return Err(ResponseVerificationError("collection_program_budget"));
    }
    if !independently_safe_collection_renderer(renderer) {
        return Err(ResponseVerificationError("collection_renderer_unsafe"));
    }
    let (output, transform_steps) = match steps.first() {
        Some(CollectionProgramStep::SelectTurnOutput { output_ordinal }) => (
            independently_active_turn_output_value(provider_payload, Some(*output_ordinal))?,
            &steps[1..],
        ),
        _ => (independently_latest_tool_output(provider_payload)?, steps),
    };
    let mut current = independently_parse_collection_value(output)?;
    let mut filter_field = None::<String>;
    for step in transform_steps {
        current = match step {
            CollectionProgramStep::SelectTurnOutput { .. } => {
                return Err(ResponseVerificationError(
                    "collection_output_selector_position",
                ));
            }
            CollectionProgramStep::SelectOnlyArrayField => {
                let object = current
                    .as_object()
                    .ok_or(ResponseVerificationError("collection_select_not_object"))?;
                let mut arrays = object.values().filter(|candidate| candidate.is_array());
                let selected = arrays
                    .next()
                    .cloned()
                    .ok_or(ResponseVerificationError("collection_select_missing"))?;
                if arrays.next().is_some() {
                    return Err(ResponseVerificationError("collection_select_ambiguous"));
                }
                selected
            }
            CollectionProgramStep::SelectField { field } => current
                .as_object()
                .and_then(|object| object.get(field))
                .cloned()
                .ok_or(ResponseVerificationError("collection_select_missing"))?,
            CollectionProgramStep::FilterFieldEquals { field, value } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let expected = value.as_json();
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| {
                            row.as_object().and_then(|object| object.get(field)) == Some(&expected)
                        })
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::FilterUniqueFieldEquals { value } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let expected = value.as_json();
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_filter_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.as_object()
                            .is_some_and(|object| object.contains_key(*field))
                    }) && rows.iter().any(|row| row.get(*field) == Some(&expected))
                });
                let field = fields
                    .next()
                    .cloned()
                    .ok_or(ResponseVerificationError("collection_filter_field_missing"))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_filter_field_ambiguous",
                    ));
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
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_filter_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let (field, expected) =
                    independently_request_collection_value(provider_payload, rows, *value_type)?;
                filter_field = Some(field.clone());
                Value::Array(
                    rows.iter()
                        .filter(|row| row.get(&field) == Some(&expected))
                        .cloned()
                        .collect(),
                )
            }
            CollectionProgramStep::ProjectField { field } => {
                if let Some(rows) = current.as_array() {
                    if rows.len() > max_items {
                        return Err(ResponseVerificationError("collection_item_budget"));
                    }
                    Value::Array(
                        rows.iter()
                            .map(|row| {
                                row.as_object()
                                    .and_then(|object| object.get(field))
                                    .cloned()
                                    .ok_or(ResponseVerificationError("collection_project_missing"))
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                } else {
                    current
                        .as_object()
                        .and_then(|object| object.get(field))
                        .cloned()
                        .ok_or(ResponseVerificationError("collection_project_missing"))?
                }
            }
            CollectionProgramStep::ProjectUniqueFieldByType { value_type } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_project_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_project_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter().all(|row| {
                        row.get(*field).is_some_and(|value| {
                            independent_collection_scalar_type(value) == Some(*value_type)
                        })
                    })
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_project_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_project_field_ambiguous",
                    ));
                }
                Value::Array(
                    rows.iter()
                        .map(|row| {
                            row.get(&field)
                                .cloned()
                                .ok_or(ResponseVerificationError("collection_project_missing"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::ProjectOnlyNonFilterField => {
                let excluded = filter_field
                    .as_deref()
                    .ok_or(ResponseVerificationError("collection_filter_role_missing"))?;
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_project_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_project_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    field.as_str() != excluded
                        && rows.iter().all(|row| {
                            row.as_object()
                                .is_some_and(|object| object.contains_key(*field))
                        })
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_project_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_project_field_ambiguous",
                    ));
                }
                Value::Array(
                    rows.iter()
                        .map(|row| {
                            row.get(&field)
                                .cloned()
                                .ok_or(ResponseVerificationError("collection_project_missing"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            }
            CollectionProgramStep::AggregateUniqueIntegerField { operation } => {
                let rows = current
                    .as_array()
                    .ok_or(ResponseVerificationError("collection_aggregate_not_array"))?;
                if rows.is_empty() || rows.len() > max_items {
                    return Err(ResponseVerificationError("collection_item_budget"));
                }
                let first = rows[0].as_object().ok_or(ResponseVerificationError(
                    "collection_aggregate_row_not_object",
                ))?;
                let mut fields = first.keys().filter(|field| {
                    rows.iter()
                        .all(|row| row.get(*field).and_then(Value::as_i64).is_some())
                });
                let field = fields.next().cloned().ok_or(ResponseVerificationError(
                    "collection_aggregate_field_missing",
                ))?;
                if fields.next().is_some() {
                    return Err(ResponseVerificationError(
                        "collection_aggregate_field_ambiguous",
                    ));
                }
                let values =
                    rows.iter()
                        .map(|row| {
                            row.get(&field).and_then(Value::as_i64).ok_or(
                                ResponseVerificationError("collection_aggregate_value_missing"),
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                let aggregate = match operation {
                    CollectionAggregateOperation::Sum => values
                        .into_iter()
                        .try_fold(0_i64, i64::checked_add)
                        .ok_or(ResponseVerificationError("collection_aggregate_overflow"))?,
                    CollectionAggregateOperation::Min => values
                        .into_iter()
                        .min()
                        .ok_or(ResponseVerificationError("collection_aggregate_empty"))?,
                    CollectionAggregateOperation::Max => values
                        .into_iter()
                        .max()
                        .ok_or(ResponseVerificationError("collection_aggregate_empty"))?,
                };
                Value::from(aggregate)
            }
            CollectionProgramStep::Count => {
                let count = current
                    .as_array()
                    .map(Vec::len)
                    .or_else(|| current.as_object().map(serde_json::Map::len))
                    .ok_or(ResponseVerificationError("collection_count_unsupported"))?;
                Value::from(
                    u64::try_from(count)
                        .map_err(|_| ResponseVerificationError("collection_count_overflow"))?,
                )
            }
        };
    }
    let computed = match format {
        ValueProjectionFormat::CanonicalJson => serde_json::to_string(&current)
            .map_err(|_| ResponseVerificationError("collection_serialization")),
        ValueProjectionFormat::PlainText => match current {
            Value::String(text) if !text.contains(['\n', '\r']) => Ok(text),
            Value::String(_) => Err(ResponseVerificationError("collection_multiline")),
            Value::Bool(_) | Value::Number(_) | Value::Null => Ok(current.to_string()),
            Value::Array(_) | Value::Object(_) => Err(ResponseVerificationError(
                "collection_plain_text_non_scalar",
            )),
        },
    }?;
    independently_apply_value_renderer(provider_payload, computed, renderer)
}

fn independently_request_collection_value(
    provider_payload: &Value,
    rows: &[Value],
    value_type: crate::CollectionScalarType,
) -> Result<(String, Value), ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut matches = BTreeMap::<Vec<u8>, (String, Value)>::new();
    for row in rows {
        let object = row.as_object().ok_or(ResponseVerificationError(
            "collection_filter_row_not_object",
        ))?;
        for (field, value) in object {
            if independently_collection_value_type(value) == Some(value_type)
                && independently_request_contains_value(&request, value)
            {
                let key = serde_json::to_vec(&(field, value))
                    .map_err(|_| ResponseVerificationError("collection_request_value_encode"))?;
                matches.insert(key, (field.clone(), value.clone()));
            }
        }
    }
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "collection_request_value_cardinality",
        ));
    }
    matches
        .into_values()
        .next()
        .ok_or(ResponseVerificationError(
            "collection_request_value_missing",
        ))
}

fn independently_request_text(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let input = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError(
            "collection_request_input_missing",
        ))?;
    let mut parts = Vec::new();
    for item in input {
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
    if parts.is_empty() {
        Err(ResponseVerificationError("collection_request_text_missing"))
    } else {
        Ok(parts.join("\n"))
    }
}

fn independently_collection_value_type(value: &Value) -> Option<crate::CollectionScalarType> {
    match value {
        Value::String(_) => Some(crate::CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(crate::CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(crate::CollectionScalarType::Boolean),
        _ => None,
    }
}

fn independently_request_contains_value(request: &str, value: &Value) -> bool {
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

fn independently_apply_output_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    match renderer {
        CollectionOutputRenderer::Direct => Ok(computed),
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            Ok(format!("{prefix}{computed}{suffix}"))
        }
        CollectionOutputRenderer::RenderSequence { .. } => Err(ResponseVerificationError(
            "collection_render_sequence_unsupported",
        )),
        CollectionOutputRenderer::RequestTemplate { marker } => {
            independently_apply_request_template(provider_payload, computed, *marker)
        }
    }
}

fn independently_apply_value_renderer(
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    independently_apply_value_renderer_inner(None, provider_payload, computed, renderer)
}

fn independently_apply_value_renderer_with_request(
    request_text: &str,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    independently_apply_value_renderer_inner(
        Some(request_text),
        provider_payload,
        computed,
        renderer,
    )
}

fn independently_apply_value_renderer_inner(
    request_text: Option<&str>,
    provider_payload: &Value,
    computed: String,
    renderer: &CollectionOutputRenderer,
) -> Result<String, ResponseVerificationError> {
    let CollectionOutputRenderer::RenderSequence { segments } = renderer else {
        return independently_apply_output_renderer(provider_payload, computed, renderer);
    };
    let mut output = String::new();
    for segment in segments {
        match segment {
            ResponseRenderSegment::Static { text } => output.push_str(text),
            ResponseRenderSegment::Primary => output.push_str(&computed),
            ResponseRenderSegment::Selected { selector, format } => {
                let selected = match request_text {
                    Some(request_text) => independently_select_scalar_with_request(
                        request_text,
                        provider_payload,
                        selector,
                    )?,
                    None => independently_select_scalar(provider_payload, selector)?,
                };
                output.push_str(&independently_format_selected_value(&selected, *format)?);
            }
        }
        if output.len() > MAX_VERIFIER_OUTPUT_BYTES {
            return Err(ResponseVerificationError("projection_output_budget"));
        }
    }
    Ok(output)
}

fn independently_apply_request_template(
    provider_payload: &Value,
    computed: String,
    marker: RequestTemplateMarker,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut templates = BTreeMap::<String, ()>::new();
    for delimiter in ['`', '\'', '"'] {
        let parts = request.split(delimiter).collect::<Vec<_>>();
        for value in parts.iter().skip(1).step_by(2) {
            let value = value.trim();
            if !value.is_empty()
                && value.len() <= 512
                && !value.contains(['\n', '\r'])
                && value.matches(marker.token()).count() == 1
            {
                templates.insert(value.to_owned(), ());
            }
        }
    }
    if templates.len() != 1 {
        return Err(ResponseVerificationError("request_template_cardinality"));
    }
    let template = templates
        .into_keys()
        .next()
        .ok_or(ResponseVerificationError("request_template_missing"))?;
    let output = template.replacen(marker.token(), &computed, 1);
    if output.is_empty() || output.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("request_template_output_budget"));
    }
    Ok(output)
}

fn independently_safe_collection_renderer(renderer: &CollectionOutputRenderer) -> bool {
    let (prefix, suffix) = match renderer {
        CollectionOutputRenderer::Direct => return true,
        CollectionOutputRenderer::RenderTemplate { prefix, suffix } => {
            (prefix.clone(), suffix.clone())
        }
        CollectionOutputRenderer::RenderSequence { segments } => {
            let primary_count = segments
                .iter()
                .filter(|segment| matches!(segment, ResponseRenderSegment::Primary))
                .count();
            let selected_count = segments
                .iter()
                .filter(|segment| matches!(segment, ResponseRenderSegment::Selected { .. }))
                .count();
            let dynamic_count = primary_count.saturating_add(selected_count);
            if !(1..=64).contains(&segments.len()) || dynamic_count == 0 || dynamic_count > 16 {
                return false;
            }
            let static_text = segments
                .iter()
                .filter_map(|segment| match segment {
                    ResponseRenderSegment::Static { text } => Some(text.as_str()),
                    ResponseRenderSegment::Primary | ResponseRenderSegment::Selected { .. } => None,
                })
                .collect::<String>();
            (static_text, String::new())
        }
        CollectionOutputRenderer::RequestTemplate { .. } => return true,
    };
    if prefix.len().saturating_add(suffix.len()) > 512 {
        return false;
    }
    let combined = format!("{prefix}{suffix}");
    if combined
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        return false;
    }
    let lower = combined.to_lowercase();
    ![
        "authorization",
        "bearer ",
        "credential",
        "password",
        "passwd",
        "secret",
        "api_key",
        "api-key",
        "apikey",
        "private_key",
        "private-key",
        "privatekey",
        "cookie",
        "token",
        "customer ",
        "client ",
        "phone ",
        "address ",
        "клиент ",
        "телефон ",
        "адрес ",
        "улица ",
        "проспект ",
    ]
    .iter()
    .any(|term| lower.contains(term))
        && !["http://", "https://", "www."]
            .iter()
            .any(|term| lower.contains(term))
        && !["/home/", "/etc/", "/var/", "/opt/", "/root/", "/tmp/"]
            .iter()
            .any(|term| lower.contains(term))
        && !independently_contains_email_like(&combined)
        && !independently_contains_windows_path(&combined)
        && !independently_contains_high_entropy_run(&combined)
        && !independently_contains_phone_like(&combined)
        && !(combined.contains('\n') && combined.chars().any(char::is_alphabetic))
}

fn independently_contains_phone_like(value: &str) -> bool {
    let mut digits = 0_usize;
    let mut span = 0_usize;
    for character in value.chars().chain(std::iter::once(' ')) {
        if character.is_ascii_digit() {
            digits = digits.saturating_add(1);
            span = span.saturating_add(1);
        } else if matches!(character, '+' | '-' | '(' | ')' | ' ') && digits > 0 {
            span = span.saturating_add(1);
        } else {
            if digits >= 7 && span <= 24 {
                return true;
            }
            digits = 0;
            span = 0;
        }
    }
    false
}

fn independently_contains_email_like(value: &str) -> bool {
    value.split_whitespace().any(|word| {
        let word = word.trim_matches(|character: char| {
            !character.is_alphanumeric() && !matches!(character, '@' | '.' | '_' | '-' | '+')
        });
        word.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && domain.contains('.') && !domain.ends_with('.')
        })
    })
}

fn independently_contains_windows_path(value: &str) -> bool {
    value.as_bytes().windows(3).any(|window| {
        window[0].is_ascii_alphabetic() && window[1] == b':' && matches!(window[2], b'\\' | b'/')
    }) || value.contains("\\\\")
}

fn independently_contains_high_entropy_run(value: &str) -> bool {
    value
        .split(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '_' | '-'))
        })
        .any(|run| {
            if run.len() < 24 {
                return false;
            }
            let has_lower = run.bytes().any(|byte| byte.is_ascii_lowercase());
            let has_upper = run.bytes().any(|byte| byte.is_ascii_uppercase());
            let has_digit = run.bytes().any(|byte| byte.is_ascii_digit());
            let long_hex = run.len() >= 32 && run.bytes().all(|byte| byte.is_ascii_hexdigit());
            long_hex || (has_lower && has_upper && has_digit)
        })
}

fn independently_parse_collection_value(
    output: &Value,
) -> Result<Value, ResponseVerificationError> {
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
                    return Err(ResponseVerificationError("collection_output_part_type"));
                }
                texts.push(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or(ResponseVerificationError("collection_output_part_text"))?,
                );
            }
        }
        _ => return Err(ResponseVerificationError("collection_output_not_text")),
    }
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    for text in texts {
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or(ResponseVerificationError("collection_input_budget"))?;
        if text.is_empty() || total_bytes > 65_536 {
            return Err(ResponseVerificationError("collection_input_budget"));
        }
        independently_collect_collection_candidates(text, &mut candidates)?;
    }
    if candidates.len() != 1 {
        return Err(ResponseVerificationError(if candidates.is_empty() {
            "collection_input_not_json"
        } else {
            "collection_input_ambiguous"
        }));
    }
    Ok(candidates.into_values().next().expect("one candidate"))
}

fn independently_collect_collection_candidates(
    output: &str,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), ResponseVerificationError> {
    let mut sources = vec![output.to_owned()];
    let mut fenced = None::<String>;
    for line in output.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if let Some(buffer) = fenced.as_mut() {
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
        if source.is_empty() || source.len() > MAX_VERIFIER_OUTPUT_BYTES {
            continue;
        }
        for object in independently_embedded_json_objects(&source) {
            let value = Value::Object(object);
            if independently_bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value)
                    .map_err(|_| ResponseVerificationError("collection_serialization"))?;
                candidates.insert(key, value);
            }
        }
        if let Ok(value @ Value::Array(_)) = serde_json::from_str::<Value>(&source)
            && !independently_is_text_part_array(&value)
        {
            let value = serde_json::json!({"items": value});
            if independently_bounded_collection_root(&value) {
                let key = serde_json::to_vec(&value)
                    .map_err(|_| ResponseVerificationError("collection_serialization"))?;
                candidates.insert(key, value);
            }
        }
    }
    Ok(())
}

fn independently_is_text_part_array(value: &Value) -> bool {
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

fn independently_bounded_collection_root(value: &Value) -> bool {
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
                    independently_safe_collection_identifier(name)
                        && independently_safe_collection_scalar(value)
                })
        })
    })
}

fn independently_safe_collection_identifier(value: &str) -> bool {
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

fn independently_safe_collection_scalar(value: &Value) -> bool {
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

fn independent_collection_scalar_type(value: &Value) -> Option<CollectionScalarType> {
    match value {
        Value::String(_) => Some(CollectionScalarType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            Some(CollectionScalarType::Integer)
        }
        Value::Bool(_) => Some(CollectionScalarType::Boolean),
        Value::Null | Value::Array(_) | Value::Object(_) | Value::Number(_) => None,
    }
}

fn independently_project_status(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
    mapping: ProjectStatusMapping,
) -> Result<&'static str, ResponseVerificationError> {
    let selector_type = match selector {
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
    };
    if selector_type != AtomValueType::Integer {
        return Err(ResponseVerificationError("status_selector_must_be_integer"));
    }
    let selected = independently_select_scalar(provider_payload, selector)?;
    if selected.value_type != AtomValueType::Integer {
        return Err(ResponseVerificationError("status_selector_type_mismatch"));
    }
    let code = selected
        .value
        .as_u64()
        .filter(|code| *code <= MAX_VERIFIER_PROJECT_STATUS_CODE)
        .ok_or(ResponseVerificationError("status_integer_out_of_bounds"))?;
    Ok(match mapping {
        ProjectStatusMapping::ZeroIsSuccess if code == 0 => "success",
        ProjectStatusMapping::ZeroIsSuccess => "failure",
        ProjectStatusMapping::ZeroIsPass if code == 0 => "PASS",
        ProjectStatusMapping::ZeroIsPass => "FAIL",
        ProjectStatusMapping::ZeroIsOk if code == 0 => "OK",
        ProjectStatusMapping::ZeroIsOk => "ERROR",
        ProjectStatusMapping::ZeroIsTrue if code == 0 => "true",
        ProjectStatusMapping::ZeroIsTrue => "false",
    })
}

fn independently_select_scalar(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let request = independently_request_text(provider_payload).unwrap_or_default();
    independently_select_scalar_with_request(&request, provider_payload, selector)
}

fn independently_select_scalar_with_request(
    request_text: &str,
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Result<VerifierScalar, ResponseVerificationError> {
    match selector {
        ResponseValueSelector::UniqueScalar { value_type } => {
            let scalar = independently_unique_scalar(provider_payload)?;
            if scalar.value_type == *value_type {
                Ok(scalar)
            } else {
                Err(ResponseVerificationError("selector_type_mismatch"))
            }
        }
        ResponseValueSelector::UniqueTurnScalar { value_type } => {
            independently_unique_turn_scalar(provider_payload, *value_type)
        }
        ResponseValueSelector::ContentLinePrefix { prefix, value_type } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let parts = independently_bounded_output_text_parts(output)?;
            let mut matches = Vec::new();
            for text in parts {
                for line in text.lines() {
                    let Some(value) = line.trim().strip_prefix(prefix).map(str::trim) else {
                        continue;
                    };
                    if !value.is_empty() {
                        matches.push(independently_parse_scalar_text(value, *value_type)?);
                    }
                }
            }
            if matches.len() != 1 {
                return Err(ResponseVerificationError("selector_prefix_cardinality"));
            }
            matches
                .pop()
                .ok_or(ResponseVerificationError("selector_prefix_missing"))
        }
        ResponseValueSelector::JsonField { field, value_type } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let parts = independently_bounded_output_text_parts(output)?;
            let mut matches = Vec::new();
            for text in parts {
                for object in independently_embedded_json_objects(text) {
                    independently_collect_json_field(
                        &Value::Object(object),
                        field,
                        *value_type,
                        0,
                        &mut matches,
                    )?;
                }
            }
            matches.sort_by_cached_key(|item| item.value.to_string());
            matches.dedup();
            if matches.len() != 1 {
                return Err(ResponseVerificationError("selector_field_cardinality"));
            }
            matches
                .pop()
                .ok_or(ResponseVerificationError("selector_field_missing"))
        }
        ResponseValueSelector::JsonScalarOrdinal {
            ordinal,
            value_type,
        } => {
            let output = independently_latest_tool_output(provider_payload)?;
            let mut matches = Vec::new();
            for text in independently_bounded_output_text_parts(output)? {
                for object in independently_embedded_json_objects(text) {
                    independently_collect_json_scalars(
                        &Value::Object(object),
                        *value_type,
                        0,
                        &mut matches,
                    )?;
                }
            }
            matches
                .into_iter()
                .nth(usize::from(*ordinal))
                .ok_or(ResponseVerificationError("selector_scalar_ordinal_missing"))
        }
        ResponseValueSelector::UniqueTurnJsonField { field, value_type } => {
            independently_unique_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::UniqueActiveTurnJsonField { field, value_type } => {
            independently_unique_active_turn_json_field(provider_payload, field, *value_type)
        }
        ResponseValueSelector::RequestReferencedJsonField { value_type } => {
            independently_request_referenced_json_field(request_text, provider_payload, *value_type)
        }
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
            ordinal,
            value_type,
        } => independently_request_referenced_json_field_ordinal(
            request_text,
            provider_payload,
            *ordinal,
            *value_type,
        ),
        ResponseValueSelector::TurnOutputLine {
            output_ordinal,
            line_index,
            value_type,
        } => independently_turn_output_line(
            provider_payload,
            *output_ordinal,
            *line_index,
            *value_type,
        ),
        ResponseValueSelector::TurnOutputScalarOrdinal {
            output_ordinal,
            scalar_ordinal,
            value_type,
        } => independently_turn_output_scalar_ordinal(
            provider_payload,
            *output_ordinal,
            *scalar_ordinal,
            *value_type,
        ),
        ResponseValueSelector::LatestTurnOutputLine {
            line_index,
            value_type,
        } => independently_latest_turn_output_line(provider_payload, *line_index, *value_type),
        ResponseValueSelector::LatestTurnOutputScalarOrdinal {
            scalar_ordinal,
            value_type,
        } => independently_latest_turn_output_scalar_ordinal(
            provider_payload,
            *scalar_ordinal,
            *value_type,
        ),
        ResponseValueSelector::LatestTurnOutputScalarFromEnd {
            reverse_ordinal,
            value_type,
        } => independently_latest_turn_output_scalar_from_end(
            provider_payload,
            *reverse_ordinal,
            *value_type,
        ),
        ResponseValueSelector::CommandOutputBody => Ok(VerifierScalar {
            value: Value::String(independently_command_output_body(provider_payload)?),
            value_type: AtomValueType::String,
        }),
        ResponseValueSelector::RequestLastToken => Ok(VerifierScalar {
            value: Value::String(independently_request_last_token(provider_payload)?),
            value_type: AtomValueType::String,
        }),
        ResponseValueSelector::RequestUniqueLiteral => Ok(VerifierScalar {
            value: Value::String(independently_request_unique_literal(provider_payload)?),
            value_type: AtomValueType::String,
        }),
    }
}

fn independently_unique_turn_scalar(
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut matches = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        for text in independently_bounded_output_text_parts(output)? {
            let mut parsed_values = Vec::new();
            if let Ok(value) = serde_json::from_str::<Value>(text) {
                parsed_values.push(value);
            } else {
                parsed_values.extend(
                    independently_embedded_json_objects(text)
                        .into_iter()
                        .map(Value::Object),
                );
            }
            for value in parsed_values {
                independently_collect_json_scalars(&value, value_type, 0, &mut matches)?;
                if matches.len() > MAX_VERIFIER_SCALARS {
                    return Err(ResponseVerificationError(
                        "selector_turn_scalar_structure_budget",
                    ));
                }
            }
        }
    }
    matches.sort_by_cached_key(|scalar| scalar.value.to_string());
    matches.dedup();
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_turn_scalar_cardinality",
        ));
    }
    matches
        .pop()
        .ok_or(ResponseVerificationError("selector_turn_scalar_missing"))
}

fn independently_collect_json_scalars(
    value: &Value,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError(
            "selector_scalar_ordinal_structure_budget",
        ));
    }
    if let Ok(scalar) = independently_typed_scalar(value.clone(), value_type) {
        output.push(scalar);
        return Ok(());
    }
    match value {
        Value::Object(object) => {
            for value in object.values() {
                independently_collect_json_scalars(
                    value,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_json_scalars(
                    value,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn independently_collect_json_field(
    value: &Value,
    field: &str,
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError("selector_field_structure_budget"));
    }
    match value {
        Value::Object(object) => {
            for (name, value) in object {
                if name == field {
                    output.push(independently_typed_scalar(value.clone(), value_type)?);
                }
                independently_collect_json_field(
                    value,
                    field,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_json_field(
                    value,
                    field,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn independently_request_referenced_json_field(
    request: &str,
    provider_payload: &Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if request.is_empty() {
        return Err(ResponseVerificationError("selector_request_text_missing"));
    }
    let request_tokens = independently_identifier_tokens(request);
    let output = independently_latest_tool_output(provider_payload)?;
    let mut matches = Vec::<(String, VerifierScalar)>::new();
    for text in independently_bounded_output_text_parts(output)? {
        for object in independently_embedded_json_objects(text) {
            independently_collect_request_referenced_fields(
                &Value::Object(object),
                &request_tokens,
                value_type,
                0,
                &mut matches,
            )?;
        }
    }
    matches.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.value.to_string().cmp(&right.1.value.to_string()))
    });
    matches.dedup();
    if matches.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_request_field_cardinality",
        ));
    }
    matches
        .pop()
        .map(|(_, scalar)| scalar)
        .ok_or(ResponseVerificationError("selector_request_field_missing"))
}

fn independently_request_referenced_json_field_ordinal(
    request: &str,
    provider_payload: &Value,
    ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if request.is_empty() {
        return Err(ResponseVerificationError("selector_request_text_missing"));
    }
    let request_tokens = independently_identifier_tokens(request);
    let output = independently_latest_tool_output(provider_payload)?;
    let mut matches = Vec::<(String, VerifierScalar)>::new();
    for text in independently_bounded_output_text_parts(output)? {
        for object in independently_embedded_json_objects(text) {
            independently_collect_request_referenced_fields(
                &Value::Object(object),
                &request_tokens,
                value_type,
                0,
                &mut matches,
            )?;
        }
    }
    matches.sort_by(|left, right| {
        independently_request_identifier_position(&request_tokens, &left.0)
            .cmp(&independently_request_identifier_position(
                &request_tokens,
                &right.0,
            ))
            .then_with(|| left.0.cmp(&right.0))
            .then_with(|| left.1.value.to_string().cmp(&right.1.value.to_string()))
    });
    matches.dedup();
    matches
        .into_iter()
        .nth(usize::from(ordinal))
        .map(|(_, scalar)| scalar)
        .ok_or(ResponseVerificationError(
            "selector_request_field_ordinal_missing",
        ))
}

fn independently_request_identifier_position(
    request_tokens: &[String],
    identifier: &str,
) -> Option<usize> {
    let identifier_tokens = independently_identifier_tokens(identifier);
    (!identifier_tokens.is_empty())
        .then(|| {
            request_tokens
                .windows(identifier_tokens.len())
                .position(|window| window == identifier_tokens)
        })
        .flatten()
}

fn independently_collect_request_referenced_fields(
    value: &Value,
    request_tokens: &[String],
    value_type: AtomValueType,
    depth: usize,
    output: &mut Vec<(String, VerifierScalar)>,
) -> Result<(), ResponseVerificationError> {
    if depth > 8 || output.len() >= 64 {
        return Err(ResponseVerificationError(
            "selector_request_field_structure_budget",
        ));
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                if independently_request_mentions_identifier(request_tokens, field)
                    && let Ok(scalar) = independently_typed_scalar(value.clone(), value_type)
                {
                    output.push((field.clone(), scalar));
                }
                independently_collect_request_referenced_fields(
                    value,
                    request_tokens,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_request_referenced_fields(
                    value,
                    request_tokens,
                    value_type,
                    depth.saturating_add(1),
                    output,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn independently_identifier_tokens(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_lowercase)
        .take(256)
        .collect()
}

fn independently_request_mentions_identifier(request_tokens: &[String], identifier: &str) -> bool {
    let identifier_tokens = independently_identifier_tokens(identifier);
    !identifier_tokens.is_empty()
        && request_tokens
            .windows(identifier_tokens.len())
            .any(|window| window == identifier_tokens)
}

fn independently_request_unique_literal(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let mut values = BTreeMap::<String, ()>::new();
    for delimiter in ['`', '\'', '"'] {
        let parts = request.split(delimiter).collect::<Vec<_>>();
        for value in parts.iter().skip(1).step_by(2) {
            let value = value.trim();
            if !value.is_empty() && value.len() <= 128 && !value.contains(['\n', '\r']) {
                values.insert(value.to_owned(), ());
            }
        }
    }
    if values.is_empty() {
        for value in request.split(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '-'
        }) {
            if (3..=128).contains(&value.len())
                && value
                    .chars()
                    .any(|character| character.is_ascii_uppercase())
                && value.chars().all(|character| {
                    character.is_ascii_uppercase()
                        || character.is_ascii_digit()
                        || matches!(character, '_' | '-')
                })
            {
                values.insert(value.to_owned(), ());
            }
        }
    }
    if values.len() != 1 {
        return Err(ResponseVerificationError(
            "request_unique_literal_cardinality",
        ));
    }
    values
        .into_keys()
        .next()
        .ok_or(ResponseVerificationError("request_unique_literal_missing"))
}

fn independently_request_last_token(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let request = independently_request_text(provider_payload)?;
    let token = request
        .split_whitespace()
        .next_back()
        .map(|value| {
            value.trim_matches(|character: char| {
                !character.is_alphanumeric() && character != '_' && character != '-'
            })
        })
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or(ResponseVerificationError("request_last_token_missing"))?;
    Ok(token.to_owned())
}

fn independently_command_output_body(
    provider_payload: &Value,
) -> Result<String, ResponseVerificationError> {
    let output = independently_latest_tool_output(provider_payload)?;
    let parts = independently_command_output_text_parts(output)?;
    let mut body = String::new();
    let mut after_marker = false;
    for part in parts {
        if !after_marker {
            let Some((_, suffix)) = part.split_once("\nOutput:\n") else {
                continue;
            };
            after_marker = true;
            body.push_str(suffix);
        } else {
            body.push_str(&part);
        }
        if body.len() > MAX_VERIFIER_OUTPUT_BYTES {
            return Err(ResponseVerificationError("command_output_body_budget"));
        }
    }
    let body = body.trim_end_matches(['\n', '\r']).to_owned();
    if !after_marker || body.is_empty() {
        Err(ResponseVerificationError("command_output_body_missing"))
    } else {
        Ok(body)
    }
}

fn independently_command_output_text_parts(
    output: &Value,
) -> Result<Vec<String>, ResponseVerificationError> {
    if let Some(text) = output.as_str()
        && let Ok(parsed) = serde_json::from_str::<Value>(text)
        && parsed.is_array()
    {
        return independently_bounded_output_text_parts(&parsed)
            .map(|parts| parts.into_iter().map(str::to_owned).collect());
    }
    independently_bounded_output_text_parts(output)
        .map(|parts| parts.into_iter().map(str::to_owned).collect())
}

fn independently_embedded_json_objects(text: &str) -> Vec<serde_json::Map<String, Value>> {
    independently_embedded_json_objects_at_depth(text, 0)
}

fn independently_embedded_json_objects_at_depth(
    text: &str,
    depth: usize,
) -> Vec<serde_json::Map<String, Value>> {
    if depth > 4 {
        return Vec::new();
    }
    let mut sources = vec![text.trim().to_owned()];
    if let Some((_, output)) = text.rsplit_once("\nOutput:\n") {
        sources.push(output.trim().to_owned());
    }
    let mut fence = None::<String>;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "```" {
            if let Some(value) = fence.take() {
                sources.push(value);
            }
        } else if trimmed.eq_ignore_ascii_case("```json") {
            fence = Some(String::new());
        } else if let Some(value) = &mut fence {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
        }
    }
    let mut candidates = BTreeMap::<Vec<u8>, serde_json::Map<String, Value>>::new();
    for source in sources {
        let Ok(value) = serde_json::from_str::<Value>(&source) else {
            continue;
        };
        independently_collect_json_objects(&value, &mut candidates, depth);
    }
    candidates.into_values().collect()
}

fn independently_collect_json_objects(
    value: &Value,
    output: &mut BTreeMap<Vec<u8>, serde_json::Map<String, Value>>,
    depth: usize,
) {
    match value {
        Value::Object(object) => {
            let mut encoded_children = BTreeMap::new();
            if depth < 4 {
                for text in object.values().filter_map(Value::as_str) {
                    for child in independently_embedded_json_objects_at_depth(text, depth + 1) {
                        if let Ok(key) = serde_json::to_vec(&child) {
                            encoded_children.insert(key, child);
                        }
                    }
                }
            }
            if encoded_children.len() == 1 {
                output.extend(encoded_children);
                return;
            }
            if let Ok(key) = serde_json::to_vec(value) {
                output.insert(key, object.clone());
            }
        }
        Value::Array(parts) => {
            for part in parts {
                let Some(text) = part.get("text").and_then(Value::as_str) else {
                    continue;
                };
                for object in independently_embedded_json_objects_at_depth(text, depth + 1) {
                    if let Ok(key) = serde_json::to_vec(&object) {
                        output.insert(key, object);
                    }
                }
            }
        }
        _ => {}
    }
}

fn independently_turn_output_line(
    provider_payload: &Value,
    output_ordinal: u16,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value_type != AtomValueType::String || output_ordinal == 0 {
        return Err(ResponseVerificationError(
            "turn_output_line_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, Some(output_ordinal))?;
    independently_output_line_scalar(output, line_index)
}

fn independently_latest_turn_output_line(
    provider_payload: &Value,
    line_index: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value_type != AtomValueType::String {
        return Err(ResponseVerificationError(
            "latest_turn_output_line_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    independently_output_line_scalar(output, line_index)
}

fn independently_output_line_scalar(
    output: &Value,
    line_index: u16,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let lines = independently_bounded_output_text_parts(output)?
        .into_iter()
        .flat_map(str::lines)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let line = lines
        .get(usize::from(line_index))
        .filter(|line| line.len() <= 512)
        .ok_or(ResponseVerificationError("turn_output_line_missing"))?;
    independently_typed_scalar(Value::String((*line).to_owned()), AtomValueType::String)
}

fn independently_turn_output_scalar_ordinal(
    provider_payload: &Value,
    output_ordinal: u16,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if output_ordinal == 0 || matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "turn_output_scalar_ordinal_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, Some(output_ordinal))?;
    independently_output_scalar_ordinal(output, scalar_ordinal, value_type)
}

fn independently_latest_turn_output_scalar_ordinal(
    provider_payload: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "latest_turn_output_scalar_ordinal_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    independently_output_scalar_ordinal(output, scalar_ordinal, value_type)
}

fn independently_latest_turn_output_scalar_from_end(
    provider_payload: &Value,
    reverse_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if matches!(value_type, AtomValueType::Collection) {
        return Err(ResponseVerificationError(
            "latest_turn_output_scalar_from_end_selector_invalid",
        ));
    }
    let output = independently_active_turn_output_value(provider_payload, None)?;
    let mut scalars = Vec::new();
    for text in independently_bounded_output_text_parts(output)? {
        independently_collect_output_scalars(text, &mut scalars)?;
    }
    scalars
        .into_iter()
        .filter(|scalar| {
            scalar.value_type == value_type
                || matches!(
                    (scalar.value_type, value_type),
                    (AtomValueType::Identifier, AtomValueType::String)
                )
        })
        .rev()
        .nth(usize::from(reverse_ordinal))
        .map(|mut scalar| {
            scalar.value_type = value_type;
            scalar
        })
        .ok_or(ResponseVerificationError(
            "latest_turn_output_scalar_from_end_missing",
        ))
}

fn independently_active_turn_output_value(
    provider_payload: &Value,
    output_ordinal: Option<u16>,
) -> Result<&Value, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut outputs = items[turn_start..].iter().filter(|item| {
        matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        )
    });
    let item = match output_ordinal {
        Some(ordinal) if ordinal > 0 => outputs.nth(usize::from(ordinal - 1)),
        Some(_) => None,
        None => outputs.next_back(),
    }
    .ok_or(ResponseVerificationError("turn_output_ordinal_missing"))?;
    item.get("output")
        .ok_or(ResponseVerificationError("turn_output_missing"))
}

fn independently_output_scalar_ordinal(
    output: &Value,
    scalar_ordinal: u16,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let mut scalars = Vec::new();
    for text in independently_bounded_output_text_parts(output)? {
        independently_collect_output_scalars(text, &mut scalars)?;
    }
    scalars
        .into_iter()
        .filter(|scalar| {
            scalar.value_type == value_type
                || matches!(
                    (scalar.value_type, value_type),
                    (AtomValueType::Identifier, AtomValueType::String)
                )
        })
        .nth(usize::from(scalar_ordinal))
        .map(|mut scalar| {
            scalar.value_type = value_type;
            scalar
        })
        .ok_or(ResponseVerificationError(
            "turn_output_scalar_ordinal_missing",
        ))
}

fn independently_collect_output_scalars(
    text: &str,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if output.len() >= MAX_VERIFIER_SCALARS {
        return Err(ResponseVerificationError(
            "turn_output_scalar_ordinal_budget",
        ));
    }
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return independently_collect_scalars(&value, 0, output);
    }
    let embedded = independently_embedded_json_objects(text);
    if !embedded.is_empty() {
        for object in embedded {
            independently_collect_scalars(&Value::Object(object), 0, output)?;
        }
        return Ok(());
    }
    independently_collect_plain_text_scalars(text, output)
}

fn independently_collect_plain_text_scalars(
    text: &str,
    output: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    for (start, token) in text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .scan(0_usize, |offset, token| {
            let relative = text[*offset..].find(token).unwrap_or(0);
            let start = (*offset).saturating_add(relative);
            *offset = start.saturating_add(token.len());
            Some((start, token))
        })
    {
        if token.is_empty() {
            continue;
        }
        if output.len() >= MAX_VERIFIER_SCALARS {
            return Err(ResponseVerificationError(
                "turn_output_scalar_ordinal_budget",
            ));
        }
        let end = start.saturating_add(token.len());
        let decimal_neighbor = text[..start].ends_with('.') || text[end..].starts_with('.');
        if token.bytes().all(|byte| byte.is_ascii_digit()) && !decimal_neighbor {
            if let Ok(value) = token.parse::<u64>() {
                output.push(VerifierScalar {
                    value: Value::from(value),
                    value_type: AtomValueType::Integer,
                });
            }
        } else if token.eq_ignore_ascii_case("true") || token.eq_ignore_ascii_case("false") {
            output.push(VerifierScalar {
                value: Value::Bool(token.eq_ignore_ascii_case("true")),
                value_type: AtomValueType::Boolean,
            });
        }
    }
    Ok(())
}

fn independently_unique_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut matches = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        for text in independently_bounded_output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            if let Some(value) = object.get(field) {
                let scalar = independently_typed_scalar(value.clone(), value_type)?;
                if !matches.contains(&scalar) {
                    matches.push(scalar);
                }
            }
        }
    }
    if matches.len() != 1 {
        return Err(ResponseVerificationError("selector_turn_field_cardinality"));
    }
    matches
        .pop()
        .ok_or(ResponseVerificationError("selector_turn_field_missing"))
}

fn independently_unique_active_turn_json_field(
    provider_payload: &Value,
    field: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let items = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .ok_or(ResponseVerificationError("turn_input_missing"))?;
    let turn_start = items
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let mut active = Vec::new();
    for item in &items[turn_start..] {
        if !matches!(
            item.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")
        ) {
            continue;
        }
        let Some(output) = item.get("output") else {
            continue;
        };
        let mut observed = Vec::new();
        let mut completed = false;
        for text in independently_bounded_output_text_parts(output)? {
            let Ok(Value::Object(object)) = serde_json::from_str::<Value>(text) else {
                continue;
            };
            completed |= object.get("exit_code").is_some_and(Value::is_i64);
            if let Some(value) = object.get(field) {
                observed.push(independently_typed_scalar(value.clone(), value_type)?);
            }
        }
        if completed && observed.is_empty() && active.len() == 1 {
            active.clear();
        }
        for scalar in observed {
            if !active.contains(&scalar) {
                active.push(scalar);
            }
        }
    }
    if active.len() != 1 {
        return Err(ResponseVerificationError(
            "selector_active_turn_field_cardinality",
        ));
    }
    active.pop().ok_or(ResponseVerificationError(
        "selector_active_turn_field_missing",
    ))
}

fn independently_latest_tool_output(
    provider_payload: &Value,
) -> Result<&Value, ResponseVerificationError> {
    let item = provider_payload
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.last())
        .ok_or(ResponseVerificationError("immediate_tool_output_missing"))?;
    if !matches!(
        item.get("type").and_then(Value::as_str),
        Some("function_call_output" | "custom_tool_call_output")
    ) {
        return Err(ResponseVerificationError("stale_tool_output"));
    }
    item.get("output")
        .ok_or(ResponseVerificationError("immediate_tool_output_missing"))
}

fn independently_unique_scalar(
    provider_payload: &Value,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let output = independently_latest_tool_output(provider_payload)?
        .as_str()
        .ok_or(ResponseVerificationError("scalar_output_not_text"))?
        .trim();
    if output.is_empty() || output.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    let value = serde_json::from_str::<Value>(output).unwrap_or_else(|_| {
        if output.contains(['\n', '\r']) {
            Value::Null
        } else {
            Value::String(output.to_owned())
        }
    });
    let mut scalars = Vec::new();
    independently_collect_scalars(&value, 0, &mut scalars)?;
    if scalars.len() != 1 {
        return Err(ResponseVerificationError("unique_scalar_cardinality"));
    }
    scalars
        .pop()
        .ok_or(ResponseVerificationError("unique_scalar_missing"))
}

fn independently_bounded_output_text_parts(
    output: &Value,
) -> Result<Vec<&str>, ResponseVerificationError> {
    let parts = if let Some(text) = output.as_str() {
        vec![text]
    } else if let Some(items) = output.as_array() {
        if items.is_empty() || items.len() > MAX_VERIFIER_SCALARS {
            return Err(ResponseVerificationError("output_part_cardinality"));
        }
        let mut parts = Vec::with_capacity(items.len());
        for item in items {
            if !matches!(
                item.get("type").and_then(Value::as_str),
                Some("text" | "input_text" | "output_text")
            ) {
                return Err(ResponseVerificationError("unsupported_output_part_type"));
            }
            parts.push(
                item.get("text")
                    .and_then(Value::as_str)
                    .ok_or(ResponseVerificationError("output_part_text_missing"))?,
            );
        }
        parts
    } else {
        return Err(ResponseVerificationError("tool_output_not_text"));
    };
    let total_bytes = parts
        .iter()
        .try_fold(0_usize, |total, part| total.checked_add(part.len()))
        .ok_or(ResponseVerificationError("scalar_output_budget"))?;
    if total_bytes == 0 || total_bytes > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    Ok(parts)
}

fn independently_parse_scalar_text(
    value: &str,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    if value.len() > MAX_VERIFIER_OUTPUT_BYTES {
        return Err(ResponseVerificationError("scalar_output_budget"));
    }
    let parsed = match value_type {
        AtomValueType::Integer => value
            .parse::<u64>()
            .map(Value::from)
            .map_err(|_| ResponseVerificationError("selector_integer_parse"))?,
        AtomValueType::Boolean => value
            .parse::<bool>()
            .map(Value::from)
            .map_err(|_| ResponseVerificationError("selector_boolean_parse"))?,
        AtomValueType::String | AtomValueType::Identifier => Value::String(value.to_owned()),
        AtomValueType::Collection => {
            return Err(ResponseVerificationError("selector_collection_unsupported"));
        }
    };
    independently_typed_scalar(parsed, value_type)
}

fn independently_typed_scalar(
    value: Value,
    value_type: AtomValueType,
) -> Result<VerifierScalar, ResponseVerificationError> {
    let actual = independently_scalar_type(&value)?;
    let compatible = actual == value_type
        || matches!(
            (actual, value_type),
            (AtomValueType::Identifier, AtomValueType::String)
        );
    if !compatible {
        return Err(ResponseVerificationError("selector_type_mismatch"));
    }
    Ok(VerifierScalar { value, value_type })
}

fn independently_collect_scalars(
    value: &Value,
    depth: usize,
    scalars: &mut Vec<VerifierScalar>,
) -> Result<(), ResponseVerificationError> {
    if depth > MAX_VERIFIER_DEPTH || scalars.len() >= MAX_VERIFIER_SCALARS {
        return Err(ResponseVerificationError("scalar_structure_budget"));
    }
    match value {
        Value::Null => {}
        Value::Bool(_) | Value::String(_) => scalars.push(VerifierScalar {
            value: value.clone(),
            value_type: independently_scalar_type(value)?,
        }),
        Value::Number(number) if number.is_i64() || number.is_u64() => {
            scalars.push(VerifierScalar {
                value: value.clone(),
                value_type: AtomValueType::Integer,
            });
        }
        Value::Number(_) => {
            return Err(ResponseVerificationError("unsupported_scalar_number"));
        }
        Value::Array(values) => {
            for value in values {
                independently_collect_scalars(value, depth.saturating_add(1), scalars)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                independently_collect_scalars(value, depth.saturating_add(1), scalars)?;
            }
        }
    }
    Ok(())
}

fn independently_scalar_type(value: &Value) -> Result<AtomValueType, ResponseVerificationError> {
    match value {
        Value::Bool(_) => Ok(AtomValueType::Boolean),
        Value::Number(number) if number.is_i64() || number.is_u64() => Ok(AtomValueType::Integer),
        Value::String(text) if independently_identifier_like(text) => Ok(AtomValueType::Identifier),
        Value::String(_) => Ok(AtomValueType::String),
        _ => Err(ResponseVerificationError("selector_scalar_unsupported")),
    }
}

fn independently_identifier_like(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
        })
}

fn independently_format_selected_value(
    selected: &VerifierScalar,
    format: crate::ValueProjectionFormat,
) -> Result<String, ResponseVerificationError> {
    let projected = match format {
        crate::ValueProjectionFormat::PlainText => match &selected.value {
            Value::String(text) if !text.contains(['\n', '\r']) => text.clone(),
            Value::String(_) => return Err(ResponseVerificationError("projection_multiline")),
            Value::Bool(_) | Value::Number(_) => selected.value.to_string(),
            _ => return Err(ResponseVerificationError("projection_non_scalar")),
        },
        crate::ValueProjectionFormat::CanonicalJson => serde_json::to_string(&selected.value)
            .map_err(|_| ResponseVerificationError("projection_serialization"))?,
    };
    if projected.is_empty() || projected.len() > 16_384 {
        return Err(ResponseVerificationError("projection_output_budget"));
    }
    Ok(projected)
}

fn sha256_scalar(value: &Value) -> Result<String, ResponseVerificationError> {
    use sha2::{Digest, Sha256};
    if !matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_)) {
        return Err(ResponseVerificationError("projection_non_scalar"));
    }
    let canonical = serde_json::to_vec(value)
        .map_err(|_| ResponseVerificationError("projection_serialization"))?;
    Ok(format!("{:x}", Sha256::digest(canonical)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AtomValueType, ProjectStatusMapping, ResponseValueSelector, ValueProjectionFormat,
    };

    fn projection_payload() -> Value {
        serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "value=ready"
            }]
        })
    }

    fn projection_verifier(require_unique_value: bool) -> VerifierProgram {
        VerifierProgram::ProjectSelectedValue {
            selector: ResponseValueSelector::ContentLinePrefix {
                prefix: "value=".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "pending".to_owned(),
            require_unique_value,
        }
    }

    #[test]
    fn call_adapter_wave_atoms_have_actor_and_independent_parity() {
        let program = crate::ResponseProgram::function_call_from_roles(
            "wait",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: "cell_id".to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
        );
        let payload = serde_json::json!({
            "input": [
                {
                    "role": "user",
                    "content": [{"type": "input_text", "text": "wait for script"}]
                },
                {
                    "type": "function_call",
                    "name": "exec_command",
                    "call_id": "call-1",
                    "arguments": "{}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "Script running with cell ID handle-1"
                }
            ]
        });
        let actor = crate::runtime::actor_adapter_phase_atom_ids(&program, &payload);
        let independent = independently_adapter_phase_atom_ids(&program, &payload);
        let verifier = crate::source_neutral_verifier_for_program(&program).expect("verifier");
        let compiled = independently_verifier_adapter_phase_atom_ids(&verifier, &payload);

        assert!(!actor.is_empty());
        assert!(actor.contains(&crate::stable_atom_id(
            "observation_call_shape:function_call"
        )));
        assert!(actor.contains(&crate::stable_atom_id("tool_kind:exec_command")));
        assert_eq!(actor, independent);
        assert_eq!(actor, compiled);

        let custom_program = crate::ResponseProgram::custom_tool_call_from_roles(
            "exec",
            "write_stdin",
            ResponseValueSelector::ContentLinePrefix {
                prefix: "Script running with cell ID ".to_owned(),
                value_type: AtomValueType::Identifier,
            },
            vec![crate::ResponseArgument::Role {
                name: "session_id".to_owned(),
                role: crate::SemanticRole::ContinuationHandle,
                value_type: Some(AtomValueType::Identifier),
            }],
            crate::CustomToolResultProjection::JsonStringifyResult,
        );
        let custom_actor = crate::runtime::actor_adapter_phase_atom_ids(&custom_program, &payload);
        let custom_independent = independently_adapter_phase_atom_ids(&custom_program, &payload);
        let custom_verifier =
            crate::source_neutral_verifier_for_program(&custom_program).expect("custom verifier");
        let custom_compiled =
            independently_verifier_adapter_phase_atom_ids(&custom_verifier, &payload);

        assert_eq!(custom_actor, custom_independent);
        assert_eq!(custom_actor, custom_compiled);
    }

    #[test]
    fn independent_value_projection_verifier_recomputes_and_rejects_mutation() {
        let verifier = projection_verifier(true);
        assert!(verify_response_independently(&verifier, &projection_payload(), "ready").is_ok());
        assert!(
            verify_response_independently(&verifier, &projection_payload(), "changed").is_err()
        );
    }

    #[test]
    fn independent_value_projection_verifier_requires_uniqueness_contract() {
        assert!(
            verify_response_independently(
                &projection_verifier(false),
                &projection_payload(),
                "ready",
            )
            .is_err()
        );
    }

    #[test]
    fn independent_value_projection_verifier_rejects_private_renderer() {
        let mut verifier = projection_verifier(true);
        if let VerifierProgram::ProjectSelectedValue { renderer, .. } = &mut verifier {
            *renderer = CollectionOutputRenderer::RenderTemplate {
                prefix: "Customer Acme secret: ".to_owned(),
                suffix: String::new(),
            };
        }
        assert_eq!(
            verify_response_independently(
                &verifier,
                &projection_payload(),
                "Customer Acme secret: ready",
            ),
            Err(ResponseVerificationError("value_renderer_unsafe"))
        );
    }

    #[test]
    fn actor_side_first_scalar_mutation_cannot_authorize_ambiguous_evidence() {
        let verifier = VerifierProgram::ProjectSelectedValue {
            selector: ResponseValueSelector::UniqueScalar {
                value_type: AtomValueType::String,
            },
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        };
        let ambiguous = serde_json::json!({
            "input": [{
                "type": "function_call_output",
                "output": "{\"first\":\"actor-selected\",\"second\":\"other\"}"
            }]
        });
        assert_eq!(
            verify_response_independently(&verifier, &ambiguous, "actor-selected"),
            Err(ResponseVerificationError("unique_scalar_cardinality"))
        );
    }

    fn status_verifier(selector: ResponseValueSelector) -> VerifierProgram {
        VerifierProgram::ProjectStatus {
            selector,
            mapping: ProjectStatusMapping::ZeroIsSuccess,
            renderer: CollectionOutputRenderer::Direct,
            completion_state: "completed".to_owned(),
            require_unique_value: true,
        }
    }

    #[test]
    fn independent_status_verifier_reparses_and_recomputes_exact_output() {
        let verifier = status_verifier(ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let success = serde_json::json!({
            "input": [{"type":"function_call_output","output":"{\"exit_code\":0}"}]
        });
        let failure = serde_json::json!({
            "input": [{"type":"function_call_output","output":"{\"exit_code\":23}"}]
        });
        assert!(verify_response_independently(&verifier, &success, "success").is_ok());
        assert!(verify_response_independently(&verifier, &failure, "failure").is_ok());
        assert_eq!(
            verify_response_independently(&verifier, &success, "failure"),
            Err(ResponseVerificationError("response_mismatch"))
        );
        assert_eq!(
            verify_response_independently(&verifier, &failure, "success"),
            Err(ResponseVerificationError("response_mismatch"))
        );
    }

    #[test]
    fn independent_status_verifier_rejects_ambiguity_staleness_type_and_bounds() {
        let verifier = status_verifier(ResponseValueSelector::ContentLinePrefix {
            prefix: "exit_code=".to_owned(),
            value_type: AtomValueType::Integer,
        });
        for payload in [
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=0\nexit_code=1"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=-1"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=1000001"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=true"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=success"}]}),
            serde_json::json!({"input":[{"type":"function_call_output","output":"exit_code=0"},{"type":"message","role":"user","content":"new turn"}]}),
        ] {
            assert!(verify_response_independently(&verifier, &payload, "success").is_err());
        }
    }

    #[test]
    fn independent_status_verifier_rejects_unknown_content_part_types() {
        let verifier = status_verifier(ResponseValueSelector::JsonField {
            field: "exit_code".to_owned(),
            value_type: AtomValueType::Integer,
        });
        let payload = serde_json::json!({
            "input":[{
                "type":"function_call_output",
                "output":[{"type":"future_text","text":"{\"exit_code\":0}"}]
            }]
        });
        assert_eq!(
            verify_response_independently(&verifier, &payload, "success"),
            Err(ResponseVerificationError("unsupported_output_part_type"))
        );
    }

    #[test]
    fn turn_unique_selector_has_actor_verifier_parity_and_abstains_on_concurrency() {
        let selector = ResponseValueSelector::UniqueTurnJsonField {
            field: "status".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let verifier = status_verifier(selector);
        let single = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":0}"}
            ]
        });
        let concurrent = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let execution = crate::execute_response(&program, "", &single);
        assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("success"));
        assert!(verify_response_independently(&verifier, &single, "success").is_ok());
        assert_eq!(
            crate::execute_response(&program, "", &concurrent).status,
            crate::ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &concurrent, "success").is_err());
    }

    #[test]
    fn active_turn_selector_releases_completed_single_handle_and_rejects_concurrency() {
        let selector = ResponseValueSelector::UniqueActiveTurnJsonField {
            field: "status".to_owned(),
            value_type: AtomValueType::Integer,
        };
        let program = ResponseProgram::project_status(
            selector.clone(),
            ProjectStatusMapping::ZeroIsSuccess,
            "completed",
        );
        let verifier = status_verifier(selector);
        let sequential = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"exit_code\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let concurrent = serde_json::json!({
            "input": [
                {"type":"message","role":"user","content":"check"},
                {"type":"function_call_output","output":"{\"status\":0}"},
                {"type":"function_call_output","output":"{\"status\":1}"}
            ]
        });
        let execution = crate::execute_response(&program, "", &sequential);
        assert_eq!(execution.status, crate::ResponseExecutionStatus::Executed);
        assert_eq!(execution.response.as_deref(), Some("failure"));
        assert!(verify_response_independently(&verifier, &sequential, "failure").is_ok());
        assert_eq!(
            crate::execute_response(&program, "", &concurrent).status,
            crate::ResponseExecutionStatus::Abstain
        );
        assert!(verify_response_independently(&verifier, &concurrent, "success").is_err());
    }

    #[test]
    fn independent_status_verifier_requires_integer_unique_freshness_contract() {
        let mut verifier = status_verifier(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Integer,
        });
        if let VerifierProgram::ProjectStatus {
            require_unique_value,
            ..
        } = &mut verifier
        {
            *require_unique_value = false;
        }
        assert_eq!(
            verify_response_independently(
                &verifier,
                &serde_json::json!({"input":[{"type":"function_call_output","output":"0"}]}),
                "success",
            ),
            Err(ResponseVerificationError("status_projection_guard_missing"))
        );

        let wrong_type = status_verifier(ResponseValueSelector::UniqueScalar {
            value_type: AtomValueType::Boolean,
        });
        assert_eq!(
            verify_response_independently(
                &wrong_type,
                &serde_json::json!({"input":[{"type":"function_call_output","output":"true"}]}),
                "success",
            ),
            Err(ResponseVerificationError("status_selector_must_be_integer"))
        );
    }

    #[test]
    fn independent_collection_verifier_rejects_private_renderer() {
        let verifier = VerifierProgram::ComposeCollection {
            steps: vec![
                CollectionProgramStep::SelectOnlyArrayField,
                CollectionProgramStep::Count,
            ],
            format: ValueProjectionFormat::PlainText,
            renderer: CollectionOutputRenderer::RenderTemplate {
                prefix: "token=AbCdEfGhIjKlMnOpQrStUv123456 ".to_owned(),
                suffix: String::new(),
            },
            completion_state: "completed".to_owned(),
            max_items: 1_024,
        };
        let payload = serde_json::json!({
            "input":[{"type":"function_call_output","output":"{\"rows\":[{\"value\":1}]}"}]
        });
        assert_eq!(
            verify_response_independently(&verifier, &payload, "1"),
            Err(ResponseVerificationError("collection_renderer_unsafe"))
        );
    }

    #[test]
    fn advance_plan_actor_and_independent_verifier_require_canonical_success() {
        let payload = serde_json::json!({
            "input": [
                {
                    "type": "function_call",
                    "name": "update_plan",
                    "call_id": "plan-1",
                    "arguments": serde_json::json!({
                        "plan": [
                            {"step":"Inspect","status":"completed"},
                            {"step":"Implement","status":"in_progress"},
                            {"step":"Verify","status":"pending"}
                        ]
                    }).to_string()
                },
                {
                    "type": "function_call_output",
                    "call_id": "tool-1",
                    "output": "Chunk ID: plan\nWall time: 0.1 seconds\nProcess exited with code 0\nFinal output:\nverified"
                }
            ]
        });
        let program = ResponseProgram::advance_plan("update_plan");
        let expected = serde_json::json!({
            "name": "update_plan",
            "arguments": {
                "plan": [
                    {"step":"Inspect","status":"completed"},
                    {"step":"Implement","status":"completed"},
                    {"step":"Verify","status":"in_progress"}
                ]
            }
        });
        let execution = crate::execute_response(&program, "", &payload);
        let response = execution.response.expect("actor response");
        assert_eq!(
            serde_json::from_str::<Value>(&response).expect("plan response json"),
            expected
        );
        let verifier = VerifierProgram::AdvancePlan {
            function_name: "update_plan".to_owned(),
            require_explicit_tool_success: true,
            require_canonical_plan: true,
        };
        assert!(verify_response_independently(&verifier, &payload, &response).is_ok());

        let mut failed = payload.clone();
        failed["input"][1]["output"] = Value::String(
            "Chunk ID: plan\nProcess exited with code 1\nFinal output:\nfailed".to_owned(),
        );
        assert!(
            crate::execute_response(&program, "", &failed)
                .response
                .is_none()
        );
        assert!(verify_response_independently(&verifier, &failed, &response).is_err());

        let mut contradictory = payload.clone();
        contradictory["input"][1]["output"] =
            Value::String("Process exited with code 0\nProcess exited with code 1".to_owned());
        assert!(
            crate::execute_response(&program, "", &contradictory)
                .response
                .is_none()
        );
        assert!(verify_response_independently(&verifier, &contradictory, &response).is_err());

        let mut noncanonical = payload;
        noncanonical["input"][0]["arguments"] = Value::String(
            serde_json::json!({
                "plan": [
                    {"step":"Inspect","status":"completed"},
                    {"step":"Implement","status":"pending"},
                    {"step":"Verify","status":"in_progress"}
                ]
            })
            .to_string(),
        );
        assert!(
            crate::execute_response(&program, "", &noncanonical)
                .response
                .is_none()
        );
    }
}
