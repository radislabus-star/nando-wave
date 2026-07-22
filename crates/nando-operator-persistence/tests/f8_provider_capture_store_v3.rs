use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_learning::{ProviderRequestCaptureInputV3, seal_provider_request_capture_v3};
use nando_operator_persistence::{
    PROVIDER_CAPTURE_STORE_SLOT_A_FILE_V3, PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3,
    ProviderCaptureStoreErrorV3, ProviderCaptureStoreReaderV3, ProviderCaptureStoreV3,
};

struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "nando-f8a-{label}-{}-{}",
            std::process::id(),
            root(label).to_hex()
        ));
        let _ = fs::remove_dir_all(&directory);
        Self { directory }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn durable_lease_is_reserved_before_use_and_restart_never_reuses_sequences() {
    let fixture = Fixture::new("lease");
    let store = ProviderCaptureStoreV3::open(&fixture.directory).expect("store");
    let first = store.reserve_sequence_lease().expect("first lease");
    assert_eq!(first.first_sequence(), 1);
    assert_eq!(first.last_sequence(), 16_384);
    assert!(!first.execution_authority());
    assert!(
        fixture
            .directory
            .join(PROVIDER_CAPTURE_STORE_SLOT_A_FILE_V3)
            .exists()
    );

    let restarted = ProviderCaptureStoreV3::open(&fixture.directory).expect("restart");
    let second = restarted.reserve_sequence_lease().expect("second lease");
    assert_eq!(second.first_sequence(), 16_385);
    assert_eq!(second.last_sequence(), 32_768);
    assert_ne!(first.epoch_root_sha256(), second.epoch_root_sha256());
    let restored = restarted.restore().expect("restore");
    assert_eq!(
        restored.index().expect("index").reserved_through_sequence(),
        32_768
    );
}

#[test]
fn append_is_atomic_byte_exact_and_private() {
    let fixture = Fixture::new("append");
    let store = ProviderCaptureStoreV3::open(&fixture.directory).expect("store");
    let lease = store.reserve_sequence_lease().expect("lease");
    let current = store.restore().expect("restore");
    let receipt = receipt(
        lease.first_sequence(),
        lease.epoch_root_sha256(),
        "private-sentinel",
    );
    let next = current
        .index()
        .expect("index")
        .append_batch(&[receipt])
        .expect("append");
    let publish = store.publish_index(&next).expect("publish");
    assert_eq!(publish.index_sha256(), next.index_sha256());
    let restored = store.restore().expect("restart restore");
    assert_eq!(restored.index(), Some(&next));
    let bytes = fs::read(
        fixture
            .directory
            .join(PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3),
    )
    .expect("slot bytes");
    assert_eq!(bytes, next.canonical_bytes().expect("canonical").as_ref());
    assert!(!String::from_utf8_lossy(&bytes).contains("private-sentinel"));

    #[cfg(unix)]
    {
        let root_mode = fs::metadata(&fixture.directory)
            .expect("root metadata")
            .permissions()
            .mode()
            & 0o777;
        let slot_mode = fs::metadata(
            fixture
                .directory
                .join(PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3),
        )
        .expect("slot metadata")
        .permissions()
        .mode()
            & 0o777;
        assert_eq!(root_mode, 0o700);
        assert_eq!(slot_mode, 0o600);
    }
}

#[test]
fn stale_temporary_is_quarantined_but_committed_corruption_blocks_restore() {
    let fixture = Fixture::new("corruption");
    let store = ProviderCaptureStoreV3::open(&fixture.directory).expect("store");
    store.reserve_sequence_lease().expect("lease");
    fs::write(
        fixture
            .directory
            .join(format!(".{PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3}.new")),
        b"interrupted-write",
    )
    .expect("stale temporary");
    drop(store);
    let restarted = ProviderCaptureStoreV3::open(&fixture.directory).expect("restart");
    let restored = restarted.restore().expect("stale recovery");
    assert_eq!(restored.quarantined_files().len(), 1);

    fs::write(
        fixture
            .directory
            .join(PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3),
        b"corrupt-committed-index",
    )
    .expect("corrupt slot");
    assert!(matches!(
        restarted.restore(),
        Err(ProviderCaptureStoreErrorV3::CommittedSlotCorrupt)
    ));
}

#[test]
fn live_reader_never_quarantines_an_inflight_writer_temporary() {
    let fixture = Fixture::new("concurrent-reader");
    let writer = ProviderCaptureStoreV3::open(&fixture.directory).expect("writer");
    writer.reserve_sequence_lease().expect("lease");
    let reader = ProviderCaptureStoreReaderV3::open(&fixture.directory).expect("reader");
    let temporary = fixture
        .directory
        .join(format!(".{PROVIDER_CAPTURE_STORE_SLOT_B_FILE_V3}.new"));
    fs::write(&temporary, b"inflight-publication").expect("temporary");

    let restored = reader.restore().expect("live restore");
    assert!(restored.quarantined_files().is_empty());
    assert!(temporary.exists());
}

fn receipt(
    sequence: u64,
    epoch: Sha256CommitmentV3,
    label: &str,
) -> nando_operator_learning::ProviderRequestCaptureReceiptV3 {
    seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
        capture_sequence: sequence,
        capture_epoch_root: epoch,
        lineage_root_sha256: root(&format!("lineage-{label}")),
        request_root_sha256: root(&format!("request-{label}")),
        projection: RuntimeProjectionV3::Responses,
        streaming: true,
        observed_at_unix_ms: 1_750_000_000_000 + sequence,
    })
    .expect("receipt")
}

fn root(label: &str) -> Sha256CommitmentV3 {
    Sha256CommitmentV3::digest_bytes(label.as_bytes())
}
