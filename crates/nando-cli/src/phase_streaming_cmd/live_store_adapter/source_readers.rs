use std::collections::BTreeSet;
use std::io::BufRead;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

use nando_core::{
    PhaseCenterAtomEncoder, PhaseCenterHotRouteTable, PhaseCenterHotWorker,
    PhaseCenterLiveOperatorStore,
};

use super::hot_path_eval::live_store_capture_direct_hot_snapshot;
use super::source_events::{
    LiveStoreAdaptiveBucketPolicy, LiveStoreParsedAtomEvent, live_store_atom_event_from_row,
};
use super::state::LiveStoreDirectHotSnapshotBank;
use super::worker_path::{
    LiveStorePreparedHotPackEval, LiveStorePreparedMemoryRow, LiveStoreWorkerBatchMessage,
    LiveStoreWorkerThreadMessage,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_score_source_adapter_reader<R: BufRead>(
    source_label: &str,
    reader: R,
    worker: &mut PhaseCenterHotWorker,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    eval: &mut LiveStorePreparedHotPackEval,
    latencies: &mut Vec<u128>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    route_index_missing_events: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            continue;
        };
        *parsed_rows += 1;
        let start = Instant::now();
        let decisions = worker
            .score_live_atom_event_with_evidence(adapter_event.to_live_operator_atom_event(), eval)
            .map_err(|error| {
                format!(
                    "failed to score source '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?;
        latencies.push(start.elapsed().as_nanos());
        if decisions.is_none() {
            *route_index_missing_events += 1;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_queue_source_reader<R: BufRead>(
    source_label: &str,
    reader: R,
    worker: &mut PhaseCenterHotWorker,
    encoder: &mut PhaseCenterAtomEncoder,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    queue: &mut Vec<LiveStorePreparedMemoryRow>,
    queue_batch_capacity: usize,
    eval: &mut LiveStorePreparedHotPackEval,
    latencies: &mut Vec<u128>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    route_index_missing_events: &mut usize,
    queue_flushes: &mut usize,
    max_observed_queue_depth: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read queue source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse queue source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            continue;
        };
        *parsed_rows += 1;
        let Some(route_index) = worker.resolve_route_index(adapter_event.route_id) else {
            *route_index_missing_events += 1;
            continue;
        };
        let phase_vector = encoder
            .encode_atom_ids(adapter_event.atom_ids.iter().copied())
            .map_err(|error| {
                format!(
                    "failed to encode queue source '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?
            .to_vec();
        queue.push(LiveStorePreparedMemoryRow::new(
            route_index,
            adapter_event.atom_ids.clone(),
            phase_vector,
            adapter_event.hot_request_evidence(),
        ));
        *max_observed_queue_depth = (*max_observed_queue_depth).max(queue.len());
        if queue.len() >= queue_batch_capacity {
            live_store_flush_worker_queue(worker, queue, eval, latencies, queue_flushes)?;
        }
    }
    Ok(())
}

pub(super) fn live_store_flush_worker_queue(
    worker: &mut PhaseCenterHotWorker,
    queue: &mut Vec<LiveStorePreparedMemoryRow>,
    eval: &mut LiveStorePreparedHotPackEval,
    latencies: &mut Vec<u128>,
    queue_flushes: &mut usize,
) -> Result<(), String> {
    if queue.is_empty() {
        return Ok(());
    }
    *queue_flushes += 1;
    for row in queue.iter() {
        let start = Instant::now();
        let _ = worker
            .score_prepared_row_with_evidence(row, eval)
            .map_err(|error| format!("failed queue worker score: {error:?}"))?;
        latencies.push(start.elapsed().as_nanos());
    }
    queue.clear();
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_thread_source_reader<R: BufRead>(
    source_label: &str,
    reader: R,
    route_table: &PhaseCenterHotRouteTable,
    encoder: &mut PhaseCenterAtomEncoder,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    sender: &SyncSender<LiveStoreWorkerThreadMessage>,
    source_prepare_latencies: &mut Vec<u128>,
    source_send_latencies: &mut Vec<u128>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    route_index_missing_events: &mut usize,
    sent_events: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read thread source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let prepare_start = Instant::now();
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse thread source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            continue;
        };
        *parsed_rows += 1;
        let Some(route_index) = route_table.resolve_route_index(adapter_event.route_id) else {
            *route_index_missing_events += 1;
            continue;
        };
        let phase_vector = encoder
            .encode_atom_ids(adapter_event.atom_ids.iter().copied())
            .map_err(|error| {
                format!(
                    "failed to encode thread source '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?
            .to_vec();
        let prepared_row = LiveStorePreparedMemoryRow::new(
            route_index,
            adapter_event.atom_ids.clone(),
            phase_vector,
            adapter_event.hot_request_evidence(),
        );
        source_prepare_latencies.push(prepare_start.elapsed().as_nanos());
        let send_start = Instant::now();
        sender
            .send(LiveStoreWorkerThreadMessage {
                row: prepared_row,
                enqueued_at: Instant::now(),
            })
            .map_err(|error| {
                format!(
                    "failed to send thread source '{}' line {}: {error}",
                    source_label,
                    line_index + 1
                )
            })?;
        source_send_latencies.push(send_start.elapsed().as_nanos());
        *sent_events += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_batch_thread_source_reader<R: BufRead>(
    source_label: &str,
    reader: R,
    route_table: &PhaseCenterHotRouteTable,
    encoder: &mut PhaseCenterAtomEncoder,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    sender: &SyncSender<LiveStoreWorkerBatchMessage>,
    source_batch_capacity: usize,
    batch: &mut Vec<LiveStorePreparedMemoryRow>,
    source_prepare_latencies: &mut Vec<u128>,
    source_send_latencies: &mut Vec<u128>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    route_index_missing_events: &mut usize,
    sent_events: &mut usize,
    sent_batches: &mut usize,
    max_sent_batch_len: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read batch-thread source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let prepare_start = Instant::now();
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse batch-thread source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            continue;
        };
        *parsed_rows += 1;
        let Some(route_index) = route_table.resolve_route_index(adapter_event.route_id) else {
            *route_index_missing_events += 1;
            continue;
        };
        let phase_vector = encoder
            .encode_atom_ids(adapter_event.atom_ids.iter().copied())
            .map_err(|error| {
                format!(
                    "failed to encode batch-thread source '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?
            .to_vec();
        batch.push(LiveStorePreparedMemoryRow::new(
            route_index,
            adapter_event.atom_ids.clone(),
            phase_vector,
            adapter_event.hot_request_evidence(),
        ));
        source_prepare_latencies.push(prepare_start.elapsed().as_nanos());
        if batch.len() >= source_batch_capacity {
            live_store_send_worker_batch(
                sender,
                source_batch_capacity,
                batch,
                source_send_latencies,
                sent_events,
                sent_batches,
                max_sent_batch_len,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_collect_direct_store_events<R: BufRead>(
    source_label: &str,
    reader: R,
    store: &mut PhaseCenterLiveOperatorStore,
    encoder: &mut PhaseCenterAtomEncoder,
    bucket_policy: &mut LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    parsed_events: &mut Vec<LiveStoreParsedAtomEvent>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    skipped_no_verifier_label: &mut usize,
    skipped_no_safe_atoms: &mut usize,
    direct_hot_snapshots: &mut LiveStoreDirectHotSnapshotBank,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read direct store source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse direct store source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            *skipped_no_verifier_label += 1;
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            *skipped_no_safe_atoms += 1;
            continue;
        };
        let decision = store
            .observe_atom_event(encoder, adapter_event.to_live_operator_atom_event())
            .map_err(|error| {
                format!(
                    "direct store observe failed for '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?;
        bucket_policy.observe_decision(&adapter_event, decision);
        parsed_events.push(adapter_event);
        *parsed_rows += 1;
        live_store_capture_direct_hot_snapshot(store, *parsed_rows, direct_hot_snapshots)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_collect_append_shadow_events<R: BufRead>(
    source_label: &str,
    reader: R,
    bucket_policy: &LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    parsed_events: &mut Vec<LiveStoreParsedAtomEvent>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    skipped_no_verifier_label: &mut usize,
    skipped_no_safe_atoms: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read append shadow source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse append shadow source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            *skipped_no_verifier_label += 1;
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            *skipped_no_safe_atoms += 1;
            continue;
        };
        parsed_events.push(adapter_event);
        *parsed_rows += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_observe_live_loop_budget_events<R: BufRead>(
    source_label: &str,
    reader: R,
    store: &mut PhaseCenterLiveOperatorStore,
    encoder: &mut PhaseCenterAtomEncoder,
    bucket_policy: &mut LiveStoreAdaptiveBucketPolicy,
    exact_cache_keys_seen: &mut BTreeSet<String>,
    parsed_events: &mut Vec<LiveStoreParsedAtomEvent>,
    total_rows: &mut usize,
    parsed_rows: &mut usize,
    skipped_no_verifier_label: &mut usize,
    skipped_no_safe_atoms: &mut usize,
) -> Result<(), String> {
    for (line_index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            format!(
                "failed to read live-loop budget source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        *total_rows += 1;
        let row = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse live-loop budget source '{}' line {}: {error}",
                source_label,
                line_index + 1
            )
        })?;
        let Some(verified_safe_accept) = row
            .get("verified_safe_accept")
            .and_then(serde_json::Value::as_bool)
        else {
            *skipped_no_verifier_label += 1;
            continue;
        };
        let Some(adapter_event) = live_store_atom_event_from_row(
            &row,
            verified_safe_accept,
            bucket_policy,
            exact_cache_keys_seen,
        ) else {
            *skipped_no_safe_atoms += 1;
            continue;
        };
        let decision = store
            .observe_atom_event(encoder, adapter_event.to_live_operator_atom_event())
            .map_err(|error| {
                format!(
                    "live-loop budget observe failed for '{}' line {}: {error:?}",
                    source_label,
                    line_index + 1
                )
            })?;
        for bucket_id in &adapter_event.auto_subcenter_bucket_ids {
            store
                .observe_atom_event(
                    encoder,
                    adapter_event.to_live_operator_atom_event_for_bucket(*bucket_id),
                )
                .map_err(|error| {
                    format!(
                        "live-loop budget auto-subcenter observe failed for '{}' line {}: {error:?}",
                        source_label,
                        line_index + 1
                    )
                })?;
        }
        bucket_policy.observe_decision(&adapter_event, decision);
        parsed_events.push(adapter_event);
        *parsed_rows += 1;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn live_store_send_parsed_events_to_batch_worker(
    parsed_events: &[LiveStoreParsedAtomEvent],
    route_table: &PhaseCenterHotRouteTable,
    encoder: &mut PhaseCenterAtomEncoder,
    sender: &SyncSender<LiveStoreWorkerBatchMessage>,
    source_batch_capacity: usize,
    batch: &mut Vec<LiveStorePreparedMemoryRow>,
    source_prepare_latencies: &mut Vec<u128>,
    source_send_latencies: &mut Vec<u128>,
    route_index_missing_events: &mut usize,
    sent_events: &mut usize,
    sent_batches: &mut usize,
    max_sent_batch_len: &mut usize,
) -> Result<(), String> {
    for event in parsed_events {
        let prepare_start = Instant::now();
        let Some(route_index) = route_table.resolve_route_index(event.route_id) else {
            *route_index_missing_events += 1;
            continue;
        };
        let phase_vector = encoder
            .encode_atom_ids(event.atom_ids.iter().copied())
            .map_err(|error| format!("failed to encode direct parsed event: {error:?}"))?
            .to_vec();
        batch.push(LiveStorePreparedMemoryRow::new(
            route_index,
            event.atom_ids.clone(),
            phase_vector,
            event.hot_request_evidence(),
        ));
        source_prepare_latencies.push(prepare_start.elapsed().as_nanos());
        if batch.len() >= source_batch_capacity {
            live_store_send_worker_batch(
                sender,
                source_batch_capacity,
                batch,
                source_send_latencies,
                sent_events,
                sent_batches,
                max_sent_batch_len,
            )?;
        }
    }
    Ok(())
}

pub(super) fn live_store_send_worker_batch(
    sender: &SyncSender<LiveStoreWorkerBatchMessage>,
    source_batch_capacity: usize,
    batch: &mut Vec<LiveStorePreparedMemoryRow>,
    source_send_latencies: &mut Vec<u128>,
    sent_events: &mut usize,
    sent_batches: &mut usize,
    max_sent_batch_len: &mut usize,
) -> Result<(), String> {
    if batch.is_empty() {
        return Ok(());
    }
    let batch_len = batch.len();
    let rows = std::mem::replace(batch, Vec::with_capacity(source_batch_capacity));
    let send_start = Instant::now();
    sender
        .send(LiveStoreWorkerBatchMessage {
            rows,
            enqueued_at: Instant::now(),
        })
        .map_err(|error| format!("failed to send batch-thread worker batch: {error}"))?;
    source_send_latencies.push(send_start.elapsed().as_nanos());
    *sent_events += batch_len;
    *sent_batches += 1;
    *max_sent_batch_len = (*max_sent_batch_len).max(batch_len);
    Ok(())
}
