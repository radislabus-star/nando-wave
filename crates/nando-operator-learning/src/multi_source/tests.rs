use std::collections::BTreeSet;

use nando_operator_kernel::{
    AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1, MultiSourceExtractionStatusV1,
    MultiSourceRelationEdgeV1, MultiSourceRelationKindV1, MultiSourceRoleNodeV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
    PreActionTopologyCommitV1, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, sha256_bytes,
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
                value: "transport-a".to_owned(),
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
