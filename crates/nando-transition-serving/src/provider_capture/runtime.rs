use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;

use nando_operator_learning::{
    ProviderRequestCaptureInputV3, ProviderRequestCaptureReceiptV3,
    seal_provider_request_capture_v3,
};

use super::telemetry::ProviderCaptureTelemetryV3;
use super::types::{
    ProviderCaptureCensoredReasonV3, ProviderCaptureConfigV3, ProviderCaptureIngressV3,
    ProviderCaptureStatusV3, ProviderCaptureSubmitV3,
};
use super::worker::run_provider_capture_writer_v3;

pub(crate) struct ProviderCaptureRuntimeV3 {
    config: ProviderCaptureConfigV3,
    telemetry: Arc<ProviderCaptureTelemetryV3>,
    sender: OnceLock<SyncSender<ProviderRequestCaptureReceiptV3>>,
    lease: Arc<RwLock<Option<Arc<ActiveSequenceLeaseV3>>>>,
    started: AtomicBool,
}

pub(super) struct ActiveSequenceLeaseV3 {
    next_sequence: AtomicU64,
    last_sequence: u64,
    epoch_root_sha256: nando_operator_kernel::Sha256CommitmentV3,
}

impl ProviderCaptureRuntimeV3 {
    pub(crate) fn new(config: ProviderCaptureConfigV3) -> Result<Self, &'static str> {
        config.validate()?;
        let enabled = config.enabled;
        Ok(Self {
            config,
            telemetry: Arc::new(ProviderCaptureTelemetryV3::new(enabled)),
            sender: OnceLock::new(),
            lease: Arc::new(RwLock::new(None)),
            started: AtomicBool::new(false),
        })
    }

    pub(crate) fn start_after_http_bind(self: &Arc<Self>) {
        if !self.config.enabled {
            return;
        }
        if self
            .started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let (sender, receiver) = sync_channel(self.config.queue_capacity);
        if self.sender.set(sender).is_err() {
            self.telemetry
                .blocked("provider_capture_sender_already_set", 0);
            return;
        }
        let store_path = self.config.store_path.clone();
        let lease = Arc::clone(&self.lease);
        let telemetry = Arc::clone(&self.telemetry);
        if let Err(error) = thread::Builder::new()
            .name("nando-provider-capture-writer".to_owned())
            .spawn(move || run_provider_capture_writer_v3(store_path, receiver, lease, telemetry))
        {
            self.telemetry
                .blocked(&format!("provider_capture_writer_spawn:{error}"), 0);
        }
    }

    pub(crate) fn try_capture(&self, ingress: ProviderCaptureIngressV3) -> ProviderCaptureSubmitV3 {
        self.telemetry.begin_submit();
        if !self.config.enabled {
            self.censored(ProviderCaptureCensoredReasonV3::Disabled)
        } else if !self.started.load(Ordering::Acquire) {
            self.censored(ProviderCaptureCensoredReasonV3::NotStarted)
        } else {
            self.try_capture_started(ingress)
        }
    }

    pub(crate) fn observe_invalid_provenance(&self) {
        self.telemetry.begin_submit();
        self.telemetry
            .ingress_censored(ProviderCaptureCensoredReasonV3::InvalidProvenance);
    }

    #[must_use]
    pub(crate) fn status(&self) -> ProviderCaptureStatusV3 {
        self.telemetry.snapshot()
    }

    fn try_capture_started(&self, ingress: ProviderCaptureIngressV3) -> ProviderCaptureSubmitV3 {
        let lease = match self.lease.try_read() {
            Ok(lease) => lease.as_ref().cloned(),
            Err(_) => None,
        };
        let Some(lease) = lease else {
            return self.censored(ProviderCaptureCensoredReasonV3::NoDurableLease);
        };
        let Some(sequence) = lease.allocate() else {
            self.telemetry.exhausted();
            return self.censored(ProviderCaptureCensoredReasonV3::BudgetExhausted);
        };
        let receipt = match seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: sequence,
            capture_epoch_root: lease.epoch_root_sha256,
            lineage_root_sha256: ingress.lineage_root_sha256,
            request_root_sha256: ingress.request_root_sha256,
            projection: ingress.projection,
            streaming: ingress.streaming,
            observed_at_unix_ms: ingress.observed_at_unix_ms,
        }) {
            Ok(receipt) => receipt,
            Err(_) => {
                return self.censored(ProviderCaptureCensoredReasonV3::InvalidProvenance);
            }
        };
        match self.sender.get() {
            Some(sender) => {
                // Publish the queue ownership before try_send makes the receipt
                // observable to the writer. This keeps status accounting monotonic.
                self.telemetry.begin_enqueue();
                match sender.try_send(receipt.clone()) {
                    Ok(()) => ProviderCaptureSubmitV3::Enqueued(receipt),
                    Err(TrySendError::Full(_)) => {
                        self.telemetry.reclassify_enqueue_as_censored(
                            ProviderCaptureCensoredReasonV3::QueueFull,
                        );
                        ProviderCaptureSubmitV3::Censored(
                            ProviderCaptureCensoredReasonV3::QueueFull,
                        )
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        self.telemetry.reclassify_enqueue_as_censored(
                            ProviderCaptureCensoredReasonV3::Disconnected,
                        );
                        ProviderCaptureSubmitV3::Censored(
                            ProviderCaptureCensoredReasonV3::Disconnected,
                        )
                    }
                }
            }
            None => self.censored(ProviderCaptureCensoredReasonV3::NotStarted),
        }
    }

    fn censored(&self, reason: ProviderCaptureCensoredReasonV3) -> ProviderCaptureSubmitV3 {
        self.telemetry.ingress_censored(reason);
        ProviderCaptureSubmitV3::Censored(reason)
    }
}

