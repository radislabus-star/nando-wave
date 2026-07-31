use std::collections::{BTreeMap, BTreeSet};

use nando_client_evidence::NandoRouteReceiptV1;
use nando_operator_kernel::{RelationFrame, canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    BlindThenRevealJoinedTransitionV1, PreActionTopologyAuditRowV1,
    join::{MultiSourceJoinCensoredReasonV1, join_explicit_topology_to_frame},
};

pub const TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1: &str = "nando.transport-terminal-receipt.v1";
pub const REQUEST_ACTION_BINDING_SCHEMA_V1: &str = "nando.request-action-binding.v1";
pub const TRANSPORT_BOUND_JOIN_MAX_ROWS_V1: usize = 16_384;
const CAPTURE_CLOCK_SKEW_NANOS: u64 = 2_000_000_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransportTerminalReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub request_event_id_sha256: String,
    pub started_at_unix_nanos: u64,
    pub completed_at_unix_nanos: u64,
    pub status: u16,
    pub successful: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestActionBindingV1 {
    pub schema: String,
    pub binding_root_sha256: String,
    pub topology_commitment_root_sha256: String,
    pub request_event_id_sha256: String,
    pub terminal_receipt_root_sha256: String,
    pub completed_frame_root_sha256: String,
    pub action_event_id_sha256: String,
    pub turn_intent_id_sha256: String,
    pub session_lineage_sha256: String,
    pub request_started_at_unix_nanos: u64,
    pub request_completed_at_unix_nanos: u64,
    pub action_observed_at_unix_nanos: u64,
    pub next_request_started_at_unix_nanos: Option<u64>,
    pub unique_response_interval: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportBoundJoinedTransitionV1 {
    pub binding: RequestActionBindingV1,
    pub joined: BlindThenRevealJoinedTransitionV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportBindingFailureV1 {
    TerminalReceiptMissing,
    TerminalReceiptInvalid,
    TerminalRequestFailed,
    CaptureOutsideRequestInterval,
    CompletedFrameMissing,
    ResponseIntervalAmbiguous,
    IdentityMismatch,
    JoinRejected,
    CapacityExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportJoinRejectionV1 {
    pub reason: MultiSourceJoinCensoredReasonV1,
    pub completed_frame_root_sha256: String,
    pub topology_captured_at_unix_nanos: u64,
    pub action_observed_at_unix_nanos: u64,
    pub turn_intent_matches: bool,
    pub session_identity_matches: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransportBindingLedgerV1 {
    bound_by_topology: BTreeMap<String, Vec<TransportBoundJoinedTransitionV1>>,
    failures_by_topology: BTreeMap<String, TransportBindingFailureV1>,
    join_rejections_by_topology: BTreeMap<String, TransportJoinRejectionV1>,
}

impl TransportTerminalReceiptV1 {
    pub fn seal(
        request_event_id_sha256: String,
        started_at_unix_nanos: u64,
        completed_at_unix_nanos: u64,
        status: u16,
    ) -> Result<Self, &'static str> {
        let successful = (200..300).contains(&status);
        let mut receipt = Self {
            schema: TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            request_event_id_sha256,
            started_at_unix_nanos,
            completed_at_unix_nanos,
            status,
            successful,
        };
        receipt.receipt_root_sha256 = receipt.expected_root();
        receipt
            .validate()
            .then_some(receipt)
            .ok_or("terminal_receipt_invalid")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1
            && valid_nonzero_sha256(&self.receipt_root_sha256)
            && valid_nonzero_sha256(&self.request_event_id_sha256)
            && self.started_at_unix_nanos > 0
            && self.completed_at_unix_nanos >= self.started_at_unix_nanos
            && self.successful == (200..300).contains(&self.status)
            && self.receipt_root_sha256 == self.expected_root()
    }

    fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            TRANSPORT_TERMINAL_RECEIPT_SCHEMA_V1,
            self.request_event_id_sha256.as_str(),
            self.started_at_unix_nanos,
            self.completed_at_unix_nanos,
            self.status,
            self.successful,
        ))
        .expect("terminal receipt serializes")
    }
}

