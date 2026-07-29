use std::collections::BTreeSet;

use crate::{
    SOURCE_NEUTRAL_EXTRACTOR_VERSION, TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1, TypedExecutionStage,
    TypedExecutionStageReceipt, VerifiedDeltaOutcome, VerifiedDeltaReceipt, VerifiedDeltaRelation,
    VerifiedDeltaRelationState,
    opportunity::{OpportunityIntentAuditRowV1, ReducibilityClass},
};
use nando_core::wave::{CircuitSynthesisConfig, OperatorGrokkingConfig};
use nando_operator_kernel::{
    AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceRoleWitnessV1, MultiSourceTemporalClassV1, MultiSourceTypeClassV1,
    PreActionMultiSourceTopologyV1, PreActionTopologyCommitV1, RELATION_FRAME_SCHEMA, RelationAtom,
    RelationFrame, ResponseArgument, ResponseOperation, ResponseProgram, ResponseRenderSegment,
    ResponseValueSelector, SemanticRole, ValueProjectionFormat, sha256_bytes, valid_nonzero_sha256,
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
                request_reference_ordinal_candidates: Vec::new(),
            },
            MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: root(&format!("other:{action_event}")),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
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
                request_reference_ordinal_candidates: Vec::new(),
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

fn acquisition_contract(
    max_new_topology_rows: u64,
    max_elapsed_seconds: u64,
) -> Ms3LinkedFrameAcquisitionContractV1 {
    Ms3LinkedFrameAcquisitionContractV1::seal(
        root("topology-prefix"),
        1_832,
        1,
        max_new_topology_rows,
        max_elapsed_seconds,
    )
    .expect("acquisition contract")
}

fn acquisition_contract_v2(
    max_eligible_topology_rows: u64,
    max_raw_topology_rows: u64,
) -> Ms3LinkedFrameAcquisitionContractV1 {
    Ms3LinkedFrameAcquisitionContractV1::seal_v2(
        root("topology-prefix-v2"),
        1_832,
        1,
        max_eligible_topology_rows,
        max_raw_topology_rows,
        60,
        5,
    )
    .expect("V2 acquisition contract")
}

fn unattributed_topology_row(
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
    row.structure.provider_bound_turn_identity = false;
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("unattributed topology commit");
    row
}

fn censored_topology_row(
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
    row.structure.topology = PreActionMultiSourceTopologyV1 {
        extraction_status: MultiSourceExtractionStatusV1::Censored {
            reason: "ambiguous_request_reference".to_owned(),
        },
        grounded_output_count: 0,
        output_part_count: 0,
        roles: Vec::new(),
        role_witnesses: Vec::new(),
        relations: Vec::new(),
    };
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("censored topology commit");
    row
}

fn frozen_unique_law_fixture(
    label: &str,
    session: &str,
    support_sequence: u64,
    contract_watermark: u64,
) -> FrozenVersionSpaceEnvelopeV1 {
    let intent = format!("{label}-support");
    let request = format!("request-{label}-support");
    let action = format!("action-{label}-support");
    let topology = t1_topology_row(&intent, &request, session, support_sequence, 1_000);
    let frame = t1_completed_frame(&intent, &action, session, 1_500);
    let terminal = terminal(&request, 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(256, 60),
        2,
        vec![topology.clone()],
        vec![frame.clone()],
        vec![terminal.clone()],
    );
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
        std::slice::from_ref(&terminal),
    );
    let bound = &ledger.bound_for_topology(&topology.commit.commitment_root_sha256)[0];
    prepare_ms3_frozen_version_space_v1(&report, bound, &frame)
        .expect("prepared fixture version space")
        .seal(
            contract_watermark,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen fixture version space")
}

fn independent_future_fixture(
    frozen: &FrozenVersionSpaceEnvelopeV1,
    label: &str,
    session: &str,
    capture_sequence: u64,
    pass: bool,
) -> Ms3IndependentFutureEnvelopeV1 {
    let intent = format!("{label}-future");
    let request = format!("request-{label}-future");
    let action = format!("action-{label}-future");
    let topology = t1_topology_row(&intent, &request, session, capture_sequence, 2_000);
    let prediction = predict_ms3_unique_law_v1(frozen, &topology, 2_050_000_000)
        .expect("fixture prediction")
        .expect("applicable fixture future");
    let mut frame = t1_completed_frame(&intent, &action, session, 2_500);
    if !pass {
        frame.verifier_label = Some(false);
    }
    let terminal = terminal(&request, 1_990, 2_100);
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
        std::slice::from_ref(&terminal),
    );
    let bound = &ledger.bound_for_topology(&topology.commit.commitment_root_sha256)[0];
    seal_ms3_independent_future_v1(
        frozen,
        &prediction,
        &root(&format!("{label}-applicability")),
        2_060_000_000,
        bound,
        &frame,
    )
    .expect("fixture independent future")
}

fn north_star_budget() -> NorthStarBudgetV1 {
    NorthStarBudgetV1 {
        total_memory_bytes: 16 * 1024,
        hot_memory_bytes: 4 * 1024,
        max_support_rows: 256,
        max_future_rows: 256,
        max_exact_checks: 10_000,
        max_cpu_nanos: 1_000_000,
    }
}

fn verified_phase_receipt(
    surface: u8,
    plane: u8,
    source_role: u8,
    target_role: u8,
    phase_re_micro: i32,
    phase_im_micro: i32,
) -> VerifiedDeltaReceipt {
    let stages = TypedExecutionStage::ALL
        .into_iter()
        .map(|stage| TypedExecutionStageReceipt {
            schema: TYPED_EXECUTION_STAGE_RECEIPT_SCHEMA_V1.to_owned(),
            stage,
            generation: 7,
            operator_fingerprint64: 42,
            surface_id_sha256: root(&format!("surface-{surface}")),
            session_id_sha256: root(&format!("phase-session-{surface}")),
            input_relation_sha256: root(&format!("phase-input-{surface}")),
            predicted_relation_sha256: root(&format!("phase-output-{surface}")),
            observed_relation_sha256: root(&format!("phase-output-{surface}")),
            stage_payload_sha256: root(&format!("phase-stage-{surface}-{stage:?}")),
            independently_observed: stage == TypedExecutionStage::IndependentVerifier,
            accepted: true,
        })
        .collect();
    VerifiedDeltaReceipt::from_typed_trace(
        stages,
        VerifiedDeltaOutcome::Positive,
        vec![VerifiedDeltaRelation {
            plane,
            source_role,
            target_role,
            state: VerifiedDeltaRelationState::Supported,
            phase_re_micro,
            phase_im_micro,
        }],
    )
    .expect("verified phase receipt")
}

