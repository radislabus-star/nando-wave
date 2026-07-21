use serde::Serialize;
use serde_json::{Value, json};

use super::*;
use crate::{
    CaptureRecordCommitment, EvidencePolicyV1, RawEvidenceEnvelope, canonicalize_evidence_envelope,
};

const SUPPORT_FREEZE_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_FREEZE.json");
const SUPPORT_WATERMARK_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_S_WATERMARK.json");
const FUTURE_PROTOCOL_BYTES: &[u8] = include_bytes!(
    "../../../plans/effect-law-unification-v1/STOP_B1B_F0_FUTURE_ACQUISITION_PROTOCOL.json"
);
const FUTURE_FREEZE_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_FREEZE.json");
const FUTURE_EXTERNAL_RECEIPT_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_EXTERNAL_RECEIPT.json");
const FUTURE_CAPTURE_REPORT_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_CAPTURE_REPORT.json");
const FUTURE_ACQUISITION_REPORT_BYTES: &[u8] =
    include_bytes!("../../../plans/effect-law-unification-v1/STOP_B1B_F_ACQUISITION_REPORT.json");
const FUTURE_FREEZE_FILE_SHA256: &str =
    "b0ae6f325ff9a5f910d9875d5430f95d5092228e371045db5d49eb365fe88717";
const FUTURE_EXTERNAL_RECEIPT_FILE_SHA256: &str =
    "3534a8227439f7db0b6611f369a3d97ca59abe5eb2cc7beb98e6c05335e4b1d1";
const FUTURE_CAPTURE_REPORT_FILE_SHA256: &str =
    "f390c33db831f30cac576753d8a174af37d7eda56dfb1e314331b0ec8aab5039";
const FUTURE_ACQUISITION_REPORT_FILE_SHA256: &str =
    "8bb8da1246339f893ff96afe4de5669080f717bc395535e3d081b217a3f2e85c";

#[derive(Clone, Copy, Default)]
struct FixtureOptions {
    collapse_shapes: bool,
    remove_trap: bool,
    overlap_support_session: bool,
    split_session_slot: bool,
}

fn digest(seed: &str) -> String {
    sha256_bytes(seed.as_bytes())
}

fn future_payload(index: usize, options: FixtureOptions) -> Value {
    let intervention = index % 6 + 1;
    let replicate = index / 6;
    let suffix = if options.remove_trap {
        format!("{intervention}-{replicate}")
    } else {
        intervention.to_string()
    };
    let values = [
        format!("future-capability-{suffix}-left"),
        format!("future-capability-{suffix}-right"),
        format!("future-parent-{suffix}"),
        "active".to_owned(),
    ];
    let shape = if options.collapse_shapes {
        0
    } else {
        index % 4
    };
    match shape {
        0 => json!({
            "future_outer_alpha": [{
                "future_value_alpha": values[0],
                "future_peer_alpha": values[1]
            }],
            "future_relation_alpha": {
                "future_parent_alpha": values[2],
                "future_state_alpha": values[3]
            }
        }),
        1 => json!([{
            "future_values_beta": [values[0], values[1]],
            "future_parent_beta": values[2],
            "future_state_beta": values[3]
        }]),
        2 => json!({
            "future_envelope_gamma": {
                "future_nested_gamma": {
                    "future_left_gamma": values[0],
                    "future_right_gamma": values[1],
                    "future_parent_gamma": values[2],
                    "future_state_gamma": values[3]
                }
            }
        }),
        _ => json!({
            "future_state_delta": values[3],
            "future_relation_delta": [values[2]],
            "future_values_delta": {
                "future_right_delta": values[1],
                "future_left_delta": values[0]
            }
        }),
    }
}

fn future_context(index: usize) -> PreActionBindingContextV1 {
    PreActionBindingContextV1 {
        call_shape_count: 1,
        capability_count: 2,
        completion_state: super::super::BindingCompletionStateV1::Unresolved,
        temporal_relation_count: 1,
        cardinality_relation_count: 1,
        topology_neighborhood_root_sha256: digest(&format!("future-topology-{}", index % 6)),
    }
}

