use std::collections::BTreeSet;

use crate::{
    SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    opportunity::{OpportunityIntentAuditRowV1, ReducibilityClass},
};
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

#[test]
fn live_snapshot_is_order_independent_and_subtracts_active_overlap() {
    let topology_a = topology_row("turn-a", "request-a", "session-a", 1, 1_000);
    let topology_b = topology_row("turn-b", "request-b", "session-b", 2, 2_000);
    let frame_a = completed_frame("turn-a", "action-a", "session-a", 1_500);
    let frame_b = completed_frame("turn-b", "action-b", "session-b", 2_500);
    let opportunity_a = opportunity("turn-a", ReducibilityClass::CpuVerified);
    let opportunity_b = opportunity("turn-b", ReducibilityClass::UnexploredMultiSource);

    let forward = build_live_multi_source_discovery_snapshot_v2(
        vec![opportunity_a.clone(), opportunity_b.clone()],
        request_snapshot(vec![topology_a.clone(), topology_b.clone()]),
        vec![frame_a.clone(), frame_b.clone()],
    );
    let reversed = build_live_multi_source_discovery_snapshot_v2(
        vec![opportunity_b, opportunity_a],
        request_snapshot(vec![topology_b, topology_a]),
        vec![frame_b, frame_a],
    );

    assert!(forward.validate());
    assert_eq!(
        forward.blocker,
        LiveMultiSourceDiscoveryBlockerV1::NoEligibleT1Cohort
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
    let no_topology = build_live_multi_source_discovery_snapshot_v2(
        Vec::new(),
        request_snapshot(Vec::new()),
        Vec::new(),
    );
    assert!(no_topology.validate());
    assert_eq!(
        no_topology.blocker,
        LiveMultiSourceDiscoveryBlockerV1::NoPreActionTopology
    );

    let no_frame = build_live_multi_source_discovery_snapshot_v2(
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
    assert!(!report.runtime_actor_verifier_parity);
    assert!(!report.execution_authority);

    let snapshot = build_live_multi_source_discovery_snapshot_v2(
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
    assert!(!report.exact_transfer_parity);
}
