use nando_operator_kernel::sha256_bytes;

use super::*;

fn root(label: &str) -> String {
    sha256_bytes(label.as_bytes())
}

fn transition(seed: &str, lineage: &str) -> GroundedTransitionEpisodeV1 {
    GroundedTransitionEpisodeV1::seal(GroundedTransitionMaterialV1 {
        evidence_class: GroundedEvidenceClassV1::Natural,
        pre_action_state_root_sha256: root(&format!("{seed}:before")),
        observed_constraint_root_sha256: None,
        grounded_role_environment_root_sha256: root(&format!("{seed}:roles")),
        k1_law_id_sha256: root("law"),
        bundle_id_sha256: root("bundle"),
        action_binding_root_sha256: root(&format!("{seed}:action")),
        verified_delta_root_sha256: root(&format!("{seed}:delta")),
        post_action_state_root_sha256: root(&format!("{seed}:after")),
        independent_verifier_root_sha256: root(&format!("{seed}:verifier")),
        lineage_root_sha256: root(lineage),
        capture_generation_root_sha256: root("capture-generation"),
        disposition: TransitionTerminalDispositionV1::Positive,
        provenance_roots_sha256: vec![
            root(&format!("{seed}:source-b")),
            root(&format!("{seed}:source-a")),
        ],
    })
    .expect("transition")
}

fn goal(seed: &str, frozen_at_sequence: u64) -> TypedGoalContractV1 {
    TypedGoalContractV1::seal(
        root(&format!("{seed}:goal-evidence")),
        root(&format!("{seed}:predicate")),
        root(&format!("{seed}:horizon")),
        root(&format!("{seed}:mask")),
        root(&format!("{seed}:excluded")),
        root(&format!("{seed}:goal-verifier")),
        root(&format!("{seed}:binder-schema")),
        frozen_at_sequence,
    )
    .expect("goal")
}

fn complete_surface(seed: &str, lineage: &str) -> DecisionEvidenceSurfaceV1 {
    let transition = transition(seed, lineage);
    let goal = goal(seed, 7);
    let binding = PreActionGoalBindingReceiptV1::seal(
        &goal,
        transition.pre_action_state_root_sha256.clone(),
        root(&format!("{seed}:binder")),
        8,
    )
    .expect("binding");
    let selected_root = root(&format!("{seed}:selected"));
    let alternative_root = root(&format!("{seed}:alternative"));
    let available = AvailableActionContractsV1::seal(
        vec![alternative_root, selected_root.clone()],
        root("abstain"),
    )
    .expect("available");
    let selected = SelectedActionSequenceV1::seal(vec![selected_root]).expect("selected");
    let satisfaction = GoalSatisfactionReceiptV1::seal(
        &goal,
        transition.post_action_state_root_sha256.clone(),
        root(&format!("{seed}:satisfaction-verifier")),
        true,
    )
    .expect("satisfaction");
    DecisionEvidenceSurfaceV1 {
        transition,
        goal_contract: Some(goal),
        goal_binding_receipt: Some(binding),
        constraint_contract_root_sha256: Some(absent_constraint_contract_root_v1()),
        available_actions: Some(available),
        selected_action_sequence: Some(selected),
        goal_satisfaction_receipt: Some(satisfaction),
        provenance_verified: true,
    }
}

#[test]
fn transition_projection_is_deterministic_and_tamper_evident() {
    let first = transition("t1", "lineage-a");
    let second = transition("t1", "lineage-a");
    assert_eq!(first, second);
    first.validate().expect("valid transition");

    let mut tampered = first;
    tampered.post_action_state_root_sha256 = root("forged-after");
    assert_eq!(
        tampered.validate(),
        Err("grounded_transition_episode_root_mismatch")
    );
}

#[test]
fn goal_contract_cannot_depend_on_selected_action_or_outcome() {
    let frozen_goal = goal("g1", 11);
    let selected_a = SelectedActionSequenceV1::seal(vec![root("action-a")]).expect("action a");
    let selected_b = SelectedActionSequenceV1::seal(vec![root("action-b")]).expect("action b");
    assert_ne!(
        selected_a.sequence_root_sha256,
        selected_b.sequence_root_sha256
    );
    assert_eq!(frozen_goal, goal("g1", 11));

    assert_eq!(
        PreActionGoalBindingReceiptV1::seal(&frozen_goal, root("observation"), root("binder"), 11,),
        Err("pre_action_goal_binding_receipt_invalid")
    );
}

