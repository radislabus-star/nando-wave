use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nando_response_actor::adjudicate_binding_hypotheses_v1;

fn main() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1).map(PathBuf::from);
    let preregistration = read_arg(&mut args)?;
    let b1a_report = read_arg(&mut args)?;
    let support_freeze = read_arg(&mut args)?;
    let support_watermark = read_arg(&mut args)?;
    let future_freeze = read_arg(&mut args)?;
    let future_external_receipt = read_arg(&mut args)?;
    let physical_receipts = read_arg(&mut args)?;
    let label_manifest = read_arg(&mut args)?;
    let trust_receipt = read_arg(&mut args)?;
    let output = args.next().ok_or_else(usage)?;
    if args.next().is_some() {
        return Err(usage());
    }
    let report = adjudicate_binding_hypotheses_v1(
        &preregistration,
        &b1a_report,
        &support_freeze,
        &support_watermark,
        &future_freeze,
        &future_external_receipt,
        &physical_receipts,
        &label_manifest,
        &trust_receipt,
    )
    .map_err(|error| format!("binding_adjudication:{error:?}"))?;
    write_new_sync(
        &output,
        &report
            .canonical_bytes()
            .map_err(|error| format!("adjudication_report_encode:{error:?}"))?,
    )?;
    Ok(())
}

fn read_arg(args: &mut impl Iterator<Item = PathBuf>) -> Result<Vec<u8>, String> {
    let path = args.next().ok_or_else(usage)?;
    fs::read(&path).map_err(|error| format!("input_read:{}:{error}", path.display()))
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

fn usage() -> String {
    "usage: nando-binding-hypothesis-adjudicate <preregistration.json> <b1a-report.json> <support-freeze.json> <support-watermark.json> <future-freeze.json> <future-external-receipt.json> <physical-receipts.json> <label-manifest.json> <trust-receipt.json> <adjudication-report.json>".to_owned()
}
