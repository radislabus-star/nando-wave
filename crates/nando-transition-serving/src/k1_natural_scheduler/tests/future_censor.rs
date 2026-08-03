use nando_operator_kernel::{
    AtomSource, AtomValueType, LEARNING_REQUEST_STRUCTURE_SCHEMA_V2, LearningRequestStructureV2,
    MultiSourceCardinalityClassV1, MultiSourceContainerClassV1, MultiSourceEvidenceOriginV1,
    MultiSourceExtractionStatusV1, MultiSourceRoleNodeV1, MultiSourceRoleWitnessV1,
    MultiSourceTemporalClassV1, MultiSourceTypeClassV1, PreActionMultiSourceTopologyV1,
    PreActionTopologyCommitV1, RELATION_FRAME_SCHEMA, RelationAtom, RelationFrame, ResponseProgram,
    sha256_bytes,
};
use nando_operator_learning::{
    SOURCE_NEUTRAL_EXTRACTOR_VERSION,
    multi_source::{
        K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1, K1FuturePredictionContractV1,
        K1FuturePredictionReceiptV1, K1IdentificationFreezeV1, PreActionTopologyAuditRowV1,
    },
};

use super::*;
use crate::k1_natural_scheduler::authority::append_and_persist;
use crate::k1_natural_scheduler::fork::ensure_epistemic_lane;
use crate::k1_natural_scheduler::future_authority::append_future_prediction_censor_authoritative;
use crate::k1_natural_scheduler::journal::restore_anchored_scheduler_for;
use crate::multi_source_frame_archive::MultiSourceFrameArchive;
use crate::multi_source_topology_archive::MultiSourceTopologyArchive;
use crate::terminal_receipt_archive::TerminalReceiptArchive;

struct CensorFixture {
    root_dir: PathBuf,
    config: CertificationAuthorityConfigV1,
    signing_key: SigningKey,
    prediction: K1FuturePredictionReceiptV1,
    request: K1FuturePredictionCensorAuthorityRequestV1,
}

fn topology(
    label: &str,
    lineage: &str,
    capture_sequence: u64,
    captured_at_unix_ms: u64,
) -> PreActionTopologyAuditRowV1 {
    let lineage_root = sha256_bytes(lineage.as_bytes());
    let provider_root = sha256_bytes(format!("provider-{label}").as_bytes());
    let structure = LearningRequestStructureV2 {
        schema: LEARNING_REQUEST_STRUCTURE_SCHEMA_V2.to_owned(),
        turn_intent_id_sha256: sha256_bytes(format!("intent-{label}").as_bytes()),
        request_event_id_sha256: sha256_bytes(label.as_bytes()),
        provider_bound_turn_identity: true,
        session_lineage_roots_sha256: vec![lineage_root.clone()],
        request_phase_atom_ids: vec![1],
        pre_action_context_atom_ids: vec![2],
        capability_atom_ids: vec![3],
        estimated_input_tokens: 10,
        provider_payload_bytes: 100,
        provider_capture_request_root_sha256: provider_root,
        decidability_reason_code: "pre_action_pending".to_owned(),
        topology: PreActionMultiSourceTopologyV1 {
            extraction_status: MultiSourceExtractionStatusV1::Complete,
            grounded_output_count: 1,
            output_part_count: 1,
            roles: vec![MultiSourceRoleNodeV1 {
                local_role_id: 1,
                source_ordinal: 0,
                value_ordinal: 0,
                type_class: MultiSourceTypeClassV1::Array,
                container_class: MultiSourceContainerClassV1::Sequence,
                cardinality_class: MultiSourceCardinalityClassV1::Many,
                temporal_class: MultiSourceTemporalClassV1::Latest,
                depth_bucket: 1,
                structural_flags: 0,
            }],
            role_witnesses: vec![MultiSourceRoleWitnessV1 {
                local_role_id: 1,
                value_sha256: sha256_bytes(format!("value-{label}").as_bytes()),
                request_reference_ordinal: None,
                request_reference_ordinal_candidates: Vec::new(),
            }],
            relations: Vec::new(),
        },
    };
    let commit = PreActionTopologyCommitV1::seal(
        &structure,
        MultiSourceEvidenceOriginV1::FreshLive,
        sha256_bytes(format!("bridge-{label}").as_bytes()),
        sha256_bytes(format!("capture-{label}").as_bytes()),
        capture_sequence,
    )
    .expect("topology commit");
    PreActionTopologyAuditRowV1 {
        bridge_epoch_sha256: sha256_bytes(b"bridge-epoch"),
        bridge_sequence: Some(capture_sequence),
        record_sha256: Some(sha256_bytes(format!("record-{label}").as_bytes())),
        capture_epoch_sha256: Some(sha256_bytes(b"capture-epoch")),
        capture_event_sha256: Some(sha256_bytes(format!("event-{label}").as_bytes())),
        capture_receipt_sha256: Some(sha256_bytes(format!("receipt-{label}").as_bytes())),
        captured_at_unix_ms: Some(captured_at_unix_ms),
        session_lineage_sha256: Some(lineage_root),
        physical_order_proven: true,
        structure,
        commit,
    }
}

