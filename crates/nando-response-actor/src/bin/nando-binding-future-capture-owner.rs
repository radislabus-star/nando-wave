use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use nando_response_actor::{
    BindingFutureAcquisitionProtocolV1, BindingFutureCaptureBatchV1, BindingFutureCaptureFreezeV1,
    BindingFutureCaptureOwnerV1, CaptureCommitmentIndex,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_FUTURE_BATCH_BYTES: u64 = 2_000_000;

#[derive(Serialize)]
struct ExternalFutureReceiptV1<'a> {
    schema: &'static str,
    stop_id: &'static str,
    protocol_file_sha256: String,
    protocol_receipt_sha256: &'a str,
    support_freeze_file_sha256: String,
    support_watermark_file_sha256: String,
    extended_capture_index_file_sha256: String,
    extended_capture_index_sha256: &'a str,
    future_batch_sha256: String,
    future_freeze_file_sha256: String,
    trusted_future_receipt_sha256: &'a str,
    future_rows: usize,
    expected_labels_joined: bool,
    h0_status: &'static str,
    h1_status: &'static str,
    execution_authority: bool,
}

fn main() -> Result<(), String> {
    require_pipe("/proc/self/fd/0", "future_batch_stdin_must_be_a_pipe")?;
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let protocol_path = args.next().ok_or_else(usage)?;
    let support_freeze_path = args.next().ok_or_else(usage)?;
    let support_watermark_path = args.next().ok_or_else(usage)?;
    let extended_index_path = args.next().ok_or_else(usage)?;
    let future_freeze_path = args.next().ok_or_else(usage)?;
    let external_receipt_path = args.next().ok_or_else(usage)?;
    let capture_report_path = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }

    let protocol_bytes = read_file(&protocol_path, "protocol")?;
    let protocol = BindingFutureAcquisitionProtocolV1::from_canonical_bytes(&protocol_bytes)
        .map_err(|error| format!("protocol_decode:{error:?}"))?;
    let support_freeze_bytes = read_file(&support_freeze_path, "support_freeze")?;
    let support_watermark_bytes = read_file(&support_watermark_path, "support_watermark")?;

    // EOF is the publication barrier: the producer persists the commitment
    // index before it writes this bounded batch to the pipe.
    let mut batch_bytes = Vec::new();
    std::io::stdin()
        .lock()
        .take(MAX_FUTURE_BATCH_BYTES + 1)
        .read_to_end(&mut batch_bytes)
        .map_err(|error| format!("future_batch_pipe_read:{error}"))?;
    if batch_bytes.len() as u64 > MAX_FUTURE_BATCH_BYTES {
        return Err("future_batch_byte_budget_exceeded".to_owned());
    }
    let batch = BindingFutureCaptureBatchV1::from_canonical_bytes(&batch_bytes, &protocol)
        .map_err(|error| format!("future_batch_decode:{error:?}"))?;
    let extended_index_bytes = read_file(&extended_index_path, "extended_index")?;
    let extended_index: CaptureCommitmentIndex = serde_cbor::from_slice(&extended_index_bytes)
        .map_err(|error| format!("extended_index_decode:{error}"))?;

    let mut owner = BindingFutureCaptureOwnerV1::new(
        protocol.clone(),
        &support_freeze_bytes,
        &support_watermark_bytes,
        extended_index.clone(),
    )
    .map_err(|error| format!("future_owner_open:{error:?}"))?;
    for input in batch.into_rows() {
        owner
            .capture_future(input)
            .map_err(|error| format!("future_capture:{error:?}"))?;
    }
    let freeze = owner
        .freeze()
        .map_err(|error| format!("future_freeze:{error:?}"))?;

    verify_unchanged(&protocol_path, &protocol_bytes, "protocol")?;
    verify_unchanged(
        &support_freeze_path,
        &support_freeze_bytes,
        "support_freeze",
    )?;
    verify_unchanged(
        &support_watermark_path,
        &support_watermark_bytes,
        "support_watermark",
    )?;
    verify_unchanged(
        &extended_index_path,
        &extended_index_bytes,
        "extended_index",
    )?;

    let freeze_bytes = freeze
        .canonical_bytes()
        .map_err(|error| format!("future_freeze_encode:{error:?}"))?;
    BindingFutureCaptureFreezeV1::from_canonical_bytes(
        &freeze_bytes,
        freeze.receipt_sha256(),
        &support_freeze_bytes,
        &support_watermark_bytes,
    )
    .map_err(|error| format!("future_freeze_restart:{error:?}"))?;
    let external_receipt = ExternalFutureReceiptV1 {
        schema: "nando.binding-future-external-receipt.v1",
        stop_id: "STOP-B1B-F",
        protocol_file_sha256: sha256_bytes(&protocol_bytes),
        protocol_receipt_sha256: &protocol.receipt_sha256,
        support_freeze_file_sha256: sha256_bytes(&support_freeze_bytes),
        support_watermark_file_sha256: sha256_bytes(&support_watermark_bytes),
        extended_capture_index_file_sha256: sha256_bytes(&extended_index_bytes),
        extended_capture_index_sha256: &extended_index.index_sha256,
        future_batch_sha256: sha256_bytes(&batch_bytes),
        future_freeze_file_sha256: sha256_bytes(&freeze_bytes),
        trusted_future_receipt_sha256: freeze.receipt_sha256(),
        future_rows: freeze.report().future_rows,
        expected_labels_joined: false,
        h0_status: "UNPROVEN",
        h1_status: "UNPROVEN",
        execution_authority: false,
    };
    let external_receipt_bytes = serde_json::to_vec_pretty(&external_receipt)
        .map_err(|error| format!("external_receipt_encode:{error}"))?;
    let report = serde_json::json!({
        "schema": "nando.binding-future-capture-owner-report.v1",
        "stage": "STOP-B1B-F",
        "protocol_file_sha256": sha256_bytes(&protocol_bytes),
        "support_freeze_file_sha256": sha256_bytes(&support_freeze_bytes),
        "support_watermark_file_sha256": sha256_bytes(&support_watermark_bytes),
        "extended_capture_index_file_sha256": sha256_bytes(&extended_index_bytes),
        "future_batch_sha256": sha256_bytes(&batch_bytes),
        "future_freeze_file_sha256": sha256_bytes(&freeze_bytes),
        "external_receipt_file_sha256": sha256_bytes(&external_receipt_bytes),
        "freeze": freeze.report(),
        "raw_payload_persisted": false,
        "trusted_labels_joined": false,
        "adjudication_status": "NOT_STARTED",
        "f4_status": "BLOCKED",
        "execution_authority": false,
    });
    let report_bytes =
        serde_json::to_vec_pretty(&report).map_err(|error| format!("report_encode:{error}"))?;

    write_new_sync(&future_freeze_path, &freeze_bytes)?;
    write_new_sync(&external_receipt_path, &external_receipt_bytes)?;
    write_new_sync(&capture_report_path, &report_bytes)?;
    Ok(())
}

fn read_file(path: &Path, kind: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{kind}_read:{}:{error}", path.display()))
}

fn verify_unchanged(path: &Path, expected: &[u8], kind: &str) -> Result<(), String> {
    if read_file(path, kind)? != expected {
        return Err(format!("{kind}_changed_during_capture"));
    }
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

#[cfg(unix)]
fn require_pipe(fd_path: &str, error: &str) -> Result<(), String> {
    let file_type = fs::metadata(fd_path)
        .map_err(|source| format!("pipe_metadata:{fd_path}:{source}"))?
        .file_type();
    if !file_type.is_fifo() && !file_type.is_socket() {
        return Err(error.to_owned());
    }
    Ok(())
}

#[cfg(not(unix))]
fn require_pipe(_fd_path: &str, _error: &str) -> Result<(), String> {
    Err("future_batch_pipe_type_check_unsupported".to_owned())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn usage() -> String {
    "usage: nando-binding-future-capture-owner <protocol.json> <support-freeze.json> <support-watermark.json> <extended-index.cbor> <future-freeze.json> <external-receipt.json> <capture-report.json>"
        .to_owned()
}
