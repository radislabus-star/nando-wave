use std::collections::BTreeSet;

use nando_operator_kernel::{
    AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
    MultiSourceExtractionStatusV1, MultiSourceRelationEdgeV1, MultiSourceRelationKindV1,
    MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1,
    MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1,
    RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, ResponseRenderSegment,
    ResponseValueSelector, ValueProjectionFormat, canonical_json_sha256, sha256_bytes,
};
use nando_operator_learning::multi_source::{
    BlindThenRevealJoinedTransitionV1, FrozenRawPhaseT1ContractV1, MultiSourceJoinLedgerV1,
    MultiSourceT1IdentificationStateV1, MultiSourceT1IdentificationV3, PreActionTopologyAuditRowV1,
    identify_multi_source_t1_operator_v1,
    identify_multi_source_t1_operator_with_frozen_raw_phase_v1,
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
    let mut topology = PreActionMultiSourceTopologyV1 {
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
            request_reference_ordinal_candidates: Vec::new(),
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
    topology.relations.sort();
    let structure = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        request_event_id_sha256: root(request_event),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![root(session)],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        estimated_input_tokens: 100,
        provider_payload_bytes: 400,
        provider_capture_request_root_sha256: root(&format!("request:{capture_sequence}")),
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology,
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("topology commit");
    let topology_row = PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: root("bridge"),
        bridge_sequence: Some(capture_sequence),
        record_sha256: Some(root(&format!("record:{capture_sequence}"))),
        capture_epoch_sha256: Some(root("capture epoch")),
        capture_event_sha256: Some(root(&format!("capture event:{capture_sequence}"))),
        capture_receipt_sha256: Some(root(&format!("receipt:{capture_sequence}"))),
        captured_at_unix_ms: Some(capture_sequence),
        session_lineage_sha256: Some(root(session)),
        physical_order_proven: true,
        structure,
        commit,
    };
    MultiSourceJoinLedgerV1::build(&[topology_row], std::slice::from_ref(frame))
        .rows()
        .into_iter()
        .next()
        .expect("joined row")
}

fn raw_phase_fixture() -> (MultiSourceT1IdentificationV3, Vec<TeacherTransition>) {
    let support_a = completed_projection_frame(
        "raw-intent-a",
        "raw-event-a",
        "raw-session-a",
        "alpha",
        1_500_000_000,
    );
    let support_b = completed_projection_frame(
        "raw-intent-b",
        "raw-event-b",
        "raw-session-b",
        "alpha",
        2_500_000_000,
    );
    let future = completed_projection_frame(
        "raw-intent-c",
        "raw-event-c",
        "raw-session-c",
        "alpha",
        3_500_000_000,
    );
    let future_d = completed_projection_frame(
        "raw-intent-d",
        "raw-event-d",
        "raw-session-d",
        "alpha",
        4_500_000_000,
    );
    let future_e = completed_projection_frame(
        "raw-intent-e",
        "raw-event-e",
        "raw-session-e",
        "alpha",
        5_500_000_000,
    );
    let future_f = completed_projection_frame(
        "raw-intent-f",
        "raw-event-f",
        "raw-session-f",
        "alpha",
        6_500_000_000,
    );
    let joined = vec![
        joined_row(&support_a, "raw-request-a", "raw-session-a", 1),
        joined_row(&support_b, "raw-request-b", "raw-session-b", 2),
        joined_row(&future, "raw-request-c", "raw-session-c", 3),
        joined_row(&future_d, "raw-request-d", "raw-session-d", 4),
        joined_row(&future_e, "raw-request-e", "raw-session-e", 5),
        joined_row(&future_f, "raw-request-f", "raw-session-f", 6),
    ];
    let frames = vec![
        support_a.clone(),
        support_b.clone(),
        future.clone(),
        future_d.clone(),
        future_e.clone(),
        future_f.clone(),
    ];
    let identification = identify_multi_source_t1_operator_with_frozen_raw_phase_v1(
        &joined,
        &frames,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &[],
        FrozenRawPhaseT1ContractV1 {
            frozen_domain_root_sha256: &root("raw phase response actor domain"),
            support_watermark: 2,
        },
        root("raw phase response actor epoch"),
    );
    assert_eq!(
        identification.state,
        MultiSourceT1IdentificationStateV1::TransferReady,
        "{identification:#?}"
    );
    assert!(identification.raw_phase_selected_executable.is_some());
    let transitions = vec![
        runtime_transition(&support_a, "alpha", 1),
        runtime_transition(&support_b, "alpha", 2),
        runtime_transition(&future, "alpha", 3),
        runtime_transition(&future_d, "alpha", 4),
        runtime_transition(&future_e, "alpha", 5),
        runtime_transition(&future_f, "alpha", 6),
    ];

    (identification, transitions)
}

