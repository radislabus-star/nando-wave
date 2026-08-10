use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{
    AdvanceInput, EvidenceBindingAccumulator, K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1,
    K1NaturalSchedulerRuntimeReportV1, K1NaturalSchedulerRuntimeStateV1 as RuntimeState,
    K1SchedulerLaneV1, K1SchedulerProjectionV1, MULTI_SOURCE_JOIN_MAX_ROWS_V1,
    MultiSourceJoinCensoredReasonV1, MultiSourceJoinReportV1, PreActionTopologyAuditRowV1,
    PreparedK1TickContextV1, RelationFrame, advance, current_deficit_snapshot,
    extend_prepared_tick_context, join_prepared_multi_source_frame_v1,
    law_lab_eligibility::law_lab_eligibility_report,
    prepare_multi_source_join_frame_v1, prepare_tick_context_from_bindings, restore_projection_for,
    stream_multi_source_joins_from_iter,
    structural_frontier_census::{
        build_report as build_frontier_report, publish_report as publish_frontier_report,
        source_root as frontier_source_root,
    },
    validate_pre_action_topology_join_eligibility_v1,
};
#[cfg(test)]
use super::{MultiSourceJoinLedgerV1, prepare_tick_context_from_join_ledger};
use crate::k1_transfer_lifecycle::{K1TransferLifecycleReportV1, advance_transfer_lifecycle};
use crate::{AppState, json_response, multi_source_live, unix_now};

pub(crate) async fn report_handler(State(state): State<AppState>) -> Response {
    report_response(&state.k1_natural_scheduler_report)
}

pub(crate) async fn mechanism_report_handler(State(state): State<AppState>) -> Response {
    report_response(&state.k1_mechanism_watch_report)
}

pub(crate) async fn law_lab_eligibility_report_handler(State(state): State<AppState>) -> Response {
    let report = restore_projection_for(
        &state.operator_certification_config,
        K1SchedulerLaneV1::Epistemic,
    )
    .and_then(|projection| {
        law_lab_eligibility_report(
            &projection,
            None,
            None,
            state.config.multi_source_research_enabled,
            unix_now(),
        )
    });
    match report {
        Ok(report) => json_response(
            StatusCode::OK,
            serde_json::to_value(report).unwrap_or_else(|_| {
                json!({
                    "schema": "nando.law-lab-k1-eligibility-error.v1",
                    "error": "report_encode"
                })
            }),
        ),
        Err(error) => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.law-lab-k1-eligibility-error.v1",
                "error": error
            }),
        ),
    }
}

fn report_response(
    slot: &std::sync::RwLock<Option<K1NaturalSchedulerRuntimeReportV1>>,
) -> Response {
    match slot.read() {
        Ok(report) => match report.as_ref() {
            Some(report) => json_response(
                StatusCode::OK,
                serde_json::to_value(report).unwrap_or_else(|_| {
                    json!({
                        "schema": "nando.k1-natural-scheduler-error.v1",
                        "error": "report_encode"
                    })
                }),
            ),
            None => json_response(
                StatusCode::SERVICE_UNAVAILABLE,
                json!({
                    "schema": "nando.k1-natural-scheduler-error.v1",
                    "error": "runtime_pending"
                }),
            ),
        },
        Err(_) => json_response(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({
                "schema": "nando.k1-natural-scheduler-error.v1",
                "error": "report_lock_poisoned"
            }),
        ),
    }
}

#[derive(Default)]
pub(crate) struct K1EvidenceCursorV1 {
    initialized: bool,
    topology_rows: usize,
    frame_rows: usize,
    frame_intent_ids_sha256: BTreeSet<String>,
    active_protocol_mode_set_root_sha256: String,
    retain_safety_payloads: bool,
    prepared: Option<PreparedK1TickContextV1>,
}

impl K1EvidenceCursorV1 {
    fn record(
        &mut self,
        topology_rows: usize,
        frame_rows: usize,
        frame_intent_ids_sha256: BTreeSet<String>,
        active_protocol_mode_set_root_sha256: String,
        retain_safety_payloads: bool,
    ) {
        self.initialized = true;
        self.topology_rows = topology_rows;
        self.frame_rows = frame_rows;
        self.frame_intent_ids_sha256 = frame_intent_ids_sha256;
        self.active_protocol_mode_set_root_sha256 = active_protocol_mode_set_root_sha256;
        self.retain_safety_payloads = retain_safety_payloads;
    }
}

