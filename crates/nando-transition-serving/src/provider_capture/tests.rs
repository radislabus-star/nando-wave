use std::{fs, sync::Arc, thread, time::Duration};

use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_persistence::ProviderCaptureStoreV3;

use super::types::ProviderCaptureSubmitV3;
use super::{ProviderCaptureConfigV3, ProviderCaptureIngressV3, ProviderCaptureRuntimeV3};

#[test]
fn hash_only_capture_is_durable_and_duplicate_requests_are_censored() {
    let directory = std::env::temp_dir().join(format!(
        "nando-f8a-serving-{}-{}",
        std::process::id(),
        Sha256CommitmentV3::digest_bytes(b"runtime").to_hex()
    ));
    let _ = fs::remove_dir_all(&directory);
    let runtime = Arc::new(
        ProviderCaptureRuntimeV3::new(ProviderCaptureConfigV3 {
            enabled: true,
            store_path: directory.clone(),
            queue_capacity: 4,
        })
        .expect("runtime"),
    );
    runtime.start_after_http_bind();
    wait_until(|| runtime.status().phase == "ready_hash_only");

    let ingress = ProviderCaptureIngressV3 {
        lineage_root_sha256: root("lineage"),
        request_root_sha256: root("private-provider-payload"),
        projection: RuntimeProjectionV3::Responses,
        streaming: true,
        observed_at_unix_ms: 1_750_000_000_000,
    };
    let first = runtime.try_capture(ingress);
    let receipt = match first {
        ProviderCaptureSubmitV3::Enqueued(receipt) => receipt,
        ProviderCaptureSubmitV3::Censored(reason) => panic!("unexpected censor: {reason:?}"),
    };
    assert_eq!(receipt.capture_sequence(), 1);
    wait_until(|| runtime.status().captured == 1);

    assert!(matches!(
        runtime.try_capture(ingress),
        ProviderCaptureSubmitV3::Enqueued(_)
    ));
    wait_until(|| runtime.status().duplicates == 1);
    let status = runtime.status();
    assert_eq!(status.captured, 1);
    assert_eq!(status.raw_payloads_persisted, 0);
    assert_eq!(status.semantic_updates_from_censored, 0);
    assert!(status.accounting_identity_holds);
    assert!(!status.execution_authority);

    let store = ProviderCaptureStoreV3::open(&directory).expect("store");
    let restored = store.restore().expect("restore");
    assert_eq!(restored.index().expect("index").records().len(), 1);
    for entry in fs::read_dir(&directory).expect("entries") {
        let path = entry.expect("entry").path();
        if path.is_file() {
            let bytes = fs::read(path).expect("bytes");
            assert!(!String::from_utf8_lossy(&bytes).contains("private-provider-payload"));
        }
    }
    drop(runtime);
    let _ = fs::remove_dir_all(directory);
}

#[test]
fn invalid_provider_input_is_terminally_censored_without_starting_capture() {
    let runtime = ProviderCaptureRuntimeV3::new(ProviderCaptureConfigV3 {
        enabled: false,
        store_path: std::env::temp_dir().join("unused-f8a-invalid-input"),
        queue_capacity: 1,
    })
    .expect("runtime");

    runtime.observe_invalid_provenance();
    let status = runtime.status();
    assert_eq!(status.submitted, 1);
    assert_eq!(status.censored, 1);
    assert_eq!(status.ingress_censored, 1);
    assert_eq!(status.semantic_updates_from_censored, 0);
    assert!(status.accounting_identity_holds);
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    for _ in 0..200 {
        if predicate() {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("condition did not become true");
}

fn root(label: &str) -> Sha256CommitmentV3 {
    Sha256CommitmentV3::digest_bytes(label.as_bytes())
}
