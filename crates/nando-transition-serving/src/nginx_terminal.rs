use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::multi_source::TransportTerminalReceiptV1;
use serde_json::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const MAX_TERMINAL_LOG_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TERMINAL_RECEIPTS: usize = 16_384;

pub(super) fn load_transport_terminal_receipts_v1(
    path: &Path,
) -> Result<Vec<TransportTerminalReceiptV1>, String> {
    let mut file = File::open(path).map_err(|error| format!("nginx_terminal_open:{error}"))?;
    let len = file
        .metadata()
        .map_err(|error| format!("nginx_terminal_metadata:{error}"))?
        .len();
    let start = len.saturating_sub(MAX_TERMINAL_LOG_BYTES);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| format!("nginx_terminal_seek:{error}"))?;
    let mut bytes =
        Vec::with_capacity(usize::try_from(len.saturating_sub(start)).unwrap_or(64 * 1024 * 1024));
    file.take(MAX_TERMINAL_LOG_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("nginx_terminal_read:{error}"))?;
    if start > 0
        && let Some(newline) = bytes.iter().position(|byte| *byte == b'\n')
    {
        bytes.drain(..=newline);
    }

    let mut by_request = BTreeMap::new();
    for line in BufReader::new(bytes.as_slice()).lines() {
        let Ok(line) = line else {
            continue;
        };
        let Some(receipt) = parse_terminal_line(&line) else {
            continue;
        };
        by_request.insert(receipt.request_event_id_sha256.clone(), receipt);
    }
    let mut receipts = by_request.into_values().collect::<Vec<_>>();
    receipts.sort_by_key(|receipt| {
        (
            receipt.started_at_unix_nanos,
            receipt.request_event_id_sha256.clone(),
        )
    });
    if receipts.len() > MAX_TERMINAL_RECEIPTS {
        receipts.drain(..receipts.len() - MAX_TERMINAL_RECEIPTS);
    }
    Ok(receipts)
}

fn parse_terminal_line(line: &str) -> Option<TransportTerminalReceiptV1> {
    let row = serde_json::from_str::<Value>(line).ok()?;
    if row.get("schema")?.as_str()? != "nando.nginx-terminal.v1" {
        return None;
    }
    let request_id = row.get("request_id")?.as_str()?;
    if request_id.is_empty() || request_id.len() > 256 {
        return None;
    }
    let status = u16::try_from(row.get("status")?.as_u64()?).ok()?;
    let completed_at_unix_nanos = row
        .get("completed_at_unix_seconds")
        .and_then(Value::as_str)
        .and_then(decimal_seconds_to_nanos)
        .or_else(|| {
            row.get("timestamp")
                .and_then(Value::as_str)
                .and_then(rfc3339_to_nanos)
        })?;
    let request_duration_nanos = row
        .get("request_time_seconds")
        .and_then(Value::as_str)
        .and_then(decimal_seconds_to_nanos)
        .or_else(|| {
            row.get("request_time")
                .and_then(Value::as_f64)
                .and_then(float_seconds_to_nanos)
        })?;
    let started_at_unix_nanos = completed_at_unix_nanos.checked_sub(request_duration_nanos)?;
    TransportTerminalReceiptV1::seal(
        sha256_bytes(request_id.as_bytes()),
        started_at_unix_nanos,
        completed_at_unix_nanos,
        status,
    )
    .ok()
}

fn decimal_seconds_to_nanos(value: &str) -> Option<u64> {
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, ""));
    let seconds = seconds.parse::<u64>().ok()?;
    if fraction.len() > 9 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut nanos = fraction.parse::<u64>().unwrap_or(0);
    for _ in fraction.len()..9 {
        nanos = nanos.checked_mul(10)?;
    }
    seconds.checked_mul(1_000_000_000)?.checked_add(nanos)
}

fn float_seconds_to_nanos(value: f64) -> Option<u64> {
    (value.is_finite() && value >= 0.0 && value <= (u64::MAX as f64 / 1_000_000_000.0))
        .then(|| (value * 1_000_000_000.0).round() as u64)
}

fn rfc3339_to_nanos(value: &str) -> Option<u64> {
    let nanos = OffsetDateTime::parse(value, &Rfc3339)
        .ok()?
        .unix_timestamp_nanos();
    u64::try_from(nanos).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_precise_terminal_receipt() {
        let receipt = parse_terminal_line(
            r#"{"schema":"nando.nginx-terminal.v1","request_id":"request-a","status":200,"completed_at_unix_seconds":"1000.250","request_time_seconds":"0.125"}"#,
        )
        .expect("receipt");

        assert_eq!(receipt.started_at_unix_nanos, 1_000_125_000_000);
        assert_eq!(receipt.completed_at_unix_nanos, 1_000_250_000_000);
        assert!(receipt.validate());
    }

    #[test]
    fn parses_legacy_rfc3339_terminal_receipt_conservatively() {
        let receipt = parse_terminal_line(
            r#"{"schema":"nando.nginx-terminal.v1","request_id":"request-a","timestamp":"2026-07-26T05:23:14+03:00","status":200,"request_time":8.037}"#,
        )
        .expect("receipt");

        assert_eq!(
            receipt.completed_at_unix_nanos - receipt.started_at_unix_nanos,
            8_037_000_000
        );
        assert!(receipt.validate());
    }
}
