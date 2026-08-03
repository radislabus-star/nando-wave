use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use nando_operator_kernel::canonical_json_sha256;
use serde::Serialize;

use crate::operator_certification::CertificationAuthorityConfigV1;

const MAX_FILES: usize = 256;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AGE_SECONDS: u64 = 30 * 24 * 60 * 60;
const RECEIPT_SCHEMA_V1: &str = "nando.k1-pre-action-evidence-cleanup-receipt.v1";
const RECEIPT_DIR: &str = "k1-pre-action-authority-evidence-cleanup-v1";

#[derive(Serialize)]
struct CleanupReceiptV1 {
    schema: &'static str,
    terminal_verdict_root_sha256: String,
    removed_capture_roots_sha256: Vec<String>,
    removed_bytes: u64,
    remaining_files: usize,
    remaining_bytes: u64,
    max_files: usize,
    max_bytes: u64,
    max_age_seconds: u64,
    cleaned_at_unix: u64,
    receipt_root_sha256: String,
}

struct EvidenceFile {
    path: std::path::PathBuf,
    capture_root: String,
    bytes: u64,
    modified_unix: u64,
}

pub(super) fn prune_after_terminal_verdict(
    config: &CertificationAuthorityConfigV1,
    terminal_verdict_root_sha256: &str,
) -> Result<(), String> {
    prune_root(&config.root, terminal_verdict_root_sha256)
}

fn prune_root(root: &Path, terminal_verdict_root_sha256: &str) -> Result<(), String> {
    let directory = root.join("k1-pre-action-authority-evidence-v1");
    if !directory.exists() {
        return Ok(());
    }
    let now = unix_now()?;
    let mut files = fs::read_dir(&directory)
        .map_err(|error| format!("k1_pre_action_retention_read_dir:{error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|value| value.to_str()) == Some("cbor")).then_some(path)
        })
        .map(|path| {
            let metadata = fs::metadata(&path)
                .map_err(|error| format!("k1_pre_action_retention_metadata:{error}"))?;
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_secs());
            let capture_root = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| "k1_pre_action_retention_file_name_invalid".to_owned())?
                .to_owned();
            Ok(EvidenceFile {
                path,
                capture_root,
                bytes: metadata.len(),
                modified_unix,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    files.sort_by_key(|file| (file.modified_unix, file.capture_root.clone()));
    let mut remaining_bytes = files.iter().map(|file| file.bytes).sum::<u64>();
    let mut remaining_files = files.len();
    let mut removed = Vec::new();
    let mut removed_bytes = 0_u64;
    for file in files {
        let expired = now.saturating_sub(file.modified_unix) > MAX_AGE_SECONDS;
        if !expired && remaining_files <= MAX_FILES && remaining_bytes <= MAX_BYTES {
            continue;
        }
        fs::remove_file(&file.path)
            .map_err(|error| format!("k1_pre_action_retention_remove:{error}"))?;
        remaining_files = remaining_files.saturating_sub(1);
        remaining_bytes = remaining_bytes.saturating_sub(file.bytes);
        removed_bytes = removed_bytes.saturating_add(file.bytes);
        removed.push(file.capture_root);
    }
    if removed.is_empty() {
        return Ok(());
    }
    fs::File::open(&directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("k1_pre_action_retention_directory_sync:{error}"))?;
    removed.sort();
    let receipt_material = (
        RECEIPT_SCHEMA_V1,
        terminal_verdict_root_sha256,
        &removed,
        removed_bytes,
        remaining_files,
        remaining_bytes,
        MAX_FILES,
        MAX_BYTES,
        MAX_AGE_SECONDS,
        now,
    );
    let receipt_root_sha256 = canonical_json_sha256(&receipt_material)
        .map_err(|error| format!("k1_pre_action_retention_receipt_root:{error}"))?;
    let receipt = CleanupReceiptV1 {
        schema: RECEIPT_SCHEMA_V1,
        terminal_verdict_root_sha256: terminal_verdict_root_sha256.to_owned(),
        removed_capture_roots_sha256: removed,
        removed_bytes,
        remaining_files,
        remaining_bytes,
        max_files: MAX_FILES,
        max_bytes: MAX_BYTES,
        max_age_seconds: MAX_AGE_SECONDS,
        cleaned_at_unix: now,
        receipt_root_sha256,
    };
    let receipt_path = root
        .join(RECEIPT_DIR)
        .join(format!("{terminal_verdict_root_sha256}.json"));
    fs::create_dir_all(
        receipt_path
            .parent()
            .ok_or_else(|| "k1_pre_action_retention_receipt_parent_missing".to_owned())?,
    )
    .map_err(|error| format!("k1_pre_action_retention_receipt_parent:{error}"))?;
    let bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("k1_pre_action_retention_receipt_encode:{error}"))?;
    if receipt_path.exists() {
        let existing = fs::read(&receipt_path)
            .map_err(|error| format!("k1_pre_action_retention_receipt_read:{error}"))?;
        return if existing == bytes {
            Ok(())
        } else {
            Err("k1_pre_action_retention_receipt_replacement_forbidden".to_owned())
        };
    }
    crate::write_bytes_atomic(&receipt_path, &bytes, "k1-pre-action-retention-receipt")
}

fn unix_now() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("k1_pre_action_retention_clock:{error}"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use nando_operator_kernel::sha256_bytes;

    use super::{MAX_FILES, prune_root};

    #[test]
    fn terminal_cleanup_is_bounded_and_receipted() {
        let root = std::env::temp_dir().join(format!(
            "nando-k1-evidence-retention-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let evidence = root.join("k1-pre-action-authority-evidence-v1");
        fs::create_dir_all(&evidence).expect("evidence directory");
        for index in 0..=MAX_FILES {
            fs::write(
                evidence.join(format!(
                    "{}.cbor",
                    sha256_bytes(index.to_string().as_bytes())
                )),
                [0_u8; 8],
            )
            .expect("evidence file");
        }
        let terminal = sha256_bytes(b"terminal");
        prune_root(&root, &terminal).expect("terminal cleanup");
        assert_eq!(
            fs::read_dir(&evidence).expect("remaining evidence").count(),
            MAX_FILES
        );
        assert!(
            root.join("k1-pre-action-authority-evidence-cleanup-v1")
                .join(format!("{terminal}.json"))
                .is_file()
        );
        fs::remove_dir_all(root).expect("cleanup test root");
    }
}