fn north_star_seed_receipt(
    contract: &NorthStarProofContractV1,
    seed: u64,
    passes: bool,
    wrong_accepts: u64,
) -> NorthStarProofSeedReceiptV1 {
    let budget_root = contract.budget.root_sha256().expect("budget root");
    let arms = contract
        .arms
        .iter()
        .copied()
        .map(|arm| {
            let primary_score_milli = if arm == NorthStarProofArmV1::CellularWaveEnsemble {
                1_000
            } else if passes {
                900
            } else {
                990
            };
            NorthStarArmMetricsV1 {
                arm,
                budget_root_sha256: budget_root.clone(),
                experiment_report_root_sha256: root(&format!("experiment-{seed}-{arm:?}")),
                future_rows_root_sha256: root(&format!("future-{seed}")),
                snapshot_root_sha256: root(&format!("snapshot-{seed}-{arm:?}")),
                primary_score_milli,
                future_rows: 10,
                correct_executions: 10,
                wrong_accepts: if arm == NorthStarProofArmV1::CellularWaveEnsemble {
                    wrong_accepts
                } else {
                    0
                },
                runtime_parity_failures: 0,
                verifier_coverage_milli: 1_000,
                exact_checks: 100,
                memory_bytes: 8 * 1024,
                cpu_nanos: 100_000,
                circuit_formed: arm == NorthStarProofArmV1::CellularWaveEnsemble,
            }
        })
        .collect();
    NorthStarProofSeedReceiptV1::seal(
        contract,
        seed,
        arms,
        NorthStarSeedConditionsV1 {
            delayed_transition_observed: true,
            exact_memory_cleanup_observed: true,
            key_ablation_drop_milli: if passes { 100 } else { 10 },
            non_key_ablation_drop_milli: 10,
            snapshot_restore_exact: true,
            snapshot_cold_start_gain_milli: if passes { 100 } else { 10 },
        },
    )
    .expect("seed receipt")
}

#[test]
fn north_star_proof_is_restart_exact_and_requires_four_of_five_safe_seeds() {
    let frozen = frozen_unique_law_fixture("north-star", "support-north-star", 1, 7);
    let contract =
        NorthStarProofContractV1::seal(1, &frozen, vec![11, 22, 33, 44, 55], north_star_budget())
            .expect("North Star contract");
    let contract_bytes = contract.canonical_bytes().expect("contract bytes");
    assert_eq!(
        NorthStarProofContractV1::from_canonical_bytes(&contract_bytes).expect("contract restore"),
        contract
    );

    let three_passes = contract
        .seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| north_star_seed_receipt(&contract, *seed, index < 3, 0))
        .collect();
    let report =
        evaluate_north_star_proof_v1(&contract, three_passes, 0, true, true).expect("report");
    assert_eq!(report.verdict, NorthStarProofVerdictV1::Fail);
    assert_eq!(report.blocker, "north_star_seed_threshold_not_met");

    let four_passes = contract
        .seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| north_star_seed_receipt(&contract, *seed, index < 4, 0))
        .collect();
    let report =
        evaluate_north_star_proof_v1(&contract, four_passes, 0, true, true).expect("report");
    assert_eq!(report.verdict, NorthStarProofVerdictV1::Pass);
    assert!(!report.authority_ready);
    assert!(!report.phase_mutation_allowed);
    let report_bytes = report.canonical_bytes(&contract).expect("report bytes");
    assert_eq!(
        NorthStarProofReportV1::from_canonical_bytes(&report_bytes, &contract)
            .expect("report restore"),
        report
    );
}

#[test]
fn north_star_cellular_support_uses_only_independently_verified_phase_receipts() {
    let frozen = frozen_unique_law_fixture("cellular-support", "cellular-support", 1, 7);
    let receipts = vec![
        verified_phase_receipt(1, 0, 0, 1, 1_000_000, 0),
        verified_phase_receipt(2, 1, 1, 2, 0, 1_000_000),
        verified_phase_receipt(3, 2, 0, 2, -1_000_000, 0),
    ];
    let support = synthesize_north_star_cellular_support_v1(
        &frozen,
        7,
        42,
        &receipts,
        CircuitSynthesisConfig::default(),
        OperatorGrokkingConfig::default(),
    )
    .expect("cellular support");

    assert_eq!(support.synthesis.emitted_circuits, 1);
    assert_eq!(support.frozen_circuits.circuits().len(), 1);
    assert!(support.report.validate());
    assert!(!support.report.authority_ready);
    assert!(!support.report.phase_mutation_allowed);
    let bytes = support.report.canonical_bytes().expect("support bytes");
    assert_eq!(
        NorthStarCellularSupportReportV1::from_canonical_bytes(&bytes).expect("support restart"),
        support.report
    );

    let duplicate = vec![
        receipts[0].clone(),
        receipts[0].clone(),
        receipts[2].clone(),
    ];
    assert_eq!(
        synthesize_north_star_cellular_support_v1(
            &frozen,
            7,
            42,
            &duplicate,
            CircuitSynthesisConfig::default(),
            OperatorGrokkingConfig::default(),
        ),
        Err(NorthStarCellularSupportErrorV1::InsufficientIndependentEvidence)
    );
}