#[test]
fn decision_episode_requires_a_nonselected_k1_alternative() {
    let mut surface = complete_surface("d1", "lineage-a");
    let selected = surface
        .selected_action_sequence
        .clone()
        .expect("selected action");
    surface.available_actions = Some(
        AvailableActionContractsV1::seal(
            selected.action_contract_roots_sha256.clone(),
            root("abstain"),
        )
        .expect("single action surface"),
    );
    let projection = GroundedTransitionProjectionSnapshotV1::seal(
        root("source"),
        1,
        1,
        vec![surface.transition.clone()],
        Default::default(),
    )
    .expect("projection");
    let census = build_grounded_decision_census_v1(&projection, vec![surface]).expect("census");
    assert_eq!(census.decision_episodes, 0);
    assert_eq!(census.dynamics_only, 1);
    assert_eq!(
        census
            .blocker_counts
            .get(&DecisionCensusBlockerV1::MissingAlternative),
        Some(&1)
    );
}

#[test]
fn empty_decision_surface_is_explicit_and_restart_stable() {
    let episode = transition("empty", "lineage-a");
    let projection = GroundedTransitionProjectionSnapshotV1::seal(
        root("source-empty"),
        1,
        1,
        vec![episode.clone()],
        Default::default(),
    )
    .expect("projection");
    let first = build_grounded_decision_census_v1(
        &projection,
        vec![DecisionEvidenceSurfaceV1::dynamics_only(episode.clone())],
    )
    .expect("first census");
    let second = build_grounded_decision_census_v1(
        &projection,
        vec![DecisionEvidenceSurfaceV1::dynamics_only(episode)],
    )
    .expect("second census");
    assert_eq!(first, second);
    assert_eq!(first.verdict, "EMPTY_DECISION_SURFACE");
    assert_eq!(first.blocker, "missing_pre_action_goal");
    assert_eq!(first.goal_bound, 0);
    assert_eq!(first.alternative_bearing, 0);
    assert_eq!(first.dynamics_only, 1);
    assert!(!first.model_training_allowed);
    assert!(!first.authority_ready);
    assert!(!first.phase_mutation_allowed);
}

#[test]
fn complete_surfaces_need_two_independent_lineages_for_baselines() {
    let surface_a = complete_surface("a", "lineage-a");
    let surface_b = complete_surface("b", "lineage-b");
    let projection = GroundedTransitionProjectionSnapshotV1::seal(
        root("source-complete"),
        2,
        2,
        vec![surface_a.transition.clone(), surface_b.transition.clone()],
        Default::default(),
    )
    .expect("projection");
    let census =
        build_grounded_decision_census_v1(&projection, vec![surface_a, surface_b]).expect("census");
    assert_eq!(census.decision_episodes, 2);
    assert_eq!(census.distinct_decision_lineages, 2);
    assert_eq!(census.lineage_independent_episodes, 2);
    assert_eq!(census.verdict, "READY_FOR_BASELINES");
    assert!(census.model_training_allowed);
    assert!(!census.authority_ready);
}

#[test]
fn projection_denominator_and_snapshot_root_detect_tampering() {
    let mut censors = std::collections::BTreeMap::new();
    censors.insert(
        TransitionProjectionCensorReasonV1::MissingTransportBinding,
        2,
    );
    let snapshot = GroundedTransitionProjectionSnapshotV1::seal(
        root("source-denominator"),
        3,
        1,
        vec![transition("projected", "lineage-a")],
        censors,
    )
    .expect("snapshot");
    snapshot.validate().expect("valid snapshot");

    let mut tampered = snapshot;
    tampered.transition_rows_scanned = 4;
    assert_eq!(
        tampered.validate(),
        Err("grounded_transition_projection_denominator_mismatch")
    );
}
