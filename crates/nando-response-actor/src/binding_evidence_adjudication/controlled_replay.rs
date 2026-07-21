//! Controlled B1B replay fixtures.
//!
//! This module reconstructs the frozen support/future fixture rows for byte
//! parity only. It is proof/eval code and does not provide runtime binding or
//! authority.

use serde::Serialize;
use serde_json::{Value, json};

use crate::binding_evidence::{
    BindingCompletionStateV1, FrozenCandidateRelationGraphV1, PreActionBindingContextV1,
    PreActionBindingSurfaceV1,
};
use crate::{
    EVIDENCE_LEDGER_SCHEMA_V1, EvidenceIngestOutcome, EvidenceLedgerRecord, EvidencePolicyV1,
    RawEvidenceEnvelope, canonical_json_sha256, canonicalize_evidence_envelope,
};

use super::canonical::checked_u16;
use super::wire::{BindingAdjudicationErrorV1, REQUEST_CONTRACT_V1};

#[derive(Clone)]
pub(super) struct PhysicalParent {
    pub(super) marker: String,
    pub(super) capability: String,
    pub(super) active: bool,
    pub(super) rank: u64,
}

#[derive(Clone)]
pub(super) struct PhysicalBindingScene {
    pub(super) parents: Vec<PhysicalParent>,
    pub(super) candidates: Vec<String>,
    pub(super) requested_parents: Vec<String>,
    pub(super) requested_capability: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) enum ReplayPartition {
    Support,
    Future,
}

pub(super) fn support_scene(intervention: usize, replicate: usize) -> PhysicalBindingScene {
    let left = format!("opaque-capability-{replicate}-left");
    let right = format!("opaque-capability-{replicate}-right");
    let parent_left = format!("opaque-parent-{replicate}-left");
    let parent_right = format!("opaque-parent-{replicate}-right");
    let mut parents = vec![PhysicalParent {
        marker: parent_left.clone(),
        capability: left.clone(),
        active: true,
        rank: 1,
    }];
    let mut candidates = vec![left.clone()];
    let (requested_parents, requested_capability) = match intervention {
        1 => {
            candidates.push(right.clone());
            if replicate == 1 {
                candidates.reverse();
            }
            (vec![parent_left], Some(left))
        }
        2 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            candidates.push(right.clone());
            if replicate == 0 {
                (vec![parent_left], Some(left))
            } else {
                (vec![parent_right], Some(right))
            }
        }
        3 => {
            candidates.extend([right, format!("opaque-decoy-{replicate}")]);
            (vec![parent_left], Some(left))
        }
        4 => {
            parents[0].active = false;
            (vec![parent_left], Some(left))
        }
        5 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            candidates.push(right);
            (vec![parent_left, parent_right], None)
        }
        6 => {
            candidates.push(right);
            (vec![format!("opaque-missing-parent-{replicate}")], None)
        }
        _ => unreachable!("bounded intervention"),
    };
    PhysicalBindingScene {
        parents,
        candidates,
        requested_parents,
        requested_capability,
    }
}

pub(super) fn future_scene(intervention: usize, replicate: usize) -> PhysicalBindingScene {
    let base = format!("future-i{intervention}");
    let left = format!("{base}-capability-left");
    let right = format!("{base}-capability-right");
    let decoy = format!("{base}-capability-decoy");
    let parent_left = format!("{base}-parent-left");
    let parent_right = format!("{base}-parent-right");
    let mut parents = vec![PhysicalParent {
        marker: parent_left.clone(),
        capability: left.clone(),
        active: true,
        rank: 1,
    }];
    let mut candidates = vec![left.clone(), right.clone()];
    let (requested_parents, requested_capability) = match intervention {
        1 => {
            if replicate == 1 {
                candidates.reverse();
            }
            (vec![parent_left], Some(left))
        }
        2 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right.clone(),
                active: true,
                rank: 1,
            });
            if replicate == 0 {
                (vec![parent_left], Some(left))
            } else {
                (vec![parent_right], Some(right))
            }
        }
        3 => {
            candidates.push(decoy);
            (vec![parent_left], Some(left))
        }
        4 => {
            parents[0].active = false;
            (vec![parent_left], Some(left))
        }
        5 => {
            parents.push(PhysicalParent {
                marker: parent_right.clone(),
                capability: right,
                active: true,
                rank: 1,
            });
            (vec![parent_left, parent_right], None)
        }
        6 => (vec![format!("{base}-parent-missing")], None),
        _ => unreachable!("bounded intervention"),
    };
    PhysicalBindingScene {
        parents,
        candidates,
        requested_parents,
        requested_capability,
    }
}