#[test]
fn north_star_proof_rejects_seed_arm_budget_and_root_substitution() {
    let frozen = frozen_unique_law_fixture("north-star-veto", "support-veto", 1, 7);
    assert_eq!(
        NorthStarProofContractV1::seal(1, &frozen, vec![11, 11, 33, 44, 55], north_star_budget()),
        Err(NorthStarProofErrorV1::InvalidContract)
    );
    let contract =
        NorthStarProofContractV1::seal(1, &frozen, vec![11, 22, 33, 44, 55], north_star_budget())
            .expect("North Star contract");
    let mut duplicated = north_star_seed_receipt(&contract, 11, true, 0);
    duplicated.arms[1].arm = duplicated.arms[0].arm;
    assert!(!duplicated.validate(&contract));

    let mut unequal_budget = north_star_seed_receipt(&contract, 11, true, 0);
    unequal_budget.arms[1].budget_root_sha256 = root("foreign-budget");
    assert!(!unequal_budget.validate(&contract));

    let receipt = north_star_seed_receipt(&contract, 11, true, 0);
    assert_eq!(
        evaluate_north_star_proof_v1(&contract, vec![receipt.clone(), receipt], 0, true, true),
        Err(NorthStarProofErrorV1::InvalidSeedReceipt)
    );

    let unsafe_receipts = contract
        .seeds
        .iter()
        .map(|seed| north_star_seed_receipt(&contract, *seed, true, u64::from(*seed == 11)))
        .collect();
    let unsafe_report =
        evaluate_north_star_proof_v1(&contract, unsafe_receipts, 0, true, true).expect("report");
    assert_eq!(unsafe_report.verdict, NorthStarProofVerdictV1::Fail);
    assert_eq!(unsafe_report.blocker, "north_star_safety_failure");
    let partial_unsafe = evaluate_north_star_proof_v1(
        &contract,
        vec![north_star_seed_receipt(&contract, 11, true, 1)],
        0,
        true,
        true,
    )
    .expect("partial unsafe report");
    assert_eq!(partial_unsafe.verdict, NorthStarProofVerdictV1::Fail);
    assert_eq!(partial_unsafe.blocker, "north_star_safety_failure");

    let mut tampered_contract = contract.clone();
    tampered_contract.support_rows_root_sha256 = root("foreign-support-root");
    assert!(!tampered_contract.validate());
    let mut tampered_report = unsafe_report;
    tampered_report.report_root_sha256 = root("foreign-report-root");
    assert!(!tampered_report.validate(&contract));
}

#[test]
fn generation_registry_blocks_reuse_and_restarts_byte_identically() {
    let frozen = frozen_unique_law_fixture("generation-one", "support-one", 1, 7);
    let contradiction =
        independent_future_fixture(&frozen, "generation-one", "future-one", 8, false);
    assert_eq!(
        contradiction.receipt.verdict,
        Ms3IndependentFutureVerdictV1::Contradiction
    );
    let mut registry = Ms3GenerationRegistryV1::new();
    assert_eq!(registry.append_generation(&frozen), Ok(1));
    assert_eq!(
        registry.append_generation(&frozen),
        Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists)
    );
    registry
        .seal_terminal(&frozen, &contradiction)
        .expect("terminal contradiction");
    assert_eq!(
        registry.append_generation(&frozen),
        Err(Ms3GenerationRegistryErrorV1::EvidenceReuse)
    );

    let reused_future = frozen_unique_law_fixture("generation-one", "future-one", 9, 15);
    assert_eq!(
        registry.append_generation(&reused_future),
        Err(Ms3GenerationRegistryErrorV1::EvidenceReuse)
    );
    let generation_two = frozen_unique_law_fixture("generation-two", "support-two", 9, 15);
    assert_eq!(registry.append_generation(&generation_two), Ok(2));
    let bytes = registry.canonical_bytes().expect("registry bytes");
    assert_eq!(
        Ms3GenerationRegistryV1::from_canonical_bytes(&bytes).expect("registry restore"),
        registry
    );
}

#[test]
fn generation_registry_rejects_successor_after_future_pass() {
    let frozen = frozen_unique_law_fixture("generation-pass", "support-pass", 1, 7);
    let future = independent_future_fixture(&frozen, "generation-pass", "future-pass", 8, true);
    let generation_two = frozen_unique_law_fixture("generation-after-pass", "support-next", 9, 15);
    let mut registry = Ms3GenerationRegistryV1::new();
    registry.append_generation(&frozen).expect("generation one");
    registry
        .seal_terminal(&frozen, &future)
        .expect("terminal pass");
    assert_eq!(
        registry.append_generation(&generation_two),
        Err(Ms3GenerationRegistryErrorV1::SuccessorAfterPass)
    );
}

#[test]
fn generation_registry_seals_and_restores_linked_evidence_reuse() {
    let frozen = frozen_unique_law_fixture("reuse-generation", "reused-support-lineage", 1, 7);
    let future = independent_future_fixture(
        &frozen,
        "reuse-generation",
        "independent-contradiction-lineage",
        8,
        false,
    );
    let topology = t1_topology_row(
        "reused-linked",
        "request-reused-linked",
        "reused-support-lineage",
        9,
        3_000,
    );
    let frame = t1_completed_frame(
        "reused-linked",
        "action-reused-linked",
        "reused-support-lineage",
        3_500,
    );
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(256, 60),
        4,
        vec![topology],
        vec![frame],
        vec![terminal("request-reused-linked", 2_990, 3_100)],
    );
    let mut registry = Ms3GenerationRegistryV1::new();
    registry.append_generation(&frozen).expect("generation one");
    registry
        .seal_terminal(&frozen, &future)
        .expect("generation one contradiction");
    assert!(
        report
            .receipts
            .iter()
            .all(|receipt| registry.linked_evidence_was_used(receipt))
    );

    let closure = registry
        .seal_linked_evidence_reuse(2, &report, 9)
        .expect("evidence reuse closure");
    assert_eq!(closure.blocker, MS3_LINKED_EVIDENCE_REUSE);
    assert!(!closure.authority_ready);
    assert!(!closure.phase_mutation_allowed);
    assert_eq!(registry.next_generation_sequence(), 3);
    let bytes = registry.canonical_bytes().expect("registry bytes");
    assert_eq!(
        Ms3GenerationRegistryV1::from_canonical_bytes(&bytes).expect("registry restore"),
        registry
    );
}