fn install_generation(
    config: &CertificationAuthorityConfigV1,
    signing_key: &SigningKey,
    predicted_topology: &PreActionTopologyAuditRowV1,
) -> K1FuturePredictionReceiptV1 {
    let candidate = candidate_freeze();
    let semantic_class = root(1_500);
    let identification = K1IdentificationFreezeV1::seal(
        &candidate,
        root(1_501),
        "nando.operator-blind-version-space-generator.v1".to_owned(),
        vec![semantic_class.clone()],
        root(1_502),
        root(1_503),
        K1_DURABLE_FUTURE_PREDICTION_SCHEMA_V1.to_owned(),
    )
    .expect("identification");
    let contract = K1FuturePredictionContractV1::seal(
        candidate.freeze_root_sha256.clone(),
        identification.freeze_root_sha256.clone(),
        semantic_class,
        root(1_504),
        ResponseProgram::advance_plan("update_plan"),
        1_050_000_000,
    )
    .expect("contract");
    let prediction = K1FuturePredictionReceiptV1::seal(
        contract.contract_root_sha256.clone(),
        candidate.freeze_root_sha256.clone(),
        identification.freeze_root_sha256.clone(),
        contract.semantic_class_root_sha256.clone(),
        predicted_topology.commit.commitment_root_sha256.clone(),
        predicted_topology
            .commit
            .provider_capture_request_root_sha256
            .clone(),
        predicted_topology.structure.turn_intent_id_sha256.clone(),
        root(1_505),
        &contract.canonical_program_root_sha256,
        predicted_topology.commit.capture_sequence,
        predicted_topology
            .captured_at_unix_ms
            .expect("capture time"),
        1_100_000_000,
    )
    .expect("prediction");

    recover_authority(config, signing_key).expect("mechanism genesis");
    let mut mechanism =
        restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Mechanism).expect("mechanism");
    append_and_persist(
        config,
        K1SchedulerLaneV1::Mechanism,
        signing_key,
        &mut mechanism,
        K1SchedulerEventPayloadV1::CandidateFreeze(candidate.clone()),
    )
    .expect("mechanism candidate");
    append_and_persist(
        config,
        K1SchedulerLaneV1::Mechanism,
        signing_key,
        &mut mechanism,
        K1SchedulerEventPayloadV1::IdentificationFreeze(identification.clone()),
    )
    .expect("mechanism identification");
    ensure_epistemic_lane(config, signing_key).expect("epistemic fork");

    let mut epistemic =
        restore_anchored_scheduler_for(config, K1SchedulerLaneV1::Epistemic).expect("epistemic");
    for payload in [
        K1SchedulerEventPayloadV1::CandidateFreeze(candidate),
        K1SchedulerEventPayloadV1::IdentificationFreeze(identification),
        K1SchedulerEventPayloadV1::FuturePredictionContract(contract),
        K1SchedulerEventPayloadV1::FuturePrediction(prediction.clone()),
    ] {
        append_and_persist(
            config,
            K1SchedulerLaneV1::Epistemic,
            signing_key,
            &mut epistemic,
            payload,
        )
        .expect("epistemic event");
    }
    prediction
}

fn completed_frame(intent_root: String) -> RelationFrame {
    RelationFrame {
        schema: RELATION_FRAME_SCHEMA.to_owned(),
        frame_id_sha256: sha256_bytes(b"completed-frame"),
        event_id_sha256: sha256_bytes(b"completed-event"),
        client_intent_id_sha256: intent_root,
        session_id_sha256: sha256_bytes(b"completed-session"),
        observed_at_unix_nanos: 2_500_000_000,
        estimated_input_tokens: 10,
        extractor_version: SOURCE_NEUTRAL_EXTRACTOR_VERSION.to_owned(),
        verifier_label: Some(true),
        atoms: vec![RelationAtom::TypedSlot {
            slot_id: 0,
            value_type: AtomValueType::Integer,
            source: AtomSource::Outcome,
            value_sha256: sha256_bytes(b"completed-value"),
        }],
        evidence_ref_sha256: sha256_bytes(b"completed-evidence"),
    }
}

