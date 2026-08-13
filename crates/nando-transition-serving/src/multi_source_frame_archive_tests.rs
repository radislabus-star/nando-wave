use nando_operator_kernel::{
    AtomSource, AtomValueType, RELATION_FRAME_SCHEMA, RelationAtom, sha256_bytes,
};
use nando_operator_learning::SOURCE_NEUTRAL_EXTRACTOR_VERSION;

use super::*;

fn frame(index: usize, intent: &str) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: sha256_bytes(format!("frame-{index}").as_bytes()),
        event_id_sha256: sha256_bytes(format!("event-{index}").as_bytes()),
        client_intent_id_sha256: sha256_bytes(intent.as_bytes()),
        session_id_sha256: sha256_bytes(format!("session-{index}").as_bytes()),
        observed_at_unix_nanos: 1_000 + index as u64,
        estimated_input_tokens: 10,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![RelationAtom::TypedSlot {
            slot_id: 0,
            value_type: AtomValueType::Integer,
            source: AtomSource::Observation,
            value_sha256: sha256_bytes(format!("value-{index}").as_bytes()),
        }],
        evidence_ref_sha256: sha256_bytes(format!("evidence-{index}").as_bytes()),
    }
}

#[test]
fn archive_retains_more_than_one_signature_reservoir_and_restarts() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-frame-archive-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceFrameArchive::open(&root).expect("archive");
    for index in 0..40 {
        archive
            .append(&frame(index, "same-intent"))
            .expect("append");
    }
    assert_eq!(archive.len(), 40);
    let expected_prefix = frame_prefix_root(&archive.append_order).expect("prefix");
    drop(archive);

    let read_only = MultiSourceFrameArchiveReadSnapshot::read(&root).expect("read only");
    assert_eq!(read_only.len(), 40);
    assert_eq!(read_only.prefix_root_sha256(), expected_prefix);
    assert_eq!(read_only.frames().len(), 40);

    let restored = MultiSourceFrameArchive::open(&root).expect("restore");
    let intents = BTreeSet::from([sha256_bytes(b"same-intent")]);
    assert_eq!(restored.frames_for_intents(&intents).len(), 40);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn read_snapshot_verifies_a_frozen_prefix_and_ignores_later_tail() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-frame-prefix-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceFrameArchive::open(&root).expect("archive");
    for index in 0..6 {
        archive
            .append(&frame(index, "intent"))
            .expect("append prefix");
    }
    let frozen_root = frame_prefix_root(&archive.append_order[..6]).expect("frozen root");
    for index in 6..10 {
        archive
            .append(&frame(index, "intent"))
            .expect("append tail");
    }
    drop(archive);

    let snapshot = MultiSourceFrameArchiveReadSnapshot::read(&root).expect("snapshot");
    let frozen = snapshot
        .verified_prefix(6, &frozen_root)
        .expect("verified prefix");
    assert_eq!(frozen.len(), 6);
    assert_eq!(
        snapshot.verified_prefix(6, &sha256_bytes(b"forged")),
        Err("multi_source_frame_archive_prefix_root_mismatch".to_owned())
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_rejects_unverified_frame() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-frame-unverified-{}",
        std::process::id()
    ));
    let mut archive = MultiSourceFrameArchive::open(&root).expect("archive");
    let mut row = frame(0, "intent");
    row.verifier_label = None;
    assert_eq!(
        archive.append(&row).expect_err("reject"),
        "multi_source_frame_archive_frame_invalid"
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_exposes_restart_stable_append_cursor() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-frame-cursor-{}",
        std::process::id()
    ));
    let expected = [
        frame(3, "intent-3"),
        frame(1, "intent-1"),
        frame(2, "intent-2"),
    ];
    let mut archive = MultiSourceFrameArchive::open(&root).expect("archive");
    for row in &expected {
        archive.append(row).expect("append");
    }
    assert_eq!(
        archive
            .shared_frames_after(1)
            .expect("cursor")
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<Vec<_>>(),
        expected[1..]
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<Vec<_>>()
    );
    drop(archive);

    let restored = MultiSourceFrameArchive::open(&root).expect("restore");
    assert_eq!(
        restored
            .shared_frames()
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|row| row.frame_id_sha256.as_str())
            .collect::<Vec<_>>()
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn archive_resolves_only_requested_canonical_roots_after_restart() {
    let root = std::env::temp_dir().join(format!(
        "nando-multi-source-frame-roots-{}",
        std::process::id()
    ));
    let expected = [frame(11, "intent-11"), frame(12, "intent-12")];
    let roots = expected
        .iter()
        .map(|row| canonical_json_sha256(row).expect("root"))
        .collect::<Vec<_>>();
    let mut archive = MultiSourceFrameArchive::open(&root).expect("archive");
    for row in &expected {
        archive.append(row).expect("append");
    }
    drop(archive);

    let restored = MultiSourceFrameArchive::open(&root).expect("restore");
    let selected = BTreeSet::from([roots[1].clone()]);
    assert_eq!(
        restored.frames_for_roots(&selected),
        vec![expected[1].clone()]
    );
    assert_eq!(restored.frame_by_root(&roots[0]), Some(expected[0].clone()));
    std::fs::remove_dir_all(root).expect("cleanup");
}
