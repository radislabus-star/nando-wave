use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_kernel::canonical_json_bytes;
use nando_operator_learning::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
#[ignore = "requires Linux bwrap and all four learned-composition binaries on the mini-PC"]
fn learned_effects_compose_into_unprovided_depth_three_program() {
    let mut environment = TestEnvironmentV1::new();
    let binaries = ProcessBinariesV1::from_cargo();
    binaries.assert_pairwise_distinct();
    let mut counters = ProcessCountersV1::default();

    assert_journal_fault_contract_v1(&environment);

    let mut main = run_positive_route_v1(
        RouteFixtureV1::main(),
        &environment,
        &binaries,
        &mut counters,
    );
    let mut topology = run_positive_route_v1(
        RouteFixtureV1::topology_control(),
        &environment,
        &binaries,
        &mut counters,
    );

    assert_eq!(
        (main.plan.valid_programs, main.plan.inapplicable_programs),
        (8, 7)
    );
    assert_eq!(main.law_set.laws.len(), 3);
    assert_eq!(main.plan.semantic_classes.len(), 5);
    assert_eq!(
        selected_class_v1(&main.plan)
            .member_program_roots_sha256
            .len(),
        3
    );
    assert_eq!(main.plan.dependency_edges.len(), 1);
    assert_eq!(
        (
            topology.plan.valid_programs,
            topology.plan.inapplicable_programs
        ),
        (3, 12)
    );
    assert_eq!(topology.law_set.laws.len(), 3);
    assert_eq!(topology.plan.semantic_classes.len(), 3);
    assert_eq!(
        selected_class_v1(&topology.plan)
            .member_program_roots_sha256
            .len(),
        1
    );
    assert_eq!(topology.plan.dependency_edges.len(), 2);
    assert_ne!(
        main.plan.normalized_topology_root_sha256,
        topology.plan.normalized_topology_root_sha256
    );

    let controls =
        run_negative_controls_v1(&main, &topology, &environment, &binaries, &mut counters);
    let ablations = K2CompositionAblationReceiptV1::seal(controls).expect("ablation receipt");
    assert_eq!(ablations.controls.len(), 18);
    assert_eq!(ablations.passed, 18);

    let main_terminal = finalize_route_v1(&mut main, &ablations, &environment);
    let topology_terminal = finalize_route_v1(&mut topology, &ablations, &environment);
    assert_eq!(counters.learners, 2);
    assert_eq!(counters.planners, 2);
    assert_eq!(counters.support_workers, 18);
    assert_eq!(counters.positive_target_workers, 2);
    assert_eq!(counters.negative_target_workers, 1);
    assert_eq!(counters.oracles, 3);

    let mut outcome = K2CompositionCapabilityOutcomeV1 {
        schema: K2_COMPOSITION_OUTCOME_SCHEMA_V1.to_owned(),
        verdict: K2_LEARNED_SEQUENTIAL_COMPOSITION_CAPABILITY_PASS_V1.to_owned(),
        route_outcome_roots_sha256: vec![
            main.plan.outcome_root_sha256.clone(),
            topology.plan.outcome_root_sha256.clone(),
        ],
        route_verification_roots_sha256: vec![
            main.verification.verification_root_sha256.clone(),
            topology.verification.verification_root_sha256.clone(),
        ],
        ablation_receipt_root_sha256: ablations.receipt_root_sha256.clone(),
        support_executions: 18,
        learned_laws: 6,
        candidate_programs: 30,
        verified_candidates: 30,
        target_executions: 2,
        exact_oracles: 3,
        journal_events_per_route: 29,
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        outcome_root_sha256: String::new(),
    };
    outcome.reseal().expect("capability outcome");
    let report = serde_json::json!({
        "verdict": outcome.verdict,
        "outcome_root_sha256": outcome.outcome_root_sha256,
        "main_plan_root_sha256": main.plan.outcome_root_sha256,
        "topology_plan_root_sha256": topology.plan.outcome_root_sha256,
        "main_terminal_journal_root_sha256": main_terminal,
        "topology_terminal_journal_root_sha256": topology_terminal,
        "ablation_root_sha256": ablations.receipt_root_sha256,
        "support_executions": counters.support_workers,
        "learned_laws": 6,
        "candidate_programs": 30,
        "negative_controls": ablations.passed,
        "authority": false
    });
    println!(
        "{}",
        serde_json::to_string(&report).expect("report encoding")
    );

    environment.cleanup();
    assert!(!environment.root.exists());
}

struct RouteFixtureV1 {
    label: &'static str,
    experiment_id_sha256: String,
    action_ids_sha256: Vec<String>,
    effects: Vec<K2CompositionLearnedEffectV1>,
    target_files: BTreeMap<String, Vec<u8>>,
    goal: K2CompositionExactGoalV1,
}

impl RouteFixtureV1 {
    fn main() -> Self {
        Self::new(
            "main",
            vec![copy("p0", "p1"), copy("p1", "p2"), remove("p3")],
            files(&[
                ("p0", b"target-main-content-v1"),
                ("p3", b"target-main-remove-v1"),
            ]),
            files(&[
                ("p0", b"target-main-content-v1"),
                ("p1", b"target-main-content-v1"),
                ("p2", b"target-main-content-v1"),
            ]),
        )
    }

    fn topology_control() -> Self {
        Self::new(
            "topology-control",
            vec![copy("q0", "q1"), copy("q1", "q2"), copy("q2", "q3")],
            files(&[("q0", b"target-topology-content-v1")]),
            files(&[
                ("q0", b"target-topology-content-v1"),
                ("q1", b"target-topology-content-v1"),
                ("q2", b"target-topology-content-v1"),
                ("q3", b"target-topology-content-v1"),
            ]),
        )
    }

