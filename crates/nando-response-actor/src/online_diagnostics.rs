//! Read-only semantic-law evidence diagnostics.
//!
//! This module can inspect miner state but never owns grouping, execution, or
//! admission decisions.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::StreamingSelfTrainingState;
use crate::runtime::{immediate_tool_output_value, output_text_parts, parse_scalar_text};
use crate::{AtomValueType, ResponseValueSelector};

pub const SEMANTIC_LAW_EVIDENCE_AUDIT_SCHEMA_V1: &str = "nando.semantic-law-evidence-audit.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticLawEvidenceAuditRow {
    pub frame_id_sha256: String,
    pub evidence_ref_sha256: String,
    pub member_signature_sha256: String,
    pub effect_law_sha256: String,
    pub protocol_class: String,
    pub selector_kind: String,
    pub selector_sha256: String,
    pub source_layout_sha256: String,
    pub observed_value_type: Option<crate::AtomValueType>,
    pub emitted_value_types: Vec<crate::AtomValueType>,
    pub argument_schema_sha256: String,
    pub constant_contract_sha256: String,
    pub capability_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticLawActorReplayOutcome {
    pub frame_id_sha256: String,
    pub outcome: String,
    pub reason: String,
    pub selector_candidate_count: Option<usize>,
    pub actual_action_sha256: Option<String>,
    pub expected_action_sha256: String,
    pub actual_arguments_sha256: Option<String>,
    pub expected_arguments_sha256: String,
    pub actual_role_value_sha256: Option<String>,
    pub expected_role_value_sha256: Option<String>,
    pub actual_role_occurrences: Vec<SemanticLawValueOccurrence>,
    pub expected_role_occurrences: Vec<SemanticLawValueOccurrence>,
    pub selector_candidates: Vec<SemanticLawSelectorCandidate>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemanticLawValueOccurrence {
    pub scope: String,
    pub path_sha256: String,
    pub encoding: String,
    pub ordinal: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemanticLawSelectorCandidate {
    pub locator_sha256: String,
    pub value_sha256: String,
    pub value_type: crate::AtomValueType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticLawActorAudit {
    pub actor_program_sha256: String,
    pub origin: String,
    pub member_signatures: Vec<String>,
    pub cohort_ids: Vec<String>,
    pub action_symbols: Vec<String>,
    pub operation_kind: String,
    pub selector_kind: String,
    pub selector_value_type: Option<crate::AtomValueType>,
    pub argument_schema_sha256: String,
    pub constant_contract_sha256: String,
    pub exact_rows: usize,
    pub wrong_rows: usize,
    pub abstain_rows: usize,
    pub verify_failed_rows: usize,
    pub outcomes: Vec<SemanticLawActorReplayOutcome>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticLawEvidenceAudit {
    pub schema: String,
    pub requested_signatures: Vec<String>,
    pub effect_law_sha256: Vec<String>,
    pub member_signatures: Vec<String>,
    pub rows: Vec<SemanticLawEvidenceAuditRow>,
    pub actors: Vec<SemanticLawActorAudit>,
    pub missing_parity_frame_ids: Vec<String>,
}

fn selector_contract(
    selector: &crate::ResponseValueSelector,
) -> (String, Option<crate::AtomValueType>) {
    let value = serde_json::to_value(selector).unwrap_or_default();
    let kind = value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown_selector")
        .to_owned();
    let value_type = value
        .get("value_type")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    (kind, value_type)
}

fn program_contract(
    program: &crate::ResponseProgram,
) -> (String, String, Option<crate::AtomValueType>, String, String) {
    let (operation_kind, selector, arguments) = match &program.operation {
        crate::ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        } => ("function_call_from_roles", Some(selector), Some(arguments)),
        crate::ResponseOperation::CustomToolCallFromRoles {
            selector,
            arguments,
            ..
        } => (
            "custom_tool_call_from_roles",
            Some(selector),
            Some(arguments),
        ),
        crate::ResponseOperation::UniqueConsensus { .. } => ("unique_consensus", None, None),
        _ => ("other", None, None),
    };
    let (selector_kind, selector_value_type) = selector
        .map(selector_contract)
        .unwrap_or_else(|| ("none".to_owned(), None));
    let argument_schema = arguments
        .into_iter()
        .flatten()
        .map(|argument| match argument {
            crate::ResponseArgument::Role {
                name, value_type, ..
            } => serde_json::json!(["role", name, value_type]),
            crate::ResponseArgument::Integer { name, .. } => {
                serde_json::json!(["integer", name])
            }
            crate::ResponseArgument::String { name, .. } => {
                serde_json::json!(["string", name])
            }
            crate::ResponseArgument::Boolean { name, .. } => {
                serde_json::json!(["boolean", name])
            }
        })
        .collect::<Vec<_>>();
    let constants = arguments
        .into_iter()
        .flatten()
        .filter_map(|argument| match argument {
            crate::ResponseArgument::Role { .. } => None,
            crate::ResponseArgument::Integer { name, value } => {
                Some(serde_json::json!(["integer", name, value]))
            }
            crate::ResponseArgument::String { name, value } => {
                Some(serde_json::json!(["string", name, value]))
            }
            crate::ResponseArgument::Boolean { name, value } => {
                Some(serde_json::json!(["boolean", name, value]))
            }
        })
        .collect::<Vec<_>>();
    (
        operation_kind.to_owned(),
        selector_kind,
        selector_value_type,
        crate::sha256_bytes(&serde_json::to_vec(&argument_schema).unwrap_or_default()),
        crate::sha256_bytes(&serde_json::to_vec(&constants).unwrap_or_default()),
    )
}

fn program_selector(program: &crate::ResponseProgram) -> Option<&crate::ResponseValueSelector> {
    match &program.operation {
        crate::ResponseOperation::FunctionCallFromRoles { selector, .. }
        | crate::ResponseOperation::CustomToolCallFromRoles { selector, .. } => Some(selector),
        _ => None,
    }
}

fn frame_protocol_class(frame: &crate::RelationFrame) -> String {
    frame
        .atoms
        .iter()
        .find_map(|atom| match atom {
            crate::RelationAtom::ActionFunction { value } => Some(format!("function:{value}")),
            crate::RelationAtom::ActionCustomTool { value } => Some(format!("custom_tool:{value}")),
            _ => None,
        })
        .unwrap_or_else(|| "unknown_action".to_owned())
}

fn frame_argument_contract(
    frame: &crate::RelationFrame,
) -> (String, String, Vec<crate::AtomValueType>) {
    let mut schema = Vec::new();
    let mut constants = Vec::new();
    let mut emitted_types = Vec::new();
    for atom in &frame.atoms {
        match atom {
            crate::RelationAtom::ActionRoleArgument {
                name, value_type, ..
            } => {
                schema.push(serde_json::json!(["role", name, value_type]));
                if let Some(value_type) = value_type {
                    emitted_types.push(*value_type);
                }
            }
            crate::RelationAtom::ActionIntegerArgument { name, value } => {
                schema.push(serde_json::json!(["integer", name]));
                constants.push(serde_json::json!(["integer", name, value]));
                emitted_types.push(crate::AtomValueType::Integer);
            }
            crate::RelationAtom::ActionStringArgument { name, value } => {
                schema.push(serde_json::json!(["string", name]));
                constants.push(serde_json::json!(["string", name, value]));
                emitted_types.push(crate::AtomValueType::String);
            }
            crate::RelationAtom::ActionBooleanArgument { name, value } => {
                schema.push(serde_json::json!(["boolean", name]));
                constants.push(serde_json::json!(["boolean", name, value]));
                emitted_types.push(crate::AtomValueType::Boolean);
            }
            _ => {}
        }
    }
    emitted_types.sort_unstable();
    emitted_types.dedup();
    (
        crate::sha256_bytes(&serde_json::to_vec(&schema).unwrap_or_default()),
        crate::sha256_bytes(&serde_json::to_vec(&constants).unwrap_or_default()),
        emitted_types,
    )
}

fn semantic_replay_outcome(
    program: &crate::ResponseProgram,
    parity: &crate::RuntimeParityCase,
) -> (String, String) {
    let execution =
        crate::execute_response(program, &parity.request_text, &parity.provider_payload);
    match execution.status {
        crate::ResponseExecutionStatus::Abstain => ("abstain".to_owned(), execution.reason),
        crate::ResponseExecutionStatus::VerifyFailed => {
            ("verify_failed".to_owned(), execution.reason)
        }
        crate::ResponseExecutionStatus::Executed => {
            let Some(actual) = execution.response.as_deref() else {
                return ("wrong".to_owned(), "executed_without_response".to_owned());
            };
            if actual == parity.expected_response {
                return ("exact".to_owned(), "response_exact".to_owned());
            }
            if crate::online_admission::responses_match_after_execution_budget_normalization(
                actual,
                &parity.expected_response,
            ) {
                return (
                    "exact".to_owned(),
                    "response_execution_budget_equivalent".to_owned(),
                );
            }
            let reason = match (
                serde_json::from_str::<serde_json::Value>(actual),
                serde_json::from_str::<serde_json::Value>(&parity.expected_response),
            ) {
                (Ok(actual), Ok(expected)) => {
                    let same_name = actual.get("name") == expected.get("name");
                    let actual_arguments = actual.get("arguments");
                    let expected_arguments = expected.get("arguments");
                    let same_argument_shape = actual_arguments.zip(expected_arguments).is_some_and(
                        |(actual, expected)| json_shape(actual) == json_shape(expected),
                    );
                    match (same_name, same_argument_shape) {
                        (false, false) => "action_name_and_argument_shape_mismatch",
                        (false, true) => "action_name_mismatch",
                        (true, false) => "argument_shape_mismatch",
                        (true, true) => "argument_value_or_type_mismatch",
                    }
                }
                _ => "response_shape_mismatch",
            };
            ("wrong".to_owned(), reason.to_owned())
        }
    }
}

fn replay_action_digests(
    program: &crate::ResponseProgram,
    parity: &crate::RuntimeParityCase,
) -> (Option<String>, String, Option<String>, String) {
    let execution =
        crate::execute_response(program, &parity.request_text, &parity.provider_payload);
    let actual = execution
        .response
        .as_deref()
        .and_then(|response| serde_json::from_str::<serde_json::Value>(response).ok());
    let expected = serde_json::from_str::<serde_json::Value>(&parity.expected_response)
        .unwrap_or(serde_json::Value::Null);
    let digest = |value: &serde_json::Value| {
        crate::sha256_bytes(&serde_json::to_vec(value).unwrap_or_default())
    };
    (
        actual.as_ref().map(&digest),
        digest(&expected),
        actual
            .as_ref()
            .and_then(|value| value.get("arguments"))
            .map(&digest),
        digest(
            expected
                .get("arguments")
                .unwrap_or(&serde_json::Value::Null),
        ),
    )
}

fn program_role_argument_name(program: &crate::ResponseProgram) -> Option<&str> {
    let arguments = match &program.operation {
        crate::ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | crate::ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return None,
    };
    arguments.iter().find_map(|argument| match argument {
        crate::ResponseArgument::Role {
            name,
            role: crate::SemanticRole::ContinuationHandle,
            ..
        } => Some(name.as_str()),
        _ => None,
    })
}

fn response_argument_value(response: &str, name: &str) -> Option<serde_json::Value> {
    let response = serde_json::from_str::<serde_json::Value>(response).ok()?;
    let arguments = response.get("arguments")?;
    let arguments = match arguments {
        serde_json::Value::String(value) => serde_json::from_str(value).ok()?,
        value => value.clone(),
    };
    arguments.get(name).cloned()
}

fn role_value_provenance(
    program: &crate::ResponseProgram,
    parity: &crate::RuntimeParityCase,
) -> (
    Option<String>,
    Option<String>,
    Vec<SemanticLawValueOccurrence>,
    Vec<SemanticLawValueOccurrence>,
) {
    let Some(argument_name) = program_role_argument_name(program) else {
        return (None, None, Vec::new(), Vec::new());
    };
    let execution =
        crate::execute_response(program, &parity.request_text, &parity.provider_payload);
    let actual = execution
        .response
        .as_deref()
        .and_then(|response| response_argument_value(response, argument_name));
    let expected = response_argument_value(&parity.expected_response, argument_name);
    let value_digest = |value: &serde_json::Value| {
        crate::sha256_bytes(&serde_json::to_vec(value).unwrap_or_default())
    };
    let actual_occurrences = actual.as_ref().map_or_else(Vec::new, |value| {
        structural_value_occurrences(&parity.provider_payload, value)
    });
    let expected_occurrences = expected.as_ref().map_or_else(Vec::new, |value| {
        structural_value_occurrences(&parity.provider_payload, value)
    });
    (
        actual.as_ref().map(&value_digest),
        expected.as_ref().map(&value_digest),
        actual_occurrences,
        expected_occurrences,
    )
}

fn structural_value_occurrences(
    root: &serde_json::Value,
    target: &serde_json::Value,
) -> Vec<SemanticLawValueOccurrence> {
    fn visit(
        value: &serde_json::Value,
        target: &serde_json::Value,
        path: &mut Vec<String>,
        scope: &str,
        output: &mut BTreeSet<SemanticLawValueOccurrence>,
    ) {
        let path_sha256 =
            || crate::sha256_bytes(&serde_json::to_vec(path.as_slice()).unwrap_or_default());
        if value == target {
            output.insert(SemanticLawValueOccurrence {
                scope: scope.to_owned(),
                path_sha256: path_sha256(),
                encoding: "json_scalar".to_owned(),
                ordinal: 0,
            });
        }
        if let serde_json::Value::String(text) = value {
            let target_text = match target {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                serde_json::Value::Bool(value) => Some(value.to_string()),
                _ => None,
            };
            if let Some(target_text) = target_text {
                for (ordinal, token) in text
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric()
                            || matches!(character, '-' | '_' | '.' | ':' | '/'))
                    })
                    .filter(|token| !token.is_empty())
                    .enumerate()
                {
                    if token == target_text {
                        output.insert(SemanticLawValueOccurrence {
                            scope: scope.to_owned(),
                            path_sha256: path_sha256(),
                            encoding: "text_token".to_owned(),
                            ordinal,
                        });
                    }
                }
            }
            if let Ok(embedded) = serde_json::from_str::<serde_json::Value>(text) {
                path.push("embedded_json".to_owned());
                visit(&embedded, target, path, scope, output);
                path.pop();
            }
        }
        match value {
            serde_json::Value::Object(values) => {
                for (key, child) in values {
                    path.push(format!("key:{}", crate::sha256_bytes(key.as_bytes())));
                    visit(child, target, path, scope, output);
                    path.pop();
                }
            }
            serde_json::Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    path.push(format!("index:{index}"));
                    let child_scope = if path.len() == 2
                        && path.first().is_some_and(|part| {
                            part == &format!("key:{}", crate::sha256_bytes(b"input"))
                        }) {
                        match (
                            child.get("role").and_then(serde_json::Value::as_str),
                            child.get("type").and_then(serde_json::Value::as_str),
                        ) {
                            (Some("user"), _) => "request",
                            (_, Some("function_call_output" | "custom_tool_call_output")) => {
                                "tool_output"
                            }
                            _ => scope,
                        }
                    } else {
                        scope
                    };
                    visit(child, target, path, child_scope, output);
                    path.pop();
                }
            }
            _ => {}
        }
    }

    let mut output = BTreeSet::new();
    visit(root, target, &mut Vec::new(), "provider", &mut output);
    output.into_iter().collect()
}