pub(super) fn render_support_scene(scene: &PhysicalBindingScene, replicate: usize) -> Value {
    let parents = scene
        .parents
        .iter()
        .map(|parent| {
            json!({
                "anchor": parent.marker,
                "capability": parent.capability,
                "state": if parent.active { "active" } else { "completed" },
                "distance": parent.rank,
            })
        })
        .collect::<Vec<_>>();
    let relation_source = if scene.requested_parents.len() == 1 {
        json!(scene.requested_parents[0])
    } else {
        json!(scene.requested_parents)
    };
    let relation = json!({
        "source": relation_source,
        "capability": scene.requested_capability,
    });
    if replicate == 0 {
        json!({
            "history": parents,
            "available": scene.candidates,
            "request_relation": relation,
        })
    } else {
        json!({
            "transport": {
                "items": parents,
                "choices": scene.candidates,
                "relation": relation,
            }
        })
    }
}

pub(super) fn render_future_scene(scene: &PhysicalBindingScene, shape: usize) -> Value {
    let relation_source = if scene.requested_parents.len() == 1 {
        json!(scene.requested_parents[0])
    } else {
        json!(scene.requested_parents)
    };
    let relation_target = json!(scene.requested_capability);
    match shape {
        0 => json!({
            "future_alpha_timeline": render_future_parents(&scene.parents, "alpha"),
            "future_alpha_options": scene.candidates,
            "future_alpha_binding": {
                "future_alpha_origin": relation_source,
                "future_alpha_target": relation_target,
            }
        }),
        1 => json!({
            "future_beta_packet": {
                "future_beta_records": render_future_parents(&scene.parents, "beta"),
                "future_beta_choices": scene.candidates,
                "future_beta_link": {
                    "future_beta_from": relation_source,
                    "future_beta_to": relation_target,
                }
            }
        }),
        2 => json!([{
            "future_gamma_records": render_future_parents(&scene.parents, "gamma"),
            "future_gamma_choices": scene.candidates,
            "future_gamma_link": [relation_source, relation_target],
        }]),
        _ => json!({
            "future_delta_state": {
                "future_delta_records": render_future_parents(&scene.parents, "delta"),
            },
            "future_delta_selection": {
                "future_delta_choices": scene.candidates,
            },
            "future_delta_relation": {
                "future_delta_from": relation_source,
                "future_delta_to": relation_target,
            }
        }),
    }
}

pub(super) fn render_future_parents(parents: &[PhysicalParent], prefix: &str) -> Vec<Value> {
    parents
        .iter()
        .map(|parent| {
            json!({
                format!("future_{prefix}_marker"): parent.marker,
                format!("future_{prefix}_endpoint"): parent.capability,
                format!("future_{prefix}_phase"): if parent.active { "active" } else { "completed" },
                format!("future_{prefix}_rank"): parent.rank,
            })
        })
        .collect()
}

pub(super) fn support_context(
    scene: &PhysicalBindingScene,
) -> Result<PreActionBindingContextV1, BindingAdjudicationErrorV1> {
    let active = scene.parents.iter().filter(|parent| parent.active).count();
    let completed = scene.parents.len().saturating_sub(active);
    Ok(PreActionBindingContextV1 {
        call_shape_count: checked_u16(scene.parents.len())?,
        capability_count: checked_u16(scene.candidates.len())?,
        completion_state: if active > 0 {
            BindingCompletionStateV1::Unresolved
        } else if completed > 0 {
            BindingCompletionStateV1::Completed
        } else {
            BindingCompletionStateV1::Unknown
        },
        temporal_relation_count: checked_u16(scene.parents.len())?,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256: canonical_json_sha256(&json!({
            "parents": scene.parents.len(),
            "active": active,
            "completed": completed,
            "candidates": scene.candidates.len(),
            "relation_present": true,
        }))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    })
}

