use serde::Serialize;
use serde_json::{Value, json};

use super::*;
use crate::{
    BindingCompletionStateV1, CaptureRecordCommitment, EvidencePolicyV1, PreActionBindingContextV1,
    PreActionBindingSurfaceV1, RawEvidenceEnvelope, canonicalize_evidence_envelope,
};

fn digest(seed: &str) -> String {
    sha256_bytes(seed.as_bytes())
}

fn frozen_graph(index: usize) -> FrozenCandidateRelationGraphV1 {
    let context = PreActionBindingContextV1 {
        call_shape_count: 1,
        capability_count: 1,
        completion_state: BindingCompletionStateV1::Unresolved,
        temporal_relation_count: 1,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256: digest(&format!("topology-{index}")),
    };
    PreActionBindingSurfaceV1::capture(
        digest(&format!("row-{index}")),
        digest(&format!("evidence-{index}")),
        "continue",
        &json!({
            "events": [{"type": "tool_result", "handle": format!("opaque-{index}")}],
            "capability": "continue_execution"
        }),
        context,
        Default::default(),
    )
    .expect("surface")
    .candidate_relation_graph(Default::default())
    .expect("graph")
    .freeze()
    .expect("freeze graph")
}

fn evidence_record(index: usize, session: usize, variant: &str) -> EvidenceLedgerRecord {
    let payload = serde_json::to_vec(&json!({
        "before": {
            "events": [{"kind": "tool_result", "opaque": format!("value-{index}-{variant}")}],
            "capabilities": ["continue_execution"]
        },
        "request": "continue"
    }))
    .expect("payload");
    let envelope = RawEvidenceEnvelope {
        source_stream_id: "b1b-support-fixture".to_owned(),
        source_offset: index as u64,
        event_id: format!("event-{index}-{variant}"),
        session_id: format!("session-{session}"),
        client_intent_id: Some(format!("intent-{index}")),
        call_id: Some(format!("call-{index}")),
        output_ordinal: Some(index as u32),
        event_time_unix_nanos: Some(1_000_000 + index as u64),
        schema_version: 1,
        payload,
    };
    let outcome = EvidenceIngestOutcome::Normalized {
        graph: canonicalize_evidence_envelope(&envelope, EvidencePolicyV1::streaming_bounded())
            .expect("canonical event"),
    };
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let sequence = 700 + index as u64;
    let previous_record_sha256 = digest(&format!("previous-{index}"));
    let record_sha256 = canonical_json_sha256(&DigestFields {
        schema: EVIDENCE_LEDGER_SCHEMA_V1,
        sequence,
        previous_record_sha256: &previous_record_sha256,
        outcome: &outcome,
    })
    .expect("record digest");
    EvidenceLedgerRecord {
        schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
        sequence,
        previous_record_sha256,
        outcome,
        record_sha256,
    }
}

fn records() -> Vec<EvidenceLedgerRecord> {
    (0..12)
        .map(|index| evidence_record(index, index / 3, "support"))
        .collect()
}

fn commitment(record: &EvidenceLedgerRecord) -> CaptureRecordCommitment {
    CaptureRecordCommitment {
        sequence: record.sequence,
        record_sha256: record.record_sha256.clone(),
    }
}

fn capture_index(records: &[EvidenceLedgerRecord]) -> CaptureCommitmentIndex {
    CaptureCommitmentIndex::new(records.iter().map(commitment).collect()).expect("capture index")
}

fn row(
    index: usize,
    record: &EvidenceLedgerRecord,
    intervention: usize,
) -> BindingSupportCaptureRowV1 {
    BindingSupportCaptureRowV1::new(
        frozen_graph(index),
        CaptureEvidenceReceipt::new(vec![commitment(record)]).expect("capture receipt"),
        record.clone(),
        format!("I{}", intervention),
    )
    .expect("support row")
}

fn freeze() -> BindingSupportFreezeV1 {
    let records = records();
    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index(&records)).expect("owner");
    for (index, record) in records.iter().enumerate() {
        owner
            .capture_support(row(index, record, index % 6 + 1))
            .expect("capture support");
    }
    owner.freeze().expect("support freeze")
}

#[test]
fn support_owner_seals_label_blind_rows_and_keeps_future_closed() {
    let report = freeze().report();
    assert_eq!(report.support_rows, 12);
    assert_eq!(report.support_session_lineages, 4);
    assert_eq!(report.intervention_rows.len(), 6);
    assert!(!report.expected_labels_joined);
    assert_eq!(report.h0_status, "UNPROVEN");
    assert_eq!(report.h1_status, "UNPROVEN");
    assert_eq!(report.future_status, "NOT_OPENED");
    assert_eq!(report.f4_status, "BLOCKED");
    assert!(!report.execution_authority);
}

#[test]
fn support_rows_have_no_label_or_post_action_serialization_surface() {
    let text = String::from_utf8(freeze().canonical_bytes().expect("freeze bytes")).expect("utf8");
    for forbidden in [
        "expected_action",
        "expected_value",
        "teacher_action",
        "post_action",
        "captured_post_freeze",
        "partition",
        "future_rows",
    ] {
        assert!(!text.contains(forbidden), "leaked field: {forbidden}");
    }
}

#[test]
fn support_freeze_restart_is_byte_identical() {
    let freeze = freeze();
    let bytes = freeze.canonical_bytes().expect("freeze bytes");
    let restored = BindingSupportFreezeV1::from_canonical_bytes(&bytes).expect("restart");
    assert_eq!(restored, freeze);
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
}

