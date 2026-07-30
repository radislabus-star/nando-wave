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
    assert_eq!(runtime.status().consumer.last_sequence, 2);
    assert_eq!(runtime.status().consumer.events, 2);
    assert_eq!(runtime.status().consumer.request_events, 1);
    assert_eq!(runtime.status().consumer.request_input_tokens, 17);
    assert_eq!(runtime.status().counter_checkpoint_last_sequence, 2);
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
    assert_eq!(runtime.status().producer.counter_started_after_sequence, 6);
    assert_eq!(runtime.status().producer.last_sequence, 7);
    assert_eq!(runtime.status().producer.events, 1);
    assert_eq!(runtime.status().consumer.counter_started_after_sequence, 6);
    assert!(!temporary.exists());
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejected_event_sequence_is_not_reused_after_restart() {
    let root = temporary_root("rejected-sequence");
    let rejected = root.join("rejected");
    fs::create_dir_all(&rejected).expect("rejected");
    let event = OpportunityBridgeEventV1::verified(intent());
    let digest = event.canonical_sha256().expect("digest");
    fs::write(
        rejected.join(format!("{}.invalid", event_file_name(9, &digest))),
        event.canonical_cbor().expect("event bytes"),
    )
    .expect("rejected event");

    let runtime =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("restarted producer");
    assert_eq!(runtime.inner.next_sequence.load(Ordering::Acquire), 10);
    runtime
        .submit(OpportunityBridgeEventV1::verified("43".repeat(32)))
        .expect("next event");
    let pending = pending_batch(&runtime.inner, 1, &std::collections::BTreeSet::new())
        .expect("pending event");
    assert_eq!(pending.first().map(|row| row.sequence), Some(10));
    drop(runtime);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn counter_checkpoint_restores_empty_spool_and_monotonic_sequence() {
    let root = temporary_root("counter-checkpoint-restart");
    let producer =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("producer");
    producer
        .submit(OpportunityBridgeEventV1::request(intent(), 17, 1))
        .expect("request");
    producer
        .submit(OpportunityBridgeEventV1::classify(
            intent(),
            ReducibilityClass::ExecutableCandidate,
            "candidate".to_owned(),
        ))
        .expect("classification");
    let consumer =
        OpportunityBridgeRuntime::new(root.clone(), false, true, Duration::from_millis(10))
            .expect("consumer");
    drain_pending_once(&consumer.inner, |_| Ok(())).expect("drain");
    let closed = consumer.status();
    assert_eq!(closed.consumer.counter_started_after_sequence, 0);
    assert_eq!(closed.consumer.last_sequence, 2);
    assert_eq!(closed.consumer.events, 2);
    assert_eq!(closed.consumer.request_events, 1);
    assert_eq!(closed.consumer.request_input_tokens, 17);
    assert_eq!(closed.pending_events, 0);
    drop(consumer);
    drop(producer);

    let restarted_consumer =
        OpportunityBridgeRuntime::new(root.clone(), false, true, Duration::from_millis(10))
            .expect("restarted consumer");
    let restored = restarted_consumer.status();
    assert!(restored.counter_checkpoint_restored);
    assert_eq!(restored.counter_checkpoint_last_sequence, 2);
    assert_eq!(restored.consumer.counter_started_after_sequence, 0);
    assert_eq!(restored.consumer.last_sequence, 2);
    assert_eq!(restored.consumer.events, 2);
    assert_eq!(restored.consumer.request_events, 1);
    assert_eq!(restored.consumer.request_input_tokens, 17);

    let restarted_producer =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("restarted producer");
    assert_eq!(
        restarted_producer
            .inner
            .next_sequence
            .load(Ordering::Acquire),
        3
    );
    assert_eq!(restarted_producer.status().producer.last_sequence, 2);
    assert_eq!(restarted_producer.status().producer.events, 2);
    restarted_producer
        .submit(OpportunityBridgeEventV1::verified(intent()))
        .expect("post-restart event");
    drain_pending_once(&restarted_consumer.inner, |_| Ok(())).expect("post-restart drain");
    assert_eq!(restarted_consumer.status().consumer.last_sequence, 3);
    assert_eq!(restarted_consumer.status().consumer.events, 3);
    assert_eq!(restarted_consumer.status().consumer.request_events, 1);
    assert_eq!(
        restarted_consumer.status().consumer.request_input_tokens,
        17
    );
    drop(restarted_consumer);
    drop(restarted_producer);

    let final_producer =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("final producer");
    assert_eq!(
        final_producer.inner.next_sequence.load(Ordering::Acquire),
        4
    );
    assert_eq!(final_producer.status().producer.last_sequence, 3);
    assert_eq!(final_producer.status().producer.events, 3);
    drop(final_producer);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn restart_discards_checkpointed_spool_residue_without_redelivery() {
    let root = temporary_root("checkpointed-residue");
    let producer =
        OpportunityBridgeRuntime::new(root.clone(), true, false, Duration::from_millis(10))
            .expect("producer");
    let event = OpportunityBridgeEventV1::request(intent(), 17, 1);
    producer.submit(event.clone()).expect("request");
    let consumer =
        OpportunityBridgeRuntime::new(root.clone(), false, true, Duration::from_millis(10))
            .expect("consumer");
    drain_pending_once(&consumer.inner, |_| Ok(())).expect("drain");
    let digest = event.canonical_sha256().expect("digest");
    fs::write(
        root.join("pending").join(event_file_name(1, &digest)),
        event.canonical_cbor().expect("event bytes"),
    )
    .expect("restore acknowledged residue");
    drop(consumer);

    let restarted =
        OpportunityBridgeRuntime::new(root.clone(), false, true, Duration::from_millis(10))
            .expect("restarted consumer");
    assert_eq!(restarted.status().pending_events, 0);
    let deliveries = AtomicUsize::new(0);
    drain_pending_once(&restarted.inner, |_| {
        deliveries.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .expect("empty drain");
    assert_eq!(deliveries.load(Ordering::Relaxed), 0);
    assert_eq!(restarted.status().consumer.events, 1);
    assert_eq!(restarted.status().consumer.last_sequence, 1);
    drop(restarted);
    drop(producer);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn corrupt_counter_checkpoint_fails_closed() {
    let root = temporary_root("counter-checkpoint-corrupt");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join("counter-checkpoint-v1.json"), b"{}").expect("corrupt checkpoint");
    let error = OpportunityBridgeRuntime::new(root.clone(), false, true, Duration::from_millis(10))
        .err()
        .expect("corrupt checkpoint must fail");
    assert!(error.starts_with("opportunity_bridge_counter_checkpoint_decode:"));
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