#[test]
fn linked_frame_acquisition_collects_then_fails_at_the_sealed_row_budget() {
    let collecting = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(2, 60),
        2,
        vec![t1_topology_row(
            "pending",
            "request-pending",
            "session",
            1,
            1_000,
        )],
        Vec::new(),
        vec![terminal("request-pending", 990, 1_100)],
    );
    assert!(collecting.validate(), "{collecting:#?}");
    assert_eq!(
        collecting.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::Collecting
    );

    let in_flight = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(2, 60),
        2,
        vec![
            t1_topology_row("a", "request-a", "session-a", 1, 1_000),
            t1_topology_row("b", "request-b", "session-b", 2, 2_000),
        ],
        Vec::new(),
        vec![terminal("request-a", 990, 1_100)],
    );
    assert!(in_flight.validate(), "{in_flight:#?}");
    assert_eq!(
        in_flight.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::Collecting
    );

    let failed = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(2, 60),
        2,
        vec![
            t1_topology_row("a", "request-a", "session-a", 1, 1_000),
            t1_topology_row("b", "request-b", "session-b", 2, 2_000),
        ],
        Vec::new(),
        vec![
            terminal("request-a", 990, 1_100),
            terminal("request-b", 1_990, 2_100),
        ],
    );
    assert!(failed.validate(), "{failed:#?}");
    assert_eq!(
        failed.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
    );
    assert_eq!(failed.blocker, MS3_LINKED_FRAME_ACQUISITION_FAIL);
    assert!(!failed.phase_update_allowed);
    assert!(!failed.authority_ready);
}

#[test]
fn linked_frame_acquisition_counts_only_eligible_rows_toward_the_budget() {
    let eligible_a = t1_topology_row("eligible-a", "request-eligible-a", "session-a", 2, 2_000);
    let eligible_b = t1_topology_row("eligible-b", "request-eligible-b", "session-b", 4, 4_000);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract_v2(2, 4),
        10,
        vec![
            unattributed_topology_row(
                "unattributed-a",
                "request-unattributed-a",
                "session-u-a",
                1,
                1_000,
            ),
            eligible_a,
            unattributed_topology_row(
                "unattributed-b",
                "request-unattributed-b",
                "session-u-b",
                3,
                3_000,
            ),
            eligible_b,
        ],
        Vec::new(),
        vec![
            terminal("request-eligible-a", 1_990, 2_100),
            terminal("request-eligible-b", 3_990, 4_100),
        ],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(report.raw_scanned_topology_rows, 4);
    assert_eq!(report.eligible_topology_rows, 2);
    assert_eq!(report.evaluated_topology_rows, 2);
    assert_eq!(report.censored_unattributed_rows, 2);
    assert_eq!(report.consumed_topology_cursor_rows, 1_836);
    assert_eq!(report.consumed_capture_sequence, 4);
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
    );
    let mut registry = Ms3GenerationRegistryV1::new();
    let receipt = registry
        .seal_linked_acquisition_failure(1, &report, report.consumed_capture_sequence)
        .expect("durable cursor-bound acquisition failure");
    assert_eq!(
        receipt.schema,
        MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2
    );
    assert_eq!(
        receipt.consumed_topology_cursor_rows,
        report.consumed_topology_cursor_rows
    );
    assert!(receipt.validate());
    let bytes = registry.canonical_bytes().expect("registry bytes");
    let restored = Ms3GenerationRegistryV1::from_canonical_bytes(&bytes).expect("registry restore");
    assert_eq!(restored, registry);
}

#[test]
fn stale_missing_terminal_is_censored_without_becoming_negative_evidence() {
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract_v2(2, 4),
        10,
        vec![
            t1_topology_row("stalled", "request-stalled", "session-stalled", 1, 1_000),
            t1_topology_row("eligible-a", "request-eligible-a", "session-a", 2, 2_000),
            t1_topology_row("eligible-b", "request-eligible-b", "session-b", 3, 3_000),
        ],
        Vec::new(),
        vec![
            terminal("request-eligible-a", 1_990, 2_100),
            terminal("request-eligible-b", 2_990, 3_100),
        ],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(report.raw_scanned_topology_rows, 3);
    assert_eq!(report.eligible_topology_rows, 2);
    assert_eq!(report.terminal_receipt_rows, 2);
    assert_eq!(report.censored_topology_rows, 1);
    assert_eq!(
        report
            .ineligible_reason_counts
            .get(&MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable),
        Some(&1)
    );
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
    );
    assert_eq!(report.blocker, MS3_LINKED_FRAME_ACQUISITION_FAIL);
    assert!(report.receipts.is_empty());
    assert!(!report.phase_update_allowed);
    assert!(!report.authority_ready);
}

#[test]
fn unattributed_raw_budget_closes_as_censor_and_restarts_byte_identically() {
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(4, 60),
        400,
        vec![
            t1_topology_row("eligible-a", "request-eligible-a", "session-a", 1, 1_000),
            t1_topology_row("eligible-b", "request-eligible-b", "session-b", 2, 2_000),
            unattributed_topology_row(
                "unattributed-a",
                "request-unattributed-a",
                "session-u-a",
                3,
                3_000,
            ),
            unattributed_topology_row(
                "unattributed-b",
                "request-unattributed-b",
                "session-u-b",
                4,
                4_000,
            ),
        ],
        Vec::new(),
        vec![
            terminal("request-eligible-a", 990, 1_100),
            terminal("request-eligible-b", 1_990, 2_100),
        ],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe
    );
    assert_eq!(report.blocker, MS3_CENSORED_UNATTRIBUTED_PROBE);
    assert_eq!(report.raw_scanned_topology_rows, 4);
    assert_eq!(report.eligible_topology_rows, 2);
    assert_eq!(report.terminal_receipt_rows, 2);
    assert!(!report.phase_update_allowed);
    assert!(!report.authority_ready);

    let mut registry = Ms3GenerationRegistryV1::new();
    let receipt = registry
        .seal_unattributed_probe_censor(1, &report, report.consumed_capture_sequence)
        .expect("durable unattributed censor");
    assert_eq!(receipt.blocker, MS3_CENSORED_UNATTRIBUTED_PROBE);
    assert_eq!(
        receipt.consumed_topology_cursor_rows,
        report.consumed_topology_cursor_rows
    );
    assert!(!receipt.phase_mutation_allowed);
    assert!(!receipt.authority_ready);
    let bytes = registry.canonical_bytes().expect("registry bytes");
    let restored = Ms3GenerationRegistryV1::from_canonical_bytes(&bytes).expect("registry restore");
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
    assert_eq!(restored, registry);
}