#[test]
fn raw_phase_selected_executable_reconstructs_before_crystallization() {
    let (identification, transitions) = raw_phase_fixture();

    let candidate =
        crystallize_multi_source_t1_candidate_v1(&identification, &transitions).expect("candidate");
    assert_eq!(candidate.package.proof.wrong_accepts, 0);
    assert_eq!(candidate.package.proof.runtime_parity_failures, 0);
    assert_eq!(candidate.support.len(), 2);
    assert_eq!(candidate.future.len(), 4);
}

#[test]
fn raw_phase_selected_receipt_tampering_is_rejected() {
    let (identification, _) = raw_phase_fixture();
    let receipt = identification
        .raw_phase_selected_executable
        .as_ref()
        .expect("selected executable receipt");
    let freeze = identification
        .candidate_freeze
        .as_ref()
        .expect("candidate freeze");
    let program = identification
        .canonical_program
        .as_ref()
        .expect("canonical program");

    let mut root_tamper = receipt.clone();
    root_tamper.receipt_root_sha256 = root("tampered selected receipt");
    assert_eq!(
        nando_operator_learning::multi_source::rebuild_raw_phase_selected_executable_v1(
            &root_tamper,
            freeze,
            program,
        ),
        Err("raw_phase_selected_executable_receipt_invalid")
    );

    let mut disposition_tamper = receipt.clone();
    disposition_tamper.selected_disposition.support_bundle_count = disposition_tamper
        .selected_disposition
        .support_bundle_count
        .saturating_add(1);
    assert_eq!(
        nando_operator_learning::multi_source::rebuild_raw_phase_selected_executable_v1(
            &disposition_tamper,
            freeze,
            program,
        ),
        Err("raw_phase_selected_executable_receipt_invalid")
    );

    let mut fingerprint_tamper = receipt.clone();
    fingerprint_tamper
        .selected_disposition
        .blueprint_fingerprints_sha256[0] = root("tampered blueprint fingerprint");
    assert_eq!(
        nando_operator_learning::multi_source::rebuild_raw_phase_selected_executable_v1(
            &fingerprint_tamper,
            freeze,
            program,
        ),
        Err("raw_phase_selected_executable_receipt_invalid")
    );
}

#[test]
fn raw_phase_capture_sequence_mismatch_is_rejected() {
    let (identification, mut transitions) = raw_phase_fixture();
    let future_frame_id = identification
        .proof_basis
        .as_ref()
        .and_then(|basis| basis.raw_phase_future_evidence.first())
        .map(|evidence| evidence.frame.frame_id_sha256.clone())
        .expect("future frame id");
    let transition = transitions
        .iter_mut()
        .find(|transition| {
            transition
                .runtime_parity_case
                .as_ref()
                .is_some_and(|parity| parity.evidence_ref_sha256 == future_frame_id)
        })
        .expect("future transition");
    let mut receipt = CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
        sequence: 99,
        record_sha256: root("capture:99"),
    }])
    .expect("tampered capture receipt");
    let binding = CaptureTransitionBinding::new(99, &future_frame_id, &receipt)
        .expect("tampered transition binding");
    receipt
        .bind_transition(binding)
        .expect("bind tampered receipt");
    transition
        .runtime_parity_case
        .as_mut()
        .expect("runtime parity")
        .capture_receipt = Some(receipt);

    assert_eq!(
        crystallize_multi_source_t1_candidate_v1(&identification, &transitions),
        Err("multi_source_raw_phase_transition_binding_mismatch".to_owned())
    );
}

