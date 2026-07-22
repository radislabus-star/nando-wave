#[path = "../../nando-operator-persistence/tests/f7_support/mod.rs"]
mod f7_support;
#[path = "f7_generation_shadow_v3/performance.rs"]
mod performance;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Bytes;
use f7_support::{FixtureV3, root, support_request_payload};
use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3, sha256_bytes};
use nando_operator_learning::{
    GenerationCaptureCommitmentV3, GenerationCaptureIndexV3, ProviderRequestCaptureInputV3,
    seal_provider_request_capture_v3,
};
use nando_operator_persistence::{
    GenerationCheckpointStoreV3, GenerationShadowReceiptStoreV3, ProviderCaptureStoreV3,
    RestoredGenerationCheckpointV3, decode_generation_checkpoint_v3,
};
use nando_transition_serving::generation_shadow::{
    GenerationShadowConfigV3, GenerationShadowEvaluationVerdictV3, GenerationShadowRequestV3,
    GenerationShadowRuntimeV3, GenerationShadowSubmitVerdictV3,
    evaluate_generation_shadow_request_v3,
};

fn capture_index(checkpoint: &RestoredGenerationCheckpointV3) -> GenerationCaptureIndexV3 {
    GenerationCaptureIndexV3::new(
        checkpoint
            .receipts()
            .iter()
            .enumerate()
            .map(|(index, pair)| {
                let receipt = pair.generation_receipt();
                GenerationCaptureCommitmentV3::new(
                    receipt.capture_sequence(),
                    root(&format!("serving capture {index}")),
                    receipt.lineage_root_sha256().to_owned(),
                    receipt.event_root_sha256().to_owned(),
                    receipt.f6_request_sha256().to_owned(),
                )
                .expect("capture commitment")
            })
            .collect(),
    )
    .expect("capture index")
}

fn write_capture_index(path: &std::path::Path, checkpoint_bytes: &[u8]) {
    let checkpoint = decode_generation_checkpoint_v3(checkpoint_bytes).expect("checkpoint");
    let index = capture_index(&checkpoint);
    fs::write(path, index.canonical_bytes().expect("index bytes")).expect("write index");
}

fn runtime(fixture: &FixtureV3, capture_path: &std::path::Path) -> Arc<GenerationShadowRuntimeV3> {
    Arc::new(
        GenerationShadowRuntimeV3::new(GenerationShadowConfigV3 {
            enabled: true,
            store_path: fixture.directory.clone(),
            capture_index_path: capture_path.to_owned(),
            provider_capture_store_path: fixture.directory.join("provider-capture-v3-f8a"),
            receipt_store_path: fixture.directory.join("generation-shadow-v3-f8b"),
            queue_capacity: 4,
            poll_interval: Duration::from_secs(60),
        })
        .expect("runtime"),
    )
}

#[test]
fn joined_generation_executes_f5_and_independent_f6_without_authority() {
    let mut fixture = FixtureV3::new("f7e-evaluation");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("store")
        .publish(&checkpoint_bytes)
        .expect("publish");
    let capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&capture_path, &checkpoint_bytes);

    let runtime = runtime(&fixture, &capture_path);
    assert!(runtime.reconcile_once().expect("reconcile"));
    let generation = runtime.registry().pin().expect("pin").expect("generation");
    let payload = support_request_payload();
    let payload_bytes = serde_json::to_vec(&payload).expect("payload bytes");
    let request_sha256 = sha256_bytes(&payload_bytes);
    let request = GenerationShadowRequestV3::new(
        root("f7e request row"),
        request_sha256,
        RuntimeProjectionV3::Responses,
        false,
        "continue CellA17".to_owned(),
        Bytes::from(payload_bytes),
    )
    .expect("request");
    let receipt = evaluate_generation_shadow_request_v3(&generation, &request);

    assert_eq!(
        receipt.verdict,
        GenerationShadowEvaluationVerdictV3::Verified
    );
    assert!(receipt.verifier_receipt_sha256.is_some());
    assert!(!receipt.parity_mismatch);
    assert_eq!(receipt.raw_payloads_persisted, 0);
    assert_eq!(receipt.local_accepts, 0);
    assert!(!receipt.execution_authority);
    assert_eq!(
        generation.traffic_generation().generation_root_sha256(),
        fixture.manifest.generation_id_sha256()
    );

    runtime.start_after_http_bind().expect("start worker");
    assert_eq!(
        runtime.try_submit(request),
        GenerationShadowSubmitVerdictV3::Enqueued
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    while runtime.status().evaluated == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let status = runtime.status();
    assert_eq!(status.evaluated, 1);
    assert_eq!(status.verified, 1);
    assert_eq!(status.false_accepts, 0);
    assert_eq!(status.local_accepts, 0);
}