pub(super) fn future_context(
    scene: &PhysicalBindingScene,
) -> Result<PreActionBindingContextV1, BindingAdjudicationErrorV1> {
    let active = scene.parents.iter().filter(|parent| parent.active).count();
    let completed = scene.parents.len().saturating_sub(active);
    Ok(PreActionBindingContextV1 {
        call_shape_count: checked_u16(scene.parents.len())?,
        capability_count: checked_u16(scene.candidates.len())?,
        completion_state: if active > 0 {
            BindingCompletionStateV1::Unresolved
        } else {
            BindingCompletionStateV1::Completed
        },
        temporal_relation_count: checked_u16(scene.parents.len())?,
        cardinality_relation_count: if scene.requested_parents.len() > 1 {
            checked_u16(scene.requested_parents.len())?
        } else {
            1
        },
        topology_neighborhood_root_sha256: canonical_json_sha256(&json!({
            "parents": scene.parents.len(),
            "active": active,
            "completed": completed,
            "candidates": scene.candidates.len(),
            "relation_sources": scene.requested_parents.len(),
            "relation_target_present": scene.requested_capability.is_some(),
        }))
        .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    })
}

pub(super) fn replay_capture_record(
    partition: ReplayPartition,
    row_index: usize,
    session: usize,
    payload: &Value,
    previous_record_sha256: &str,
    first_sequence: u64,
) -> Result<EvidenceLedgerRecord, BindingAdjudicationErrorV1> {
    let (source_stream_id, event_id, session_id, intent_id, call_id, event_time) = match partition {
        ReplayPartition::Support => (
            "nando-b1b-support-acquisition-v1".to_owned(),
            format!("b1b-support-event-{row_index}"),
            format!("b1b-support-session-{session}"),
            format!("b1b-support-intent-{row_index}"),
            format!("b1b-support-call-{row_index}"),
            10_000_000 + row_index as u64,
        ),
        ReplayPartition::Future => (
            "nando-b1b-future-acquisition-v1".to_owned(),
            format!("b1b-future-event-{row_index}"),
            format!("b1b-future-session-S{session}"),
            format!("b1b-future-intent-{row_index}"),
            format!("b1b-future-call-{row_index}"),
            20_000_000 + row_index as u64,
        ),
    };
    let envelope = RawEvidenceEnvelope {
        source_stream_id,
        source_offset: row_index as u64,
        event_id,
        session_id,
        client_intent_id: Some(intent_id),
        call_id: Some(call_id),
        output_ordinal: Some(row_index as u32),
        event_time_unix_nanos: Some(event_time),
        schema_version: 1,
        payload: serde_json::to_vec(payload)
            .map_err(|_| BindingAdjudicationErrorV1::Serialization)?,
    };
    let outcome = EvidenceIngestOutcome::Normalized {
        graph: canonicalize_evidence_envelope(&envelope, EvidencePolicyV1::streaming_bounded())
            .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?,
    };
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let sequence = first_sequence + row_index as u64;
    let record_sha256 = canonical_json_sha256(&DigestFields {
        schema: EVIDENCE_LEDGER_SCHEMA_V1,
        sequence,
        previous_record_sha256,
        outcome: &outcome,
    })
    .map_err(|_| BindingAdjudicationErrorV1::Serialization)?;
    Ok(EvidenceLedgerRecord {
        schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        outcome,
        record_sha256,
    })
}

pub(super) fn validate_replayed_row(
    frozen_graph: &FrozenCandidateRelationGraphV1,
    frozen_record: &EvidenceLedgerRecord,
    replayed_record: &EvidenceLedgerRecord,
    payload: &Value,
    context: PreActionBindingContextV1,
) -> Result<(), BindingAdjudicationErrorV1> {
    if frozen_record != replayed_record {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    let replayed_graph = PreActionBindingSurfaceV1::capture(
        frozen_graph.graph.row_id_sha256.clone(),
        frozen_graph.graph.evidence_ref_sha256.clone(),
        REQUEST_CONTRACT_V1,
        payload,
        context,
        Default::default(),
    )
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?
    .candidate_relation_graph(Default::default())
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?
    .freeze()
    .map_err(|_| BindingAdjudicationErrorV1::FrozenReplayMismatch)?;
    if replayed_graph != *frozen_graph {
        return Err(BindingAdjudicationErrorV1::FrozenReplayMismatch);
    }
    Ok(())
}
