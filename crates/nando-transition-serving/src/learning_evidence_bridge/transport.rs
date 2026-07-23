use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;

use axum::body::Bytes;
use nando_operator_learning::{
    LEARNING_EVIDENCE_ENVELOPE_MAX_BYTES_V1, LearningEvidenceEnvelopeV1, LearningRequestStructureV1,
};
use nando_operator_proof::independent_verifier_v3::{
    F6_MAX_RAW_REQUEST_BYTES_V3, F6_MAX_REQUEST_TEXT_BYTES_V3, derive_request_text_v3,
};

use crate::generation_shadow::{
    GenerationShadowRequestV3, GenerationShadowRuntimeV3, GenerationShadowSubmitVerdictV3,
};
use crate::session_stream::RequestLearningIndex;

use super::{LearningEvidenceBridgeInnerV1, LearningEvidenceIngressV1, record_failure};

pub(super) const ACK_STRUCTURAL_RAW_ENQUEUED: u8 = 1;
pub(super) const ACK_STRUCTURAL_ONLY: u8 = 2;
const ACK_INVALID: u8 = 3;
const ACK_STRUCTURAL_RAW_CENSORED: u8 = 4;
const IO_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_RETRY_ATTEMPTS: usize = 20;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(50);

pub(super) struct DecodedLearningEvidenceV1 {
    pub(super) structure: LearningRequestStructureV1,
    pub(super) generation_request: Option<GenerationShadowRequestV3>,
    pub(super) raw_was_present: bool,
    pub(super) raw_payload_bytes: u64,
}

pub(super) fn start_producer(
    inner: Arc<LearningEvidenceBridgeInnerV1>,
    receiver: Receiver<LearningEvidenceIngressV1>,
) -> Result<(), String> {
    thread::Builder::new()
        .name("nando-learning-evidence-producer".to_owned())
        .spawn(move || run_producer(inner, receiver))
        .map(|_| ())
        .map_err(|error| format!("learning_evidence_bridge_producer_spawn:{error}"))
}

fn run_producer(
    inner: Arc<LearningEvidenceBridgeInnerV1>,
    receiver: Receiver<LearningEvidenceIngressV1>,
) {
    while let Ok(ingress) = receiver.recv() {
        let payload_len = u64::try_from(ingress.provider_payload.len()).unwrap_or(u64::MAX);
        let provider_bound_turn = ingress.structure.provider_bound_turn_identity();
        let session_bound = !ingress.structure.session_identity_sha256s().is_empty();
        let capability_bound = !ingress.structure.capability_atom_ids().is_empty();
        let raw_eligible = ingress.provider_payload.len() <= F6_MAX_RAW_REQUEST_BYTES_V3;
        if raw_eligible {
            inner.producer.raw_eligible.fetch_add(1, Ordering::Relaxed);
        } else {
            inner
                .producer
                .raw_budget_censored
                .fetch_add(1, Ordering::Relaxed);
        }
        let result = LearningEvidenceEnvelopeV1::new(
            ingress.capture_receipt,
            ingress.structure,
            &ingress.provider_payload,
        )
        .and_then(|envelope| envelope.canonical_cbor())
        .map_err(|error| format!("learning_evidence_bridge_envelope:{error:?}"))
        .and_then(|frame| send_frame(&inner.socket_path, &frame));
        match result {
            Ok(ACK_STRUCTURAL_RAW_ENQUEUED) => {
                inner.producer.accepted.fetch_add(1, Ordering::Relaxed);
                record_structure_accept(
                    &inner.producer,
                    provider_bound_turn,
                    session_bound,
                    capability_bound,
                );
                inner.producer.raw_accepted.fetch_add(1, Ordering::Relaxed);
                inner
                    .producer
                    .payload_bytes
                    .fetch_add(payload_len, Ordering::Relaxed);
            }
            Ok(ACK_STRUCTURAL_ONLY) => {
                inner.producer.accepted.fetch_add(1, Ordering::Relaxed);
                record_structure_accept(
                    &inner.producer,
                    provider_bound_turn,
                    session_bound,
                    capability_bound,
                );
            }
            Ok(ACK_STRUCTURAL_RAW_CENSORED) => {
                inner.producer.accepted.fetch_add(1, Ordering::Relaxed);
                record_structure_accept(
                    &inner.producer,
                    provider_bound_turn,
                    session_bound,
                    capability_bound,
                );
                inner.producer.raw_censored.fetch_add(1, Ordering::Relaxed);
            }
            Ok(ACK_INVALID) => {
                inner.producer.invalid.fetch_add(1, Ordering::Relaxed);
                inner.producer.censored.fetch_add(1, Ordering::Relaxed);
            }
            Ok(_) => record_failure(&inner.producer, "learning_evidence_bridge_ack_invalid"),
            Err(error) => {
                inner.producer.censored.fetch_add(1, Ordering::Relaxed);
                record_failure(&inner.producer, &error);
            }
        }
    }
}

