use nando_operator_kernel::sha256_bytes;

use super::*;

fn topology(label: &str, captured_at_unix_ms: Option<u64>) -> Ms3ReceiptTopologyObservationV1 {
    Ms3ReceiptTopologyObservationV1 {
        topology_root_sha256: sha256_bytes(format!("topology-{label}").as_bytes()),
        request_event_id_sha256: sha256_bytes(format!("request-{label}").as_bytes()),
        captured_at_unix_ms,
    }
}

#[test]
fn young_uncovered_topology_is_in_flight() {
    let row = topology("young", Some(1_010_000));
    let report = build_ms3_receipt_health_report_v1(1_309, false, &[row], &BTreeSet::new());

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::InFlight);
    assert_eq!(report.in_flight_rows, 1);
    assert_eq!(report.stalled_rows, 0);
    assert!(!report.operational_repair_allowed);
    assert!(report.denominator_preserved);
    assert!(!report.negative_evidence_allowed);
}

#[test]
fn uncovered_topology_crosses_receipt_stall_slo() {
    let row = topology("stalled", Some(1_010_000));
    let report = build_ms3_receipt_health_report_v1(1_310, false, &[row], &BTreeSet::new());

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::ReceiptStalled);
    assert_eq!(report.blocker, "RECEIPT_STALLED");
    assert_eq!(report.stalled_rows, 1);
    assert_eq!(report.oldest_uncovered_lag_seconds, 300);
    assert!(report.operational_repair_allowed);
    assert!(report.scientific_verdict_unchanged);
}

#[test]
fn matching_terminal_completes_the_exact_request() {
    let covered = topology("covered", Some(1_010_000));
    let other = topology("other", Some(1_010_000));
    let terminal_request_ids = BTreeSet::from([covered.request_event_id_sha256.clone()]);
    let report =
        build_ms3_receipt_health_report_v1(1_020, false, &[covered, other], &terminal_request_ids);

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::InFlight);
    assert_eq!(report.topology_rows, 2);
    assert_eq!(report.terminal_receipt_rows, 1);
    assert_eq!(report.in_flight_rows, 1);
}

#[test]
fn all_exact_requests_with_terminals_are_complete() {
    let row = topology("complete", Some(1_010_000));
    let terminal_request_ids = BTreeSet::from([row.request_event_id_sha256.clone()]);
    let report = build_ms3_receipt_health_report_v1(2_000, false, &[row], &terminal_request_ids);

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::Complete);
    assert_eq!(report.terminal_receipt_rows, 1);
    assert_eq!(report.in_flight_rows, 0);
    assert_eq!(report.stalled_rows, 0);
}

#[test]
fn missing_capture_time_is_unavailable_not_stalled() {
    let report = build_ms3_receipt_health_report_v1(
        2_000,
        false,
        &[topology("unknown-time", None)],
        &BTreeSet::new(),
    );

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::EvidenceUnavailable);
    assert_eq!(report.timestamp_missing_rows, 1);
    assert!(!report.operational_repair_allowed);
}

#[test]
fn closed_acquisition_does_not_reopen_operational_repair() {
    let report = build_ms3_receipt_health_report_v1(
        2_000,
        true,
        &[topology("closed", Some(1_000_000))],
        &BTreeSet::new(),
    );

    assert_eq!(report.status, Ms3ReceiptHealthStatusV1::AcquisitionClosed);
    assert!(!report.operational_repair_allowed);
}
