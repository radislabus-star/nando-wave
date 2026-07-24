use std::path::PathBuf;

use nando_operator_learning::{CaptureEvidenceReceipt, CaptureRecordCommitment};
use nando_response_actor::CaptureTransitionBindingArchiveReader;

use super::*;

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
fn writer_and_reader_reject_receipt_substitution() {
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
        reader.verify_binding(first.transition_binding.as_ref().expect("first binding")),
        Ok(())
    );
    assert_eq!(
        reader.verify(&"1".repeat(64), &second),
        Err("capture_transition_binding_mismatch".to_owned())
    );
}

#[test]
fn reader_rejects_a_tampered_writer_chain() {
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

#[test]
fn replay_is_idempotent_and_frame_rebinding_fails_closed() {
    let directory = root("replay");
    let _ = std::fs::remove_dir_all(&directory);
    let first_receipt = receipt(20, 'd');
    let mut archive = CaptureTransitionBindingArchive::open(&directory).expect("archive");
    let first = archive
        .append(&"4".repeat(64), &first_receipt)
        .expect("first binding");
    archive.seal().expect("seal");
    drop(archive);

    let bytes_before = std::fs::metadata(directory.join(DATA_FILE))
        .expect("binding metadata")
        .len();
    let mut restored = CaptureTransitionBindingArchive::open(&directory).expect("restore");
    let replay = restored
        .append(&"4".repeat(64), &first_receipt)
        .expect("idempotent replay");
    assert_eq!(replay, first);
    assert_eq!(
        std::fs::metadata(directory.join(DATA_FILE))
            .expect("binding metadata")
            .len(),
        bytes_before
    );
    assert_eq!(
        restored.append(&"4".repeat(64), &receipt(21, 'e')),
        Err("capture_transition_binding_frame_rebound".to_owned())
    );
}