impl RequestActionBindingV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == REQUEST_ACTION_BINDING_SCHEMA_V1
            && [
                &self.binding_root_sha256,
                &self.topology_commitment_root_sha256,
                &self.request_event_id_sha256,
                &self.terminal_receipt_root_sha256,
                &self.completed_frame_root_sha256,
                &self.action_event_id_sha256,
                &self.turn_intent_id_sha256,
                &self.session_lineage_sha256,
            ]
            .into_iter()
            .all(|root| valid_nonzero_sha256(root))
            && self.request_started_at_unix_nanos > 0
            && self.request_completed_at_unix_nanos >= self.request_started_at_unix_nanos
            && self.action_observed_at_unix_nanos >= self.request_started_at_unix_nanos
            && self.next_request_started_at_unix_nanos.is_none_or(|next| {
                next > self.request_started_at_unix_nanos
                    && self.action_observed_at_unix_nanos < next
            })
            && self.unique_response_interval
            && self.binding_root_sha256 == self.expected_root()
    }

    fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            REQUEST_ACTION_BINDING_SCHEMA_V1,
            self.topology_commitment_root_sha256.as_str(),
            self.request_event_id_sha256.as_str(),
            self.terminal_receipt_root_sha256.as_str(),
            self.completed_frame_root_sha256.as_str(),
            self.action_event_id_sha256.as_str(),
            self.turn_intent_id_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.request_started_at_unix_nanos,
            self.request_completed_at_unix_nanos,
            self.action_observed_at_unix_nanos,
            self.next_request_started_at_unix_nanos,
            self.unique_response_interval,
        ))
        .expect("request-action binding serializes")
    }
}

