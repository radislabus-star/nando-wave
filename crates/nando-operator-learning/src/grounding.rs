use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use nando_operator_kernel::{
    AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, ROLE_HYPOTHESIS_SCHEMA, RelationAtom,
    RelationFrame, RoleHypothesis, SemanticRole, canonical_response_value_selector,
    selector_denotes_continuation_handle,
};

pub const SOURCE_NEUTRAL_EXTRACTOR_VERSION: &str = "response-relation-extractor.v16";
const CAPABILITY_EXTRACTOR_VERSION: &str = "response-relation-extractor.v15";
const TURN_REPLAY_EXTRACTOR_VERSION: &str = "response-relation-extractor.v14";
const ACTIVE_TURN_EXTRACTOR_VERSION: &str = "response-relation-extractor.v13";
const TURN_UNIQUE_EXTRACTOR_VERSION: &str = "response-relation-extractor.v12";
const BASE_SOURCE_NEUTRAL_EXTRACTOR_VERSION: &str = "response-relation-extractor.v7";
const PREVIOUS_SOURCE_NEUTRAL_EXTRACTOR_VERSION: &str = "response-relation-extractor.v6";
const OLDEST_SOURCE_NEUTRAL_EXTRACTOR_VERSION: &str = "response-relation-extractor.v5";
const CONCURRENCY_READINESS_EXTRACTOR_VERSION: &str = "response-relation-extractor.v8";
const WAIT_CHAIN_CONTINUITY_EXTRACTOR_VERSION: &str = "response-relation-extractor.v9";
const CANONICAL_WAIT_POLICY_EXTRACTOR_VERSION: &str = "response-relation-extractor.v10";

#[must_use]
pub fn is_source_neutral_relation_frame(frame: &RelationFrame) -> bool {
    frame.schema == RELATION_FRAME_SCHEMA
        && matches!(
            frame.extractor_version.as_str(),
            SOURCE_NEUTRAL_EXTRACTOR_VERSION
                | CAPABILITY_EXTRACTOR_VERSION
                | TURN_REPLAY_EXTRACTOR_VERSION
                | ACTIVE_TURN_EXTRACTOR_VERSION
                | TURN_UNIQUE_EXTRACTOR_VERSION
                | BASE_SOURCE_NEUTRAL_EXTRACTOR_VERSION
                | PREVIOUS_SOURCE_NEUTRAL_EXTRACTOR_VERSION
                | OLDEST_SOURCE_NEUTRAL_EXTRACTOR_VERSION
                | CONCURRENCY_READINESS_EXTRACTOR_VERSION
                | WAIT_CHAIN_CONTINUITY_EXTRACTOR_VERSION
                | CANONICAL_WAIT_POLICY_EXTRACTOR_VERSION
                | "response-relation-extractor.v11"
        )
}

/// Action-neutral family identity available both during teacher training and
/// before a runtime decision is made.
#[must_use]
pub fn relation_frame_structural_family_id(frame: &RelationFrame) -> Option<u64> {
    is_source_neutral_relation_frame(frame).then(|| {
        preaction_unique_integer_family_id(frame).unwrap_or_else(|| family_id(&frame.atoms))
    })
}

