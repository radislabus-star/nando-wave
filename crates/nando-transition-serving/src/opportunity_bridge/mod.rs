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
use spool::{
    PendingBridgeEvent, acknowledge_pending, create_private_directory, next_pending_sequence,
    pending_batch, pending_stats, persist_event, recover_temporary_events, sync_pending_spool,
};

const BRIDGE_STATUS_SCHEMA_V1: &str = "nando.opportunity-process-bridge-status.v1";
const PRODUCER_DURABILITY_INTERVAL: Duration = Duration::from_millis(10);
const MAX_CONSUMER_INFLIGHT_EVENTS: usize = 256;

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
    pub last_error: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct OpportunityBridgeStatusV1 {
    pub schema: String,
    pub root: String,
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

struct BridgeInner {
    root: PathBuf,
    staging_dir: PathBuf,
    pending_dir: PathBuf,
    rejected_dir: PathBuf,
    producer_enabled: bool,
    consumer_enabled: bool,
    consumer_poll: Duration,
    next_sequence: AtomicU64,
    persist_lock: Mutex<()>,
    producer: BridgeEndpointCounters,
    consumer: BridgeEndpointCounters,
    consumer_started: AtomicBool,
    consumer_inflight: AtomicU64,
    producer_sync_requested: AtomicBool,
}

struct InflightOpportunity {
    pending: PendingBridgeEvent,
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
        let next_sequence = next_pending_sequence(&[&pending_dir, &staging_dir])?;
        let producer = BridgeEndpointCounters::default();
        producer
            .last_sequence
            .store(next_sequence.saturating_sub(1), Ordering::Release);
        let runtime = Self {
            inner: Arc::new(BridgeInner {
                root,
                staging_dir,
                pending_dir,
                rejected_dir,
                producer_enabled,
                consumer_enabled,
                consumer_poll: consumer_poll.max(Duration::from_millis(10)),
                next_sequence: AtomicU64::new(next_sequence),
                persist_lock: Mutex::new(()),
                producer,
                consumer: BridgeEndpointCounters::default(),
                consumer_started: AtomicBool::new(false),
                consumer_inflight: AtomicU64::new(0),
                producer_sync_requested: AtomicBool::new(next_sequence > 1),
            }),
        };
        if producer_enabled {
            runtime.start_producer_durability_worker()?;
        }
        Ok(runtime)
    }

    #[must_use]
    pub fn producer_enabled(&self) -> bool {
        self.inner.producer_enabled
    }

    pub fn submit(&self, event: OpportunityBridgeEventV1) -> Result<(), String> {
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
                while let Some(inner) = inner.upgrade() {
                    if let Err(error) = advance_consumer_pipeline(&inner, &worker, &mut inflight) {
                        record_failure(&inner.consumer, &error);
                        inner.consumer_started.store(false, Ordering::Release);
                        return;
                    }
                    inner.consumer_inflight.store(
                        u64::try_from(inflight.len()).unwrap_or(u64::MAX),
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
                    drop(inner);
                    thread::sleep(PRODUCER_DURABILITY_INTERVAL);
                }
            })
            .map(|_| ())
            .map_err(|error| format!("opportunity_bridge_durability_spawn:{error}"))
    }

    #[must_use]
    pub fn status(&self) -> OpportunityBridgeStatusV1 {
        let (pending_events, pending_bytes) = pending_stats(&self.inner.pending_dir);
        OpportunityBridgeStatusV1 {
            schema: BRIDGE_STATUS_SCHEMA_V1.to_owned(),
            root: self.inner.root.display().to_string(),
            pending_events,
            pending_bytes,
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
    inflight: &mut VecDeque<InflightOpportunity>,
) -> Result<(), String> {
    while let Some(front) = inflight.front() {
        match front.ack.try_recv() {
            Ok(Ok(())) => {
                let acknowledged = inflight
                    .pop_front()
                    .ok_or_else(|| "opportunity_bridge_inflight_underflow".to_owned())?;
                acknowledge_pending(inner, acknowledged.pending, acknowledged.started)?;
            }
            Ok(Err(error)) => return Err(format!("opportunity_bridge_worker_ack:{error}")),
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                return Err("opportunity_bridge_worker_ack_disconnected".to_owned());
            }
        }
    }

    let mut submitted_paths = inflight
        .iter()
        .map(|entry| entry.pending.path.clone())
        .collect::<BTreeSet<_>>();
    for pending in pending_batch(inner)? {
        if inflight.len() >= MAX_CONSUMER_INFLIGHT_EVENTS {
            break;
        }
        if !submitted_paths.insert(pending.path.clone()) {
            continue;
        }
        let ack = match worker.submit_opportunity_durable_async(pending.event.clone()) {
            Ok(ack) => ack,
            Err(error) if error == "miner_worker_queue_full" => break,
            Err(error) => return Err(format!("opportunity_bridge_submit:{error}")),
        };
        inflight.push_back(InflightOpportunity {
            pending,
            ack,
            started: Instant::now(),
        });
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
        last_error: counters
            .last_error
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "opportunity_bridge_status_lock_poisoned".to_owned()),
    }
}

#[cfg(test)]
mod tests;