#[test]
fn mixed_unattributed_and_topology_censors_are_operational_not_scientific_failure() {
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract_v2(4, 4),
        10,
        vec![
            t1_topology_row("eligible-a", "request-eligible-a", "session-a", 1, 1_000),
            t1_topology_row("eligible-b", "request-eligible-b", "session-b", 2, 2_000),
            unattributed_topology_row(
                "unattributed",
                "request-unattributed",
                "session-u",
                3,
                3_000,
            ),
            censored_topology_row("censored", "request-censored", "session-c", 4, 4_000),
        ],
        Vec::new(),
        vec![
            terminal("request-eligible-a", 990, 1_100),
            terminal("request-eligible-b", 1_990, 2_100),
        ],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe
    );
    assert_eq!(report.blocker, MS3_CENSORED_INELIGIBLE_PROBE);
    assert_eq!(report.censored_unattributed_rows, 1);
    assert_eq!(report.censored_topology_rows, 1);
    assert!(!report.phase_update_allowed);
    assert!(!report.authority_ready);

    let mut registry = Ms3GenerationRegistryV1::new();
    let receipt = registry
        .seal_ineligible_probe_censor(1, &report, report.consumed_capture_sequence)
        .expect("durable mixed censor");
    assert_eq!(receipt.blocker, MS3_CENSORED_INELIGIBLE_PROBE);
    assert_eq!(receipt.censored_unattributed_rows, 1);
    assert_eq!(receipt.censored_topology_rows, 1);
    assert!(receipt.validate());
}

#[test]
fn linked_frame_receipt_binds_topology_frame_terminal_and_identity() {
    let topology = t1_topology_row("linked", "request-linked", "session", 1, 1_000);
    let frame = t1_completed_frame("linked", "action-linked", "session", 1_500);
    let terminal = terminal("request-linked", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![topology.clone()],
        vec![frame],
        vec![terminal.clone()],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved
    );
    assert_eq!(report.linked_frame_rows, 1);
    let receipt = &report.receipts[0];
    assert_eq!(
        receipt.topology_commitment_root_sha256,
        topology.commit.commitment_root_sha256
    );
    assert_eq!(
        receipt.terminal_receipt_root_sha256,
        terminal.receipt_root_sha256
    );
    assert!(valid_nonzero_sha256(&receipt.completed_frame_root_sha256));
    assert!(valid_nonzero_sha256(&receipt.session_lineage_sha256));
    assert!(valid_nonzero_sha256(&receipt.request_event_id_sha256));
    assert!(valid_nonzero_sha256(&receipt.action_event_id_sha256));
    assert!(!receipt.phase_update_allowed);
    assert!(!receipt.authority_ready);
}

#[test]
fn reused_linked_evidence_stays_in_denominator_without_ending_acquisition() {
    let topology = t1_topology_row("reused", "request-reused", "reused-lineage", 1, 1_000);
    let frame = t1_completed_frame("reused", "action-reused", "reused-lineage", 1_500);
    let terminal = terminal("request-reused", 990, 1_100);
    let used_evidence_roots = BTreeSet::from([root("reused-lineage")]);
    let report = build_ms3_linked_frame_acquisition_report_excluding_used_evidence_v1(
        acquisition_contract(8, 60),
        2,
        vec![topology],
        vec![frame],
        vec![terminal],
        &used_evidence_roots,
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(report.evaluated_topology_rows, 1);
    assert_eq!(report.terminal_receipt_rows, 1);
    assert_eq!(report.relevant_verified_frame_rows, 1);
    assert_eq!(report.linked_frame_rows, 0);
    assert!(report.receipts.is_empty());
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::Collecting
    );
    assert_eq!(report.blocker, "linked_frame_pending");
    assert!(!report.phase_update_allowed);
    assert!(!report.authority_ready);
}

#[test]
fn linked_frame_acquisition_excludes_outcomes_after_the_frozen_deadline() {
    let topology = t1_topology_row("late", "request-late", "session", 1, 1_000);
    let mut frame = t1_completed_frame("late", "action-late", "session", 1_500);
    frame.observed_at_unix_nanos = 62_000_000_000;
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        61,
        vec![topology],
        vec![frame],
        vec![terminal("request-late", 990, 1_100)],
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.verdict,
        Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
    );
    assert_eq!(report.linked_frame_rows, 0);
}

#[test]
fn frozen_version_space_excludes_the_pre_freeze_buffer_with_two_watermarks() {
    let topology = t1_topology_row("freeze", "request-freeze", "session", 1, 1_000);
    let frame = t1_completed_frame("freeze", "action-freeze", "session", 1_500);
    let terminal = terminal("request-freeze", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![topology.clone()],
        vec![frame.clone()],
        vec![terminal.clone()],
    );
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
        std::slice::from_ref(&terminal),
    );
    let bound = &ledger.bound_for_topology(&topology.commit.commitment_root_sha256)[0];

    let envelope = prepare_ms3_frozen_version_space_v1(&report, bound, &frame)
        .expect("prepared version space")
        .seal(
            7,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen version space");

    assert_eq!(envelope.contract.support_watermark, 1);
    assert_eq!(envelope.contract.contract_watermark, 7);
    assert_eq!(envelope.contract.future_min_sequence, 8);
    assert_eq!(envelope.contract.pre_freeze_buffer_sequence_span, 6);
    assert_eq!(
        envelope.contract.pre_freeze_buffer_disposition,
        MS3_PRE_FREEZE_BUFFER_EXCLUDED
    );
    assert!(matches!(
        envelope.contract.state,
        Ms3FrozenVersionSpaceStateV1::UniqueLawFrozen { .. }
    ));
    assert!(!envelope.contract.authority_ready);
    assert!(!envelope.contract.phase_mutation_allowed);
}

#[test]
fn frozen_version_space_restart_is_byte_identical_and_rejects_tampering() {
    let topology = t1_topology_row("restart", "request-restart", "session", 3, 1_000);
    let frame = t1_completed_frame("restart", "action-restart", "session", 1_500);
    let terminal = terminal("request-restart", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![topology.clone()],
        vec![frame.clone()],
        vec![terminal.clone()],
    );
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
        std::slice::from_ref(&terminal),
    );
    let bound = &ledger.bound_for_topology(&topology.commit.commitment_root_sha256)[0];
    let envelope = prepare_ms3_frozen_version_space_v1(&report, bound, &frame)
        .expect("prepared version space")
        .seal(
            9,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen version space");
    let bytes = envelope.canonical_bytes().expect("canonical envelope");
    let restored =
        FrozenVersionSpaceEnvelopeV1::from_canonical_bytes(&bytes).expect("restored envelope");
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);

    let mut tampered = bytes;
    let last = tampered.last_mut().expect("non-empty envelope");
    *last ^= 1;
    assert!(FrozenVersionSpaceEnvelopeV1::from_canonical_bytes(&tampered).is_err());
}