#[must_use]
pub fn relation_frame_online_family_id(frame: &RelationFrame) -> Option<u64> {
    let representation_family = relation_frame_structural_family_id(frame)?;
    let material = (
        "response_online_family_generation",
        frame.extractor_version.as_str(),
        representation_family,
    );
    let digest = Sha256::digest(serde_json::to_vec(&material).unwrap_or_default());
    Some(u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8])))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TraceSlot {
    pub slot_id: u16,
    pub value_type: AtomValueType,
    pub source: AtomSource,
    pub value_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceNeutralTrace {
    pub event_id_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub observed_at_unix_nanos: u64,
    pub verifier_label: Option<bool>,
    pub evidence_ref_sha256: String,
    pub tool_kind: String,
    pub observation_call_shape: String,
    pub observation_selector: crate::ResponseValueSelector,
    pub completion_state: String,
    pub output_status: Option<String>,
    pub response_shape: String,
    pub slots: Vec<TraceSlot>,
    pub equal_slots: Vec<(u16, u16)>,
    pub unique_slots: Vec<u16>,
    pub action_function: Option<String>,
    pub action_role_arguments: Vec<(String, u16)>,
    pub action_integer_arguments: Vec<(String, u64)>,
}

pub fn extract_relation_frame(trace: &SourceNeutralTrace) -> RelationFrame {
    let mut atoms = Vec::new();
    atoms.push(RelationAtom::ToolKind {
        value: trace.tool_kind.clone(),
    });
    atoms.push(RelationAtom::ObservationCallShape {
        value: trace.observation_call_shape.clone(),
    });
    atoms.push(RelationAtom::CompletionState {
        value: trace.completion_state.clone(),
    });
    if let Some(status) = &trace.output_status {
        atoms.push(RelationAtom::OutputStatus {
            value: status.clone(),
        });
    }
    atoms.push(RelationAtom::ResponseShape {
        value: trace.response_shape.clone(),
    });
    atoms.extend(trace.slots.iter().map(|slot| RelationAtom::TypedSlot {
        slot_id: slot.slot_id,
        value_type: slot.value_type,
        source: slot.source,
        value_sha256: slot.value_sha256.clone(),
    }));
    if let Some(slot_id) = trace
        .slots
        .iter()
        .find(|slot| slot.source == AtomSource::Observation)
        .map(|slot| slot.slot_id)
    {
        atoms.push(RelationAtom::ObservationSelector {
            slot_id,
            selector: trace.observation_selector.clone(),
        });
    }
    atoms.extend(trace.equal_slots.iter().map(|(left_slot, right_slot)| {
        RelationAtom::SlotEquality {
            left_slot: *left_slot,
            right_slot: *right_slot,
        }
    }));
    atoms.extend(
        trace
            .unique_slots
            .iter()
            .map(|slot_id| RelationAtom::UniqueSlot { slot_id: *slot_id }),
    );
    if let Some(value) = &trace.action_function {
        atoms.push(RelationAtom::ActionFunction {
            value: value.clone(),
        });
    }
    atoms.extend(trace.action_role_arguments.iter().map(|(name, slot_id)| {
        RelationAtom::ActionRoleArgument {
            name: name.clone(),
            slot_id: *slot_id,
            value_type: None,
        }
    }));
    atoms.extend(trace.action_integer_arguments.iter().map(|(name, value)| {
        RelationAtom::ActionIntegerArgument {
            name: name.clone(),
            value: *value,
        }
    }));
    let frame_id_sha256 = digest_json(&atoms);
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256,
        event_id_sha256: trace.event_id_sha256.clone(),
        client_intent_id_sha256: trace.client_intent_id_sha256.clone(),
        session_id_sha256: trace.session_id_sha256.clone(),
        observed_at_unix_nanos: trace.observed_at_unix_nanos,
        estimated_input_tokens: 0,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: trace.verifier_label,
        atoms,
        evidence_ref_sha256: trace.evidence_ref_sha256.clone(),
    }
}

pub fn ground_roles(frame: &RelationFrame) -> Vec<RoleHypothesis> {
    if !is_source_neutral_relation_frame(frame) {
        return Vec::new();
    }
    let Some(structural_family_id) = relation_frame_structural_family_id(frame) else {
        return Vec::new();
    };
    let slots = frame
        .atoms
        .iter()
        .enumerate()
        .filter_map(|(index, atom)| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                value_type,
                source,
                ..
            } => Some((*slot_id, (index, *value_type, *source))),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    if frame.atoms.iter().any(|atom| match atom {
        RelationAtom::ObservationSelector { slot_id, .. } => slots
            .get(slot_id)
            .is_none_or(|(_, _, source)| *source != AtomSource::Observation),
        RelationAtom::ActionRoleArgument { slot_id, .. } => slots
            .get(slot_id)
            .is_none_or(|(_, _, source)| *source != AtomSource::Action),
        _ => false,
    }) {
        return Vec::new();
    }
    if frame
        .atoms
        .iter()
        .any(|atom| matches!(atom, RelationAtom::ActionStatusProjection { .. }))
    {
        return ground_status_role(frame, &slots).into_iter().collect();
    }
    let pending = frame.atoms.iter().any(|atom| {
        matches!(atom, RelationAtom::CompletionState { value } if value == "pending")
            || matches!(
                atom,
                RelationAtom::ObservationSelector { selector, .. }
                    if selector_denotes_continuation_handle(selector)
            )
    });
    let mut hypotheses = Vec::new();
    for atom in &frame.atoms {
        let RelationAtom::SlotEquality {
            left_slot,
            right_slot,
        } = atom
        else {
            continue;
        };
        let (
            Some((left_index, left_type, left_source)),
            Some((right_index, right_type, right_source)),
        ) = (slots.get(left_slot), slots.get(right_slot))
        else {
            continue;
        };
        let (observation_index, observation_type, observation_slot, action_index) =
            match (*left_source, *right_source) {
                (AtomSource::Observation, AtomSource::Action) => {
                    (*left_index, *left_type, *left_slot, *right_index)
                }
                (AtomSource::Action, AtomSource::Observation) => {
                    (*right_index, *right_type, *right_slot, *left_index)
                }
                _ => continue,
            };
        if pending
            && observation_type != AtomValueType::Collection
            && left_type == right_type
        {
            let bindings = BTreeMap::from([
                (SemanticRole::ContinuationHandle, observation_index),
                (SemanticRole::TargetValue, action_index),
            ]);
            let material = serde_json::to_vec(&bindings).unwrap_or_default();
            hypotheses.push(RoleHypothesis {
                schema: ROLE_HYPOTHESIS_SCHEMA.to_owned(),
                hypothesis_id_sha256: digest_bytes(&material),
                frame_family_id: structural_family_id,
                competing_binding_count: 0,
                description_length_bytes: material.len(),
                bindings,
            });
        } else if !pending
            && left_type == right_type
            && frame.atoms.iter().any(
                |atom| matches!(atom, RelationAtom::UniqueSlot { slot_id } if slot_id == &observation_slot),
            )
        {
            let bindings = BTreeMap::from([
                (SemanticRole::SourceValue, observation_index),
                (SemanticRole::TargetValue, action_index),
            ]);
            let material = serde_json::to_vec(&bindings).unwrap_or_default();
            hypotheses.push(RoleHypothesis {
                schema: ROLE_HYPOTHESIS_SCHEMA.to_owned(),
                hypothesis_id_sha256: digest_bytes(&material),
                frame_family_id: structural_family_id,
                competing_binding_count: 0,
                description_length_bytes: material.len(),
                bindings,
            });
        }
    }
    let competing = hypotheses.len().saturating_sub(1);
    for hypothesis in &mut hypotheses {
        hypothesis.competing_binding_count = competing;
    }
    hypotheses
}