fn record_structure_accept(
    counters: &super::EndpointCountersV1,
    provider_bound_turn: bool,
    session_bound: bool,
    capability_bound: bool,
) {
    if provider_bound_turn {
        counters
            .provider_bound_turns
            .fetch_add(1, Ordering::Relaxed);
    }
    if session_bound {
        counters
            .session_bound_requests
            .fetch_add(1, Ordering::Relaxed);
    }
    if capability_bound {
        counters
            .capability_bound_requests
            .fetch_add(1, Ordering::Relaxed);
    }
}

fn send_frame(socket_path: &Path, frame: &[u8]) -> Result<u8, String> {
    let frame_len = u32::try_from(frame.len())
        .map_err(|_| "learning_evidence_bridge_frame_too_large".to_owned())?;
    let mut stream = connect_with_retry(socket_path)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|error| format!("learning_evidence_bridge_timeout:{error}"))?;
    stream
        .write_all(&frame_len.to_be_bytes())
        .and_then(|()| stream.write_all(frame))
        .map_err(|error| format!("learning_evidence_bridge_write:{error}"))?;
    let mut ack = [0_u8; 1];
    stream
        .read_exact(&mut ack)
        .map_err(|error| format!("learning_evidence_bridge_ack:{error}"))?;
    Ok(ack[0])
}

fn connect_with_retry(socket_path: &Path) -> Result<UnixStream, String> {
    let mut last_error = None;
    for attempt in 0..CONNECT_RETRY_ATTEMPTS {
        match UnixStream::connect(socket_path) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
        if attempt + 1 < CONNECT_RETRY_ATTEMPTS {
            thread::sleep(CONNECT_RETRY_DELAY);
        }
    }
    Err(format!(
        "learning_evidence_bridge_connect:{}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown".to_owned())
    ))
}

pub(super) fn start_consumer(
    inner: Arc<LearningEvidenceBridgeInnerV1>,
    generation_shadow: Arc<GenerationShadowRuntimeV3>,
    request_learning: Arc<RequestLearningIndex>,
) -> Result<(), String> {
    if inner
        .consumer_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(());
    }
    let listener = bind_private_listener(&inner.socket_path)?;
    thread::Builder::new()
        .name("nando-learning-evidence-consumer".to_owned())
        .spawn(move || run_consumer(inner, generation_shadow, request_learning, listener))
        .map(|_| ())
        .map_err(|error| format!("learning_evidence_bridge_consumer_spawn:{error}"))
}

fn run_consumer(
    inner: Arc<LearningEvidenceBridgeInnerV1>,
    generation_shadow: Arc<GenerationShadowRuntimeV3>,
    request_learning: Arc<RequestLearningIndex>,
    listener: UnixListener,
) {
    for connection in listener.incoming() {
        let mut stream = match connection {
            Ok(stream) => stream,
            Err(error) => {
                record_failure(
                    &inner.consumer,
                    &format!("learning_evidence_bridge_accept:{error}"),
                );
                continue;
            }
        };
        inner.consumer.received.fetch_add(1, Ordering::Relaxed);
        let ack = match read_frame(&mut stream).and_then(|frame| decode_learning_evidence(&frame)) {
            Ok(decoded) => {
                if let Err(error) = request_learning.observe_structure(&decoded.structure) {
                    inner.consumer.invalid.fetch_add(1, Ordering::Relaxed);
                    inner.consumer.censored.fetch_add(1, Ordering::Relaxed);
                    record_failure(&inner.consumer, error);
                    ACK_INVALID
                } else {
                    inner.consumer.accepted.fetch_add(1, Ordering::Relaxed);
                    record_structure_accept(
                        &inner.consumer,
                        decoded.structure.provider_bound_turn_identity(),
                        !decoded.structure.session_identity_sha256s().is_empty(),
                        !decoded.structure.capability_atom_ids().is_empty(),
                    );
                    if decoded.raw_was_present {
                        inner.consumer.raw_eligible.fetch_add(1, Ordering::Relaxed);
                    }
                    match decoded.generation_request {
                        Some(request) => {
                            let verdict = generation_shadow.try_submit(request);
                            if verdict == GenerationShadowSubmitVerdictV3::Enqueued {
                                inner.consumer.raw_accepted.fetch_add(1, Ordering::Relaxed);
                                inner
                                    .consumer
                                    .payload_bytes
                                    .fetch_add(decoded.raw_payload_bytes, Ordering::Relaxed);
                                ACK_STRUCTURAL_RAW_ENQUEUED
                            } else {
                                inner.consumer.raw_censored.fetch_add(1, Ordering::Relaxed);
                                ACK_STRUCTURAL_RAW_CENSORED
                            }
                        }
                        None => {
                            if decoded.raw_was_present {
                                inner.consumer.raw_censored.fetch_add(1, Ordering::Relaxed);
                                ACK_STRUCTURAL_RAW_CENSORED
                            } else {
                                inner
                                    .consumer
                                    .raw_budget_censored
                                    .fetch_add(1, Ordering::Relaxed);
                                ACK_STRUCTURAL_ONLY
                            }
                        }
                    }
                }
            }
            Err(error) => {
                inner.consumer.invalid.fetch_add(1, Ordering::Relaxed);
                inner.consumer.censored.fetch_add(1, Ordering::Relaxed);
                record_failure(&inner.consumer, &error);
                ACK_INVALID
            }
        };
        if let Err(error) = stream.write_all(&[ack]) {
            record_failure(
                &inner.consumer,
                &format!("learning_evidence_bridge_ack_write:{error}"),
            );
        }
    }
}

