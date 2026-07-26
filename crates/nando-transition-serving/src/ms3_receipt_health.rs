//! Read-only health of the MS3 topology-to-terminal-receipt edge.

use std::collections::BTreeSet;

use serde::Serialize;

pub(super) const MS3_RECEIPT_STALL_LAG_SECONDS_V1: u64 = 300;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum Ms3ReceiptHealthStatusV1 {
    EvidenceUnavailable,
    AcquisitionClosed,
    WaitingForTopology,
    InFlight,
    Complete,
    ReceiptStalled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Ms3ReceiptTopologyObservationV1 {
    pub(super) topology_root_sha256: String,
    pub(super) request_event_id_sha256: String,
    pub(super) captured_at_unix_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct Ms3ReceiptHealthReportV1 {
    pub(super) schema: &'static str,
    pub(super) receipt_lag_slo_seconds: u64,
    pub(super) topology_rows: u64,
    pub(super) terminal_receipt_rows: u64,
    pub(super) in_flight_rows: u64,
    pub(super) stalled_rows: u64,
    pub(super) timestamp_missing_rows: u64,
    pub(super) oldest_uncovered_lag_seconds: u64,
    pub(super) oldest_uncovered_topology_root_sha256: Option<String>,
    pub(super) status: Ms3ReceiptHealthStatusV1,
    pub(super) blocker: &'static str,
    pub(super) operational_repair_allowed: bool,
    pub(super) denominator_preserved: bool,
    pub(super) negative_evidence_allowed: bool,
    pub(super) scientific_verdict_unchanged: bool,
}

pub(super) fn build_ms3_receipt_health_report_v1(
    sampled_at_unix: u64,
    acquisition_closed: bool,
    topologies: &[Ms3ReceiptTopologyObservationV1],
    terminal_request_ids: &BTreeSet<String>,
) -> Ms3ReceiptHealthReportV1 {
    let sampled_at_unix_ms = sampled_at_unix.saturating_mul(1_000);
    let mut terminal_receipt_rows = 0_u64;
    let mut in_flight_rows = 0_u64;
    let mut stalled_rows = 0_u64;
    let mut timestamp_missing_rows = 0_u64;
    let mut oldest_uncovered_lag_seconds = 0_u64;
    let mut oldest_uncovered_topology_root_sha256 = None;

    for topology in topologies {
        if terminal_request_ids.contains(&topology.request_event_id_sha256) {
            terminal_receipt_rows = terminal_receipt_rows.saturating_add(1);
            continue;
        }
        let Some(captured_at_unix_ms) = topology.captured_at_unix_ms else {
            timestamp_missing_rows = timestamp_missing_rows.saturating_add(1);
            continue;
        };
        let lag_seconds = sampled_at_unix_ms
            .saturating_sub(captured_at_unix_ms)
            .saturating_div(1_000);
        if lag_seconds >= MS3_RECEIPT_STALL_LAG_SECONDS_V1 {
            stalled_rows = stalled_rows.saturating_add(1);
        } else {
            in_flight_rows = in_flight_rows.saturating_add(1);
        }
        if oldest_uncovered_topology_root_sha256.is_none()
            || lag_seconds > oldest_uncovered_lag_seconds
        {
            oldest_uncovered_lag_seconds = lag_seconds;
            oldest_uncovered_topology_root_sha256 = Some(topology.topology_root_sha256.clone());
        }
    }

    // Receipt health is operational only. Missing or delayed receipts never
    // become semantic negatives and never alter the frozen acquisition.
    let status = if acquisition_closed {
        Ms3ReceiptHealthStatusV1::AcquisitionClosed
    } else if topologies.is_empty() {
        Ms3ReceiptHealthStatusV1::WaitingForTopology
    } else if stalled_rows > 0 {
        Ms3ReceiptHealthStatusV1::ReceiptStalled
    } else if timestamp_missing_rows > 0 {
        Ms3ReceiptHealthStatusV1::EvidenceUnavailable
    } else if in_flight_rows > 0 {
        Ms3ReceiptHealthStatusV1::InFlight
    } else {
        Ms3ReceiptHealthStatusV1::Complete
    };
    let blocker = match status {
        Ms3ReceiptHealthStatusV1::EvidenceUnavailable => "receipt_lag_timestamp_unavailable",
        Ms3ReceiptHealthStatusV1::AcquisitionClosed
        | Ms3ReceiptHealthStatusV1::WaitingForTopology
        | Ms3ReceiptHealthStatusV1::Complete => "",
        Ms3ReceiptHealthStatusV1::InFlight => "terminal_receipt_in_flight",
        Ms3ReceiptHealthStatusV1::ReceiptStalled => "RECEIPT_STALLED",
    };

    Ms3ReceiptHealthReportV1 {
        schema: "nando.ms3-receipt-health.v1",
        receipt_lag_slo_seconds: MS3_RECEIPT_STALL_LAG_SECONDS_V1,
        topology_rows: u64::try_from(topologies.len()).unwrap_or(u64::MAX),
        terminal_receipt_rows,
        in_flight_rows,
        stalled_rows,
        timestamp_missing_rows,
        oldest_uncovered_lag_seconds,
        oldest_uncovered_topology_root_sha256,
        status,
        blocker,
        operational_repair_allowed: status == Ms3ReceiptHealthStatusV1::ReceiptStalled,
        denominator_preserved: true,
        negative_evidence_allowed: false,
        scientific_verdict_unchanged: true,
    }
}

#[cfg(test)]
#[path = "ms3_receipt_health_tests.rs"]
mod tests;
