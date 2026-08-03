use std::fs;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3, sha256_bytes};
use nando_operator_learning::{
    LearningEvidenceEnvelopeV1, LearningRequestStructureInputV1, LearningRequestStructureV1,
    ProviderRequestCaptureInputV3, seal_provider_request_capture_v3,
};
use nando_operator_proof::independent_verifier_v3::F6_MAX_RAW_REQUEST_BYTES_V3;

use super::transport::{ACK_STRUCTURAL_RAW_ENQUEUED, read_frame};
use super::*;

fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nando-learning-evidence-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

fn receipt(payload: &[u8]) -> ProviderRequestCaptureReceiptV3 {
    seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: 17,
        capture_epoch_root: Sha256CommitmentV3::digest_bytes(b"epoch"),
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"lineage"),
        request_root_sha256: Sha256CommitmentV3::digest_bytes(payload),
        projection: RuntimeProjectionV3::Responses,
        streaming: true,
        observed_at_unix_ms: 1_750_000_000_000,
    })
    .expect("capture receipt")
}

fn structure(payload: &[u8]) -> LearningRequestStructureV1 {
    LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
        client_intent_id_sha256: sha256_bytes(b"turn-a"),
        session_identity_sha256s: vec![sha256_bytes(b"session-a")],
        request_phase_atom_ids: vec![3, 1, 3],
        pre_action_context_atom_ids: vec![9, 7],
        capability_atom_ids: vec![13, 11],
        provider_bound_turn_identity: true,
        estimated_input_tokens: 17,
        provider_payload_bytes: u64::try_from(payload.len()).unwrap_or(u64::MAX),
    })
    .expect("request structure")
}

fn shadow(root: &std::path::Path, enabled: bool) -> Arc<GenerationShadowRuntimeV3> {
    Arc::new(
        GenerationShadowRuntimeV3::new(crate::generation_shadow::GenerationShadowConfigV3 {
            enabled,
            store_path: root.join("generation"),
            capture_index_path: root.join("capture.cbor"),
            provider_capture_store_path: root.join("provider"),
            receipt_store_path: root.join("receipts"),
            queue_capacity: 1,
            poll_interval: Duration::from_millis(100),
        })
        .expect("shadow"),
    )
}

fn certification(root: &std::path::Path) -> Arc<CertificationAuthorityConfigV1> {
    Arc::new(CertificationAuthorityConfigV1 {
        root: root.join("certification"),
        cleanup_receipts_path: root.join("cleanup"),
        anchor_path: root.join("anchor.json"),
        authority_socket_path: root.join("authority.sock"),
        authority_public_key_path: root.join("authority.pub"),
        cleanup_public_key_path: root.join("cleanup.pub"),
        response_registry_path: root.join("registry.json"),
        runtime_revocations_path: root.join("revocations.json"),
    })
}

