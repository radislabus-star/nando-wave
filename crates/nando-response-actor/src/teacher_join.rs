use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AtomSource, EconomicsReceipt, RelationAtom, RelationFrame, ResponseArgument, ResponseOperation,
    ResponseProgram, RuntimeFrame, RuntimeParityCase, SemanticRole, TEACHER_OUTCOME_SCHEMA_V1,
    TEACHER_TRANSITION_SCHEMA_V1, TeacherActionAst, TeacherOutcome, TeacherTransition,
    TeacherVerifierEvidence, ground_roles, relation_atom_is_teacher_only,
};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct TeacherJoinKey {
    pub session_id_sha256: String,
    pub client_intent_id_sha256: String,
    pub event_id_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TeacherJoinRejection {
    MissingVerifierLabel,
    MissingTeacherAction,
    MissingRuntimeFrame,
    DuplicateConflict,
    CapacityEviction,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherJoinReport {
    pub runtime_frames_seen: u64,
    pub outcomes_seen: u64,
    pub joined_transitions: u64,
    pub duplicate_idempotent: u64,
    pub duplicate_conflicts: u64,
    pub capacity_evictions: u64,
    pub pending_runtime_frames: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PendingRuntime {
    frame: RuntimeFrame,
    economics: Option<EconomicsReceipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TeacherJoin {
    max_pending: usize,
    pending: BTreeMap<TeacherJoinKey, PendingRuntime>,
    order: VecDeque<TeacherJoinKey>,
    completed_digests: BTreeMap<TeacherJoinKey, String>,
    report: TeacherJoinReport,
}

impl TeacherJoin {
    #[must_use]
    pub fn new(max_pending: usize) -> Self {
        Self {
            max_pending: max_pending.max(1),
            pending: BTreeMap::new(),
            order: VecDeque::new(),
            completed_digests: BTreeMap::new(),
            report: TeacherJoinReport::default(),
        }
    }

    pub fn observe_runtime(
        &mut self,
        frame: RuntimeFrame,
        economics: Option<EconomicsReceipt>,
    ) -> Result<TeacherJoinKey, TeacherJoinRejection> {
        self.report.runtime_frames_seen = self.report.runtime_frames_seen.saturating_add(1);
        let key = teacher_join_key(&frame);
        if let Some(existing) = self.pending.get(&key) {
            if existing.frame == frame && existing.economics == economics {
                self.report.duplicate_idempotent =
                    self.report.duplicate_idempotent.saturating_add(1);
                return Ok(key);
            }
            self.report.duplicate_conflicts = self.report.duplicate_conflicts.saturating_add(1);
            return Err(TeacherJoinRejection::DuplicateConflict);
        }
        while self.pending.len() >= self.max_pending {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if self.pending.remove(&oldest).is_some() {
                self.report.capacity_evictions = self.report.capacity_evictions.saturating_add(1);
                break;
            }
        }
        self.order.push_back(key.clone());
        self.pending
            .insert(key.clone(), PendingRuntime { frame, economics });
        self.report.pending_runtime_frames = self.pending.len();
        Ok(key)
    }

    pub fn observe_outcome(
        &mut self,
        key: &TeacherJoinKey,
        outcome: TeacherOutcome,
    ) -> Result<TeacherTransition, TeacherJoinRejection> {
        self.report.outcomes_seen = self.report.outcomes_seen.saturating_add(1);
        let digest = digest_json(&outcome);
        if let Some(existing) = self.completed_digests.get(key) {
            if existing == &digest {
                self.report.duplicate_idempotent =
                    self.report.duplicate_idempotent.saturating_add(1);
            } else {
                self.report.duplicate_conflicts = self.report.duplicate_conflicts.saturating_add(1);
            }
            return Err(TeacherJoinRejection::DuplicateConflict);
        }
        let Some(pending) = self.pending.remove(key) else {
            return Err(TeacherJoinRejection::MissingRuntimeFrame);
        };
        self.completed_digests.insert(key.clone(), digest);
        trim_completed(
            &mut self.completed_digests,
            self.max_pending.saturating_mul(2),
        );
        self.report.joined_transitions = self.report.joined_transitions.saturating_add(1);
        self.report.pending_runtime_frames = self.pending.len();
        Ok(TeacherTransition {
            schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
            before: pending.frame,
            outcome,
            economics: pending.economics,
            runtime_parity_case: None,
        })
    }

    pub fn observe_completed_frame(
        &mut self,
        frame: &RelationFrame,
        economics: Option<EconomicsReceipt>,
    ) -> Result<TeacherTransition, TeacherJoinRejection> {
        let runtime = RuntimeFrame::from_completed(frame);
        let key = self.observe_runtime(runtime, economics)?;
        let outcome = teacher_outcome_from_completed(frame)?;
        self.observe_outcome(&key, outcome)
    }

    #[must_use]
    pub fn report(&self) -> TeacherJoinReport {
        let mut report = self.report.clone();
        report.pending_runtime_frames = self.pending.len();
        report
    }
}

impl Default for TeacherJoin {
    fn default() -> Self {
        Self::new(4_096)
    }
}

#[must_use]
pub fn teacher_join_key(frame: &RuntimeFrame) -> TeacherJoinKey {
    TeacherJoinKey {
        session_id_sha256: frame.session_id_sha256.clone(),
        client_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        event_id_sha256: frame.event_id_sha256.clone(),
    }
}

pub fn teacher_transition_from_completed(
    frame: &RelationFrame,
    economics: Option<EconomicsReceipt>,
) -> Result<TeacherTransition, TeacherJoinRejection> {
    let before = RuntimeFrame::from_completed(frame);
    let outcome = teacher_outcome_from_completed(frame)?;
    Ok(TeacherTransition {
        schema: TEACHER_TRANSITION_SCHEMA_V1.to_owned(),
        before,
        outcome,
        economics,
        runtime_parity_case: None,
    })
}

pub fn teacher_outcome_from_completed(
    frame: &RelationFrame,
) -> Result<TeacherOutcome, TeacherJoinRejection> {
    let accepted = frame
        .verifier_label
        .ok_or(TeacherJoinRejection::MissingVerifierLabel)?;
    let action = teacher_action_ast(frame).ok_or(TeacherJoinRejection::MissingTeacherAction)?;
    let output_digest_sha256 =
        digest_json(&(frame.evidence_ref_sha256.as_str(), &action.atoms, accepted));
    Ok(TeacherOutcome {
        schema: TEACHER_OUTCOME_SCHEMA_V1.to_owned(),
        action,
        verifier: TeacherVerifierEvidence {
            accepted,
            evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
            output_digest_sha256,
        },
        completed_at_unix_nanos: frame.observed_at_unix_nanos,
    })
}

#[must_use]
pub fn teacher_action_ast(frame: &RelationFrame) -> Option<TeacherActionAst> {
    let signature_sha256 = teacher_program_signature(frame)?;
    let action_slots = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: AtomSource::Action | AtomSource::Outcome,
                ..
            } => Some(*slot_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut atoms = frame
        .atoms
        .iter()
        .filter(|atom| {
            relation_atom_is_teacher_only(atom)
                || matches!(
                    atom,
                    RelationAtom::SlotEquality {
                        left_slot,
                        right_slot
                    } if action_slots.contains(left_slot) || action_slots.contains(right_slot)
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    atoms.sort();
    atoms.dedup();
    Some(TeacherActionAst {
        signature_sha256,
        action_symbol: teacher_action_symbol(frame),
        atoms,
    })
}

/// Stable post-completion teacher identity. It is legal in training and never
/// appears in runtime routing atoms.
#[must_use]
pub fn teacher_program_signature(frame: &RelationFrame) -> Option<String> {
    let mut action_atoms = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument {
                name, value_type, ..
            } => {
                let mut signature = serde_json::json!({
                    "kind": "action_role_argument",
                    "name": name,
                });
                if let Some(value_type) = value_type {
                    signature["value_type"] = serde_json::json!(value_type);
                }
                Some(signature)
            }
            RelationAtom::ActionIntegerArgument { name, .. }
                if !is_execution_budget_argument(name) =>
            {
                Some(serde_json::json!({
                    "kind": "action_integer_argument",
                    "name": name,
                }))
            }
            RelationAtom::ActionStringArgument { name, value }
                if !(name == "chars" && value.is_empty()) =>
            {
                Some(serde_json::json!({
                    "kind": "action_string_argument",
                    "name": name,
                }))
            }
            RelationAtom::ActionBooleanArgument { name, .. } => Some(serde_json::json!({
                "kind": "action_boolean_argument",
                "name": name,
            })),
            RelationAtom::ActionFunction { value } => Some(serde_json::json!({
                "kind": "action_function",
                "value": value,
            })),
            RelationAtom::ActionCustomTool { value } => Some(serde_json::json!({
                "kind": "action_custom_tool",
                "value": value,
            })),
            RelationAtom::ActionInnerTool { value } => Some(serde_json::json!({
                "kind": "action_inner_tool",
                "value": value,
            })),
            RelationAtom::ActionResultProjection { .. } => Some(serde_json::json!({
                "kind": "action_result_projection",
            })),
            RelationAtom::ActionOutputProjection { .. } => Some(serde_json::json!({
                "kind": "action_output_projection",
            })),
            RelationAtom::ActionJsonResultProjection => Some(serde_json::json!({
                "kind": "action_json_result_projection",
            })),
            RelationAtom::ActionValueProjection { format, renderer } => Some(serde_json::json!({
                "kind": "action_value_projection",
                "format": format,
                "renderer_kind": collection_renderer_kind(renderer),
            })),
            RelationAtom::ActionStatusProjection { mapping } => Some(serde_json::json!({
                "kind": "action_status_projection",
                "mapping": mapping,
            })),
            _ => None,
        })
        .collect::<Vec<_>>();
    action_atoms.sort_by_cached_key(|atom| serde_json::to_vec(atom).unwrap_or_default());
    (!action_atoms.is_empty()).then(|| digest_json(&action_atoms))
}

/// Restores the exact wire schema from an independently captured runtime
/// parity case. Older frames omitted role value types and constant arguments;
/// keeping them untyped would split one teacher program during migration.
pub(crate) fn action_schema_enriched_frame(
    frame: &RelationFrame,
    parity: Option<&RuntimeParityCase>,
) -> RelationFrame {
    let Some(parity) = parity else {
        return frame.clone();
    };
    let Ok(expected) = serde_json::from_str::<serde_json::Value>(&parity.expected_response) else {
        return frame.clone();
    };
    let Some(function_name) = expected.get("name").and_then(serde_json::Value::as_str) else {
        return frame.clone();
    };
    if !frame.atoms.iter().any(
        |atom| matches!(atom, RelationAtom::ActionFunction { value } if value == function_name),
    ) {
        return frame.clone();
    }
    let Some(arguments) = expected
        .get("arguments")
        .and_then(serde_json::Value::as_object)
    else {
        return frame.clone();
    };
    let role_names = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut enriched = frame.clone();
    for atom in &mut enriched.atoms {
        if let RelationAtom::ActionRoleArgument {
            name, value_type, ..
        } = atom
            && let Some(value) = arguments.get(name)
        {
            *value_type = action_scalar_type(value);
        }
    }
    enriched.atoms.retain(|atom| match atom {
        RelationAtom::ActionIntegerArgument { name, .. }
        | RelationAtom::ActionStringArgument { name, .. }
        | RelationAtom::ActionBooleanArgument { name, .. } => {
            role_names.contains(name) || !arguments.contains_key(name)
        }
        _ => true,
    });
    for (name, value) in arguments {
        if role_names.contains(name) {
            continue;
        }
        let atom = match value {
            serde_json::Value::Number(value) => {
                value
                    .as_u64()
                    .map(|value| RelationAtom::ActionIntegerArgument {
                        name: name.clone(),
                        value,
                    })
            }
            serde_json::Value::String(value) => Some(RelationAtom::ActionStringArgument {
                name: name.clone(),
                value: value.clone(),
            }),
            serde_json::Value::Bool(value) => Some(RelationAtom::ActionBooleanArgument {
                name: name.clone(),
                value: *value,
            }),
            _ => None,
        };
        if let Some(atom) = atom {
            enriched.atoms.push(atom);
        }
    }
    enriched.atoms.sort();
    enriched.atoms.dedup();
    enriched
}

fn action_scalar_type(value: &serde_json::Value) -> Option<crate::AtomValueType> {
    match value {
        serde_json::Value::Number(value) if value.as_u64().is_some() => {
            Some(crate::AtomValueType::Integer)
        }
        serde_json::Value::String(_) => Some(crate::AtomValueType::String),
        serde_json::Value::Bool(_) => Some(crate::AtomValueType::Boolean),
        _ => None,
    }
}

fn collection_renderer_kind(renderer: &crate::CollectionOutputRenderer) -> &'static str {
    match renderer {
        crate::CollectionOutputRenderer::Direct => "direct",
        crate::CollectionOutputRenderer::RenderTemplate { .. } => "template",
        crate::CollectionOutputRenderer::RenderSequence { .. } => "sequence",
    }
}

#[must_use]
pub fn teacher_action_symbol(frame: &RelationFrame) -> String {
    let mut custom_tool = None;
    let mut inner_tool = None;
    for atom in &frame.atoms {
        match atom {
            RelationAtom::ActionFunction { value } => return format!("function:{value}"),
            RelationAtom::ActionCustomTool { value } => custom_tool = Some(value.as_str()),
            RelationAtom::ActionInnerTool { value } => inner_tool = Some(value.as_str()),
            RelationAtom::ActionValueProjection { .. } => return "value_projection".to_owned(),
            RelationAtom::ActionStatusProjection { .. } => return "status_projection".to_owned(),
            _ => {}
        }
    }
    match (custom_tool, inner_tool) {
        (Some(outer), Some(inner)) => format!("custom_tool:{outer}/{inner}"),
        (Some(outer), None) => format!("custom_tool:{outer}"),
        _ => "unknown".to_owned(),
    }
}

/// Accepted teacher actions can use different transport APIs while expressing
/// the same pre-action effect. Keep those rows as transfer evidence instead of
/// teaching CEGIS that one valid continuation is a counterexample to another.
pub(crate) fn teacher_actions_have_compatible_effect(
    left: &RelationFrame,
    right: &RelationFrame,
) -> bool {
    teacher_action_is_non_destructive_continuation(left)
        && teacher_action_is_non_destructive_continuation(right)
}

pub(crate) fn program_has_compatible_teacher_effect(
    program: &ResponseProgram,
    frame: &RelationFrame,
) -> bool {
    program_is_non_destructive_continuation(program)
        && teacher_action_is_non_destructive_continuation(frame)
}

fn teacher_action_is_non_destructive_continuation(frame: &RelationFrame) -> bool {
    let has_transport = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ActionFunction { .. } | RelationAtom::ActionCustomTool { .. }
        )
    });
    if !has_transport
        || frame.atoms.iter().any(|atom| {
            matches!(atom, RelationAtom::ActionBooleanArgument { value: true, .. })
                || matches!(atom, RelationAtom::ActionStringArgument { value, .. } if !value.is_empty())
                || matches!(atom, RelationAtom::ActionIntegerArgument { name, .. } if !is_execution_budget_argument(name))
        })
    {
        return false;
    }
    ground_roles(frame).into_iter().any(|hypothesis| {
        let Some(target_index) = hypothesis.bindings.get(&SemanticRole::TargetValue) else {
            return false;
        };
        if !hypothesis
            .bindings
            .contains_key(&SemanticRole::ContinuationHandle)
        {
            return false;
        }
        let Some(RelationAtom::TypedSlot { slot_id, .. }) = frame.atoms.get(*target_index) else {
            return false;
        };
        frame.atoms.iter().any(
            |atom| matches!(atom, RelationAtom::ActionRoleArgument { slot_id: argument_slot, .. } if argument_slot == slot_id),
        )
    })
}

fn program_is_non_destructive_continuation(program: &ResponseProgram) -> bool {
    let arguments = match &program.operation {
        ResponseOperation::FunctionCallFromRoles { arguments, .. }
        | ResponseOperation::CustomToolCallFromRoles { arguments, .. } => arguments,
        _ => return false,
    };
    arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: SemanticRole::ContinuationHandle,
                ..
            }
        )
    }) && arguments.iter().all(|argument| match argument {
        ResponseArgument::Boolean { value, .. } => !value,
        ResponseArgument::String { value, .. } => value.is_empty(),
        ResponseArgument::Integer { name, .. } => is_execution_budget_argument(name),
        ResponseArgument::Role { .. } => true,
    })
}