#[test]
fn unique_law_future_is_predicted_before_terminal_and_restarts_byte_identically() {
    let support_topology =
        t1_topology_row("support", "request-support", "support-lineage", 1, 1_000);
    let support_frame = t1_completed_frame("support", "action-support", "support-lineage", 1_500);
    let support_terminal = terminal("request-support", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![support_topology.clone()],
        vec![support_frame.clone()],
        vec![support_terminal.clone()],
    );
    let support_ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&support_topology),
        std::slice::from_ref(&support_frame),
        std::slice::from_ref(&support_terminal),
    );
    let support_bound =
        &support_ledger.bound_for_topology(&support_topology.commit.commitment_root_sha256)[0];
    let frozen = prepare_ms3_frozen_version_space_v1(&report, support_bound, &support_frame)
        .expect("prepared version space")
        .seal(
            7,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen version space");

    let future_topology = t1_topology_row("future", "request-future", "future-lineage", 8, 2_000);
    let prediction = predict_ms3_unique_law_v1(&frozen, &future_topology, 2_050_000_000)
        .expect("pre-action prediction")
        .expect("applicable future");
    let applicability_contract = Ms3FutureApplicabilityContractV1::seal(
        frozen.contract.contract_root_sha256.clone(),
        7,
        8,
        1,
    )
    .expect("applicability contract");
    let mut applicability =
        Ms3FutureApplicabilityLedgerV1::new(applicability_contract.clone()).expect("ledger");
    let committed = Ms3FutureApplicabilityEventV1::seal(
        &applicability_contract,
        8,
        future_topology.commit.commitment_root_sha256.clone(),
        future_topology
            .session_lineage_sha256
            .clone()
            .expect("lineage"),
        Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
        String::new(),
        Some(&prediction),
        Some(2_060_000_000),
        None,
        2_060_000_000,
    )
    .expect("durable prediction event");
    let committed_root = committed.event_root_sha256.clone();
    assert!(applicability.append(committed).expect("append"));
    assert_eq!(
        applicability.report(101).verdict,
        Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending
    );
    let future_frame = t1_completed_frame("future", "action-future", "future-lineage", 2_500);
    let future_terminal = terminal("request-future", 1_990, 2_100);
    let future_ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&future_topology),
        std::slice::from_ref(&future_frame),
        std::slice::from_ref(&future_terminal),
    );
    let future_bound =
        &future_ledger.bound_for_topology(&future_topology.commit.commitment_root_sha256)[0];
    let future = seal_ms3_independent_future_v1(
        &frozen,
        &prediction,
        &committed_root,
        2_060_000_000,
        future_bound,
        &future_frame,
    )
    .expect("independent future");

    assert_eq!(future.receipt.verdict, Ms3IndependentFutureVerdictV1::Pass);
    assert!(future.receipt.exact_transfer_parity);
    assert!(!future.receipt.runtime_actor_verifier_parity);
    assert!(!future.receipt.authority_ready);
    let bytes = future
        .canonical_bytes(&frozen)
        .expect("future canonical bytes");
    let restored = Ms3IndependentFutureEnvelopeV1::from_canonical_bytes(&bytes, &frozen)
        .expect("future restart");
    assert_eq!(
        restored
            .canonical_bytes(&frozen)
            .expect("restored future bytes"),
        bytes
    );

    let missing = Ms3FutureApplicabilityEventV1::seal(
        &applicability_contract,
        8,
        future_topology.commit.commitment_root_sha256.clone(),
        future_topology
            .session_lineage_sha256
            .clone()
            .expect("lineage"),
        Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing,
        "PRECOMMITTED_PREDICTION_MISSING".to_owned(),
        Some(&prediction),
        Some(2_060_000_000),
        Some((
            &future_terminal.receipt_root_sha256,
            2_050_000_000,
            future_bound.binding.action_observed_at_unix_nanos,
        )),
        2_070_000_000,
    )
    .expect("precommitted exclusion");
    assert!(applicability.append(missing).expect("append exclusion"));
    let report = applicability.report(101);
    assert_eq!(report.independent_topologies, 1);
    assert_eq!(report.precommitted_prediction_missing, 1);
    assert_eq!(report.active_predictions, 0);
    assert_eq!(report.verdict, Ms3FutureApplicabilityVerdictV1::Collecting);
    let gate_bytes = applicability.canonical_bytes().expect("gate bytes");
    assert_eq!(
        Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&gate_bytes)
            .expect("gate restart")
            .canonical_bytes()
            .expect("restored gate bytes"),
        gate_bytes
    );
}

#[test]
fn independent_future_rejects_prediction_durable_after_action_observation() {
    let support_topology = t1_topology_row(
        "support-delay",
        "request-support-delay",
        "support-lineage",
        1,
        1_000,
    );
    let support_frame = t1_completed_frame(
        "support-delay",
        "action-support-delay",
        "support-lineage",
        1_500,
    );
    let support_terminal = terminal("request-support-delay", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![support_topology.clone()],
        vec![support_frame.clone()],
        vec![support_terminal.clone()],
    );
    let support_ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&support_topology),
        std::slice::from_ref(&support_frame),
        std::slice::from_ref(&support_terminal),
    );
    let support_bound =
        &support_ledger.bound_for_topology(&support_topology.commit.commitment_root_sha256)[0];
    let frozen = prepare_ms3_frozen_version_space_v1(&report, support_bound, &support_frame)
        .expect("prepared version space")
        .seal(
            7,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen version space");
    let future_topology = t1_topology_row(
        "future-delay",
        "request-future-delay",
        "future-lineage",
        8,
        2_000,
    );
    let prediction = predict_ms3_unique_law_v1(&frozen, &future_topology, 2_050_000_000)
        .expect("prediction")
        .expect("applicable");
    let future_frame = t1_completed_frame(
        "future-delay",
        "action-future-delay",
        "future-lineage",
        2_500,
    );
    let future_terminal = terminal("request-future-delay", 1_990, 2_700);
    let future_ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&future_topology),
        std::slice::from_ref(&future_frame),
        std::slice::from_ref(&future_terminal),
    );
    let future_bound =
        &future_ledger.bound_for_topology(&future_topology.commit.commitment_root_sha256)[0];

    assert_eq!(
        seal_ms3_independent_future_v1(
            &frozen,
            &prediction,
            &root("durable-applicability-event"),
            2_600_000_000,
            future_bound,
            &future_frame,
        ),
        Err("future_prediction_binding_mismatch")
    );
}