fn json_shape(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Null => serde_json::Value::String("null".to_owned()),
        serde_json::Value::Bool(_) => serde_json::Value::String("boolean".to_owned()),
        serde_json::Value::Number(number) if number.is_i64() || number.is_u64() => {
            serde_json::Value::String("integer".to_owned())
        }
        serde_json::Value::Number(_) => serde_json::Value::String("number".to_owned()),
        serde_json::Value::String(_) => serde_json::Value::String("string".to_owned()),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(json_shape).collect())
        }
        serde_json::Value::Object(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), json_shape(value)))
                .collect(),
        ),
    }
}

fn selector_candidate_count(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Option<usize> {
    let output = immediate_tool_output_value(provider_payload)?;
    match selector {
        ResponseValueSelector::ContentLinePrefix { prefix, .. } => Some(
            output_text_parts(output)
                .ok()?
                .into_iter()
                .flat_map(str::lines)
                .filter_map(|line| line.trim().strip_prefix(prefix))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .count(),
        ),
        ResponseValueSelector::ContinuationHandle { .. } => {
            let mut matches = output_text_parts(output)
                .ok()?
                .into_iter()
                .flat_map(str::lines)
                .filter_map(|line| {
                    let line = line.trim();
                    [
                        "Script running with cell ID ",
                        "Process running with session ID ",
                    ]
                    .into_iter()
                    .find_map(|prefix| line.strip_prefix(prefix))
                })
                .filter_map(|tail| tail.split_whitespace().next())
                .filter(|value| {
                    !value.is_empty()
                        && value.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
                        })
                })
                .collect::<Vec<_>>();
            matches.sort_unstable();
            matches.dedup();
            Some(matches.len())
        }
        _ => None,
    }
}