#[must_use]
pub fn is_execution_budget_argument(name: &str) -> bool {
    name.ends_with("_ms") || name == "max_tokens" || name.ends_with("_output_tokens")
}

fn trim_completed(completed: &mut BTreeMap<TeacherJoinKey, String>, limit: usize) {
    while completed.len() > limit {
        let Some(key) = completed.keys().next().cloned() else {
            break;
        };
        completed.remove(&key);
    }
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(atoms: Vec<RelationAtom>) -> RelationFrame {
        RelationFrame {
            schema: crate::RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: "1".repeat(64),
            event_id_sha256: "2".repeat(64),
            client_intent_id_sha256: "3".repeat(64),
            session_id_sha256: "4".repeat(64),
            observed_at_unix_nanos: 1,
            estimated_input_tokens: 10,
            extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms,
            evidence_ref_sha256: "5".repeat(64),
        }
    }

    #[test]
    fn teacher_signature_is_independent_of_atom_order() {
        let atoms = vec![
            RelationAtom::ActionFunction {
                value: "write_stdin".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 7,
                value_type: None,
            },
            RelationAtom::ActionIntegerArgument {
                name: "yield_time_ms".to_owned(),
                value: 1_000,
            },
        ];
        let mut reversed = atoms.clone();
        reversed.reverse();
        assert_eq!(
            teacher_program_signature(&frame(atoms)),
            teacher_program_signature(&frame(reversed))
        );
    }

    #[test]
    fn runtime_parity_recanonicalizes_legacy_action_schema() {
        let legacy = frame(vec![
            RelationAtom::ActionFunction {
                value: "write_stdin".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "session_id".to_owned(),
                slot_id: 7,
                value_type: None,
            },
        ]);
        let parity = RuntimeParityCase {
            evidence_ref_sha256: legacy.frame_id_sha256.clone(),
            request_text: "continue".to_owned(),
            provider_payload: serde_json::json!({"input":[]}),
            expected_response: serde_json::json!({
                "name": "write_stdin",
                "arguments": {
                    "session_id": 42,
                    "chars": "",
                    "yield_time_ms": 1_000,
                    "max_output_tokens": 5_000
                }
            })
            .to_string(),
        };
        let enriched = action_schema_enriched_frame(&legacy, Some(&parity));

        assert!(enriched.atoms.iter().any(|atom| matches!(
            atom,
            RelationAtom::ActionRoleArgument {
                name,
                value_type: Some(crate::AtomValueType::Integer),
                ..
            } if name == "session_id"
        )));
        assert!(enriched.atoms.iter().any(|atom| matches!(
            atom,
            RelationAtom::ActionStringArgument { name, value }
                if name == "chars" && value.is_empty()
        )));
        assert_ne!(
            teacher_program_signature(&legacy),
            teacher_program_signature(&enriched)
        );
    }

    #[test]
    fn normalized_teacher_pool_keeps_content_addressed_training_rows() {
        let first = frame(vec![
            RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            RelationAtom::ActionIntegerArgument {
                name: "yield_time_ms".to_owned(),
                value: 1_000,
            },
        ]);
        let second = frame(vec![
            RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            RelationAtom::ActionIntegerArgument {
                name: "yield_time_ms".to_owned(),
                value: 30_000,
            },
        ]);
        assert_eq!(
            teacher_program_signature(&first),
            teacher_program_signature(&second)
        );

        let first = teacher_transition_from_completed(&first, None)
            .expect("first transition")
            .as_training_relation_frame();
        let second = teacher_transition_from_completed(&second, None)
            .expect("second transition")
            .as_training_relation_frame();
        assert_ne!(first.frame_id_sha256, second.frame_id_sha256);
    }

    #[test]
    fn action_unique_slot_is_not_teacher_semantics() {
        let action = teacher_action_ast(&frame(vec![
            RelationAtom::ActionFunction {
                value: "wait".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: crate::AtomValueType::Identifier,
                source: crate::AtomSource::Action,
                value_sha256: "6".repeat(64),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 7,
            },
        ]))
        .expect("teacher action");
        assert!(
            action
                .atoms
                .iter()
                .all(|atom| !matches!(atom, RelationAtom::UniqueSlot { .. }))
        );
        assert!(
            action
                .atoms
                .iter()
                .any(|atom| matches!(atom, RelationAtom::SlotEquality { .. }))
        );
    }

    fn continuation_frame(custom_transport: bool, terminate: bool) -> RelationFrame {
        let mut atoms = vec![
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: crate::AtomValueType::Identifier,
                source: crate::AtomSource::Observation,
                value_sha256: "6".repeat(64),
            },
            RelationAtom::TypedSlot {
                slot_id: 2,
                value_type: crate::AtomValueType::Identifier,
                source: crate::AtomSource::Action,
                value_sha256: "6".repeat(64),
            },
            RelationAtom::ObservationSelector {
                slot_id: 1,
                selector: crate::ResponseValueSelector::ContentLinePrefix {
                    prefix: "Script running with cell ID ".to_owned(),
                    value_type: crate::AtomValueType::Identifier,
                },
            },
            RelationAtom::UniqueSlot { slot_id: 1 },
            RelationAtom::SlotEquality {
                left_slot: 1,
                right_slot: 2,
            },
            RelationAtom::CompletionState {
                value: "pending".to_owned(),
            },
        ];
        if custom_transport {
            atoms.extend([
                RelationAtom::ActionCustomTool {
                    value: "exec".to_owned(),
                },
                RelationAtom::ActionInnerTool {
                    value: "write_stdin".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "session_id".to_owned(),
                    slot_id: 2,
                    value_type: None,
                },
                RelationAtom::ActionStringArgument {
                    name: "chars".to_owned(),
                    value: String::new(),
                },
            ]);
        } else {
            atoms.extend([
                RelationAtom::ActionFunction {
                    value: "wait".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "cell_id".to_owned(),
                    slot_id: 2,
                    value_type: None,
                },
            ]);
        }
        if terminate {
            atoms.push(RelationAtom::ActionBooleanArgument {
                name: "terminate".to_owned(),
                value: true,
            });
        }
        frame(atoms)
    }

    #[test]
    fn continuation_transport_alias_is_transfer_but_terminate_is_negative() {
        let function_wait = continuation_frame(false, false);
        let custom_wait = continuation_frame(true, false);
        let terminate = continuation_frame(false, true);

        assert!(teacher_actions_have_compatible_effect(
            &function_wait,
            &custom_wait
        ));
        assert!(!teacher_actions_have_compatible_effect(
            &function_wait,
            &terminate
        ));
    }
}
