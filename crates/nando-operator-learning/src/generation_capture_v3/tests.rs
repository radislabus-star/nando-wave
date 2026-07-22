use nando_operator_kernel::sha256_bytes;

use super::{GenerationCaptureCommitmentV3, GenerationCaptureErrorV3, GenerationCaptureIndexV3};

fn root(label: &str) -> String {
    sha256_bytes(label.as_bytes())
}

#[test]
fn capture_owner_commitment_binds_all_generation_roots() {
    let record_root = root("record-7");
    let lineage_root = root("lineage-a");
    let event_root = root("event-7");
    let request_root = root("raw request");
    let commitment = GenerationCaptureCommitmentV3::new(
        7,
        record_root.clone(),
        lineage_root.clone(),
        event_root.clone(),
        request_root.clone(),
    )
    .expect("commitment");

    assert_eq!(commitment.capture_sequence(), 7);
    assert_eq!(commitment.record_root_sha256(), record_root);
    assert_eq!(commitment.lineage_root_sha256(), lineage_root);
    assert_eq!(commitment.event_root_sha256(), event_root);
    assert_eq!(commitment.request_root_sha256(), request_root);
}

#[test]
fn index_is_sorted_restart_exact_and_fail_closed() {
    let first = GenerationCaptureCommitmentV3::new(
        3,
        root("record-3"),
        root("lineage-3"),
        root("event-3"),
        root("raw request 3"),
    )
    .expect("first");
    let second = GenerationCaptureCommitmentV3::new(
        9,
        root("record-9"),
        root("lineage-9"),
        root("event-9"),
        root("raw request 9"),
    )
    .expect("second");
    let index = GenerationCaptureIndexV3::new(vec![second.clone(), first]).expect("index");
    let bytes = index.canonical_bytes().expect("canonical bytes");
    let restored = GenerationCaptureIndexV3::from_canonical_bytes(&bytes).expect("restore");

    assert_eq!(restored, index);
    assert_eq!(restored.records()[0].capture_sequence(), 3);
    assert!(restored.contains_exact(
        second.capture_sequence(),
        second.lineage_root_sha256(),
        second.event_root_sha256(),
        second.request_root_sha256(),
    ));
    assert!(!restored.contains_exact(
        second.capture_sequence(),
        second.lineage_root_sha256(),
        second.event_root_sha256(),
        &root("foreign-request"),
    ));

    let mut tampered = bytes.into_vec();
    let last = tampered.len().saturating_sub(1);
    tampered[last] ^= 1;
    assert_eq!(
        GenerationCaptureIndexV3::from_canonical_bytes(&tampered),
        Err(GenerationCaptureErrorV3::InvalidIndex)
    );
}

#[test]
fn duplicate_sequence_or_record_root_is_rejected() {
    let first = GenerationCaptureCommitmentV3::new(
        4,
        root("record-a"),
        root("lineage-a"),
        root("event-a"),
        root("request-a"),
    )
    .expect("first");
    let duplicate_sequence = GenerationCaptureCommitmentV3::new(
        4,
        root("record-b"),
        root("lineage-b"),
        root("event-b"),
        root("request-b"),
    )
    .expect("duplicate sequence");
    assert_eq!(
        GenerationCaptureIndexV3::new(vec![first.clone(), duplicate_sequence]),
        Err(GenerationCaptureErrorV3::DuplicateCommitment)
    );

    let duplicate_record = GenerationCaptureCommitmentV3::new(
        5,
        first.record_root_sha256().to_owned(),
        root("lineage-c"),
        root("event-c"),
        root("request-c"),
    )
    .expect("duplicate record");
    assert_eq!(
        GenerationCaptureIndexV3::new(vec![first, duplicate_record]),
        Err(GenerationCaptureErrorV3::DuplicateCommitment)
    );
}
