use std::{fs, path::PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_learning::{
    GenerationShadowReceiptInputV3, GenerationShadowReceiptLedgerV3,
    GenerationShadowTerminalOutcomeV3, ProviderCaptureIndexV3, ProviderRequestCaptureInputV3,
    ProviderRequestCaptureReceiptV3, seal_provider_request_capture_v3,
};
use nando_operator_persistence::{
    GENERATION_SHADOW_STORE_SLOT_A_FILE_V3, GenerationShadowReceiptStoreV3,
    GenerationShadowStoreErrorV3,
};

struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "nando-f8b-{label}-{}-{}",
            std::process::id(),
            root(label)
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
fn shadow_ledger_publish_is_private_monotonic_and_restart_exact() {
    let fixture = Fixture::new("restart");
    let store = GenerationShadowReceiptStoreV3::open(&fixture.directory).expect("store");
    let (index, captures) = captures(2);
    let mut ledger = ledger();
    append(&mut ledger, &index, &captures[0], "traffic-a");
    let first = store.publish(&ledger).expect("first publish");
    assert_eq!(first.publish_sequence(), 1);
    append(&mut ledger, &index, &captures[1], "traffic-b");
    let second = store.publish(&ledger).expect("second publish");
    assert_eq!(second.publish_sequence(), 2);

    let restarted = GenerationShadowReceiptStoreV3::open(&fixture.directory).expect("restart");
    let restored = restarted.restore().expect("restore");
    assert_eq!(restored.ledger(), Some(&ledger));
    assert!(!restored.execution_authority());
    assert_eq!(
        restored.ledger().expect("ledger").raw_payloads_persisted(),
        0
    );

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
                .join(GENERATION_SHADOW_STORE_SLOT_A_FILE_V3),
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
fn shadow_store_blocks_rollback_and_foreign_generation() {
    let fixture = Fixture::new("rollback");
    let store = GenerationShadowReceiptStoreV3::open(&fixture.directory).expect("store");
    let (index, captures) = captures(2);
    let mut first = ledger();
    append(&mut first, &index, &captures[0], "traffic-a");
    store.publish(&first).expect("publish");
    assert_eq!(
        store.publish(&first),
        Err(GenerationShadowStoreErrorV3::EvidenceRollback)
    );

    let mut foreign = GenerationShadowReceiptLedgerV3::new(root("foreign"), 9, root("checkpoint"))
        .expect("foreign");
    append(&mut foreign, &index, &captures[1], "traffic-b");
    assert_eq!(
        store.publish(&foreign),
        Err(GenerationShadowStoreErrorV3::ForeignGeneration)
    );
}

fn ledger() -> GenerationShadowReceiptLedgerV3 {
    GenerationShadowReceiptLedgerV3::new(root("generation"), 9, root("checkpoint")).expect("ledger")
}

fn append(
    ledger: &mut GenerationShadowReceiptLedgerV3,
    index: &ProviderCaptureIndexV3,
    capture: &ProviderRequestCaptureReceiptV3,
    traffic: &str,
) {
    let generation_id_sha256 = ledger.generation_id_sha256().to_owned();
    ledger
        .append(
            index,
            GenerationShadowReceiptInputV3 {
                capture_receipt: capture,
                traffic_receipt_sha256: &root(traffic),
                traffic_generation_sequence: 3,
                traffic_generation_id_sha256: &generation_id_sha256,
                traffic_index_sha256: &root("index"),
                traffic_request_sha256: &capture.request_root_sha256().to_hex(),
                traffic_verdict_code: 2,
                traffic_phase_report_sha256: None,
                traffic_operator_receipt_sha256: None,
                phase_control_evidence: None,
                f6_receipt: None,
                outcome: GenerationShadowTerminalOutcomeV3::Censored,
                parity_mismatch: false,
            },
        )
        .expect("append");
}

fn captures(count: usize) -> (ProviderCaptureIndexV3, Vec<ProviderRequestCaptureReceiptV3>) {
    let (reserved, lease) = ProviderCaptureIndexV3::empty()
        .expect("empty")
        .reserve_next_lease()
        .expect("lease");
    let receipts = (1..=count)
        .map(|sequence| {
            seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
                capture_sequence: sequence as u64,
                capture_epoch_root: lease.epoch_root_sha256(),
                lineage_root_sha256: Sha256CommitmentV3::digest_bytes(
                    format!("lineage-{sequence}").as_bytes(),
                ),
                request_root_sha256: Sha256CommitmentV3::digest_bytes(
                    format!("request-{sequence}").as_bytes(),
                ),
                projection: RuntimeProjectionV3::Responses,
                streaming: true,
                observed_at_unix_ms: 1_750_000_000_000 + sequence as u64,
            })
            .expect("receipt")
        })
        .collect::<Vec<_>>();
    let index = reserved.append_batch(&receipts).expect("append index");
    (index, receipts)
}

fn root(label: &str) -> String {
    Sha256CommitmentV3::digest_bytes(label.as_bytes()).to_hex()
}
