use std::collections::BTreeSet;

use crate::{
    AtomSource, AtomValueType, RelationAtom, RelationFrame, ResponseArgument, ResponseOperation,
    ResponseProgram, SemanticRole, selector_phase_atom_id, stable_atom_id, stable_atom_id_parts,
};

pub fn relation_frame_phase_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = frame
        .atoms
        .iter()
        .map(relation_atom_phase_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[doc(hidden)]
pub fn relation_atom_phase_id(atom: &RelationAtom) -> u64 {
    match atom {
        RelationAtom::TypedSlot {
            value_type, source, ..
        } => stable_atom_id_parts(&[
            "slot:",
            value_type_name(*value_type),
            ":",
            source_name(*source),
        ]),
        RelationAtom::SlotEquality { .. } => stable_atom_id("relation:slot_equality"),
        RelationAtom::UniqueSlot { .. } => stable_atom_id("relation:unique_slot"),
        RelationAtom::ObservationSelector { selector, .. } => selector_phase_atom_id(selector),
        RelationAtom::ObservationCallShape { value } => {
            stable_atom_id_parts(&["observation_call_shape:", value])
        }
        RelationAtom::ActionFunction { value } => {
            stable_atom_id_parts(&["action_function:", value])
        }
        RelationAtom::ActionCustomTool { value } => {
            stable_atom_id_parts(&["action_custom_tool:", value])
        }
        RelationAtom::ActionInnerTool { value } => {
            stable_atom_id_parts(&["action_inner_tool:", value])
        }
        RelationAtom::ActionRoleArgument { name, .. } => {
            stable_atom_id_parts(&["action_role_argument:", name])
        }
        RelationAtom::ActionIntegerArgument { name, .. } => {
            stable_atom_id_parts(&["action_integer_argument:", name])
        }
        RelationAtom::ActionStringArgument { name, .. } => {
            stable_atom_id_parts(&["action_string_argument:", name])
        }
        RelationAtom::ActionBooleanArgument { name, .. } => {
            stable_atom_id_parts(&["action_boolean_argument:", name])
        }
        RelationAtom::PlanState { .. } => stable_atom_id("relation:plan_state"),
        RelationAtom::ActionPlanAdvance => stable_atom_id("action_plan_advance"),
        RelationAtom::ActionResultProjection {
            output_field,
            continuation_field,
            continuation_prefix,
        } => stable_atom_id_parts(&[
            "action_result_projection:",
            output_field,
            ":",
            continuation_field,
            ":",
            continuation_prefix,
        ]),
        RelationAtom::ActionOutputProjection { output_field } => {
            stable_atom_id_parts(&["action_output_projection:", output_field])
        }
        RelationAtom::ActionJsonResultProjection => stable_atom_id("action_json_result_projection"),
        RelationAtom::ActionValueProjection { format, renderer } => stable_atom_id(&format!(
            "action_value_projection:{}:{}",
            match format {
                crate::ValueProjectionFormat::PlainText => "plain_text",
                crate::ValueProjectionFormat::CanonicalJson => "canonical_json",
            },
            serde_json::to_string(renderer).unwrap_or_default(),
        )),
        RelationAtom::ActionStatusProjection { mapping } => match mapping {
            crate::ProjectStatusMapping::ZeroIsSuccess => {
                stable_atom_id("action_status_projection:ZeroIsSuccess")
            }
            crate::ProjectStatusMapping::ZeroIsPass => {
                stable_atom_id("action_status_projection:ZeroIsPass")
            }
            crate::ProjectStatusMapping::ZeroIsOk => {
                stable_atom_id("action_status_projection:ZeroIsOk")
            }
            crate::ProjectStatusMapping::ZeroIsTrue => {
                stable_atom_id("action_status_projection:ZeroIsTrue")
            }
        },
        RelationAtom::CollectionShape { .. } => stable_atom_id("observation:json_collection"),
        RelationAtom::RequestPhaseAtom { atom_id } => *atom_id,
        RelationAtom::ClientCapabilityAtom { atom_id } => *atom_id,
        RelationAtom::ReconstructedClientCapabilityAtom { atom_id } => *atom_id,
        RelationAtom::ToolKind { .. } => stable_atom_id("relation:tool_kind"),
        RelationAtom::OutputStatus { value } => stable_atom_id_parts(&["status:", value]),
        RelationAtom::TypedEquality { .. } => stable_atom_id("relation:typed_equality"),
        RelationAtom::Cardinality { role, count } => {
            let count = count.to_string();
            stable_atom_id_parts(&["cardinality:", role, ":", &count])
        }
        RelationAtom::TemporalEdge { .. } => stable_atom_id("relation:temporal_edge"),
        RelationAtom::ResponseShape { value } => stable_atom_id_parts(&["shape:", value]),
        RelationAtom::CompletionState { value } => stable_atom_id_parts(&["completion:", value]),
    }
}

pub fn relation_frame_routing_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = frame
        .atoms
        .iter()
        .filter(|atom| {
            !matches!(
                atom,
                RelationAtom::TypedSlot {
                    source: AtomSource::Action | AtomSource::Outcome,
                    ..
                } | RelationAtom::SlotEquality { .. }
                    | RelationAtom::ActionFunction { .. }
                    | RelationAtom::ActionCustomTool { .. }
                    | RelationAtom::ActionInnerTool { .. }
                    | RelationAtom::ActionRoleArgument { .. }
                    | RelationAtom::ActionIntegerArgument { .. }
                    | RelationAtom::ActionStringArgument { .. }
                    | RelationAtom::ActionBooleanArgument { .. }
                    | RelationAtom::ActionResultProjection { .. }
                    | RelationAtom::ActionOutputProjection { .. }
                    | RelationAtom::ActionJsonResultProjection
                    | RelationAtom::ActionValueProjection { .. }
                    | RelationAtom::ActionStatusProjection { .. }
                    | RelationAtom::ReconstructedClientCapabilityAtom { .. }
                    | RelationAtom::ResponseShape { .. }
            )
        })
        .map(relation_atom_phase_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Online-learning namespace may preserve an observed protocol symbol while
/// the stable serving/package ABI continues to emit its legacy generic atom.
#[must_use]
pub fn relation_frame_online_routing_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    let mut ids = relation_frame_routing_atom_ids(frame);
    let continuation_pending = frame.atoms.iter().any(|atom| {
        matches!(
            atom,
            RelationAtom::ObservationSelector { selector, .. }
                if crate::contracts::selector_denotes_continuation_handle(selector)
        )
    });
    if continuation_pending {
        ids.retain(|atom| *atom != stable_atom_id("completion:completed"));
        ids.push(stable_atom_id("completion:pending"));
    }
    ids.extend(frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ToolKind { value } => Some(stable_atom_id_parts(&["tool_kind:", value])),
        _ => None,
    }));
    ids.extend(relation_frame_hidden_wave_atom_ids(frame));
    ids.sort_unstable();
    ids.dedup();
    ids
}