fn selector_candidate_provenance(
    provider_payload: &Value,
    selector: &ResponseValueSelector,
) -> Vec<(String, String, AtomValueType)> {
    let Some(output) = immediate_tool_output_value(provider_payload) else {
        return Vec::new();
    };
    let ResponseValueSelector::ContentLinePrefix { prefix, value_type } = selector else {
        return Vec::new();
    };
    let Ok(parts) = output_text_parts(output) else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    for (part_ordinal, part) in parts.into_iter().enumerate() {
        for (line_ordinal, line) in part.lines().enumerate() {
            let Some(value) = line.trim().strip_prefix(prefix).map(str::trim) else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            let Ok(value) = parse_scalar_text(value, *value_type) else {
                continue;
            };
            let locator_sha256 = crate::sha256_bytes(
                &serde_json::to_vec(&(
                    "immediate_tool_output_line",
                    part_ordinal,
                    line_ordinal,
                    crate::sha256_bytes(prefix.as_bytes()),
                ))
                .unwrap_or_default(),
            );
            let value_sha256 =
                crate::sha256_bytes(&serde_json::to_vec(&value.value).unwrap_or_default());
            candidates.push((locator_sha256, value_sha256, value.value_type));
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

impl StreamingSelfTrainingState {
    /// Read-only, privacy-safe replay of one semantic law. This diagnostic never
    /// mutates CEGIS, generations, parity reservoirs, or execution authority.
    #[must_use]
    pub fn semantic_law_evidence_audit(
        &self,
        requested_signatures: &BTreeSet<String>,
    ) -> SemanticLawEvidenceAudit {
        #[derive(Clone)]
        struct ActorCandidate {
            program: crate::ResponseProgram,
            origins: BTreeSet<String>,
            member_signatures: BTreeSet<String>,
            cohort_ids: BTreeSet<String>,
            action_symbols: BTreeSet<String>,
        }

        let requested_laws = requested_signatures
            .iter()
            .filter_map(|signature| self.discovery.semantic_law_signature(signature))
            .collect::<BTreeSet<_>>();
        let members = self
            .cegis
            .winners()
            .into_iter()
            .filter(|winner| requested_signatures.contains(&winner.teacher_signature_sha256))
            .collect::<Vec<_>>();
        let member_signatures = members
            .iter()
            .map(|winner| winner.teacher_signature_sha256.clone())
            .collect::<BTreeSet<_>>();

        let mut frames = BTreeMap::<String, crate::RelationFrame>::new();
        for signature in &member_signatures {
            if let Some(pool) = self.pool_snapshot_with_parity(signature) {
                for frame in pool.positives.into_iter().chain(pool.negatives) {
                    frames.insert(frame.frame_id_sha256.clone(), frame);
                }
            }
        }
        for cohort in self.admission_cohorts().into_iter().filter(|cohort| {
            cohort
                .physical_members
                .iter()
                .any(|member| requested_signatures.contains(&member.teacher_signature_sha256))
        }) {
            for frame in cohort
                .generation
                .support
                .into_iter()
                .chain(cohort.generation.future)
            {
                frames.insert(frame.frame_id_sha256.clone(), frame);
            }
        }

        let mut missing_parity_frame_ids = Vec::new();
        let mut audit_cases = Vec::<(crate::RelationFrame, crate::RuntimeParityCase)>::new();
        for frame in frames.into_values() {
            if let Some(parity) = self.support_parity_case(&frame.frame_id_sha256) {
                audit_cases.push((frame, parity));
            } else {
                missing_parity_frame_ids.push(frame.frame_id_sha256);
            }
        }
        audit_cases.sort_by(|left, right| left.0.frame_id_sha256.cmp(&right.0.frame_id_sha256));
        missing_parity_frame_ids.sort();

        let rows = audit_cases
            .iter()
            .map(|(frame, parity)| {
                let (selector_kind, selector_sha256, observed_value_type) = frame
                    .atoms
                    .iter()
                    .find_map(|atom| match atom {
                        crate::RelationAtom::ObservationSelector { selector, .. } => {
                            let (kind, value_type) = selector_contract(selector);
                            Some((
                                kind,
                                crate::sha256_bytes(
                                    &serde_json::to_vec(selector).unwrap_or_default(),
                                ),
                                value_type,
                            ))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| ("none".to_owned(), crate::sha256_bytes(b"none"), None));
                let (argument_schema_sha256, constant_contract_sha256, emitted_value_types) =
                    frame_argument_contract(frame);
                let protocol_class = frame_protocol_class(frame);
                SemanticLawEvidenceAuditRow {
                    frame_id_sha256: frame.frame_id_sha256.clone(),
                    evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
                    member_signature_sha256: crate::teacher_program_signature(frame)
                        .unwrap_or_else(|| "unknown_teacher_signature".to_owned()),
                    effect_law_sha256: crate::teacher_semantic_law_signature(frame)
                        .unwrap_or_else(|| "unknown_effect_law".to_owned()),
                    protocol_class: protocol_class.clone(),
                    selector_kind,
                    selector_sha256,
                    source_layout_sha256: crate::runtime::actor_structural_layout_sha256(
                        &parity.provider_payload,
                    )
                    .unwrap_or_else(|reason| crate::sha256_bytes(reason.as_bytes())),
                    observed_value_type,
                    emitted_value_types,
                    argument_schema_sha256,
                    constant_contract_sha256,
                    capability_class: protocol_class,
                }
            })
            .collect::<Vec<_>>();

        let mut actor_candidates = BTreeMap::<String, ActorCandidate>::new();
        let mut insert_actor =
            |program: crate::ResponseProgram, origin: &str, member: &crate::CegisWinner| {
                let digest = crate::sha256_bytes(&serde_json::to_vec(&program).unwrap_or_default());
                let actor = actor_candidates
                    .entry(digest)
                    .or_insert_with(|| ActorCandidate {
                        program,
                        origins: BTreeSet::new(),
                        member_signatures: BTreeSet::new(),
                        cohort_ids: BTreeSet::new(),
                        action_symbols: BTreeSet::new(),
                    });
                actor.origins.insert(origin.to_owned());
                actor
                    .member_signatures
                    .insert(member.teacher_signature_sha256.clone());
                actor.cohort_ids.insert(member.cohort_id_sha256.clone());
                actor.action_symbols.insert(member.action_symbol.clone());
            };
        for member in &members {
            insert_actor(member.program.clone(), "physical_winner", member);
            let support = audit_cases
                .iter()
                .filter(|(frame, _)| {
                    crate::teacher_program_signature(frame).as_deref()
                        == Some(member.teacher_signature_sha256.as_str())
                })
                .map(|(frame, _)| frame.clone())
                .collect::<Vec<_>>();
            if let Some(program) =
                crate::synthesis::canonicalize_continuation_role_program(&member.program, &support)
            {
                insert_actor(program, "canonical_continuation", member);
            }
        }
        let mut actors = actor_candidates
            .into_iter()
            .map(|(actor_program_sha256, actor)| {
                let (
                    operation_kind,
                    selector_kind,
                    selector_value_type,
                    argument_schema_sha256,
                    constant_contract_sha256,
                ) = program_contract(&actor.program);
                let mut exact_rows = 0_usize;
                let mut wrong_rows = 0_usize;
                let mut abstain_rows = 0_usize;
                let mut verify_failed_rows = 0_usize;
                let outcomes = audit_cases
                    .iter()
                    .map(|(frame, parity)| {
                        let (outcome, reason) = semantic_replay_outcome(&actor.program, parity);
                        let (
                            actual_action_sha256,
                            expected_action_sha256,
                            actual_arguments_sha256,
                            expected_arguments_sha256,
                        ) = replay_action_digests(&actor.program, parity);
                        let (
                            actual_role_value_sha256,
                            expected_role_value_sha256,
                            actual_role_occurrences,
                            expected_role_occurrences,
                        ) = role_value_provenance(&actor.program, parity);
                        match outcome.as_str() {
                            "exact" => exact_rows = exact_rows.saturating_add(1),
                            "wrong" => wrong_rows = wrong_rows.saturating_add(1),
                            "abstain" => abstain_rows = abstain_rows.saturating_add(1),
                            "verify_failed" => {
                                verify_failed_rows = verify_failed_rows.saturating_add(1)
                            }
                            _ => {}
                        }
                        SemanticLawActorReplayOutcome {
                            frame_id_sha256: frame.frame_id_sha256.clone(),
                            outcome,
                            reason,
                            selector_candidate_count: program_selector(&actor.program).and_then(
                                |selector| {
                                    selector_candidate_count(&parity.provider_payload, selector)
                                },
                            ),
                            actual_action_sha256,
                            expected_action_sha256,
                            actual_arguments_sha256,
                            expected_arguments_sha256,
                            actual_role_value_sha256,
                            expected_role_value_sha256,
                            actual_role_occurrences,
                            expected_role_occurrences,
                            selector_candidates: program_selector(&actor.program)
                                .map(|selector| {
                                    selector_candidate_provenance(
                                        &parity.provider_payload,
                                        selector,
                                    )
                                    .into_iter()
                                    .map(|(locator_sha256, value_sha256, value_type)| {
                                        SemanticLawSelectorCandidate {
                                            locator_sha256,
                                            value_sha256,
                                            value_type,
                                        }
                                    })
                                    .collect()
                                })
                                .unwrap_or_default(),
                        }
                    })
                    .collect();
                SemanticLawActorAudit {
                    actor_program_sha256,
                    origin: actor.origins.into_iter().collect::<Vec<_>>().join("+"),
                    member_signatures: actor.member_signatures.into_iter().collect(),
                    cohort_ids: actor.cohort_ids.into_iter().collect(),
                    action_symbols: actor.action_symbols.into_iter().collect(),
                    operation_kind,
                    selector_kind,
                    selector_value_type,
                    argument_schema_sha256,
                    constant_contract_sha256,
                    exact_rows,
                    wrong_rows,
                    abstain_rows,
                    verify_failed_rows,
                    outcomes,
                }
            })
            .collect::<Vec<_>>();
        actors.sort_by(|left, right| {
            right
                .exact_rows
                .cmp(&left.exact_rows)
                .then_with(|| left.wrong_rows.cmp(&right.wrong_rows))
                .then_with(|| left.actor_program_sha256.cmp(&right.actor_program_sha256))
        });

        SemanticLawEvidenceAudit {
            schema: SEMANTIC_LAW_EVIDENCE_AUDIT_SCHEMA_V1.to_owned(),
            requested_signatures: requested_signatures.iter().cloned().collect(),
            effect_law_sha256: requested_laws.into_iter().collect(),
            member_signatures: member_signatures.into_iter().collect(),
            rows,
            actors,
            missing_parity_frame_ids,
        }
    }

    /// Builds label-blind candidate graphs from the same frozen parity rows,
    /// freezes every graph, and only then joins expected binding receipts.
    pub fn semantic_law_binding_evidence_report(
        &self,
        requested_signatures: &BTreeSet<String>,
    ) -> Result<crate::BindingVersionSpaceReportV1, String> {
        let members = self
            .cegis
            .winners()
            .into_iter()
            .filter(|winner| requested_signatures.contains(&winner.teacher_signature_sha256))
            .collect::<Vec<_>>();
        let member_signatures = members
            .iter()
            .map(|winner| winner.teacher_signature_sha256.clone())
            .collect::<BTreeSet<_>>();
        let mut frames = BTreeMap::<String, crate::RelationFrame>::new();
        for signature in &member_signatures {
            if let Some(pool) = self.pool_snapshot_with_parity(signature) {
                for frame in pool.positives {
                    frames.insert(frame.frame_id_sha256.clone(), frame);
                }
            }
        }

        let budget = crate::BindingEvidenceBudgetV1::default();
        let mut frozen_graphs = Vec::new();
        for frame in frames.values() {
            let Some(parity) = self.support_parity_case(&frame.frame_id_sha256) else {
                continue;
            };
            let context = pre_action_binding_context(frame);
            let surface = crate::PreActionBindingSurfaceV1::capture(
                frame.frame_id_sha256.clone(),
                frame.evidence_ref_sha256.clone(),
                &parity.request_text,
                &parity.provider_payload,
                context,
                budget,
            )
            .map_err(|error| format!("binding_surface_capture:{error:?}"))?;
            let graph = surface
                .candidate_relation_graph(budget)
                .map_err(|error| format!("binding_candidate_graph:{error:?}"))?
                .freeze()
                .map_err(|error| format!("binding_candidate_graph_freeze:{error:?}"))?;
            frozen_graphs.push(graph);
        }
        frozen_graphs
            .sort_by(|left, right| left.graph.row_id_sha256.cmp(&right.graph.row_id_sha256));

        // Expected action data is deliberately unavailable until every
        // pre-action candidate graph above has become immutable.
        let audit = self.semantic_law_evidence_audit(requested_signatures);
        let rows = audit
            .rows
            .iter()
            .map(|row| (row.frame_id_sha256.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let protocol_rows = audit
            .rows
            .iter()
            .map(|row| (row.frame_id_sha256.as_str(), row.protocol_class.as_str()))
            .collect::<BTreeMap<_, _>>();
        let protocols = audit
            .rows
            .iter()
            .map(|row| row.protocol_class.clone())
            .collect::<BTreeSet<_>>();
        let target_actors = protocols
            .into_iter()
            .filter_map(|protocol| {
                audit
                    .actors
                    .iter()
                    .filter(|actor| {
                        actor.origin.contains("physical_winner")
                            && actor.action_symbols.iter().any(|value| value == &protocol)
                    })
                    .max_by(|left, right| {
                        scoped_exact_rows(left, &protocol, &protocol_rows)
                            .cmp(&scoped_exact_rows(right, &protocol, &protocol_rows))
                            .then_with(|| {
                                right.actor_program_sha256.cmp(&left.actor_program_sha256)
                            })
                    })
                    .map(|actor| (protocol, actor))
            })
            .collect::<BTreeMap<_, _>>();

        let mut expected_receipts = Vec::with_capacity(frozen_graphs.len());
        for graph in &frozen_graphs {
            let row = rows
                .get(graph.graph.row_id_sha256.as_str())
                .ok_or_else(|| "binding_audit_row_missing".to_owned())?;
            let actor = target_actors
                .get(&row.protocol_class)
                .ok_or_else(|| "binding_target_actor_missing".to_owned())?;
            let outcome = actor
                .outcomes
                .iter()
                .find(|outcome| outcome.frame_id_sha256 == row.frame_id_sha256)
                .ok_or_else(|| "binding_target_actor_outcome_missing".to_owned())?;
            let expected = outcome
                .expected_role_value_sha256
                .as_deref()
                .ok_or_else(|| "binding_expected_role_value_missing".to_owned())?;
            let baseline_outcome = match outcome.outcome.as_str() {
                "exact" => crate::BindingBaselineOutcomeV1::Exact,
                "wrong" => crate::BindingBaselineOutcomeV1::Wrong,
                "abstain" => crate::BindingBaselineOutcomeV1::Abstain,
                "verify_failed" => crate::BindingBaselineOutcomeV1::VerifyFailed,
                _ => return Err("binding_unknown_baseline_outcome".to_owned()),
            };
            expected_receipts.push(
                crate::ExpectedBindingReceiptV1::positive(graph, expected, baseline_outcome)
                    .map_err(|error| format!("binding_expected_receipt:{error:?}"))?,
            );
        }
        crate::evaluate_binding_version_space_v1(
            frozen_graphs,
            expected_receipts,
            audit.missing_parity_frame_ids,
            budget,
        )
        .map_err(|error| format!("binding_version_space:{error:?}"))
    }
}

fn scoped_exact_rows(
    actor: &SemanticLawActorAudit,
    protocol: &str,
    protocol_rows: &BTreeMap<&str, &str>,
) -> usize {
    actor
        .outcomes
        .iter()
        .filter(|outcome| {
            outcome.outcome == "exact"
                && protocol_rows
                    .get(outcome.frame_id_sha256.as_str())
                    .is_some_and(|value| *value == protocol)
        })
        .count()
}

fn pre_action_binding_context(frame: &crate::RelationFrame) -> crate::PreActionBindingContextV1 {
    let mut call_shape_count = 0_u16;
    let mut capability_count = 0_u16;
    let mut temporal_relation_count = 0_u16;
    let mut cardinality_relation_count = 0_u16;
    let mut completion_state = crate::BindingCompletionStateV1::Unknown;
    let mut topology = Vec::new();
    for atom in &frame.atoms {
        match atom {
            crate::RelationAtom::ObservationCallShape { .. } => {
                call_shape_count = call_shape_count.saturating_add(1);
                topology.push(serde_json::json!(["observation_call_shape"]));
            }
            crate::RelationAtom::ClientCapabilityAtom { .. } => {
                capability_count = capability_count.saturating_add(1);
                topology.push(serde_json::json!(["client_capability"]));
            }
            crate::RelationAtom::ToolKind { .. } => {
                capability_count = capability_count.saturating_add(1);
                topology.push(serde_json::json!(["tool_kind"]));
            }
            crate::RelationAtom::TemporalEdge { .. } => {
                temporal_relation_count = temporal_relation_count.saturating_add(1);
                topology.push(serde_json::json!(["temporal_edge"]));
            }
            crate::RelationAtom::Cardinality { count, .. } => {
                cardinality_relation_count = cardinality_relation_count.saturating_add(1);
                topology.push(serde_json::json!(["cardinality", count]));
            }
            crate::RelationAtom::CompletionState { value } => {
                completion_state = canonical_binding_completion_state(value);
                topology.push(serde_json::json!(["completion_state", completion_state]));
            }
            crate::RelationAtom::TypedSlot {
                source: crate::AtomSource::Request | crate::AtomSource::Observation,
                value_type,
                ..
            } => topology.push(serde_json::json!(["typed_slot", value_type])),
            crate::RelationAtom::SlotEquality { .. } => {
                topology.push(serde_json::json!(["slot_equality"]));
            }
            crate::RelationAtom::UniqueSlot { .. } => {
                topology.push(serde_json::json!(["unique_slot"]));
            }
            crate::RelationAtom::RequestPhaseAtom { .. } => {
                topology.push(serde_json::json!(["request_relation"]));
            }
            crate::RelationAtom::OutputStatus { .. } => {
                topology.push(serde_json::json!(["output_status"]));
            }
            crate::RelationAtom::ObservationSelector { .. } => {}
            atom if crate::relation_atom_is_teacher_only(atom) => {}
            _ => {}
        }
    }
    topology.sort_by(|left, right| {
        serde_json::to_string(left)
            .unwrap_or_default()
            .cmp(&serde_json::to_string(right).unwrap_or_default())
    });
    crate::PreActionBindingContextV1 {
        call_shape_count,
        capability_count,
        completion_state,
        temporal_relation_count,
        cardinality_relation_count,
        topology_neighborhood_root_sha256: crate::sha256_bytes(
            &serde_json::to_vec(&topology).unwrap_or_default(),
        ),
    }
}

fn canonical_binding_completion_state(value: &str) -> crate::BindingCompletionStateV1 {
    match value.to_ascii_lowercase().as_str() {
        "active" | "in_progress" | "pending" | "running" | "yielded" => {
            crate::BindingCompletionStateV1::Unresolved
        }
        "cancelled" | "completed" | "failed" | "success" | "terminated" => {
            crate::BindingCompletionStateV1::Completed
        }
        _ => crate::BindingCompletionStateV1::Unknown,
    }
}
