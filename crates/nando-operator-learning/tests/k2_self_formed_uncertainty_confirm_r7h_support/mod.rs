use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_learning::{
    K2UncertaintyAuthorizationSlotLedgerV1, K2UncertaintyConfirmAttemptDescriptorV1,
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptJournalV1,
    K2UncertaintyR10AuthorizationReceiptV1, composition_root_v1,
    required_r10_authorization_text_v1,
};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const TEST_SESSION_ID: &str = "019f4904-6810-74d3-9343-e7a29224a2fd";

pub(super) fn confirm_descriptor(
    root_path: &Path,
    label: &str,
) -> (
    K2UncertaintyConfirmAttemptDescriptorV1,
    K2UncertaintyR10AuthorizationReceiptV1,
) {
    let ledger_root = root_path.join(format!("ledger-{label}"));
    let receipt = authorization(label, "2026-08-16T04:12:00+03:00");
    let claim = K2UncertaintyAuthorizationSlotLedgerV1::open_or_create(&ledger_root)
        .expect("open descriptor ledger")
        .claim(&receipt, root(&format!("slot-owner-{label}")))
        .expect("claim descriptor slot");
    let descriptor = K2UncertaintyConfirmAttemptDescriptorV1::confirm(
        receipt.experiment_id_sha256.clone(),
        receipt.successor_freeze_root_sha256.clone(),
        receipt.executable_manifest_root_sha256.clone(),
        root(&format!("confirm-owner-{label}")),
        root(&format!("generator-{label}")),
        &receipt,
        &claim,
    )
    .expect("confirm descriptor");
    (descriptor, receipt)
}

pub(super) fn authorization(
    label: &str,
    authorized_at: &str,
) -> K2UncertaintyR10AuthorizationReceiptV1 {
    let successor = root(&format!("successor-{label}"));
    K2UncertaintyR10AuthorizationReceiptV1::seal(
        required_r10_authorization_text_v1(&successor).expect("authorization text"),
        TEST_SESSION_ID.to_owned(),
        authorized_at.to_owned(),
        root(&format!("experiment-{label}")),
        successor,
        root(&format!("manifest-{label}")),
    )
    .expect("authorization receipt")
}

pub(super) fn rehearsal_descriptor(label: &str) -> K2UncertaintyConfirmAttemptDescriptorV1 {
    K2UncertaintyConfirmAttemptDescriptorV1::development_rehearsal(
        root(&format!("experiment-{label}")),
        root(&format!("freeze-{label}")),
        root(&format!("manifest-{label}")),
        root(&format!("owner-{label}")),
        root(&format!("generator-{label}")),
    )
    .expect("rehearsal descriptor")
}

pub(super) fn append(
    journal: &mut K2UncertaintyConfirmAttemptJournalV1,
    kind: K2UncertaintyConfirmAttemptEventKindV1,
    sequence: usize,
) {
    journal
        .append(
            kind,
            root(&format!("owner-{sequence}")),
            root(&format!("request-{sequence}")),
            root(&format!("payload-{sequence}")),
        )
        .expect("append attempt event");
}

pub(super) fn mode(path: &Path) -> u32 {
    fs::metadata(path).expect("metadata").permissions().mode() & 0o777
}

pub(super) fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

pub(super) fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-self-formed-r7h-test.v1", label)).expect("test root")
}

pub(super) struct TestEnvironment {
    pub(super) root: PathBuf,
}

impl TestEnvironment {
    pub(super) fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-self-formed-r7h-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test root");
        Self { root }
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
