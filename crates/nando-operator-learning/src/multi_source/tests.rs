use std::collections::BTreeSet;

use crate::{
    SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    opportunity::{OpportunityIntentAuditRowV1, ReducibilityClass},
};
use nando_operator_kernel::{
    AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseArgument, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, SemanticRole, ValueProjectionFormat, sha256_bytes,
};

use super::*;

fn root(label: &str) -> String {
    sha256_bytes(label.as_bytes())
}

fn topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
) -> PreActionTopologyAuditRowV1 {
    let action_event = request_event.replacen("request", "action", 1);
    let topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Complete,
        grounded_output_count: 1,
        output_part_count: 2,
        roles: vec![
            MultiSourceRoleNodeV1 {
                local_role_id: 0,
                source_ordinal: 0,
                value_ordinal: 0,
                type_class: MultiSourceTypeClassV1::Number,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 1,
            },
            MultiSourceRoleNodeV1 {
                local_role_id: 1,
                source_ordinal: 0,
                value_ordinal: 1,
                type_class: MultiSourceTypeClassV1::String,
                container_class: MultiSourceContainerClassV1::Scalar,
                cardinality_class: MultiSourceCardinalityClassV1::One,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 0,
            },
        ],
        role_witnesses: vec![
            MultiSourceRoleWitnessV1 {
                local_role_id: 0,
                value_sha256: root(&format!("value:{action_event}")),
                request_reference_ordinal: Some(0),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: root(&format!("other:{action_event}")),
                request_reference_ordinal: None,
            },
        ],
        relations: vec![
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id: 0,
                target_role_id: 1,
            },
            MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::RequestReferencesRole,
                source_role_id: 0,
                target_role_id: 0,
            },
        ],
    };
    let structure = nando_operator_kernel::LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: root(intent),
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
    .expect("commit");
    PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: root("bridge"),
        bridge_sequence: Some(capture_sequence),
        record_sha256: Some(root(&format!("record:{capture_sequence}"))),
        capture_epoch_sha256: Some(root("capture-epoch")),
        capture_event_sha256: Some(root(&format!("capture-event:{capture_sequence}"))),
        capture_receipt_sha256: Some(root(&format!("receipt:{capture_sequence}"))),
        captured_at_unix_ms: Some(captured_at_unix_ms),
        session_lineage_sha256: Some(root(session)),
        physical_order_proven: true,
        structure,
        commit,
    }
}

fn completed_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
        estimated_input_tokens: 100,
        extractor_version: "test".to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::ActionFunction {
                value: "transport_a".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 1,
                value_type: AtomValueType::Integer,
                source: AtomSource::Action,
                value_sha256: root(&format!("value:{event}")),
            },
            RelationAtom::ActionRoleArgument {
                name: "value".to_owned(),
                slot_id: 1,
                value_type: Some(AtomValueType::Integer),
            },
        ],
        evidence_ref_sha256: root(&format!("evidence:{event}")),
    }
}

#[test]
fn approximate_token_counts_do_not_participate_in_transition_identity() {
    let topology = topology_row("intent-token-drift", "request", "session", 1, 10);
    let mut completed = completed_frame("intent-token-drift", "action", "session", 11);
    completed.estimated_input_tokens = 9_999;

    let joined = MultiSourceJoinLedgerV1::build(&[topology], &[completed]);

    assert_eq!(joined.report().joined_rows, 1);
    assert_eq!(joined.report().accepted_rows, 1);
    assert_eq!(
        joined
            .report()
            .censored
            .get(&MultiSourceJoinCensoredReasonV1::TokenCountMismatch),
        None
    );
}

fn t1_topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
) -> PreActionTopologyAuditRowV1 {
    let mut row = topology_row(
        intent,
        request_event,
        session,
        capture_sequence,
        captured_at_unix_ms,
    );
    row.structure.topology.roles.truncate(1);
    row.structure.topology.role_witnesses.truncate(1);
    row.structure
        .topology
        .relations
        .retain(|edge| edge.source_role_id == 0 && edge.target_role_id == 0);
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("T1 commit");
    row
}

fn t1_continuation_topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
    include_historical_role: bool,
) -> PreActionTopologyAuditRowV1 {
    let mut row = t1_topology_row(
        intent,
        request_event,
        session,
        capture_sequence,
        captured_at_unix_ms,
    );
    row.structure.topology.roles[0].type_class = MultiSourceTypeClassV1::String;
    row.structure.topology.role_witnesses[0].request_reference_ordinal = None;
    row.structure.topology.grounded_output_count = 1;
    row.structure.topology.output_part_count = 1;
    row.structure.topology.relations.clear();
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::ContinuationHandle,
            source_role_id: 0,
            target_role_id: 0,
        });
    if include_historical_role {
        let mut historical_role = row.structure.topology.roles[0].clone();
        historical_role.local_role_id = 1;
        historical_role.temporal_class = MultiSourceTemporalClassV1::Historical;
        historical_role.value_ordinal = 1;
        row.structure.topology.roles.push(historical_role);
        row.structure
            .topology
            .role_witnesses
            .push(MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: root(&format!("historical:{request_event}")),
                request_reference_ordinal: None,
            });
        row.structure
            .topology
            .relations
            .push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::ContinuationHandle,
                source_role_id: 1,
                target_role_id: 1,
            });
        row.structure.topology.output_part_count = 2;
    }
    row.structure.topology.relations.sort();
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("continuation topology");
    row
}

fn t1_continuation_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
    prefix: &str,
) -> RelationFrame {
    let mut frame = t1_completed_frame(intent, event, session, observed_at_unix_ms);
    for atom in &mut frame.atoms {
        match atom {
            RelationAtom::TypedSlot { value_type, .. } => {
                *value_type = AtomValueType::Identifier;
            }
            RelationAtom::ObservationSelector { selector, .. } => {
                *selector = ResponseValueSelector::ContentLinePrefix {
                    prefix: prefix.to_owned(),
                    value_type: AtomValueType::Identifier,
                };
            }
            RelationAtom::ActionRoleArgument { value_type, .. } => {
                *value_type = Some(AtomValueType::Identifier);
            }
            _ => {}
        }
    }
    frame
}

fn t1_value_topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
) -> PreActionTopologyAuditRowV1 {
    let mut row = t1_topology_row(
        intent,
        request_event,
        session,
        capture_sequence,
        captured_at_unix_ms,
    );
    row.structure.topology.roles[0].type_class = MultiSourceTypeClassV1::String;
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("T1 value commit");
    row
}

