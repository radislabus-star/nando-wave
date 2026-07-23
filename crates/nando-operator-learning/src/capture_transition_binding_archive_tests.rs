use std::path::PathBuf;

use super::*;
use crate::CaptureRecordCommitment;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nando-capture-binding-{}-{name}",
        std::process::id()
    ))
}

fn receipt(sequence: u64, byte: char) -> CaptureEvidenceReceipt {
    CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
        sequence,
        record_sha256: byte.to_string().repeat(64),
    }])
    .expect("receipt")
}

#[test]
fn archived_binding_rejects_a_different_valid_receipt_for_the_same_frame() {
    let directory = root("substitution");
    let _ = std::fs::remove_dir_all(&directory);
    let mut archive = CaptureTransitionBindingArchive::open(&directory).expect("archive");

    let mut first = receipt(10, 'a');
    let first_binding = archive
        .append(&"1".repeat(64), &first)
        .expect("first binding");
    first
        .bind_transition(first_binding)
        .expect("bind first receipt");

    let mut second = receipt(11, 'b');
    let second_binding = archive
        .append(&"2".repeat(64), &second)
        .expect("second binding");
    second
        .bind_transition(second_binding)
        .expect("bind second receipt");
    archive.seal().expect("seal");

    let mut reader =
        CaptureTransitionBindingArchiveReader::open(&directory).expect("binding reader");
    assert_eq!(reader.verify(&"1".repeat(64), &first), Ok(()));
    assert_eq!(
        reader.verify(&"1".repeat(64), &second),
        Err("capture_transition_binding_mismatch".to_owned())
    );
}

#[test]
fn binding_archive_rejects_tampered_chain() {
    let directory = root("tampered-chain");
    let _ = std::fs::remove_dir_all(&directory);
    let mut archive = CaptureTransitionBindingArchive::open(&directory).expect("archive");
    archive
        .append(&"3".repeat(64), &receipt(12, 'c'))
        .expect("binding");
    archive.seal().expect("seal");
    drop(archive);

    let data_path = directory.join(DATA_FILE);
    let mut bytes = std::fs::read(&data_path).expect("binding bytes");
    bytes[20] ^= 0xff;
    std::fs::write(&data_path, bytes).expect("tamper binding archive");
    assert!(CaptureTransitionBindingArchiveReader::open(&directory).is_err());
}