#[derive(Clone, Copy)]
enum WaveAtomLayer {
    Request,
    State,
    Tool,
}

#[doc(hidden)]
pub fn relation_frame_hidden_wave_atom_ids(frame: &RelationFrame) -> Vec<u64> {
    const BASIS_LIMIT: usize = 6;
    const LEGACY_HIDDEN_LIMIT: usize = 12;
    const BALANCED_HIDDEN_LIMIT: usize = 12;

    let pre_action_slots = frame
        .atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: AtomSource::Request | AtomSource::Observation,
                ..
            } => Some(*slot_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut request = Vec::new();
    let mut state = Vec::new();
    let mut tool = Vec::new();
    let mut ranked_request = Vec::new();
    let mut ranked_state = Vec::new();
    let mut ranked_tool = Vec::new();
    for atom in &frame.atoms {
        let layer = match atom {
            RelationAtom::TypedSlot {
                source: AtomSource::Request,
                ..
            }
            | RelationAtom::RequestPhaseAtom { .. }
            | RelationAtom::ClientCapabilityAtom { .. }
            | RelationAtom::ReconstructedClientCapabilityAtom { .. } => {
                Some(WaveAtomLayer::Request)
            }
            RelationAtom::TypedSlot {
                source: AtomSource::Observation,
                ..
            }
            | RelationAtom::ObservationCallShape { .. }
            | RelationAtom::CollectionShape { .. }
            | RelationAtom::ToolKind { .. }
            | RelationAtom::OutputStatus { .. } => Some(WaveAtomLayer::Tool),
            RelationAtom::ObservationSelector { slot_id, .. }
                if pre_action_slots.contains(slot_id) =>
            {
                Some(WaveAtomLayer::Tool)
            }
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } if pre_action_slots.contains(left_slot) && pre_action_slots.contains(right_slot) => {
                Some(WaveAtomLayer::State)
            }
            RelationAtom::UniqueSlot { slot_id } if pre_action_slots.contains(slot_id) => {
                Some(WaveAtomLayer::State)
            }
            RelationAtom::TypedEquality { .. }
            | RelationAtom::Cardinality { .. }
            | RelationAtom::TemporalEdge { .. }
            | RelationAtom::CompletionState { .. } => Some(WaveAtomLayer::State),
            _ => None,
        };
        let Some(layer) = layer else {
            continue;
        };
        let id = relation_atom_phase_id(atom);
        let ranked = (wave_hidden_source_priority(atom), id);
        match layer {
            WaveAtomLayer::Request => {
                request.push(id);
                ranked_request.push(ranked);
            }
            WaveAtomLayer::State => {
                state.push(id);
                ranked_state.push(ranked);
            }
            WaveAtomLayer::Tool => {
                tool.push(id);
                ranked_tool.push(ranked);
            }
        }
    }
    for basis in [&mut request, &mut state, &mut tool] {
        basis.sort_unstable();
        basis.dedup();
        basis.truncate(BASIS_LIMIT);
    }
    for basis in [&mut ranked_request, &mut ranked_state, &mut ranked_tool] {
        basis.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        basis.dedup_by_key(|entry| entry.1);
        basis.truncate(BASIS_LIMIT);
    }

    // Keep the original hash-selected atoms so already compiled packages keep
    // exactly the routing vocabulary they were trained with.
    let mut hidden = Vec::with_capacity(LEGACY_HIDDEN_LIMIT + BALANCED_HIDDEN_LIMIT);
    extend_wave_pairs(&mut hidden, 1, &request, &state);
    extend_wave_pairs(&mut hidden, 2, &state, &tool);
    extend_wave_pairs(&mut hidden, 3, &request, &tool);
    for request_id in &request {
        for state_id in &state {
            for tool_id in &tool {
                hidden.push(wave_hidden_atom_id(4, &[*request_id, *state_id, *tool_id]));
            }
        }
    }
    hidden.sort_unstable();
    hidden.dedup();
    hidden.truncate(LEGACY_HIDDEN_LIMIT);

    let balanced = balanced_wave_hidden_atom_ids(
        &ranked_request,
        &ranked_state,
        &ranked_tool,
        BALANCED_HIDDEN_LIMIT,
    );
    hidden.extend(balanced);
    hidden.sort_unstable();
    hidden.dedup();
    hidden
}

