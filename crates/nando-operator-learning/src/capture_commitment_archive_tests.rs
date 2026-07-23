use super::*;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "nando-capture-archive-{}-{name}",
        std::process::id(),
    ))
}

#[test]
fn sealed_archive_verifies_evicted_receipts_and_rejects_tampering() {
    let root = root("verify");
    let _ = std::fs::remove_dir_all(&root);
    let mut archive = CaptureCommitmentArchive::open(&root, 10).expect("archive");
    let records = (10_u64..13)
        .map(|sequence| CaptureRecordCommitment {
            sequence,
            record_sha256: format!("{sequence:064x}"),
        })
        .collect::<Vec<_>>();
    for record in &records {
        archive.append(record).expect("append");
    }
    archive.seal().expect("seal");
    drop(archive);

    let receipt = CaptureEvidenceReceipt::new(records[..2].to_vec()).expect("receipt");
    let mut reader = CaptureCommitmentArchiveReader::open(&root).expect("reader");
    reader
        .verify_receipt(&receipt)
        .expect("durable verification");

    let mut tampered = receipt;
    tampered.records[0].record_sha256 = "f".repeat(64);
    assert!(reader.verify_receipt(&tampered).is_err());
    drop(reader);

    let mut file = OpenOptions::new()
        .write(true)
        .open(root.join(DATA_FILE))
        .expect("open archive data");
    file.seek(SeekFrom::Start(8)).expect("seek archive data");
    file.write_all(&[0xff]).expect("tamper archive data");
    drop(file);
    assert!(matches!(
        CaptureCommitmentArchiveReader::open(&root),
        Err(error) if error == "capture_archive_root_mismatch"
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn restart_discards_only_the_unsealed_tail() {
    let root = root("restart");
    let _ = std::fs::remove_dir_all(&root);
    let committed = CaptureRecordCommitment {
        sequence: 20,
        record_sha256: format!("{:064x}", 20),
    };
    let unsealed = CaptureRecordCommitment {
        sequence: 21,
        record_sha256: format!("{:064x}", 21),
    };

    let mut archive = CaptureCommitmentArchive::open(&root, 20).expect("archive");
    archive.append(&committed).expect("append committed");
    archive.seal().expect("seal");
    archive.append(&unsealed).expect("append unsealed");
    drop(archive);

    let mut recovered = CaptureCommitmentArchive::open(&root, 0).expect("recover");
    recovered.append(&unsealed).expect("reappend");
    recovered.seal().expect("reseal");
    drop(recovered);

    let receipt = CaptureEvidenceReceipt::new(vec![committed, unsealed]).expect("receipt");
    CaptureCommitmentArchiveReader::open(&root)
        .expect("reader")
        .verify_receipt(&receipt)
        .expect("recovered verification");
    assert_eq!(
        std::fs::metadata(root.join(DATA_FILE))
            .expect("archive metadata")
            .len(),
        2 * RECORD_BYTES
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn replay_of_an_already_sealed_record_is_idempotent() {
    let root = root("replay");
    let _ = std::fs::remove_dir_all(&root);
    let first = CaptureRecordCommitment {
        sequence: 30,
        record_sha256: format!("{:064x}", 30),
    };
    let second = CaptureRecordCommitment {
        sequence: 31,
        record_sha256: format!("{:064x}", 31),
    };

    let mut archive = CaptureCommitmentArchive::open(&root, 30).expect("archive");
    archive.append(&first).expect("first append");
    archive.seal().expect("seal first");
    drop(archive);

    let mut restored = CaptureCommitmentArchive::open(&root, 0).expect("restore");
    restored.append(&first).expect("idempotent replay");
    restored.append(&second).expect("second append");
    restored.seal().expect("seal second");
    assert_eq!(
        std::fs::metadata(root.join(DATA_FILE))
            .expect("archive metadata")
            .len(),
        2 * RECORD_BYTES
    );
    let _ = std::fs::remove_dir_all(root);
}
