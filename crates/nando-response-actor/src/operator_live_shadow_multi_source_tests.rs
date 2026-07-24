use std::collections::BTreeSet;

use nando_operator_kernel::{
    AtomSource, AtomValueType, MultiSourceCardinalityClassV1, MultiSourceContainerClassV1,
    MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
    MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseRenderSegment, ResponseValueSelector, ValueProjectionFormat,
    canonical_json_sha256, sha256_bytes,
};
use nando_operator_learning::multi_source::{
    BLIND_THEN_REVEAL_JOIN_SCHEMA_V1, BlindThenRevealJoinedTransitionV1, CompletedEffectAtomV1,
    MultiSourceT1IdentificationStateV1, identify_multi_source_t1_operator_v1,
};
use nando_operator_learning::{
    CaptureEvidenceReceipt, CaptureRecordCommitment, CaptureTransitionBinding,
};
use serde_json::json;

use crate::{
    ECONOMICS_RECEIPT_SCHEMA_V1, EconomicsReceipt, RuntimeParityCase, TeacherTransition,
    build_crystallized_admission_snapshot, crystallize_multi_source_t1_candidate_v1,
    teacher_transition_from_completed,
};

fn root(label: &str) -> String {
    sha256_bytes(label.as_bytes())
}

fn completed_projection_frame(
    intent: &str,
    event: &str,
    session: &str,
    field: &str,
    observed_at: u64,
) -> RelationFrame {
    let value_root = canonical_json_sha256(&json!(7)).expect("value root");
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at,
        estimated_input_tokens: 100,
        extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::Integer,
                source: AtomSource::Observation,
                value_sha256: value_root.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: ResponseValueSelector::JsonField {
                    field: field.to_owned(),
                    value_type: AtomValueType::Integer,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type: AtomValueType::Integer,
                source: AtomSource::Action,
                value_sha256: value_root,
            },
            RelationAtom::SlotEquality {
                left_slot: 7,
                right_slot: 11,
            },
            RelationAtom::ActionValueProjection {
                format: ValueProjectionFormat::PlainText,
                renderer: nando_operator_kernel::CollectionOutputRenderer::Direct,
            },
        ],
        evidence_ref_sha256: root(&format!("evidence:{event}")),
    }
}

fn joined_row(
    frame: &RelationFrame,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
) -> BlindThenRevealJoinedTransitionV1 {
    let value_root = canonical_json_sha256(&json!(7)).expect("value root");
    let topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 1,
        roles: vec![MultiSourceRoleNodeV1 {
            local_role_id: 0,
            source_ordinal: 0,
            value_ordinal: 0,
            type_class: MultiSourceTypeClassV1::Number,
            container_class: MultiSourceContainerClassV1::Scalar,
            cardinality_class: MultiSourceCardinalityClassV1::One,
            temporal_class: MultiSourceTemporalClassV1::Latest,
            depth_bucket: 1,
            structural_flags: 1,
        }],
        role_witnesses: vec![MultiSourceRoleWitnessV1 {
            local_role_id: 0,
            value_sha256: value_root,
            request_reference_ordinal: Some(0),
        }],
        relations: vec![
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 0,
                target_role_id: 0,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::LatestOutput,
                source_role_id: 0,
                target_role_id: 0,
            },
        ],
    };
    BlindThenRevealJoinedTransitionV1 {
        schema: BLIND_THEN_REVEAL_JOIN_SCHEMA_V1.to_owned(),
        join_root_sha256: root(&format!("join:{request_event}")),
        capture_sequence,
        turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        request_event_id_sha256: root(request_event),
        action_event_id_sha256: frame.event_id_sha256.clone(),
        session_lineage_sha256: root(session),
        session_id_sha256: frame.session_id_sha256.clone(),
        topology_commitment_root_sha256: root(&format!("topology:{request_event}")),
        pre_action_record_root_sha256: root(&format!("record:{request_event}")),
        completed_frame_root_sha256: canonical_json_sha256(frame).expect("frame root"),
        physical_action_root_sha256: root("physical action"),
        semantic_action_root_sha256: root("semantic action"),
        effect_atoms: vec![
            CompletedEffectAtomV1::RoleInput,
            CompletedEffectAtomV1::ValueProjection,
        ],
        verifier_receipt_root_sha256: root(&format!("verifier:{request_event}")),
        input_tokens: 100,
        captured_at_unix_ms: capture_sequence,
        completed_at_unix_nanos: frame.observed_at_unix_nanos,
        accepted: true,
        topology,
    }
}

