use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomValueType, MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1,
    PreActionMultiSourceTopologyV1, RelationAtom, RelationFrame, canonical_json_sha256,
    valid_nonzero_sha256,
};
use serde::{Deserialize, Serialize};

use crate::{teacher_action_ast, teacher_outcome_from_completed, teacher_semantic_law_signature};

use super::PreActionTopologyAuditRowV1;

pub const BLIND_THEN_REVEAL_JOIN_SCHEMA_V1: &str = "nando.multi-source-blind-then-reveal-join.v1";
pub const MULTI_SOURCE_JOIN_MAX_ROWS_V1: usize = 16_384;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSourceJoinCensoredReasonV1 {
    LegacyOrderingUnproven,
    ProviderIdentityUnproven,
    TopologyCensored,
    MissingTeacherAction,
    MissingVerifierReceipt,
    IdentityMismatch,
    TokenCountMismatch,
    PreActionOrderInvalid,
    AmbiguousPreActionMatch,
    DuplicateConflict,
    CapacityExhausted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedTeacherActionRefV1 {
    pub turn_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub action_event_id_sha256: String,
    pub completed_frame_root_sha256: String,
    pub physical_action_root_sha256: String,
    pub semantic_action_root_sha256: String,
    pub effect_atoms: Vec<CompletedEffectAtomV1>,
    pub observed_at_unix_nanos: u64,
    pub estimated_input_tokens: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletedEffectAtomV1 {
    RoleInput,
    RoleInputSlot {
        slot_id: u16,
        value_type: Option<AtomValueType>,
    },
    IntegerConstant,
    StringConstant,
    BooleanConstant,
    ResultProjection,
    OutputProjection,
    JsonResultProjection,
    ValueProjection,
    StatusProjection,
    PlanAdvance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedOutcomeReceiptRefV1 {
    pub turn_intent_id_sha256: String,
    pub action_event_id_sha256: String,
    pub completed_frame_root_sha256: String,
    pub physical_action_root_sha256: String,
    pub verifier_evidence_root_sha256: String,
    pub verified_output_root_sha256: String,
    pub verifier_receipt_root_sha256: String,
    pub accepted: bool,
    pub verified_at_unix_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlindThenRevealJoinedTransitionV1 {
    pub schema: String,
    pub join_root_sha256: String,
    pub capture_sequence: u64,
    pub turn_intent_id_sha256: String,
    pub request_event_id_sha256: String,
    pub action_event_id_sha256: String,
    pub session_lineage_sha256: String,
    pub session_id_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub pre_action_record_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub physical_action_root_sha256: String,
    pub semantic_action_root_sha256: String,
    pub effect_atoms: Vec<CompletedEffectAtomV1>,
    pub verifier_receipt_root_sha256: String,
    pub input_tokens: u64,
    pub captured_at_unix_ms: u64,
    pub completed_at_unix_nanos: u64,
    pub accepted: bool,
    pub topology: PreActionMultiSourceTopologyV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceJoinReportV1 {
    pub topology_rows: u64,
    pub completed_frames: u64,
    pub joined_rows: u64,
    pub accepted_rows: u64,
    pub negative_rows: u64,
    pub duplicate_idempotent: u64,
    pub censored: BTreeMap<MultiSourceJoinCensoredReasonV1, u64>,
    pub authority_ready: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct MultiSourceJoinLedgerV1 {
    joined_by_root: BTreeMap<String, BlindThenRevealJoinedTransitionV1>,
    used_completed_frames: BTreeSet<String>,
    report: MultiSourceJoinReportV1,
}

#[derive(Serialize)]
struct JoinedDigest<'a> {
    schema: &'a str,
    capture_sequence: u64,
    turn_intent_id_sha256: &'a str,
    request_event_id_sha256: &'a str,
    action_event_id_sha256: &'a str,
    session_lineage_sha256: &'a str,
    session_id_sha256: &'a str,
    topology_commitment_root_sha256: &'a str,
    pre_action_record_root_sha256: &'a str,
    completed_frame_root_sha256: &'a str,
    physical_action_root_sha256: &'a str,
    semantic_action_root_sha256: &'a str,
    effect_atoms: &'a [CompletedEffectAtomV1],
    verifier_receipt_root_sha256: &'a str,
    input_tokens: u64,
    captured_at_unix_ms: u64,
    completed_at_unix_nanos: u64,
    accepted: bool,
    topology: &'a PreActionMultiSourceTopologyV1,
}

impl MultiSourceJoinLedgerV1 {
    #[must_use]
    pub fn build(topologies: &[PreActionTopologyAuditRowV1], frames: &[RelationFrame]) -> Self {
        let mut ledger = Self::default();
        ledger.report.topology_rows = u64::try_from(topologies.len()).unwrap_or(u64::MAX);
        ledger.report.completed_frames = u64::try_from(frames.len()).unwrap_or(u64::MAX);

        let mut eligible = Vec::new();
        for row in topologies {
            match validate_topology_row(row) {
                Ok(()) => eligible.push(row),
                Err(reason) => ledger.censor(reason),
            }
        }
        eligible.sort_by_key(|row| {
            (
                row.structure.turn_intent_id_sha256.as_str(),
                row.captured_at_unix_ms.unwrap_or(0),
                row.commit.commitment_root_sha256.as_str(),
            )
        });
        let mut eligible_by_intent = BTreeMap::<&str, Vec<&PreActionTopologyAuditRowV1>>::new();
        for row in &eligible {
            eligible_by_intent
                .entry(row.structure.turn_intent_id_sha256.as_str())
                .or_default()
                .push(*row);
        }

        for frame in frames {
            if ledger.joined_by_root.len() >= MULTI_SOURCE_JOIN_MAX_ROWS_V1 {
                ledger.censor(MultiSourceJoinCensoredReasonV1::CapacityExhausted);
                break;
            }
            let (action, outcome) = match completed_refs(frame) {
                Ok(refs) => refs,
                Err(reason) => {
                    ledger.censor(reason);
                    continue;
                }
            };
            if ledger
                .used_completed_frames
                .contains(&action.completed_frame_root_sha256)
            {
                ledger.report.duplicate_idempotent =
                    ledger.report.duplicate_idempotent.saturating_add(1);
                continue;
            }
            let same_intent = eligible_by_intent
                .get(action.turn_intent_id_sha256.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            // One immutable request topology may ground several unique actions from the
            // same response. Evidence independence is enforced later by session lineage.
            let candidates = same_intent
                .iter()
                .copied()
                .filter(|row| topology_matches_frame(row, &action))
                .collect::<Vec<_>>();
            let Some(selected) = select_latest_unique(&candidates) else {
                let reason = classify_missing_match(same_intent, &action, &candidates);
                ledger.censor(reason);
                continue;
            };
            match joined_row(selected, &action, &outcome) {
                Ok(joined) => {
                    if let Some(existing) = ledger.joined_by_root.get(&joined.join_root_sha256) {
                        if existing == &joined {
                            ledger.report.duplicate_idempotent =
                                ledger.report.duplicate_idempotent.saturating_add(1);
                        } else {
                            ledger.censor(MultiSourceJoinCensoredReasonV1::DuplicateConflict);
                        }
                        continue;
                    }
                    ledger
                        .used_completed_frames
                        .insert(action.completed_frame_root_sha256.clone());
                    if joined.accepted {
                        ledger.report.accepted_rows = ledger.report.accepted_rows.saturating_add(1);
                    } else {
                        ledger.report.negative_rows = ledger.report.negative_rows.saturating_add(1);
                    }
                    ledger
                        .joined_by_root
                        .insert(joined.join_root_sha256.clone(), joined);
                }
                Err(reason) => ledger.censor(reason),
            }
        }
        ledger.report.joined_rows = u64::try_from(ledger.joined_by_root.len()).unwrap_or(u64::MAX);
        ledger
    }

    #[must_use]
    pub fn rows(&self) -> Vec<BlindThenRevealJoinedTransitionV1> {
        self.joined_by_root.values().cloned().collect()
    }

    #[must_use]
    pub fn report(&self) -> MultiSourceJoinReportV1 {
        self.report.clone()
    }

    fn censor(&mut self, reason: MultiSourceJoinCensoredReasonV1) {
        let count = self.report.censored.entry(reason).or_default();
        *count = count.saturating_add(1);
    }
}

impl BlindThenRevealJoinedTransitionV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != BLIND_THEN_REVEAL_JOIN_SCHEMA_V1
            || self.capture_sequence == 0
            || self.input_tokens == 0
            || self.captured_at_unix_ms == 0
            || self.completed_at_unix_nanos < self.captured_at_unix_ms.saturating_mul(1_000_000)
            || [
                &self.join_root_sha256,
                &self.turn_intent_id_sha256,
                &self.request_event_id_sha256,
                &self.action_event_id_sha256,
                &self.session_lineage_sha256,
                &self.session_id_sha256,
                &self.topology_commitment_root_sha256,
                &self.pre_action_record_root_sha256,
                &self.completed_frame_root_sha256,
                &self.physical_action_root_sha256,
                &self.semantic_action_root_sha256,
                &self.verifier_receipt_root_sha256,
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.topology.validate().is_err()
            || self.join_root_sha256 != self.expected_root()?
        {
            return Err("blind_then_reveal_join_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&JoinedDigest {
            schema: BLIND_THEN_REVEAL_JOIN_SCHEMA_V1,
            capture_sequence: self.capture_sequence,
            turn_intent_id_sha256: &self.turn_intent_id_sha256,
            request_event_id_sha256: &self.request_event_id_sha256,
            action_event_id_sha256: &self.action_event_id_sha256,
            session_lineage_sha256: &self.session_lineage_sha256,
            session_id_sha256: &self.session_id_sha256,
            topology_commitment_root_sha256: &self.topology_commitment_root_sha256,
            pre_action_record_root_sha256: &self.pre_action_record_root_sha256,
            completed_frame_root_sha256: &self.completed_frame_root_sha256,
            physical_action_root_sha256: &self.physical_action_root_sha256,
            semantic_action_root_sha256: &self.semantic_action_root_sha256,
            effect_atoms: &self.effect_atoms,
            verifier_receipt_root_sha256: &self.verifier_receipt_root_sha256,
            input_tokens: self.input_tokens,
            captured_at_unix_ms: self.captured_at_unix_ms,
            completed_at_unix_nanos: self.completed_at_unix_nanos,
            accepted: self.accepted,
            topology: &self.topology,
        })
    }
}

fn validate_topology_row(
    row: &PreActionTopologyAuditRowV1,
) -> Result<(), MultiSourceJoinCensoredReasonV1> {
    if !row.physical_order_proven {
        return Err(MultiSourceJoinCensoredReasonV1::LegacyOrderingUnproven);
    }
    if !row.structure.provider_bound_turn_identity
        || row.structure.request_event_id_sha256.is_empty()
    {
        return Err(MultiSourceJoinCensoredReasonV1::ProviderIdentityUnproven);
    }
    if !matches!(
        row.commit.evidence_origin,
        MultiSourceEvidenceOriginV1::FreshLive
    ) {
        return Err(MultiSourceJoinCensoredReasonV1::LegacyOrderingUnproven);
    }
    if matches!(
        row.structure.topology.extraction_status,
        MultiSourceExtractionStatusV1::Censored { .. }
    ) {
        return Err(MultiSourceJoinCensoredReasonV1::TopologyCensored);
    }
    if row.structure.validate().is_err()
        || row.commit.validate().is_err()
        || row
            .record_sha256
            .as_deref()
            .is_none_or(|root| !valid_nonzero_sha256(root))
        || row.bridge_sequence.is_none_or(|sequence| sequence == 0)
        || row
            .session_lineage_sha256
            .as_deref()
            .is_none_or(|root| !valid_nonzero_sha256(root))
        || row.captured_at_unix_ms.is_none_or(|value| value == 0)
    {
        return Err(MultiSourceJoinCensoredReasonV1::IdentityMismatch);
    }
    Ok(())
}

fn completed_refs(
    frame: &RelationFrame,
) -> Result<
    (ObservedTeacherActionRefV1, VerifiedOutcomeReceiptRefV1),
    MultiSourceJoinCensoredReasonV1,
> {
    let action =
        teacher_action_ast(frame).ok_or(MultiSourceJoinCensoredReasonV1::MissingTeacherAction)?;
    let semantic_action_root_sha256 = teacher_semantic_law_signature(frame)
        .ok_or(MultiSourceJoinCensoredReasonV1::MissingTeacherAction)?;
    let outcome = teacher_outcome_from_completed(frame)
        .map_err(|_| MultiSourceJoinCensoredReasonV1::MissingVerifierReceipt)?;
    let completed_frame_root_sha256 = canonical_json_sha256(frame)
        .map_err(|_| MultiSourceJoinCensoredReasonV1::MissingVerifierReceipt)?;
    let physical_action_root_sha256 = canonical_json_sha256(&action)
        .map_err(|_| MultiSourceJoinCensoredReasonV1::MissingTeacherAction)?;
    let verifier_receipt_root_sha256 = canonical_json_sha256(&outcome.verifier)
        .map_err(|_| MultiSourceJoinCensoredReasonV1::MissingVerifierReceipt)?;
    Ok((
        ObservedTeacherActionRefV1 {
            turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
            session_id_sha256: frame.session_id_sha256.clone(),
            action_event_id_sha256: frame.event_id_sha256.clone(),
            completed_frame_root_sha256: completed_frame_root_sha256.clone(),
            physical_action_root_sha256: physical_action_root_sha256.clone(),
            semantic_action_root_sha256,
            effect_atoms: completed_effect_atoms(&action.atoms, &frame.atoms),
            observed_at_unix_nanos: frame.observed_at_unix_nanos,
            estimated_input_tokens: frame.estimated_input_tokens,
        },
        VerifiedOutcomeReceiptRefV1 {
            turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
            action_event_id_sha256: frame.event_id_sha256.clone(),
            completed_frame_root_sha256,
            physical_action_root_sha256,
            verifier_evidence_root_sha256: outcome.verifier.evidence_ref_sha256,
            verified_output_root_sha256: outcome.verifier.output_digest_sha256,
            verifier_receipt_root_sha256,
            accepted: outcome.verifier.accepted,
            verified_at_unix_nanos: outcome.completed_at_unix_nanos,
        },
    ))
}

fn topology_matches_frame(
    row: &PreActionTopologyAuditRowV1,
    action: &ObservedTeacherActionRefV1,
) -> bool {
    row.structure.turn_intent_id_sha256 == action.turn_intent_id_sha256
        && row
            .captured_at_unix_ms
            .is_some_and(|time| time.saturating_mul(1_000_000) <= action.observed_at_unix_nanos)
        && (row.structure.session_lineage_roots_sha256.is_empty()
            || row
                .structure
                .session_lineage_roots_sha256
                .contains(&action.session_id_sha256))
}

fn select_latest_unique<'a>(
    candidates: &[&'a PreActionTopologyAuditRowV1],
) -> Option<&'a PreActionTopologyAuditRowV1> {
    let latest = candidates
        .iter()
        .filter_map(|row| row.captured_at_unix_ms)
        .max()?;
    let mut latest_rows = candidates
        .iter()
        .copied()
        .filter(|row| row.captured_at_unix_ms == Some(latest));
    let selected = latest_rows.next()?;
    latest_rows.next().is_none().then_some(selected)
}

fn classify_missing_match(
    same_intent: &[&PreActionTopologyAuditRowV1],
    action: &ObservedTeacherActionRefV1,
    candidates: &[&PreActionTopologyAuditRowV1],
) -> MultiSourceJoinCensoredReasonV1 {
    if candidates.len() > 1
        && candidates
            .iter()
            .filter_map(|row| row.captured_at_unix_ms)
            .max()
            .is_some_and(|latest| {
                candidates
                    .iter()
                    .filter(|row| row.captured_at_unix_ms == Some(latest))
                    .count()
                    > 1
            })
    {
        return MultiSourceJoinCensoredReasonV1::AmbiguousPreActionMatch;
    }
    if same_intent.iter().any(|row| {
        row.captured_at_unix_ms
            .is_some_and(|time| time.saturating_mul(1_000_000) > action.observed_at_unix_nanos)
    }) {
        MultiSourceJoinCensoredReasonV1::PreActionOrderInvalid
    } else {
        MultiSourceJoinCensoredReasonV1::IdentityMismatch
    }
}

fn joined_row(
    topology: &PreActionTopologyAuditRowV1,
    action: &ObservedTeacherActionRefV1,
    outcome: &VerifiedOutcomeReceiptRefV1,
) -> Result<BlindThenRevealJoinedTransitionV1, MultiSourceJoinCensoredReasonV1> {
    if action.turn_intent_id_sha256 != outcome.turn_intent_id_sha256
        || action.action_event_id_sha256 != outcome.action_event_id_sha256
        || action.completed_frame_root_sha256 != outcome.completed_frame_root_sha256
        || action.physical_action_root_sha256 != outcome.physical_action_root_sha256
        || outcome.verified_at_unix_nanos < action.observed_at_unix_nanos
    {
        return Err(MultiSourceJoinCensoredReasonV1::IdentityMismatch);
    }
    let input_tokens = if action.estimated_input_tokens > 0 {
        action.estimated_input_tokens
    } else {
        topology.structure.estimated_input_tokens
    };
    let mut joined = BlindThenRevealJoinedTransitionV1 {
        schema: BLIND_THEN_REVEAL_JOIN_SCHEMA_V1.to_owned(),
        join_root_sha256: String::new(),
        capture_sequence: topology
            .bridge_sequence
            .ok_or(MultiSourceJoinCensoredReasonV1::IdentityMismatch)?,
        turn_intent_id_sha256: action.turn_intent_id_sha256.clone(),
        request_event_id_sha256: topology.structure.request_event_id_sha256.clone(),
        action_event_id_sha256: action.action_event_id_sha256.clone(),
        session_lineage_sha256: topology
            .session_lineage_sha256
            .clone()
            .ok_or(MultiSourceJoinCensoredReasonV1::IdentityMismatch)?,
        session_id_sha256: action.session_id_sha256.clone(),
        topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
        pre_action_record_root_sha256: topology
            .record_sha256
            .clone()
            .ok_or(MultiSourceJoinCensoredReasonV1::IdentityMismatch)?,
        completed_frame_root_sha256: action.completed_frame_root_sha256.clone(),
        physical_action_root_sha256: action.physical_action_root_sha256.clone(),
        semantic_action_root_sha256: action.semantic_action_root_sha256.clone(),
        effect_atoms: action.effect_atoms.clone(),
        verifier_receipt_root_sha256: outcome.verifier_receipt_root_sha256.clone(),
        input_tokens,
        captured_at_unix_ms: topology
            .captured_at_unix_ms
            .ok_or(MultiSourceJoinCensoredReasonV1::IdentityMismatch)?,
        completed_at_unix_nanos: outcome.verified_at_unix_nanos,
        accepted: outcome.accepted,
        topology: topology.structure.topology.clone(),
    };
    joined.join_root_sha256 = joined
        .expected_root()
        .map_err(|_| MultiSourceJoinCensoredReasonV1::IdentityMismatch)?;
    joined
        .validate()
        .map_err(|_| MultiSourceJoinCensoredReasonV1::IdentityMismatch)?;
    Ok(joined)
}

pub(super) fn join_explicit_topology_to_frame(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
) -> Result<BlindThenRevealJoinedTransitionV1, MultiSourceJoinCensoredReasonV1> {
    validate_topology_row(topology)?;
    let (action, outcome) = completed_refs(frame)?;
    if !topology_matches_frame(topology, &action) {
        return Err(MultiSourceJoinCensoredReasonV1::IdentityMismatch);
    }
    joined_row(topology, &action, &outcome)
}

fn completed_effect_atoms(
    action_atoms: &[RelationAtom],
    frame_atoms: &[RelationAtom],
) -> Vec<CompletedEffectAtomV1> {
    let action_slots = frame_atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: nando_operator_kernel::AtomSource::Action,
                value_type,
                ..
            } => Some((*slot_id, *value_type)),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let observation_slots = frame_atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::TypedSlot {
                slot_id,
                source: nando_operator_kernel::AtomSource::Observation,
                ..
            } => Some(*slot_id),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let action_to_observation = action_atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } if action_slots.contains_key(left_slot) && observation_slots.contains(right_slot) => {
                Some((*left_slot, *right_slot))
            }
            RelationAtom::SlotEquality {
                left_slot,
                right_slot,
            } if action_slots.contains_key(right_slot) && observation_slots.contains(left_slot) => {
                Some((*right_slot, *left_slot))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let mut result = action_atoms
        .iter()
        .filter_map(|atom| match atom {
            RelationAtom::ActionRoleArgument {
                slot_id,
                value_type,
                ..
            } => Some(CompletedEffectAtomV1::RoleInputSlot {
                slot_id: action_to_observation
                    .get(slot_id)
                    .copied()
                    .unwrap_or(*slot_id),
                value_type: *value_type,
            }),
            RelationAtom::ActionIntegerArgument { .. } => {
                Some(CompletedEffectAtomV1::IntegerConstant)
            }
            RelationAtom::ActionStringArgument { .. } => {
                Some(CompletedEffectAtomV1::StringConstant)
            }
            RelationAtom::ActionBooleanArgument { .. } => {
                Some(CompletedEffectAtomV1::BooleanConstant)
            }
            RelationAtom::ActionResultProjection { .. } => {
                Some(CompletedEffectAtomV1::ResultProjection)
            }
            RelationAtom::ActionOutputProjection { .. } => {
                Some(CompletedEffectAtomV1::OutputProjection)
            }
            RelationAtom::ActionJsonResultProjection => {
                Some(CompletedEffectAtomV1::JsonResultProjection)
            }
            RelationAtom::ActionValueProjection { .. } => {
                Some(CompletedEffectAtomV1::ValueProjection)
            }
            RelationAtom::ActionStatusProjection { .. } => {
                Some(CompletedEffectAtomV1::StatusProjection)
            }
            RelationAtom::ActionPlanAdvance => Some(CompletedEffectAtomV1::PlanAdvance),
            _ => None,
        })
        .collect::<Vec<_>>();
    for (action_slot_id, observation_slot_id) in action_to_observation {
        result.push(CompletedEffectAtomV1::RoleInputSlot {
            slot_id: observation_slot_id,
            value_type: action_slots.get(&action_slot_id).copied(),
        });
    }
    result.sort_unstable();
    result.dedup();
    result
}
