use super::*;
use nando_operator_kernel::{
    LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, MultiSourceEvidenceOriginV1,
    MultiSourceExtractionStatusV1, PreActionMultiSourceTopologyV1, RuntimeProjectionV3,
    Sha256CommitmentV3,
};
use nando_operator_learning::{
    LearningRequestStructureInputV1, ProviderRequestCaptureInputV3,
    seal_provider_request_capture_v3,
};

#[test]
fn candidate_record_is_complete_and_durable_before_return() {
    let root = std::env::temp_dir().join(format!(
        "nando-structure-durable-{}-{}",
        std::process::id(),
        unix_now_nanos()
    ));
    let (runtime, _) =
        LearningStructureBridgeRuntimeV2::open(root.clone(), true, false, Duration::from_secs(1))
            .expect("bridge");
    let payload = b"{\"input\":\"candidate\"}";
    let request_root = Sha256CommitmentV3::digest_bytes(payload);
    let capture = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: 7,
        capture_epoch_root: Sha256CommitmentV3::digest_bytes(b"capture-epoch"),
        lineage_root_sha256: Sha256CommitmentV3::digest_bytes(b"lineage"),
        request_root_sha256: request_root,
        projection: RuntimeProjectionV3::Responses,
        streaming: true,
        observed_at_unix_ms: 7,
    })
    .expect("capture");
    let structure_v1 = LearningRequestStructureV1::new(LearningRequestStructureInputV1 {
        client_intent_id_sha256: sha256_bytes(b"turn"),
        session_identity_sha256s: vec![sha256_bytes(b"session")],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        provider_bound_turn_identity: true,
        estimated_input_tokens: 10,
        provider_payload_bytes: u64::try_from(payload.len()).expect("payload size"),
    })
    .expect("structure v1");
    let structure_v2 = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: structure_v1.client_intent_id_sha256().to_owned(),
        request_event_id_sha256: sha256_bytes(b"request-event"),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: structure_v1.session_identity_sha256s().to_vec(),
        request_phase_atom_ids: structure_v1.request_phase_atom_ids().to_vec(),
        pre_action_context_atom_ids: structure_v1.pre_action_context_atom_ids().to_vec(),
        capability_atom_ids: structure_v1.capability_atom_ids().to_vec(),
        estimated_input_tokens: structure_v1.estimated_input_tokens(),
        provider_payload_bytes: structure_v1.provider_payload_bytes(),
        provider_capture_request_root_sha256: request_root.to_hex(),
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology: PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 0,
            output_part_count: 0,
            roles: Vec::new(),
            role_witnesses: Vec::new(),
            relations: Vec::new(),
        },
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure_v2,
        MultiSourceEvidenceOriginV1::FreshLive,
        sha256_bytes(b"extractor"),
        sha256_bytes(b"extractor-config"),
        capture.capture_sequence(),
    )
    .expect("commit");

    let row = runtime
        .submit_v3_durable(capture, structure_v1, structure_v2, commit)
        .expect("durable publish");

    assert!(row.physical_order_proven);
    assert_eq!(row.bridge_sequence, Some(1));
    assert_eq!(runtime.status().producer.durable_sequence, 1);
    assert_eq!(runtime.status().producer.durability_syncs, 1);
    assert_eq!(
        fs::read_dir(&runtime.inner.staging_dir)
            .expect("staging")
            .count(),
        0
    );
    let pending = fs::read_dir(&runtime.inner.pending_dir)
        .expect("pending")
        .next()
        .expect("record")
        .expect("record entry")
        .path();
    LearningStructureRecordV3::from_canonical_cbor(&fs::read(pending).expect("record bytes"))
        .expect("complete record");
    fs::remove_dir_all(root).expect("cleanup");
}
