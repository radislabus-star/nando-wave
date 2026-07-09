use std::collections::BTreeSet;

use nando_core::{
    PhaseCenterHotCandidateDecision, PhaseCenterHotRequest, PhaseCenterHotRequestEvidence,
    PhaseCenterHotRouteTable, PhaseCenterHotRowPreparer, PhaseCenterHotRuntime,
    PhaseCenterHotScratch, PhaseCenterLiveOperatorStore, PhaseCenterPreparedHotRequest,
};

use super::reports::PhaseStreamLiveStoreDirectHotReport;
use super::source_events::LiveStoreParsedAtomEvent;
use super::state::{
    LiveStoreDirectHotSnapshot, LiveStoreDirectHotSnapshotBank, LiveStoreDirectHotSnapshotEval,
    LiveStoreHotPathDenominator, LiveStoreProductHotCreditRow,
};
use super::worker_path::{LiveStorePreparedHotPackEval, LiveStorePreparedMemoryRow};

pub(super) fn live_store_capture_direct_hot_snapshot(
    store: &PhaseCenterLiveOperatorStore,
    parsed_rows: usize,
    direct_hot_snapshots: &mut LiveStoreDirectHotSnapshotBank,
) -> Result<(), String> {
    let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table()
        .map_err(|error| format!("failed to capture direct hot snapshot: {error:?}"))?
    else {
        return Ok(());
    };
    direct_hot_snapshots.push(LiveStoreDirectHotSnapshot {
        frozen_after_parsed_rows: parsed_rows,
        hot_runtime,
        route_table,
    });
    Ok(())
}

pub(super) fn live_store_direct_hot_report(
    store: &PhaseCenterLiveOperatorStore,
    events: &[LiveStoreParsedAtomEvent],
    cells: usize,
) -> Result<PhaseStreamLiveStoreDirectHotReport, String> {
    let Some((hot_runtime, route_table)) = store
        .candidate_hot_runtime_and_route_table()
        .map_err(|error| format!("failed to build direct live hot runtime/table: {error:?}"))?
    else {
        return Ok(PhaseStreamLiveStoreDirectHotReport {
            bridge_kind: "mutable_live_store_to_hot_runtime_route_table_v1",
            blocker: "direct_live_hot_no_candidate_runtime",
            ..PhaseStreamLiveStoreDirectHotReport::default()
        });
    };
    let snapshot = store.runtime_budget_snapshot();
    let mut report = PhaseStreamLiveStoreDirectHotReport {
        bridge_kind: "mutable_live_store_to_hot_runtime_route_table_v1",
        available: true,
        hot_route_count: route_table.route_count(),
        hot_profile_count: hot_runtime.profile_count(),
        hot_route_profile_edges: route_table.profile_edge_count(),
        hot_bytes_estimate: hot_runtime
            .bytes_estimate()
            .saturating_add(route_table.bytes_estimate()),
        product_runtime_budget_passed: snapshot.product_runtime_budget_passed(),
        local_accept_enabled: false,
        blocker: "none",
        ..PhaseStreamLiveStoreDirectHotReport::default()
    };
    let mut scratch = PhaseCenterHotScratch::new(cells, store.memory_config().max_route_top_k)
        .map_err(|error| format!("failed to build direct live hot scratch: {error:?}"))?;
    for event in events {
        let Some(route_index) = route_table.resolve_route_index(event.route_id) else {
            report.route_index_missing_events += 1;
            continue;
        };
        let decisions = hot_runtime
            .score_hot_request_candidates(
                &route_table,
                PhaseCenterHotRequest::new(route_index, &event.atom_ids),
                &mut scratch,
            )
            .map_err(|error| format!("failed direct live hot score: {error:?}"))?;
        report.score_events += 1;
        for decision in decisions {
            if !decision.score_candidate {
                continue;
            }
            report.score_candidate_events += 1;
            report.verifier_required_events += usize::from(decision.verifier_required);
            report.local_accept_events += usize::from(decision.local_accept);
            if event.verified_safe_accept {
                if !event.exact_cache_hit {
                    report.unique_cpu_accepts_over_exact_cache += 1;
                    report.tokens_saved = report.tokens_saved.saturating_add(event.tokens);
                    report.cost_saved_microusd = report
                        .cost_saved_microusd
                        .saturating_add(event.cost_microusd);
                }
            } else {
                report.score_false_label_events += 1;
            }
        }
    }
    report.blocker = live_store_direct_hot_blocker(&report);
    report.passed = report.blocker == "none";
    Ok(report)
}

fn live_store_direct_hot_blocker(report: &PhaseStreamLiveStoreDirectHotReport) -> &'static str {
    if !report.available {
        "direct_live_hot_no_candidate_runtime"
    } else if report.hot_route_count == 0 || report.hot_profile_count == 0 {
        "direct_live_hot_empty_manifest"
    } else if report.score_events == 0 {
        "direct_live_hot_no_score_events"
    } else if report.score_candidate_events == 0 {
        "direct_live_hot_no_score_candidates"
    } else if report.verifier_required_events != report.score_candidate_events {
        "direct_live_hot_missing_verifier_required"
    } else if report.local_accept_events > 0 {
        "direct_live_hot_local_accept_enabled"
    } else if !report.product_runtime_budget_passed {
        "direct_live_hot_budget_failed"
    } else {
        "none"
    }
}