pub(super) fn read_frame(stream: &mut UnixStream) -> Result<Vec<u8>, String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|error| format!("learning_evidence_bridge_timeout:{error}"))?;
    let mut length = [0_u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|error| format!("learning_evidence_bridge_length:{error}"))?;
    let length = usize::try_from(u32::from_be_bytes(length))
        .map_err(|_| "learning_evidence_bridge_length_invalid".to_owned())?;
    if length == 0 || length > LEARNING_EVIDENCE_ENVELOPE_MAX_BYTES_V1 {
        return Err("learning_evidence_bridge_frame_budget".to_owned());
    }
    let mut frame = vec![0_u8; length];
    stream
        .read_exact(&mut frame)
        .map_err(|error| format!("learning_evidence_bridge_frame:{error}"))?;
    Ok(frame)
}

pub(super) fn decode_learning_evidence(frame: &[u8]) -> Result<DecodedLearningEvidenceV1, String> {
    let envelope = LearningEvidenceEnvelopeV1::from_canonical_cbor(frame)
        .map_err(|error| format!("learning_evidence_bridge_decode:{error:?}"))?;
    let (capture_receipt, structure, raw_provider_payload) = envelope.into_parts();
    let raw_payload_bytes = raw_provider_payload.as_ref().map_or(0, |payload| {
        u64::try_from(payload.len()).unwrap_or(u64::MAX)
    });
    let raw_was_present = raw_provider_payload.is_some();
    let generation_request = raw_provider_payload.and_then(|provider_payload| {
        let payload: serde_json::Value = serde_json::from_slice(&provider_payload).ok()?;
        let request_text = derive_request_text_v3(
            &payload,
            capture_receipt.projection(),
            F6_MAX_REQUEST_TEXT_BYTES_V3,
        )
        .ok()?;
        GenerationShadowRequestV3::from_provider_capture(
            capture_receipt.clone(),
            request_text,
            Bytes::from(provider_payload),
        )
        .ok()
    });
    Ok(DecodedLearningEvidenceV1 {
        structure,
        generation_request,
        raw_was_present,
        raw_payload_bytes,
    })
}

fn bind_private_listener(path: &Path) -> Result<UnixListener, String> {
    let parent = path
        .parent()
        .ok_or_else(|| "learning_evidence_bridge_socket_parent_missing".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("learning_evidence_bridge_mkdir:{error}"))?;
    #[cfg(unix)]
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("learning_evidence_bridge_dir_mode:{error}"))?;
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err("learning_evidence_bridge_consumer_already_live".to_owned());
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("learning_evidence_bridge_socket_metadata:{error}"))?;
        if !metadata.file_type().is_socket() {
            return Err("learning_evidence_bridge_path_not_socket".to_owned());
        }
        fs::remove_file(path)
            .map_err(|error| format!("learning_evidence_bridge_stale_socket:{error}"))?;
    }
    let listener = UnixListener::bind(path)
        .map_err(|error| format!("learning_evidence_bridge_bind:{error}"))?;
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("learning_evidence_bridge_socket_mode:{error}"))?;
    Ok(listener)
}
