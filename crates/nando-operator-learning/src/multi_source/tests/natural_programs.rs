use std::collections::{BTreeMap, BTreeSet};

use nando_operator_kernel::{
    AtomSource, AtomValueType, CollectionProgramStep, MultiSourceCardinalityClassV1,
    MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1, MultiSourceRelationEdgeV1,
    MultiSourceRelationKindV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionTopologyCommitV1,
    ProjectStatusMapping, RelationAtom, RelationFrame, ResponseOperation, ResponseProgram,
    ResponseValueSelector, ValueProjectionFormat,
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

fn with_ambient_scalar_role(
    mut row: PreActionTopologyAuditRowV1,
    value_type: MultiSourceTypeClassV1,
    witness_label: &str,
) -> PreActionTopologyAuditRowV1 {
    let collection_role_id = row.structure.topology.roles[0].local_role_id;
    let local_role_id = row
        .structure
        .topology
        .roles
        .iter()
        .map(|role| role.local_role_id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    row.structure.topology.roles.push(MultiSourceRoleNodeV1 {
        local_role_id,
        source_ordinal: local_role_id,
        value_ordinal: 0,
        type_class: value_type,
        container_class: MultiSourceContainerClassV1::Scalar,
        cardinality_class: MultiSourceCardinalityClassV1::One,
        temporal_class: MultiSourceTemporalClassV1::Latest,
        depth_bucket: 2,
        structural_flags: 1,
    });
    row.structure
        .topology
        .role_witnesses
        .push(MultiSourceRoleWitnessV1 {
            local_role_id,
            value_sha256: root(witness_label),
            request_reference_ordinal: None,
            request_reference_ordinal_candidates: Vec::new(),
        });
    row.structure
        .topology
        .relations
        .push(MultiSourceRelationEdgeV1 {
            relation: MultiSourceRelationKindV1::Contains,
            source_role_id: collection_role_id,
            target_role_id: local_role_id,
        });
    row.structure.topology.relations.sort();
    row.structure.topology.validate().expect("ambient topology");
    row.commit = PreActionTopologyCommitV1::seal(
        &row.structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        root("extractor"),
        root("config"),
        row.commit.capture_sequence,
    )
    .expect("ambient topology commit");
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
            let predicted = observed_typed_consequence_root_v1(
                frames
                    .iter()
                    .find(|frame| frame.session_id_sha256 == row.session_id_sha256)
                    .expect("joined frame"),
            )
            .expect("observed consequence");
            NaturalT1ProgramArtifactV1::seal_with_predictions(
                row.turn_intent_id_sha256.clone(),
                row.session_id_sha256.clone(),
                binding,
                BTreeMap::from([(program_root.clone(), program.clone())]),
                vec![program_root.clone()],
                BTreeMap::from([(program_root.clone(), predicted)]),
            )
            .expect("artifact")
        })
        .collect::<Vec<_>>();
    let hypothesis_only = artifacts
        .iter()
        .map(|artifact| {
            NaturalT1ProgramArtifactV1::seal(
                artifact.turn_intent_id_sha256.clone(),
                artifact.session_id_sha256.clone(),
                artifact.capture_binding.clone(),
                artifact.programs.clone(),
                artifact.hypothesis_program_roots_sha256.clone(),
            )
            .expect("hypothesis artifact")
        })
        .collect::<Vec<_>>();
    let hypothesis_report = identify_multi_source_t1_operator_with_candidate_artifacts_v1(
        &ledger.rows(),
        &frames,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &hypothesis_only,
        root("hypothesis-only epoch"),
    );
    assert_ne!(
        hypothesis_report.state,
        MultiSourceT1IdentificationStateV1::TransferReady,
        "artifact membership must not vote"
    );
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

#[test]
fn frozen_motif_identifier_transfers_across_different_ambient_topologies() {
    let topologies = vec![
        with_ambient_scalar_role(
            collection_topology_row("motif-a", "request-ma", "session-ma", 1, 1_000),
            MultiSourceTypeClassV1::String,
            "ambient-string",
        ),
        with_ambient_scalar_role(
            collection_topology_row("motif-b", "request-mb", "session-mb", 2, 2_000),
            MultiSourceTypeClassV1::Number,
            "ambient-number",
        ),
        with_ambient_scalar_role(
            collection_topology_row("motif-c", "request-mc", "session-mc", 3, 3_000),
            MultiSourceTypeClassV1::Boolean,
            "ambient-boolean",
        ),
    ];
    let frames = vec![
        completed_collection_frame("motif-a", "action-ma", "session-ma", 1_500),
        completed_collection_frame("motif-b", "action-mb", "session-mb", 2_500),
        completed_collection_frame("motif-c", "action-mc", "session-mc", 3_500),
    ];
    let ledger = MultiSourceJoinLedgerV1::build(&topologies, &frames);
    let rows = ledger.rows();
    assert_eq!(rows.len(), 3);
    let program = ResponseProgram::compose_collection(
        vec![
            CollectionProgramStep::SelectOnlyArrayField,
            CollectionProgramStep::Count,
        ],
        ValueProjectionFormat::PlainText,
        "completed",
    );
    let program_root = nando_operator_kernel::response_program_version_root_sha256(&program)
        .expect("program root");
    let artifacts = rows
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
            let frame = frames
                .iter()
                .find(|frame| frame.session_id_sha256 == row.session_id_sha256)
                .expect("joined frame");
            NaturalT1ProgramArtifactV1::seal_with_predictions(
                row.turn_intent_id_sha256.clone(),
                row.session_id_sha256.clone(),
                binding,
                BTreeMap::from([(program_root.clone(), program.clone())]),
                vec![program_root.clone()],
                BTreeMap::from([(
                    program_root.clone(),
                    observed_typed_consequence_root_v1(frame).expect("observed consequence"),
                )]),
            )
            .expect("artifact")
        })
        .collect::<Vec<_>>();
    let motifs =
        rows.iter()
            .map(|row| {
                let collection_role_id = row
                    .topology
                    .roles
                    .iter()
                    .find(|role| role.type_class == MultiSourceTypeClassV1::Array)
                    .expect("collection role")
                    .local_role_id;
                source_neutral_topology_motifs_v1(&row.topology)
                    .expect("motif enumeration")
                    .into_iter()
                    .find(|motif| {
                        motif.role_count == 1
                            && motif.embeddings.iter().any(|embedding| {
                                embedding.local_role_ids == vec![collection_role_id]
                            })
                    })
                    .expect("collection motif")
            })
            .collect::<Vec<_>>();
    assert!(
        motifs
            .iter()
            .all(|motif| motif.motif_root_sha256 == motifs[0].motif_root_sha256)
    );
    let contract = FrozenRawPhaseT1ContractV1 {
        frozen_domain_root_sha256: &root("frozen motif domain"),
        support_watermark: 2,
        candidate_generator_schema: MULTI_SOURCE_T1_CANDIDATE_GENERATOR_V4,
    };
    let report = identify_multi_source_t1_operator_with_frozen_motif_v1(
        &rows,
        &motifs,
        &frames,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &artifacts,
        contract,
        root("frozen motif epoch"),
    );

    assert!(report.validate(), "{report:#?}");
    assert_eq!(
        report.selected_shape_root_sha256.as_deref(),
        Some(motifs[0].motif_root_sha256.as_str())
    );
    assert!(report.raw_phase_selected_executable.is_some());
    assert!(report.exact_transfer_parity);
    assert_eq!(
        report.state,
        MultiSourceT1IdentificationStateV1::TransferReady,
        "{report:#?}"
    );
}