fn evidence_record(
    index: usize,
    previous_record_sha256: &str,
    payload: &Value,
    options: FixtureOptions,
) -> EvidenceLedgerRecord {
    let session = if options.overlap_support_session && index == 0 {
        "b1b-support-session-0".to_owned()
    } else if options.split_session_slot && index == 1 {
        "b1b-future-session-split".to_owned()
    } else {
        format!("b1b-future-session-{}", index / 3)
    };
    let payload_bytes = serde_json::to_vec(payload).expect("future payload");
    let envelope = RawEvidenceEnvelope {
        source_stream_id: "nando-b1b-future-acquisition-v1".to_owned(),
        source_offset: index as u64,
        event_id: format!("b1b-future-event-{index}"),
        session_id: session,
        client_intent_id: Some(format!("b1b-future-intent-{index}")),
        call_id: Some(format!("b1b-future-call-{index}")),
        output_ordinal: Some(index as u32),
        event_time_unix_nanos: Some(20_000_000 + index as u64),
        schema_version: 1,
        payload: payload_bytes,
    };
    let outcome = EvidenceIngestOutcome::Normalized {
        graph: canonicalize_evidence_envelope(&envelope, EvidencePolicyV1::streaming_bounded())
            .expect("canonical future event"),
    };
    #[derive(Serialize)]
    struct DigestFields<'a> {
        schema: &'a str,
        sequence: u64,
        previous_record_sha256: &'a str,
        outcome: &'a EvidenceIngestOutcome,
    }
    let sequence = PINNED_SUPPORT_WATERMARK_NEXT_SEQUENCE + index as u64;
    let record_sha256 = canonical_json_sha256(&DigestFields {
        schema: EVIDENCE_LEDGER_SCHEMA_V1,
        sequence,
        previous_record_sha256,
        outcome: &outcome,
    })
    .expect("future record digest");
    EvidenceLedgerRecord {
        schema: EVIDENCE_LEDGER_SCHEMA_V1.to_owned(),
        sequence,
        previous_record_sha256: previous_record_sha256.to_owned(),
        outcome,
        record_sha256,
    }
}

fn future_inputs_and_index(
    options: FixtureOptions,
) -> (CaptureCommitmentIndex, Vec<BindingFutureCaptureInputV1>) {
    let support = BindingSupportFreezeV1::from_canonical_bytes(SUPPORT_FREEZE_BYTES)
        .expect("checked-in support freeze");
    let mut commitments = support.capture_index().records.clone();
    let mut previous = commitments
        .last()
        .expect("support tail")
        .record_sha256
        .clone();
    let mut inputs = Vec::new();
    for index in 0..FUTURE_ROWS_V1 {
        let payload = future_payload(index, options);
        let record = evidence_record(index, &previous, &payload, options);
        previous = record.record_sha256.clone();
        let commitment = CaptureRecordCommitment {
            sequence: record.sequence,
            record_sha256: record.record_sha256.clone(),
        };
        commitments.push(commitment.clone());
        inputs.push(BindingFutureCaptureInputV1 {
            slot_id: format!("F{index:02}"),
            capture_receipt: CaptureEvidenceReceipt::new(vec![commitment])
                .expect("future capture receipt"),
            capture_record: record,
            provider_payload: payload,
            context: future_context(index),
        });
    }
    (
        CaptureCommitmentIndex::new(commitments).expect("extended capture index"),
        inputs,
    )
}

fn owner_and_inputs(
    options: FixtureOptions,
) -> (
    BindingFutureCaptureOwnerV1,
    Vec<BindingFutureCaptureInputV1>,
) {
    let (index, inputs) = future_inputs_and_index(options);
    let owner = BindingFutureCaptureOwnerV1::new(
        binding_future_acquisition_protocol_v1(),
        SUPPORT_FREEZE_BYTES,
        SUPPORT_WATERMARK_BYTES,
        index,
    )
    .expect("future owner");
    (owner, inputs)
}

fn complete_freeze(options: FixtureOptions) -> BindingFutureCaptureFreezeV1 {
    let (mut owner, inputs) = owner_and_inputs(options);
    for input in inputs {
        owner.capture_future(input).expect("future row");
    }
    owner.freeze().expect("future freeze")
}

fn recompute_local_future_freeze(freeze: &mut BindingFutureCaptureFreezeV1) {
    for row in &mut freeze.future_rows {
        row.row_sha256 = future_row_digest(row).expect("local row digest");
    }
    let row_roots = freeze
        .future_rows
        .iter()
        .map(|row| row.row_sha256.as_str())
        .collect::<Vec<_>>();
    let graph_roots = freeze
        .future_rows
        .iter()
        .map(|row| row.frozen_graph.graph_root_sha256.as_str())
        .collect::<Vec<_>>();
    freeze.future_rows_root_sha256 = sha256_json(&row_roots).expect("local row roots");
    freeze.future_graphs_root_sha256 = sha256_json(&graph_roots).expect("local graph roots");
    freeze.receipt_sha256 = future_freeze_digest(freeze).expect("local future digest");
}