fn ground_status_role(
    frame: &RelationFrame,
    slots: &BTreeMap<u16, (usize, AtomValueType, AtomSource)>,
) -> Option<RoleHypothesis> {
    let mut projections = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ActionStatusProjection { mapping } => Some(*mapping),
        _ => None,
    });
    let _mapping = projections.next()?;
    if projections.next().is_some()
        || frame.atoms.iter().any(|atom| {
            matches!(
                atom,
                RelationAtom::ActionFunction { .. }
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
            )
        })
    {
        return None;
    }
    let mut selectors = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector { slot_id, selector } => Some((*slot_id, selector)),
        _ => None,
    });
    let (slot_id, selector) = selectors.next()?;
    if selectors.next().is_some() || selector_value_type(selector) != AtomValueType::Integer {
        return None;
    }
    let (source_index, value_type, source) = slots.get(&slot_id)?;
    if *value_type != AtomValueType::Integer
        || *source != AtomSource::Observation
        || frame
            .atoms
            .iter()
            .filter(|atom| {
                matches!(atom, RelationAtom::TypedSlot { slot_id: candidate, .. } if *candidate == slot_id)
            })
            .count()
            != 1
        || frame
            .atoms
            .iter()
            .filter(|atom| {
                matches!(atom, RelationAtom::UniqueSlot { slot_id: candidate } if *candidate == slot_id)
            })
            .count()
            != 1
    {
        return None;
    }
    let mut completion_states = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::CompletionState { value }
            if matches!(value.as_str(), "pending" | "completed") =>
        {
            Some(value.as_str())
        }
        RelationAtom::CompletionState { .. } => None,
        _ => None,
    });
    let _completion_state = completion_states.next()?;
    if completion_states.next().is_some()
        || frame
            .atoms
            .iter()
            .filter(|atom| matches!(atom, RelationAtom::CompletionState { .. }))
            .count()
            != 1
    {
        return None;
    }
    let bindings = BTreeMap::from([(SemanticRole::StatusOrResult, *source_index)]);
    let material = serde_json::to_vec(&bindings).unwrap_or_default();
    Some(RoleHypothesis {
        schema: ROLE_HYPOTHESIS_SCHEMA.to_owned(),
        hypothesis_id_sha256: digest_bytes(&material),
        frame_family_id: relation_frame_structural_family_id(frame)?,
        competing_binding_count: 0,
        description_length_bytes: material.len(),
        bindings,
    })
}