#[test]
fn provider_capture_joins_to_a_durable_generation_owned_shadow_receipt() {
    let mut fixture = FixtureV3::new("f8b-durable-shadow");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("generation store")
        .publish(&checkpoint_bytes)
        .expect("publish generation");
    let generation_capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&generation_capture_path, &checkpoint_bytes);

    let provider_store_path = fixture.directory.join("provider-capture-v3-f8a");
    let provider_store = ProviderCaptureStoreV3::open(&provider_store_path).expect("capture store");
    let lease = provider_store
        .reserve_sequence_lease()
        .expect("capture lease");
    let payload_bytes = serde_json::to_vec(&support_request_payload()).expect("payload bytes");
    let capture = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: lease.first_sequence(),
        capture_epoch_root: lease.epoch_root_sha256(),
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"ordinary-live-lineage"),
        request_root_sha256: Sha256CommitmentV3::digest_bytes(&payload_bytes),
        projection: RuntimeProjectionV3::Responses,
        streaming: false,
        observed_at_unix_ms: 1_750_000_000_000,
    })
    .expect("capture receipt");
    let restored = provider_store.restore().expect("capture restore");
    let next = restored
        .index()
        .expect("capture index")
        .append_batch(std::slice::from_ref(&capture))
        .expect("capture append");
    provider_store
        .publish_index(&next)
        .expect("capture publish");

    let runtime = runtime(&fixture, &generation_capture_path);
    assert!(runtime.reconcile_once().expect("reconcile"));
    runtime.start_after_http_bind().expect("start worker");
    let request = GenerationShadowRequestV3::from_provider_capture(
        capture.clone(),
        "continue CellA17".to_owned(),
        Bytes::from(payload_bytes),
    )
    .expect("request");
    assert_eq!(
        runtime.try_submit(request),
        GenerationShadowSubmitVerdictV3::Enqueued
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    while runtime.status().durable_appends == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    let status = runtime.status();
    assert_eq!(status.durable_appends, 1);
    assert_eq!(status.durable_censored, 0);
    assert_eq!(status.verified, 1);
    assert!(!status.shadow_ledger_sha256.is_empty());

    let generation = runtime.registry().pin().expect("pin").expect("generation");
    let ledger_store = GenerationShadowReceiptStoreV3::open(
        fixture
            .directory
            .join("generation-shadow-v3-f8b")
            .join(fixture.manifest.generation_id_sha256())
            .join(generation.checkpoint().checkpoint_sha256()),
    )
    .expect("ledger store");
    let persisted = ledger_store.restore().expect("ledger restore");
    let ledger = persisted.ledger().expect("ledger");
    assert_eq!(ledger.publish_sequence(), 1);
    assert_eq!(
        ledger.receipts()[0].capture_sequence(),
        capture.capture_sequence()
    );
    assert_eq!(ledger.receipts()[0].semantic_updates(), 1);
    assert!(ledger.receipts()[0].verifier_receipt().is_some());
    let phase = ledger.receipts()[0]
        .phase_control_evidence()
        .expect("runtime-owned phase controls");
    assert_eq!(
        Some(phase.report_sha256()),
        ledger.receipts()[0].traffic_phase_report_sha256()
    );
    assert_eq!(phase.raw_payloads_persisted(), 0);
    assert!(!phase.execution_authority());
    assert_eq!(ledger.raw_payloads_persisted(), 0);
    assert!(!ledger.execution_authority());
}

