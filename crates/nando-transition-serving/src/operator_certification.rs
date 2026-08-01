use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use nando_operator_admission::{
    ExactMemoryCleanupReceiptV1, K1VocabularyGateV1, OperatorCertificationEntryV1,
    OperatorCertificationLedgerV1,
};

use crate::write_bytes_atomic;

const LEDGER_FILE: &str = "operator-certification-ledger-v1.json";
const CLEANUP_DIR: &str = "exact-memory-cleanup-receipts-v1";

pub(super) struct CertificationProjectionV1 {
    pub ledger_root_sha256: String,
    pub entry: OperatorCertificationEntryV1,
    pub k1_vocabulary_gate: K1VocabularyGateV1,
}

pub(super) fn append_entry(
    root: &Path,
    entry: OperatorCertificationEntryV1,
) -> Result<CertificationProjectionV1, String> {
    let mut ledger = restore_ledger(root)?;
    let changed = ledger.append(entry.clone()).map_err(str::to_owned)?;
    if changed {
        fs::create_dir_all(root)
            .map_err(|error| format!("operator_certification_parent_create:{error}"))?;
        let bytes = serde_json::to_vec(&ledger)
            .map_err(|error| format!("operator_certification_encode:{error}"))?;
        write_bytes_atomic(
            &root.join(LEDGER_FILE),
            &bytes,
            "operator-certification-ledger",
        )?;
        let restored = restore_ledger(root)?;
        if restored != ledger {
            return Err("operator_certification_restart_parity_mismatch".to_owned());
        }
    }
    let entry = ledger
        .latest_entries()
        .into_iter()
        .find(|candidate| candidate.package_id == entry.package_id)
        .cloned()
        .ok_or_else(|| "operator_certification_projection_missing".to_owned())?;
    Ok(CertificationProjectionV1 {
        ledger_root_sha256: ledger.ledger_root_sha256.clone(),
        entry,
        k1_vocabulary_gate: ledger.k1_vocabulary_gate().map_err(str::to_owned)?,
    })
}

pub(super) fn validate_projection(
    root: &Path,
    ledger_root_sha256: &str,
    entry: &OperatorCertificationEntryV1,
    gate: &K1VocabularyGateV1,
) -> Result<(), String> {
    let ledger = restore_ledger(root)?;
    let persisted_entry = ledger
        .latest_entries()
        .into_iter()
        .find(|candidate| candidate.package_id == entry.package_id)
        .ok_or_else(|| "operator_certification_projection_missing".to_owned())?;
    let persisted_gate = ledger.k1_vocabulary_gate().map_err(str::to_owned)?;
    if ledger.ledger_root_sha256 != ledger_root_sha256
        || persisted_entry != entry
        || &persisted_gate != gate
    {
        return Err("operator_certification_projection_binding_mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn restore_cleanup_receipt(
    root: &Path,
    bundle_id_sha256: &str,
    package_id: &str,
    candidate_root_sha256: &str,
) -> Result<Option<ExactMemoryCleanupReceiptV1>, String> {
    let path = root
        .join(CLEANUP_DIR)
        .join(format!("{bundle_id_sha256}.json"));
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("exact_memory_cleanup_receipt_read:{error}")),
    };
    let receipt: ExactMemoryCleanupReceiptV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("exact_memory_cleanup_receipt_decode:{error}"))?;
    receipt.validate().map_err(str::to_owned)?;
    if receipt.bundle_id_sha256 != bundle_id_sha256
        || receipt.package_id != package_id
        || receipt.candidate_root_sha256 != candidate_root_sha256
    {
        return Err("exact_memory_cleanup_receipt_binding_mismatch".to_owned());
    }
    Ok(Some(receipt))
}

fn restore_ledger(root: &Path) -> Result<OperatorCertificationLedgerV1, String> {
    let bytes = match fs::read(root.join(LEDGER_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return OperatorCertificationLedgerV1::empty().map_err(str::to_owned);
        }
        Err(error) => return Err(format!("operator_certification_restore_read:{error}")),
    };
    let ledger: OperatorCertificationLedgerV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("operator_certification_restore_decode:{error}"))?;
    ledger.validate().map_err(str::to_owned)?;
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use nando_operator_admission::{
        ExecutionCertificateStatusV1, ExecutionCertificateV1, LawCertificateStatusV1,
        LawCertificateV1, MechanismCertificateStatusV1, MechanismCertificateV1,
        OperatorMechanismClassV1,
    };

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn root(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn test_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "nando-operator-certification-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn entry() -> OperatorCertificationEntryV1 {
        let bundle = root('a');
        let package = "package-one";
        OperatorCertificationEntryV1::seal(
            &bundle,
            package,
            &root('b'),
            &root('c'),
            ExecutionCertificateV1::seal(
                &bundle,
                package,
                ExecutionCertificateStatusV1::Pass,
                vec![root('d')],
                "",
            )
            .expect("execution"),
            LawCertificateV1::seal(
                &bundle,
                package,
                LawCertificateStatusV1::Partial,
                vec![root('e')],
                None,
                "exact_memory_cleanup_receipt_missing",
            )
            .expect("law"),
            MechanismCertificateV1::seal(
                &bundle,
                package,
                MechanismCertificateStatusV1::Collecting,
                OperatorMechanismClassV1::Unresolved,
                vec![root('f')],
                "post_center_holdout_collecting",
            )
            .expect("mechanism"),
            0,
        )
        .expect("entry")
    }

    #[test]
    fn append_is_durable_and_idempotent() {
        let root = test_root();
        let first = append_entry(&root, entry()).expect("first append");
        let second = append_entry(&root, entry()).expect("second append");
        assert_eq!(first.ledger_root_sha256, second.ledger_root_sha256);
        validate_projection(
            &root,
            &second.ledger_root_sha256,
            &second.entry,
            &second.k1_vocabulary_gate,
        )
        .expect("projection parity");
        assert_eq!(
            validate_projection(
                &root,
                &"f".repeat(64),
                &second.entry,
                &second.k1_vocabulary_gate,
            ),
            Err("operator_certification_projection_binding_mismatch".to_owned())
        );
        assert_eq!(restore_ledger(&root).expect("restore").revision, 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