fn t1_completed_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    let value_root = root(&format!("value:{event}"));
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
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
                selector: nando_operator_kernel::ResponseValueSelector::JsonField {
                    field: "opaque".to_owned(),
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
            RelationAtom::ActionFunction {
                value: "transport_a".to_owned(),
            },
            RelationAtom::ActionRoleArgument {
                name: "value".to_owned(),
                slot_id: 11,
                value_type: Some(AtomValueType::Integer),
            },
        ],
        evidence_ref_sha256: root(&format!("evidence:{event}")),
    }
}

fn t1_completed_custom_tool_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    let mut frame = t1_completed_frame(intent, event, session, observed_at_unix_ms);
    frame
        .atoms
        .retain(|atom| !matches!(atom, RelationAtom::ActionFunction { .. }));
    frame.atoms.extend([
        RelationAtom::ActionCustomTool {
            value: "custom_tool_router".to_owned(),
        },
        RelationAtom::ActionInnerTool {
            value: "transport_b".to_owned(),
        },
        RelationAtom::ActionJsonResultProjection,
    ]);
    frame
}

fn t1_completed_value_projection_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    let value_root = root(&format!("value:{event}"));
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: value_root.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: nando_operator_kernel::ResponseValueSelector::JsonField {
                    field: "opaque".to_owned(),
                    value_type: AtomValueType::String,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: value_root,
            },
            RelationAtom::SlotEquality {
                left_slot: 7,
                right_slot: 11,
            },
            RelationAtom::ActionValueProjection {
                format: nando_operator_kernel::ValueProjectionFormat::PlainText,
                renderer: nando_operator_kernel::CollectionOutputRenderer::Direct,
            },
        ],
        evidence_ref_sha256: root(&format!("evidence:{event}")),
    }
}

fn t1_multi_role_topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
) -> PreActionTopologyAuditRowV1 {
    let mut row = topology_row(
        intent,
        request_event,
        session,
        capture_sequence,
        captured_at_unix_ms,
    );
    let action_event = request_event.replacen("request", "action", 1);
    for (ordinal, role) in row.structure.topology.roles.iter_mut().enumerate() {
        role.type_class = MultiSourceTypeClassV1::String;
        role.value_ordinal = u16::try_from(ordinal).expect("bounded role ordinal");
    }
    row.structure.topology.role_witnesses[0].value_sha256 = root(&format!("first:{action_event}"));
    row.structure.topology.role_witnesses[0].request_reference_ordinal = Some(0);
    row.structure.topology.role_witnesses[1].value_sha256 = root(&format!("second:{action_event}"));
    row.structure.topology.role_witnesses[1].request_reference_ordinal = Some(1);
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::RequestReferencesRole,
            source_role_id: 1,
            target_role_id: 1,
        });
    row.structure.topology.relations.sort();
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("multi-role topology");
    row
}

fn t1_multi_role_projection_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
    first_field: &str,
    second_field: &str,
) -> RelationFrame {
    let first_root = root(&format!("first:{event}"));
    let second_root = root(&format!("second:{event}"));
    let first_selector = ResponseValueSelector::JsonField {
        field: first_field.to_owned(),
        value_type: AtomValueType::String,
    };
    let second_selector = ResponseValueSelector::JsonField {
        field: second_field.to_owned(),
        value_type: AtomValueType::String,
    };
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
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
                selector: first_selector.clone(),
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

fn t1_competing_role_topology_row(
    intent: &str,
    request_event: &str,
    session: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
    equal_values: bool,
) -> PreActionTopologyAuditRowV1 {
    let mut row = topology_row(
        intent,
        request_event,
        session,
        capture_sequence,
        captured_at_unix_ms,
    );
    let action_event = request_event.replacen("request", "action", 1);
    for (ordinal, role) in row.structure.topology.roles.iter_mut().enumerate() {
        role.type_class = MultiSourceTypeClassV1::String;
        role.value_ordinal = u16::try_from(ordinal).expect("bounded role ordinal");
        role.temporal_class = MultiSourceTemporalClassV1::Latest;
    }
    let first_root = if equal_values {
        root(&format!("shared:{action_event}"))
    } else {
        root(&format!("first:{action_event}"))
    };
    let second_root = if equal_values {
        first_root.clone()
    } else {
        root(&format!("second:{action_event}"))
    };
    row.structure.topology.role_witnesses[0].value_sha256 = first_root;
    row.structure.topology.role_witnesses[0].request_reference_ordinal = Some(0);
    row.structure.topology.role_witnesses[1].value_sha256 = second_root;
    row.structure.topology.role_witnesses[1].request_reference_ordinal = Some(1);
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::RequestReferencesRole,
            source_role_id: 1,
            target_role_id: 1,
        });
    row.structure.topology.relations.sort();
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("competing role topology");
    row
}

fn t1_competing_role_projection_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
    equal_values: bool,
) -> RelationFrame {
    let selected_root = if equal_values {
        root(&format!("shared:{event}"))
    } else {
        root(&format!("first:{event}"))
    };
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: root(&format!("frame:{event}")),
        event_id_sha256: root(event),
        client_intent_id_sha256: root(intent),
        session_id_sha256: root(session),
        observed_at_unix_nanos: observed_at_unix_ms.saturating_mul(1_000_000),
        estimated_input_tokens: 100,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![
            RelationAtom::CompletionState {
                value: "completed".to_owned(),
            },
            RelationAtom::TypedSlot {
                slot_id: 7,
                value_type: AtomValueType::String,
                source: AtomSource::Observation,
                value_sha256: selected_root.clone(),
            },
            RelationAtom::UniqueSlot { slot_id: 7 },
            RelationAtom::ObservationSelector {
                slot_id: 7,
                selector: ResponseValueSelector::JsonField {
                    field: "physical_surface".to_owned(),
                    value_type: AtomValueType::String,
                },
            },
            RelationAtom::TypedSlot {
                slot_id: 11,
                value_type: AtomValueType::String,
                source: AtomSource::Action,
                value_sha256: selected_root,
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

fn opportunity(intent: &str, class: ReducibilityClass) -> OpportunityIntentAuditRowV1 {
    OpportunityIntentAuditRowV1 {
        intent_sha256: root(intent),
        input_tokens: 100,
        class,
        verifier_available: true,
        observed_at_unix: 10,
        authority_observed: true,
    }
}

fn request_snapshot(
    topologies: Vec<PreActionTopologyAuditRowV1>,
) -> RequestStructureAuditSnapshotV1 {
    RequestStructureAuditSnapshotV1 {
        rows: Vec::new(),
        stored_turns: u64::try_from(topologies.len()).unwrap_or(u64::MAX),
        stored_topologies: u64::try_from(topologies.len()).unwrap_or(u64::MAX),
        topologies,
        evictions: 0,
        provider_bound_by_construction: true,
        pre_action_context_persisted: true,
    }
}

fn terminal(
    request_event: &str,
    started_at_unix_ms: u64,
    completed_at_unix_ms: u64,
) -> TransportTerminalReceiptV1 {
    TransportTerminalReceiptV1::seal(
        root(request_event),
        started_at_unix_ms.saturating_mul(1_000_000),
        completed_at_unix_ms.saturating_mul(1_000_000),
        200,
    )
    .expect("terminal")
}

#[test]
fn ms3_failure_corpus_accounts_for_every_frozen_topology() {
    let topologies = vec![
        t1_topology_row("missing", "request-missing", "session-a", 1, 1_000),
        t1_topology_row("joined", "request-joined", "session-b", 2, 2_000),
    ];
    let frames = vec![t1_completed_frame(
        "joined",
        "action-joined",
        "session-b",
        2_500,
    )];
    let terminals = vec![
        terminal("request-missing", 990, 1_100),
        terminal("request-joined", 1_990, 2_100),
    ];
    let corpus = build_ms3_failure_corpus_v1(request_snapshot(topologies), frames, terminals);

    assert!(corpus.validate(), "{corpus:#?}");
    assert_eq!(corpus.topology_denominator, 2);
    assert_eq!(corpus.completed_frame_denominator, 1);
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::MissingCompletedObservation),
        Some(&1)
    );
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::UniqueHypothesis),
        Some(&1),
        "{corpus:#?}"
    );
    assert!(!corpus.post_hoc_selection_allowed);
    assert!(!corpus.authority_ready);
}