fn runtime_transition(frame: &RelationFrame, field: &str, sequence: u64) -> TeacherTransition {
    let economics = EconomicsReceipt {
        schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
        exact_input_tokens: 100,
        ordinary: true,
        controlled: false,
        replay: false,
        dedupe_eligible: true,
        provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
    };
    let mut transition =
        teacher_transition_from_completed(frame, Some(economics)).expect("teacher transition");
    let mut receipt = CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
        sequence,
        record_sha256: root(&format!("capture:{sequence}")),
    }])
    .expect("capture receipt");
    let binding = CaptureTransitionBinding::new(sequence, &frame.frame_id_sha256, &receipt)
        .expect("transition binding");
    receipt.bind_transition(binding).expect("bind receipt");
    transition.runtime_parity_case = Some(RuntimeParityCase {
        evidence_ref_sha256: frame.frame_id_sha256.clone(),
        capture_receipt: Some(receipt),
        request_text: format!("Return {field}"),
        provider_payload: json!({
            "input": [{
                "type": "function_call_output",
                "output": format!("{{\"{field}\":7}}")
            }]
        }),
        expected_response: "7".to_owned(),
    });
    transition
}

fn completed_multi_projection_frame(
    intent: &str,
    event: &str,
    session: &str,
    fields: (&str, &str),
    values: (&str, &str),
    observed_at: u64,
) -> RelationFrame {
    let first_root = canonical_json_sha256(&json!(values.0)).expect("first root");
    let second_root = canonical_json_sha256(&json!(values.1)).expect("second root");
    let first_selector = ResponseValueSelector::JsonField {
        field: fields.0.to_owned(),
        value_type: AtomValueType::String,
    };
    let second_selector = ResponseValueSelector::JsonField {
        field: fields.1.to_owned(),
        value_type: AtomValueType::String,
    };
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at,
        estimated_input_tokens: 100,
        extractor_version: crate::SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: first_root.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: first_selector,
            },
            RelationAtom::TypedSlot {
                slot_id: 8,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: second_root.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 8 },
            RelationAtom::ObservationSelector {
                slot_id: 8,
                selector: second_selector.clone(),
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: first_root,
            },
            RelationAtom::SlotEquality {
                left_slot: 7,
                right_slot: 11,
            },
            RelationAtom::TypedSlot {
                slot_id: 12,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: second_root,
            },
            RelationAtom::SlotEquality {
                left_slot: 8,
                right_slot: 12,
            },
            RelationAtom::ActionValueProjection {
                format: ValueProjectionFormat::PlainText,
                renderer: nando_operator_kernel::CollectionOutputRenderer::RenderSequence {
                    segments: vec![
                        ResponseRenderSegment::Primary,
                        ResponseRenderSegment::Static {
                            text: ": ".to_owned(),
                        },
                        ResponseRenderSegment::Selected {
                            selector: second_selector,
                            format: ValueProjectionFormat::PlainText,
                        },
                    ],
                },
            },
        ],
        evidence_ref_sha256: root(&format!("evidence:{event}")),
    }
}

fn joined_multi_row(
    frame: &RelationFrame,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    values: (&str, &str),
) -> BlindThenRevealJoinedTransitionV1 {
    let topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 2,
        roles: (0..2)
            .map(|ordinal| MultiSourceRoleNodeV1 {
                local_role_id: ordinal,
                source_ordinal: 0,
                value_ordinal: ordinal,
                type_class: MultiSourceTypeClassV1::String,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 1,
            })
            .collect(),
        role_witnesses: vec![
            MultiSourceRoleWitnessV1 {
                local_role_id: 0,
                value_sha256: canonical_json_sha256(&json!(values.0)).expect("first root"),
                request_reference_ordinal: Some(0),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: canonical_json_sha256(&json!(values.1)).expect("second root"),
                request_reference_ordinal: Some(1),
            },
        ],
        relations: vec![
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 0,
                target_role_id: 0,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 1,
                target_role_id: 1,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id: 0,
                target_role_id: 1,
            },
        ],
    };
    BlindThenRevealJoinedTransitionV1 {
        schema: BLIND_THEN_REVEAL_JOIN_SCHEMA_V1.to_owned(),
        join_root_sha256: root(&format!("join:{request_event}")),
        capture_sequence,
        turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        request_event_id_sha256: root(request_event),
        action_event_id_sha256: frame.event_id_sha256.clone(),
        session_lineage_sha256: root(session),
        session_id_sha256: frame.session_id_sha256.clone(),
        topology_commitment_root_sha256: root(&format!("topology:{request_event}")),
        pre_action_record_root_sha256: root(&format!("record:{request_event}")),
        completed_frame_root_sha256: canonical_json_sha256(frame).expect("frame root"),
        physical_action_root_sha256: root(&format!("physical:{request_event}")),
        semantic_action_root_sha256: root("multi-role semantic action"),
        effect_atoms: vec![
            CompletedEffectAtomV1::RoleInputSlot {
                slot_id: 7,
                value_type: Some(AtomValueType::String),
            },
            CompletedEffectAtomV1::RoleInputSlot {
                slot_id: 8,
                value_type: Some(AtomValueType::String),
            },
            CompletedEffectAtomV1::ValueProjection,
        ],
        verifier_receipt_root_sha256: root(&format!("verifier:{request_event}")),
        input_tokens: 100,
        captured_at_unix_ms: capture_sequence,
        completed_at_unix_nanos: frame.observed_at_unix_nanos,
        accepted: true,
        topology,
    }
}