#[test]
fn row_order_does_not_change_the_freeze() {
    let records = records();
    let index = capture_index(&records);
    let mut forward = BindingSupportCaptureOwnerV1::new(index.clone()).expect("owner");
    let mut reverse = BindingSupportCaptureOwnerV1::new(index).expect("owner");
    for (row_index, record) in records.iter().enumerate() {
        forward
            .capture_support(row(row_index, record, row_index % 6 + 1))
            .expect("forward capture");
    }
    for (row_index, record) in records.iter().enumerate().rev() {
        reverse
            .capture_support(row(row_index, record, row_index % 6 + 1))
            .expect("reverse capture");
    }
    assert_eq!(forward.freeze(), reverse.freeze());
}

#[test]
fn foreign_capture_receipt_is_rejected() {
    let records = records();
    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index(&records)).expect("owner");
    let foreign = evidence_record(0, 0, "foreign");
    assert_eq!(
        owner.capture_support(row(0, &foreign, 1)),
        Err(BindingSupportCaptureErrorV1::InvalidCaptureReceipt)
    );
}

#[test]
fn missing_intervention_denominator_blocks_freeze() {
    let records = records();
    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index(&records)).expect("owner");
    for (index, record) in records.iter().enumerate() {
        owner
            .capture_support(row(index, record, index % 5 + 1))
            .expect("capture support");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingSupportCaptureErrorV1::MissingInterventionDenominator)
    );
}

#[test]
fn missing_session_lineage_denominator_blocks_freeze() {
    let records = (0..12)
        .map(|index| evidence_record(index, index / 6, "support"))
        .collect::<Vec<_>>();
    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index(&records)).expect("owner");
    for (index, record) in records.iter().enumerate() {
        owner
            .capture_support(row(index, record, index % 6 + 1))
            .expect("capture support");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingSupportCaptureErrorV1::MissingSessionLineageDenominator)
    );
}

#[test]
fn duplicate_lineage_vote_is_rejected_at_freeze() {
    let mut records = records();
    records.push(evidence_record(12, 0, "duplicate-vote"));
    let mut owner = BindingSupportCaptureOwnerV1::new(capture_index(&records)).expect("owner");
    for (index, record) in records.iter().take(12).enumerate() {
        owner
            .capture_support(row(index, record, index % 6 + 1))
            .expect("capture support");
    }
    owner
        .capture_support(row(12, &records[12], 1))
        .expect("capture duplicate vote before aggregate validation");
    assert_eq!(
        owner.freeze(),
        Err(BindingSupportCaptureErrorV1::DuplicateLineageVote)
    );
}

#[test]
fn recomputed_tampering_is_rejected_on_restart() {
    let bytes = freeze().canonical_bytes().expect("freeze bytes");
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["future_opened"] = serde_json::Value::Bool(true);
    let tampered = serde_json::to_vec(&value).expect("tampered bytes");
    assert_eq!(
        BindingSupportFreezeV1::from_canonical_bytes(&tampered),
        Err(BindingSupportCaptureErrorV1::InvalidFreezeReceipt)
    );
}

#[test]
fn watermark_is_the_exact_post_support_prefix_boundary() {
    let freeze = freeze();
    assert_eq!(freeze.watermark_next_sequence(), 712);
    assert_eq!(freeze.watermark_bytes_sha256().len(), 64);
}

#[test]
fn report_is_privacy_safe_and_deterministic() {
    let first = serde_json::to_vec(&freeze().report()).expect("report");
    let second = serde_json::to_vec(&freeze().report()).expect("report");
    assert_eq!(first, second);
    let text = String::from_utf8(first).expect("utf8");
    assert!(!text.contains("opaque-"));
    assert!(!text.contains("continue_execution"));
}

#[test]
fn frozen_graph_is_revalidated_instead_of_trusted_by_name() {
    let records = records();
    let mut graph = frozen_graph(0);
    graph.graph_root_sha256 = digest("forged-graph-root");
    assert_eq!(
        BindingSupportCaptureRowV1::new(
            graph,
            CaptureEvidenceReceipt::new(vec![commitment(&records[0])]).expect("receipt"),
            records[0].clone(),
            "I1",
        ),
        Err(BindingSupportCaptureErrorV1::InvalidFrozenGraph)
    );
}

#[test]
fn recomputed_row_cannot_forge_the_evidence_ledger_record() {
    let records = records();
    let mut forged = records[0].clone();
    forged.record_sha256 = digest("recomputed-but-foreign");
    assert_eq!(
        BindingSupportCaptureRowV1::new(
            frozen_graph(0),
            CaptureEvidenceReceipt::new(vec![commitment(&forged)]).expect("receipt"),
            forged,
            "I1",
        ),
        Err(BindingSupportCaptureErrorV1::InvalidCaptureReceipt)
    );
}

#[test]
fn checked_in_stop_b1b_s_report_matches_the_sealed_freeze() {
    let freeze_bytes =
        include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_FREEZE.json");
    let watermark_bytes =
        include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_WATERMARK.json");
    let report: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../plans/effect-law-unification-v1/STOP_B1B_S_SUPPORT_FREEZE.json"
    ))
    .expect("checked-in report");
    let freeze = BindingSupportFreezeV1::from_canonical_bytes(freeze_bytes)
        .expect("checked-in freeze must restart");
    assert_eq!(
        report.get("freeze"),
        Some(&serde_json::to_value(freeze.report()).expect("freeze report"))
    );
    let freeze_file_sha256 = sha256_bytes(freeze_bytes);
    let watermark_file_sha256 = sha256_bytes(watermark_bytes);
    assert_eq!(
        report.get("freeze_file_sha256").and_then(Value::as_str),
        Some(freeze_file_sha256.as_str())
    );
    assert_eq!(
        report.get("watermark_file_sha256").and_then(Value::as_str),
        Some(watermark_file_sha256.as_str())
    );
    assert_eq!(
        freeze.watermark_canonical_bytes().expect("watermark"),
        watermark_bytes
    );
}
