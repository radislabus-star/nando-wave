use crate::multi_source::K1ConsequenceTypeV1;

use super::*;

fn root(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn goal(sequence: u64) -> ExactPreActionGoalBindingV1 {
    let predicate = TypedGoalPredicateArtifactV1::seal(
        TypedGoalComparatorV1::CollectionMultisetRootEquals,
        K1ConsequenceTypeV1::Collection,
        root('a'),
        root('b'),
    )
    .expect("predicate");
    bind_exact_pre_action_goal_v1(ExactPreActionGoalInputV1 {
        predicate_artifact: predicate,
        pre_action_goal_evidence_root_sha256: root('c'),
        outcome_horizon_contract_root_sha256: root('d'),
        observation_mask_root_sha256: root('e'),
        feature_exclusion_root_sha256: root('f'),
        binder_schema_root_sha256: root('1'),
        pre_action_observation_root_sha256: root('2'),
        independent_binder_root_sha256: root('3'),
        frozen_at_sequence: sequence,
        action_selection_not_before_sequence: sequence + 2,
    })
    .expect("goal")
}

fn authority() -> DecisionAuthoritySnapshotV1 {
    DecisionAuthoritySnapshotV1::seal(
        "nando.response-registry.v6".to_owned(),
        11,
        root('4'),
        root('5'),
        12,
        root('6'),
        root('7'),
        root('8'),
    )
    .expect("authority")
}

fn precommit(sequence: u64) -> DecisionContractPrecommitV1 {
    let goal = goal(sequence);
    let actions = AvailableActionContractsV1::seal(vec![root('9')], root('a')).expect("actions");
    DecisionContractPrecommitV1::seal(DecisionContractPrecommitInputV1 {
        request_event_identity_root_sha256: root('b'),
        process_epoch_root_sha256: root('c'),
        pre_action_observation_root_sha256: root('2'),
        pre_action_topology_root_sha256: root('d'),
        goal_contract: goal.goal_contract,
        goal_binding_receipt: goal.binding_receipt,
        constraint_contract_root_sha256: root('e'),
        authority_snapshot: authority(),
        applicability_evaluator_schema: "nando.response-pre-action-evaluator.v1".to_owned(),
        available_action_contracts_root_sha256: actions.contracts_root_sha256,
        opaque_execution_binding_set_root_sha256: root('f'),
        journal_sequence: sequence + 1,
        action_selection_not_before_sequence: sequence + 2,
        precommit_monotonic_nanos: sequence + 3,
    })
    .expect("precommit")
}

#[test]
fn typed_predicate_is_exact_canonical_and_type_checked() {
    let artifact = TypedGoalPredicateArtifactV1::seal(
        TypedGoalComparatorV1::BooleanEquals,
        K1ConsequenceTypeV1::Boolean,
        root('1'),
        root('2'),
    )
    .expect("artifact");
    let bytes = artifact.canonical_bytes().expect("bytes");
    assert_eq!(
        TypedGoalPredicateArtifactV1::from_canonical_bytes(&bytes).expect("roundtrip"),
        artifact
    );
    assert!(
        TypedGoalPredicateArtifactV1::seal(
            TypedGoalComparatorV1::BooleanEquals,
            K1ConsequenceTypeV1::Scalar,
            root('1'),
            root('2'),
        )
        .is_err()
    );
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    value["free_text"] = serde_json::Value::String("forbidden".to_owned());
    let forged = serde_json::to_vec(&value).expect("forged");
    assert!(TypedGoalPredicateArtifactV1::from_canonical_bytes(&forged).is_err());
}

#[test]
fn action_identity_is_stable_across_package_bindings() {
    let projection = K1ActionContractProjectionV1::seal(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        root('6'),
        root('7'),
        K1ConsequenceTypeV1::Collection,
    )
    .expect("projection");
    let left = OpaqueActionExecutionBindingV1::seal(
        projection.action_contract_root_sha256.clone(),
        root('8'),
        root('9'),
        root('a'),
        root('b'),
        20,
        root('c'),
        21,
    )
    .expect("left");
    let right = OpaqueActionExecutionBindingV1::seal(
        projection.action_contract_root_sha256.clone(),
        root('d'),
        root('e'),
        root('f'),
        root('b'),
        20,
        root('c'),
        21,
    )
    .expect("right");
    assert_ne!(left.binding_root_sha256, right.binding_root_sha256);
    assert_eq!(
        left.action_contract_root_sha256,
        right.action_contract_root_sha256
    );
}

#[test]
fn precommit_is_fail_closed_and_tamper_evident() {
    let precommit = precommit(100);
    precommit.validate().expect("valid");
    assert!(!precommit.authority_ready);
    assert!(!precommit.phase_mutation_allowed);
    let mut forged = precommit.clone();
    forged.available_action_contracts_root_sha256 = root('0');
    assert_eq!(
        forged.validate(),
        Err("decision_contract_precommit_invalid")
    );
}

#[test]
fn selected_action_must_follow_the_durable_boundary() {
    let precommit = precommit(200);
    assert!(
        SelectedActionBindingReceiptV1::seal(
            &precommit,
            root('1'),
            root('2'),
            root('3'),
            precommit.action_selection_not_before_sequence - 1,
            precommit.precommit_monotonic_nanos,
            precommit.process_epoch_root_sha256.clone(),
        )
        .is_err()
    );
    let receipt = SelectedActionBindingReceiptV1::seal(
        &precommit,
        root('1'),
        root('2'),
        root('3'),
        precommit.action_selection_not_before_sequence,
        precommit.precommit_monotonic_nanos + 1,
        precommit.process_epoch_root_sha256.clone(),
    )
    .expect("receipt");
    receipt.validate().expect("valid");
}
