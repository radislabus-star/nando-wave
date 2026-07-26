use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use nando_operator_learning::{
    OpportunityBridgeEventKindV1, OpportunityBridgeEventV1, ReducibilityClass,
};

use super::OpportunityBridgeRuntime;
use super::spool::{drain_pending_once, event_file_name, pending_batch};

fn temporary_root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nando-opportunity-bridge-{name}-{}-{}",
        std::process::id(),
        crate::unix_now()
    ))
}

fn intent() -> String {
    "42".repeat(32)
}

#[test]
fn spool_capacity_rejects_new_events_before_disk_growth() {
    let root = temporary_root("capacity");
    let rejected = root.join("rejected");
    fs::create_dir_all(&rejected).expect("rejected");
    let sentinel = rejected.join("capacity.sparse");
    fs::File::create(&sentinel)
        .expect("sentinel")
        .set_len(super::spool::MAX_SPOOL_BYTES)
        .expect("sparse capacity sentinel");
    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("runtime");
    let error = runtime
        .submit(OpportunityBridgeEventV1::request(intent(), 17, 1))
        .expect_err("full spool must reject");
    assert!(error.starts_with("opportunity_bridge_spool_capacity_exceeded:"));
    assert_eq!(runtime.status().pending_events, 0);
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn spool_preserves_event_order_and_deletes_only_after_ack() {
    let root = temporary_root("ordered");
    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("runtime");
    runtime
        .submit(OpportunityBridgeEventV1::request(intent(), 17, 1))
        .expect("request");
    runtime
        .submit(OpportunityBridgeEventV1::classify(
            intent(),
            ReducibilityClass::ExecutableCandidate,
            "candidate".to_owned(),
        ))
        .expect("classification");
    for _ in 0..100 {
        if runtime.status().producer.durable_sequence >= 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(runtime.status().producer.durable_sequence, 2);
    let mut delivered = Vec::new();
    drain_pending_once(&runtime.inner, |event| {
        delivered.push(event);
        Ok(())
    })
    .expect("drain");
    assert!(matches!(
        delivered.first().map(|event| &event.event),
        Some(OpportunityBridgeEventKindV1::Request { .. })
    ));
    assert!(matches!(
        delivered.get(1).map(|event| &event.event),
        Some(OpportunityBridgeEventKindV1::Classify { .. })
    ));
    assert_eq!(runtime.status().pending_events, 0);
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_delivery_remains_pending_for_at_least_once_retry() {
    let root = temporary_root("retry");
    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("runtime");
    runtime
        .submit(OpportunityBridgeEventV1::verified(intent()))
        .expect("event");
    let attempts = AtomicUsize::new(0);
    assert!(
        drain_pending_once(&runtime.inner, |_| {
            attempts.fetch_add(1, Ordering::Relaxed);
            Err("cold_worker_unavailable".to_owned())
        })
        .is_err()
    );
    assert_eq!(runtime.status().pending_events, 1);
    drain_pending_once(&runtime.inner, |_| {
        attempts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .expect("retry");
    assert_eq!(attempts.load(Ordering::Relaxed), 2);
    assert_eq!(runtime.status().pending_events, 0);
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn producer_recovers_a_complete_staging_event_after_restart() {
    let root = temporary_root("staging-recovery");
    let staging = root.join("staging");
    fs::create_dir_all(&staging).expect("staging");
    let event = OpportunityBridgeEventV1::verified(intent());
    let digest = event.canonical_sha256().expect("digest");
    let name = event_file_name(7, &digest);
    let temporary = staging.join(format!("{name}.tmp"));
    fs::write(&temporary, event.canonical_cbor().expect("event bytes")).expect("staged event");

    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("restarted producer");
    assert_eq!(runtime.status().pending_events, 1);
    assert_eq!(runtime.inner.next_sequence.load(Ordering::Acquire), 8);
    assert_eq!(runtime.status().producer.counter_started_after_sequence, 7);
    assert_eq!(runtime.status().consumer.counter_started_after_sequence, 6);
    assert!(!temporary.exists());
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn pending_batch_decodes_only_the_requested_backlog_window() {
    let root = temporary_root("bounded-batch");
    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("runtime");
    for index in 0..(super::MAX_CONSUMER_INFLIGHT_EVENTS + 17) {
        runtime
            .submit(OpportunityBridgeEventV1::request(
                format!("{index:064x}"),
                17,
                u64::try_from(index.saturating_add(1)).unwrap_or(u64::MAX),
            ))
            .expect("request");
    }

    let first = pending_batch(&runtime.inner, 17, &std::collections::BTreeSet::new())
        .expect("bounded batch");
    assert_eq!(first.len(), 17);
    assert_eq!(first.first().map(|event| event.sequence), Some(1));
    assert_eq!(first.last().map(|event| event.sequence), Some(17));
    let excluded = first
        .iter()
        .map(|event| event.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let second = pending_batch(&runtime.inner, 5, &excluded).expect("next bounded batch");
    assert_eq!(second.len(), 5);
    assert_eq!(second.first().map(|event| event.sequence), Some(18));
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}
