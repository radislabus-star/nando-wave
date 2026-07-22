use std::sync::{Arc, Barrier, mpsc::sync_channel};
use std::thread;

use nando_operator_kernel::RuntimeProjectionV3;

use super::{execute_control, generation};
use crate::{
    TrafficShadowHandoffCountersV3, TrafficShadowHandoffVerdictV3, TrafficShadowRegistryV3,
    TrafficShadowSourceV3, TrafficShadowVerdictV3,
};

#[test]
fn pinned_requests_never_mix_generation_during_swap() {
    const REQUESTS: usize = 100;
    let registry = Arc::new(TrafficShadowRegistryV3::new(generation(1, 711)));
    let barrier = Arc::new(Barrier::new(REQUESTS + 1));
    let mut handles = Vec::with_capacity(REQUESTS);
    for _ in 0..REQUESTS {
        let registry = Arc::clone(&registry);
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let pinned = registry.pin().expect("pin generation one");
            barrier.wait();
            execute_control(
                pinned,
                RuntimeProjectionV3::Responses,
                false,
                TrafficShadowSourceV3::Replay,
            )
        }));
    }
    barrier.wait();
    let retired = registry.swap(generation(2, 712)).expect("monotonic swap");
    assert_eq!(retired.sequence(), 1);

    for handle in handles {
        let receipt = handle.join().expect("shadow request");
        assert_eq!(receipt.generation_sequence(), 1);
        assert_eq!(
            receipt.generation_root_sha256(),
            retired.generation_root_sha256()
        );
        assert_eq!(receipt.verdict(), TrafficShadowVerdictV3::CompleteShadow);
    }
    let current = registry.pin().expect("pin generation two");
    let receipt = execute_control(
        current,
        RuntimeProjectionV3::Responses,
        false,
        TrafficShadowSourceV3::Replay,
    );
    assert_eq!(receipt.generation_sequence(), 2);
    assert_ne!(
        receipt.generation_root_sha256(),
        retired.generation_root_sha256()
    );
}

#[test]
fn caller_owned_try_send_accounts_every_overload_outcome() {
    let (sender, receiver) = sync_channel(1);
    let mut counters = TrafficShadowHandoffCountersV3::default();
    assert_eq!(
        counters.observe(sender.try_send(1_u8)),
        TrafficShadowHandoffVerdictV3::Enqueued
    );
    assert_eq!(
        counters.observe(sender.try_send(2_u8)),
        TrafficShadowHandoffVerdictV3::CensoredQueueFull
    );
    drop(receiver);
    assert_eq!(
        counters.observe(sender.try_send(3_u8)),
        TrafficShadowHandoffVerdictV3::CensoredDisconnected
    );
    assert_eq!(counters.attempted(), 3);
    assert_eq!(counters.accounted(), 3);
    assert_eq!(counters.enqueued(), 1);
    assert_eq!(counters.censored_queue_full(), 1);
    assert_eq!(counters.censored_disconnected(), 1);
    assert!(!counters.execution_authority());
}

#[test]
fn non_monotonic_generation_swap_is_rejected() {
    let registry = TrafficShadowRegistryV3::new(generation(5, 713));
    assert!(registry.swap(generation(5, 714)).is_err());
    assert!(registry.swap(generation(4, 715)).is_err());
    assert_eq!(registry.pin().expect("current generation").sequence(), 5);
    assert!(!registry.execution_authority());
}
