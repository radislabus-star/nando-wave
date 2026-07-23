mod transport;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

use axum::body::Bytes;
use nando_operator_learning::{LearningRequestStructureV1, ProviderRequestCaptureReceiptV3};
use serde::Serialize;

use crate::generation_shadow::GenerationShadowRuntimeV3;
use crate::session_stream::RequestLearningIndex;

const LEARNING_EVIDENCE_BRIDGE_SCHEMA_V1: &str = "nando.learning-evidence-bridge-status.v1";
pub const LEARNING_EVIDENCE_BRIDGE_MAX_QUEUE_V1: usize = 48;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LearningEvidenceBridgeEndpointStatusV1 {
    pub enabled: bool,
    pub ready: bool,
    pub submitted: u64,
    pub enqueued: u64,
    pub received: u64,
    pub accepted: u64,
    pub censored: u64,
    pub queue_full: u64,
    pub failures: u64,
    pub invalid: u64,
    pub provider_bound_turns: u64,
    pub session_bound_requests: u64,
    pub capability_bound_requests: u64,
    pub raw_eligible: u64,
    pub raw_accepted: u64,
    pub raw_censored: u64,
    pub raw_budget_censored: u64,
    pub payload_bytes: u64,
    pub last_submit_micros: u64,
    pub max_submit_micros: u64,
    pub last_error: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LearningEvidenceBridgeStatusV1 {
    pub schema: String,
    pub socket_path: String,
    pub producer: LearningEvidenceBridgeEndpointStatusV1,
    pub consumer: LearningEvidenceBridgeEndpointStatusV1,
    pub raw_payloads_persisted: u8,
    pub execution_authority: bool,
}

#[derive(Clone)]
pub struct LearningEvidenceBridgeRuntimeV1 {
    inner: Arc<LearningEvidenceBridgeInnerV1>,
}

pub(super) struct LearningEvidenceIngressV1 {
    pub(super) capture_receipt: ProviderRequestCaptureReceiptV3,
    pub(super) structure: LearningRequestStructureV1,
    pub(super) provider_payload: Bytes,
}

pub(super) struct LearningEvidenceBridgeInnerV1 {
    pub(super) socket_path: PathBuf,
    producer_enabled: bool,
    consumer_enabled: bool,
    queue_capacity: usize,
    sender: OnceLock<SyncSender<LearningEvidenceIngressV1>>,
    producer_started: AtomicBool,
    consumer_started: AtomicBool,
    pub(super) producer: EndpointCountersV1,
    pub(super) consumer: EndpointCountersV1,
}

#[derive(Default)]
pub(super) struct EndpointCountersV1 {
    submitted: AtomicU64,
    enqueued: AtomicU64,
    received: AtomicU64,
    accepted: AtomicU64,
    censored: AtomicU64,
    queue_full: AtomicU64,
    failures: AtomicU64,
    invalid: AtomicU64,
    provider_bound_turns: AtomicU64,
    session_bound_requests: AtomicU64,
    capability_bound_requests: AtomicU64,
    raw_eligible: AtomicU64,
    raw_accepted: AtomicU64,
    raw_censored: AtomicU64,
    raw_budget_censored: AtomicU64,
    payload_bytes: AtomicU64,
    last_submit_micros: AtomicU64,
    max_submit_micros: AtomicU64,
    last_error: RwLock<String>,
}

impl LearningEvidenceBridgeRuntimeV1 {
    pub fn new(
        socket_path: PathBuf,
        producer_enabled: bool,
        consumer_enabled: bool,
        queue_capacity: usize,
    ) -> Result<Self, String> {
        if producer_enabled && consumer_enabled {
            return Err("learning_evidence_bridge_roles_overlap".to_owned());
        }
        if queue_capacity == 0 || queue_capacity > LEARNING_EVIDENCE_BRIDGE_MAX_QUEUE_V1 {
            return Err("learning_evidence_bridge_queue_invalid".to_owned());
        }
        if socket_path.file_name().is_none() {
            return Err("learning_evidence_bridge_socket_invalid".to_owned());
        }
        Ok(Self {
            inner: Arc::new(LearningEvidenceBridgeInnerV1 {
                socket_path,
                producer_enabled,
                consumer_enabled,
                queue_capacity,
                sender: OnceLock::new(),
                producer_started: AtomicBool::new(false),
                consumer_started: AtomicBool::new(false),
                producer: EndpointCountersV1::default(),
                consumer: EndpointCountersV1::default(),
            }),
        })
    }

    pub fn start(
        &self,
        generation_shadow: Arc<GenerationShadowRuntimeV3>,
        request_learning: Arc<RequestLearningIndex>,
    ) -> Result<(), String> {
        if self.inner.producer_enabled {
            self.start_producer()?;
        }
        if self.inner.consumer_enabled {
            transport::start_consumer(
                Arc::clone(&self.inner),
                generation_shadow,
                request_learning,
            )?;
        }
        Ok(())
    }

    fn start_producer(&self) -> Result<(), String> {
        if self
            .inner
            .producer_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(());
        }
        let (sender, receiver) = sync_channel(self.inner.queue_capacity);
        self.inner
            .sender
            .set(sender)
            .map_err(|_| "learning_evidence_bridge_sender_already_set".to_owned())?;
        transport::start_producer(Arc::clone(&self.inner), receiver)
    }

    pub fn submit(
        &self,
        capture_receipt: ProviderRequestCaptureReceiptV3,
        structure: LearningRequestStructureV1,
        provider_payload: Bytes,
    ) -> Result<(), String> {
        if !self.inner.producer_enabled {
            return Err("learning_evidence_bridge_producer_disabled".to_owned());
        }
        let started = Instant::now();
        self.inner
            .producer
            .submitted
            .fetch_add(1, Ordering::Relaxed);
        if provider_payload.is_empty() {
            self.inner.producer.censored.fetch_add(1, Ordering::Relaxed);
            record_submit_timing(&self.inner.producer, started);
            return Err("learning_evidence_bridge_payload_budget".to_owned());
        }
        let sender = self
            .inner
            .sender
            .get()
            .ok_or_else(|| "learning_evidence_bridge_producer_not_started".to_owned())?;
        let result = match sender.try_send(LearningEvidenceIngressV1 {
            capture_receipt,
            structure,
            provider_payload,
        }) {
            Ok(()) => {
                self.inner.producer.enqueued.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Full(_)) => {
                self.inner
                    .producer
                    .queue_full
                    .fetch_add(1, Ordering::Relaxed);
                self.inner.producer.censored.fetch_add(1, Ordering::Relaxed);
                Err("learning_evidence_bridge_queue_full".to_owned())
            }
            Err(TrySendError::Disconnected(_)) => {
                record_failure(&self.inner.producer, "learning_evidence_bridge_stopped");
                Err("learning_evidence_bridge_stopped".to_owned())
            }
        };
        record_submit_timing(&self.inner.producer, started);
        result
    }

    #[must_use]
    pub fn producer_enabled(&self) -> bool {
        self.inner.producer_enabled
    }

    #[must_use]
    pub fn status(&self) -> LearningEvidenceBridgeStatusV1 {
        LearningEvidenceBridgeStatusV1 {
            schema: LEARNING_EVIDENCE_BRIDGE_SCHEMA_V1.to_owned(),
            socket_path: self.inner.socket_path.display().to_string(),
            producer: endpoint_status(
                &self.inner.producer,
                self.inner.producer_enabled,
                self.inner.producer_started.load(Ordering::Acquire),
            ),
            consumer: endpoint_status(
                &self.inner.consumer,
                self.inner.consumer_enabled,
                self.inner.consumer_started.load(Ordering::Acquire),
            ),
            raw_payloads_persisted: 0,
            execution_authority: false,
        }
    }
}