#[test]
fn ms3_failure_corpus_separates_missing_from_ambiguous_observation() {
    let topologies = vec![
        t1_topology_row("missing", "request-missing", "session-a", 1, 1_000),
        t1_topology_row("ambiguous", "request-ambiguous", "session-b", 2, 2_000),
    ];
    let mut missing = t1_completed_frame("missing", "action-missing", "session-a", 1_500);
    missing
        .atoms
        .retain(|atom| !matches!(atom, RelationAtom::ObservationSelector { .. }));
    let mut ambiguous = t1_completed_frame("ambiguous", "action-ambiguous", "session-b", 2_500);
    let conflicting_selector = ambiguous
        .atoms
        .iter()
        .find_map(|atom| match atom {
            RelationAtom::ObservationSelector { selector, .. } => Some(selector.clone()),
            _ => None,
        })
        .expect("observation selector");
    ambiguous.atoms.extend([
        RelationAtom::TypedSlot {
            slot_id: 8,
            value_type: AtomValueType::Integer,
            source: AtomSource::Observation,
            value_sha256: root("conflicting-observation-value"),
        },
        RelationAtom::ObservationSelector {
            slot_id: 8,
            selector: conflicting_selector,
        },
    ]);

    let corpus = build_ms3_failure_corpus_v1(
        request_snapshot(topologies),
        vec![missing, ambiguous],
        vec![
            terminal("request-missing", 990, 1_100),
            terminal("request-ambiguous", 1_990, 2_100),
        ],
    );

    assert!(corpus.validate(), "{corpus:#?}");
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::SelectedObservationMissing),
        Some(&1)
    );
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::SelectedObservationAmbiguous),
        Some(&1)
    );
}

#[test]
fn repeated_identical_observation_does_not_create_fake_ambiguity() {
    let topology = t1_topology_row("duplicate", "request-duplicate", "session-a", 1, 1_000);
    let mut frame = t1_completed_frame("duplicate", "action-duplicate", "session-a", 1_500);
    let duplicate = frame
        .atoms
        .iter()
        .find(|atom| matches!(atom, RelationAtom::ObservationSelector { .. }))
        .cloned()
        .expect("observation selector");
    frame.atoms.push(duplicate);
    let corpus = build_ms3_failure_corpus_v1(
        request_snapshot(vec![topology]),
        vec![frame],
        vec![terminal("request-duplicate", 990, 1_100)],
    );

    assert!(corpus.validate(), "{corpus:#?}");
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::UniqueHypothesis),
        Some(&1),
        "{corpus:#?}"
    );
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::SelectedObservationAmbiguous),
        None
    );
}

#[test]
fn ms3_failure_corpus_is_byte_stable_under_input_reordering() {
    let topologies = vec![
        t1_topology_row("a", "request-a", "session-a", 1, 1_000),
        t1_topology_row("b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_frame("a", "action-a", "session-a", 1_500),
        t1_completed_frame("b", "action-b", "session-b", 2_500),
    ];
    let mut reversed_topologies = topologies.clone();
    let mut reversed_frames = frames.clone();
    reversed_topologies.reverse();
    reversed_frames.reverse();

    let terminals = vec![
        terminal("request-a", 990, 1_100),
        terminal("request-b", 1_990, 2_100),
    ];
    let mut reversed_terminals = terminals.clone();
    reversed_terminals.reverse();
    let forward = build_ms3_failure_corpus_v1(request_snapshot(topologies), frames, terminals);
    let reversed = build_ms3_failure_corpus_v1(
        request_snapshot(reversed_topologies),
        reversed_frames,
        reversed_terminals,
    );

    assert_eq!(
        serde_json::to_vec(&forward).expect("forward"),
        serde_json::to_vec(&reversed).expect("reversed")
    );
}

#[test]
fn transport_binding_does_not_supersede_distinct_requests_in_one_turn() {
    let topologies = vec![
        t1_topology_row("turn", "request-a", "session", 1, 1_000),
        t1_topology_row("turn", "request-b", "session", 2, 2_000),
    ];
    let frames = vec![t1_completed_frame("turn", "action-a", "session", 1_500)];
    let corpus = build_ms3_failure_corpus_v1(
        request_snapshot(topologies),
        frames,
        vec![
            terminal("request-a", 990, 1_100),
            terminal("request-b", 1_990, 2_100),
        ],
    );

    assert!(corpus.validate(), "{corpus:#?}");
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::MissingCompletedObservation),
        Some(&1)
    );
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::UniqueHypothesis),
        Some(&1)
    );
    assert_eq!(
        corpus
            .rows
            .iter()
            .map(|row| row.request_event_id_sha256.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        2
    );
}

#[test]
fn transport_binding_rejects_overlapping_response_intervals() {
    let topologies = vec![
        t1_topology_row("turn", "request-a", "session", 1, 1_000),
        t1_topology_row("turn", "request-b", "session", 2, 1_050),
    ];
    let frame = t1_completed_frame("turn", "action", "session", 1_100);
    let corpus = build_ms3_failure_corpus_v1(
        request_snapshot(topologies),
        vec![frame],
        vec![
            terminal("request-a", 990, 1_200),
            terminal("request-b", 1_040, 1_250),
        ],
    );

    assert!(corpus.validate(), "{corpus:#?}");
    assert_eq!(
        corpus
            .disposition_counts
            .get(&Ms3FailureDispositionV1::TransportBindingUnresolved),
        Some(&2)
    );
}