#[test]
fn f0_protocol_keeps_future_closed_and_authority_false() {
    let protocol = binding_future_acquisition_protocol_v1();
    protocol.validate().expect("F0 protocol");
    assert_eq!(protocol.stop_id, "STOP-B1B-F0");
    assert_eq!(protocol.future_rows_captured, 0);
    assert!(!protocol.expected_labels_joined);
    assert!(!protocol.h0_adjudicated);
    assert!(!protocol.h1_adjudicated);
    assert!(!protocol.f4_started);
    assert!(!protocol.execution_authority);
}

#[test]
fn f0_protocol_is_pinned_to_the_checked_in_support_boundary() {
    let protocol = binding_future_acquisition_protocol_v1();
    assert_eq!(
        sha256_bytes(SUPPORT_FREEZE_BYTES),
        protocol.support_freeze_file_sha256
    );
    assert_eq!(
        sha256_bytes(SUPPORT_WATERMARK_BYTES),
        protocol.support_watermark_file_sha256
    );
    assert_eq!(protocol.support_watermark_next_sequence, 12);
    assert_eq!(protocol.source.planned_future_rows, 12);
    assert_eq!(protocol.source.planned_session_slots, 4);
}

#[test]
fn f0_schedule_freezes_two_rows_for_every_intervention() {
    let protocol = binding_future_acquisition_protocol_v1();
    assert_eq!(protocol.source.slots.len(), 12);
    assert!(
        protocol
            .source
            .rows_per_intervention
            .values()
            .all(|count| *count == 2)
    );
    assert_eq!(protocol.source.rows_per_intervention.len(), 6);
}

#[test]
fn future_pipe_batch_roundtrips_canonically_without_labels() {
    let protocol = binding_future_acquisition_protocol_v1();
    let (_, inputs) = future_inputs_and_index(FixtureOptions::default());
    let batch = BindingFutureCaptureBatchV1::new(&protocol, inputs).expect("future batch");
    let bytes = batch.canonical_bytes().expect("future batch bytes");
    let restored = BindingFutureCaptureBatchV1::from_canonical_bytes(&bytes, &protocol)
        .expect("future batch restart");
    assert_eq!(restored, batch);
    let text = String::from_utf8(bytes).expect("future batch JSON");
    for forbidden in [
        "expected_action",
        "teacher_action",
        "post_action_state",
        "selected_hypothesis",
    ] {
        assert!(!text.contains(forbidden), "leaked batch field: {forbidden}");
    }
}

#[test]
fn future_pipe_batch_rejects_duplicate_slots() {
    let protocol = binding_future_acquisition_protocol_v1();
    let (_, mut inputs) = future_inputs_and_index(FixtureOptions::default());
    inputs[1].slot_id = inputs[0].slot_id.clone();
    assert_eq!(
        BindingFutureCaptureBatchV1::new(&protocol, inputs),
        Err(BindingFutureCaptureErrorV1::InvalidBatch)
    );
}

#[test]
fn future_pipe_batch_rejects_unknown_label_fields() {
    let protocol = binding_future_acquisition_protocol_v1();
    let (_, inputs) = future_inputs_and_index(FixtureOptions::default());
    let batch = BindingFutureCaptureBatchV1::new(&protocol, inputs).expect("future batch");
    let mut value = serde_json::to_value(batch).expect("future batch value");
    value
        .as_object_mut()
        .expect("future batch object")
        .insert("expected_label".to_owned(), json!("forbidden"));
    let bytes = serde_json::to_vec(&value).expect("tampered batch bytes");
    assert_eq!(
        BindingFutureCaptureBatchV1::from_canonical_bytes(&bytes, &protocol),
        Err(BindingFutureCaptureErrorV1::InvalidBatch)
    );
}

#[test]
fn exact_post_watermark_fixture_satisfies_the_future_owner() {
    let freeze = complete_freeze(FixtureOptions::default());
    let report = freeze.report();
    assert_eq!(report.future_rows, 12);
    assert_eq!(report.future_session_lineages, 4);
    assert!(report.distinct_future_shape_roots >= 3);
    assert!(report.ordinal_layout_trap_pairs >= 1);
    assert!(!report.expected_labels_joined);
    assert_eq!(report.h0_status, "UNPROVEN");
    assert_eq!(report.h1_status, "UNPROVEN");
    assert_eq!(report.adjudication_status, "NOT_STARTED");
    assert_eq!(report.f4_status, "BLOCKED");
    assert!(!report.execution_authority);
}