#[doc(hidden)]
pub fn balanced_wave_hidden_atom_ids(
    request: &[(u16, u64)],
    state: &[(u16, u64)],
    tool: &[(u16, u64)],
    limit: usize,
) -> Vec<u64> {
    let mut groups = [
        ranked_wave_pairs(1, request, state),
        ranked_wave_pairs(2, state, tool),
        ranked_wave_pairs(3, request, tool),
        ranked_wave_triples(request, state, tool),
    ];
    for group in &mut groups {
        group.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        group.dedup_by_key(|entry| entry.1);
    }
    let mut output = Vec::with_capacity(limit);
    let mut seen = BTreeSet::new();
    for rank in 0..groups.iter().map(Vec::len).max().unwrap_or(0) {
        for group in &groups {
            let Some((_, atom_id)) = group.get(rank) else {
                continue;
            };
            if seen.insert(*atom_id) {
                output.push(*atom_id);
                if output.len() == limit {
                    return output;
                }
            }
        }
    }
    output
}

#[doc(hidden)]
pub fn ranked_wave_pairs(kind: u64, left: &[(u16, u64)], right: &[(u16, u64)]) -> Vec<(u32, u64)> {
    left.iter()
        .flat_map(|(left_priority, left_id)| {
            right.iter().map(move |(right_priority, right_id)| {
                (
                    u32::from(*left_priority).saturating_add(u32::from(*right_priority)),
                    wave_hidden_atom_id(kind, &[*left_id, *right_id]),
                )
            })
        })
        .collect()
}

#[doc(hidden)]
pub fn ranked_wave_triples(
    request: &[(u16, u64)],
    state: &[(u16, u64)],
    tool: &[(u16, u64)],
) -> Vec<(u32, u64)> {
    let mut output = Vec::new();
    for (request_priority, request_id) in request {
        for (state_priority, state_id) in state {
            for (tool_priority, tool_id) in tool {
                output.push((
                    u32::from(*request_priority)
                        .saturating_add(u32::from(*state_priority))
                        .saturating_add(u32::from(*tool_priority)),
                    wave_hidden_atom_id(4, &[*request_id, *state_id, *tool_id]),
                ));
            }
        }
    }
    output
}