#[test]
fn raw_phase_future_evidence_removal_invalidates_identification() {
    let (mut identification, transitions) = raw_phase_fixture();
    let basis = identification
        .proof_basis
        .as_mut()
        .expect("raw Phase proof basis");
    basis.raw_phase_future_evidence.clear();
    basis.basis_root_sha256 = basis.expected_root();
    identification.report_root_sha256 = identification.expected_root();

    assert!(!identification.validate());
    assert_eq!(
        crystallize_multi_source_t1_candidate_v1(&identification, &transitions),
        Err("multi_source_identification_not_transfer_ready".to_owned())
    );
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
    let mut topology = PreActionMultiSourceTopologyV1 {
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
                request_reference_ordinal_candidates: Vec::new(),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: canonical_json_sha256(&json!(values.1)).expect("second root"),
                request_reference_ordinal: Some(1),
                request_reference_ordinal_candidates: Vec::new(),
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
    topology.relations.sort();
    let structure = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: frame.client_intent_id_sha256.clone(),
        request_event_id_sha256: root(request_event),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![root(session)],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        estimated_input_tokens: 100,
        provider_payload_bytes: 400,
        provider_capture_request_root_sha256: root(&format!("request:{capture_sequence}")),
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology,
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("topology commit");
    let topology_row = PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: root("bridge"),
        bridge_sequence: Some(capture_sequence),
        record_sha256: Some(root(&format!("record:{capture_sequence}"))),
        capture_epoch_sha256: Some(root("capture epoch")),
        capture_event_sha256: Some(root(&format!("capture event:{capture_sequence}"))),
        capture_receipt_sha256: Some(root(&format!("receipt:{capture_sequence}"))),
        captured_at_unix_ms: Some(capture_sequence),
        session_lineage_sha256: Some(root(session)),
        physical_order_proven: true,
        structure,
        commit,
    };
    MultiSourceJoinLedgerV1::build(&[topology_row], std::slice::from_ref(frame))
        .rows()
        .into_iter()
        .next()
        .expect("joined row")
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
    let support_frame =
        completed_projection_frame("intent-a", "event-a", "session-a", "alpha", 1_500_000_000);
    let future_frame =
        completed_projection_frame("intent-b", "event-b", "session-b", "beta", 2_500_000_000);
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
    assert!(identification.raw_phase_selected_executable.is_none());
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

    assert_eq!(
        candidate.package.admission_candidate_blocker(),
        Some("package_not_active")
    );
    let mut authority_candidate = candidate.clone();
    authority_candidate.package.state = crate::ResponsePackageState::Active;
    assert_eq!(
        authority_candidate.package.admission_candidate_blocker(),
        Some("semantic_applicability_guard_missing")
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
    .expect("admission evaluation");
    assert!(
        admitted.is_none(),
        "a transfer-ready projection remains non-authoritative until applicability negatives exist"
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
        1_500_000_000,
    );
    let future_frame = completed_multi_projection_frame(
        "intent-rich-b",
        "event-rich-b",
        "session-rich-b",
        future_fields,
        future_values,
        2_500_000_000,
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

    assert_eq!(
        candidate.package.admission_candidate_blocker(),
        Some("package_not_active")
    );
    let mut authority_candidate = candidate.clone();
    authority_candidate.package.state = crate::ResponsePackageState::Active;
    assert_eq!(
        authority_candidate.package.admission_candidate_blocker(),
        Some("semantic_applicability_guard_missing")
    );
    let admitted = build_crystallized_admission_snapshot(
        &[candidate],
        "rich-multi-source-test",
        1,
        100,
        30,
        &"a".repeat(64),
        &"b".repeat(64),
    )
    .expect("admission evaluation");
    assert!(
        admitted.is_none(),
        "a rich projection remains non-authoritative until applicability negatives exist"
    );
}