pub(super) fn record_failure(counters: &EndpointCountersV1, error: &str) {
    counters.failures.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut last_error) = counters.last_error.write() {
        *last_error = error.to_owned();
    }
}

fn record_submit_timing(counters: &EndpointCountersV1, started: Instant) {
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    counters.last_submit_micros.store(micros, Ordering::Relaxed);
    counters
        .max_submit_micros
        .fetch_max(micros, Ordering::Relaxed);
}

fn endpoint_status(
    counters: &EndpointCountersV1,
    enabled: bool,
    ready: bool,
) -> LearningEvidenceBridgeEndpointStatusV1 {
    LearningEvidenceBridgeEndpointStatusV1 {
        enabled,
        ready,
        submitted: counters.submitted.load(Ordering::Relaxed),
        enqueued: counters.enqueued.load(Ordering::Relaxed),
        received: counters.received.load(Ordering::Relaxed),
        accepted: counters.accepted.load(Ordering::Relaxed),
        censored: counters.censored.load(Ordering::Relaxed),
        queue_full: counters.queue_full.load(Ordering::Relaxed),
        failures: counters.failures.load(Ordering::Relaxed),
        invalid: counters.invalid.load(Ordering::Relaxed),
        provider_bound_turns: counters.provider_bound_turns.load(Ordering::Relaxed),
        session_bound_requests: counters.session_bound_requests.load(Ordering::Relaxed),
        capability_bound_requests: counters.capability_bound_requests.load(Ordering::Relaxed),
        raw_eligible: counters.raw_eligible.load(Ordering::Relaxed),
        raw_accepted: counters.raw_accepted.load(Ordering::Relaxed),
        raw_censored: counters.raw_censored.load(Ordering::Relaxed),
        raw_budget_censored: counters.raw_budget_censored.load(Ordering::Relaxed),
        payload_bytes: counters.payload_bytes.load(Ordering::Relaxed),
        last_submit_micros: counters.last_submit_micros.load(Ordering::Relaxed),
        max_submit_micros: counters.max_submit_micros.load(Ordering::Relaxed),
        last_error: counters
            .last_error
            .read()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "learning_evidence_bridge_status_lock_poisoned".to_owned()),
    }
}

#[cfg(test)]
mod tests;