#[test]
fn future_freeze_restart_is_byte_identical() {
    let freeze = complete_freeze(FixtureOptions::default());
    let trusted_receipt = freeze.receipt_sha256().to_owned();
    let bytes = freeze.canonical_bytes().expect("future freeze bytes");
    let restored = BindingFutureCaptureFreezeV1::from_canonical_bytes(
        &bytes,
        &trusted_receipt,
        SUPPORT_FREEZE_BYTES,
        SUPPORT_WATERMARK_BYTES,
    )
    .expect("future freeze restart");
    assert_eq!(restored, freeze);
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
}

#[test]
fn future_restart_rejects_a_recomputed_foreign_support_prefix() {
    let mut freeze = complete_freeze(FixtureOptions::default());
    let mut records = freeze.capture_index.records.clone();
    records[0].record_sha256 = digest("foreign-support-prefix");
    freeze.capture_index = CaptureCommitmentIndex::new(records).expect("foreign capture index");
    freeze.receipt_sha256 = future_freeze_digest(&freeze).expect("recomputed local receipt");
    let locally_trusted_receipt = freeze.receipt_sha256().to_owned();
    let bytes = freeze.canonical_bytes().expect("foreign future freeze");

    assert_eq!(
        BindingFutureCaptureFreezeV1::from_canonical_bytes(
            &bytes,
            &locally_trusted_receipt,
            SUPPORT_FREEZE_BYTES,
            SUPPORT_WATERMARK_BYTES,
        ),
        Err(BindingFutureCaptureErrorV1::CaptureIndexNotExtension)
    );
}

#[test]
fn future_restart_requires_the_pinned_support_artifact() {
    let freeze = complete_freeze(FixtureOptions::default());
    let trusted_receipt = freeze.receipt_sha256().to_owned();
    let bytes = freeze.canonical_bytes().expect("future freeze bytes");
    let mut foreign_support = SUPPORT_FREEZE_BYTES.to_vec();
    foreign_support.push(b'\n');

    assert_eq!(
        BindingFutureCaptureFreezeV1::from_canonical_bytes(
            &bytes,
            &trusted_receipt,
            &foreign_support,
            SUPPORT_WATERMARK_BYTES,
        ),
        Err(BindingFutureCaptureErrorV1::InvalidPinnedSupport)
    );
}

#[test]
fn future_restart_rejects_a_receipt_for_a_different_row() {
    let mut freeze = complete_freeze(FixtureOptions::default());
    freeze.future_rows[0].capture_receipt = freeze.future_rows[1].capture_receipt.clone();
    recompute_local_future_freeze(&mut freeze);
    let locally_trusted_receipt = freeze.receipt_sha256().to_owned();
    let bytes = freeze.canonical_bytes().expect("mismatched row receipt");

    assert_eq!(
        BindingFutureCaptureFreezeV1::from_canonical_bytes(
            &bytes,
            &locally_trusted_receipt,
            SUPPORT_FREEZE_BYTES,
            SUPPORT_WATERMARK_BYTES,
        ),
        Err(BindingFutureCaptureErrorV1::InvalidCaptureReceipt)
    );
}

#[test]
fn recomputed_derived_graph_cannot_replace_the_external_future_root() {
    let mut freeze = complete_freeze(FixtureOptions::default());
    let trusted_receipt = freeze.receipt_sha256().to_owned();
    freeze.future_rows[0].frozen_graph.graph.nodes[0].occurrence_count += 1;
    freeze.future_rows[0].frozen_graph = freeze.future_rows[0]
        .frozen_graph
        .graph
        .clone()
        .freeze()
        .expect("locally refrozen graph");
    recompute_local_future_freeze(&mut freeze);
    let bytes = freeze.canonical_bytes().expect("locally recomputed future");

    assert_eq!(
        BindingFutureCaptureFreezeV1::from_canonical_bytes(
            &bytes,
            &trusted_receipt,
            SUPPORT_FREEZE_BYTES,
            SUPPORT_WATERMARK_BYTES,
        ),
        Err(BindingFutureCaptureErrorV1::InvalidFreeze)
    );
}

#[test]
fn tampered_protocol_is_rejected_even_with_a_recomputed_local_receipt() {
    let mut protocol = binding_future_acquisition_protocol_v1();
    protocol.source.planned_future_rows = 11;
    protocol.receipt_sha256 = protocol_digest(&protocol).expect("local checksum");
    assert_eq!(
        protocol.validate(),
        Err(BindingFutureCaptureErrorV1::InvalidProtocol)
    );
}