#[test]
fn blind_then_reveal_join_preserves_repeated_turn_events_without_overwrite() {
    let topologies = vec![
        topology_row("turn", "request-a", "session", 1, 1_000),
        topology_row("turn", "request-b", "session", 2, 2_000),
    ];
    let frames = vec![
        completed_frame("turn", "action-a", "session", 1_500),
        completed_frame("turn", "action-b", "session", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = ledger.report();
    assert_eq!(report.joined_rows, 2);
    assert_eq!(report.accepted_rows, 2);
    assert_eq!(ledger.rows().len(), 2);
    assert!(ledger.rows().iter().all(|row| row.validate().is_ok()));
}

#[test]
fn one_pre_action_topology_can_ground_multiple_unique_actions() {
    let topologies = vec![topology_row("turn", "request-a", "session", 1, 1_000)];
    let frames = vec![
        completed_frame("turn", "action-a", "session", 1_500),
        completed_frame("turn", "action-b", "session", 1_600),
        completed_frame("turn", "action-a", "session", 1_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = ledger.report();

    assert_eq!(report.joined_rows, 2);
    assert_eq!(report.accepted_rows, 2);
    assert_eq!(report.duplicate_idempotent, 1);
    assert_eq!(
        report
            .censored
            .get(&MultiSourceJoinCensoredReasonV1::PreActionOrderInvalid),
        None
    );
    assert!(
        ledger
            .rows()
            .iter()
            .all(|row| row.topology_commitment_root_sha256
                == topologies[0].commit.commitment_root_sha256)
    );
}

#[test]
fn topology_captured_after_action_never_joins() {
    let topologies = vec![topology_row("turn", "request-a", "session", 1, 2_000)];
    let frames = vec![completed_frame("turn", "action-a", "session", 1_500)];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().is_empty());
    assert_eq!(
        ledger
            .report()
            .censored
            .get(&MultiSourceJoinCensoredReasonV1::PreActionOrderInvalid),
        Some(&1)
    );
}

#[test]
fn factorizer_keeps_applicability_pre_action_and_effect_separate() {
    let ledger = MultiSourceJoinLedgerV1::build(
        &[topology_row("turn", "request-a", "session", 1, 1_000)],
        &[completed_frame("turn", "action-a", "session", 1_500)],
    );
    let factorized = factor_multi_source_row_v1(&ledger.rows()[0]);
    assert_eq!(
        factorized.pre_action_shape,
        PreActionShapeClassV1::OneOutputManyScalarRoles
    );
    assert_eq!(
        factorized.completed_effect,
        CompletedEffectFormV1::SingleRoleProjection
    );
}

#[test]
fn factorizer_preserves_two_independent_action_role_inputs() {
    let mut frame = completed_frame("turn", "action-a", "session", 1_500);
    frame.atoms.push(RelationAtom::TypedSlot {
        slot_id: 2,
        value_type: AtomValueType::String,
        source: AtomSource::Action,
        value_sha256: root("other:action-a"),
    });
    frame.atoms.push(RelationAtom::ActionRoleArgument {
        name: "other".to_owned(),
        slot_id: 2,
        value_type: Some(AtomValueType::String),
    });
    let ledger = MultiSourceJoinLedgerV1::build(
        &[topology_row("turn", "request-a", "session", 1, 1_000)],
        &[frame],
    );
    let joined = &ledger.rows()[0];
    assert_eq!(
        joined
            .effect_atoms
            .iter()
            .filter(|atom| matches!(atom, CompletedEffectAtomV1::RoleInputSlot { .. }))
            .count(),
        2
    );
    assert_eq!(
        factor_multi_source_row_v1(joined).completed_effect,
        CompletedEffectFormV1::MultiRoleRendering
    );
}

#[test]
fn factorizer_does_not_split_one_role_on_compatible_type_tags() {
    let ledger = MultiSourceJoinLedgerV1::build(
        &[topology_row("turn", "request-a", "session", 1, 1_000)],
        &[completed_frame("turn", "action-a", "session", 1_500)],
    );
    let mut joined = ledger.rows()[0].clone();
    let slot_id = joined
        .effect_atoms
        .iter()
        .find_map(|atom| match atom {
            CompletedEffectAtomV1::RoleInputSlot { slot_id, .. } => Some(*slot_id),
            _ => None,
        })
        .expect("role input slot");
    joined
        .effect_atoms
        .push(CompletedEffectAtomV1::RoleInputSlot {
            slot_id,
            value_type: Some(AtomValueType::Identifier),
        });
    joined.effect_atoms.sort_unstable();
    joined.effect_atoms.dedup();

    assert_eq!(
        factor_multi_source_row_v1(&joined).completed_effect,
        CompletedEffectFormV1::SingleRoleProjection
    );
}

#[test]
fn marginal_ledger_buys_each_intent_once_and_subtracts_active() {
    let ledger = MultiSourceJoinLedgerV1::build(
        &[
            topology_row("turn-a", "request-a", "session-a", 1, 1_000),
            topology_row("turn-b", "request-b", "session-b", 2, 2_000),
        ],
        &[
            completed_frame("turn-a", "action-a", "session-a", 1_500),
            completed_frame("turn-b", "action-b", "session-b", 2_500),
        ],
    );
    let rows = ledger
        .rows()
        .iter()
        .map(factor_multi_source_row_v1)
        .collect::<Vec<_>>();
    let active = BTreeSet::from([root("turn-a")]);
    let snapshot = build_coverage_opportunity_snapshot_v1(&rows, &active, root("epoch"));
    assert!(snapshot.validate());
    assert_eq!(snapshot.total.intents, 2);
    assert_eq!(snapshot.already_active.intents, 1);
    assert_eq!(snapshot.unresolved.intents, 1);
    assert_eq!(snapshot.duplicate_marginal_purchase, 0);
}

#[test]
fn live_snapshot_is_order_independent_and_subtracts_active_overlap() {
    let topology_a = topology_row("turn-a", "request-a", "session-a", 1, 1_000);
    let topology_b = topology_row("turn-b", "request-b", "session-b", 2, 2_000);
    let frame_a = completed_frame("turn-a", "action-a", "session-a", 1_500);
    let frame_b = completed_frame("turn-b", "action-b", "session-b", 2_500);
    let opportunity_a = opportunity("turn-a", ReducibilityClass::CpuVerified);
    let opportunity_b = opportunity("turn-b", ReducibilityClass::UnexploredMultiSource);

    let forward = build_live_multi_source_discovery_snapshot_v3(
        vec![opportunity_a.clone(), opportunity_b.clone()],
        request_snapshot(vec![topology_a.clone(), topology_b.clone()]),
        vec![frame_a.clone(), frame_b.clone()],
    );
    let reversed = build_live_multi_source_discovery_snapshot_v3(
        vec![opportunity_b, opportunity_a],
        request_snapshot(vec![topology_b, topology_a]),
        vec![frame_b, frame_a],
    );

    assert!(forward.validate());
    assert_eq!(
        forward.blocker,
        LiveMultiSourceDiscoveryBlockerV1::T1CandidateGenerationBlocked
    );
    assert_eq!(forward.join.joined_rows, 2);
    assert_eq!(forward.opportunity.already_active.intents, 1);
    assert_eq!(forward.opportunity.unresolved.intents, 1);
    assert_eq!(
        serde_json::to_vec(&forward).expect("snapshot serializes"),
        serde_json::to_vec(&reversed).expect("snapshot serializes")
    );
}

#[test]
fn live_snapshot_reports_the_first_missing_signal_boundary() {
    let no_topology = build_live_multi_source_discovery_snapshot_v3(
        Vec::new(),
        request_snapshot(Vec::new()),
        Vec::new(),
    );
    assert!(no_topology.validate());
    assert_eq!(
        no_topology.blocker,
        LiveMultiSourceDiscoveryBlockerV1::NoPreActionTopology
    );

    let no_frame = build_live_multi_source_discovery_snapshot_v3(
        vec![opportunity(
            "turn",
            ReducibilityClass::UnexploredMultiSource,
        )],
        request_snapshot(vec![topology_row("turn", "request", "session", 1, 1_000)]),
        Vec::new(),
    );
    assert!(no_frame.validate());
    assert_eq!(
        no_frame.blocker,
        LiveMultiSourceDiscoveryBlockerV1::NoCompletedRelationFrame
    );
    assert!(!no_frame.identification_ready);
    assert!(!no_frame.authority_ready);
}

#[test]
fn t1_identification_uses_one_support_and_one_independent_future() {
    let topologies = vec![
        t1_topology_row("turn-a", "request-a", "session-a", 1, 1_000),
        t1_topology_row("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_frame("turn-a", "action-a", "session-a", 1_500),
        t1_completed_frame("turn-b", "action-b", "session-b", 2_500),
    ];
    assert_eq!(crate::ground_roles(&frames[0]).len(), 1);
    let candidates =
        crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(&frames[0]));
    assert!(!candidates.is_empty());
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.validate().is_ok()),
        "{candidates:#?}"
    );
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.independent_future_lineages, 1);
    assert_eq!(report.wrong_role_bindings, 0);
    assert_eq!(report.negative_accepts, 0);
    assert!(report.exact_transfer_parity);
    let basis = report.proof_basis.as_ref().expect("sealed runtime basis");
    assert!(basis.validate());
    assert_eq!(
        basis.support_capture_frame_ids_sha256,
        [frames[0].frame_id_sha256.clone()]
    );
    assert_eq!(
        basis.future_capture_frame_ids_sha256,
        [frames[1].frame_id_sha256.clone()]
    );
    assert!(!report.runtime_actor_verifier_parity);
    assert!(!report.execution_authority);

    let snapshot = build_live_multi_source_discovery_snapshot_v3(
        vec![
            opportunity("turn-a", ReducibilityClass::UnexploredMultiSource),
            opportunity("turn-b", ReducibilityClass::UnexploredMultiSource),
        ],
        request_snapshot(topologies),
        frames.clone(),
    );
    assert!(snapshot.validate(), "{snapshot:#?}");
    assert_eq!(
        snapshot.blocker,
        LiveMultiSourceDiscoveryBlockerV1::T1TransferReady
    );
    assert!(snapshot.identification_ready);
    assert!(snapshot.transfer_ready);
    assert!(!snapshot.authority_ready);

    let mut reversed_rows = ledger.rows();
    reversed_rows.reverse();
    let mut reversed_frames = frames;
    reversed_frames.reverse();
    let reversed = identify_multi_source_t1_operator_v1(
        &reversed_rows,
        &reversed_frames,
        &BTreeSet::new(),
        root("T1 epoch"),
    );
    assert_eq!(
        serde_json::to_vec(&report).expect("report"),
        serde_json::to_vec(&reversed).expect("reversed report")
    );
}