    fn new(
        label: &'static str,
        effects: Vec<K2CompositionLearnedEffectV1>,
        target_files: BTreeMap<String, Vec<u8>>,
        goal_files: BTreeMap<String, Vec<u8>>,
    ) -> Self {
        let seed = root(&format!("k2-composition-{label}-seed"));
        let experiment_id_sha256 = root(&format!("k2-composition-{label}-experiment"));
        let action_ids_sha256 = (0..3)
            .map(|slot| opaque_action_id_v1(&seed, slot).expect("opaque action"))
            .collect();
        let goal = K2CompositionExactGoalV1::seal(
            K2CompositionTreeManifestV1::from_files(&goal_files).expect("goal manifest"),
        )
        .expect("exact goal");
        Self {
            label,
            experiment_id_sha256,
            action_ids_sha256,
            effects,
            target_files,
            goal,
        }
    }

    fn mapping(&self) -> K2CompositionPrivateMappingV1 {
        K2CompositionPrivateMappingV1::seal(
            self.experiment_id_sha256.clone(),
            self.action_ids_sha256
                .iter()
                .cloned()
                .zip(self.effects.iter().cloned())
                .map(|(action_id_sha256, effect)| K2CompositionMappingEntryV1 {
                    action_id_sha256,
                    effect,
                })
                .collect(),
        )
        .expect("private mapping")
    }

    fn support_files(&self, action_slot: usize, world: usize) -> BTreeMap<String, Vec<u8>> {
        let mut value = BTreeMap::new();
        let content = format!("support-{}-{action_slot}-{world}-content", self.label).into_bytes();
        match &self.effects[action_slot] {
            K2CompositionLearnedEffectV1::CopyFile { source_path, .. } => {
                value.insert(source_path.clone(), content);
            }
            K2CompositionLearnedEffectV1::RemoveFile { path } => {
                value.insert(path.clone(), content);
            }
        }
        value.insert(
            format!("noise/a{action_slot}/w{world}.txt"),
            format!("noise-{}-{action_slot}-{world}", self.label).into_bytes(),
        );
        value
    }
}

struct RouteRunV1 {
    fixture: RouteFixtureV1,
    mapping_directory: PathBuf,
    mapping_receipt: K2CompositionPrivateMappingArtifactReceiptV1,
    learning_request: K2CompositionLearningRequestV1,
    law_set: K2CompositionLearnedLawSetV1,
    planning_request: K2CompositionPlanningRequestV1,
    plan: K2CompositionPlannerOutcomeV1,
    verification: K2CompositionPlanVerificationReceiptV1,
    target_execution: K2CompositionSandboxExecutionV1,
    oracle: K2CompositionOracleOutcomeV1,
    journal: K2CompositionJournalV1,
}

