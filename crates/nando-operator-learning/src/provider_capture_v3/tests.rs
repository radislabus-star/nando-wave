use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};

use super::*;

fn root(label: &str) -> Sha256CommitmentV3 {
    Sha256CommitmentV3::digest_bytes(label.as_bytes())
}

fn receipt(sequence: u64, request_label: &str) -> ProviderRequestCaptureReceiptV3 {
    receipt_in_epoch(sequence, request_label, root("epoch-a"))
}

fn receipt_in_epoch(
    sequence: u64,
    request_label: &str,
    epoch_root: Sha256CommitmentV3,
) -> ProviderRequestCaptureReceiptV3 {
    seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: sequence,
        capture_epoch_root: epoch_root,
        lineage_root_sha256: root("lineage-a"),
        request_root_sha256: root(request_label),
        projection: RuntimeProjectionV3::Responses,
        streaming: sequence.is_multiple_of(2),
        observed_at_unix_ms: 1_750_000_000_000 + sequence,
    })
    .expect("receipt")
}

#[test]
fn receipt_roundtrip_is_canonical_hash_only_and_tamper_evident() {
    let receipt = receipt(7, "request-7-private-payload");
    let bytes = receipt.canonical_bytes().expect("bytes");
    let restored = ProviderRequestCaptureReceiptV3::from_canonical_bytes(&bytes).expect("restore");

    assert_eq!(restored, receipt);
    assert!(
        !bytes
            .windows(b"request-7-private-payload".len())
            .any(|window| { window == b"request-7-private-payload" })
    );
    assert!(!restored.execution_authority());

    let mut tampered = bytes.into_vec();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert!(ProviderRequestCaptureReceiptV3::from_canonical_bytes(&tampered).is_err());
}

#[test]
fn index_blocks_duplicate_request_event_and_sequence_roots() {
    let (index, lease) = ProviderCaptureIndexV3::empty()
        .expect("empty")
        .reserve_next_lease()
        .expect("reserve");
    let first = receipt_in_epoch(1, "same-request", lease.epoch_root_sha256());
    let duplicate_request = receipt_in_epoch(2, "same-request", lease.epoch_root_sha256());
    let index = index
        .append_batch(std::slice::from_ref(&first))
        .expect("append");

    assert_eq!(
        index.append_batch(&[duplicate_request]),
        Err(ProviderCaptureIndexErrorV3::DuplicateCommitment)
    );
    assert_eq!(
        index.append_batch(&[first]),
        Err(ProviderCaptureIndexErrorV3::DuplicateCommitment)
    );
}

#[test]
fn index_roundtrip_preserves_gaps_and_transition_extension() {
    let empty = ProviderCaptureIndexV3::empty().expect("empty");
    let (reserved, lease) = empty.reserve_next_lease().expect("reserve");
    let appended = reserved
        .append_batch(&[
            receipt_in_epoch(9, "request-9", lease.epoch_root_sha256()),
            receipt_in_epoch(3, "request-3", lease.epoch_root_sha256()),
        ])
        .expect("append out of order");
    appended
        .validate_transition_from(&reserved)
        .expect("transition");
    let bytes = appended.canonical_bytes().expect("bytes");
    let restored = ProviderCaptureIndexV3::from_canonical_bytes(&bytes).expect("restore");

    assert_eq!(restored, appended);
    assert_eq!(restored.records()[0].capture_sequence(), 3);
    assert_eq!(restored.records()[1].capture_sequence(), 9);
    assert_eq!(restored.leases(), &[lease]);
    assert_eq!(restored.raw_payloads_persisted(), 0);
    assert!(!restored.execution_authority());
}

#[test]
fn receipt_with_a_foreign_epoch_cannot_enter_the_reserved_index() {
    let (reserved, _) = ProviderCaptureIndexV3::empty()
        .expect("empty")
        .reserve_next_lease()
        .expect("reserve");

    assert_eq!(
        reserved.append_batch(&[receipt(1, "foreign-epoch")]),
        Err(ProviderCaptureIndexErrorV3::SequenceOutsideLease)
    );
}

#[test]
fn maximal_index_fits_the_frozen_eight_mib_budget() {
    let (reserved, lease) = ProviderCaptureIndexV3::empty()
        .expect("empty")
        .reserve_next_lease()
        .expect("reserve");
    let records = (1..=PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3 as u64)
        .map(|sequence| {
            receipt_in_epoch(
                sequence,
                &format!("request-{sequence}"),
                lease.epoch_root_sha256(),
            )
        })
        .collect::<Vec<_>>();
    let index = reserved.append_batch(&records).expect("maximal index");
    let bytes = index.canonical_bytes().expect("maximal bytes");

    assert_eq!(index.records().len(), PROVIDER_CAPTURE_INDEX_MAX_RECORDS_V3);
    assert!(bytes.len() <= PROVIDER_CAPTURE_INDEX_MAX_BYTES_V3);
}
