use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nando_response_actor::{
    BindingSupportCaptureBatchV1, BindingSupportCaptureOwnerV1, CaptureCommitmentIndex,
};
use sha2::{Digest, Sha256};

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let capture_index_path = args.next().ok_or_else(usage)?;
    let support_batch_path = args.next().ok_or_else(usage)?;
    let freeze_path = args.next().ok_or_else(usage)?;
    let watermark_path = args.next().ok_or_else(usage)?;
    let report_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }

    let capture_index_bytes = fs::read(&capture_index_path).map_err(|error| {
        format!(
            "capture_index_read:{}:{error}",
            capture_index_path.display()
        )
    })?;
    let source_capture_index_file_sha256 = sha256_bytes(&capture_index_bytes);
    let capture_index: CaptureCommitmentIndex = serde_cbor::from_slice(&capture_index_bytes)
        .map_err(|error| format!("capture_index_decode:{error}"))?;
    let support_batch_bytes = fs::read(&support_batch_path).map_err(|error| {
        format!(
            "support_batch_read:{}:{error}",
            support_batch_path.display()
        )
    })?;
    let support_batch: BindingSupportCaptureBatchV1 = serde_json::from_slice(&support_batch_bytes)
        .map_err(|error| format!("support_batch_decode:{error}"))?;
    support_batch
        .validate()
        .map_err(|error| format!("support_batch_validate:{error:?}"))?;

    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index)
        .map_err(|error| format!("capture_owner_open:{error:?}"))?;
    owner
        .capture_batch(support_batch)
        .map_err(|error| format!("support_capture:{error:?}"))?;
    let freeze = owner
        .freeze()
        .map_err(|error| format!("support_freeze:{error:?}"))?;

    // Seal only the exact snapshot opened above. Concurrent capture forces a
    // retry instead of moving the support/future boundary after review.
    let capture_index_after = fs::read(&capture_index_path).map_err(|error| {
        format!(
            "capture_index_reread:{}:{error}",
            capture_index_path.display()
        )
    })?;
    if capture_index_after != capture_index_bytes {
        return Err("capture_index_changed_during_freeze".to_owned());
    }

    let freeze_bytes = freeze
        .canonical_bytes()
        .map_err(|error| format!("support_freeze_encode:{error:?}"))?;
    let watermark_bytes = freeze
        .watermark_canonical_bytes()
        .map_err(|error| format!("support_watermark_encode:{error:?}"))?;
    let report = serde_json::json!({
        "source_capture_index_file_sha256": source_capture_index_file_sha256,
        "support_batch_file_sha256": sha256_bytes(&support_batch_bytes),
        "freeze_file_sha256": sha256_bytes(&freeze_bytes),
        "watermark_file_sha256": sha256_bytes(&watermark_bytes),
        "freeze": freeze.report(),
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("support_report_encode:{error}"))?;

    write_new_sync(&freeze_path, &freeze_bytes)?;
    write_new_sync(&watermark_path, &watermark_bytes)?;
    write_new_sync(&report_path, &report_bytes)?;
    Ok(())
}

fn write_new_sync(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("output_dir_create:{}:{error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("output_create_new:{}:{error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("output_write:{}:{error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("output_sync:{}:{error}", path.display()))?;
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn usage() -> String {
    "usage: nando-binding-support-capture-owner <capture-index.cbor> <support-batch.json> <freeze.json> <watermark.json> <report.json>".to_owned()
}
