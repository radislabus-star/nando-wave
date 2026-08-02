use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::json;

use super::{
    K1NaturalSchedulerRuntimeReportV1, K1NaturalSchedulerRuntimeStateV1 as RuntimeState, advance,
};
use crate::k1_transfer_lifecycle::{K1TransferLifecycleReportV1, advance_transfer_lifecycle};
use crate::{AppState, json_response, multi_source_live, unix_now};

pub(crate) async fn report_handler(State(state): State<AppState>) -> Response {
    match state.k1_natural_scheduler_report.read() {
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

pub(crate) fn advance_state(state: &AppState) -> Result<(), String> {
    let topologies = state
        .multi_source_topology_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_topology_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_topology_archive_lock_poisoned".to_owned())?
        .rows();
    let frames = state
        .multi_source_frame_archive
        .as_ref()
        .ok_or_else(|| "k1_scheduler_frame_archive_not_configured".to_owned())?
        .lock()
        .map_err(|_| "k1_scheduler_frame_archive_lock_poisoned".to_owned())?
        .frames();
    let active_protocols =
        multi_source_live::active_protocol_mode_roots(&state.config.response_registry_path)?;
    for _ in 0..16 {
        let now = unix_now();
        let mut report = advance(
            &state.operator_certification_config,
            &topologies,
            &frames,
            &active_protocols,
            now,
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
            store_report(state, report)?;
            return Ok(());
        }
        let stable = matches!(
            report.state,
            RuntimeState::WaitingForEvidence
                | RuntimeState::ProbePending
                | RuntimeState::AwaitingIndependentFuture
                | RuntimeState::TerminalAbstain
                | RuntimeState::TerminalAcquisitionFail
                | RuntimeState::TerminalProbeExhausted
                | RuntimeState::K1VocabularyOpen
        );
        store_report(state, report)?;
        if stable {
            return Ok(());
        }
    }
    Err("k1_scheduler_tick_budget_exhausted".to_owned())
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

fn store_report(state: &AppState, report: K1NaturalSchedulerRuntimeReportV1) -> Result<(), String> {
    *state
        .k1_natural_scheduler_report
        .write()
        .map_err(|_| "k1_scheduler_report_lock_poisoned".to_owned())? = Some(report);
    Ok(())
}
