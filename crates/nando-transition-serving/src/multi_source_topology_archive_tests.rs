use nando_operator_kernel::{
    LearningRequestStructureV2, MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1,
    PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1, sha256_bytes,
};
use nando_operator_learning::multi_source::PreActionTopologyAuditRowV1;

use super::*;

fn row(index: u64) -> PreActionTopologyAuditRowV1 {
    let turn = sha256_bytes(format!("turn-{index}").as_bytes());
    let request_root = sha256_bytes(format!("request-{index}").as_bytes());
    let structure = LearningRequestStructureV2 {
        schema: nando_operator_kernel::LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: turn,
        request_event_id_sha256: sha256_bytes(format!("event-{index}").as_bytes()),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![sha256_bytes(format!("session-{index}").as_bytes())],
        request_phase_atom_ids: vec![index],
        pre_action_context_atom_ids: vec![index + 100],
        capability_atom_ids: vec![index + 200],
        estimated_input_tokens: 32,
        provider_payload_bytes: 64,
        provider_capture_request_root_sha256: request_root,
        decidability_reason_code: "test".to_owned(),
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
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        sha256_bytes(b"extractor"),
        sha256_bytes(b"config"),
        index + 1,
    )
    .expect("commit");
    PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: sha256_bytes(b"bridge"),
        bridge_sequence: Some(index + 1),
        record_sha256: Some(sha256_bytes(format!("record-{index}").as_bytes())),
        capture_epoch_sha256: Some(sha256_bytes(b"capture-epoch")),
        capture_event_sha256: Some(sha256_bytes(format!("capture-event-{index}").as_bytes())),
        capture_receipt_sha256: Some(sha256_bytes(format!("capture-receipt-{index}").as_bytes())),
        captured_at_unix_ms: Some(index + 1),
        session_lineage_sha256: Some(sha256_bytes(format!("lineage-{index}").as_bytes())),
        physical_order_proven: true,
        structure,
        commit,
    }
}

#[test]
fn archive_retains_rows_across_restart() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-archive-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    for index in 0..40 {
        archive.append(&row(index)).expect("append");
    }
    assert_eq!(archive.len(), 40);
    let prefix = archive.prefix_root(40).expect("prefix");
    drop(archive);

    let restored = MultiSourceTopologyArchive::open(&root).expect("restore");
    assert_eq!(restored.rows().len(), 40);
    assert_eq!(restored.prefix_root(40).expect("restored prefix"), prefix);
    assert!(restored.rows_after(40).expect("tail").is_empty());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_rejects_unproven_physical_order() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-invalid-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    let mut invalid = row(0);
    invalid.physical_order_proven = false;
    assert_eq!(
        archive.append(&invalid).expect_err("reject"),
        "multi_source_topology_archive_row_invalid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_maps_closure_sequence_to_the_exact_consumed_cursor() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-cursor-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    for index in 0..8 {
        archive.append(&row(index)).expect("append");
    }

    assert_eq!(archive.bridge_sequence_at_cursor(4).expect("sequence"), 4);
    assert_eq!(
        archive
            .cursor_after_bridge_sequence(4)
            .expect("closure cursor"),
        4
    );
    assert_eq!(archive.rows_between(2, 5).expect("bounded rows").len(), 3);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_accepts_backfilled_order_with_a_clean_closure_boundary() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-backfill-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    for index in [3, 1, 2, 0, 4, 5] {
        archive.append(&row(index)).expect("append");
    }

    assert_eq!(
        archive
            .cursor_after_bridge_sequence(4)
            .expect("closure cursor"),
        4
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_rejects_a_closure_that_would_consume_future_rows() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-crossed-boundary-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    for index in [0, 4, 3, 5] {
        archive.append(&row(index)).expect("append");
    }

    assert_eq!(
        archive
            .cursor_after_bridge_sequence(4)
            .expect_err("crossed boundary"),
        "multi_source_topology_archive_sequence_boundary_invalid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_requires_the_exact_closure_sequence() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-missing-closure-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    for index in [0, 1, 3] {
        archive.append(&row(index)).expect("append");
    }

    assert_eq!(
        archive
            .cursor_after_bridge_sequence(3)
            .expect_err("missing closure"),
        "multi_source_topology_archive_sequence_missing"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_rejects_an_ambiguous_closure_sequence() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-topology-ambiguous-closure-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceTopologyArchive::open(&root).expect("archive");
    archive.append(&row(0)).expect("first append");
    let mut duplicate_sequence = row(9);
    duplicate_sequence.bridge_sequence = Some(1);
    archive.append(&duplicate_sequence).expect("second append");

    assert_eq!(
        archive
            .cursor_after_bridge_sequence(1)
            .expect_err("ambiguous closure"),
        "multi_source_topology_archive_sequence_order_invalid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