pub(super) fn live_store_select_direct_hot_snapshot(
    direct_hot_snapshots: &LiveStoreDirectHotSnapshotBank,
    parsed_events: &[LiveStoreParsedAtomEvent],
    cells: usize,
) -> Result<Option<LiveStoreDirectHotSnapshotEval>, String> {
    let mut best = None::<LiveStoreDirectHotSnapshotEval>;
    for (snapshot_index, snapshot) in direct_hot_snapshots.iter().enumerate() {
        let mut candidate =
            live_store_eval_direct_hot_snapshot(snapshot_index, snapshot, parsed_events, cells)?;
        candidate.snapshot_index = snapshot_index;
        let safe = candidate.validation_eval.false_accepts == 0
            && candidate.validation_eval.score_candidate_events > 0
            && candidate.future_eval_start_after_parsed_rows < parsed_events.len();
        if !safe {
            continue;
        }
        let replace = best.as_ref().is_none_or(|current| {
            candidate
                .validation_eval
                .unique_cpu_accepts_over_exact_cache
                .cmp(&current.validation_eval.unique_cpu_accepts_over_exact_cache)
                .then_with(|| {
                    candidate
                        .validation_eval
                        .tokens_saved
                        .cmp(&current.validation_eval.tokens_saved)
                })
                .then_with(|| {
                    candidate
                        .validation_eval
                        .cost_saved_microusd
                        .cmp(&current.validation_eval.cost_saved_microusd)
                })
                .then_with(|| {
                    current
                        .frozen_after_parsed_rows
                        .cmp(&candidate.frozen_after_parsed_rows)
                })
                .is_gt()
        });
        if replace {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn live_store_eval_direct_hot_snapshot(
    snapshot_index: usize,
    snapshot: &LiveStoreDirectHotSnapshot,
    parsed_events: &[LiveStoreParsedAtomEvent],
    cells: usize,
) -> Result<LiveStoreDirectHotSnapshotEval, String> {
    let future_start = snapshot.frozen_after_parsed_rows.min(parsed_events.len());
    let (validation_end, validation_score_events) =
        live_store_validation_end_for_snapshot(snapshot, parsed_events, future_start, 1);
    let mut eval = LiveStoreDirectHotSnapshotEval {
        snapshot_index,
        frozen_after_parsed_rows: snapshot.frozen_after_parsed_rows,
        future_eval_start_after_parsed_rows: validation_end,
        validation_score_events,
        ..LiveStoreDirectHotSnapshotEval::default()
    };
    let mut validation_eval = LiveStorePreparedHotPackEval::default();
    let mut scratch =
        PhaseCenterHotScratch::new(cells, snapshot.route_table.profile_edge_count().max(1))
            .map_err(|error| format!("failed to build direct snapshot scratch: {error:?}"))?;
    for event in &parsed_events[future_start..validation_end] {
        let Some(route_index) = snapshot.route_table.resolve_route_index(event.route_id) else {
            eval.validation_route_index_missing_events += 1;
            continue;
        };
        let decisions = snapshot
            .hot_runtime
            .score_hot_request_candidates(
                &snapshot.route_table,
                PhaseCenterHotRequest::new(route_index, &event.atom_ids),
                &mut scratch,
            )
            .map_err(|error| format!("failed to eval direct hot snapshot: {error:?}"))?;
        live_store_update_candidate_decision_eval(
            event.verified_safe_accept,
            event.exact_cache_hit,
            event.tokens,
            event.cost_microusd,
            decisions,
            &mut validation_eval,
        );
    }
    eval.validation_eval = validation_eval;
    Ok(eval)
}

fn live_store_validation_end_for_snapshot(
    snapshot: &LiveStoreDirectHotSnapshot,
    parsed_events: &[LiveStoreParsedAtomEvent],
    future_start: usize,
    validation_score_event_target: usize,
) -> (usize, usize) {
    let mut validation_score_events = 0usize;
    for (index, event) in parsed_events.iter().enumerate().skip(future_start) {
        if snapshot
            .route_table
            .resolve_route_index(event.route_id)
            .is_none()
        {
            continue;
        }
        validation_score_events += 1;
        if validation_score_events >= validation_score_event_target {
            return (index + 1, validation_score_events);
        }
    }
    (parsed_events.len(), validation_score_events)
}

pub(super) fn live_store_prepare_parsed_events_for_hot_path(
    parsed_events: &[LiveStoreParsedAtomEvent],
    route_table: &PhaseCenterHotRouteTable,
    preparer: &mut PhaseCenterHotRowPreparer,
    route_index_missing_events: &mut usize,
) -> Result<Vec<LiveStorePreparedMemoryRow>, String> {
    let mut prepared = Vec::with_capacity(parsed_events.len());
    for event in parsed_events {
        match preparer
            .prepare_live_atom_event(route_table, event.to_live_operator_atom_event())
            .map_err(|error| format!("failed to prepare hot-path event: {error:?}"))?
        {
            Some(row) => prepared.push(row),
            None => *route_index_missing_events += 1,
        }
    }
    Ok(prepared)
}

pub(super) fn live_store_hot_path_runtime_parity(
    hot_runtime: &PhaseCenterHotRuntime,
    route_table: &PhaseCenterHotRouteTable,
    prepared_rows: &[LiveStorePreparedMemoryRow],
    cells: usize,
) -> Result<(usize, usize, usize), String> {
    let mut atom_scratch =
        PhaseCenterHotScratch::new(cells, route_table.profile_edge_count().max(1))
            .map_err(|error| format!("failed to build hot-path atom parity scratch: {error:?}"))?;
    let mut prepared_scratch =
        PhaseCenterHotScratch::new(cells, route_table.profile_edge_count().max(1)).map_err(
            |error| format!("failed to build hot-path prepared parity scratch: {error:?}"),
        )?;
    let mut checks = 0usize;
    let mut margin_mismatches = 0usize;
    let mut decision_mismatches = 0usize;
    for row in prepared_rows {
        let atom_decisions = hot_runtime
            .score_hot_request_candidates(
                route_table,
                PhaseCenterHotRequest::new(row.route_index, &row.atom_ids),
                &mut atom_scratch,
            )
            .map_err(|error| format!("failed hot-path atom parity score: {error:?}"))?
            .to_vec();
        let prepared_decisions = hot_runtime
            .score_prepared_hot_request_candidates(
                route_table,
                PhaseCenterPreparedHotRequest::new(row.route_index, &row.phase_vector),
                &mut prepared_scratch,
            )
            .map_err(|error| format!("failed hot-path prepared parity score: {error:?}"))?
            .to_vec();
        if atom_decisions.len() != prepared_decisions.len() {
            decision_mismatches += 1;
            continue;
        }
        for (atom, prepared) in atom_decisions.iter().zip(prepared_decisions.iter()) {
            checks += 1;
            if atom.margin_micro != prepared.margin_micro {
                margin_mismatches += 1;
            }
            if atom.profile_id != prepared.profile_id
                || atom.score_candidate != prepared.score_candidate
                || atom.verifier_required != prepared.verifier_required
                || atom.local_accept != prepared.local_accept
            {
                decision_mismatches += 1;
            }
        }
    }
    Ok((checks, margin_mismatches, decision_mismatches))
}

pub(super) fn live_store_hot_path_denominator(
    prepared_rows: &[LiveStorePreparedMemoryRow],
) -> LiveStoreHotPathDenominator {
    let mut denominator = LiveStoreHotPathDenominator::default();
    for row in prepared_rows {
        denominator.observe_evidence(row.evidence());
    }
    denominator
}

pub(super) fn live_store_update_prepared_hot_pack_eval(
    row: &super::reports::PhaseStreamLiveStorePreparedHotPackRow,
    decisions: &[PhaseCenterHotCandidateDecision],
    eval: &mut LiveStorePreparedHotPackEval,
) {
    live_store_update_candidate_decision_eval(
        row.verified_safe_accept,
        row.exact_cache_hit,
        row.tokens,
        row.cost_microusd,
        decisions,
        eval,
    );
}

pub(super) fn live_store_update_memory_hot_worker_eval(
    row: &LiveStorePreparedMemoryRow,
    decisions: &[PhaseCenterHotCandidateDecision],
    eval: &mut LiveStorePreparedHotPackEval,
) {
    live_store_update_candidate_decision_eval(
        row.verified_safe_accept,
        row.exact_cache_hit,
        row.tokens,
        row.cost_microusd,
        decisions,
        eval,
    );
}

pub(super) fn live_store_product_hot_clean_credit_totals(
    rows: &[LiveStoreProductHotCreditRow],
    quarantined_profile_ids: &BTreeSet<u32>,
) -> (usize, u64, u64) {
    rows.iter()
        .filter(|row| {
            row.profile_ids
                .iter()
                .any(|profile_id| !quarantined_profile_ids.contains(profile_id))
        })
        .fold((0usize, 0u64, 0u64), |(calls, tokens, cost), row| {
            (
                calls.saturating_add(1),
                tokens.saturating_add(row.tokens),
                cost.saturating_add(row.cost_microusd),
            )
        })
}

pub(super) fn live_store_update_candidate_decision_eval(
    verified_safe_accept: bool,
    exact_cache_hit: bool,
    tokens: u64,
    cost_microusd: u64,
    decisions: &[PhaseCenterHotCandidateDecision],
    eval: &mut LiveStorePreparedHotPackEval,
) {
    eval.observe_candidate_decisions(
        PhaseCenterHotRequestEvidence {
            verified_safe_accept,
            exact_cache_hit,
            tokens,
            cost_microusd,
        },
        decisions,
    );
}
