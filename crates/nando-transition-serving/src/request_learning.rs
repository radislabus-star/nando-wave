use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::LearningRequestStructureV1;
use nando_operator_learning::multi_source::{
    RequestStructureAuditRowV1, RequestStructureAuditSnapshotV1,
};
use serde::{Deserialize, Serialize};

const REQUEST_LEARNING_CHECKPOINT_SCHEMA_V2: &str = "nando.request-learning-checkpoint.v2";
const MAX_REQUEST_LEARNING_IDENTITIES: usize = 4_096;
pub(crate) const REQUEST_LEARNING_CHECKPOINT_MAX_BYTES_V2: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RequestLearningAtoms {
    pub(crate) request_phase_atom_ids: Vec<u64>,
    pub(crate) capability_atom_ids: Vec<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RequestLearningStatusV2 {
    pub(crate) structures_applied: u64,
    pub(crate) session_inserts: u64,
    pub(crate) session_updates: u64,
    pub(crate) turn_inserts: u64,
    pub(crate) turn_updates: u64,
    pub(crate) evictions: u64,
    pub(crate) lookup_attempts: u64,
    pub(crate) lookup_hits: u64,
    pub(crate) lookup_misses: u64,
    pub(crate) stored_sessions: u64,
    pub(crate) stored_turns: u64,
}

#[derive(Default)]
struct RequestLearningCounters {
    structures_applied: AtomicU64,
    session_inserts: AtomicU64,
    session_updates: AtomicU64,
    turn_inserts: AtomicU64,
    turn_updates: AtomicU64,
    evictions: AtomicU64,
    lookup_attempts: AtomicU64,
    lookup_hits: AtomicU64,
    lookup_misses: AtomicU64,
}

#[derive(Default)]
struct RequestLearningState {
    capability_by_session: BTreeMap<String, Vec<u64>>,
    session_order: VecDeque<String>,
    structure_by_turn: BTreeMap<String, RequestLearningAtoms>,
    turn_order: VecDeque<String>,
}

#[derive(Deserialize, Serialize)]
struct RequestLearningCheckpointWireV2 {
    schema: String,
    bridge_epoch_sha256: String,
    last_sequence: u64,
    last_record_sha256: String,
    capability_by_session: BTreeMap<String, Vec<u64>>,
    session_order: Vec<String>,
    structure_by_turn: BTreeMap<String, RequestLearningAtoms>,
    turn_order: Vec<String>,
    status: RequestLearningStatusV2,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RequestLearningWatermarkV2 {
    pub(crate) bridge_epoch_sha256: String,
    pub(crate) last_sequence: u64,
    pub(crate) last_record_sha256: String,
}

#[derive(Default)]
pub(crate) struct RequestLearningIndex {
    state: Mutex<RequestLearningState>,
    counters: RequestLearningCounters,
}

impl RequestLearningIndex {
    pub(crate) fn audit_snapshot_v1(
        &self,
    ) -> Result<RequestStructureAuditSnapshotV1, &'static str> {
        let state = self
            .state
            .lock()
            .map_err(|_| "request_learning_index_lock_poisoned")?;
        let rows = state
            .structure_by_turn
            .iter()
            .map(|(intent_sha256, atoms)| RequestStructureAuditRowV1 {
                intent_sha256: intent_sha256.clone(),
                request_phase_atom_count: atoms.request_phase_atom_ids.len(),
                capability_atom_count: atoms.capability_atom_ids.len(),
            })
            .collect();
        Ok(RequestStructureAuditSnapshotV1 {
            rows,
            evictions: self.counters.evictions.load(Ordering::Relaxed),
            stored_turns: u64::try_from(state.structure_by_turn.len()).unwrap_or(u64::MAX),
            provider_bound_by_construction: true,
            pre_action_context_persisted: false,
        })
    }

    pub(crate) fn observe_structure(
        &self,
        structure: &LearningRequestStructureV1,
    ) -> Result<(), &'static str> {
        let session_keys = structure.session_identity_sha256s();
        let capability_atoms = structure.capability_atom_ids();
        if session_keys.len() > 4
            || capability_atoms.len() > 64
            || session_keys.iter().any(|key| !valid_sha256(key))
            || !valid_sha256(structure.client_intent_id_sha256())
            || !strictly_ordered(session_keys)
            || !strictly_ordered(capability_atoms)
        {
            return Err("request_learning_structure_invalid");
        }
        let Ok(mut state) = self.state.lock() else {
            return Err("request_learning_index_lock_poisoned");
        };
        if !capability_atoms.is_empty() {
            for key in session_keys.iter().cloned() {
                if state.capability_by_session.contains_key(&key) {
                    self.counters
                        .session_updates
                        .fetch_add(1, Ordering::Relaxed);
                } else {
                    state.session_order.push_back(key.clone());
                    self.counters
                        .session_inserts
                        .fetch_add(1, Ordering::Relaxed);
                }
                state
                    .capability_by_session
                    .insert(key, capability_atoms.to_vec());
            }
        }
        if structure.provider_bound_turn_identity() {
            let turn = structure.client_intent_id_sha256().to_owned();
            if state.structure_by_turn.contains_key(&turn) {
                self.counters.turn_updates.fetch_add(1, Ordering::Relaxed);
            } else {
                state.turn_order.push_back(turn.clone());
                self.counters.turn_inserts.fetch_add(1, Ordering::Relaxed);
            }
            state.structure_by_turn.insert(
                turn,
                RequestLearningAtoms {
                    request_phase_atom_ids: structure.request_phase_atom_ids().to_vec(),
                    capability_atom_ids: capability_atoms.to_vec(),
                },
            );
        }
        while state.capability_by_session.len() > MAX_REQUEST_LEARNING_IDENTITIES {
            let Some(key) = state.session_order.pop_front() else {
                break;
            };
            if state.capability_by_session.remove(&key).is_some() {
                self.counters.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
        while state.structure_by_turn.len() > MAX_REQUEST_LEARNING_IDENTITIES {
            let Some(key) = state.turn_order.pop_front() else {
                break;
            };
            if state.structure_by_turn.remove(&key).is_some() {
                self.counters.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.counters
            .structures_applied
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(crate) fn lookup(
        &self,
        session_id_sha256: &str,
        turn_intent_sha256: &str,
    ) -> RequestLearningAtoms {
        self.counters
            .lookup_attempts
            .fetch_add(1, Ordering::Relaxed);
        let Ok(state) = self.state.lock() else {
            self.counters.lookup_misses.fetch_add(1, Ordering::Relaxed);
            return RequestLearningAtoms::default();
        };
        let mut atoms = state
            .structure_by_turn
            .get(turn_intent_sha256)
            .cloned()
            .unwrap_or_default();
        if let Some(capabilities) = state.capability_by_session.get(session_id_sha256) {
            atoms.capability_atom_ids.clone_from(capabilities);
        }
        if atoms == RequestLearningAtoms::default() {
            self.counters.lookup_misses.fetch_add(1, Ordering::Relaxed);
        } else {
            self.counters.lookup_hits.fetch_add(1, Ordering::Relaxed);
        }
        atoms
    }

    pub(crate) fn status(&self) -> RequestLearningStatusV2 {
        let (stored_sessions, stored_turns) = self.state.lock().map_or((0, 0), |state| {
            (
                u64::try_from(state.capability_by_session.len()).unwrap_or(u64::MAX),
                u64::try_from(state.structure_by_turn.len()).unwrap_or(u64::MAX),
            )
        });
        RequestLearningStatusV2 {
            structures_applied: self.counters.structures_applied.load(Ordering::Relaxed),
            session_inserts: self.counters.session_inserts.load(Ordering::Relaxed),
            session_updates: self.counters.session_updates.load(Ordering::Relaxed),
            turn_inserts: self.counters.turn_inserts.load(Ordering::Relaxed),
            turn_updates: self.counters.turn_updates.load(Ordering::Relaxed),
            evictions: self.counters.evictions.load(Ordering::Relaxed),
            lookup_attempts: self.counters.lookup_attempts.load(Ordering::Relaxed),
            lookup_hits: self.counters.lookup_hits.load(Ordering::Relaxed),
            lookup_misses: self.counters.lookup_misses.load(Ordering::Relaxed),
            stored_sessions,
            stored_turns,
        }
    }

    pub(crate) fn checkpoint_cbor(
        &self,
        watermark: &RequestLearningWatermarkV2,
    ) -> Result<Vec<u8>, String> {
        if !valid_sha256(&watermark.bridge_epoch_sha256)
            || (watermark.last_sequence > 0 && !valid_sha256(&watermark.last_record_sha256))
        {
            return Err("request_learning_checkpoint_watermark_invalid".to_owned());
        }
        let state = self
            .state
            .lock()
            .map_err(|_| "request_learning_index_lock_poisoned".to_owned())?;
        let wire = RequestLearningCheckpointWireV2 {
            schema: REQUEST_LEARNING_CHECKPOINT_SCHEMA_V2.to_owned(),
            bridge_epoch_sha256: watermark.bridge_epoch_sha256.clone(),
            last_sequence: watermark.last_sequence,
            last_record_sha256: watermark.last_record_sha256.clone(),
            capability_by_session: state.capability_by_session.clone(),
            session_order: state.session_order.iter().cloned().collect(),
            structure_by_turn: state.structure_by_turn.clone(),
            turn_order: state.turn_order.iter().cloned().collect(),
            status: self.status_without_state_lock(&state),
        };
        let bytes = serde_cbor::to_vec(&wire)
            .map_err(|error| format!("request_learning_checkpoint_encode:{error}"))?;
        if bytes.len() > REQUEST_LEARNING_CHECKPOINT_MAX_BYTES_V2 {
            return Err("request_learning_checkpoint_budget".to_owned());
        }
        Ok(bytes)
    }

    pub(crate) fn from_checkpoint_cbor(
        bytes: &[u8],
    ) -> Result<(Self, RequestLearningWatermarkV2), String> {
        if bytes.is_empty() || bytes.len() > REQUEST_LEARNING_CHECKPOINT_MAX_BYTES_V2 {
            return Err("request_learning_checkpoint_budget".to_owned());
        }
        let wire: RequestLearningCheckpointWireV2 = serde_cbor::from_slice(bytes)
            .map_err(|error| format!("request_learning_checkpoint_decode:{error}"))?;
        validate_checkpoint(&wire)?;
        let index = Self {
            state: Mutex::new(RequestLearningState {
                capability_by_session: wire.capability_by_session,
                session_order: wire.session_order.into(),
                structure_by_turn: wire.structure_by_turn,
                turn_order: wire.turn_order.into(),
            }),
            counters: counters_from_status(&wire.status),
        };
        let watermark = RequestLearningWatermarkV2 {
            bridge_epoch_sha256: wire.bridge_epoch_sha256,
            last_sequence: wire.last_sequence,
            last_record_sha256: wire.last_record_sha256,
        };
        Ok((index, watermark))
    }

    fn status_without_state_lock(&self, state: &RequestLearningState) -> RequestLearningStatusV2 {
        let mut status = self.status_counters();
        status.stored_sessions =
            u64::try_from(state.capability_by_session.len()).unwrap_or(u64::MAX);
        status.stored_turns = u64::try_from(state.structure_by_turn.len()).unwrap_or(u64::MAX);
        status
    }

    fn status_counters(&self) -> RequestLearningStatusV2 {
        RequestLearningStatusV2 {
            structures_applied: self.counters.structures_applied.load(Ordering::Relaxed),
            session_inserts: self.counters.session_inserts.load(Ordering::Relaxed),
            session_updates: self.counters.session_updates.load(Ordering::Relaxed),
            turn_inserts: self.counters.turn_inserts.load(Ordering::Relaxed),
            turn_updates: self.counters.turn_updates.load(Ordering::Relaxed),
            evictions: self.counters.evictions.load(Ordering::Relaxed),
            lookup_attempts: self.counters.lookup_attempts.load(Ordering::Relaxed),
            lookup_hits: self.counters.lookup_hits.load(Ordering::Relaxed),
            lookup_misses: self.counters.lookup_misses.load(Ordering::Relaxed),
            ..RequestLearningStatusV2::default()
        }
    }
}

fn validate_checkpoint(wire: &RequestLearningCheckpointWireV2) -> Result<(), String> {
    let session_order_is_exact = wire
        .session_order
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        == wire.capability_by_session.keys().collect();
    let turn_order_is_exact = wire
        .turn_order
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        == wire.structure_by_turn.keys().collect();
    if wire.schema != REQUEST_LEARNING_CHECKPOINT_SCHEMA_V2
        || !valid_sha256(&wire.bridge_epoch_sha256)
        || (wire.last_sequence > 0 && !valid_sha256(&wire.last_record_sha256))
        || wire.capability_by_session.len() > MAX_REQUEST_LEARNING_IDENTITIES
        || wire.structure_by_turn.len() > MAX_REQUEST_LEARNING_IDENTITIES
        || wire.session_order.len() != wire.capability_by_session.len()
        || wire.turn_order.len() != wire.structure_by_turn.len()
        || !session_order_is_exact
        || !turn_order_is_exact
        || wire
            .capability_by_session
            .keys()
            .chain(wire.structure_by_turn.keys())
            .any(|key| !valid_sha256(key))
    {
        return Err("request_learning_checkpoint_invalid".to_owned());
    }
    Ok(())
}

fn counters_from_status(status: &RequestLearningStatusV2) -> RequestLearningCounters {
    RequestLearningCounters {
        structures_applied: AtomicU64::new(status.structures_applied),
        session_inserts: AtomicU64::new(status.session_inserts),
        session_updates: AtomicU64::new(status.session_updates),
        turn_inserts: AtomicU64::new(status.turn_inserts),
        turn_updates: AtomicU64::new(status.turn_updates),
        evictions: AtomicU64::new(status.evictions),
        lookup_attempts: AtomicU64::new(status.lookup_attempts),
        lookup_hits: AtomicU64::new(status.lookup_hits),
        lookup_misses: AtomicU64::new(status.lookup_misses),
    }
}

fn strictly_ordered<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