#[test]
fn producer_uses_bounded_queue_and_receives_cold_ack() {
    let root = root();
    fs::create_dir_all(&root).expect("root");
    let socket = root.join("bridge.sock");
    let server_socket = socket.clone();
    let server = thread::spawn(move || {
        thread::sleep(Duration::from_millis(100));
        let listener = UnixListener::bind(&server_socket).expect("listener");
        let (mut stream, _) = listener.accept().expect("accept");
        let frame = read_frame(&mut stream).expect("frame");
        let envelope = LearningEvidenceEnvelopeV1::from_canonical_cbor(&frame).expect("envelope");
        assert_eq!(
            envelope.raw_provider_payload(),
            Some(br#"{"input":"continue"}"#.as_slice())
        );
        assert_eq!(envelope.structure().capability_atom_ids(), &[11, 13]);
        std::io::Write::write_all(&mut stream, &[ACK_STRUCTURAL_RAW_ENQUEUED]).expect("ack");
    });
    let runtime = LearningEvidenceBridgeRuntimeV1::new(socket, true, false, 2).expect("runtime");
    let payload = Bytes::from_static(br#"{"input":"continue"}"#);
    runtime
        .start(
            shadow(&root, false),
            Arc::new(RequestLearningIndex::default()),
            false,
            certification(&root),
        )
        .expect("start");
    runtime
        .submit(receipt(&payload), structure(&payload), payload)
        .expect("submit evidence");
    server.join().expect("server");
    for _ in 0..50 {
        if runtime.status().producer.accepted == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let status = runtime.status();
    assert_eq!(status.producer.submitted, 1);
    assert_eq!(status.producer.enqueued, 1);
    assert_eq!(status.producer.accepted, 1);
    assert_eq!(status.producer.provider_bound_turns, 1);
    assert_eq!(status.producer.session_bound_requests, 1);
    assert_eq!(status.producer.capability_bound_requests, 1);
    assert_eq!(status.producer.raw_eligible, 1);
    assert_eq!(status.producer.raw_accepted, 1);
    assert_eq!(status.producer.failures, 0);
    assert_eq!(status.raw_payloads_persisted, 0);
    assert!(!status.execution_authority);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn cold_decode_derives_request_text_and_rejects_digest_tampering() {
    let payload = br#"{"input":[{"role":"user","content":[{"type":"input_text","text":"continue CellA17"}]}]}"#;
    let envelope = LearningEvidenceEnvelopeV1::new(receipt(payload), structure(payload), payload)
        .expect("evidence envelope");
    let frame = envelope.canonical_cbor().expect("frame");
    let decoded = super::transport::decode_learning_evidence(&frame).expect("learning evidence");
    assert_eq!(
        decoded.raw_payload_bytes,
        u64::try_from(payload.len()).unwrap_or(u64::MAX)
    );
    assert!(decoded.raw_was_present);
    assert_eq!(
        decoded
            .generation_request
            .expect("generation request")
            .request_text(),
        "continue CellA17"
    );

    let mut tampered = frame;
    let last = tampered.len().saturating_sub(1);
    tampered[last] ^= 1;
    assert!(super::transport::decode_learning_evidence(&tampered).is_err());
}

#[test]
fn oversized_payload_crosses_as_structure_without_raw_f8_authority() {
    let payload = vec![b'x'; F6_MAX_RAW_REQUEST_BYTES_V3 + 1];
    let envelope =
        LearningEvidenceEnvelopeV1::new(receipt(&payload), structure(&payload), &payload)
            .expect("structural envelope");
    let decoded = super::transport::decode_learning_evidence(
        &envelope.canonical_cbor().expect("canonical envelope"),
    )
    .expect("structural evidence");

    assert!(!decoded.raw_was_present);
    assert_eq!(decoded.raw_payload_bytes, 0);
    assert!(decoded.generation_request.is_none());
    assert_eq!(
        decoded.structure.provider_payload_bytes(),
        u64::try_from(payload.len()).unwrap_or(u64::MAX)
    );
}

#[test]
fn process_bridge_delivers_every_structure_and_only_bounded_raw_evidence() {
    let root = root();
    fs::create_dir_all(&root).expect("root");
    let socket = root.join("bridge.sock");
    let request_learning = Arc::new(RequestLearningIndex::default());
    let consumer = LearningEvidenceBridgeRuntimeV1::new(socket.clone(), false, true, 2)
        .expect("consumer runtime");
    consumer
        .start(
            shadow(&root, true),
            Arc::clone(&request_learning),
            false,
            certification(&root),
        )
        .expect("consumer start");
    let producer =
        LearningEvidenceBridgeRuntimeV1::new(socket, true, false, 2).expect("producer runtime");
    producer
        .start(
            shadow(&root, false),
            Arc::new(RequestLearningIndex::default()),
            false,
            certification(&root),
        )
        .expect("producer start");

    let small = Bytes::from_static(br#"{"input":"continue"}"#);
    producer
        .submit(receipt(&small), structure(&small), small)
        .expect("small structure");
    let large = Bytes::from(vec![b'x'; F6_MAX_RAW_REQUEST_BYTES_V3 + 1]);
    producer
        .submit(receipt(&large), structure(&large), large)
        .expect("large structure");

    for _ in 0..100 {
        if consumer.status().consumer.accepted == 2 && producer.status().producer.accepted == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let cold = consumer.status().consumer;
    let hot = producer.status().producer;
    assert_eq!(cold.received, 2);
    assert_eq!(cold.accepted, 2);
    assert_eq!(cold.censored, 0);
    assert_eq!(cold.invalid, 0);
    assert_eq!(cold.session_bound_requests, 2);
    assert_eq!(cold.capability_bound_requests, 2);
    assert_eq!(cold.raw_eligible, 1);
    assert_eq!(cold.raw_censored, 1);
    assert_eq!(cold.raw_budget_censored, 1);
    assert_eq!(hot.accepted, 2);
    assert_eq!(hot.censored, 0);
    assert_eq!(hot.raw_eligible, 1);
    assert_eq!(hot.raw_censored, 1);
    assert_eq!(hot.raw_budget_censored, 1);
    let joined = request_learning.lookup(&sha256_bytes(b"session-a"), &sha256_bytes(b"turn-a"));
    assert_eq!(joined.request_phase_atom_ids, vec![1, 3]);
    assert_eq!(joined.capability_atom_ids, vec![11, 13]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn consumer_refuses_to_replace_a_non_socket_path() {
    let root = root();
    fs::create_dir_all(&root).expect("root");
    let socket = root.join("bridge.sock");
    fs::write(&socket, b"do-not-delete").expect("sentinel");
    let runtime =
        LearningEvidenceBridgeRuntimeV1::new(socket.clone(), false, true, 2).expect("runtime");
    assert_eq!(
        runtime
            .start(
                shadow(&root, true),
                Arc::new(RequestLearningIndex::default()),
                false,
                certification(&root),
            )
            .expect_err("regular file must block"),
        "learning_evidence_bridge_path_not_socket"
    );
    assert_eq!(
        fs::read(&socket).expect("sentinel remains"),
        b"do-not-delete"
    );
    let _ = fs::remove_dir_all(root);
}