#[test]
fn ambiguous_roles_build_version_space_and_passive_distinguishing_probe() {
    let topologies = vec![t1_competing_role_topology_row(
        "ambiguous",
        "request-ambiguous",
        "session-a",
        1,
        1_000,
        true,
    )];
    let frames = vec![t1_competing_role_projection_frame(
        "ambiguous",
        "action-ambiguous",
        "session-a",
        1_500,
        true,
    )];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("ambiguous role epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(report.state, MultiSourceT1IdentificationStateV1::Ambiguous);
    assert_eq!(report.candidate_programs, 2, "{report:#?}");
    assert_eq!(report.semantic_classes_remaining, 2, "{report:#?}");
    assert_eq!(report.support_rows, 1, "{report:#?}");
    let probe = report.passive_probe.as_ref().expect("distinguishing probe");
    assert_eq!(probe.expected_partition_gain, 1);
    assert_eq!(probe.estimated_cost_units, 1);
    assert!(report.candidate_freeze.is_none());
    assert!(!report.execution_authority);
}

#[test]
fn distinguishing_observation_selects_role_then_requires_independent_future() {
    let topologies = vec![
        t1_competing_role_topology_row(
            "support-equal",
            "request-support-equal",
            "session-a",
            1,
            1_000,
            true,
        ),
        t1_competing_role_topology_row(
            "support-split",
            "request-support-split",
            "session-b",
            2,
            2_000,
            false,
        ),
        t1_competing_role_topology_row("future", "request-future", "session-c", 3, 3_000, false),
    ];
    let frames = vec![
        t1_competing_role_projection_frame(
            "support-equal",
            "action-support-equal",
            "session-a",
            1_500,
            true,
        ),
        t1_competing_role_projection_frame(
            "support-split",
            "action-support-split",
            "session-b",
            2_500,
            false,
        ),
        t1_competing_role_projection_frame("future", "action-future", "session-c", 3_500, false),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("distinguished role epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady,
        "{report:#?}"
    );
    assert_eq!(report.candidate_programs, 2, "{report:#?}");
    assert_eq!(report.support_rows, 2, "{report:#?}");
    assert_eq!(report.support_lineages, 2, "{report:#?}");
    assert_eq!(report.independent_future_rows, 1, "{report:#?}");
    assert_eq!(report.independent_future_lineages, 1, "{report:#?}");
    assert_eq!(report.wrong_role_bindings, 0, "{report:#?}");
    assert_eq!(report.negative_accepts, 0, "{report:#?}");
    assert!(report.exact_transfer_parity);
    assert!(matches!(
        report
            .canonical_program
            .as_ref()
            .map(|program| &program.operation),
        Some(ResponseOperation::ProjectSelectedValue {
            selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal: 0, .. },
            ..
        })
    ));
    assert!(!report.execution_authority);
}

#[test]
fn multi_role_projection_identifies_one_law_across_renamed_surfaces() {
    let topologies = vec![
        t1_multi_role_topology_row("turn-a", "request-a", "session-a", 1, 1_000),
        t1_multi_role_topology_row("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_multi_role_projection_frame(
            "turn-a",
            "action-a",
            "session-a",
            1_500,
            "city",
            "temperature",
        ),
        t1_multi_role_projection_frame(
            "turn-b",
            "action-b",
            "session-b",
            2_500,
            "place",
            "degrees",
        ),
    ];
    let physical =
        crate::synthesis::enumerate_response_program_candidates(std::slice::from_ref(&frames[0]));
    assert_eq!(physical.len(), 1, "{physical:#?}");
    assert!(crate::synthesis::program_is_consistent(
        &physical[0],
        &frames[0]
    ));
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).completed_effect
            == CompletedEffectFormV1::MultiRoleRendering
    }));

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("multi-role projection epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.wrong_role_bindings, 0);
    assert!(report.exact_transfer_parity);
    let program = report
        .canonical_program
        .as_ref()
        .expect("canonical program");
    let ResponseOperation::ProjectSelectedValue {
        selector, renderer, ..
    } = &program.operation
    else {
        panic!("multi-role projection expected");
    };
    assert!(matches!(
        selector,
        ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal: 0, .. }
    ));
    assert!(matches!(
        renderer,
        nando_operator_kernel::CollectionOutputRenderer::RenderSequence { segments }
            if segments.iter().any(|segment| matches!(
                segment,
                ResponseRenderSegment::Selected {
                    selector: ResponseValueSelector::RequestReferencedJsonFieldOrdinal {
                        ordinal: 1,
                        ..
                    },
                    ..
                }
            ))
    ));
}

