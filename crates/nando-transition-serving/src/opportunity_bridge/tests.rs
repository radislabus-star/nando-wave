use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use nando_operator_learning::{
    OpportunityBridgeEventKindV1, OpportunityBridgeEventV1, ReducibilityClass,
};

use super::OpportunityBridgeRuntime;
use super::spool::{drain_pending_once, event_file_name};

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
    assert!(!temporary.exists());
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}