fn fixture(
    fence_lineage: &str,
    fence_sequence: u64,
    fence_captured_at_unix_ms: u64,
    with_completed_frame: bool,
) -> CensorFixture {
    let (root_dir, config, signing_key) = test_context();
    let predicted_topology = topology("predicted-request", "lineage", 10, 1_000);
    let fence_topology = topology(
        "fence-request",
        fence_lineage,
        fence_sequence,
        fence_captured_at_unix_ms,
    );
    let topology_path = config
        .root
        .parent()
        .expect("state parent")
        .join("pre-action-topology-archive-v1");
    let mut topology_archive =
        MultiSourceTopologyArchive::open(&topology_path).expect("topology archive");
    topology_archive
        .append(&predicted_topology)
        .expect("predicted topology");
    topology_archive
        .append(&fence_topology)
        .expect("fence topology");

    let prediction = install_generation(&config, &signing_key, &predicted_topology);
    let terminal_source = root_dir.join("terminal.jsonl");
    std::fs::write(
        &terminal_source,
        "{\"schema\":\"nando.nginx-terminal.v1\",\"request_id\":\"predicted-request\",\"status\":418,\"completed_at_unix_seconds\":\"2.000\",\"request_time_seconds\":\"0.100\"}\n",
    )
    .expect("terminal source");
    let terminal_path = config
        .root
        .parent()
        .expect("state parent")
        .join("terminal-receipt-archive-v1");
    TerminalReceiptArchive::open(&terminal_path)
        .expect("terminal archive")
        .sync_source(&terminal_source)
        .expect("terminal sync");

    let frame_path = config
        .root
        .parent()
        .expect("state parent")
        .join("relation-frame-archive-v1");
    let mut frame_archive = MultiSourceFrameArchive::open(&frame_path).expect("frame archive");
    if with_completed_frame {
        frame_archive
            .append(&completed_frame(prediction.turn_intent_id_sha256.clone()))
            .expect("completed frame");
    }

    let request = K1FuturePredictionCensorAuthorityRequestV1 {
        schema: K1_FUTURE_PREDICTION_CENSOR_AUTHORITY_REQUEST_SCHEMA_V1.to_owned(),
        lane: K1SchedulerLaneV1::Epistemic,
        prediction_root_sha256: prediction.prediction_root_sha256.clone(),
        fence_topology_commitment_root_sha256: fence_topology.commit.commitment_root_sha256,
        fence_provider_capture_request_root_sha256: fence_topology
            .commit
            .provider_capture_request_root_sha256,
    };
    CensorFixture {
        root_dir,
        config,
        signing_key,
        prediction,
        request,
    }
}

#[test]
fn authority_censors_missing_frame_and_restart_projection_is_identical() {
    let fixture = fixture("lineage", 11, 3_000, false);
    let projection = append_future_prediction_censor_authoritative(
        &fixture.config,
        &fixture.signing_key,
        fixture.request.clone(),
    )
    .expect("censor");
    assert_eq!(projection.future_prediction_censors.len(), 1);
    assert!(projection.future_outcomes.is_empty());
    assert!(!projection.authority_ready);
    assert!(!projection.phase_mutation_allowed);
    assert_eq!(
        projection.future_prediction_censors[0].prediction_root_sha256,
        fixture.prediction.prediction_root_sha256
    );

    let restored = restore_projection_for(&fixture.config, K1SchedulerLaneV1::Epistemic)
        .expect("restart projection");
    assert_eq!(
        restored.projection_root_sha256,
        projection.projection_root_sha256
    );
    let idempotent = append_future_prediction_censor_authoritative(
        &fixture.config,
        &fixture.signing_key,
        fixture.request,
    )
    .expect("idempotent retry");
    assert_eq!(
        idempotent.projection_root_sha256,
        projection.projection_root_sha256
    );
    std::fs::remove_dir_all(fixture.root_dir).expect("cleanup");
}

#[test]
fn authority_rejects_censor_when_completed_frame_exists() {
    let fixture = fixture("lineage", 11, 3_000, true);
    assert_eq!(
        append_future_prediction_censor_authoritative(
            &fixture.config,
            &fixture.signing_key,
            fixture.request,
        ),
        Err("k1_future_prediction_censor_completed_frame_exists".to_owned())
    );
    std::fs::remove_dir_all(fixture.root_dir).expect("cleanup");
}

#[test]
fn authority_rejects_wrong_lineage_and_preterminal_fence() {
    for (lineage, sequence, captured_at) in [("other", 11, 3_000), ("lineage", 11, 1_500)] {
        let fixture = fixture(lineage, sequence, captured_at, false);
        assert_eq!(
            append_future_prediction_censor_authoritative(
                &fixture.config,
                &fixture.signing_key,
                fixture.request,
            ),
            Err("k1_future_prediction_censor_fence_invalid".to_owned())
        );
        std::fs::remove_dir_all(fixture.root_dir).expect("cleanup");
    }
}