#[test]
fn replaced_support_freeze_is_rejected() {
    let (index, _) = future_inputs_and_index(FixtureOptions::default());
    let mut bytes = SUPPORT_FREEZE_BYTES.to_vec();
    bytes.push(b'\n');
    assert_eq!(
        BindingFutureCaptureOwnerV1::new(
            binding_future_acquisition_protocol_v1(),
            &bytes,
            SUPPORT_WATERMARK_BYTES,
            index,
        )
        .err(),
        Some(BindingFutureCaptureErrorV1::InvalidPinnedSupport)
    );
}

#[test]
fn replaced_watermark_is_rejected() {
    let (index, _) = future_inputs_and_index(FixtureOptions::default());
    let mut bytes = SUPPORT_WATERMARK_BYTES.to_vec();
    bytes.push(b'\n');
    assert_eq!(
        BindingFutureCaptureOwnerV1::new(
            binding_future_acquisition_protocol_v1(),
            SUPPORT_FREEZE_BYTES,
            &bytes,
            index,
        )
        .err(),
        Some(BindingFutureCaptureErrorV1::InvalidPinnedSupport)
    );
}

#[test]
fn non_prefix_capture_index_is_rejected() {
    let (index, _) = future_inputs_and_index(FixtureOptions::default());
    let mut records = index.records;
    records[0].record_sha256 = digest("replaced-support-prefix");
    let index = CaptureCommitmentIndex::new(records).expect("locally valid foreign index");
    assert_eq!(
        BindingFutureCaptureOwnerV1::new(
            binding_future_acquisition_protocol_v1(),
            SUPPORT_FREEZE_BYTES,
            SUPPORT_WATERMARK_BYTES,
            index,
        )
        .err(),
        Some(BindingFutureCaptureErrorV1::CaptureIndexNotExtension)
    );
}

#[test]
fn support_session_cannot_reappear_in_future() {
    let (mut owner, mut inputs) = owner_and_inputs(FixtureOptions {
        overlap_support_session: true,
        ..FixtureOptions::default()
    });
    assert_eq!(
        owner.capture_future(inputs.remove(0)),
        Err(BindingFutureCaptureErrorV1::SupportFutureLineageOverlap)
    );
}

#[test]
fn payload_must_match_the_captured_pre_action_graph() {
    let (mut owner, mut inputs) = owner_and_inputs(FixtureOptions::default());
    inputs[0].provider_payload = json!({"future_tampered_payload": true});
    assert_eq!(
        owner.capture_future(inputs.remove(0)),
        Err(BindingFutureCaptureErrorV1::PayloadGraphMismatch)
    );
}

#[test]
fn one_frozen_slot_cannot_vote_twice() {
    let (mut owner, inputs) = owner_and_inputs(FixtureOptions::default());
    owner
        .capture_future(inputs[0].clone())
        .expect("first slot vote");
    assert_eq!(
        owner.capture_future(inputs[0].clone()),
        Err(BindingFutureCaptureErrorV1::DuplicateSlot)
    );
}

#[test]
fn incomplete_future_batch_cannot_freeze() {
    let (mut owner, inputs) = owner_and_inputs(FixtureOptions::default());
    for input in inputs.into_iter().take(11) {
        owner.capture_future(input).expect("partial future row");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingFutureCaptureErrorV1::MissingSlot)
    );
}

#[test]
fn session_slot_cannot_split_across_two_lineages() {
    let (mut owner, inputs) = owner_and_inputs(FixtureOptions {
        split_session_slot: true,
        ..FixtureOptions::default()
    });
    for input in inputs {
        owner.capture_future(input).expect("future row");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingFutureCaptureErrorV1::InvalidSessionPartition)
    );
}

#[test]
fn repeated_layout_without_shape_diversity_is_rejected() {
    let (mut owner, inputs) = owner_and_inputs(FixtureOptions {
        collapse_shapes: true,
        ..FixtureOptions::default()
    });
    for input in inputs {
        owner.capture_future(input).expect("future row");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingFutureCaptureErrorV1::MissingShapeChallenge)
    );
}

#[test]
fn future_without_an_ordinal_layout_trap_is_rejected() {
    let (mut owner, inputs) = owner_and_inputs(FixtureOptions {
        remove_trap: true,
        ..FixtureOptions::default()
    });
    for input in inputs {
        owner.capture_future(input).expect("future row");
    }
    assert_eq!(
        owner.freeze(),
        Err(BindingFutureCaptureErrorV1::MissingOrdinalLayoutTrap)
    );
}

