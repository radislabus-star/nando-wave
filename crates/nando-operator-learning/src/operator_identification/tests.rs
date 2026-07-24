use nando_operator_kernel::{
    AtomValueType, OperatorGenerationComponentRootsV3, ProgramSemanticClassDescriptorV1,
    ProgramSemanticClassInputV1, ProjectStatusMapping, ResponseProgram, ResponseValueSelector,
    canonical_json_sha256, seal_operator_generation_manifest_v3, seal_program_semantic_class_v1,
};

use super::*;
use crate::{
    CandidateSearchCompletion, ExactProgramEvaluation, GenerationCensoredReasonV3,
    GenerationLearningOutcomeV3, VersionSpaceConfig,
};

fn root(label: &str) -> String {
    canonical_json_sha256(&label).expect("root")
}

fn manifest(label: &str) -> nando_operator_kernel::OperatorGenerationManifestV3 {
    seal_operator_generation_manifest_v3(
        1,
        None,
        OperatorGenerationComponentRootsV3 {
            artifact_set_sha256: root(&format!("{label}:artifacts")),
            dispatch_index_sha256: root(&format!("{label}:index")),
            actor_program_sha256: root(&format!("{label}:actor")),
            renderer_program_sha256: root(&format!("{label}:renderer")),
            verifier_contract_sha256: root(&format!("{label}:verifier")),
            capability_contract_sha256: root(&format!("{label}:capability")),
            resource_budget_sha256: root(&format!("{label}:budget")),
        },
    )
    .expect("manifest")
}

fn descriptor(label: &str) -> ProgramSemanticClassDescriptorV1 {
    seal_program_semantic_class_v1(ProgramSemanticClassInputV1 {
        effect_law_id_sha256: root(&format!("{label}:effect")),
        role_schema_root_sha256: root(&format!("{label}:roles")),
        protocol_mode_set_root_sha256: root(&format!("{label}:modes")),
        executable_behavior_root_sha256: root(&format!("{label}:behavior")),
        verifier_contract_root_sha256: root(&format!("{label}:verifier")),
    })
    .expect("descriptor")
}

fn program(field: &str) -> ResponseProgram {
    ResponseProgram::project_status(
        ResponseValueSelector::JsonField {
            field: field.to_owned(),
            value_type: AtomValueType::Integer,
        },
        ProjectStatusMapping::ZeroIsSuccess,
        "completed",
    )
}

fn observation(
    sequence: u64,
    label: &str,
    outcome: GenerationLearningOutcomeV3,
    evaluations: Vec<ExactProgramEvaluation>,
) -> OperatorObservationV1 {
    seal_operator_observation_v1(OperatorObservationInputV1 {
        capture_sequence: sequence,
        lineage_root_sha256: root(&format!("{label}:lineage")),
        event_root_sha256: root(&format!("{label}:event")),
        request_root_sha256: root(&format!("{label}:request")),
        pre_action_relation_root_sha256: root(&format!("{label}:before")),
        observed_action_root_sha256: root(&format!("{label}:action")),
        observed_delta_root_sha256: root(&format!("{label}:delta")),
        verifier_receipt_root_sha256: root(&format!("{label}:receipt")),
        outcome,
        evaluations,
    })
    .expect("observation")
}

fn accepted(program_digest_sha256: &str) -> ExactProgramEvaluation {
    ExactProgramEvaluation {
        program_digest_sha256: program_digest_sha256.to_owned(),
        accepted: true,
        reason: String::new(),
    }
}

fn rejected(program_digest_sha256: &str, reason: &str) -> ExactProgramEvaluation {
    ExactProgramEvaluation {
        program_digest_sha256: program_digest_sha256.to_owned(),
        accepted: false,
        reason: reason.to_owned(),
    }
}