impl ActiveSequenceLeaseV3 {
    pub(super) fn new(
        first_sequence: u64,
        last_sequence: u64,
        epoch_root_sha256: nando_operator_kernel::Sha256CommitmentV3,
    ) -> Self {
        Self {
            next_sequence: AtomicU64::new(first_sequence),
            last_sequence,
            epoch_root_sha256,
        }
    }

    fn allocate(&self) -> Option<u64> {
        self.next_sequence
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |sequence| {
                (sequence <= self.last_sequence).then(|| sequence.saturating_add(1))
            })
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};

    use super::*;

    #[test]
    fn queue_overload_is_censored_without_semantic_updates() {
        let runtime = ProviderCaptureRuntimeV3::new(ProviderCaptureConfigV3 {
            enabled: true,
            store_path: PathBuf::from("unused-f8a-overload"),
            queue_capacity: 1,
        })
        .expect("runtime");
        let (sender, _receiver) = sync_channel(1);
        runtime.sender.set(sender).expect("sender");
        *runtime.lease.write().expect("lease") =
            Some(Arc::new(ActiveSequenceLeaseV3::new(1, 2, root("epoch"))));
        runtime.started.store(true, Ordering::Release);

        assert!(matches!(
            runtime.try_capture(ingress("request-a", 1)),
            ProviderCaptureSubmitV3::Enqueued(_)
        ));
        assert_eq!(
            runtime.try_capture(ingress("request-b", 2)),
            ProviderCaptureSubmitV3::Censored(ProviderCaptureCensoredReasonV3::QueueFull)
        );

        let status = runtime.status();
        assert_eq!(status.submitted, 2);
        assert_eq!(status.enqueued, 1);
        assert_eq!(status.censored, 1);
        assert_eq!(status.queue_full, 1);
        assert_eq!(status.semantic_updates_from_censored, 0);
        assert!(status.accounting_identity_holds);
    }

    fn ingress(label: &str, observed_at_unix_ms: u64) -> ProviderCaptureIngressV3 {
        ProviderCaptureIngressV3 {
            lineage_root_sha256: root("lineage"),
            request_root_sha256: root(label),
            projection: RuntimeProjectionV3::Responses,
            streaming: false,
            observed_at_unix_ms,
        }
    }

    fn root(label: &str) -> Sha256CommitmentV3 {
        Sha256CommitmentV3::digest_bytes(label.as_bytes())
    }
}