fn runtime_multi_transition(
    frame: &RelationFrame,
    fields: (&str, &str),
    values: (&str, &str),
    sequence: u64,
) -> TeacherTransition {
    let economics = EconomicsReceipt {
        schema: ECONOMICS_RECEIPT_SCHEMA_V1.to_owned(),
        exact_input_tokens: 100,
        ordinary: true,
        controlled: false,
        replay: false,
        dedupe_eligible: true,
        provider_evidence_ref_sha256: frame.evidence_ref_sha256.clone(),
    };
    let mut transition =
        teacher_transition_from_completed(frame, Some(economics)).expect("teacher transition");
    let mut receipt = CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
        sequence,
        record_sha256: root(&format!("capture:{sequence}")),
    }])
    .expect("capture receipt");
    let binding = CaptureTransitionBinding::new(sequence, &frame.frame_id_sha256, &receipt)
        .expect("transition binding");
    receipt.bind_transition(binding).expect("bind receipt");
    transition.runtime_parity_case = Some(RuntimeParityCase {
        evidence_ref_sha256: frame.frame_id_sha256.clone(),
        capture_receipt: Some(receipt),
        request_text: format!("Return {} then {}", fields.0, fields.1),
        provider_payload: json!({
            "input": [{
                "type": "function_call_output",
                "output": format!(
                    "{{\"{}\":\"{}\",\"{}\":\"{}\"}}",
                    fields.0, values.0, fields.1, values.1
                )
            }]
        }),
        expected_response: format!("{}: {}", values.0, values.1),
    });
    transition
}

#[test]
fn multi_source_freeze_reconstructs_through_independent_admission() {
    let support_frame = completed_projection_frame("intent-a", "event-a", "session-a", "alpha", 1);
    let future_frame = completed_projection_frame("intent-b", "event-b", "session-b", "beta", 2);
    let joined = vec![
        joined_row(&support_frame, "request-a", "session-a", 1),
        joined_row(&future_frame, "request-b", "session-b", 2),
    ];
    let frames = vec![support_frame.clone(), future_frame.clone()];
    let identification = identify_multi_source_t1_operator_v1(
        &joined,
        &frames,
        &BTreeSet::new(),
        root("multi-source epoch"),
    );
    assert_eq!(
        identification.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    let transitions = vec![
        runtime_transition(&support_frame, "alpha", 1),
        runtime_transition(&future_frame, "beta", 2),
    ];
    let candidate =
        crystallize_multi_source_t1_candidate_v1(&identification, &transitions).expect("candidate");
    assert_eq!(
        candidate
            .multi_source_identification
            .as_ref()
            .map(|report| report.report_root_sha256.as_str()),
        Some(identification.report_root_sha256.as_str())
    );

    let admitted = build_crystallized_admission_snapshot(
        &[candidate],
        "multi-source-test",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation")
    .expect("active package");
    assert_eq!(admitted.registry.packages.len(), 1);
    assert_eq!(admitted.registry.packages[0].proof.wrong_accepts, 0);
    assert_eq!(
        admitted.registry.packages[0].proof.runtime_parity_failures,
        0
    );
}

#[test]
fn rich_multi_role_law_reconstructs_through_independent_admission() {
    let support_fields = ("city", "temperature");
    let future_fields = ("place", "degrees");
    let support_values = ("Tallinn", "7");
    let future_values = ("Tartu", "4");
    let support_frame = completed_multi_projection_frame(
        "intent-rich-a",
        "event-rich-a",
        "session-rich-a",
        support_fields,
        support_values,
        1,
    );
    let future_frame = completed_multi_projection_frame(
        "intent-rich-b",
        "event-rich-b",
        "session-rich-b",
        future_fields,
        future_values,
        2,
    );
    let joined = vec![
        joined_multi_row(
            &support_frame,
            "request-rich-a",
            "session-rich-a",
            1,
            support_values,
        ),
        joined_multi_row(
            &future_frame,
            "request-rich-b",
            "session-rich-b",
            2,
            future_values,
        ),
    ];
    let frames = vec![support_frame.clone(), future_frame.clone()];
    let identification = identify_multi_source_t1_operator_v1(
        &joined,
        &frames,
        &BTreeSet::new(),
        root("rich multi-source epoch"),
    );
    assert_eq!(
        identification.state,
        MultiSourceT1IdentificationStateV1::TransferReady,
        "{identification:#?}"
    );
    let transitions = vec![
        runtime_multi_transition(&support_frame, support_fields, support_values, 1),
        runtime_multi_transition(&future_frame, future_fields, future_values, 2),
    ];
    let candidate =
        crystallize_multi_source_t1_candidate_v1(&identification, &transitions).expect("candidate");
    assert_eq!(candidate.package.proof.wrong_accepts, 0);
    assert_eq!(candidate.package.proof.runtime_parity_failures, 0);

    let admitted = build_crystallized_admission_snapshot(
        &[candidate],
        "rich-multi-source-test",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation")
    .expect("active package");
    assert_eq!(admitted.registry.packages.len(), 1);
    assert_eq!(admitted.registry.packages[0].proof.wrong_accepts, 0);
    assert_eq!(
        admitted.registry.packages[0].proof.runtime_parity_failures,
        0
    );
}
