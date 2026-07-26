use std::collections::BTreeSet;

use nando_operator_kernel::sha256_bytes;
use nando_operator_learning::{OpportunityIntentAuditRowV1, ReducibilityClass};

use super::*;
use crate::ms3_receipt_health::{Ms3ReceiptHealthStatusV1, build_ms3_receipt_health_report_v1};

fn contract() -> Ms3LinkedFrameAcquisitionContractV1 {
    Ms3LinkedFrameAcquisitionContractV1::seal(sha256_bytes(b"prefix"), 10, 1_000, 256, 86_400)
        .expect("contract")
}

fn ordinary(observed_at_unix: u64, input_tokens: u64) -> OpportunityIntentAuditRowV1 {
    OpportunityIntentAuditRowV1 {
        intent_sha256: sha256_bytes(format!("intent-{observed_at_unix}").as_bytes()),
        input_tokens,
        class: ReducibilityClass::UnexploredMultiSource,
        verifier_available: false,
        observed_at_unix,
        authority_observed: true,
    }
}

fn receipt() -> Ms3ReceiptHealthReportV1 {
    build_ms3_receipt_health_report_v1(1_000, false, &[], &BTreeSet::new())
}

#[test]
fn traffic_without_topology_crosses_operational_stall_slo() {
    let report = build_ms3_capture_health_report_v1(
        &contract(),
        1_310,
        10,
        false,
        Some(&[ordinary(1_010, 7)]),
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );

    assert_eq!(report.status, Ms3CaptureHealthStatusV1::CaptureStalled);
    assert_eq!(report.blocker, "CAPTURE_STALLED");
    assert_eq!(report.ingestion_lag_seconds, 300);
    assert!(report.operational_repair_allowed);
    assert!(report.scientific_verdict_unchanged);
    assert!(!report.phase_update_allowed);
    assert!(!report.authority_ready);
}

#[test]
fn traffic_within_slo_waits_without_declaring_scientific_failure() {
    let report = build_ms3_capture_health_report_v1(
        &contract(),
        1_309,
        10,
        false,
        Some(&[ordinary(1_010, 11)]),
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );

    assert_eq!(report.status, Ms3CaptureHealthStatusV1::WaitingForTopology);
    assert_eq!(report.blocker, "topology_ingestion_pending");
    assert!(!report.operational_repair_allowed);
}

#[test]
fn any_post_watermark_topology_is_capture_progress() {
    let report = build_ms3_capture_health_report_v1(
        &contract(),
        2_000,
        11,
        false,
        Some(&[ordinary(1_010, 13)]),
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );

    assert_eq!(report.status, Ms3CaptureHealthStatusV1::CaptureProgress);
    assert_eq!(report.topology_delta_rows, 1);
    assert_eq!(report.blocker, "");
}

#[test]
fn pre_open_and_non_authoritative_rows_do_not_count_as_ordinary_traffic() {
    let mut non_authoritative = ordinary(1_010, 17);
    non_authoritative.authority_observed = false;
    let report = build_ms3_capture_health_report_v1(
        &contract(),
        2_000,
        10,
        false,
        Some(&[ordinary(999, 19), non_authoritative]),
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );

    assert_eq!(
        report.status,
        Ms3CaptureHealthStatusV1::WaitingForOrdinaryTraffic
    );
    assert_eq!(report.ordinary_intents_observed, 0);
}

#[test]
fn unavailable_evidence_and_closed_acquisition_are_not_capture_stalls() {
    let unavailable = build_ms3_capture_health_report_v1(
        &contract(),
        2_000,
        10,
        false,
        None,
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );
    let closed = build_ms3_capture_health_report_v1(
        &contract(),
        2_000,
        10,
        true,
        Some(&[ordinary(1_010, 23)]),
        Ms3CaptureOperationalCountersV1::default(),
        receipt(),
    );

    assert_eq!(
        unavailable.status,
        Ms3CaptureHealthStatusV1::EvidenceUnavailable
    );
    assert_eq!(closed.status, Ms3CaptureHealthStatusV1::AcquisitionClosed);
    assert!(!unavailable.operational_repair_allowed);
    assert!(!closed.operational_repair_allowed);
    assert_eq!(
        unavailable.receipt.status,
        Ms3ReceiptHealthStatusV1::WaitingForTopology
    );
}
