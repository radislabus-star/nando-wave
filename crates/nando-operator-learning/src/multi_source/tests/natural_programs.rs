use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomSource, AtomValueType, CollectionProgramStep, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1, MultiSourceTypeClassV1,
    PreActionTopologyCommitV1, ProjectStatusMapping, RelationAtom, RelationFrame,
    ResponseOperation, ResponseProgram, ResponseValueSelector, ValueProjectionFormat,
};

use crate::{CaptureEvidenceReceipt, CaptureRecordCommitment, CaptureTransitionBinding};

use super::*;

fn completed_status_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    let mut frame = t1_completed_frame(intent, event, session, observed_at_unix_ms);
    frame.atoms.retain(|atom| {
        !matches!(
            atom,
            RelationAtom::TypedSlot {
                source: AtomSource::Action,
                ..
            } | RelationAtom::SlotEquality { .. }
                | RelationAtom::ActionFunction { .. }
                | RelationAtom::ActionRoleArgument { .. }
        )
    });
    frame.atoms.push(RelationAtom::ActionStatusProjection {
        mapping: ProjectStatusMapping::ZeroIsSuccess,
    });
    frame
}

fn collection_topology_row(
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
    row.structure.topology.roles[0].type_class = MultiSourceTypeClassV1::Array;
    row.structure.topology.roles[0].container_class = MultiSourceContainerClassV1::Sequence;
    row.structure.topology.roles[0].cardinality_class = MultiSourceCardinalityClassV1::Many;
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        capture_sequence,
    )
    .expect("collection topology commit");
    row
}

fn completed_collection_frame(
    intent: &str,
    event: &str,
    session: &str,
    observed_at_unix_ms: u64,
) -> RelationFrame {
    let mut frame =
        t1_completed_value_projection_frame(intent, event, session, observed_at_unix_ms);
    for atom in &mut frame.atoms {
        match atom {
            RelationAtom::TypedSlot { value_type, .. } => *value_type = AtomValueType::Collection,
            RelationAtom::ObservationSelector { selector, .. } => {
                *selector = ResponseValueSelector::JsonField {
                    field: "opaque_collection".to_owned(),
                    value_type: AtomValueType::Collection,
                };
            }
            _ => {}
        }
    }
    frame
}

#[test]
fn natural_status_branch_reaches_source_neutral_identification() {
    let topologies = vec![
        t1_topology_row("status-a", "request-a", "session-a", 1, 1_000),
        t1_topology_row("status-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        completed_status_frame("status-a", "action-a", "session-a", 1_500),
        completed_status_frame("status-b", "action-b", "session-b", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).completed_effect == CompletedEffectFormV1::StatusValueBranch
    }));
    let report = identify_multi_source_t1_operator_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        root("natural status epoch"),
    );
    assert!(report.validate(), "{report:#?}");
    assert!(matches!(
        report
            .canonical_program
            .as_ref()
            .map(|program| &program.operation),
        Some(ResponseOperation::ProjectStatus { .. })
    ));
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
}

#[test]
fn natural_collection_artifact_reaches_existing_identifier_without_scheduler_hint() {
    let topologies = vec![
        collection_topology_row("collection-a", "request-a", "session-a", 1, 1_000),
        collection_topology_row("collection-b", "request-b", "session-b", 2, 2_000),
    ];
    let frames = vec![
        completed_collection_frame("collection-a", "action-a", "session-a", 1_500),
        completed_collection_frame("collection-b", "action-b", "session-b", 2_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    assert!(ledger.rows().iter().all(|row| {
        factor_multi_source_row_v1(row).completed_effect
            == CompletedEffectFormV1::CollectionTransform
    }));
    let program = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    );
    assert!(pre_action_t1_binding_root(&program, &topologies[0].structure.topology).is_ok());
    let program_root = nando_operator_kernel::response_program_version_root_sha256(&program)
        .expect("program root");
    let artifacts = ledger
        .rows()
        .iter()
        .map(|row| {
            let receipt = CaptureEvidenceReceipt::new(vec![CaptureRecordCommitment {
                sequence: row.capture_sequence,
                record_sha256: root(&format!("capture-record:{}", row.join_root_sha256)),
            }])
            .expect("capture receipt");
            let binding = CaptureTransitionBinding::new(
                row.capture_sequence,
                &root(&format!("capture:{}", row.join_root_sha256)),
                &receipt,
            )
            .expect("capture binding");
            NaturalT1ProgramArtifactV1::seal(
                row.turn_intent_id_sha256.clone(),
                row.session_id_sha256.clone(),
                binding,
                BTreeMap::from([(program_root.clone(), program.clone())]),
                vec![program_root.clone()],
            )
            .expect("artifact")
        })
        .collect::<Vec<_>>();
    let report = identify_multi_source_t1_operator_with_candidate_artifacts_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &artifacts,
        root("natural collection epoch"),
    );
    assert!(report.validate(), "{report:#?}");
    assert!(matches!(
        report
            .canonical_program
            .as_ref()
            .map(|program| &program.operation),
        Some(ResponseOperation::ComposeCollection { .. })
    ));
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady
    );
}
