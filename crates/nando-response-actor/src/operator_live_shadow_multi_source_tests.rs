use std::collections::BTreeSet;

use nando_operator_kernel::{
    AtomSource, AtomValueType, MultiSourceCardinalityClassV1, MultiSourceContainerClassV1,
    MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
    MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseValueSelector, ValueProjectionFormat, canonical_json_sha256,
    sha256_bytes,
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