pub(crate) fn advance_state(
    state: &AppState,
    evidence_cursor: &mut K1EvidenceCursorV1,
) -> Result<(), String> {
    let active_protocols =
        multi_source_live::active_protocol_mode_roots(&state.config.response_registry_path)?;
    let active_protocol_mode_set_root_sha256 =
        crate::k1_natural_scheduler::duplicate_cohorts::active_protocol_mode_set_root(
            &active_protocols,
        )?;
    let epistemic_projection = restore_projection_for(
        &state.operator_certification_config,
        K1SchedulerLaneV1::Epistemic,
    )?;
    let deficit = current_deficit_snapshot(&state.operator_certification_config)?;
    let mechanism_terminal = mechanism_watch_is_terminal(state)?;
    let mechanism_projection = (!mechanism_terminal)
        .then(|| {
            restore_projection_for(
                &state.operator_certification_config,
                K1SchedulerLaneV1::Mechanism,
            )
        })
        .transpose()?;
    let retain_safety_payloads =
        legacy_safety_payloads_required(&epistemic_projection, mechanism_projection.as_ref());
    if reuse_waiting_tick(
        state,
        evidence_cursor,
        mechanism_terminal,
        &active_protocol_mode_set_root_sha256,
        &epistemic_projection.projection_root_sha256,
        &deficit.snapshot_root_sha256,
        retain_safety_payloads,
    )? {
        return Ok(());
    }

    refresh_prepared_context(
        state,
        evidence_cursor,
        &active_protocols,
        retain_safety_payloads,
    )?;
    let candidate_artifacts = crate::current_collection_miner(state)
        .map(|miner| {
            miner
                .lock()
                .map_err(|_| "k1_scheduler_collection_miner_lock_poisoned".to_owned())?
                .natural_t1_program_artifacts()
        })
        .transpose()?
        .unwrap_or_default();
    let evidence_snapshot_required = epistemic_projection.active_candidate_freeze.is_some()
        || !epistemic_projection.future_predictions.is_empty()
        || mechanism_projection
            .as_ref()
            .is_some_and(|projection| projection.active_candidate_freeze.is_some());
    let (mut topologies, mut frames) = if evidence_snapshot_required {
        materialize_current_evidence(state)?
    } else {
        (Vec::new(), Vec::new())
    };
    let prepared = evidence_cursor
        .prepared
        .as_ref()
        .ok_or_else(|| "k1_scheduler_prepared_context_missing".to_owned())?;
    if !mechanism_terminal {
        let mechanism = advance(
            &state.operator_certification_config,
            K1SchedulerLaneV1::Mechanism,
            false,
            AdvanceInput {
                prepared,
                topologies: &topologies,
                frames: &frames,
                terminal_receipts: &[],
                active_protocol_mode_roots_sha256: &active_protocols,
                candidate_artifacts: &candidate_artifacts,
                generated_at_unix: unix_now(),
            },
        )?;
        store_mechanism_report(state, mechanism)?;
    }
    let pending_request_ids = epistemic_projection
        .future_predictions
        .iter()
        .filter(|prediction| {
            !epistemic_projection
                .future_outcomes
                .iter()
                .any(|outcome| outcome.prediction_root_sha256 == prediction.prediction_root_sha256)
                && !epistemic_projection
                    .future_prediction_censors
                    .iter()
                    .any(|receipt| {
                        receipt.prediction_root_sha256 == prediction.prediction_root_sha256
                    })
        })
        .filter_map(|prediction| {
            topologies.iter().find(|topology| {
                topology.commit.commitment_root_sha256 == prediction.topology_commitment_root_sha256
            })
        })
        .map(|topology| topology.structure.request_event_id_sha256.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let terminal_receipts = state
        .terminal_receipt_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_terminal_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_terminal_archive_lock_poisoned".to_owned())?
        .receipts_for_requests(&pending_request_ids);
    for _ in 0..16 {
        let now = unix_now();
        let mut report = advance(
            &state.operator_certification_config,
            K1SchedulerLaneV1::Epistemic,
            true,
            AdvanceInput {
                prepared,
                topologies: &topologies,
                frames: &frames,
                terminal_receipts: &terminal_receipts,
                active_protocol_mode_roots_sha256: &active_protocols,
                candidate_artifacts: &candidate_artifacts,
                generated_at_unix: now,
            },
        )?;
        if report.state == RuntimeState::TerminalPass {
            trigger_candidate_publication(state);
            continue;
        }
        if report.state == RuntimeState::AwaitingCertification {
            trigger_candidate_publication(state);
            let terminal = report
                .projection
                .pending_terminal_transfer
                .as_ref()
                .ok_or_else(|| "k1_transfer_terminal_projection_missing".to_owned())?;
            let lifecycle = match retained_transitions(state).and_then(|transitions| {
                advance_transfer_lifecycle(
                    &state.operator_certification_config,
                    &state.config.ms4_ordinary_economics_path,
                    terminal,
                    &transitions,
                    now,
                )
            }) {
                Ok(lifecycle) => lifecycle,
                Err(error) => {
                    eprintln!("nando-k1-transfer-lifecycle: {error}");
                    K1TransferLifecycleReportV1::pending(terminal, now, error)?
                }
            };
            if lifecycle.settled() {
                continue;
            }
            report.attach_transfer_lifecycle(lifecycle)?;
            store_report(state, prepared, report)?;
            return Ok(());
        }
        let stable = matches!(
            report.state,
            RuntimeState::WaitingForEvidence
                | RuntimeState::ProbePending
                | RuntimeState::AwaitingIndependentFuture
                | RuntimeState::TerminalAbstain
                | RuntimeState::TerminalAcquisitionFail
                | RuntimeState::TerminalIndependentFutureNotObserved
                | RuntimeState::TerminalProbeExhausted
                | RuntimeState::K1VocabularyOpen
                | RuntimeState::MechanismWatchComplete
        );
        store_report(state, prepared, report)?;
        if stable {
            return Ok(());
        }
        if topologies.is_empty() && frames.is_empty() {
            (topologies, frames) = materialize_current_evidence(state)?;
        }
    }
    Err("k1_scheduler_tick_budget_exhausted".to_owned())
}

fn refresh_prepared_context(
    state: &AppState,
    evidence_cursor: &mut K1EvidenceCursorV1,
    active_protocols: &BTreeSet<String>,
    retain_safety_payloads: bool,
) -> Result<(), String> {
    let active_protocol_root =
        crate::k1_natural_scheduler::duplicate_cohorts::active_protocol_mode_set_root(
            active_protocols,
        )?;
    let (topology_rows, appended_since_cursor) = {
        let archive = state
            .multi_source_topology_archive
            .as_ref()
            .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
            .lock()
            .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?;
        let rows = archive.len();
        let appended = if evidence_cursor.initialized && rows >= evidence_cursor.topology_rows {
            archive.shared_rows_after(evidence_cursor.topology_rows)?
        } else {
            Vec::new()
        };
        (rows, appended)
    };
    let frame_rows = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .len();

    let invalid_cursor = !evidence_cursor.initialized
        || evidence_cursor.prepared.is_none()
        || topology_rows < evidence_cursor.topology_rows
        || frame_rows < evidence_cursor.frame_rows
        || retain_safety_payloads != evidence_cursor.retain_safety_payloads;
    let late_topology = appended_since_cursor.iter().any(|row| {
        evidence_cursor
            .frame_intent_ids_sha256
            .contains(&row.structure.turn_intent_id_sha256)
    });
    if invalid_cursor || late_topology {
        return rebuild_prepared_context(
            state,
            evidence_cursor,
            active_protocols,
            retain_safety_payloads,
        );
    }

    if frame_rows == evidence_cursor.frame_rows {
        evidence_cursor.topology_rows = topology_rows;
        if evidence_cursor.active_protocol_mode_set_root_sha256 != active_protocol_root {
            let prepared = evidence_cursor
                .prepared
                .as_mut()
                .ok_or_else(|| "k1_scheduler_prepared_context_missing".to_owned())?;
            prepared.active_protocol_mode_set_root_sha256 = active_protocol_root.clone();
            evidence_cursor.active_protocol_mode_set_root_sha256 = active_protocol_root;
        }
        return Ok(());
    }

    let new_frames = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .shared_frames_after(evidence_cursor.frame_rows)?;
    let frame_intents = new_frames
        .iter()
        .map(|frame| frame.client_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let prepared_rows = evidence_cursor
        .prepared
        .as_ref()
        .and_then(|prepared| usize::try_from(prepared.join_report.topology_rows).ok())
        .ok_or_else(|| "k1_scheduler_prepared_topology_count_invalid".to_owned())?;
    if prepared_rows > topology_rows {
        return rebuild_prepared_context(
            state,
            evidence_cursor,
            active_protocols,
            retain_safety_payloads,
        );
    }
    let (topologies, pending_topologies) = {
        let archive = state
            .multi_source_topology_archive
            .as_ref()
            .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
            .lock()
            .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?;
        let relevant = archive
            .shared_rows()
            .into_iter()
            .filter(|row| frame_intents.contains(&row.structure.turn_intent_id_sha256))
            .collect::<Vec<_>>();
        (relevant, archive.shared_rows_after(prepared_rows)?)
    };

    let incremental = extend_prepared_from_delta(
        evidence_cursor
            .prepared
            .as_mut()
            .ok_or_else(|| "k1_scheduler_prepared_context_missing".to_owned())?,
        topology_rows,
        frame_rows,
        &topologies,
        &pending_topologies,
        &new_frames,
        active_protocols,
    );
    if matches!(
        &incremental,
        Err(error) if error == "k1_incremental_evidence_out_of_order"
    ) {
        return rebuild_prepared_context(
            state,
            evidence_cursor,
            active_protocols,
            retain_safety_payloads,
        );
    }
    incremental?;

    let mut frame_intent_ids_sha256 = evidence_cursor.frame_intent_ids_sha256.clone();
    frame_intent_ids_sha256.extend(frame_intents);
    evidence_cursor.record(
        topology_rows,
        frame_rows,
        frame_intent_ids_sha256,
        active_protocol_root,
        retain_safety_payloads,
    );
    Ok(())
}

fn extend_prepared_from_delta(
    prepared: &mut PreparedK1TickContextV1,
    topology_rows: usize,
    frame_rows: usize,
    relevant_topologies: &[Arc<PreActionTopologyAuditRowV1>],
    pending_topologies: &[Arc<PreActionTopologyAuditRowV1>],
    new_frames: &[Arc<RelationFrame>],
    active_protocols: &BTreeSet<String>,
) -> Result<(), String> {
    let mut report = prepared.join_report.clone();
    report.topology_rows = u64::try_from(topology_rows).unwrap_or(u64::MAX);
    report.completed_frames = u64::try_from(frame_rows).unwrap_or(u64::MAX);
    for row in pending_topologies {
        if let Err(reason) = validate_pre_action_topology_join_eligibility_v1(row) {
            increment_censor(&mut report, reason);
        }
    }

    let mut eligible_by_intent = BTreeMap::<String, Vec<&PreActionTopologyAuditRowV1>>::new();
    for row in relevant_topologies {
        if validate_pre_action_topology_join_eligibility_v1(row).is_ok() {
            eligible_by_intent
                .entry(row.structure.turn_intent_id_sha256.clone())
                .or_default()
                .push(row.as_ref());
        }
    }
    let mut used_completed_frames = prepared
        .bindings
        .iter()
        .map(|binding| binding.completed_frame_root_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut joined_roots = prepared
        .bindings
        .iter()
        .map(|binding| binding.join_root_sha256().to_owned())
        .collect::<BTreeSet<_>>();
    let mut joined_rows = Vec::new();
    let capacity_already_exhausted = report
        .censored
        .get(&MultiSourceJoinCensoredReasonV1::CapacityExhausted)
        .copied()
        .unwrap_or(0)
        > 0;
    for frame in new_frames {
        if capacity_already_exhausted {
            break;
        }
        if usize::try_from(report.joined_rows).unwrap_or(usize::MAX)
            >= MULTI_SOURCE_JOIN_MAX_ROWS_V1
        {
            increment_censor(
                &mut report,
                MultiSourceJoinCensoredReasonV1::CapacityExhausted,
            );
            break;
        }
        let frame = match prepare_multi_source_join_frame_v1(frame) {
            Ok(frame) => frame,
            Err(reason) => {
                increment_censor(&mut report, reason);
                continue;
            }
        };
        if used_completed_frames.contains(&frame.action.completed_frame_root_sha256) {
            report.duplicate_idempotent = report.duplicate_idempotent.saturating_add(1);
            continue;
        }
        let same_intent = eligible_by_intent
            .get(frame.action.turn_intent_id_sha256.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let joined = match join_prepared_multi_source_frame_v1(&frame, same_intent) {
            Ok(joined) => joined,
            Err(reason) => {
                increment_censor(&mut report, reason);
                continue;
            }
        };
        if joined_roots.contains(&joined.join_root_sha256) {
            let idempotent = prepared
                .bindings
                .iter()
                .find(|binding| binding.join_root_sha256() == joined.join_root_sha256)
                .is_some_and(|binding| !binding.payload_retained() || binding.joined() == &joined)
                || joined_rows.iter().any(|existing| existing == &joined);
            if idempotent {
                report.duplicate_idempotent = report.duplicate_idempotent.saturating_add(1);
            } else {
                increment_censor(
                    &mut report,
                    MultiSourceJoinCensoredReasonV1::DuplicateConflict,
                );
            }
            continue;
        }
        used_completed_frames.insert(frame.action.completed_frame_root_sha256);
        joined_roots.insert(joined.join_root_sha256.clone());
        report.joined_rows = report.joined_rows.saturating_add(1);
        if joined.accepted {
            report.accepted_rows = report.accepted_rows.saturating_add(1);
        } else {
            report.negative_rows = report.negative_rows.saturating_add(1);
        }
        joined_rows.push(joined);
    }

    if joined_rows.is_empty() {
        prepared.join_report = report;
        prepared.active_protocol_mode_set_root_sha256 =
            crate::k1_natural_scheduler::duplicate_cohorts::active_protocol_mode_set_root(
                active_protocols,
            )?;
        return Ok(());
    }
    extend_prepared_tick_context(prepared, joined_rows, report, active_protocols)
}

fn increment_censor(report: &mut MultiSourceJoinReportV1, reason: MultiSourceJoinCensoredReasonV1) {
    let count = report.censored.entry(reason).or_default();
    *count = count.saturating_add(1);
}

fn rebuild_prepared_context(
    state: &AppState,
    evidence_cursor: &mut K1EvidenceCursorV1,
    active_protocols: &BTreeSet<String>,
    retain_safety_payloads: bool,
) -> Result<(), String> {
    let topologies = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?
        .shared_rows();
    let frames = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .shared_frames();
    let frame_intent_ids_sha256 = frames
        .iter()
        .map(|frame| frame.client_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let mut accumulator = EvidenceBindingAccumulator::new(retain_safety_payloads);
    let join_report = stream_multi_source_joins_from_iter(
        topologies.iter().map(|row| row.as_ref()),
        frames.iter().map(|frame| frame.as_ref()),
        |joined| accumulator.push(joined),
    )?;
    let bindings = accumulator.finish()?;
    let prepared = prepare_tick_context_from_bindings(
        join_report,
        bindings,
        active_protocols,
        retain_safety_payloads,
    )?;
    let active_protocol_root = prepared.active_protocol_mode_set_root_sha256.clone();
    evidence_cursor.prepared = Some(prepared);
    evidence_cursor.record(
        topologies.len(),
        frames.len(),
        frame_intent_ids_sha256,
        active_protocol_root,
        retain_safety_payloads,
    );
    Ok(())
}

fn materialize_current_evidence(
    state: &AppState,
) -> Result<(Vec<PreActionTopologyAuditRowV1>, Vec<RelationFrame>), String> {
    let topologies = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?
        .shared_rows();
    let frames = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .shared_frames();
    Ok(materialize_evidence(&topologies, &frames))
}

fn materialize_evidence(
    topologies: &[Arc<PreActionTopologyAuditRowV1>],
    frames: &[Arc<RelationFrame>],
) -> (Vec<PreActionTopologyAuditRowV1>, Vec<RelationFrame>) {
    (
        topologies.iter().map(|row| row.as_ref().clone()).collect(),
        frames.iter().map(|frame| frame.as_ref().clone()).collect(),
    )
}

fn reuse_waiting_tick(
    state: &AppState,
    evidence_cursor: &mut K1EvidenceCursorV1,
    mechanism_terminal: bool,
    active_protocol_mode_set_root_sha256: &str,
    projection_root_sha256: &str,
    deficit_snapshot_root_sha256: &str,
    retain_safety_payloads: bool,
) -> Result<bool, String> {
    let report = state
        .k1_natural_scheduler_report
        .read()
        .map_err(|_| "k1_scheduler_report_lock_poisoned".to_owned())?;
    let Some(report) = report.as_ref() else {
        return Ok(false);
    };
    let Some(prepared) = evidence_cursor.prepared.as_ref() else {
        return Ok(false);
    };
    if !waiting_lanes_are_reusable(mechanism_terminal, report.state)
        || report.projection.projection_root_sha256 != projection_root_sha256
        || report.queue.k1_deficit_snapshot_root_sha256 != deficit_snapshot_root_sha256
        || report.join != prepared.join_report
        || report.catalog.catalog_root_sha256 != prepared.catalog.catalog_root_sha256
        || evidence_cursor.active_protocol_mode_set_root_sha256
            != active_protocol_mode_set_root_sha256
        || evidence_cursor.retain_safety_payloads != retain_safety_payloads
    {
        return Ok(false);
    }
    let frame_rows = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .len();
    let topology_archive = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?;
    let topology_rows = topology_archive.len();
    let appended = if evidence_cursor.initialized && topology_rows >= evidence_cursor.topology_rows
    {
        topology_archive.shared_rows_after(evidence_cursor.topology_rows)?
    } else {
        Vec::new()
    };
    if waiting_delta_requires_rebuild(
        evidence_cursor,
        topology_rows,
        frame_rows,
        appended
            .iter()
            .map(|row| row.structure.turn_intent_id_sha256.as_str()),
    ) {
        return Ok(false);
    }
    evidence_cursor.topology_rows = topology_rows;
    Ok(true)
}

fn legacy_safety_payloads_required(
    epistemic: &K1SchedulerProjectionV1,
    mechanism: Option<&K1SchedulerProjectionV1>,
) -> bool {
    std::iter::once(epistemic)
        .chain(mechanism)
        .filter_map(|projection| projection.active_candidate_freeze.as_ref())
        .any(|freeze| freeze.schema == K1_NATURAL_CANDIDATE_FREEZE_SCHEMA_V1)
}

fn waiting_lanes_are_reusable(mechanism_terminal: bool, epistemic_state: RuntimeState) -> bool {
    mechanism_terminal && epistemic_state == RuntimeState::WaitingForEvidence
}

fn waiting_delta_requires_rebuild<'a>(
    evidence_cursor: &K1EvidenceCursorV1,
    topology_rows: usize,
    frame_rows: usize,
    appended_topology_intents_sha256: impl IntoIterator<Item = &'a str>,
) -> bool {
    !evidence_cursor.initialized
        || topology_rows < evidence_cursor.topology_rows
        || frame_rows != evidence_cursor.frame_rows
        || appended_topology_intents_sha256
            .into_iter()
            .any(|intent| evidence_cursor.frame_intent_ids_sha256.contains(intent))
}

fn mechanism_watch_is_terminal(state: &AppState) -> Result<bool, String> {
    Ok(state
        .k1_mechanism_watch_report
        .read()
        .map_err(|_| "k1_mechanism_watch_report_lock_poisoned".to_owned())?
        .as_ref()
        .is_some_and(|report| report.state == RuntimeState::MechanismWatchComplete))
}

fn retained_transitions(
    state: &AppState,
) -> Result<Vec<nando_operator_learning::TeacherTransition>, String> {
    let miner = crate::current_response_miner(state)
        .ok_or_else(|| "k1_transfer_response_miner_pending".to_owned())?;
    let miner = miner
        .lock()
        .map_err(|_| "k1_transfer_response_miner_lock_poisoned".to_owned())?;
    Ok(miner.retained_teacher_transitions_for_multi_source_proof_v1())
}

fn trigger_candidate_publication(state: &AppState) {
    if let Ok(trigger) = state.authority_trigger.lock()
        && let Some(trigger) = trigger.as_ref()
    {
        let _ = trigger.try_send(());
    }
}

fn store_report(
    state: &AppState,
    prepared: &PreparedK1TickContextV1,
    report: K1NaturalSchedulerRuntimeReportV1,
) -> Result<(), String> {
    publish_frontier_if_changed(state, prepared, &report)?;
    *state
        .k1_natural_scheduler_report
        .write()
        .map_err(|_| "k1_scheduler_report_lock_poisoned".to_owned())? = Some(report);
    Ok(())
}

fn publish_frontier_if_changed(
    state: &AppState,
    prepared: &PreparedK1TickContextV1,
    runtime: &K1NaturalSchedulerRuntimeReportV1,
) -> Result<(), String> {
    let source_root = frontier_source_root(prepared, runtime)?;
    let root = state
        .config
        .multi_source_topology_archive_path
        .parent()
        .ok_or_else(|| "structural_frontier_root_parent_missing".to_owned())?
        .join("structural-frontier-census-v2");
    let already_published = state
        .k1_structural_frontier_source_root
        .read()
        .map_err(|_| "structural_frontier_source_root_lock_poisoned".to_owned())?
        .as_ref()
        .is_some_and(|published| published == &source_root)
        && root.join("latest.json").is_file();
    if already_published {
        return Ok(());
    }
    let report = build_frontier_report(prepared, runtime)?;
    if report.source_root_sha256 != source_root {
        return Err("structural_frontier_source_root_mismatch".to_owned());
    }
    publish_frontier_report(&root, &report)?;
    *state
        .k1_structural_frontier_source_root
        .write()
        .map_err(|_| "structural_frontier_source_root_lock_poisoned".to_owned())? =
        Some(source_root);
    Ok(())
}

fn store_mechanism_report(
    state: &AppState,
    report: K1NaturalSchedulerRuntimeReportV1,
) -> Result<(), String> {
    *state
        .k1_mechanism_watch_report
        .write()
        .map_err(|_| "k1_mechanism_watch_report_lock_poisoned".to_owned())? = Some(report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::{
        AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2,
        LearningRequestStructureV2, MultiSourceCardinalityClassV1, MultiSourceContainerClassV1,
        MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1, MultiSourceRoleNodeV1,
        MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
        PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1, RELATION_FRAME_SCHEMA,
        RelationAtom, sha256_bytes,
    };
    use nando_operator_learning::SOURCE_NEUTRAL_EXTRACTOR_VERSION;

    fn root(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn topology(label: &str, capture_sequence: u64) -> PreActionTopologyAuditRowV1 {
        let session_root = root(&format!("session-{label}"));
        let structure = LearningRequestStructureV2 {
            schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
            turn_intent_id_sha256: root(&format!("intent-{label}")),
            request_event_id_sha256: root(&format!("request-{label}")),
            provider_bound_turn_identity: true,
            session_lineage_roots_sha256: vec![session_root.clone()],
            request_phase_atom_ids: vec![capture_sequence.saturating_mul(10).saturating_add(1)],
            pre_action_context_atom_ids: vec![
                capture_sequence.saturating_mul(10).saturating_add(2),
            ],
            capability_atom_ids: vec![capture_sequence.saturating_mul(10).saturating_add(3)],
            estimated_input_tokens: 100,
            provider_payload_bytes: 400,
            provider_capture_request_root_sha256: root(&format!("provider-{label}")),
            decidability_reason_code: "pre_action_pending".to_owned(),
            topology: PreActionMultiSourceTopologyV1 {
                extraction_status: MultiSourceExtractionStatusV1::Complete,
                grounded_output_count: 1,
                output_part_count: 1,
                roles: vec![MultiSourceRoleNodeV1 {
                    local_role_id: 0,
                    source_ordinal: 0,
                    value_ordinal: 0,
                    type_class: MultiSourceTypeClassV1::Number,
                    container_class: MultiSourceContainerClassV1::Scalar,
                    cardinality_class: MultiSourceCardinalityClassV1::One,
                    temporal_class: MultiSourceTemporalClassV1::Latest,
                    depth_bucket: 1,
                    structural_flags: 1,
                }],
                role_witnesses: vec![MultiSourceRoleWitnessV1 {
                    local_role_id: 0,
                    value_sha256: root(&format!("value-{label}")),
                    request_reference_ordinal: None,
                    request_reference_ordinal_candidates: Vec::new(),
                }],
                relations: Vec::new(),
            },
        };
        let commit = PreActionTopologyCommitV1::seal(
            &structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            sha256_bytes(b"nando.multi-source-extractor.v2"),
            sha256_bytes(b"nando.multi-source-extractor-config.v2"),
            capture_sequence,
        )
        .expect("topology commit");
        PreActionTopologyAuditRowV1 {
            bridge_epoch_sha256: root("bridge-epoch"),
            bridge_sequence: Some(capture_sequence),
            record_sha256: Some(root(&format!("record-{label}"))),
            capture_epoch_sha256: Some(root("capture-epoch")),
            capture_event_sha256: Some(root(&format!("capture-event-{label}"))),
            capture_receipt_sha256: Some(root(&format!("capture-receipt-{label}"))),
            captured_at_unix_ms: Some(1_000_u64.saturating_add(capture_sequence)),
            session_lineage_sha256: Some(session_root),
            physical_order_proven: true,
            structure,
            commit,
        }
    }

    fn completed_frame(label: &str, capture_sequence: u64) -> RelationFrame {
        let observation_slot = u16::try_from(capture_sequence).unwrap_or(u16::MAX);
        let action_slot = observation_slot.saturating_add(100);
        let value_root = root(&format!("value-{label}"));
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: root(&format!("frame-{label}")),
            event_id_sha256: root(&format!("action-{label}")),
            client_intent_id_sha256: root(&format!("intent-{label}")),
            session_id_sha256: root(&format!("session-{label}")),
            observed_at_unix_nanos: 1_001_u64
                .saturating_add(capture_sequence)
                .saturating_mul(1_000_000),
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: observation_slot,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: value_root.clone(),
                },
                RelationAtom::TypedSlot {
                    slot_id: action_slot,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: value_root,
                },
                RelationAtom::SlotEquality {
                    left_slot: observation_slot,
                    right_slot: action_slot,
                },
                RelationAtom::ActionFunction {
                    value: "transport_a".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: action_slot,
                    value_type: Some(AtomValueType::Integer),
                },
            ],
            evidence_ref_sha256: root(&format!("evidence-{label}")),
        }
    }

    #[test]
    fn waiting_tick_rebuilds_only_for_join_relevant_archive_deltas() {
        let mut cursor = K1EvidenceCursorV1 {
            initialized: true,
            topology_rows: 10,
            frame_rows: 4,
            frame_intent_ids_sha256: BTreeSet::from(["completed-intent".to_owned()]),
            active_protocol_mode_set_root_sha256: "active-root".to_owned(),
            retain_safety_payloads: false,
            prepared: None,
        };

        assert!(!waiting_delta_requires_rebuild(
            &cursor,
            12,
            4,
            ["unsettled-a", "unsettled-b"]
        ));
        assert!(waiting_delta_requires_rebuild(
            &cursor,
            12,
            4,
            ["completed-intent"]
        ));
        assert!(waiting_delta_requires_rebuild(&cursor, 12, 5, []));
        assert!(waiting_delta_requires_rebuild(&cursor, 9, 4, []));

        cursor.initialized = false;
        assert!(waiting_delta_requires_rebuild(&cursor, 10, 4, []));
    }

    #[test]
    fn waiting_tick_cannot_skip_a_live_mechanism_lane() {
        assert!(!waiting_lanes_are_reusable(
            false,
            RuntimeState::WaitingForEvidence
        ));
        assert!(!waiting_lanes_are_reusable(
            true,
            RuntimeState::AwaitingIndependentFuture
        ));
        assert!(waiting_lanes_are_reusable(
            true,
            RuntimeState::WaitingForEvidence
        ));
    }

    #[test]
    fn incremental_prepared_context_matches_full_join_oracle() {
        let active_protocols = BTreeSet::new();
        let first_topology = topology("first", 1);
        let second_topology = topology("second", 2);
        let first_frame = completed_frame("first", 1);
        let second_frame = completed_frame("second", 2);

        let oracle_ledger = MultiSourceJoinLedgerV1::build(
            &[first_topology.clone(), second_topology.clone()],
            &[first_frame.clone(), second_frame.clone()],
        );
        let oracle = prepare_tick_context_from_join_ledger(oracle_ledger, &active_protocols)
            .expect("full oracle");

        let mut accumulator = EvidenceBindingAccumulator::new(false);
        let streamed_report = stream_multi_source_joins_from_iter(
            [first_topology.clone(), second_topology.clone()].iter(),
            [first_frame.clone(), second_frame.clone()].iter(),
            |joined| accumulator.push(joined),
        )
        .expect("streamed join");
        let streamed = prepare_tick_context_from_bindings(
            streamed_report,
            accumulator.finish().expect("streamed bindings"),
            &active_protocols,
            false,
        )
        .expect("streamed context");

        let initial_ledger = MultiSourceJoinLedgerV1::build(
            std::slice::from_ref(&first_topology),
            std::slice::from_ref(&first_frame),
        );
        let mut incremental =
            prepare_tick_context_from_join_ledger(initial_ledger, &active_protocols)
                .expect("initial context");
        extend_prepared_from_delta(
            &mut incremental,
            2,
            2,
            &[Arc::new(second_topology.clone())],
            &[Arc::new(second_topology)],
            &[Arc::new(second_frame)],
            &active_protocols,
        )
        .expect("incremental context");

        assert_eq!(incremental.join_report, oracle.join_report);
        assert_eq!(incremental.bindings, oracle.bindings);
        assert_eq!(incremental.catalog, oracle.catalog);
        assert_eq!(
            incremental.evidence_epoch_root_sha256,
            oracle.evidence_epoch_root_sha256
        );
        assert_eq!(
            incremental.active_protocol_mode_set_root_sha256,
            oracle.active_protocol_mode_set_root_sha256
        );
        assert_eq!(incremental.contract_watermark, oracle.contract_watermark);
        assert_eq!(streamed.join_report, oracle.join_report);
        assert_eq!(streamed.catalog, oracle.catalog);
        assert_eq!(
            streamed.evidence_epoch_root_sha256,
            oracle.evidence_epoch_root_sha256
        );
        assert_eq!(streamed.contract_watermark, oracle.contract_watermark);
    }
}

#[cfg(test)]
#[path = "service_live_diagnostics_tests.rs"]
mod live_diagnostics_tests;
