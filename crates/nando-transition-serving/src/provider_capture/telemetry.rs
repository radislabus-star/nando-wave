use std::sync::RwLock;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use super::types::{ProviderCaptureCensoredReasonV3, ProviderCaptureStatusV3};

const PHASE_DISABLED: u8 = 0;
const PHASE_STARTING: u8 = 1;
const PHASE_READY: u8 = 2;
const PHASE_BLOCKED: u8 = 3;
const PHASE_EXHAUSTED: u8 = 4;

#[derive(Clone, Debug, Default)]
struct DurableStateV3 {
    records: u64,
    publish_sequence: u64,
    reserved_through_sequence: u64,
    last_error: String,
}

pub(super) struct ProviderCaptureTelemetryV3 {
    enabled: bool,
    phase: AtomicU8,
    durable: RwLock<DurableStateV3>,
    submitted: AtomicU64,
    enqueued: AtomicU64,
    captured: AtomicU64,
    censored: AtomicU64,
    ingress_censored: AtomicU64,
    queue_full: AtomicU64,
    duplicates: AtomicU64,
    persistence_failures: AtomicU64,
    post_enqueue_censored: AtomicU64,
}

impl ProviderCaptureTelemetryV3 {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            phase: AtomicU8::new(if enabled {
                PHASE_STARTING
            } else {
                PHASE_DISABLED
            }),
            durable: RwLock::new(DurableStateV3::default()),
            submitted: AtomicU64::new(0),
            enqueued: AtomicU64::new(0),
            captured: AtomicU64::new(0),
            censored: AtomicU64::new(0),
            ingress_censored: AtomicU64::new(0),
            queue_full: AtomicU64::new(0),
            duplicates: AtomicU64::new(0),
            persistence_failures: AtomicU64::new(0),
            post_enqueue_censored: AtomicU64::new(0),
        }
    }

    pub(super) fn begin_submit(&self) {
        self.submitted.fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn begin_enqueue(&self) {
        self.enqueued.fetch_add(1, Ordering::Release);
    }

    pub(super) fn ingress_censored(&self, reason: ProviderCaptureCensoredReasonV3) {
        self.censored.fetch_add(1, Ordering::Relaxed);
        self.ingress_censored.fetch_add(1, Ordering::Relaxed);
        if reason == ProviderCaptureCensoredReasonV3::QueueFull {
            self.queue_full.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(super) fn reclassify_enqueue_as_censored(&self, reason: ProviderCaptureCensoredReasonV3) {
        self.enqueued.fetch_sub(1, Ordering::AcqRel);
        self.ingress_censored(reason);
    }

    pub(super) fn ready(&self, records: usize, publish_sequence: u64, reserved: u64) {
        self.update_durable(records, publish_sequence, reserved, "");
        self.phase.store(PHASE_READY, Ordering::Release);
    }

    pub(super) fn captured(&self, count: usize, records: usize, publish: u64, reserved: u64) {
        self.captured
            .fetch_add(u64::try_from(count).unwrap_or(u64::MAX), Ordering::Relaxed);
        self.update_durable(records, publish, reserved, "");
    }

    pub(super) fn duplicate(&self, count: usize) {
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        self.censored.fetch_add(count, Ordering::Relaxed);
        self.duplicates.fetch_add(count, Ordering::Relaxed);
    }

    pub(super) fn blocked(&self, error: &str, queued: usize) {
        let queued = u64::try_from(queued).unwrap_or(u64::MAX);
        self.post_enqueue_censored
            .fetch_add(queued, Ordering::Relaxed);
        self.censored.fetch_add(queued, Ordering::Relaxed);
        self.persistence_failures.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut durable) = self.durable.write() {
            durable.last_error = error.to_owned();
        }
        self.phase.store(PHASE_BLOCKED, Ordering::Release);
    }

    pub(super) fn exhausted(&self) {
        self.phase.store(PHASE_EXHAUSTED, Ordering::Release);
    }

    pub(super) fn snapshot(&self) -> ProviderCaptureStatusV3 {
        let durable = self
            .durable
            .read()
            .map(|durable| durable.clone())
            .unwrap_or_default();
        let enqueued = self.enqueued.load(Ordering::Relaxed);
        let captured = self.captured.load(Ordering::Relaxed);
        let duplicates = self.duplicates.load(Ordering::Relaxed);
        let post_enqueue_censored = self.post_enqueue_censored.load(Ordering::Relaxed);
        let ingress_censored = self.ingress_censored.load(Ordering::Relaxed);
        let queued_now = enqueued.saturating_sub(
            captured
                .saturating_add(duplicates)
                .saturating_add(post_enqueue_censored),
        );
        ProviderCaptureStatusV3 {
            enabled: self.enabled,
            phase: phase_name(self.phase.load(Ordering::Acquire)).to_owned(),
            submitted: self.submitted.load(Ordering::Relaxed),
            enqueued,
            captured,
            censored: self.censored.load(Ordering::Relaxed),
            ingress_censored,
            writer_censored: duplicates.saturating_add(post_enqueue_censored),
            queue_full: self.queue_full.load(Ordering::Relaxed),
            duplicates,
            persistence_failures: self.persistence_failures.load(Ordering::Relaxed),
            queued_now,
            records: durable.records,
            publish_sequence: durable.publish_sequence,
            reserved_through_sequence: durable.reserved_through_sequence,
            restart_sequence_reuse: 0,
            raw_payloads_persisted: 0,
            semantic_updates_from_censored: 0,
            local_accepts: 0,
            execution_authority: false,
            accounting_identity_holds: self.submitted.load(Ordering::Relaxed)
                == enqueued.saturating_add(ingress_censored)
                && enqueued
                    == captured
                        .saturating_add(duplicates)
                        .saturating_add(post_enqueue_censored)
                        .saturating_add(queued_now),
            last_error: durable.last_error,
        }
    }

    fn update_durable(&self, records: usize, publish: u64, reserved: u64, error: &str) {
        if let Ok(mut durable) = self.durable.write() {
            durable.records = u64::try_from(records).unwrap_or(u64::MAX);
            durable.publish_sequence = publish;
            durable.reserved_through_sequence = reserved;
            durable.last_error = error.to_owned();
        }
    }
}

const fn phase_name(phase: u8) -> &'static str {
    match phase {
        PHASE_STARTING => "starting",
        PHASE_READY => "ready_hash_only",
        PHASE_BLOCKED => "blocked_fail_closed",
        PHASE_EXHAUSTED => "budget_exhausted",
        _ => "disabled",
    }
}