const fn selector_value_type(selector: &crate::ResponseValueSelector) -> AtomValueType {
    match selector {
        crate::ResponseValueSelector::ContinuationHandle { value_type }
        | crate::ResponseValueSelector::UniqueScalar { value_type }
        | crate::ResponseValueSelector::UniqueTurnScalar { value_type }
        | crate::ResponseValueSelector::ContentLinePrefix { value_type, .. }
        | crate::ResponseValueSelector::JsonField { value_type, .. }
        | crate::ResponseValueSelector::JsonScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::UniqueTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::UniqueActiveTurnJsonField { value_type, .. }
        | crate::ResponseValueSelector::RequestReferencedJsonField { value_type }
        | crate::ResponseValueSelector::RequestReferencedJsonFieldOrdinal { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::TurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputLine { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarOrdinal { value_type, .. }
        | crate::ResponseValueSelector::LatestTurnOutputScalarFromEnd { value_type, .. } => {
            *value_type
        }
        crate::ResponseValueSelector::CommandOutputBody
        | crate::ResponseValueSelector::RequestLastToken
        | crate::ResponseValueSelector::RequestUniqueLiteral => AtomValueType::String,
    }
}

fn family_id(atoms: &[RelationAtom]) -> u64 {
    let slot_sources = atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id, source, ..
            } => Some((*slot_id, *source)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let is_pre_action_slot = |slot_id: &u16| {
        slot_sources
            .get(slot_id)
            .is_some_and(|source| matches!(source, AtomSource::Request | AtomSource::Observation))
    };
    let mut structural = atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                value_type, source, ..
            } if matches!(source, AtomSource::Request | AtomSource::Observation) => {
                Some(format!("slot:{value_type:?}:{source:?}"))
            }
            RelationAtom::TypedSlot { .. } => None,
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } if is_pre_action_slot(left_slot) && is_pre_action_slot(right_slot) => {
                Some("slot_equality".to_owned())
            }
            RelationAtom::SlotEquality { .. } => None,
            RelationAtom::UniqueSlot { slot_id } if is_pre_action_slot(slot_id) => {
                Some("unique_slot".to_owned())
            }
            RelationAtom::UniqueSlot { .. } => None,
            RelationAtom::ObservationSelector { slot_id, selector }
                if is_pre_action_slot(slot_id) =>
            {
                Some(format!(
                    "observation_selector:{}",
                    canonical_response_value_selector(selector)
                ))
            }
            RelationAtom::ObservationSelector { .. } => None,
            RelationAtom::ObservationCallShape { value } => {
                Some(format!("observation_call_shape:{value}"))
            }
            RelationAtom::ActionFunction { .. }
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
            | RelationAtom::ActionPlanAdvance
            | RelationAtom::ResponseShape { .. } => None,
            RelationAtom::PlanState {
                step_count,
                completed_count,
                active_index,
            } => Some(format!(
                "plan_state:{step_count}:{completed_count}:{active_index}"
            )),
            RelationAtom::CollectionShape {
                array_fields,
                row_fields,
            } => Some(format!("collection_shape:{array_fields}:{row_fields}")),
            RelationAtom::RequestPhaseAtom { .. } => Some("request_phase_atom".to_owned()),
            RelationAtom::ClientCapabilityAtom { atom_id } => {
                Some(format!("client_capability:{atom_id}"))
            }
            RelationAtom::ReconstructedClientCapabilityAtom { atom_id } => {
                Some(format!("client_capability:{atom_id}"))
            }
            RelationAtom::ToolKind { .. } => Some("tool_kind".to_owned()),
            RelationAtom::OutputStatus { value } => Some(format!("status:{value}")),
            RelationAtom::TypedEquality { .. } => Some("typed_equality".to_owned()),
            RelationAtom::Cardinality { role, .. } => Some(format!("cardinality:{role}")),
            RelationAtom::TemporalEdge { .. } => Some("temporal_edge".to_owned()),
            RelationAtom::CompletionState { value } => Some(format!("completion:{value}")),
        })
        .collect::<Vec<_>>();
    structural.sort();
    let digest = Sha256::digest(serde_json::to_vec(&structural).unwrap_or_default());
    u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]))
}

fn preaction_unique_integer_family_id(frame: &RelationFrame) -> Option<u64> {
    let mut selectors = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::ObservationSelector { slot_id, selector }
            if selector_value_type(selector) == AtomValueType::Integer =>
        {
            Some((*slot_id, selector))
        }
        _ => None,
    });
    let (slot_id, selector) = selectors.next()?;
    if selectors.next().is_some()
        || frame
            .atoms
            .iter()
            .filter(|atom| matches!(atom, RelationAtom::UniqueSlot { slot_id: candidate } if *candidate == slot_id))
            .count()
            != 1
    {
        return None;
    }
    let mut completion_states = frame.atoms.iter().filter_map(|atom| match atom {
        RelationAtom::CompletionState { value } => Some(value.as_str()),
        _ => None,
    });
    let completion_state = completion_states.next()?;
    if completion_states.next().is_some() {
        return None;
    }
    let material = (
        "unique_integer_observation",
        canonical_response_value_selector(selector),
        completion_state,
    );
    let digest = Sha256::digest(serde_json::to_vec(&material).unwrap_or_default());
    Some(u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8])))
}

fn digest_json<T: Serialize>(value: &T) -> String {
    digest_bytes(&serde_json::to_vec(value).unwrap_or_default())
}

fn digest_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

include!("grounding_tests.rs");