#[test]
fn missing_completed_frame_is_censored_after_a_durable_same_lineage_fence() {
    let contract =
        Ms3FutureApplicabilityContractV1::seal(root("censor-law"), 7, 8, 100).expect("contract");
    let prediction = Ms3FuturePredictionV1 {
        schema: MS3_FUTURE_PREDICTION_SCHEMA_V1.to_owned(),
        prediction_root_sha256: root("censor-prediction"),
        contract_root_sha256: root("censor-contract"),
        candidate_freeze_root_sha256: root("censor-freeze"),
        canonical_program_root_sha256: root("censor-program"),
        capture_sequence: 8,
        topology_root_sha256: root("censor-topology"),
        request_event_id_sha256: root("censor-request"),
        turn_intent_id_sha256: root("censor-intent"),
        session_lineage_sha256: root("censor-lineage"),
        pre_action_binding_root_sha256: root("censor-binding"),
        predicted_at_unix_nanos: 100_000_000_001,
        authority_ready: false,
        phase_mutation_allowed: false,
    };
    let mut ledger = Ms3FutureApplicabilityLedgerV1::new(contract.clone()).expect("ledger");
    let committed = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        prediction.capture_sequence,
        prediction.topology_root_sha256.clone(),
        prediction.session_lineage_sha256.clone(),
        Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
        String::new(),
        Some(&prediction),
        Some(101_000_000_000),
        None,
        101_000_000_000,
    )
    .expect("committed");
    assert!(ledger.append(committed).expect("append committed"));

    let censored = Ms3FutureApplicabilityEventV1::seal_censored_missing_completed_frame(
        &contract,
        &prediction,
        101_000_000_000,
        &root("censor-terminal"),
        100_500_000_000,
        Ms3CompletedFrameCaptureFenceV1 {
            topology_root_sha256: root("censor-fence-topology"),
            request_event_id_sha256: root("censor-fence-request"),
            session_lineage_sha256: prediction.session_lineage_sha256.clone(),
            capture_sequence: 9,
            captured_at_unix_nanos: 102_500_000_000,
        },
        103_000_000_000,
    )
    .expect("censored");
    assert!(!censored.authority_ready);
    assert!(!censored.phase_mutation_allowed);
    assert!(ledger.append(censored).expect("append censored"));

    let report = ledger.report(103);
    assert_eq!(report.independent_topologies, 1);
    assert_eq!(report.censored_missing_completed_frame, 1);
    assert_eq!(report.active_predictions, 0);
    assert_eq!(report.verdict, Ms3FutureApplicabilityVerdictV1::Collecting);

    let bytes = ledger.canonical_bytes().expect("canonical bytes");
    let restored =
        Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&bytes).expect("restart restore");
    assert_eq!(restored.canonical_bytes().expect("restored bytes"), bytes);
}

#[test]
fn future_applicability_gate_fails_only_after_its_frozen_deadline() {
    let contract =
        Ms3FutureApplicabilityContractV1::seal(root("law"), 7, 8, 100).expect("contract");
    let mut ledger = Ms3FutureApplicabilityLedgerV1::new(contract.clone()).expect("ledger");
    let event = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        8,
        root("topology"),
        root("independent-lineage"),
        Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
        "structural_role_missing_or_ambiguous".to_owned(),
        None,
        None,
        None,
        101_000_000_000,
    )
    .expect("event");
    assert!(ledger.append(event).expect("append"));
    let collecting = ledger.report(contract.deadline_unix - 1);
    assert_eq!(
        collecting.verdict,
        Ms3FutureApplicabilityVerdictV1::Collecting
    );
    assert_eq!(collecting.independent_topologies, 1);
    assert_eq!(collecting.structurally_not_applicable, 1);
    let failed = ledger.report(contract.deadline_unix);
    assert_eq!(
        failed.verdict,
        Ms3FutureApplicabilityVerdictV1::AcquisitionFail
    );
    assert_eq!(failed.blocker, MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL);
    assert!(!failed.authority_ready);
    assert!(!failed.phase_mutation_allowed);
}

#[test]
fn future_applicability_deadline_expires_active_prediction() {
    let support_topology = t1_topology_row(
        "support-active",
        "request-support-active",
        "support-lineage",
        1,
        1_000,
    );
    let support_frame = t1_completed_frame(
        "support-active",
        "action-support-active",
        "support-lineage",
        1_500,
    );
    let support_terminal = terminal("request-support-active", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![support_topology.clone()],
        vec![support_frame.clone()],
        vec![support_terminal.clone()],
    );
    let support_ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&support_topology),
        std::slice::from_ref(&support_frame),
        std::slice::from_ref(&support_terminal),
    );
    let support_bound =
        &support_ledger.bound_for_topology(&support_topology.commit.commitment_root_sha256)[0];
    let frozen = prepare_ms3_frozen_version_space_v1(&report, support_bound, &support_frame)
        .expect("prepared")
        .seal(
            7,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("frozen");
    let topology = t1_topology_row(
        "future-active",
        "request-future-active",
        "future-lineage",
        8,
        2_000,
    );
    let prediction = predict_ms3_unique_law_v1(&frozen, &topology, 2_050_000_000)
        .expect("prediction")
        .expect("applicable");
    let contract =
        Ms3FutureApplicabilityContractV1::seal(root("law-active"), 7, 8, 100).expect("contract");
    let mut ledger = Ms3FutureApplicabilityLedgerV1::new(contract.clone()).expect("ledger");
    let event = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        8,
        topology.commit.commitment_root_sha256,
        topology.session_lineage_sha256.expect("lineage"),
        Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
        String::new(),
        Some(&prediction),
        Some(101_000_000_000),
        None,
        101_000_000_000,
    )
    .expect("event");
    assert!(ledger.append(event).expect("append"));
    assert_eq!(
        ledger.report(contract.deadline_unix - 1).verdict,
        Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending
    );
    let expired = ledger.report(contract.deadline_unix);
    assert!(expired.validate());
    assert_eq!(
        expired.verdict,
        Ms3FutureApplicabilityVerdictV1::AcquisitionFail
    );
}

