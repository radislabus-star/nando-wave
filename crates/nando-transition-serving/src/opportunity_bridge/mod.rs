mod checkpoint;
mod deadline;
mod spool;

use std::collections::{BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::OpportunityBridgeEventV1;
use serde::Serialize;

use crate::miner_worker::MinerWorkerHandle;
use checkpoint::{OpportunityBridgeCounterCheckpointV1, load_counter_checkpoint};
pub(crate) use deadline::OpportunityWindowClosureV1;
pub(crate) use deadline::{OpportunityWindowBoundaryV1, S1C4_WINDOW_BOUNDARY_FILE_V1};
use spool::{
    PendingBridgeEvent, acknowledge_pending_batch, create_private_directory,
    discard_acknowledged_prefix, first_pending_sequence, next_pending_sequence, pending_batch,
    pending_stats, persist_event, recover_temporary_events, refresh_pending_counters, spool_stats,
    sync_pending_spool,
};

const BRIDGE_STATUS_SCHEMA_V2: &str = "nando.opportunity-process-bridge-status.v2";
const PRODUCER_DURABILITY_INTERVAL: Duration = Duration::from_millis(10);
const SPOOL_RECONCILE_INTERVAL: Duration = Duration::from_secs(15);
const MAX_CONSUMER_INFLIGHT_EVENTS: usize = 256;
// Preserve durable apply and spool acknowledgement order until the bridge
// protocol carries an explicit persisted batch sequence.
const MAX_CONSUMER_INFLIGHT_BATCHES: usize = 1;

#[derive(Default)]
struct BridgeEndpointCounters {
    events: AtomicU64,
    request_events: AtomicU64,
    request_input_tokens: AtomicU64,
    duplicates: AtomicU64,
    failures: AtomicU64,
    invalid_events: AtomicU64,
    last_sequence: AtomicU64,
    last_micros: AtomicU64,
    max_micros: AtomicU64,
    total_micros: AtomicU64,
    durable_sequence: AtomicU64,
    durability_syncs: AtomicU64,
    counter_started_after_sequence: AtomicU64,
    last_error: RwLock<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct OpportunityBridgeEndpointStatusV1 {
    pub enabled: bool,
    pub ready: bool,
    pub events: u64,
    pub request_events: u64,
    pub request_input_tokens: u64,
    pub duplicates: u64,
    pub failures: u64,
    pub invalid_events: u64,
    pub last_sequence: u64,
    pub last_micros: u64,
    pub max_micros: u64,
    pub total_micros: u64,
    pub durable_sequence: u64,
    pub durability_syncs: u64,
    pub counter_started_after_sequence: u64,
    pub last_error: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct OpportunityBridgeStatusV1 {
    pub schema: String,
    pub root: String,
    pub counter_checkpoint_restored: bool,
    pub counter_checkpoint_last_sequence: u64,
    pub pending_events: u64,
    pub pending_bytes: u64,
    pub consumer_inflight_events: u64,
    pub durability_interval_ms: u64,
    pub producer: OpportunityBridgeEndpointStatusV1,
    pub consumer: OpportunityBridgeEndpointStatusV1,
}

#[derive(Clone)]
pub struct OpportunityBridgeRuntime {
    inner: Arc<BridgeInner>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpportunityBridgeSubmissionReceiptV1 {
    pub sequence: u64,
    pub event_root_sha256: String,
    pub request_ordinal: Option<u64>,
    pub s1c4_deadline_eligible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpportunityBridgeDurableCursorV1 {
    pub counter_started_after_sequence: u64,
    pub last_sequence: u64,
    pub durable_sequence: u64,
    pub request_events: u64,
    pub request_input_tokens: u64,
}

pub(crate) struct BridgeInner {
    root: PathBuf,
    counter_checkpoint_path: PathBuf,
    counter_checkpoint: Mutex<OpportunityBridgeCounterCheckpointV1>,
    counter_checkpoint_restored: bool,
    staging_dir: PathBuf,
    pending_dir: PathBuf,
    rejected_dir: PathBuf,
    producer_enabled: bool,
    consumer_enabled: bool,
    consumer_poll: Duration,
    next_sequence: AtomicU64,
    persist_lock: Mutex<()>,
    spool_files: AtomicU64,
    spool_bytes: AtomicU64,
    pending_events: AtomicU64,
    pending_bytes: AtomicU64,
    producer: BridgeEndpointCounters,
    consumer: BridgeEndpointCounters,
    consumer_started: AtomicBool,
    consumer_inflight: AtomicU64,
    producer_sync_requested: AtomicBool,
    deadline_capture: Mutex<Option<deadline::OpportunityWindowCaptureV1>>,
}

struct InflightOpportunityBatch {
    pending: Vec<PendingBridgeEvent>,
    ack: Receiver<Result<(), String>>,
    started: Instant,
}

impl OpportunityBridgeRuntime {
    pub fn new(
        root: PathBuf,
        producer_enabled: bool,
        consumer_enabled: bool,
        consumer_poll: Duration,
    ) -> Result<Self, String> {
        if producer_enabled && consumer_enabled {
            return Err("opportunity_bridge_roles_overlap".to_owned());
        }
        let staging_dir = root.join("staging");
        let pending_dir = root.join("pending");
        let rejected_dir = root.join("rejected");
        if producer_enabled || consumer_enabled {
            create_private_directory(&root)?;
            create_private_directory(&pending_dir)?;
            create_private_directory(&rejected_dir)?;
        }
        if producer_enabled {
            create_private_directory(&staging_dir)?;
            recover_temporary_events(&staging_dir, &pending_dir, &rejected_dir)?;
        }
        let counter_checkpoint_path = root.join("counter-checkpoint-v1.json");
        let restored_counter_checkpoint = load_counter_checkpoint(&counter_checkpoint_path)?;
        let counter_checkpoint_restored = restored_counter_checkpoint.is_some();
        let first_pending_sequence = first_pending_sequence(&pending_dir)?;
        let scanned_next_sequence =
            next_pending_sequence(&[&pending_dir, &staging_dir, &rejected_dir])?;
        let counter_checkpoint = restored_counter_checkpoint.unwrap_or_else(|| {
            OpportunityBridgeCounterCheckpointV1::empty(first_pending_sequence.map_or_else(
                || scanned_next_sequence.saturating_sub(1),
                |sequence| sequence.saturating_sub(1),
            ))
        });
        let next_sequence = scanned_next_sequence
            .max(counter_checkpoint.last_sequence.saturating_add(1))
            .max(1);
        let (spool_files, spool_bytes) = spool_stats(&[&staging_dir, &pending_dir, &rejected_dir])?;
        let (pending_events, pending_bytes) = pending_stats(&pending_dir);
        let producer = BridgeEndpointCounters::default();
        let consumer = BridgeEndpointCounters::default();
        let runtime = Self {
            inner: Arc::new(BridgeInner {
                root,
                counter_checkpoint_path,
                counter_checkpoint: Mutex::new(counter_checkpoint.clone()),
                counter_checkpoint_restored,
                staging_dir,
                pending_dir,
                rejected_dir,
                producer_enabled,
                consumer_enabled,
                consumer_poll: consumer_poll.max(Duration::from_millis(10)),
                next_sequence: AtomicU64::new(next_sequence),
                persist_lock: Mutex::new(()),
                spool_files: AtomicU64::new(spool_files),
                spool_bytes: AtomicU64::new(spool_bytes),
                pending_events: AtomicU64::new(pending_events),
                pending_bytes: AtomicU64::new(pending_bytes),
                producer,
                consumer,
                consumer_started: AtomicBool::new(false),
                consumer_inflight: AtomicU64::new(0),
                producer_sync_requested: AtomicBool::new(
                    next_sequence.saturating_sub(1) > counter_checkpoint.last_sequence,
                ),
                deadline_capture: Mutex::new(None),
            }),
        };
        let mut recovered_counter_checkpoint = counter_checkpoint;
        let producer_checkpoint = if producer_enabled {
            let mut checkpoint_seen = counter_checkpoint_restored;
            let mut recovered_producer = None;
            for _ in 0..8 {
                discard_acknowledged_prefix(
                    &runtime.inner,
                    recovered_counter_checkpoint.last_sequence,
                )?;
                let pending = pending_batch(&runtime.inner, usize::MAX, &BTreeSet::new())?;
                let latest = match load_counter_checkpoint(&runtime.inner.counter_checkpoint_path)?
                {
                    Some(latest) => {
                        checkpoint_seen = true;
                        latest
                    }
                    None if !checkpoint_seen => recovered_counter_checkpoint.clone(),
                    None => {
                        return Err("opportunity_bridge_counter_checkpoint_disappeared".to_owned());
                    }
                };
                if latest == recovered_counter_checkpoint {
                    recovered_producer = Some(recovered_counter_checkpoint.with_pending(&pending)?);
                    break;
                }
                recovered_counter_checkpoint = latest;
            }
            recovered_producer
                .ok_or_else(|| "opportunity_bridge_counter_recovery_unstable".to_owned())?
        } else {
            discard_acknowledged_prefix(
                &runtime.inner,
                recovered_counter_checkpoint.last_sequence,
            )?;
            recovered_counter_checkpoint.clone()
        };
        *runtime
            .inner
            .counter_checkpoint
            .lock()
            .map_err(|_| "opportunity_bridge_counter_checkpoint_lock_poisoned".to_owned())? =
            recovered_counter_checkpoint.clone();
        runtime.inner.next_sequence.fetch_max(
            producer_checkpoint.last_sequence.saturating_add(1),
            Ordering::AcqRel,
        );
        restore_endpoint_counters(
            &runtime.inner.producer,
            &producer_checkpoint,
            recovered_counter_checkpoint.last_sequence,
        );
        restore_endpoint_counters(
            &runtime.inner.consumer,
            &recovered_counter_checkpoint,
            recovered_counter_checkpoint.last_sequence,
        );
        if producer_enabled {
            runtime.start_producer_durability_worker()?;
        }
        Ok(runtime)
    }

    #[must_use]
    pub fn producer_enabled(&self) -> bool {
        self.inner.producer_enabled
    }

    pub fn submit(
        &self,
        event: OpportunityBridgeEventV1,
    ) -> Result<OpportunityBridgeSubmissionReceiptV1, String> {
        if !self.inner.producer_enabled {
            return Err("opportunity_bridge_producer_disabled".to_owned());
        }
        let started = Instant::now();
        let result = persist_event(&self.inner, &event);
        record_timing(&self.inner.producer, started);
        if let Err(error) = &result {
            record_failure(&self.inner.producer, error);
        }
        result
    }

    pub(crate) fn with_durable_cursor<T>(
        &self,
        inspect: impl FnOnce(OpportunityBridgeDurableCursorV1, &BridgeInner) -> Result<T, String>,
    ) -> Result<T, String> {
        let _guard = self
            .inner
            .persist_lock
            .lock()
            .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
        let last_sequence = self.inner.producer.last_sequence.load(Ordering::Acquire);
        let durable_sequence = self.inner.producer.durable_sequence.load(Ordering::Acquire);
        if durable_sequence < last_sequence {
            return Err("opportunity_bridge_durable_cursor_pending".to_owned());
        }
        inspect(
            OpportunityBridgeDurableCursorV1 {
                counter_started_after_sequence: self
                    .inner
                    .producer
                    .counter_started_after_sequence
                    .load(Ordering::Acquire),
                last_sequence,
                durable_sequence,
                request_events: self.inner.producer.request_events.load(Ordering::Acquire),
                request_input_tokens: self
                    .inner
                    .producer
                    .request_input_tokens
                    .load(Ordering::Acquire),
            },
            &self.inner,
        )
    }

    pub(crate) fn configure_request_deadline_capture_locked(
        inner: &BridgeInner,
        cursor_root_sha256: String,
        deadline_at_unix: u64,
        maximum_request_ordinal: u64,
        boundary_path: PathBuf,
    ) -> Result<(), String> {
        deadline::configure_window_capture(
            inner,
            cursor_root_sha256,
            deadline_at_unix,
            maximum_request_ordinal,
            boundary_path,
        )
    }

    pub(crate) fn disable_request_deadline_capture_locked(inner: &BridgeInner) {
        deadline::disable_window_capture(inner);
    }

    pub(crate) fn configure_request_deadline_capture(
        &self,
        cursor_root_sha256: String,
        deadline_at_unix: u64,
        maximum_request_ordinal: u64,
        boundary_path: PathBuf,
    ) -> Result<(), String> {
        let _guard = self
            .inner
            .persist_lock
            .lock()
            .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
        Self::configure_request_deadline_capture_locked(
            &self.inner,
            cursor_root_sha256,
            deadline_at_unix,
            maximum_request_ordinal,
            boundary_path,
        )
    }

    pub(crate) fn disable_request_deadline_capture(&self) {
        deadline::disable_window_capture(&self.inner);
    }

    pub(crate) fn freeze_request_deadline_boundary(
        &self,
        observed_at_unix: u64,
    ) -> Result<Option<OpportunityWindowBoundaryV1>, String> {
        let _guard = self
            .inner
            .persist_lock
            .lock()
            .map_err(|_| "opportunity_bridge_persist_lock_poisoned".to_owned())?;
        deadline::freeze_time_limit_if_due(&self.inner, observed_at_unix)
    }

    pub fn start_consumer(&self, worker: MinerWorkerHandle) -> Result<(), String> {
        if !self.inner.consumer_enabled {
            return Ok(());
        }
        if self
            .inner
            .consumer_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err("opportunity_bridge_consumer_already_started".to_owned());
        }
        let inner = Arc::downgrade(&self.inner);
        thread::Builder::new()
            .name("nando-opportunity-bridge-v1".to_owned())
            .spawn(move || {
                let mut inflight = VecDeque::new();
                let mut last_spool_reconcile = Instant::now();
                while let Some(inner) = inner.upgrade() {
                    if let Err(error) = advance_consumer_pipeline(&inner, &worker, &mut inflight) {
                        record_failure(&inner.consumer, &error);
                        inner.consumer_started.store(false, Ordering::Release);
                        return;
                    }
                    if last_spool_reconcile.elapsed() >= SPOOL_RECONCILE_INTERVAL {
                        refresh_pending_counters(&inner);
                        last_spool_reconcile = Instant::now();
                    }
                    inner.consumer_inflight.store(
                        inflight
                            .iter()
                            .map(|batch| u64::try_from(batch.pending.len()).unwrap_or(u64::MAX))
                            .fold(0_u64, u64::saturating_add),
                        Ordering::Release,
                    );
                    let poll = inner.consumer_poll;
                    drop(inner);
                    thread::sleep(poll);
                }
            })
            .map(|_| ())
            .map_err(|error| format!("opportunity_bridge_consumer_spawn:{error}"))
    }

    fn start_producer_durability_worker(&self) -> Result<(), String> {
        let inner = Arc::downgrade(&self.inner);
        thread::Builder::new()
            .name("nando-opportunity-durability-v1".to_owned())
            .spawn(move || {
                thread::sleep(PRODUCER_DURABILITY_INTERVAL);
                let mut last_spool_reconcile = Instant::now();
                while let Some(inner) = inner.upgrade() {
                    if inner.producer_sync_requested.swap(false, Ordering::AcqRel) {
                        let previous = inner.producer.durable_sequence.load(Ordering::Acquire);
                        let target = inner.producer.last_sequence.load(Ordering::Acquire);
                        match sync_pending_spool(&inner.pending_dir, previous, target) {
                            Ok(()) => {
                                inner
                                    .producer
                                    .durable_sequence
                                    .store(target, Ordering::Release);
                                inner
                                    .producer
                                    .durability_syncs
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                            Err(error) => {
                                inner.producer_sync_requested.store(true, Ordering::Release);
                                record_failure(&inner.producer, &error);
                            }
                        }
                    }
                    if last_spool_reconcile.elapsed() >= SPOOL_RECONCILE_INTERVAL {
                        refresh_pending_counters(&inner);
                        last_spool_reconcile = Instant::now();
                    }
                    drop(inner);
                    thread::sleep(PRODUCER_DURABILITY_INTERVAL);
                }
            })
            .map(|_| ())
            .map_err(|error| format!("opportunity_bridge_durability_spawn:{error}"))
    }

    #[must_use]
    pub fn status(&self) -> OpportunityBridgeStatusV1 {
        let counter_checkpoint_last_sequence = self
            .inner
            .counter_checkpoint
            .lock()
            .map_or(0, |checkpoint| checkpoint.last_sequence);
        OpportunityBridgeStatusV1 {
            schema: BRIDGE_STATUS_SCHEMA_V2.to_owned(),
            root: self.inner.root.display().to_string(),
            counter_checkpoint_restored: self.inner.counter_checkpoint_restored,
            counter_checkpoint_last_sequence,
            pending_events: self.inner.pending_events.load(Ordering::Acquire),
            pending_bytes: self.inner.pending_bytes.load(Ordering::Acquire),
            consumer_inflight_events: self.inner.consumer_inflight.load(Ordering::Acquire),
            durability_interval_ms: u64::try_from(PRODUCER_DURABILITY_INTERVAL.as_millis())
                .unwrap_or(u64::MAX),
            producer: endpoint_status(
                &self.inner.producer,
                self.inner.producer_enabled,
                self.inner.producer_enabled,
            ),
            consumer: endpoint_status(
                &self.inner.consumer,
                self.inner.consumer_enabled,
                self.inner.consumer_started.load(Ordering::Acquire),
            ),
        }
    }
}

fn advance_consumer_pipeline(
    inner: &BridgeInner,
    worker: &MinerWorkerHandle,
    inflight: &mut VecDeque<InflightOpportunityBatch>,
) -> Result<(), String> {
    while let Some(front) = inflight.front() {
        match front.ack.try_recv() {
            Ok(Ok(())) => {
                let acknowledged = inflight
                    .pop_front()
                    .ok_or_else(|| "opportunity_bridge_inflight_underflow".to_owned())?;
                acknowledge_pending_batch(inner, acknowledged.pending, acknowledged.started)?;
            }
            Ok(Err(error)) => return Err(format!("opportunity_bridge_worker_ack:{error}")),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                return Err("opportunity_bridge_worker_ack_disconnected".to_owned());
            }
        }
    }

    let submitted_paths = inflight
        .iter()
        .flat_map(|entry| entry.pending.iter().map(|pending| pending.path.clone()))
        .collect::<BTreeSet<_>>();
    if inflight.len() < MAX_CONSUMER_INFLIGHT_BATCHES {
        let pending = pending_batch(inner, MAX_CONSUMER_INFLIGHT_EVENTS, &submitted_paths)?;
        if !pending.is_empty() {
            let events = pending.iter().map(|row| row.event.clone()).collect();
            let ack = match worker.submit_opportunity_durable_batch_async(events) {
                Ok(ack) => ack,
                Err(error) if error == "miner_worker_queue_full" => return Ok(()),
                Err(error) => return Err(format!("opportunity_bridge_submit:{error}")),
            };
            inflight.push_back(InflightOpportunityBatch {
                pending,
                ack,
                started: Instant::now(),
            });
        }
    }
    Ok(())
}

fn record_event(
    counters: &BridgeEndpointCounters,
    event: &OpportunityBridgeEventV1,
    sequence: u64,
) {
    counters.events.fetch_add(1, Ordering::Relaxed);
    counters.last_sequence.store(sequence, Ordering::Release);
    if let Some((input_tokens, _)) = event.request_economics() {
        counters.request_events.fetch_add(1, Ordering::Relaxed);
        counters
            .request_input_tokens
            .fetch_add(input_tokens, Ordering::Relaxed);
    }
}

fn restore_endpoint_counters(
    counters: &BridgeEndpointCounters,
    checkpoint: &OpportunityBridgeCounterCheckpointV1,
    durable_sequence: u64,
) {
    counters.events.store(checkpoint.events, Ordering::Release);
    counters
        .request_events
        .store(checkpoint.request_events, Ordering::Release);
    counters
        .request_input_tokens
        .store(checkpoint.request_input_tokens, Ordering::Release);
    counters
        .last_sequence
        .store(checkpoint.last_sequence, Ordering::Release);
    counters
        .durable_sequence
        .store(durable_sequence, Ordering::Release);
    counters
        .counter_started_after_sequence
        .store(checkpoint.counter_started_after_sequence, Ordering::Release);
}

fn record_failure(counters: &BridgeEndpointCounters, error: &str) {
    counters.failures.fetch_add(1, Ordering::Relaxed);
    set_last_error(counters, error);
}

fn record_timing(counters: &BridgeEndpointCounters, started: Instant) {
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    counters.last_micros.store(micros, Ordering::Relaxed);
    counters.max_micros.fetch_max(micros, Ordering::Relaxed);
    let _ = counters
        .total_micros
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(micros))
        });
}

fn set_last_error(counters: &BridgeEndpointCounters, error: &str) {
    if let Ok(mut last_error) = counters.last_error.write() {
        *last_error = error.to_owned();
    }
}

fn endpoint_status(
    counters: &BridgeEndpointCounters,
    enabled: bool,
    ready: bool,
) -> OpportunityBridgeEndpointStatusV1 {
    OpportunityBridgeEndpointStatusV1 {
        enabled,
        ready,
        events: counters.events.load(Ordering::Relaxed),
        request_events: counters.request_events.load(Ordering::Relaxed),
        request_input_tokens: counters.request_input_tokens.load(Ordering::Relaxed),
        duplicates: counters.duplicates.load(Ordering::Relaxed),
        failures: counters.failures.load(Ordering::Relaxed),
        invalid_events: counters.invalid_events.load(Ordering::Relaxed),
        last_sequence: counters.last_sequence.load(Ordering::Acquire),
        last_micros: counters.last_micros.load(Ordering::Relaxed),
        max_micros: counters.max_micros.load(Ordering::Relaxed),
        total_micros: counters.total_micros.load(Ordering::Relaxed),
        durable_sequence: counters.durable_sequence.load(Ordering::Acquire),
        durability_syncs: counters.durability_syncs.load(Ordering::Relaxed),
        counter_started_after_sequence: counters
            .counter_started_after_sequence
            .load(Ordering::Acquire),
        last_error: counters
            .last_error
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "opportunity_bridge_status_lock_poisoned".to_owned()),
    }
}

#[cfg(test)]
mod tests;