#[test]
fn serialized_f0_contract_has_no_label_or_post_action_input() {
    let text =
        serde_json::to_string(&binding_future_acquisition_protocol_v1()).expect("F0 protocol JSON");
    assert!(text.contains("expected_labels_available\":false"));
    for forbidden in [
        "expected_action_equivalence",
        "teacher_action",
        "post_action_state",
        "state_after",
        "selected_hypothesis",
        "protocol_mode_bytecode",
    ] {
        assert!(!text.contains(forbidden), "leaked F0 field: {forbidden}");
    }
}

#[test]
fn protocol_json_matches_the_checked_in_receipt() {
    let protocol = binding_future_acquisition_protocol_v1();
    assert_eq!(
        protocol.canonical_bytes().expect("canonical F0 protocol"),
        FUTURE_PROTOCOL_BYTES
    );
}

#[test]
fn checked_in_protocol_reopens_through_the_runtime_decoder() {
    let restored = BindingFutureAcquisitionProtocolV1::from_canonical_bytes(FUTURE_PROTOCOL_BYTES)
        .expect("checked-in protocol must be executable");
    assert_eq!(restored, binding_future_acquisition_protocol_v1());
}

#[test]
fn checked_in_future_freeze_restarts_from_the_external_receipt() {
    assert_eq!(sha256_bytes(FUTURE_FREEZE_BYTES), FUTURE_FREEZE_FILE_SHA256);
    assert_eq!(
        sha256_bytes(FUTURE_EXTERNAL_RECEIPT_BYTES),
        FUTURE_EXTERNAL_RECEIPT_FILE_SHA256
    );
    assert_eq!(
        sha256_bytes(FUTURE_CAPTURE_REPORT_BYTES),
        FUTURE_CAPTURE_REPORT_FILE_SHA256
    );
    assert_eq!(
        sha256_bytes(FUTURE_ACQUISITION_REPORT_BYTES),
        FUTURE_ACQUISITION_REPORT_FILE_SHA256
    );

    let receipt: Value =
        serde_json::from_slice(FUTURE_EXTERNAL_RECEIPT_BYTES).expect("external future receipt");
    assert_eq!(
        receipt["future_freeze_file_sha256"],
        FUTURE_FREEZE_FILE_SHA256
    );
    let trusted_receipt = receipt["trusted_future_receipt_sha256"]
        .as_str()
        .expect("trusted future root");
    let freeze = BindingFutureCaptureFreezeV1::from_canonical_bytes(
        FUTURE_FREEZE_BYTES,
        trusted_receipt,
        SUPPORT_FREEZE_BYTES,
        SUPPORT_WATERMARK_BYTES,
    )
    .expect("checked-in future freeze restart");
    let report = freeze.report();
    assert_eq!(report.future_rows, 12);
    assert_eq!(report.future_session_lineages, 4);
    assert_eq!(report.distinct_future_shape_roots, 12);
    assert_eq!(report.ordinal_layout_trap_pairs, 6);
    assert!(!report.expected_labels_joined);
    assert_eq!(report.h0_status, "UNPROVEN");
    assert_eq!(report.h1_status, "UNPROVEN");
    assert_eq!(report.f4_status, "BLOCKED");
    assert!(!report.execution_authority);
}

#[test]
fn checked_in_future_artifacts_do_not_persist_raw_values_or_labels() {
    let freeze_text = std::str::from_utf8(FUTURE_FREEZE_BYTES).expect("future freeze UTF-8");
    for forbidden in [
        "future-i1-capability-left",
        "future-i2-parent-right",
        "future_alpha_timeline",
        "expected_action_equivalence",
        "teacher_action",
        "post_action_state",
        "selected_hypothesis",
    ] {
        assert!(
            !freeze_text.contains(forbidden),
            "leaked future artifact field or value: {forbidden}"
        );
    }

    for bytes in [
        FUTURE_ACQUISITION_REPORT_BYTES,
        FUTURE_CAPTURE_REPORT_BYTES,
        FUTURE_EXTERNAL_RECEIPT_BYTES,
    ] {
        let value: Value = serde_json::from_slice(bytes).expect("future report JSON");
        assert_eq!(value["execution_authority"], false);
        assert_ne!(value["h0_status"], "PROVEN");
        assert_ne!(value["h1_status"], "PROVEN");
    }
}
