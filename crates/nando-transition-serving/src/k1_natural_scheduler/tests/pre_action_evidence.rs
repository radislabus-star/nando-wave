use nando_operator_kernel::{
    LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
    MultiSourceExtractionStatusV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
    PreActionTopologyCommitV1, sha256_bytes,
};
use nando_operator_learning::multi_source::PreActionTopologyAuditRowV1;

use super::*;
use crate::multi_source_topology_archive::MultiSourceTopologyArchive;

fn topology_row(request_root: String) -> PreActionTopologyAuditRowV1 {
    let structure = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: root(700),
        request_event_id_sha256: root(701),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![root(702)],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        estimated_input_tokens: 10,
        provider_payload_bytes: 100,
        provider_capture_request_root_sha256: request_root,
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology: PreActionMultiSourceTopologyV1 {
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
                value_sha256: root(703),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
            }],
            relations: Vec::new(),
        },
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root(704),
        root(705),
        1,
    )
    .expect("commit");
    PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: root(706),
        bridge_sequence: Some(1),
        record_sha256: Some(root(707)),
        capture_epoch_sha256: Some(root(708)),
        capture_event_sha256: Some(root(709)),
        capture_receipt_sha256: Some(root(710)),
        captured_at_unix_ms: Some(1),
        session_lineage_sha256: Some(root(702)),
        physical_order_proven: true,
        structure,
        commit,
    }
}

#[test]
fn self_valid_topology_absent_from_archive_is_rejected() {
    let (root_path, config, _) = test_context();
    let payload = r#"{"input":"count items"}"#;
    let row = topology_row(sha256_bytes(payload.as_bytes()));
    let error = super::super::pre_action_evidence::archive(
        &config,
        &row.commit.commitment_root_sha256,
        &row.commit.provider_capture_request_root_sha256,
        payload.to_owned(),
    )
    .expect_err("unarchived topology must fail");
    assert!(error.starts_with("k1_pre_action_topology_archive:"));
    std::fs::remove_dir_all(root_path).expect("cleanup");
}

#[test]
fn authority_archive_restores_hash_bound_payload_by_roots() {
    let (root_path, config, _) = test_context();
    let payload = r#"{"input":"count items"}"#;
    let row = topology_row(sha256_bytes(payload.as_bytes()));
    let archive_path = config
        .root
        .parent()
        .expect("state parent")
        .join("pre-action-topology-archive-v1");
    MultiSourceTopologyArchive::open(&archive_path)
        .expect("topology archive")
        .append(&row)
        .expect("append topology");
    super::super::pre_action_evidence::archive(
        &config,
        &row.commit.commitment_root_sha256,
        &row.commit.provider_capture_request_root_sha256,
        payload.to_owned(),
    )
    .expect("archive evidence");
    let restored = super::super::pre_action_evidence::restore(
        &config,
        &row.commit.commitment_root_sha256,
        &row.commit.provider_capture_request_root_sha256,
    )
    .expect("restore evidence");
    assert_eq!(restored.topology, row);
    assert_eq!(restored.provider_payload_json, payload);
    std::fs::remove_dir_all(root_path).expect("cleanup");
}