#[test]
fn duplicate_capture_root_is_censored_without_waiting_for_the_join_deadline() {
    let mut fixture = FixtureV3::new("f8b-duplicate-capture");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("generation store")
        .publish(&checkpoint_bytes)
        .expect("publish generation");
    let generation_capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&generation_capture_path, &checkpoint_bytes);

    let provider_store_path = fixture.directory.join("provider-capture-v3-f8a");
    let provider_store = ProviderCaptureStoreV3::open(&provider_store_path).expect("capture store");
    let lease = provider_store
        .reserve_sequence_lease()
        .expect("capture lease");
    let payload_bytes = serde_json::to_vec(&support_request_payload()).expect("payload bytes");
    let request_root = Sha256CommitmentV3::digest_bytes(&payload_bytes);
    let first = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: lease.first_sequence(),
        capture_epoch_root: lease.epoch_root_sha256(),
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"duplicate-first-lineage"),
        request_root_sha256: request_root,
        projection: RuntimeProjectionV3::Responses,
        streaming: false,
        observed_at_unix_ms: 1_750_000_000_000,
    })
    .expect("first capture");
    let restored = provider_store.restore().expect("capture restore");
    let next = restored
        .index()
        .expect("capture index")
        .append_batch(std::slice::from_ref(&first))
        .expect("capture append");
    provider_store
        .publish_index(&next)
        .expect("capture publish");

    let duplicate = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: lease.first_sequence() + 1,
        capture_epoch_root: lease.epoch_root_sha256(),
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"duplicate-second-lineage"),
        request_root_sha256: request_root,
        projection: RuntimeProjectionV3::Responses,
        streaming: false,
        observed_at_unix_ms: 1_750_000_000_001,
    })
    .expect("duplicate capture");
    let runtime = runtime(&fixture, &generation_capture_path);
    assert!(runtime.reconcile_once().expect("reconcile"));
    runtime.start_after_http_bind().expect("start worker");
    let request = GenerationShadowRequestV3::from_provider_capture(
        duplicate,
        "continue CellA17".to_owned(),
        Bytes::from(payload_bytes),
    )
    .expect("request");
    let started = Instant::now();
    assert_eq!(
        runtime.try_submit(request),
        GenerationShadowSubmitVerdictV3::Enqueued
    );
    let deadline = started + Duration::from_secs(2);
    while runtime.status().durable_censored == 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(runtime.status().durable_censored, 1);
    assert!(started.elapsed() < Duration::from_millis(250));
}

#[test]
fn capture_mismatch_keeps_registry_empty_and_start_is_nonblocking() {
    let mut fixture = FixtureV3::new("f7e-capture-block");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("store")
        .publish(&checkpoint_bytes)
        .expect("publish");
    let capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    fs::write(
        &capture_path,
        GenerationCaptureIndexV3::new(Vec::new())
            .expect("empty index")
            .canonical_bytes()
            .expect("index bytes"),
    )
    .expect("write index");
    let runtime = runtime(&fixture, &capture_path);

    assert!(runtime.reconcile_once().is_err());
    assert!(runtime.registry().pin().expect("pin").is_none());
    assert_eq!(runtime.status().phase, "blocked");
    let started = Instant::now();
    runtime.start_after_http_bind().expect("start");
    assert!(started.elapsed() < Duration::from_millis(100));
    let payload_bytes = serde_json::to_vec(&support_request_payload()).expect("payload bytes");
    let request = GenerationShadowRequestV3::new(
        root("blocked request row"),
        sha256_bytes(&payload_bytes),
        RuntimeProjectionV3::Responses,
        false,
        "continue CellA17".to_owned(),
        Bytes::from(payload_bytes),
    )
    .expect("blocked request");
    assert_eq!(
        runtime.try_submit(request),
        GenerationShadowSubmitVerdictV3::CensoredNoGeneration
    );
    assert!(!runtime.execution_authority());
}