fn run_positive_route_v1(
    fixture: RouteFixtureV1,
    environment: &TestEnvironmentV1,
    binaries: &ProcessBinariesV1,
    counters: &mut ProcessCountersV1,
) -> RouteRunV1 {
    let mapping = fixture.mapping();
    let mapping_directory = environment.private_store.join(fixture.label);
    let mapping_receipt = publish_private_mapping_artifact_v1(&mapping_directory, &mapping)
        .expect("mapping publication");
    let mut journal = K2CompositionJournalV1::create(
        &environment.journal_store,
        fixture.experiment_id_sha256.clone(),
    )
    .expect("composition journal");
    let freeze_root = composition_root_v1(&(
        "nando.k2-composition-freeze.v1",
        &fixture.experiment_id_sha256,
        &mapping_receipt.receipt_root_sha256,
        &binaries.learner_sha256,
        &binaries.planner_sha256,
        &binaries.worker_sha256,
        &binaries.oracle_sha256,
    ))
    .expect("freeze root");
    journal
        .append(
            K2CompositionJournalEventKindV1::ExperimentFrozen,
            "nando.k2-composition-freeze.v1",
            &freeze_root,
            1,
        )
        .expect("freeze append");
    assert_restart_prefix_v1(environment, &fixture, &journal);

    let adapter = K2CompositionSandboxAdapterV1::new(
        binaries.worker.clone(),
        binaries.worker_sha256.clone(),
        environment.workspace_store.clone(),
    )
    .expect("sandbox adapter");
    let mut observations = Vec::new();
    for action_slot in 0..3 {
        for world in 0..3 {
            let initial_files = fixture.support_files(action_slot, world);
            let initial_manifest =
                K2CompositionTreeManifestV1::from_files(&initial_files).expect("support manifest");
            let support_world_root = composition_root_v1(&(
                "nando.k2-composition-support-world.v1",
                fixture.label,
                action_slot,
                world,
                &initial_manifest,
            ))
            .expect("support world root");
            let request = K2CompositionSandboxRequestV1::seal(
                fixture.experiment_id_sha256.clone(),
                binaries.worker_sha256.clone(),
                initial_manifest.clone(),
                vec![fixture.effects[action_slot].clone()],
            )
            .expect("support sandbox request");
            journal
                .append(
                    K2CompositionJournalEventKindV1::SupportDispatched,
                    K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
                    &request.request_root_sha256,
                    2 + observations.len() as u64 * 2,
                )
                .expect("support dispatch append");
            assert_restart_prefix_v1(environment, &fixture, &journal);
            let execution = adapter
                .execute(&request, &initial_files)
                .expect("support bwrap");
            counters.support_workers += 1;
            assert!(execution.outcome.success);
            let observation = K2CompositionSupportObservationV1::seal(
                fixture.experiment_id_sha256.clone(),
                fixture.action_ids_sha256[action_slot].clone(),
                support_world_root,
                initial_manifest,
                execution.adapter_observed_post,
            )
            .expect("support observation");
            journal
                .append(
                    K2CompositionJournalEventKindV1::SupportObserved,
                    K2_COMPOSITION_OBSERVATION_SCHEMA_V1,
                    &observation.observation_root_sha256,
                    3 + observations.len() as u64 * 2,
                )
                .expect("support observation append");
            observations.push(observation);
            assert_restart_prefix_v1(environment, &fixture, &journal);
        }
    }

    let learning_request = K2CompositionLearningRequestV1::seal(
        fixture.experiment_id_sha256.clone(),
        binaries.learner_sha256.clone(),
        observations,
    )
    .expect("learning request");
    let law_set: K2CompositionLearnedLawSetV1 =
        run_isolated_protocol_v1(&binaries.learner, &learning_request);
    counters.learners += 1;
    let learner_receipt = K2CompositionProcessReceiptV1::seal(
        "effect_learner",
        binaries.learner_sha256.clone(),
        learning_request.request_root_sha256.clone(),
        law_set.law_set_root_sha256.clone(),
    )
    .expect("learner process receipt");
    assert_eq!(
        learner_receipt.authority,
        K2CompositionAuthorityBoundaryV1::denied()
    );
    journal
        .append(
            K2CompositionJournalEventKindV1::LearnedLawsFrozen,
            K2_COMPOSITION_LAW_SET_SCHEMA_V1,
            &law_set.law_set_root_sha256,
            20,
        )
        .expect("laws append");
    assert_restart_prefix_v1(environment, &fixture, &journal);

    let target =
        K2CompositionTreeManifestV1::from_files(&fixture.target_files).expect("target manifest");
    let independence = target_independence_v1(&learning_request, &target, &fixture.target_files);
    let target_goal_root = composition_root_v1(&(
        "nando.k2-composition-target-goal-freeze.v1",
        &target,
        &fixture.goal,
        &independence,
    ))
    .expect("target goal root");
    journal
        .append(
            K2CompositionJournalEventKindV1::TargetAndGoalFrozen,
            "nando.k2-composition-target-goal-freeze.v1",
            &target_goal_root,
            21,
        )
        .expect("target append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    let planning_request = K2CompositionPlanningRequestV1::seal(
        fixture.experiment_id_sha256.clone(),
        binaries.planner_sha256.clone(),
        law_set.clone(),
        target.clone(),
        fixture.goal.clone(),
        independence.receipt_root_sha256,
    )
    .expect("planning request");
    journal
        .append(
            K2CompositionJournalEventKindV1::PlanningRequestFrozen,
            K2_COMPOSITION_PLANNING_REQUEST_SCHEMA_V1,
            &planning_request.request_root_sha256,
            22,
        )
        .expect("planning request append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    let plan: K2CompositionPlannerOutcomeV1 =
        run_isolated_protocol_v1(&binaries.planner, &planning_request);
    counters.planners += 1;
    let planner_receipt = K2CompositionProcessReceiptV1::seal(
        "composition_planner",
        binaries.planner_sha256.clone(),
        planning_request.request_root_sha256.clone(),
        plan.outcome_root_sha256.clone(),
    )
    .expect("planner process receipt");
    assert_eq!(
        planner_receipt.authority,
        K2CompositionAuthorityBoundaryV1::denied()
    );
    journal
        .append(
            K2CompositionJournalEventKindV1::PlanFrozen,
            K2_COMPOSITION_PLANNER_OUTCOME_SCHEMA_V1,
            &plan.outcome_root_sha256,
            23,
        )
        .expect("plan append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    let verification = verify_composition_plan_v1(&planning_request, &plan)
        .expect("independent plan verification");
    journal
        .append(
            K2CompositionJournalEventKindV1::IndependentPlanVerificationFrozen,
            K2_COMPOSITION_PLAN_VERIFICATION_SCHEMA_V1,
            &verification.verification_root_sha256,
            24,
        )
        .expect("verification append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    assert_eq!(journal.projection().event_count, 24);

    let reopened_mapping = reopen_private_mapping_artifact_v1(&mapping_directory, &mapping_receipt)
        .expect("mapping reopen after plan");
    for law in &law_set.laws {
        assert_eq!(
            reopened_mapping.effect(&law.action_id_sha256),
            Some(&law.effect)
        );
    }
    let operations = plan
        .selected_program
        .action_ids_sha256
        .iter()
        .map(|action_id| {
            reopened_mapping
                .effect(action_id)
                .cloned()
                .expect("resolved operation")
        })
        .collect();
    let target_request = K2CompositionSandboxRequestV1::seal(
        fixture.experiment_id_sha256.clone(),
        binaries.worker_sha256.clone(),
        target,
        operations,
    )
    .expect("target sandbox request");
    journal
        .append(
            K2CompositionJournalEventKindV1::ExecutionDispatched,
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &target_request.request_root_sha256,
            25,
        )
        .expect("execution dispatch append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    assert!(
        !journal
            .projection()
            .same_identity_execution_dispatch_allowed
    );
    let reopened_dispatch = K2CompositionJournalV1::open_existing(
        &environment.journal_store,
        fixture.experiment_id_sha256.clone(),
    )
    .expect("dispatch restart");
    assert!(
        reopened_dispatch
            .projection()
            .indeterminate_after_execution_dispatch
    );
    let target_execution = adapter
        .execute(&target_request, &fixture.target_files)
        .expect("target bwrap");
    counters.positive_target_workers += 1;
    assert!(target_execution.outcome.success);
    let predicted = selected_candidate_terminal_v1(&plan);
    assert_eq!(&target_execution.adapter_observed_post, predicted);
    journal
        .append(
            K2CompositionJournalEventKindV1::ExecutionObserved,
            K2_COMPOSITION_SANDBOX_OUTCOME_SCHEMA_V1,
            &target_execution.outcome.outcome_root_sha256,
            26,
        )
        .expect("execution observation append");
    assert_restart_prefix_v1(environment, &fixture, &journal);
    let oracle_request = K2CompositionOracleRequestV1::seal(
        fixture.experiment_id_sha256.clone(),
        target_execution.adapter_observed_post.clone(),
        fixture.goal.clone(),
    )
    .expect("oracle request");
    let oracle: K2CompositionOracleOutcomeV1 =
        run_isolated_protocol_v1(&binaries.oracle, &oracle_request);
    counters.oracles += 1;
    assert!(oracle.exact_goal_satisfied);
    journal
        .append(
            K2CompositionJournalEventKindV1::ExactGoalVerified,
            K2_COMPOSITION_ORACLE_OUTCOME_SCHEMA_V1,
            &oracle.outcome_root_sha256,
            27,
        )
        .expect("oracle append");
    assert_restart_prefix_v1(environment, &fixture, &journal);

    RouteRunV1 {
        fixture,
        mapping_directory,
        mapping_receipt,
        learning_request,
        law_set,
        planning_request,
        plan,
        verification,
        target_execution,
        oracle,
        journal,
    }
}

fn run_negative_controls_v1(
    main: &RouteRunV1,
    topology: &RouteRunV1,
    environment: &TestEnvironmentV1,
    binaries: &ProcessBinariesV1,
    counters: &mut ProcessCountersV1,
) -> Vec<K2CompositionControlResultV1> {
    let mut controls = Vec::new();

    let insufficient = K2CompositionLearningRequestV1::seal(
        main.fixture.experiment_id_sha256.clone(),
        binaries.learner_sha256.clone(),
        main.learning_request.observations[1..].to_vec(),
    )
    .expect("bounded insufficient request");
    controls.push(control(
        1,
        "remove_one_support_world",
        "INSUFFICIENT_SUPPORT",
        error_contains(
            learn_composition_effects_v1(&insufficient),
            "insufficient_support",
        ),
    ));

    let no_delta_observations = main
        .learning_request
        .observations
        .iter()
        .map(|observation| {
            K2CompositionSupportObservationV1::seal(
                observation.experiment_id_sha256.clone(),
                observation.action_id_sha256.clone(),
                observation.support_world_root_sha256.clone(),
                observation.before.clone(),
                observation.before.clone(),
            )
            .expect("no-delta observation")
        })
        .collect();
    let no_delta = K2CompositionLearningRequestV1::seal(
        main.fixture.experiment_id_sha256.clone(),
        binaries.learner_sha256.clone(),
        no_delta_observations,
    )
    .expect("no-delta request");
    controls.push(control(
        2,
        "erase_post_action_deltas",
        "NO_IDENTIFIABLE_EFFECT",
        error_contains(
            learn_composition_effects_v1(&no_delta),
            "no_identifiable_effect",
        ),
    ));

    let ambiguous_action = &main.fixture.action_ids_sha256[0];
    let mut ambiguous_observations = Vec::new();
    for observation in &main.learning_request.observations {
        if &observation.action_id_sha256 != ambiguous_action {
            ambiguous_observations.push(observation.clone());
            continue;
        }
        let source_path = match &main.fixture.effects[0] {
            K2CompositionLearnedEffectV1::CopyFile { source_path, .. } => source_path,
            K2CompositionLearnedEffectV1::RemoveFile { .. } => panic!("main A must copy"),
        };
        let mut before_entries = observation.before.entries.clone();
        let mut alias = observation
            .before
            .entry(source_path)
            .expect("source entry")
            .clone();
        alias.path = "alias/shared.txt".to_owned();
        before_entries.push(alias.clone());
        let mut after_entries = observation.after.entries.clone();
        after_entries.push(alias);
        ambiguous_observations.push(
            K2CompositionSupportObservationV1::seal(
                observation.experiment_id_sha256.clone(),
                observation.action_id_sha256.clone(),
                observation.support_world_root_sha256.clone(),
                K2CompositionTreeManifestV1::seal_entries(before_entries)
                    .expect("ambiguous before"),
                K2CompositionTreeManifestV1::seal_entries(after_entries).expect("ambiguous after"),
            )
            .expect("ambiguous observation"),
        );
    }
    let ambiguous = K2CompositionLearningRequestV1::seal(
        main.fixture.experiment_id_sha256.clone(),
        binaries.learner_sha256.clone(),
        ambiguous_observations,
    )
    .expect("ambiguous request");
    controls.push(control(
        3,
        "duplicate_matching_copy_source",
        "AMBIGUOUS_EFFECT",
        error_contains(learn_composition_effects_v1(&ambiguous), "ambiguous_effect"),
    ));

    controls.push(control(
        4,
        "distinct_three_copy_topology",
        "DISTINCT_TOPOLOGY_PASS",
        topology.plan.valid_programs == 3
            && topology.plan.inapplicable_programs == 12
            && topology.plan.semantic_classes.len() == 3
            && topology.plan.normalized_topology_root_sha256
                != main.plan.normalized_topology_root_sha256,
    ));

    let mut leaked_learner = serde_json::to_value(&main.learning_request).expect("learner value");
    leaked_learner
        .as_object_mut()
        .expect("learner object")
        .insert(
            "private_mapping".to_owned(),
            serde_json::json!({"forbidden": true}),
        );
    let leaked_learner_bytes = canonical_json_bytes(&leaked_learner).expect("leaked learner bytes");
    controls.push(control(
        5,
        "private_mapping_leak",
        "PRIVATE_INPUT_REJECTED",
        composition_decode_v1::<K2CompositionLearningRequestV1>(&leaked_learner_bytes).is_err(),
    ));

    let mut leaked_planner = serde_json::to_value(&main.planning_request).expect("planner value");
    leaked_planner
        .as_object_mut()
        .expect("planner object")
        .insert(
            "expected_sequence".to_owned(),
            serde_json::json!(["A", "B", "C"]),
        );
    let leaked_planner_bytes = canonical_json_bytes(&leaked_planner).expect("leaked planner bytes");
    controls.push(control(
        6,
        "expected_sequence_leak",
        "PRIVATE_INPUT_REJECTED",
        composition_decode_v1::<K2CompositionPlanningRequestV1>(&leaked_planner_bytes).is_err(),
    ));

    let action_b = &main.fixture.action_ids_sha256[1];
    let action_a = &main.fixture.action_ids_sha256[0];
    let action_c = &main.fixture.action_ids_sha256[2];
    controls.push(control(
        7,
        "omit_learned_law_b",
        "NO_SATISFYING_PROGRAM",
        !main.plan.candidates.iter().any(|candidate| {
            !candidate.program.action_ids_sha256.contains(action_b)
                && candidate_satisfies(candidate)
        }),
    ));
    controls.push(control(
        8,
        "force_b_before_a",
        "INAPPLICABLE_AT_STEP",
        main.plan.candidates.iter().any(|candidate| {
            candidate
                .program
                .action_ids_sha256
                .starts_with(&[action_b.clone(), action_a.clone()])
                && matches!(
                    candidate.disposition,
                    K2CompositionProgramDispositionV1::InapplicableAtStep { step: 0, .. }
                )
        }),
    ));
    controls.push(control(
        9,
        "truncate_depth_two",
        "NO_SATISFYING_PROGRAM",
        !main
            .plan
            .candidates
            .iter()
            .any(|candidate| candidate.program.depth() <= 2 && candidate_satisfies(candidate)),
    ));
    controls.push(control(
        10,
        "omit_c_from_selected_program",
        "EXACT_GOAL_UNSATISFIED",
        !main.plan.candidates.iter().any(|candidate| {
            !candidate.program.action_ids_sha256.contains(action_c)
                && candidate_satisfies(candidate)
        }),
    ));

    let mut tampered_terminal = main.plan.clone();
    let valid_prediction = tampered_terminal
        .candidates
        .iter_mut()
        .find(|candidate| {
            matches!(
                candidate.disposition,
                K2CompositionProgramDispositionV1::ValidPrediction { .. }
            )
        })
        .expect("valid prediction to tamper");
    let K2CompositionProgramDispositionV1::ValidPrediction { terminal, .. } =
        &mut valid_prediction.disposition
    else {
        unreachable!("candidate was selected by valid disposition")
    };
    terminal.tree_root_sha256 = "f".repeat(64);
    tampered_terminal
        .reseal()
        .expect("tampered terminal reseal");
    controls.push(control(
        11,
        "tamper_candidate_terminal",
        "PLANNER_PARITY_FAILURE",
        verify_composition_plan_v1(&main.planning_request, &tampered_terminal).is_err(),
    ));

    let mut dropped = main.plan.clone();
    dropped.candidates.pop();
    dropped.reseal().expect("dropped candidate reseal");
    controls.push(control(
        12,
        "drop_enumerated_program",
        "PLANNER_PARITY_FAILURE",
        verify_composition_plan_v1(&main.planning_request, &dropped).is_err(),
    ));

    let mut split = main.plan.clone();
    let class_index = split
        .semantic_classes
        .iter()
        .position(|class| class.exact_goal_satisfied)
        .expect("satisfying class");
    let original = split.semantic_classes.remove(class_index);
    let mut remaining = original.member_program_roots_sha256.clone();
    let isolated = remaining.pop().expect("isolated member");
    split.semantic_classes.push(
        K2CompositionSemanticClassV1::seal(
            original.depth,
            original.action_multiset_sha256.clone(),
            original.terminal_tree_root_sha256.clone(),
            remaining,
            true,
        )
        .expect("split class one"),
    );
    split.semantic_classes.push(
        K2CompositionSemanticClassV1::seal(
            original.depth,
            original.action_multiset_sha256,
            original.terminal_tree_root_sha256,
            vec![isolated],
            true,
        )
        .expect("split class two"),
    );
    split
        .semantic_classes
        .sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    split.reseal().expect("split outcome reseal");
    controls.push(control(
        13,
        "split_equivalent_schedules",
        "QUOTIENT_MISMATCH",
        verify_composition_plan_v1(&main.planning_request, &split).is_err(),
    ));

    let mut merged = main.plan.clone();
    let first = merged.semantic_classes.remove(0);
    let second = merged.semantic_classes.remove(0);
    let mut members = first.member_program_roots_sha256.clone();
    members.extend(second.member_program_roots_sha256);
    merged.semantic_classes.push(
        K2CompositionSemanticClassV1::seal(
            first.depth,
            first.action_multiset_sha256,
            first.terminal_tree_root_sha256,
            members,
            first.exact_goal_satisfied,
        )
        .expect("merged class"),
    );
    merged
        .semantic_classes
        .sort_by(|left, right| left.class_root_sha256.cmp(&right.class_root_sha256));
    merged.reseal().expect("merged outcome reseal");
    controls.push(control(
        14,
        "merge_distinct_terminal_states",
        "QUOTIENT_MISMATCH",
        verify_composition_plan_v1(&main.planning_request, &merged).is_err(),
    ));

    let reopened =
        reopen_private_mapping_artifact_v1(&main.mapping_directory, &main.mapping_receipt)
            .expect("negative mapping reopen");
    let wrong_operations = [action_b, action_a, action_c]
        .iter()
        .map(|action| reopened.effect(action).cloned().expect("wrong operation"))
        .collect();
    let wrong_request = K2CompositionSandboxRequestV1::seal(
        main.fixture.experiment_id_sha256.clone(),
        binaries.worker_sha256.clone(),
        K2CompositionTreeManifestV1::from_files(&main.fixture.target_files)
            .expect("wrong target manifest"),
        wrong_operations,
    )
    .expect("wrong sandbox request");
    let adapter = K2CompositionSandboxAdapterV1::new(
        binaries.worker.clone(),
        binaries.worker_sha256.clone(),
        environment.workspace_store.clone(),
    )
    .expect("negative adapter");
    let wrong_execution = adapter
        .execute(&wrong_request, &main.fixture.target_files)
        .expect("negative bwrap execution");
    counters.negative_target_workers += 1;
    let wrong_oracle_request = K2CompositionOracleRequestV1::seal(
        main.fixture.experiment_id_sha256.clone(),
        wrong_execution.adapter_observed_post,
        main.fixture.goal.clone(),
    )
    .expect("negative oracle request");
    let wrong_oracle: K2CompositionOracleOutcomeV1 =
        run_isolated_protocol_v1(&binaries.oracle, &wrong_oracle_request);
    counters.oracles += 1;
    controls.push(control(
        15,
        "execute_nonrepresentative_wrong_order",
        "SANDBOX_EXECUTION_REJECTED",
        !wrong_execution.outcome.success && !wrong_oracle.exact_goal_satisfied,
    ));

    controls.push(control(
        16,
        "cross_experiment_plan_replay",
        "CROSS_EXPERIMENT_REPLAY",
        verify_composition_plan_v1(&topology.planning_request, &main.plan).is_err(),
    ));

    let mut authority = main.plan.clone();
    authority.authority.natural_k2_authority = true;
    authority.reseal().expect("authority outcome reseal");
    controls.push(control(
        17,
        "authority_bit_true",
        "AUTHORITY_BOUNDARY_VIOLATED",
        verify_composition_plan_v1(&main.planning_request, &authority).is_err(),
    ));

    let mut budget = main.plan.clone();
    let program = budget.candidates[0].program.clone();
    budget.candidates[0] = K2CompositionCandidateV1::seal(
        program,
        K2CompositionProgramDispositionV1::BudgetRejected {
            reason: "artificial".to_owned(),
        },
    )
    .expect("budget candidate");
    budget.budget_rejected_programs = 1;
    budget.reseal().expect("budget outcome reseal");
    controls.push(control(
        18,
        "artificial_budget_disposition",
        "PROGRAM_DENOMINATOR_MISMATCH",
        verify_composition_plan_v1(&main.planning_request, &budget).is_err(),
    ));
    controls
}

fn finalize_route_v1(
    route: &mut RouteRunV1,
    ablations: &K2CompositionAblationReceiptV1,
    environment: &TestEnvironmentV1,
) -> String {
    route
        .journal
        .append(
            K2CompositionJournalEventKindV1::AblationsFrozen,
            K2_COMPOSITION_ABLATION_SCHEMA_V1,
            &ablations.receipt_root_sha256,
            28,
        )
        .expect("ablations append");
    assert_restart_prefix_v1(environment, &route.fixture, &route.journal);
    let terminal_root = composition_root_v1(&(
        "nando.k2-composition-route-terminal.v1",
        &route.plan.outcome_root_sha256,
        &route.verification.verification_root_sha256,
        &route.target_execution.outcome.outcome_root_sha256,
        &route.oracle.outcome_root_sha256,
        &ablations.receipt_root_sha256,
        &K2CompositionAuthorityBoundaryV1::denied(),
    ))
    .expect("route terminal root");
    route
        .journal
        .append(
            K2CompositionJournalEventKindV1::Terminal,
            "nando.k2-composition-route-terminal.v1",
            &terminal_root,
            29,
        )
        .expect("terminal append");
    assert_restart_prefix_v1(environment, &route.fixture, &route.journal);
    assert_eq!(route.journal.events().len(), 29);
    let reopened = K2CompositionJournalV1::open_existing(
        &environment.journal_store,
        route.fixture.experiment_id_sha256.clone(),
    )
    .expect("terminal journal restart");
    assert_eq!(reopened.projection(), route.journal.projection());
    assert_eq!(
        reopened.projection().state,
        K2CompositionJournalStateV1::Terminal
    );
    reopened
        .projection()
        .latest_entry_root_sha256
        .clone()
        .expect("terminal journal root")
}

fn target_independence_v1(
    learning_request: &K2CompositionLearningRequestV1,
    target: &K2CompositionTreeManifestV1,
    target_files: &BTreeMap<String, Vec<u8>>,
) -> K2CompositionTargetIndependenceReceiptV1 {
    let support_roots = learning_request
        .observations
        .iter()
        .flat_map(|observation| {
            [
                observation.before.tree_root_sha256.as_str(),
                observation.after.tree_root_sha256.as_str(),
            ]
        })
        .collect::<BTreeSet<_>>();
    let support_content = learning_request
        .observations
        .iter()
        .flat_map(|observation| {
            observation
                .before
                .entries
                .iter()
                .chain(&observation.after.entries)
                .map(|entry| entry.content_sha256.as_str())
        })
        .collect::<BTreeSet<_>>();
    let learning_bytes = composition_bytes_v1(learning_request).expect("learning bytes");
    let target_bytes_absent = target_files.values().all(|bytes| {
        !learning_bytes
            .windows(bytes.len())
            .any(|window| window == bytes.as_slice())
    });
    K2CompositionTargetIndependenceReceiptV1::seal(
        learning_request.experiment_id_sha256.clone(),
        learning_request.request_root_sha256.clone(),
        target.tree_root_sha256.clone(),
        target
            .entries
            .iter()
            .all(|entry| !support_content.contains(entry.content_sha256.as_str())),
        !support_roots.contains(target.tree_root_sha256.as_str()),
        target_bytes_absent,
    )
    .expect("target independence")
}

fn assert_restart_prefix_v1(
    environment: &TestEnvironmentV1,
    fixture: &RouteFixtureV1,
    journal: &K2CompositionJournalV1,
) {
    let reopened = K2CompositionJournalV1::open_existing(
        &environment.journal_store,
        fixture.experiment_id_sha256.clone(),
    )
    .expect("journal prefix restart");
    assert_eq!(reopened.projection(), journal.projection());
}

fn assert_journal_fault_contract_v1(environment: &TestEnvironmentV1) {
    let before_publish_id = root("k2-composition-journal-fault-before-publish");
    let mut before_publish =
        journal_at_plan_verified_v1(&environment.journal_store, before_publish_id.clone());
    before_publish.set_next_fault_for_test_v1(K2CompositionJournalFaultPointV1::AfterTempSync);
    let payload_root = root("k2-composition-journal-fault-dispatch-before-publish");
    assert!(error_contains(
        before_publish.append(
            K2CompositionJournalEventKindV1::ExecutionDispatched,
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &payload_root,
            25,
        ),
        "injected_after_temp_sync",
    ));
    let reopened_before =
        K2CompositionJournalV1::open_existing(&environment.journal_store, before_publish_id)
            .expect("restart after unpublished dispatch");
    assert_eq!(reopened_before.projection().event_count, 24);
    assert!(
        reopened_before
            .projection()
            .same_identity_execution_dispatch_allowed
    );
    assert!(
        !reopened_before
            .projection()
            .indeterminate_after_execution_dispatch
    );

    let after_publish_id = root("k2-composition-journal-fault-after-publish");
    let mut after_publish =
        journal_at_plan_verified_v1(&environment.journal_store, after_publish_id.clone());
    after_publish.set_next_fault_for_test_v1(
        K2CompositionJournalFaultPointV1::AfterPublishBeforeDirectorySync,
    );
    let published_payload_root = root("k2-composition-journal-fault-dispatch-after-publish");
    assert!(error_contains(
        after_publish.append(
            K2CompositionJournalEventKindV1::ExecutionDispatched,
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &published_payload_root,
            25,
        ),
        "injected_after_event_publish",
    ));
    let mut reopened_after =
        K2CompositionJournalV1::open_existing(&environment.journal_store, after_publish_id)
            .expect("restart after published dispatch");
    assert_eq!(reopened_after.projection().event_count, 25);
    assert!(
        reopened_after
            .projection()
            .indeterminate_after_execution_dispatch
    );
    assert!(
        !reopened_after
            .projection()
            .same_identity_execution_dispatch_allowed
    );
    assert!(error_contains(
        reopened_after.append(
            K2CompositionJournalEventKindV1::ExecutionDispatched,
            K2_COMPOSITION_SANDBOX_REQUEST_SCHEMA_V1,
            &published_payload_root,
            26,
        ),
        "composition_journal_kind_order",
    ));
}

fn journal_at_plan_verified_v1(
    store: &Path,
    experiment_id_sha256: String,
) -> K2CompositionJournalV1 {
    let mut journal =
        K2CompositionJournalV1::create(store, experiment_id_sha256).expect("fault journal");
    for sequence in 0..24 {
        let payload_root = root(&format!("k2-composition-journal-fault-prefix-{sequence}"));
        journal
            .append(
                journal_kind_for_sequence_v1(sequence),
                "nando.k2-composition-journal-fault-payload.v1",
                &payload_root,
                sequence + 1,
            )
            .expect("fault journal prefix append");
    }
    assert_eq!(
        journal.projection().state,
        K2CompositionJournalStateV1::PlanVerified
    );
    journal
}

fn journal_kind_for_sequence_v1(sequence: u64) -> K2CompositionJournalEventKindV1 {
    match sequence {
        0 => K2CompositionJournalEventKindV1::ExperimentFrozen,
        1..=18 if sequence % 2 == 1 => K2CompositionJournalEventKindV1::SupportDispatched,
        2..=18 => K2CompositionJournalEventKindV1::SupportObserved,
        19 => K2CompositionJournalEventKindV1::LearnedLawsFrozen,
        20 => K2CompositionJournalEventKindV1::TargetAndGoalFrozen,
        21 => K2CompositionJournalEventKindV1::PlanningRequestFrozen,
        22 => K2CompositionJournalEventKindV1::PlanFrozen,
        23 => K2CompositionJournalEventKindV1::IndependentPlanVerificationFrozen,
        _ => panic!("unexpected fault journal sequence {sequence}"),
    }
}

fn selected_class_v1(plan: &K2CompositionPlannerOutcomeV1) -> &K2CompositionSemanticClassV1 {
    plan.semantic_classes
        .iter()
        .find(|class| class.class_root_sha256 == plan.selected_class_root_sha256)
        .expect("selected class")
}

fn selected_candidate_terminal_v1(
    plan: &K2CompositionPlannerOutcomeV1,
) -> &K2CompositionTreeManifestV1 {
    let candidate = plan
        .candidates
        .iter()
        .find(|candidate| candidate.program == plan.selected_program)
        .expect("selected candidate");
    match &candidate.disposition {
        K2CompositionProgramDispositionV1::ValidPrediction { terminal, .. } => terminal,
        _ => panic!("selected candidate must be valid"),
    }
}

fn candidate_satisfies(candidate: &K2CompositionCandidateV1) -> bool {
    matches!(
        candidate.disposition,
        K2CompositionProgramDispositionV1::ValidPrediction {
            exact_goal_satisfied: true,
            ..
        }
    )
}

fn control(
    control_id: u64,
    name: &str,
    expected: &str,
    passed: bool,
) -> K2CompositionControlResultV1 {
    K2CompositionControlResultV1 {
        control_id,
        name: name.to_owned(),
        expected_verdict: expected.to_owned(),
        observed_verdict: if passed { expected } else { "CONTROL_FAILED" }.to_owned(),
        passed,
    }
}

fn error_contains<T>(result: K2CompositionResultV1<T>, reason: &str) -> bool {
    result.is_err_and(|error| error.to_string().contains(reason))
}

fn copy(source: &str, target: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::CopyFile {
        source_path: source.to_owned(),
        target_path: target.to_owned(),
    }
}

fn remove(path: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::RemoveFile {
        path: path.to_owned(),
    }
}

fn files(rows: &[(&str, &[u8])]) -> BTreeMap<String, Vec<u8>> {
    rows.iter()
        .map(|(path, bytes)| ((*path).to_owned(), bytes.to_vec()))
        .collect()
}

fn root(label: &str) -> String {
    composition_root_v1(&("nando.k2-composition-test-root.v1", label)).expect("test root")
}

#[derive(Default)]
struct ProcessCountersV1 {
    learners: u64,
    planners: u64,
    support_workers: u64,
    positive_target_workers: u64,
    negative_target_workers: u64,
    oracles: u64,
}

struct ProcessBinariesV1 {
    learner: PathBuf,
    planner: PathBuf,
    worker: PathBuf,
    oracle: PathBuf,
    learner_sha256: String,
    planner_sha256: String,
    worker_sha256: String,
    oracle_sha256: String,
}

impl ProcessBinariesV1 {
    fn from_cargo() -> Self {
        let learner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-composition-effect-learner"));
        let planner = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-composition-planner"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-composition-sequential-worker"));
        let oracle = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-composition-exact-oracle"));
        Self {
            learner_sha256: composition_sha256_file_v1(&learner).expect("learner sha"),
            planner_sha256: composition_sha256_file_v1(&planner).expect("planner sha"),
            worker_sha256: composition_sha256_file_v1(&worker).expect("worker sha"),
            oracle_sha256: composition_sha256_file_v1(&oracle).expect("oracle sha"),
            learner,
            planner,
            worker,
            oracle,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let orchestrator = std::env::current_exe().expect("orchestrator executable");
        let orchestrator_sha = composition_sha256_file_v1(&orchestrator).expect("orchestrator sha");
        assert_eq!(
            [
                &self.learner_sha256,
                &self.planner_sha256,
                &self.worker_sha256,
                &self.oracle_sha256,
                &orchestrator_sha,
            ]
            .into_iter()
            .collect::<BTreeSet<_>>()
            .len(),
            5
        );
    }
}

fn run_isolated_protocol_v1<T, U>(executable: &Path, request: &T) -> U
where
    T: serde::Serialize,
    U: serde::de::DeserializeOwned + serde::Serialize,
{
    let guest = "/nando/bin/process";
    let mut command = Command::new("/usr/bin/bwrap");
    command.args([
        "--unshare-all",
        "--die-with-parent",
        "--new-session",
        "--cap-drop",
        "ALL",
        "--clearenv",
    ]);
    for path in ["/usr", "/lib", "/lib64"] {
        if Path::new(path).exists() {
            command.args(["--ro-bind", path, path]);
        }
    }
    command
        .args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp"])
        .args(["--dir", "/nando", "--dir", "/nando/bin"])
        .arg("--ro-bind")
        .arg(executable)
        .arg(guest)
        .args(["--setenv", "HOME", "/tmp", "--setenv", "LANG", "C"])
        .args(["--", "/usr/bin/prlimit", "--cpu=2:2"])
        .args(["--as=268435456:268435456", "--nproc=16:16"])
        .args(["--fsize=1048576:1048576", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn isolated protocol");
    child
        .stdin
        .take()
        .expect("protocol stdin")
        .write_all(&composition_bytes_v1(request).expect("protocol request bytes"))
        .expect("write protocol request");
    let mut stdout = child.stdout.take().expect("protocol stdout");
    let mut stderr = child.stderr.take().expect("protocol stderr");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .read_to_end(&mut bytes)
            .expect("read protocol stdout");
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .read_to_end(&mut bytes)
            .expect("read protocol stderr");
        bytes
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll protocol") {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "protocol timed out"
        );
        thread::sleep(Duration::from_millis(2));
    };
    let stdout = stdout_reader.join().expect("stdout reader");
    let stderr = stderr_reader.join().expect("stderr reader");
    assert!(
        status.success(),
        "protocol failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    composition_decode_v1(&stdout).expect("protocol outcome")
}

struct TestEnvironmentV1 {
    root: PathBuf,
    workspace_store: PathBuf,
    private_store: PathBuf,
    journal_store: PathBuf,
    cleaned: bool,
}

impl TestEnvironmentV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-composition-{}-{sequence}",
            std::process::id()
        ));
        let workspace_store = root.join("workspaces");
        let private_store = root.join("private");
        let journal_store = root.join("journals");
        fs::create_dir_all(&workspace_store).expect("workspace store");
        fs::create_dir_all(&private_store).expect("private store");
        fs::create_dir_all(&journal_store).expect("journal store");
        Self {
            root,
            workspace_store,
            private_store,
            journal_store,
            cleaned: false,
        }
    }

    fn cleanup(&mut self) {
        fs::remove_dir_all(&self.root).expect("test cleanup");
        self.cleaned = true;
    }
}

impl Drop for TestEnvironmentV1 {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
