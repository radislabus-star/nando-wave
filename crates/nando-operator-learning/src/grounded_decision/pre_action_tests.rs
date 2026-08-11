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

struct DurableDecisionFixture {
    precommit: DecisionContractPrecommitV1,
    selected: DurableSelectedActionBindingV1,
    goal_contract: TypedGoalContractV1,
    predicate: TypedGoalPredicateArtifactV1,
}

fn durable_decision_fixture(
    target_root_sha256: String,
    observed_root_sha256: String,
) -> DurableDecisionFixture {
    let sequence = 300;
    let verifier_contract_root_sha256 = root('6');
    let predicate = TypedGoalPredicateArtifactV1::seal(
        TypedGoalComparatorV1::TypedValueRootEquals,
        K1ConsequenceTypeV1::Scalar,
        target_root_sha256,
        verifier_contract_root_sha256.clone(),
    )
    .expect("predicate");
    let bound_goal = bind_exact_pre_action_goal_v1(ExactPreActionGoalInputV1 {
        predicate_artifact: predicate.clone(),
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
    .expect("goal");
    let authority = authority();
    let projection = K1ActionContractProjectionV1::seal(
        root('1'),
        root('2'),
        root('3'),
        root('4'),
        root('5'),
        verifier_contract_root_sha256,
        root('7'),
        K1ConsequenceTypeV1::Scalar,
    )
    .expect("projection");
    let binding = OpaqueActionExecutionBindingV1::seal(
        projection.action_contract_root_sha256.clone(),
        root('8'),
        root('9'),
        root('a'),
        authority.response_registry_root_sha256.clone(),
        authority.response_registry_revision,
        authority.certification_ledger_root_sha256.clone(),
        authority.certification_ledger_revision,
    )
    .expect("binding");
    let available_actions = AvailableActionContractsV1::seal(
        vec![projection.action_contract_root_sha256.clone()],
        root('b'),
    )
    .expect("available actions");
    let binding_roots = vec![binding.binding_root_sha256.clone()];
    let precommit = DecisionContractPrecommitV1::seal(DecisionContractPrecommitInputV1 {
        request_event_identity_root_sha256: root('c'),
        process_epoch_root_sha256: root('d'),
        pre_action_observation_root_sha256: root('2'),
        pre_action_topology_root_sha256: root('e'),
        goal_contract: bound_goal.goal_contract.clone(),
        goal_binding_receipt: bound_goal.binding_receipt,
        constraint_contract_root_sha256: root('f'),
        authority_snapshot: authority,
        applicability_evaluator_schema: "nando.response-pre-action-evaluator.v1".to_owned(),
        available_action_contracts_root_sha256: available_actions.contracts_root_sha256.clone(),
        opaque_execution_binding_set_root_sha256: opaque_action_execution_binding_set_root_v1(
            binding_roots.clone(),
        )
        .expect("binding set"),
        journal_sequence: sequence + 1,
        action_selection_not_before_sequence: sequence + 2,
        precommit_monotonic_nanos: sequence + 3,
    })
    .expect("precommit");
    let receipt = SelectedActionBindingReceiptV1::seal(
        &precommit,
        projection.action_contract_root_sha256.clone(),
        binding.binding_root_sha256.clone(),
        root('e'),
        precommit.action_selection_not_before_sequence,
        precommit.precommit_monotonic_nanos + 1,
        precommit.process_epoch_root_sha256.clone(),
    )
    .expect("selected receipt");
    let selected = DurableSelectedActionBindingV1::seal(
        &precommit,
        receipt,
        projection,
        binding,
        available_actions,
        binding_roots,
        observed_root_sha256,
    )
    .expect("selected action");
    DurableDecisionFixture {
        precommit,
        selected,
        goal_contract: bound_goal.goal_contract,
        predicate,
    }
}

#[test]
fn durable_selected_action_rejects_membership_rebound_and_wrong_epoch() {
    let fixture = durable_decision_fixture(root('a'), root('a'));
    fixture
        .selected
        .validate_join(&fixture.precommit)
        .expect("exact membership");

    let unavailable_actions =
        AvailableActionContractsV1::seal(vec![root('c')], root('b')).expect("other actions");
    assert!(
        DurableSelectedActionBindingV1::seal(
            &fixture.precommit,
            fixture.selected.receipt.clone(),
            fixture.selected.action_projection.clone(),
            fixture.selected.execution_binding.clone(),
            unavailable_actions,
            fixture
                .selected
                .opaque_execution_binding_roots_sha256
                .clone(),
            fixture.selected.observed_consequence_root_sha256.clone(),
        )
        .is_err()
    );

    let rebound = OpaqueActionExecutionBindingV1::seal(
        fixture
            .selected
            .action_projection
            .action_contract_root_sha256
            .clone(),
        root('8'),
        root('9'),
        root('a'),
        root('f'),
        fixture.precommit.response_registry_revision,
        fixture.precommit.certification_ledger_root_sha256.clone(),
        fixture.precommit.certification_ledger_revision,
    )
    .expect("rebound binding");
    let rebound_receipt = SelectedActionBindingReceiptV1::seal(
        &fixture.precommit,
        fixture
            .selected
            .action_projection
            .action_contract_root_sha256
            .clone(),
        rebound.binding_root_sha256.clone(),
        root('e'),
        fixture.precommit.action_selection_not_before_sequence,
        fixture.precommit.precommit_monotonic_nanos + 1,
        fixture.precommit.process_epoch_root_sha256.clone(),
    )
    .expect("rebound receipt");
    assert!(
        DurableSelectedActionBindingV1::seal(
            &fixture.precommit,
            rebound_receipt,
            fixture.selected.action_projection.clone(),
            rebound.clone(),
            fixture.selected.available_actions.clone(),
            vec![rebound.binding_root_sha256],
            fixture.selected.observed_consequence_root_sha256.clone(),
        )
        .is_err()
    );
    assert!(
        SelectedActionBindingReceiptV1::seal(
            &fixture.precommit,
            fixture
                .selected
                .action_projection
                .action_contract_root_sha256
                .clone(),
            fixture
                .selected
                .execution_binding
                .binding_root_sha256
                .clone(),
            root('e'),
            fixture.precommit.action_selection_not_before_sequence,
            fixture.precommit.precommit_monotonic_nanos + 1,
            root('f'),
        )
        .is_err()
    );
}

#[test]
fn exact_satisfaction_persists_true_and_false_but_rejects_wrong_truth_binding() {
    for (observed, expected) in [(root('a'), true), (root('c'), false)] {
        let fixture = durable_decision_fixture(root('a'), observed.clone());
        assert_eq!(
            verify_exact_goal_predicate_v1(
                &fixture.predicate,
                fixture.selected.action_projection.consequence_type,
                &fixture
                    .selected
                    .action_projection
                    .verifier_contract_root_sha256,
                &observed,
            ),
            Ok(expected)
        );
        let receipt = GoalSatisfactionReceiptV1::seal(
            &fixture.goal_contract,
            observed,
            fixture
                .selected
                .receipt
                .runtime_verification_receipt_root_sha256
                .clone(),
            expected,
        )
        .expect("satisfaction receipt");
        let durable = DurableGoalSatisfactionV1::seal(
            &fixture.precommit,
            &fixture.selected,
            fixture.goal_contract.clone(),
            fixture.predicate.clone(),
            receipt,
        )
        .expect("durable satisfaction");
        assert_eq!(durable.receipt.satisfied, expected);
    }

    let fixture = durable_decision_fixture(root('a'), root('a'));
    assert!(
        verify_exact_goal_predicate_v1(
            &fixture.predicate,
            K1ConsequenceTypeV1::Scalar,
            &root('f'),
            &root('a'),
        )
        .is_err()
    );
    let wrong_terminal = GoalSatisfactionReceiptV1::seal(
        &fixture.goal_contract,
        root('c'),
        fixture
            .selected
            .receipt
            .runtime_verification_receipt_root_sha256
            .clone(),
        false,
    )
    .expect("wrong terminal receipt");
    assert!(
        DurableGoalSatisfactionV1::seal(
            &fixture.precommit,
            &fixture.selected,
            fixture.goal_contract.clone(),
            fixture.predicate.clone(),
            wrong_terminal,
        )
        .is_err()
    );
    let wrong_verifier =
        GoalSatisfactionReceiptV1::seal(&fixture.goal_contract, root('a'), root('f'), true)
            .expect("wrong verifier receipt");
    assert!(
        DurableGoalSatisfactionV1::seal(
            &fixture.precommit,
            &fixture.selected,
            fixture.goal_contract.clone(),
            fixture.predicate.clone(),
            wrong_verifier,
        )
        .is_err()
    );
    let mut wrong_horizon = GoalSatisfactionReceiptV1::seal(
        &fixture.goal_contract,
        root('a'),
        fixture
            .selected
            .receipt
            .runtime_verification_receipt_root_sha256
            .clone(),
        true,
    )
    .expect("receipt");
    wrong_horizon.outcome_horizon_contract_root_sha256 = root('f');
    assert!(
        DurableGoalSatisfactionV1::seal(
            &fixture.precommit,
            &fixture.selected,
            fixture.goal_contract,
            fixture.predicate,
            wrong_horizon,
        )
        .is_err()
    );
}