#[test]
fn one_support_identifies_and_freezes_without_row_threshold() {
    let mut machine =
        OperatorIdentificationMachineV1::new(manifest("one-shot"), VersionSpaceConfig::default());
    let digest = machine
        .register_candidate(program("status"), descriptor("status"))
        .expect("candidate");
    assert_eq!(
        machine.complete_candidate_generation(),
        CandidateSearchCompletion::Complete
    );
    let update = machine
        .apply_support(observation(
            1,
            "support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&digest)],
        ))
        .expect("support");
    assert!(matches!(
        update.state,
        OperatorIdentificationStateV1::Identified { .. }
    ));

    let freeze = machine
        .freeze_candidate(2, root("narrow-scope"))
        .expect("freeze");
    assert!(!freeze.execution_authority());
    assert_eq!(
        machine
            .evidence_ledger()
            .expect("ledger")
            .accounting()
            .support_rows,
        1
    );
    let checkpoint = machine.checkpoint_bytes().expect("checkpoint");
    let restored =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(&checkpoint).expect("restored");
    assert_eq!(restored.checkpoint_bytes().expect("bytes"), checkpoint);
    assert!(!restored.execution_authority());
}

#[test]
fn one_independent_future_transfers_after_freeze_without_row_threshold() {
    let mut machine =
        OperatorIdentificationMachineV1::new(manifest("transfer"), VersionSpaceConfig::default());
    let digest = machine
        .register_candidate(program("status"), descriptor("status"))
        .expect("candidate");
    machine.complete_candidate_generation();
    machine
        .apply_support(observation(
            1,
            "support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&digest)],
        ))
        .expect("support");
    machine
        .freeze_candidate(2, root("narrow-scope"))
        .expect("freeze");

    let accounting = machine
        .apply_future(observation(
            2,
            "future",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&digest)],
        ))
        .expect("future");
    assert_eq!(accounting.support_rows, 1);
    assert_eq!(accounting.future_rows, 1);
    assert_eq!(accounting.support_lineages, 1);
    assert_eq!(accounting.future_lineages, 1);

    let checkpoint = machine.checkpoint_bytes().expect("checkpoint");
    let restored =
        OperatorIdentificationMachineV1::from_checkpoint_bytes(&checkpoint).expect("restored");
    assert_eq!(
        restored
            .evidence_ledger()
            .expect("restored ledger")
            .accounting(),
        accounting
    );
}

#[test]
fn future_rejects_support_lineage_and_foreign_semantic_class() {
    let mut machine = OperatorIdentificationMachineV1::new(
        manifest("future-guards"),
        VersionSpaceConfig::default(),
    );
    let selected = machine
        .register_candidate(program("selected"), descriptor("selected"))
        .expect("selected");
    let competing = machine
        .register_candidate(program("competing"), descriptor("competing"))
        .expect("competing");
    machine.complete_candidate_generation();
    machine
        .apply_support(observation(
            1,
            "support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![
                accepted(&selected),
                rejected(&competing, "support mismatch"),
            ],
        ))
        .expect("support");
    machine
        .freeze_candidate(2, root("narrow-scope"))
        .expect("freeze");

    let foreign = machine.apply_future(observation(
        2,
        "future",
        GenerationLearningOutcomeV3::VerifiedPass,
        vec![accepted(&selected), accepted(&competing)],
    ));
    assert_eq!(
        foreign.err(),
        Some(OperatorIdentificationErrorV1::FutureSemanticContradiction)
    );

    let same_lineage = seal_operator_observation_v1(OperatorObservationInputV1 {
        capture_sequence: 2,
        lineage_root_sha256: root("support:lineage"),
        event_root_sha256: root("same-lineage:event"),
        request_root_sha256: root("same-lineage:request"),
        pre_action_relation_root_sha256: root("same-lineage:before"),
        observed_action_root_sha256: root("same-lineage:action"),
        observed_delta_root_sha256: root("same-lineage:delta"),
        verifier_receipt_root_sha256: root("same-lineage:receipt"),
        outcome: GenerationLearningOutcomeV3::VerifiedPass,
        evaluations: vec![accepted(&selected), rejected(&competing, "future mismatch")],
    })
    .expect("same-lineage observation");
    assert_eq!(
        machine.apply_future(same_lineage).err(),
        Some(OperatorIdentificationErrorV1::EvidenceLedger)
    );
}

