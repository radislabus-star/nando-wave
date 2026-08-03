use nando_operator_kernel::{
    AtomValueType, CollectionProgramStep, CollectionScalarType,
    LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
    MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
    MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1,
    ResponseProgram, ResponseValueSelector, ValueProjectionFormat, canonical_json_sha256,
    response_program_version_root_sha256,
};
use nando_operator_learning::multi_source::PreActionTopologyAuditRowV1;

use super::*;

fn root(label: &str) -> String {
    sha256_bytes(label.as_bytes())
}

fn collection_topology() -> PreActionMultiSourceTopologyV1 {
    PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles: vec![MultiSourceRoleNodeV1 {
            local_role_id: 1,
            source_ordinal: 0,
            value_ordinal: 0,
            type_class: MultiSourceTypeClassV1::Array,
            container_class: MultiSourceContainerClassV1::Sequence,
            cardinality_class: MultiSourceCardinalityClassV1::Many,
            temporal_class: MultiSourceTemporalClassV1::Latest,
            depth_bucket: 1,
            structural_flags: 0,
        }],
        role_witnesses: vec![MultiSourceRoleWitnessV1 {
            local_role_id: 1,
            value_sha256: canonical_json_sha256(&serde_json::json!({
                "items": [{"id": 1}, {"id": 2}, {"id": 3}]
            }))
            .expect("collection witness"),
            request_reference_ordinal: None,
            request_reference_ordinal_candidates: Vec::new(),
        }],
        relations: Vec::new(),
    }
}

fn topology_row(request_root: String) -> PreActionTopologyAuditRowV1 {
    let structure = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: root("turn"),
        request_event_id_sha256: root("request-event"),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![root("session")],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        estimated_input_tokens: 10,
        provider_payload_bytes: 10,
        provider_capture_request_root_sha256: request_root,
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology: collection_topology(),
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("extractor-config"),
        1,
    )
    .expect("commit");
    PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: root("bridge"),
        bridge_sequence: Some(1),
        record_sha256: Some(root("record")),
        capture_epoch_sha256: Some(root("capture-epoch")),
        capture_event_sha256: Some(root("capture-event")),
        capture_receipt_sha256: Some(root("capture-receipt")),
        captured_at_unix_ms: Some(1),
        session_lineage_sha256: Some(root("session")),
        physical_order_proven: true,
        structure,
        commit,
    }
}

#[test]
fn independent_authority_executes_and_seals_the_typed_consequence() {
    let payload = r#"{"input":[{"type":"function_call_output","call_id":"call-1","output":"{\"items\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"}]}"#;
    let program = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let receipt = execute_collection_prediction(
        root("contract"),
        &program,
        &topology_row(sha256_bytes(payload.as_bytes())),
        payload,
    )
    .expect("authority execution");

    assert_eq!(
        receipt.canonical_program_root_sha256,
        response_program_version_root_sha256(&program).expect("program root")
    );
    assert!(!receipt.authority_ready);
    assert!(!receipt.phase_mutation_allowed);
    receipt.validate().expect("receipt");
}

#[test]
fn authority_rejects_payload_that_does_not_match_the_provider_capture() {
    let payload = r#"{"input":[{"type":"function_call_output","call_id":"call-1","output":"{\"items\":[{\"id\":1},{\"id\":2},{\"id\":3}]}"}]}"#;
    let program = ResponseProgram::compose_collection(
        vec![CollectionProgramStep::SelectOnlyArrayField],
        ValueProjectionFormat::CanonicalJson,
        "completed",
    );
    assert_eq!(
        execute_collection_prediction(
            root("contract"),
            &program,
            &topology_row(root("different-request")),
            payload,
        ),
        Err("k1_pre_action_request_digest_mismatch".to_owned())
    );
}

#[test]
fn authority_wire_rejects_a_client_supplied_execution_receipt() {
    let request = K1FuturePredictionAuthorityRequestV1 {
        schema: K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
        contract_root_sha256: root("contract"),
        topology_commitment_root_sha256: root("topology"),
        provider_capture_request_root_sha256: root("request"),
    };
    let mut value = serde_json::to_value(request).expect("request value");
    value["pre_action_execution_receipt"] = serde_json::json!({
        "receipt_root_sha256": root("forged-receipt")
    });
    assert!(serde_json::from_value::<K1FuturePredictionAuthorityRequestV1>(value).is_err());
}