impl TransportBindingLedgerV1 {
    #[must_use]
    pub fn build(
        topologies: &[PreActionTopologyAuditRowV1],
        frames: &[RelationFrame],
        terminals: &[TransportTerminalReceiptV1],
    ) -> Self {
        let terminal_by_request = validated_terminals(terminals);
        let mut ledger = Self::default();
        let mut eligible_by_turn = BTreeMap::<
            &str,
            Vec<(&PreActionTopologyAuditRowV1, &TransportTerminalReceiptV1)>,
        >::new();

        for topology in topologies {
            let topology_root = topology.commit.commitment_root_sha256.clone();
            let Some(terminal) = terminal_by_request
                .get(topology.structure.request_event_id_sha256.as_str())
                .copied()
            else {
                ledger.failures_by_topology.insert(
                    topology_root,
                    TransportBindingFailureV1::TerminalReceiptMissing,
                );
                continue;
            };
            if !terminal.validate() {
                ledger.failures_by_topology.insert(
                    topology_root,
                    TransportBindingFailureV1::TerminalReceiptInvalid,
                );
                continue;
            }
            if !terminal.successful {
                ledger.failures_by_topology.insert(
                    topology_root,
                    TransportBindingFailureV1::TerminalRequestFailed,
                );
                continue;
            }
            if !capture_belongs_to_request(topology, terminal) {
                ledger.failures_by_topology.insert(
                    topology_root,
                    TransportBindingFailureV1::CaptureOutsideRequestInterval,
                );
                continue;
            }
            eligible_by_turn
                .entry(topology.structure.turn_intent_id_sha256.as_str())
                .or_default()
                .push((topology, terminal));
        }
        for rows in eligible_by_turn.values_mut() {
            rows.sort_by_key(|(topology, terminal)| {
                (
                    terminal.started_at_unix_nanos,
                    topology.commit.commitment_root_sha256.as_str(),
                )
            });
        }

        for frame in frames {
            let Ok(frame_root) = canonical_json_sha256(frame) else {
                continue;
            };
            let Some(rows) = eligible_by_turn.get(frame.client_intent_id_sha256.as_str()) else {
                continue;
            };
            if rows
                .windows(2)
                .any(|pair| pair[0].1.completed_at_unix_nanos > pair[1].1.started_at_unix_nanos)
            {
                for (topology, _) in rows {
                    ledger.failures_by_topology.insert(
                        topology.commit.commitment_root_sha256.clone(),
                        TransportBindingFailureV1::ResponseIntervalAmbiguous,
                    );
                }
                continue;
            }
            let candidates = response_interval_candidates(rows, frame);
            if candidates.len() != 1 {
                let failure = if candidates.is_empty() {
                    TransportBindingFailureV1::CompletedFrameMissing
                } else {
                    TransportBindingFailureV1::ResponseIntervalAmbiguous
                };
                for (topology, _) in rows {
                    ledger
                        .failures_by_topology
                        .entry(topology.commit.commitment_root_sha256.clone())
                        .or_insert(failure);
                }
                continue;
            }
            if ledger.bound_count() >= TRANSPORT_BOUND_JOIN_MAX_ROWS_V1 {
                let (topology, _, _) = candidates[0];
                ledger.failures_by_topology.insert(
                    topology.commit.commitment_root_sha256.clone(),
                    TransportBindingFailureV1::CapacityExhausted,
                );
                break;
            }
            let (topology, terminal, next_start) = candidates[0];
            let Some(session_lineage_sha256) = topology.session_lineage_sha256.clone() else {
                ledger.failures_by_topology.insert(
                    topology.commit.commitment_root_sha256.clone(),
                    TransportBindingFailureV1::IdentityMismatch,
                );
                continue;
            };
            let joined = match join_explicit_topology_to_frame(topology, frame) {
                Ok(joined) => joined,
                Err(reason) => {
                    ledger.failures_by_topology.insert(
                        topology.commit.commitment_root_sha256.clone(),
                        TransportBindingFailureV1::JoinRejected,
                    );
                    ledger.join_rejections_by_topology.insert(
                        topology.commit.commitment_root_sha256.clone(),
                        TransportJoinRejectionV1 {
                            reason,
                            completed_frame_root_sha256: frame_root.clone(),
                            topology_captured_at_unix_nanos: topology
                                .captured_at_unix_ms
                                .unwrap_or_default()
                                .saturating_mul(1_000_000),
                            action_observed_at_unix_nanos: frame.observed_at_unix_nanos,
                            turn_intent_matches: topology.structure.turn_intent_id_sha256
                                == frame.client_intent_id_sha256,
                            session_identity_matches: topology
                                .structure
                                .session_lineage_roots_sha256
                                .contains(&frame.session_id_sha256),
                        },
                    );
                    continue;
                }
            };
            let mut binding = RequestActionBindingV1 {
                schema: REQUEST_ACTION_BINDING_SCHEMA_V1.to_owned(),
                binding_root_sha256: String::new(),
                topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
                request_event_id_sha256: topology.structure.request_event_id_sha256.clone(),
                terminal_receipt_root_sha256: terminal.receipt_root_sha256.clone(),
                completed_frame_root_sha256: frame_root,
                action_event_id_sha256: frame.event_id_sha256.clone(),
                turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
                session_lineage_sha256,
                request_started_at_unix_nanos: terminal.started_at_unix_nanos,
                request_completed_at_unix_nanos: terminal.completed_at_unix_nanos,
                action_observed_at_unix_nanos: frame.observed_at_unix_nanos,
                next_request_started_at_unix_nanos: next_start,
                unique_response_interval: true,
            };
            binding.binding_root_sha256 = binding.expected_root();
            if !binding.validate() {
                ledger.failures_by_topology.insert(
                    topology.commit.commitment_root_sha256.clone(),
                    TransportBindingFailureV1::IdentityMismatch,
                );
                continue;
            }
            ledger
                .failures_by_topology
                .remove(topology.commit.commitment_root_sha256.as_str());
            ledger
                .join_rejections_by_topology
                .remove(topology.commit.commitment_root_sha256.as_str());
            ledger
                .bound_by_topology
                .entry(topology.commit.commitment_root_sha256.clone())
                .or_default()
                .push(TransportBoundJoinedTransitionV1 { binding, joined });
        }

        // A topology with a valid terminal but no owned frame remains explicit.
        for rows in eligible_by_turn.values() {
            for (topology, _) in rows {
                let root = topology.commit.commitment_root_sha256.clone();
                if !ledger.bound_by_topology.contains_key(&root) {
                    ledger
                        .failures_by_topology
                        .entry(root)
                        .or_insert(TransportBindingFailureV1::CompletedFrameMissing);
                }
            }
        }
        ledger
    }

