//! Read-only operational guard beside the immutable MS3 acquisition proof.

use nando_operator_learning::{
    OpportunityIntentAuditRowV1, multi_source::Ms3LinkedFrameAcquisitionContractV1,
};
use serde::Serialize;

use crate::ms3_receipt_health::Ms3ReceiptHealthReportV1;

pub(super) const MS3_CAPTURE_STALL_LAG_SECONDS_V1: u64 = 300;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum Ms3CaptureHealthStatusV1 {
    EvidenceUnavailable,
    AcquisitionClosed,
    WaitingForOrdinaryTraffic,
    WaitingForTopology,
    CaptureProgress,
    CaptureStalled,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Ms3CaptureOperationalCountersV1 {
    pub(super) structural_pending_records: u64,
    pub(super) structural_consumer_failures: u64,
    pub(super) structural_sequence_gaps: u64,
    pub(super) opportunity_pending_events: u64,
    pub(super) opportunity_consumer_failures: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Ms3CaptureTopologyProgressV1<'a> {
    pub(super) current_rows: u64,
    pub(super) observed_at_unix: &'a [u64],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct Ms3CaptureHealthReportV1 {
    pub(super) schema: &'static str,
    pub(super) sampled_at_unix: u64,
    pub(super) acquisition_contract_root_sha256: String,
    pub(super) acquisition_opened_at_unix: u64,
    pub(super) stall_lag_seconds: u64,
    pub(super) ordinary_intents_observed: u64,
    pub(super) ordinary_input_tokens_observed: u64,
    pub(super) first_ordinary_observed_at_unix: Option<u64>,
    pub(super) last_ordinary_observed_at_unix: Option<u64>,
    pub(super) topology_watermark_rows: u64,
    pub(super) current_topology_rows: u64,
    pub(super) topology_delta_rows: u64,
    pub(super) ingestion_lag_seconds: u64,
    pub(super) structural_pending_records: u64,
    pub(super) structural_consumer_failures: u64,
    pub(super) structural_sequence_gaps: u64,
    pub(super) opportunity_pending_events: u64,
    pub(super) opportunity_consumer_failures: u64,
    pub(super) receipt: Ms3ReceiptHealthReportV1,
    pub(super) status: Ms3CaptureHealthStatusV1,
    pub(super) blocker: &'static str,
    pub(super) operational_repair_allowed: bool,
    pub(super) scientific_verdict_unchanged: bool,
    pub(super) phase_update_allowed: bool,
    pub(super) authority_ready: bool,
}

pub(super) fn build_ms3_capture_health_report_v1(
    contract: &Ms3LinkedFrameAcquisitionContractV1,
    sampled_at_unix: u64,
    topology: Ms3CaptureTopologyProgressV1<'_>,
    acquisition_closed: bool,
    opportunities: Option<&[OpportunityIntentAuditRowV1]>,
    counters: Ms3CaptureOperationalCountersV1,
    receipt: Ms3ReceiptHealthReportV1,
) -> Ms3CaptureHealthReportV1 {
    let rows = opportunities.map(|rows| {
        rows.iter()
            .filter(|row| {
                row.authority_observed
                    && row.observed_at_unix >= contract.opened_at_unix
                    && row.observed_at_unix <= sampled_at_unix
            })
            .collect::<Vec<_>>()
    });
    let opportunity_intents_observed = rows
        .as_ref()
        .map_or(0, |rows| u64::try_from(rows.len()).unwrap_or(u64::MAX));
    let topology_delta_rows = topology
        .current_rows
        .saturating_sub(contract.topology_watermark_rows);
    let ordinary_intents_observed = opportunity_intents_observed.max(topology_delta_rows);
    let ordinary_input_tokens_observed = rows.as_ref().map_or(0, |rows| {
        rows.iter()
            .fold(0_u64, |total, row| total.saturating_add(row.input_tokens))
    });
    let opportunity_first_observed_at_unix = rows
        .as_ref()
        .and_then(|rows| rows.iter().map(|row| row.observed_at_unix).min());
    let opportunity_last_observed_at_unix = rows
        .as_ref()
        .and_then(|rows| rows.iter().map(|row| row.observed_at_unix).max());
    let topology_first_observed_at_unix = topology.observed_at_unix.iter().copied().min();
    let topology_last_observed_at_unix = topology.observed_at_unix.iter().copied().max();
    let first_ordinary_observed_at_unix = [
        opportunity_first_observed_at_unix,
        topology_first_observed_at_unix,
    ]
    .into_iter()
    .flatten()
    .min();
    let last_ordinary_observed_at_unix = [
        opportunity_last_observed_at_unix,
        topology_last_observed_at_unix,
    ]
    .into_iter()
    .flatten()
    .max();
    let ingestion_lag_seconds =
        first_ordinary_observed_at_unix.map_or(0, |first| sampled_at_unix.saturating_sub(first));

    // This guard diagnoses transport/capture reachability only. It cannot
    // mutate, complete, or overrule the frozen scientific acquisition.
    let status = if opportunities.is_none() && topology_delta_rows == 0 {
        Ms3CaptureHealthStatusV1::EvidenceUnavailable
    } else if acquisition_closed {
        Ms3CaptureHealthStatusV1::AcquisitionClosed
    } else if ordinary_intents_observed == 0 {
        Ms3CaptureHealthStatusV1::WaitingForOrdinaryTraffic
    } else if topology_delta_rows > 0 {
        Ms3CaptureHealthStatusV1::CaptureProgress
    } else if ingestion_lag_seconds >= MS3_CAPTURE_STALL_LAG_SECONDS_V1 {
        Ms3CaptureHealthStatusV1::CaptureStalled
    } else {
        Ms3CaptureHealthStatusV1::WaitingForTopology
    };
    let blocker = match status {
        Ms3CaptureHealthStatusV1::EvidenceUnavailable => "ordinary_evidence_unavailable",
        Ms3CaptureHealthStatusV1::AcquisitionClosed => "",
        Ms3CaptureHealthStatusV1::WaitingForOrdinaryTraffic => "ordinary_traffic_pending",
        Ms3CaptureHealthStatusV1::WaitingForTopology => "topology_ingestion_pending",
        Ms3CaptureHealthStatusV1::CaptureProgress => "",
        Ms3CaptureHealthStatusV1::CaptureStalled => "CAPTURE_STALLED",
    };

    Ms3CaptureHealthReportV1 {
        schema: "nando.ms3-capture-health.v1",
        sampled_at_unix,
        acquisition_contract_root_sha256: contract.contract_root_sha256.clone(),
        acquisition_opened_at_unix: contract.opened_at_unix,
        stall_lag_seconds: MS3_CAPTURE_STALL_LAG_SECONDS_V1,
        ordinary_intents_observed,
        ordinary_input_tokens_observed,
        first_ordinary_observed_at_unix,
        last_ordinary_observed_at_unix,
        topology_watermark_rows: contract.topology_watermark_rows,
        current_topology_rows: topology.current_rows,
        topology_delta_rows,
        ingestion_lag_seconds,
        structural_pending_records: counters.structural_pending_records,
        structural_consumer_failures: counters.structural_consumer_failures,
        structural_sequence_gaps: counters.structural_sequence_gaps,
        opportunity_pending_events: counters.opportunity_pending_events,
        opportunity_consumer_failures: counters.opportunity_consumer_failures,
        receipt,
        status,
        blocker,
        operational_repair_allowed: status == Ms3CaptureHealthStatusV1::CaptureStalled,
        scientific_verdict_unchanged: true,
        phase_update_allowed: false,
        authority_ready: false,
    }
}

#[cfg(test)]
#[path = "ms3_capture_health_tests.rs"]
mod tests;