#[test]
fn active_physical_continuation_program_is_removed_from_t1_discovery() {
    let topologies = vec![
        t1_continuation_topology_row("turn-a", "request-a", "session-a", 1, 1_000, false),
        t1_continuation_topology_row("turn-b", "request-b", "session-b", 2, 2_000, false),
    ];
    let frames = vec![
        t1_continuation_frame("turn-a", "action-a", "session-a", 1_500, "surface A "),
        t1_continuation_frame("turn-b", "action-b", "session-b", 2_500, "surface B "),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let baseline = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("active protocol baseline"),
    );
    let mut physical = baseline
        .canonical_program
        .clone()
        .expect("canonical program");
    let ResponseOperation::FunctionCallFromRoles { selector, .. } = &mut physical.operation else {
        panic!("function program expected");
    };
    let ResponseValueSelector::ContinuationHandle { value_type } = selector else {
        panic!("continuation selector expected");
    };
    *selector = ResponseValueSelector::ContentLinePrefix {
        prefix: "physical surface ".to_owned(),
        value_type: *value_type,
    };
    let active_root = active_t1_protocol_mode_root_v1(&physical).expect("active protocol root");
    assert_eq!(
        Some(active_root.as_str()),
        baseline.selected_protocol_mode_root_sha256.as_deref(),
        "physical normalization diverged: {physical:#?}\n{baseline:#?}"
    );

    let report = identify_multi_source_t1_operator_with_active_protocols_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        &BTreeSet::from([active_root]),
        root("active protocol filtered"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::NoEligibleCohort
    );
    assert_eq!(
        report.blocker.as_deref(),
        Some("all_supported_t1_protocol_modes_already_active")
    );
}

#[test]
fn t1_projection_can_select_one_role_from_a_multi_scalar_state() {
    let topologies = vec![
        topology_row("turn-a", "request-a", "session-a", 1, 1_000),
        topology_row("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_frame("turn-a", "action-a", "session-a", 1_500),
        t1_completed_frame("turn-b", "action-b", "session-b", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).pre_action_shape
            == PreActionShapeClassV1::OneOutputManyScalarRoles
    }));

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("multi-scalar T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.wrong_role_bindings, 0);
    assert_eq!(report.negative_accepts, 0);
}

#[test]
fn t1_value_projection_derives_role_input_from_slot_transfer() {
    let topologies = vec![
        t1_value_topology_row("turn-a", "request-a", "session-a", 1, 1_000),
        t1_value_topology_row("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_value_projection_frame("turn-a", "action-a", "session-a", 1_500),
        t1_completed_value_projection_frame("turn-b", "action-b", "session-b", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).completed_effect
            == CompletedEffectFormV1::SingleRoleProjection
    }));

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("value projection T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.wrong_role_bindings, 0);
}

#[test]
fn t1_value_projection_selects_a_source_neutral_program_across_renamed_fields() {
    let topologies = vec![
        t1_value_topology_row("turn-a", "request-a", "session-a", 1, 1_000),
        t1_value_topology_row("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let mut support = t1_completed_value_projection_frame("turn-a", "action-a", "session-a", 1_500);
    let mut future = t1_completed_value_projection_frame("turn-b", "action-b", "session-b", 2_500);
    set_observed_json_field(&mut support, "alpha");
    set_observed_json_field(&mut future, "renamed_beta");
    let frames = vec![support, future];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("renamed field T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    let Some(ResponseProgram {
        operation:
            ResponseOperation::ProjectSelectedValue {
                selector:
                    ResponseValueSelector::RequestReferencedJsonFieldOrdinal { ordinal: 0, .. },
                ..
            },
        ..
    }) = report.canonical_program.as_ref()
    else {
        panic!("source-neutral program missing: {report:#?}");
    };
    let encoded = serde_json::to_string(&report).expect("report");
    assert!(!encoded.contains("alpha"));
    assert!(!encoded.contains("renamed_beta"));
}

#[test]
fn t1_value_projection_refuses_physical_program_without_pre_action_witness() {
    let mut topology = t1_topology_row("turn-a", "request-a", "session-a", 1, 1_000);
    topology.structure.topology.role_witnesses.clear();
    topology.commit = PreActionTopologyCommitV1::seal(
        &topology.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        1,
    )
    .expect("legacy commit");
    let frame = t1_completed_value_projection_frame("turn-a", "action-a", "session-a", 1_500);
    let ledger = MultiSourceJoinLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
    );

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        std::slice::from_ref(&frame),
        &BTreeSet::new(),
        root("witnessless T1 epoch"),
    );

    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::NoEligibleCohort
    );
    assert!(report.canonical_program.is_none());
}