    #[must_use]
    pub fn bound_for_topology(
        &self,
        topology_root_sha256: &str,
    ) -> &[TransportBoundJoinedTransitionV1] {
        self.bound_by_topology
            .get(topology_root_sha256)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    #[must_use]
    pub fn failure_for_topology(
        &self,
        topology_root_sha256: &str,
    ) -> Option<TransportBindingFailureV1> {
        self.failures_by_topology.get(topology_root_sha256).copied()
    }

    #[must_use]
    pub fn join_rejection_for_topology(
        &self,
        topology_root_sha256: &str,
    ) -> Option<&TransportJoinRejectionV1> {
        self.join_rejections_by_topology.get(topology_root_sha256)
    }

    #[must_use]
    pub fn bound_count(&self) -> usize {
        self.bound_by_topology.values().map(Vec::len).sum()
    }

    #[must_use]
    pub fn failure_counts(&self) -> BTreeMap<TransportBindingFailureV1, u64> {
        self.failures_by_topology
            .values()
            .copied()
            .fold(BTreeMap::new(), |mut counts, failure| {
                *counts.entry(failure).or_default() += 1;
                counts
            })
    }
}

/// Bind an independently observed upstream action after Nando explicitly handed the
/// request off with HTTP 418. The ordinary transport ledger keeps treating 418 as
/// unsuccessful; only this route-receipt-bound proof path may cross that boundary.
pub fn bind_independent_fallback_transition_v1(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
    terminal: &TransportTerminalReceiptV1,
    route_receipt: &NandoRouteReceiptV1,
) -> Result<TransportBoundJoinedTransitionV1, &'static str> {
    let frame_root = canonical_json_sha256(frame).map_err(|_| "fallback_frame_root_invalid")?;
    let session_lineage_sha256 = topology
        .session_lineage_sha256
        .clone()
        .ok_or("fallback_session_lineage_missing")?;
    if !terminal.validate()
        || terminal.status != 418
        || terminal.successful
        || !route_receipt.validate()
        || route_receipt.remote_status != 418
        || topology.structure.request_event_id_sha256 != terminal.request_event_id_sha256
        || topology.structure.provider_capture_request_root_sha256
            != route_receipt.request_body_sha256
        || topology.structure.turn_intent_id_sha256 != route_receipt.turn_intent_id_sha256
        || topology.structure.turn_intent_id_sha256 != frame.client_intent_id_sha256
        || route_receipt.session_id_sha256 != frame.session_id_sha256
        || !topology
            .structure
            .session_lineage_roots_sha256
            .contains(&frame.session_id_sha256)
        || !capture_belongs_to_request(topology, terminal)
        || route_receipt
            .request_observed_at_unix_nanos
            .saturating_add(CAPTURE_CLOCK_SKEW_NANOS)
            < terminal.started_at_unix_nanos
        || route_receipt
            .route_confirmed_at_unix_nanos
            .saturating_add(CAPTURE_CLOCK_SKEW_NANOS)
            < terminal.completed_at_unix_nanos
        || route_receipt.route_confirmed_at_unix_nanos > frame.observed_at_unix_nanos
    {
        return Err("fallback_transport_identity_mismatch");
    }
    let joined = join_explicit_topology_to_frame(topology, frame)
        .map_err(|_| "fallback_transport_join_rejected")?;
    let mut binding = RequestActionBindingV1 {
        schema: REQUEST_ACTION_BINDING_SCHEMA_V1.to_owned(),
        binding_root_sha256: String::new(),
        topology_commitment_root_sha256: topology.commit.commitment_root_sha256.clone(),
        request_event_id_sha256: topology.structure.request_event_id_sha256.clone(),
        terminal_receipt_root_sha256: terminal.receipt_root_sha256.clone(),
        completed_frame_root_sha256: frame_root,
        action_event_id_sha256: frame.event_id_sha256.clone(),
        turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        session_lineage_sha256,
        request_started_at_unix_nanos: terminal.started_at_unix_nanos,
        request_completed_at_unix_nanos: terminal.completed_at_unix_nanos,
        action_observed_at_unix_nanos: frame.observed_at_unix_nanos,
        next_request_started_at_unix_nanos: None,
        unique_response_interval: true,
    };
    binding.binding_root_sha256 = binding.expected_root();
    binding
        .validate()
        .then_some(TransportBoundJoinedTransitionV1 { binding, joined })
        .ok_or("fallback_transport_binding_invalid")
}