#[test]
fn applicability_restore_rejects_duplicate_classification_and_orphan_disqualification() {
    fn encode_ledger(
        contract: Ms3FutureApplicabilityContractV1,
        events: Vec<Ms3FutureApplicabilityEventV1>,
    ) -> Vec<u8> {
        let ledger_root_sha256 = nando_operator_kernel::canonical_json_sha256(&(
            MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1,
            contract.contract_root_sha256.as_str(),
            events
                .iter()
                .map(|event| event.event_root_sha256.as_str())
                .collect::<Vec<_>>(),
            false,
            false,
        ))
        .expect("ledger root");
        serde_cbor::to_vec(&Ms3FutureApplicabilityLedgerV1 {
            schema: MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1.to_owned(),
            ledger_root_sha256,
            contract,
            events,
            authority_ready: false,
            phase_mutation_allowed: false,
        })
        .expect("ledger bytes")
    }

    let contract =
        Ms3FutureApplicabilityContractV1::seal(root("restore-law"), 7, 8, 100).expect("contract");
    let first = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        8,
        root("same-topology"),
        root("lineage-a"),
        Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
        "missing-role".to_owned(),
        None,
        None,
        None,
        101_000_000_000,
    )
    .expect("first");
    let second = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        9,
        root("same-topology"),
        root("lineage-b"),
        Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
        "missing-role".to_owned(),
        None,
        None,
        None,
        102_000_000_000,
    )
    .expect("second");
    assert!(
        Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&encode_ledger(
            contract.clone(),
            vec![first, second],
        ))
        .is_err()
    );

    let orphan = Ms3FutureApplicabilityEventV1::seal(
        &contract,
        8,
        root("orphan-topology"),
        root("orphan-lineage"),
        Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing,
        "PRECOMMITTED_PREDICTION_MISSING".to_owned(),
        Some(&Ms3FuturePredictionV1 {
            schema: MS3_FUTURE_PREDICTION_SCHEMA_V1.to_owned(),
            prediction_root_sha256: root("orphan-prediction"),
            contract_root_sha256: root("orphan-contract"),
            candidate_freeze_root_sha256: root("orphan-freeze"),
            canonical_program_root_sha256: root("orphan-program"),
            capture_sequence: 8,
            topology_root_sha256: root("orphan-topology"),
            request_event_id_sha256: root("orphan-request"),
            turn_intent_id_sha256: root("orphan-intent"),
            session_lineage_sha256: root("orphan-lineage"),
            pre_action_binding_root_sha256: root("orphan-binding"),
            predicted_at_unix_nanos: 100_000_000_001,
            authority_ready: false,
            phase_mutation_allowed: false,
        }),
        Some(101_000_000_000),
        Some((&root("terminal"), 100_000_000_000, 102_000_000_000)),
        102_000_000_000,
    )
    .expect("orphan event");
    assert!(
        Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&encode_ledger(
            contract,
            vec![orphan],
        ))
        .is_err()
    );
}

#[test]
fn future_applicability_gate_fails_at_its_frozen_topology_budget() {
    let contract =
        Ms3FutureApplicabilityContractV1::seal(root("law"), 7, 8, 100).expect("contract");
    let mut ledger = Ms3FutureApplicabilityLedgerV1::new(contract.clone()).expect("ledger");
    for index in 0..contract.max_independent_topologies {
        let event = Ms3FutureApplicabilityEventV1::seal(
            &contract,
            8 + index,
            root(&format!("topology-{index}")),
            root(&format!("independent-lineage-{index}")),
            Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
            "structural_role_missing_or_ambiguous".to_owned(),
            None,
            None,
            None,
            101_000_000_000 + index,
        )
        .expect("event");
        assert!(ledger.append(event).expect("append"));
    }

    let report = ledger.report(contract.opened_at_unix + 1);
    assert!(report.validate());
    assert_eq!(
        report.independent_topologies,
        contract.max_independent_topologies
    );
    assert_eq!(
        report.verdict,
        Ms3FutureApplicabilityVerdictV1::AcquisitionFail
    );
    assert_eq!(report.blocker, MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL);

    let mut tampered = report;
    tampered.independent_topologies -= 1;
    assert!(!tampered.validate());
}

#[test]
fn ambiguous_linked_frame_freezes_predictions_without_opening_authority() {
    let topology = t1_competing_role_topology_row(
        "ambiguous-freeze",
        "request-ambiguous-freeze",
        "session",
        5,
        1_000,
        true,
    );
    let frame = t1_competing_role_projection_frame(
        "ambiguous-freeze",
        "action-ambiguous-freeze",
        "session",
        1_500,
        true,
    );
    let terminal = terminal("request-ambiguous-freeze", 990, 1_100);
    let report = build_ms3_linked_frame_acquisition_report_v1(
        acquisition_contract(8, 60),
        2,
        vec![topology.clone()],
        vec![frame.clone()],
        vec![terminal.clone()],
    );
    let ledger = TransportBindingLedgerV1::build(
        std::slice::from_ref(&topology),
        std::slice::from_ref(&frame),
        std::slice::from_ref(&terminal),
    );
    let bound = &ledger.bound_for_topology(&topology.commit.commitment_root_sha256)[0];
    let envelope = prepare_ms3_frozen_version_space_v1(&report, bound, &frame)
        .expect("prepared version space")
        .seal(
            8,
            Ms3VersionSpaceVersionsV1 {
                compiler_version: "test-compiler.v1".to_owned(),
                vm_abi: "test-vm.v1".to_owned(),
            },
        )
        .expect("ambiguous version space");

    assert!(matches!(
        envelope.contract.state,
        Ms3FrozenVersionSpaceStateV1::Ambiguous {
            semantic_classes: 2
        }
    ));
    assert_eq!(
        envelope.contract.future_collector_kind(),
        Some("distinguishing_observation")
    );
    assert!(envelope.contract.passive_probe.is_some());
    assert!(!envelope.contract.authority_ready);
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
                request_reference_ordinal_candidates: Vec::new(),
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
