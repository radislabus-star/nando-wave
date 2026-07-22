use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{Arc, RwLock, mpsc::Receiver},
    time::{Duration, Instant},
};

use nando_operator_learning::{ProviderCaptureIndexV3, ProviderRequestCaptureReceiptV3};
use nando_operator_persistence::ProviderCaptureStoreV3;

use super::runtime::ActiveSequenceLeaseV3;
use super::telemetry::ProviderCaptureTelemetryV3;
use super::types::PROVIDER_CAPTURE_MAX_QUEUE_V3;

const CAPTURE_BATCH_COALESCE_V3: Duration = Duration::from_millis(2);

pub(super) fn run_provider_capture_writer_v3(
    store_path: PathBuf,
    receiver: Receiver<ProviderRequestCaptureReceiptV3>,
    active_lease: Arc<RwLock<Option<Arc<ActiveSequenceLeaseV3>>>>,
    telemetry: Arc<ProviderCaptureTelemetryV3>,
) {
    let store = match ProviderCaptureStoreV3::open(store_path) {
        Ok(store) => store,
        Err(error) => {
            telemetry.blocked(&format!("provider_capture_store_open:{error:?}"), 0);
            return;
        }
    };
    let lease = match store.reserve_sequence_lease() {
        Ok(lease) => lease,
        Err(error) => {
            telemetry.blocked(&format!("provider_capture_lease:{error:?}"), 0);
            return;
        }
    };
    let mut index = match store
        .restore()
        .ok()
        .and_then(|restore| restore.index().cloned())
    {
        Some(index) => index,
        None => {
            telemetry.blocked("provider_capture_restore_after_lease", 0);
            return;
        }
    };
    let runtime_lease = Arc::new(ActiveSequenceLeaseV3::new(
        lease.first_sequence(),
        lease.last_sequence(),
        lease.epoch_root_sha256(),
    ));
    match active_lease.write() {
        Ok(mut slot) => *slot = Some(runtime_lease),
        Err(_) => {
            telemetry.blocked("provider_capture_lease_lock_poisoned", 0);
            return;
        }
    }
    telemetry.ready(
        index.records().len(),
        index.publish_sequence(),
        index.reserved_through_sequence(),
    );

    while let Ok(first) = receiver.recv() {
        let mut batch = Vec::with_capacity(PROVIDER_CAPTURE_MAX_QUEUE_V3);
        batch.push(first);
        let deadline = Instant::now() + CAPTURE_BATCH_COALESCE_V3;
        while batch.len() < PROVIDER_CAPTURE_MAX_QUEUE_V3 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match receiver.recv_timeout(remaining) {
                Ok(receipt) => batch.push(receipt),
                Err(_) => break,
            }
        }
        let (accepted, duplicates) = filter_duplicates(&index, batch);
        if duplicates > 0 {
            telemetry.duplicate(duplicates);
        }
        if accepted.is_empty() {
            continue;
        }
        let next = match index.append_batch(&accepted) {
            Ok(next) => next,
            Err(error) => {
                telemetry.blocked(
                    &format!("provider_capture_append:{error:?}"),
                    accepted.len(),
                );
                return;
            }
        };
        if let Err(error) = store.publish_index(&next) {
            telemetry.blocked(
                &format!("provider_capture_publish:{error:?}"),
                accepted.len(),
            );
            return;
        }
        index = next;
        telemetry.captured(
            accepted.len(),
            index.records().len(),
            index.publish_sequence(),
            index.reserved_through_sequence(),
        );
    }
}

fn filter_duplicates(
    index: &ProviderCaptureIndexV3,
    batch: Vec<ProviderRequestCaptureReceiptV3>,
) -> (Vec<ProviderRequestCaptureReceiptV3>, usize) {
    let mut sequences = BTreeSet::new();
    let mut events = BTreeSet::new();
    let mut requests = BTreeSet::new();
    let mut receipts = BTreeSet::new();
    let mut accepted = Vec::with_capacity(batch.len());
    let mut duplicates = 0;
    for receipt in batch {
        let duplicate = index
            .records()
            .iter()
            .any(|existing| existing.capture_sequence() == receipt.capture_sequence())
            || index.contains_event_root(receipt.event_root_sha256())
            || index.contains_request_root(receipt.request_root_sha256())
            || index.contains_receipt_root(receipt.receipt_sha256())
            || !sequences.insert(receipt.capture_sequence())
            || !events.insert(receipt.event_root_sha256())
            || !requests.insert(receipt.request_root_sha256())
            || !receipts.insert(receipt.receipt_sha256());
        if duplicate {
            duplicates += 1;
        } else {
            accepted.push(receipt);
        }
    }
    (accepted, duplicates)
}