/// Revalidate a frozen request/action binding without reconstructing the full
/// response-interval cohort that supplied its next-request fence.
pub fn validate_request_action_binding_v1(
    topology: &PreActionTopologyAuditRowV1,
    frame: &RelationFrame,
    terminal: &TransportTerminalReceiptV1,
    binding: &RequestActionBindingV1,
) -> Result<(), &'static str> {
    let frame_root = canonical_json_sha256(frame).map_err(|_| "transport_frame_root_invalid")?;
    if !terminal.validate()
        || !terminal.successful
        || !binding.validate()
        || binding.topology_commitment_root_sha256 != topology.commit.commitment_root_sha256
        || binding.request_event_id_sha256 != topology.structure.request_event_id_sha256
        || binding.request_event_id_sha256 != terminal.request_event_id_sha256
        || binding.terminal_receipt_root_sha256 != terminal.receipt_root_sha256
        || binding.completed_frame_root_sha256 != frame_root
        || binding.action_event_id_sha256 != frame.event_id_sha256
        || binding.turn_intent_id_sha256 != topology.structure.turn_intent_id_sha256
        || binding.turn_intent_id_sha256 != frame.client_intent_id_sha256
        || topology.session_lineage_sha256.as_deref()
            != Some(binding.session_lineage_sha256.as_str())
        || !topology
            .structure
            .session_lineage_roots_sha256
            .contains(&frame.session_id_sha256)
        || binding.request_started_at_unix_nanos != terminal.started_at_unix_nanos
        || binding.request_completed_at_unix_nanos != terminal.completed_at_unix_nanos
        || binding.action_observed_at_unix_nanos != frame.observed_at_unix_nanos
        || !capture_belongs_to_request(topology, terminal)
        || join_explicit_topology_to_frame(topology, frame).is_err()
    {
        return Err("transport_frozen_binding_mismatch");
    }
    Ok(())
}

fn validated_terminals(
    terminals: &[TransportTerminalReceiptV1],
) -> BTreeMap<&str, &TransportTerminalReceiptV1> {
    let mut conflicts = BTreeSet::new();
    let mut by_request = BTreeMap::new();
    for terminal in terminals.iter().filter(|terminal| terminal.validate()) {
        let request = terminal.request_event_id_sha256.as_str();
        if by_request.insert(request, terminal).is_some() {
            conflicts.insert(request);
        }
    }
    for conflict in conflicts {
        by_request.remove(conflict);
    }
    by_request
}

fn capture_belongs_to_request(
    topology: &PreActionTopologyAuditRowV1,
    terminal: &TransportTerminalReceiptV1,
) -> bool {
    let Some(captured_at) = topology
        .captured_at_unix_ms
        .map(|value| value.saturating_mul(1_000_000))
    else {
        return false;
    };
    captured_at.saturating_add(CAPTURE_CLOCK_SKEW_NANOS) >= terminal.started_at_unix_nanos
        && captured_at
            <= terminal
                .completed_at_unix_nanos
                .saturating_add(CAPTURE_CLOCK_SKEW_NANOS)
}

fn response_interval_candidates<'a>(
    rows: &'a [(
        &'a PreActionTopologyAuditRowV1,
        &'a TransportTerminalReceiptV1,
    )],
    frame: &RelationFrame,
) -> Vec<(
    &'a PreActionTopologyAuditRowV1,
    &'a TransportTerminalReceiptV1,
    Option<u64>,
)> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, (topology, terminal))| {
            if !topology
                .structure
                .session_lineage_roots_sha256
                .contains(&frame.session_id_sha256)
                || frame.observed_at_unix_nanos < terminal.started_at_unix_nanos
            {
                return None;
            }
            let next_start = rows
                .get(index.saturating_add(1))
                .map(|(_, next)| next.started_at_unix_nanos);
            let overlaps_previous = index.checked_sub(1).is_some_and(|previous| {
                rows[previous].1.completed_at_unix_nanos > terminal.started_at_unix_nanos
            });
            let overlaps_next =
                next_start.is_some_and(|next| terminal.completed_at_unix_nanos > next);
            if overlaps_previous || overlaps_next {
                return None;
            }
            if next_start.is_some_and(|next| frame.observed_at_unix_nanos >= next) {
                return None;
            }
            Some((*topology, *terminal, next_start))
        })
        .collect()
}