fn wave_hidden_source_priority(atom: &RelationAtom) -> u16 {
    match atom {
        RelationAtom::CompletionState { .. } | RelationAtom::OutputStatus { .. } => 1_000,
        RelationAtom::ObservationSelector { .. } | RelationAtom::Cardinality { .. } => 900,
        RelationAtom::TypedSlot { .. }
        | RelationAtom::SlotEquality { .. }
        | RelationAtom::UniqueSlot { .. } => 800,
        RelationAtom::ToolKind { .. }
        | RelationAtom::ObservationCallShape { .. }
        | RelationAtom::TypedEquality { .. }
        | RelationAtom::TemporalEdge { .. } => 700,
        RelationAtom::CollectionShape { .. } | RelationAtom::RequestPhaseAtom { .. } => 600,
        RelationAtom::ClientCapabilityAtom { .. }
        | RelationAtom::ReconstructedClientCapabilityAtom { .. } => 500,
        _ => 0,
    }
}

fn extend_wave_pairs(out: &mut Vec<u64>, kind: u64, left: &[u64], right: &[u64]) {
    for left_id in left {
        for right_id in right {
            out.push(wave_hidden_atom_id(kind, &[*left_id, *right_id]));
        }
    }
}

fn wave_hidden_atom_id(kind: u64, parts: &[u64]) -> u64 {
    let mut hash = 0x6e61_6e64_6f77_6176_u64;
    for byte in kind
        .to_le_bytes()
        .into_iter()
        .chain(parts.iter().flat_map(|part| part.to_le_bytes()))
    {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    hash
}

const fn value_type_name(value: AtomValueType) -> &'static str {
    match value {
        AtomValueType::String => "string",
        AtomValueType::Integer => "integer",
        AtomValueType::Boolean => "boolean",
        AtomValueType::Identifier => "identifier",
        AtomValueType::Collection => "collection",
    }
}

const fn source_name(source: AtomSource) -> &'static str {
    match source {
        AtomSource::Request => "request",
        AtomSource::Observation => "observation",
        AtomSource::Action => "action",
        AtomSource::Outcome => "outcome",
    }
}
#[must_use]
pub fn response_program_required_routing_atom_ids(program: &ResponseProgram) -> Vec<u64> {
    let mut atoms = match &program.operation {
        ResponseOperation::UniqueConsensus { variants, .. } => {
            let mut variants = variants.iter();
            let Some(first) = variants.next() else {
                return Vec::new();
            };
            let mut common = response_program_required_routing_atom_ids(&first.program)
                .into_iter()
                .collect::<BTreeSet<_>>();
            for variant in variants {
                let atoms = response_program_required_routing_atom_ids(&variant.program)
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                common.retain(|atom| atoms.contains(atom));
            }
            common.into_iter().collect()
        }
        ResponseOperation::AdvancePlan { function_name } => vec![
            stable_atom_id("relation:plan_state"),
            stable_atom_id("status:success"),
            stable_atom_id(&format!("client_capability:function:{function_name}")),
        ],
        ResponseOperation::FunctionCallFromRoles {
            selector,
            arguments,
            ..
        } => {
            let completion = if arguments.iter().any(|argument| {
                matches!(
                    argument,
                    ResponseArgument::Role {
                        role: SemanticRole::ContinuationHandle,
                        ..
                    }
                )
            }) {
                "pending"
            } else {
                "completed"
            };
            vec![
                stable_atom_id(&format!("completion:{completion}")),
                stable_atom_id("relation:unique_slot"),
                selector_phase_atom_id(selector),
            ]
        }
        ResponseOperation::CustomToolCallFromRoles {
            custom_tool_name, ..
        } => vec![
            stable_atom_id("completion:pending"),
            stable_atom_id("relation:unique_slot"),
            stable_atom_id(&format!("client_capability:custom:{custom_tool_name}")),
        ],
        ResponseOperation::ProjectSelectedValue {
            selector,
            completion_state,
            ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("relation:unique_slot"),
            selector_phase_atom_id(selector),
        ],
        ResponseOperation::ProjectStatus {
            selector,
            completion_state,
            ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("relation:unique_slot"),
            selector_phase_atom_id(selector),
        ],
        ResponseOperation::ComposeCollection {
            completion_state, ..
        } => vec![
            stable_atom_id(&format!("completion:{completion_state}")),
            stable_atom_id("observation:json_collection"),
        ],
        _ => Vec::new(),
    };
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}