#[test]
fn authority_wire_rejects_forged_request_text_and_topology() {
    let request = K1FuturePredictionAuthorityRequestV1 {
        schema: K1_FUTURE_PREDICTION_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
        contract_root_sha256: root("contract"),
        topology_commitment_root_sha256: root("topology"),
        provider_capture_request_root_sha256: root("request"),
    };
    let mut value = serde_json::to_value(request).expect("request value");
    value["request_text"] = serde_json::json!("forged selector input");
    value["topology"] = serde_json::to_value(topology_row(root("request"))).expect("topology");
    assert!(serde_json::from_value::<K1FuturePredictionAuthorityRequestV1>(value).is_err());
}

#[test]
fn authority_rejects_runtime_selector_value_that_differs_from_frozen_witness() {
    let payload = r#"{"input":[{"role":"user","content":"count status active"},{"type":"function_call_output","call_id":"call-1","output":"{\"items\":[{\"status\":\"active\"}]}"}]}"#;
    let mut row = topology_row(sha256_bytes(payload.as_bytes()));
    row.structure.topology.role_witnesses[0].value_sha256 =
        canonical_json_sha256(&serde_json::json!({"items": [{"status": "active"}]}))
            .expect("collection witness");
    row.structure.topology.roles.push(MultiSourceRoleNodeV1 {
        local_role_id: 2,
        source_ordinal: 1,
        value_ordinal: 0,
        type_class: MultiSourceTypeClassV1::String,
        container_class: MultiSourceContainerClassV1::Scalar,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: MultiSourceTemporalClassV1::Latest,
        depth_bucket: 2,
        structural_flags: 1,
    });
    row.structure
        .topology
        .role_witnesses
        .push(MultiSourceRoleWitnessV1 {
            local_role_id: 2,
            value_sha256: canonical_json_sha256(&serde_json::json!("forged"))
                .expect("witness root"),
            request_reference_ordinal: Some(0),
            request_reference_ordinal_candidates: Vec::new(),
        });
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::RequestReferencesRole,
            source_role_id: 2,
            target_role_id: 2,
        });
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("extractor-config"),
        1,
    )
    .expect("commit");
    let program = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsSelectedValue {
                selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                    ordinal: 0,
                    value_type: AtomValueType::String,
                },
                value_type: CollectionScalarType::String,
            },
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    );
    assert_eq!(
        execute_collection_prediction(root("contract"), &program, &row, payload),
        Err("k1_pre_action_input_witness_mismatch".to_owned())
    );
}

#[test]
fn authority_rejects_implicit_request_value_that_differs_from_frozen_witness() {
    let payload = r#"{"input":[{"role":"user","content":"count status active"},{"type":"function_call_output","call_id":"call-1","output":"{\"items\":[{\"status\":\"active\"},{\"status\":\"idle\"}]}"}]}"#;
    let mut row = topology_row(sha256_bytes(payload.as_bytes()));
    row.structure.topology.role_witnesses[0].value_sha256 = canonical_json_sha256(
        &serde_json::json!({"items": [{"status": "active"}, {"status": "idle"}]}),
    )
    .expect("collection witness");
    row.structure.topology.roles.push(MultiSourceRoleNodeV1 {
        local_role_id: 2,
        source_ordinal: 0,
        value_ordinal: 1,
        type_class: MultiSourceTypeClassV1::String,
        container_class: MultiSourceContainerClassV1::Scalar,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: MultiSourceTemporalClassV1::Latest,
        depth_bucket: 2,
        structural_flags: 1,
    });
    row.structure
        .topology
        .role_witnesses
        .push(MultiSourceRoleWitnessV1 {
            local_role_id: 2,
            value_sha256: canonical_json_sha256(&serde_json::json!("forged"))
                .expect("forged witness"),
            request_reference_ordinal: Some(0),
            request_reference_ordinal_candidates: Vec::new(),
        });
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::RequestReferencesRole,
            source_role_id: 2,
            target_role_id: 2,
        });
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("extractor-config"),
        1,
    )
    .expect("commit");
    let program = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::FilterUniqueFieldEqualsRequestValue {
                value_type: CollectionScalarType::String,
            },
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    );

    assert_eq!(
        execute_collection_prediction(root("contract"), &program, &row, payload),
        Err("k1_pre_action_input_witness_mismatch".to_owned())
    );
}