#[test]
fn removing_the_store_clears_the_shadow_registry() {
    let mut fixture = FixtureV3::new("f7e-clear");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("store")
        .publish(&checkpoint_bytes)
        .expect("publish");
    let capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&capture_path, &checkpoint_bytes);
    let runtime = runtime(&fixture, &capture_path);
    runtime.reconcile_once().expect("load");
    assert!(runtime.registry().pin().expect("pin").is_some());

    for slot in [
        nando_operator_persistence::GENERATION_STORE_SLOT_A_FILE_V3,
        nando_operator_persistence::GENERATION_STORE_SLOT_B_FILE_V3,
    ] {
        let _ = fs::remove_file(fixture.directory.join(slot));
    }
    assert!(!runtime.reconcile_once().expect("clear"));
    assert!(runtime.registry().pin().expect("pin").is_none());
    let status = runtime.status();
    assert_eq!(status.phase, "empty");
    assert!(status.generation_id_sha256.is_empty());
}

#[test]
fn tampered_capture_index_revokes_the_loaded_shadow_snapshot() {
    let mut fixture = FixtureV3::new("f7e-capture-revoke");
    fixture.append_support();
    fixture.freeze_and_append_future();
    let checkpoint_bytes = fixture.checkpoint(1);
    GenerationCheckpointStoreV3::open(&fixture.directory)
        .expect("store")
        .publish(&checkpoint_bytes)
        .expect("publish");
    let capture_path = fixture.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&capture_path, &checkpoint_bytes);
    let runtime = runtime(&fixture, &capture_path);
    runtime.reconcile_once().expect("load");
    assert!(runtime.registry().pin().expect("pin").is_some());

    fs::write(&capture_path, b"tampered capture index").expect("tamper capture index");
    assert!(runtime.reconcile_once().is_err());
    assert!(runtime.registry().pin().expect("pin").is_none());
    assert_eq!(runtime.status().phase, "blocked");
}

#[test]
fn generation_swap_preserves_old_pin_and_installs_exact_child() {
    let mut first = FixtureV3::new("f7e-swap-one");
    first.append_support();
    first.freeze_and_append_future();
    let first_bytes = first.checkpoint(1);
    let store = GenerationCheckpointStoreV3::open(&first.directory).expect("store");
    store.publish(&first_bytes).expect("publish first");
    let capture_path = first.directory.join("generation-capture-index-v3.cbor");
    write_capture_index(&capture_path, &first_bytes);
    let runtime = runtime(&first, &capture_path);
    runtime.reconcile_once().expect("load first");
    let old = runtime
        .registry()
        .pin()
        .expect("old pin")
        .expect("old generation");

    let mut second = FixtureV3::new_generation(
        "f7e-swap-two",
        2,
        Some(first.manifest.generation_id_sha256().to_owned()),
        "actor-v2",
    );
    second.append_support();
    second.freeze_and_append_future();
    let second_bytes = second.checkpoint(2);
    store.publish(&second_bytes).expect("publish child");
    write_capture_index(&capture_path, &second_bytes);
    assert!(runtime.reconcile_once().expect("load child"));
    let current = runtime
        .registry()
        .pin()
        .expect("current pin")
        .expect("current generation");

    assert_eq!(old.checkpoint().generation().manifest().sequence(), 1);
    assert_eq!(current.checkpoint().generation().manifest().sequence(), 2);
    assert_eq!(
        current
            .checkpoint()
            .generation()
            .manifest()
            .parent_generation_id_sha256(),
        Some(
            old.checkpoint()
                .generation()
                .manifest()
                .generation_id_sha256()
        )
    );
    assert!(!old.execution_authority());
    assert!(!current.execution_authority());
}