#[test]
fn ambiguity_requests_a_distinguishing_probe_then_collapses() {
    let mut machine =
        OperatorIdentificationMachineV1::new(manifest("ambiguous"), VersionSpaceConfig::default());
    let left_descriptor = descriptor("left");
    let right_descriptor = descriptor("right");
    let left = machine
        .register_candidate(program("left"), left_descriptor.clone())
        .expect("left");
    let right = machine
        .register_candidate(program("right"), right_descriptor.clone())
        .expect("right");
    machine.complete_candidate_generation();
    let state = machine
        .apply_support(observation(
            1,
            "ambiguous-support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&left), accepted(&right)],
        ))
        .expect("support")
        .state;
    let classes = match state {
        OperatorIdentificationStateV1::Ambiguous { report } => report.competing_class_ids,
        other => panic!("expected ambiguity, got {other:?}"),
    };
    let probe = select_distinguishing_probe_v1(
        &classes,
        &[DistinguishingProbeCandidateV1 {
            probe_root_sha256: root("probe"),
            observable_difference_root_sha256: root("different-selected-field"),
            source: EvidenceSourceContractV1::PassiveLiveTraffic,
            estimated_cost_units: 1,
            predictions: vec![
                ProbeClassPredictionV1 {
                    class_id: left_descriptor.class_id().clone(),
                    outcome_partition_root_sha256: root("left-outcome"),
                },
                ProbeClassPredictionV1 {
                    class_id: right_descriptor.class_id().clone(),
                    outcome_partition_root_sha256: root("right-outcome"),
                },
            ],
        }],
    )
    .expect("probe");
    assert_eq!(probe.expected_partition_gain(), 1);

    let update = machine
        .apply_support(observation(
            2,
            "distinguishing-support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&left), rejected(&right, "wrong selected role")],
        ))
        .expect("distinguishing support");
    assert_eq!(update.evidence.information_gain, 1);
    assert!(matches!(
        update.state,
        OperatorIdentificationStateV1::Identified { .. }
    ));
}

#[test]
fn syntax_variants_share_one_class_only_with_explicit_descriptor() {
    let mut machine =
        OperatorIdentificationMachineV1::new(manifest("quotient"), VersionSpaceConfig::default());
    let shared = descriptor("shared-law");
    let left = machine
        .register_candidate(program("physical_a"), shared.clone())
        .expect("left");
    let right = machine
        .register_candidate(program("physical_b"), shared)
        .expect("right");
    machine.complete_candidate_generation();
    let update = machine
        .apply_support(observation(
            1,
            "shared-support",
            GenerationLearningOutcomeV3::VerifiedPass,
            vec![accepted(&left), accepted(&right)],
        ))
        .expect("support");
    match update.state {
        OperatorIdentificationStateV1::Identified { class } => {
            assert_eq!(class.semantic_class().member_program_sha256().len(), 2);
        }
        other => panic!("expected one explicit semantic class, got {other:?}"),
    }
}

#[test]
fn incomplete_search_and_censored_outcome_never_freeze() {
    let mut machine =
        OperatorIdentificationMachineV1::new(manifest("incomplete"), VersionSpaceConfig::default());
    machine
        .register_candidate(program("status"), descriptor("status"))
        .expect("candidate");
    assert!(matches!(
        machine.state().expect("state"),
        OperatorIdentificationStateV1::Collecting { .. }
    ));
    assert_eq!(
        machine.freeze_candidate(2, root("scope")).err(),
        Some(OperatorIdentificationErrorV1::NotIdentified)
    );

    machine.complete_candidate_generation();
    machine
        .apply_support(observation(
            1,
            "censored",
            GenerationLearningOutcomeV3::Censored(GenerationCensoredReasonV3::VerifierUnavailable),
            Vec::new(),
        ))
        .expect("censored");
    assert!(matches!(
        machine.state().expect("state"),
        OperatorIdentificationStateV1::Collecting { .. }
    ));
    assert_eq!(machine.metrics().censored, 1);
    assert_eq!(machine.metrics().total_information_gain, 0);
}
