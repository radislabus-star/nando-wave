#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::time::{SystemTime, UNIX_EPOCH};

    use nando_client_evidence::{
        ClientRouteIdentityV1, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES, NandoRouteReceiptIndex,
        NandoRouteReceiptLedger, NandoRouteReceiptV1, evidence_client_intent_id_sha256,
        evidence_session_id_sha256, route_receipt_genesis_root,
        sha256_bytes as client_sha256_bytes,
    };
    use nando_operator_kernel::{
        AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame,
        ResponseValueSelector, sha256_bytes,
    };
    use nando_operator_learning::{RuntimeParityCase, SOURCE_NEUTRAL_EXTRACTOR_VERSION};
    use serde_json::json;

    use super::{
        HttpEndpoint, LocalEvidenceOutbox, OutboxSink,
        RemoteEvidenceFrameValidationBlockerV1, RouteBindingMetrics, TRANSPORT_CENSOR_PREFIX,
        TransportCensorLedger, VerifiedRelationFrameSink, decode_chunked_body,
        parse_http_response, retry_backoff, valid_root,
    };

    fn hash(value: &str) -> String {
        sha256_bytes(value.as_bytes())
    }

    fn completed_frame(label: &str) -> RelationFrame {
        let value_root = hash(&format!("value:{label}"));
        RelationFrame {
            schema: RELATION_FRAME_SCHEMA.to_owned(),
            frame_id_sha256: hash(&format!("frame:{label}")),
            event_id_sha256: hash(&format!("event:{label}")),
            client_intent_id_sha256: hash(&format!("intent:{label}")),
            session_id_sha256: hash(&format!("session:{label}")),
            observed_at_unix_nanos: 1_700_000_000_000_000_000,
            estimated_input_tokens: 100,
            extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
            verifier_label: Some(true),
            atoms: vec![
                RelationAtom::CompletionState {
                    value: "completed".to_owned(),
                },
                RelationAtom::TypedSlot {
                    slot_id: 7,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Observation,
                    value_sha256: value_root.clone(),
                },
                RelationAtom::UniqueSlot { slot_id: 7 },
                RelationAtom::ObservationSelector {
                    slot_id: 7,
                    selector: ResponseValueSelector::JsonField {
                        field: "opaque".to_owned(),
                        value_type: AtomValueType::Integer,
                    },
                },
                RelationAtom::TypedSlot {
                    slot_id: 11,
                    value_type: AtomValueType::Integer,
                    source: AtomSource::Action,
                    value_sha256: value_root,
                },
                RelationAtom::SlotEquality {
                    left_slot: 7,
                    right_slot: 11,
                },
                RelationAtom::ActionFunction {
                    value: "transport_a".to_owned(),
                },
                RelationAtom::ActionRoleArgument {
                    name: "value".to_owned(),
                    slot_id: 11,
                    value_type: Some(AtomValueType::Integer),
                },
            ],
            evidence_ref_sha256: hash(&format!("evidence:{label}")),
        }
    }

    fn temporary_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "nando-evidence-agent-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn route_receipt_for_frame(frame: &RelationFrame) -> NandoRouteReceiptV1 {
        NandoRouteReceiptV1::seal(
            1,
            route_receipt_genesis_root(),
            &ClientRouteIdentityV1 {
                turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
                session_id_sha256: frame.session_id_sha256.clone(),
            },
            client_sha256_bytes(b"request"),
            418,
            frame.observed_at_unix_nanos.saturating_sub(2),
            frame.observed_at_unix_nanos.saturating_sub(1),
        )
        .expect("route receipt")
    }

    fn runtime_parity(frame: &RelationFrame, expected_response: &str) -> RuntimeParityCase {
        RuntimeParityCase {
            evidence_ref_sha256: frame.frame_id_sha256.clone(),
            capture_receipt: None,
            request_text: "Return opaque".to_owned(),
            provider_payload: json!({
                "input": [{
                    "type": "function_call_output",
                    "output": "{\"opaque\":7}"
                }]
            }),
            expected_response: expected_response.to_owned(),
        }
    }

    #[test]
    fn parses_lan_origin_without_response_api_prefix() {
        let endpoint = HttpEndpoint::parse("http://192.168.3.94:8787/").expect("endpoint");
        assert_eq!(endpoint.socket_host, "192.168.3.94");
        assert_eq!(endpoint.port, 8787);
        assert_eq!(endpoint.path, "/_nando/evidence/v1/batches");
        assert!(HttpEndpoint::parse("https://192.168.3.94:8787").is_err());
        assert!(HttpEndpoint::parse("http://192.168.3.94:8787/v1").is_err());
    }

    #[test]
    fn parses_content_length_and_chunked_json_responses() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"ok\":true}";
        let (status, body) = parse_http_response(response).expect("response");
        assert_eq!(status, 200);
        assert_eq!(body, b"{\"ok\":true}");

        let encoded = b"4\r\n{\"ok\r\n7\r\n\":true}\r\n0\r\n\r\n";
        assert_eq!(
            decode_chunked_body(encoded).expect("chunked"),
            b"{\"ok\":true}"
        );
    }

    #[test]
    fn retry_backoff_is_bounded() {
        let base = Duration::from_millis(250);
        assert_eq!(retry_backoff(base, 1), base);
        assert_eq!(retry_backoff(base, 4), Duration::from_secs(2));
        assert_eq!(retry_backoff(base, 100), Duration::from_secs(30));
    }

    #[test]
    fn acknowledged_outbox_compacts_and_restarts_empty() {
        let root = temporary_root("compact");
        let frame = completed_frame("compact");
        let mut outbox = LocalEvidenceOutbox::open(&root).expect("outbox");
        outbox
            .append(frame.clone(), route_receipt_for_frame(&frame), None)
            .expect("append");
        assert_eq!(outbox.frames.len(), 1);
        outbox.compact_all().expect("compact");
        assert!(outbox.frames.is_empty());
        assert_eq!(outbox.payload_bytes, 0);
        drop(outbox);

        let restored = LocalEvidenceOutbox::open(&root).expect("restore");
        assert!(restored.frames.is_empty());
        assert_eq!(restored.payload_bytes, 0);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn outbox_allows_only_monotonic_runtime_parity_enrichment() {
        let root = temporary_root("parity-enrichment");
        let frame = completed_frame("parity-enrichment");
        let route_receipt = route_receipt_for_frame(&frame);
        let mut outbox = LocalEvidenceOutbox::open(&root).expect("outbox");
        outbox
            .append(frame.clone(), route_receipt.clone(), None)
            .expect("legacy append");
        outbox
            .append(
                frame.clone(),
                route_receipt.clone(),
                Some(runtime_parity(&frame, "7")),
            )
            .expect("parity enrichment");
        assert_eq!(outbox.frames.len(), 1);
        assert!(
            outbox
                .materialized_frames()
                .expect("materialize frames")
                .iter()
                .all(|bound| bound.runtime_parity_case.is_some())
        );
        drop(outbox);

        let mut restored = LocalEvidenceOutbox::open(&root).expect("restore");
        assert!(
            restored
                .materialized_frames()
                .expect("materialize restored frames")
                .iter()
                .all(|bound| bound.runtime_parity_case.is_some())
        );
        restored
            .append(frame.clone(), route_receipt.clone(), None)
            .expect("legacy replay cannot erase parity");
        assert_eq!(
            restored
                .append(
                    frame.clone(),
                    route_receipt,
                    Some(runtime_parity(&frame, "8")),
                )
                .expect_err("parity rebound must fail"),
            "evidence_agent_outbox_rebound"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn outbox_accepts_only_pre_action_nando_route_bound_frames() {
        let root = temporary_root("route-bound");
        let route_path = root.join("route-receipts-v1.jsonl");
        let outbox = Arc::new(Mutex::new(
            LocalEvidenceOutbox::open(&root.join("outbox")).expect("outbox"),
        ));
        let transport_censors = Arc::new(Mutex::new(
            TransportCensorLedger::open(&root.join("transport-censors"))
                .expect("transport censors"),
        ));
        let route_index = Arc::new(Mutex::new(
            NandoRouteReceiptIndex::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .expect("route index"),
        ));
        let metrics = Arc::new(RouteBindingMetrics::default());
        let sink = OutboxSink {
            outbox: Arc::clone(&outbox),
            transport_censors,
            route_receipts: Arc::clone(&route_index),
            route_metrics: Arc::clone(&metrics),
        };

        let mut unbound = completed_frame("unbound");
        unbound.client_intent_id_sha256 = evidence_client_intent_id_sha256("turn-unbound");
        unbound.session_id_sha256 = evidence_session_id_sha256("session-unbound");
        sink.append_verified_frame(unbound).expect("censor unbound");
        assert!(outbox.lock().expect("outbox lock").frames.is_empty());
        assert_eq!(metrics.route_unbound_frames.load(Ordering::Relaxed), 1);

        let identity = ClientRouteIdentityV1 {
            turn_intent_id_sha256: evidence_client_intent_id_sha256("turn-bound"),
            session_id_sha256: evidence_session_id_sha256("session-bound"),
        };
        let mut ledger =
            NandoRouteReceiptLedger::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .expect("route ledger");
        ledger
            .append(
                &identity,
                client_sha256_bytes(b"request"),
                418,
                1_600_000_000_000_000_000,
                1_650_000_000_000_000_000,
            )
            .expect("route receipt");
        let mut bound = completed_frame("bound");
        bound.client_intent_id_sha256 = identity.turn_intent_id_sha256;
        bound.session_id_sha256 = identity.session_id_sha256;
        sink.append_verified_frame(bound).expect("append bound");
        let outbox = outbox.lock().expect("outbox lock");
        assert_eq!(outbox.frames.len(), 1);
        assert!(outbox.materialized_frames().expect("materialize frames").iter().all(|bound| {
            valid_root(&bound.route_receipt_root_sha256)
                && bound.route_receipt.validate()
                && bound.route_receipt.receipt_root_sha256 == bound.route_receipt_root_sha256
        }));
        assert_eq!(metrics.route_bound_frames.load(Ordering::Relaxed), 1);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn invalid_transport_frame_is_censored_before_next_valid_frame() {
        let root = temporary_root("transport-censor-continue");
        let censor_directory = root.join("transport-censors");
        let outbox = Arc::new(Mutex::new(
            LocalEvidenceOutbox::open(&root.join("outbox")).expect("outbox"),
        ));
        let transport_censors = Arc::new(Mutex::new(
            TransportCensorLedger::open(&censor_directory).expect("transport censors"),
        ));
        let route_index = Arc::new(Mutex::new(
            NandoRouteReceiptIndex::open(
                &root.join("route-receipts-v1.jsonl"),
                DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES,
            )
            .expect("route index"),
        ));
        let metrics = Arc::new(RouteBindingMetrics::default());
        let sink = OutboxSink {
            outbox: Arc::clone(&outbox),
            transport_censors: Arc::clone(&transport_censors),
            route_receipts: route_index,
            route_metrics: Arc::clone(&metrics),
        };

        let invalid = completed_frame("invalid-parity");
        let mut invalid_parity = runtime_parity(&invalid, "7");
        invalid_parity.evidence_ref_sha256 = hash("wrong-evidence-reference");
        sink.append_route_bound_frame(
            invalid.clone(),
            route_receipt_for_frame(&invalid),
            Some(invalid_parity),
        )
        .expect("invalid transport frame is terminally censored");
        assert!(outbox.lock().expect("outbox lock").frames.is_empty());
        assert_eq!(
            metrics
                .transport_censored_frames
                .load(Ordering::Relaxed),
            1
        );

        let valid = completed_frame("valid-after-censor");
        sink.append_route_bound_frame(
            valid.clone(),
            route_receipt_for_frame(&valid),
            Some(runtime_parity(&valid, "7")),
        )
        .expect("next valid frame reaches outbox");
        assert_eq!(outbox.lock().expect("outbox lock").frames.len(), 1);
        assert_eq!(metrics.route_bound_frames.load(Ordering::Relaxed), 1);

        let restored = TransportCensorLedger::open(&censor_directory).expect("restart restore");
        assert_eq!(restored.len(), 1);
        assert!(restored.receipts.values().all(|receipt| {
            receipt.blocker
                == RemoteEvidenceFrameValidationBlockerV1::ParityEvidenceReferenceMismatch
                && !receipt.authority_ready
                && !receipt.phase_mutation_allowed
        }));
        drop(sink);
        drop(transport_censors);
        drop(outbox);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn corrupted_transport_censor_ledger_is_fatal_on_restart() {
        let root = temporary_root("transport-censor-corrupt");
        let directory = root.join("transport-censors");
        let frame = completed_frame("corrupt-censor");
        let bound = super::RouteBoundOutboxFrameV1::new(
            frame.clone(),
            route_receipt_for_frame(&frame),
            None,
        );
        let mut ledger = TransportCensorLedger::open(&directory).expect("transport censors");
        ledger
            .append(
                &bound,
                RemoteEvidenceFrameValidationBlockerV1::ParityRequestMissing,
            )
            .expect("append censor");
        drop(ledger);

        let segment = directory.join(format!("{TRANSPORT_CENSOR_PREFIX}-{:020}.cbor", 0));
        let mut bytes = fs::read(&segment).expect("segment bytes");
        bytes[0] ^= 0xff;
        fs::write(&segment, bytes).expect("corrupt segment");
        assert!(TransportCensorLedger::open(&directory).is_err());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
