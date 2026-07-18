use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AtomSource, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, RuntimeParityCase};

pub const RUNTIME_FRAME_SCHEMA_V1: &str = "nando.runtime-frame.v1";
pub const TEACHER_OUTCOME_SCHEMA_V1: &str = "nando.teacher-outcome.v1";
pub const TEACHER_TRANSITION_SCHEMA_V1: &str = "nando.teacher-transition.v1";
pub const ECONOMICS_RECEIPT_SCHEMA_V1: &str = "nando.economics-receipt.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeFrame {
    pub schema: String,
    pub frame_id_sha256: String,
    pub event_id_sha256: String,
    pub client_intent_id_sha256: String,
    pub session_id_sha256: String,
    pub observed_at_unix_nanos: u64,
    pub extractor_version: String,
    pub atoms: Vec<RelationAtom>,
    pub evidence_ref_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherActionAst {
    pub signature_sha256: String,
    pub action_symbol: String,
    pub atoms: Vec<RelationAtom>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherVerifierEvidence {
    pub accepted: bool,
    pub evidence_ref_sha256: String,
    pub output_digest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherOutcome {
    pub schema: String,
    pub action: TeacherActionAst,
    pub verifier: TeacherVerifierEvidence,
    pub completed_at_unix_nanos: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EconomicsReceipt {
    pub schema: String,
    pub exact_input_tokens: u64,
    pub ordinary: bool,
    pub controlled: bool,
    pub replay: bool,
    pub dedupe_eligible: bool,
    pub provider_evidence_ref_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeacherTransition {
    pub schema: String,
    pub before: RuntimeFrame,
    pub outcome: TeacherOutcome,
    pub economics: Option<EconomicsReceipt>,
    #[serde(default)]
    pub runtime_parity_case: Option<RuntimeParityCase>,
}

impl RuntimeFrame {
    /// Builds the only frame representation accepted by routing and guards.
    /// Action/outcome slots and relations that reference them are removed.
    #[must_use]
    pub fn from_completed(frame: &RelationFrame) -> Self {
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
        let mut atoms = frame
            .atoms
            .iter()
            .filter(|atom| runtime_atom_is_observable(atom, &pre_action_slots))
            .cloned()
            .collect::<Vec<_>>();
        atoms.sort();
        atoms.dedup();
        let frame_id_sha256 = digest_json(&(
            RUNTIME_FRAME_SCHEMA_V1,
            frame.frame_id_sha256.as_str(),
            &atoms,
        ));
        Self {
            schema: RUNTIME_FRAME_SCHEMA_V1.to_owned(),
            frame_id_sha256,
            event_id_sha256: frame.event_id_sha256.clone(),
            client_intent_id_sha256: frame.client_intent_id_sha256.clone(),
            session_id_sha256: frame.session_id_sha256.clone(),
            observed_at_unix_nanos: frame.observed_at_unix_nanos,
            extractor_version: frame.extractor_version.clone(),
            atoms,
            evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
        }
    }

    #[must_use]
    pub fn as_routing_relation_frame(&self) -> RelationFrame {
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: self.frame_id_sha256.clone(),
            event_id_sha256: self.event_id_sha256.clone(),
            client_intent_id_sha256: self.client_intent_id_sha256.clone(),
            session_id_sha256: self.session_id_sha256.clone(),
            observed_at_unix_nanos: self.observed_at_unix_nanos,
            estimated_input_tokens: 0,
            extractor_version: self.extractor_version.clone(),
            verifier_label: None,
            atoms: self.atoms.clone(),
            evidence_ref_sha256: self.evidence_ref_sha256.clone(),
        }
    }

    #[must_use]
    pub fn contains_teacher_atoms(&self) -> bool {
        self.atoms.iter().any(relation_atom_is_teacher_only)
    }
}

impl TeacherTransition {
    /// Reconstructs the completed training row for cold synthesis only.
    /// Runtime code has no inverse conversion and never imports this type.
    #[must_use]
    pub fn as_training_relation_frame(&self) -> RelationFrame {
        let mut atoms = self.before.atoms.clone();
        atoms.extend(self.outcome.action.atoms.iter().cloned());
        atoms.sort();
        atoms.dedup();
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: digest_json(&(
                TEACHER_TRANSITION_SCHEMA_V1,
                self.before.frame_id_sha256.as_str(),
                self.outcome.action.signature_sha256.as_str(),
                self.outcome.verifier.accepted,
                &atoms,
            )),
            event_id_sha256: self.before.event_id_sha256.clone(),
            client_intent_id_sha256: self.before.client_intent_id_sha256.clone(),
            session_id_sha256: self.before.session_id_sha256.clone(),
            observed_at_unix_nanos: self.outcome.completed_at_unix_nanos,
            estimated_input_tokens: self
                .economics
                .as_ref()
                .map_or(0, |receipt| receipt.exact_input_tokens),
            extractor_version: self.before.extractor_version.clone(),
            verifier_label: Some(self.outcome.verifier.accepted),
            atoms,
            evidence_ref_sha256: self.outcome.verifier.evidence_ref_sha256.clone(),
        }
    }
}

/// Hashes only immutable learning semantics. Event time, token accounting,
/// joined lineage, extractor metadata, and receipt provenance may be enriched
/// after capture without creating a new training example.
pub fn relation_frame_learning_digest(frame: &RelationFrame) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(&(
        frame.schema.as_str(),
        frame.frame_id_sha256.as_str(),
        frame.verifier_label,
        &frame.atoms,
    ))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[must_use]
pub fn relation_atom_is_teacher_only(atom: &RelationAtom) -> bool {
    matches!(
        atom,
        RelationAtom::TypedSlot {
            source: AtomSource::Action | AtomSource::Outcome,
            ..
        } | RelationAtom::ActionFunction { .. }
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
            | RelationAtom::ReconstructedClientCapabilityAtom { .. }
            | RelationAtom::ResponseShape { .. }
    )
}

fn runtime_atom_is_observable(atom: &RelationAtom, slots: &BTreeSet<u16>) -> bool {
    if relation_atom_is_teacher_only(atom) {
        return false;
    }
    match atom {
        RelationAtom::SlotEquality {
            left_slot,
            right_slot,
        } => slots.contains(left_slot) && slots.contains(right_slot),
        RelationAtom::UniqueSlot { slot_id }
        | RelationAtom::ObservationSelector { slot_id, .. } => slots.contains(slot_id),
        _ => true,
    }
}

fn digest_json<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}
