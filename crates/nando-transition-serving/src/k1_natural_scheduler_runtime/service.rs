use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::Arc;

use super::{
    AdvanceInput, K1NaturalSchedulerRuntimeReportV1,
    K1NaturalSchedulerRuntimeStateV1 as RuntimeState, K1SchedulerLaneV1, MultiSourceJoinLedgerV1,
    PreActionTopologyAuditRowV1, PreparedK1TickContextV1, RelationFrame, advance,
    current_deficit_snapshot,
    law_lab_eligibility::law_lab_eligibility_report,
    prepare_tick_context_from_join_ledger, restore_projection_for,
    structural_frontier_census::{
        build_report as build_frontier_report, publish_report as publish_frontier_report,
        source_root as frontier_source_root,
    },
};
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
}

impl K1EvidenceCursorV1 {
    fn record(
        &mut self,
        topology_rows: usize,
        frame_rows: usize,
        frame_intent_ids_sha256: BTreeSet<String>,
        active_protocol_mode_set_root_sha256: String,
    ) {
        self.initialized = true;
        self.topology_rows = topology_rows;
        self.frame_rows = frame_rows;
        self.frame_intent_ids_sha256 = frame_intent_ids_sha256;
        self.active_protocol_mode_set_root_sha256 = active_protocol_mode_set_root_sha256;
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
    if reuse_waiting_tick(
        state,
        evidence_cursor,
        mechanism_terminal,
        &active_protocol_mode_set_root_sha256,
        &epistemic_projection.projection_root_sha256,
        &deficit.snapshot_root_sha256,
    )? {
        return Ok(());
    }

    let topologies_shared = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?
        .shared_rows();
    let frames_shared = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .shared_frames();
    let frame_intent_ids_sha256 = frames_shared
        .iter()
        .map(|frame| frame.client_intent_id_sha256.clone())
        .collect::<BTreeSet<_>>();
    let join_ledger = MultiSourceJoinLedgerV1::build_from_iter(
        topologies_shared.iter().map(|row| row.as_ref()),
        frames_shared.iter().map(|frame| frame.as_ref()),
    );
    let prepared = prepare_tick_context_from_join_ledger(join_ledger, &active_protocols)?;
    let candidate_artifacts = crate::current_collection_miner(state)
        .map(|miner| {
            miner
                .lock()
                .map_err(|_| "k1_scheduler_collection_miner_lock_poisoned".to_owned())?
                .read_snapshot()
                .natural_t1_program_artifacts()
        })
        .transpose()?
        .unwrap_or_default();
    let mechanism_projection = (!mechanism_terminal)
        .then(|| {
            restore_projection_for(
                &state.operator_certification_config,
                K1SchedulerLaneV1::Mechanism,
            )
        })
        .transpose()?;
    let evidence_snapshot_required = epistemic_projection.active_candidate_freeze.is_some()
        || !epistemic_projection.future_predictions.is_empty()
        || mechanism_projection
            .as_ref()
            .is_some_and(|projection| projection.active_candidate_freeze.is_some());
    let (mut topologies, mut frames) = if evidence_snapshot_required {
        materialize_evidence(&topologies_shared, &frames_shared)
    } else {
        (Vec::new(), Vec::new())
    };
    if !mechanism_terminal {
        let mechanism = advance(
            &state.operator_certification_config,
            K1SchedulerLaneV1::Mechanism,
            false,
            AdvanceInput {
                prepared: &prepared,
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
                prepared: &prepared,
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
            store_report(state, &prepared, report)?;
            evidence_cursor.record(
                topologies_shared.len(),
                frames_shared.len(),
                frame_intent_ids_sha256.clone(),
                active_protocol_mode_set_root_sha256.clone(),
            );
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
        store_report(state, &prepared, report)?;
        if stable {
            evidence_cursor.record(
                topologies_shared.len(),
                frames_shared.len(),
                frame_intent_ids_sha256.clone(),
                active_protocol_mode_set_root_sha256.clone(),
            );
            return Ok(());
        }
        if topologies.is_empty() && frames.is_empty() {
            (topologies, frames) = materialize_evidence(&topologies_shared, &frames_shared);
        }
    }
    Err("k1_scheduler_tick_budget_exhausted".to_owned())
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
) -> Result<bool, String> {
    let report = state
        .k1_natural_scheduler_report
        .read()
        .map_err(|_| "k1_scheduler_report_lock_poisoned".to_owned())?;
    let Some(report) = report.as_ref() else {
        return Ok(false);
    };
    if !waiting_lanes_are_reusable(mechanism_terminal, report.state)
        || report.projection.projection_root_sha256 != projection_root_sha256
        || report.queue.k1_deficit_snapshot_root_sha256 != deficit_snapshot_root_sha256
        || evidence_cursor.active_protocol_mode_set_root_sha256
            != active_protocol_mode_set_root_sha256
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

    #[test]
    fn waiting_tick_rebuilds_only_for_join_relevant_archive_deltas() {
        let mut cursor = K1EvidenceCursorV1 {
            initialized: true,
            topology_rows: 10,
            frame_rows: 4,
            frame_intent_ids_sha256: BTreeSet::from(["completed-intent".to_owned()]),
            active_protocol_mode_set_root_sha256: "active-root".to_owned(),
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
}