#[test]
fn witnessless_legacy_mass_cannot_hide_a_smaller_fresh_identifiable_cohort() {
    let mut legacy_topology =
        t1_value_topology_row("legacy", "request-legacy", "legacy-session", 1, 1_000);
    legacy_topology.structure.topology.role_witnesses.clear();
    legacy_topology.commit = PreActionTopologyCommitV1::seal(
        &legacy_topology.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        1,
    )
    .expect("legacy commit");
    let mut legacy_frame =
        t1_completed_value_projection_frame("legacy", "action-legacy", "legacy-session", 1_500);
    legacy_frame.estimated_input_tokens = 50_000;
    let topologies = vec![
        legacy_topology,
        t1_value_topology_row("turn-a", "request-a", "session-a", 2, 2_000),
        t1_value_topology_row("turn-b", "request-b", "session-b", 3, 3_000),
    ];
    let frames = vec![
        legacy_frame,
        t1_completed_value_projection_frame("turn-a", "action-a", "session-a", 2_500),
        t1_completed_value_projection_frame("turn-b", "action-b", "session-b", 3_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("fresh witness epoch"),
    );

    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.selected_marginal_input_tokens, 200);
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
}

#[test]
fn unmatched_legacy_witness_cannot_starve_a_fresh_identifiable_cohort() {
    let mut legacy = t1_value_topology_row("legacy", "request-legacy", "legacy-session", 1, 1_000);
    legacy.structure.topology.role_witnesses[0].value_sha256 = root("unmatched legacy value");
    legacy.commit = PreActionTopologyCommitV1::seal(
        &legacy.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        1,
    )
    .expect("legacy commit");
    let mut legacy_frame =
        t1_completed_value_projection_frame("legacy", "action-legacy", "legacy-session", 1_500);
    legacy_frame.estimated_input_tokens = 50_000;
    let topologies = vec![
        legacy,
        t1_value_topology_row("turn-a", "request-a", "session-a", 2, 2_000),
        t1_value_topology_row("turn-b", "request-b", "session-b", 3, 3_000),
    ];
    let frames = vec![
        legacy_frame,
        t1_completed_value_projection_frame("turn-a", "action-a", "session-a", 2_500),
        t1_completed_value_projection_frame("turn-b", "action-b", "session-b", 3_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("unmatched legacy witness epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.selected_marginal_input_tokens, 200);
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
}

fn set_observed_json_field(frame: &mut RelationFrame, field: &str) {
    for atom in &mut frame.atoms {
        if let RelationAtom::ObservationSelector { selector, .. } = atom {
            let value_type = match selector {
                ResponseValueSelector::JsonField { value_type, .. } => *value_type,
                _ => continue,
            };
            *selector = ResponseValueSelector::JsonField {
                field: field.to_owned(),
                value_type,
            };
        }
    }
}

#[test]
fn t1_projection_can_select_the_latest_role_from_multiple_outputs() {
    let make_topology = |intent, request, session, sequence, captured_at| {
        let mut row = topology_row(intent, request, session, sequence, captured_at);
        row.structure.topology.roles[0].source_ordinal = 0;
        row.structure.topology.roles[0].temporal_class = MultiSourceTemporalClassV1::Historical;
        row.structure.topology.roles[0].structural_flags = 0;
        row.structure.topology.roles[1].source_ordinal = 1;
        row.structure.topology.roles[1].temporal_class = MultiSourceTemporalClassV1::Latest;
        row.structure.topology.roles[1].type_class = MultiSourceTypeClassV1::Number;
        row.structure.topology.roles[1].structural_flags = 0;
        row.structure.topology.role_witnesses.swap(0, 1);
        for (index, witness) in row.structure.topology.role_witnesses.iter_mut().enumerate() {
            witness.local_role_id = u16::try_from(index).expect("role id");
            witness.request_reference_ordinal = None;
        }
        row.structure.topology.role_witnesses[0].value_sha256 =
            row.structure.topology.role_witnesses[1]
                .value_sha256
                .clone();
        row.structure
            .topology
            .relations
            .retain(|edge| edge.relation != MultiSourceRelationKindV1::RequestReferencesRole);
        let mut additional_role = row.structure.topology.roles[1].clone();
        additional_role.local_role_id = 2;
        additional_role.value_ordinal = 2;
        additional_role.type_class = MultiSourceTypeClassV1::String;
        row.structure.topology.roles.push(additional_role);
        row.structure
            .topology
            .role_witnesses
            .push(MultiSourceRoleWitnessV1 {
                local_role_id: 2,
                value_sha256: root(&format!("latest-other:{request}")),
                request_reference_ordinal: None,
            });
        row.structure.topology.output_part_count = 3;
        row.structure
            .topology
            .relations
            .push(MultiSourceRelationEdgeV1 {
                relation: MultiSourceRelationKindV1::Precedes,
                source_role_id: 1,
                target_role_id: 2,
            });
        row.structure.topology.relations.sort();
        row.commit = PreActionTopologyCommitV1::seal(
            &row.structure,
            MultiSourceEvidenceOriginV1::FreshLive,
            root("extractor"),
            root("config"),
            sequence,
        )
        .expect("multi-output T1 commit");
        row
    };
    let topologies = vec![
        make_topology("turn-a", "request-a", "session-a", 1, 1_000),
        make_topology("turn-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_frame("turn-a", "action-a", "session-a", 1_500),
        t1_completed_frame("turn-b", "action-b", "session-b", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).pre_action_shape
            == PreActionShapeClassV1::ManyOutputsLatestRelevantRole
    }));

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("multi-output T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.wrong_role_bindings, 0);
    assert_eq!(report.negative_accepts, 0);
    let Some(ResponseProgram {
        operation:
            ResponseOperation::FunctionCallFromRoles {
                selector:
                    ResponseValueSelector::LatestTurnOutputScalarOrdinal {
                        scalar_ordinal: 1, ..
                    },
                ..
            },
        ..
    }) = report.canonical_program.as_ref()
    else {
        panic!("latest-output ordinal program missing: {report:#?}");
    };
}

#[test]
fn t1_continuation_surface_compiles_to_semantic_handle_role() {
    let topologies = vec![
        t1_continuation_topology_row("turn-a", "request-a", "session-a", 1, 1_000, true),
        t1_continuation_topology_row("turn-b", "request-b", "session-b", 2, 2_000, true),
    ];
    let frames = vec![
        t1_continuation_frame("turn-a", "action-a", "session-a", 1_500, "surface A "),
        t1_continuation_frame("turn-b", "action-b", "session-b", 2_500, "surface B "),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("continuation T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    let Some(ResponseProgram {
        operation:
            ResponseOperation::FunctionCallFromRoles {
                selector: ResponseValueSelector::ContinuationHandle { .. },
                arguments,
                ..
            },
        ..
    }) = report.canonical_program.as_ref()
    else {
        panic!("semantic continuation program missing: {report:#?}");
    };
    assert!(arguments.iter().any(|argument| {
        matches!(
            argument,
            ResponseArgument::Role {
                role: SemanticRole::ContinuationHandle,
                ..
            }
        )
    }));
    let encoded = serde_json::to_string(&report).expect("report");
    assert!(!encoded.contains("surface A"));
    assert!(!encoded.contains("surface B"));
}

#[test]
fn t1_continuation_transfers_across_pre_action_topologies() {
    let topologies = vec![
        t1_continuation_topology_row(
            "support-a",
            "request-support-a",
            "support-session",
            1,
            1_000,
            true,
        ),
        t1_continuation_topology_row(
            "support-b",
            "request-support-b",
            "support-session",
            2,
            2_000,
            true,
        ),
        t1_continuation_topology_row(
            "future",
            "request-future",
            "future-session",
            3,
            3_000,
            false,
        ),
    ];
    let frames = vec![
        t1_continuation_frame(
            "support-a",
            "action-support-a",
            "support-session",
            1_500,
            "surface A ",
        ),
        t1_continuation_frame(
            "support-b",
            "action-support-b",
            "support-session",
            2_500,
            "surface B ",
        ),
        t1_continuation_frame(
            "future",
            "action-future",
            "future-session",
            3_500,
            "surface C ",
        ),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let rows = ledger.rows();
    let multi_role = rows
        .iter()
        .find(|row| row.topology.roles.len() == 2)
        .expect("multi-role topology");
    let single_role = rows
        .iter()
        .find(|row| row.topology.roles.len() == 1)
        .expect("single-role topology");
    assert_eq!(
        factor_multi_source_row_v1(multi_role).pre_action_shape,
        PreActionShapeClassV1::OneOutputManyScalarRoles
    );
    assert_eq!(
        factor_multi_source_row_v1(single_role).pre_action_shape,
        PreActionShapeClassV1::SingleRoleProjection
    );

    let report = identify_multi_source_t1_operator_v1(
        &rows,
        &frames,
        &BTreeSet::new(),
        root("cross-topology continuation epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.support_reuse_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.independent_future_lineages, 1);
    assert_eq!(report.wrong_role_bindings, 0);
    assert!(report.exact_transfer_parity);
    assert_eq!(
        report
            .proof_basis
            .as_ref()
            .expect("proof basis")
            .future_capture_frame_ids_sha256,
        [frames[2].frame_id_sha256.clone()]
    );
}

#[test]
fn t1_identifies_protocol_modes_without_splitting_the_common_effect_law() {
    let topologies = vec![
        t1_topology_row("wait-a", "request-wait-a", "session-wait-a", 1, 1_000),
        t1_topology_row("wait-b", "request-wait-b", "session-wait-b", 2, 2_000),
        t1_topology_row("custom-a", "request-custom-a", "session-custom-a", 3, 3_000),
        t1_topology_row("custom-b", "request-custom-b", "session-custom-b", 4, 4_000),
    ];
    let frames = vec![
        t1_completed_frame("wait-a", "action-wait-a", "session-wait-a", 1_500),
        t1_completed_frame("wait-b", "action-wait-b", "session-wait-b", 2_500),
        t1_completed_custom_tool_frame("custom-a", "action-custom-a", "session-custom-a", 3_500),
        t1_completed_custom_tool_frame("custom-b", "action-custom-b", "session-custom-b", 4_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let factorized = ledger
        .rows()
        .iter()
        .map(factor_multi_source_row_v1)
        .collect::<Vec<_>>();
    assert!(
        factorized
            .windows(2)
            .all(|rows| rows[0].applicability_shape_root_sha256
                == rows[1].applicability_shape_root_sha256)
    );

    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("mixed protocol T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.candidate_programs, 1);
    assert!(report.selected_shape_root_sha256.is_some());
    assert!(report.selected_protocol_mode_root_sha256.is_some());
    assert_eq!(report.wrong_role_bindings, 0);
    assert_eq!(report.negative_accepts, 0);
}

#[test]
fn t1_never_merges_distinct_function_capabilities_into_one_protocol_mode() {
    let topologies = vec![
        t1_topology_row("a-1", "request-a-1", "session-a-1", 1, 1_000),
        t1_topology_row("a-2", "request-a-2", "session-a-2", 2, 2_000),
        t1_topology_row("b-1", "request-b-1", "session-b-1", 3, 3_000),
        t1_topology_row("b-2", "request-b-2", "session-b-2", 4, 4_000),
    ];
    let mut frames = vec![
        t1_completed_frame("a-1", "action-a-1", "session-a-1", 1_500),
        t1_completed_frame("a-2", "action-a-2", "session-a-2", 2_500),
        t1_completed_frame("b-1", "action-b-1", "session-b-1", 3_500),
        t1_completed_frame("b-2", "action-b-2", "session-b-2", 4_500),
    ];
    for frame in &mut frames[2..] {
        for atom in &mut frame.atoms {
            if let RelationAtom::ActionFunction { value } = atom {
                *value = "transport_b".to_owned();
            }
        }
    }
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("distinct function modes T1 epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.independent_future_rows, 1);
    assert_eq!(report.candidate_programs, 1);
    assert_eq!(report.wrong_role_bindings, 0);
}

#[test]
fn t1_identification_never_counts_support_lineage_reuse_as_future() {
    let topologies = vec![
        t1_topology_row("turn-a", "request-a", "session", 1, 1_000),
        t1_topology_row("turn-b", "request-b", "session", 2, 2_000),
    ];
    let frames = vec![
        t1_completed_frame("turn-a", "action-a", "session", 1_500),
        t1_completed_frame("turn-b", "action-b", "session", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("support reuse epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::FrozenAwaitingIndependentFuture
    );
    assert_eq!(report.support_rows, 1);
    assert_eq!(report.support_reuse_rows, 1);
    assert_eq!(report.independent_future_rows, 0);
    assert!(
        report.proof_basis.as_ref().is_some_and(
            |basis| basis.validate() && basis.future_capture_frame_ids_sha256.is_empty()
        )
    );
    assert!(!report.exact_transfer_parity);
}
