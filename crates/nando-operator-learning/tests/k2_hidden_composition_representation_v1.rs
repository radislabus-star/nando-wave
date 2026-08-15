use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn relation_atoms_compare_file_values_instead_of_path_identity() {
    let fixture = confirm_fixtures_v1().remove(1);
    let first = copy("confirm/c2/s", "confirm/c2/h");
    let second = copy("confirm/c2/h", "confirm/c2/l");
    let good = copy("confirm/c2/l", "confirm/c2/o");
    let bad = copy("confirm/c2/bad", "confirm/c2/o");
    let first_id = action_id_for_effect_v1(&fixture.task, &first);
    let second_id = action_id_for_effect_v1(&fixture.task, &second);
    let good_id = action_id_for_effect_v1(&fixture.task, &good);
    let bad_id = action_id_for_effect_v1(&fixture.task, &bad);
    let after_first =
        apply_feature_transition_v1(&fixture.task.initial, &first).expect("first copy");
    let current = apply_feature_transition_v1(&after_first, &second).expect("second copy");
    let used = vec![first_id, second_id];

    let good_features = extract_policy_features_v1(&fixture.task, &current, &used, &good_id)
        .expect("good copy features");
    let bad_features = extract_policy_features_v1(&fixture.task, &current, &used, &bad_id)
        .expect("bad copy features");
    assert_eq!(good_features.values[5], K2_REPRESENTATION_FEATURE_SCALE_V1);
    assert_eq!(bad_features.values[5], 0);

    let after_good = apply_feature_transition_v1(&current, &good).expect("good target copy");
    let settled = extract_policy_features_v1(&fixture.task, &after_good, &used, &good_id)
        .expect("settled target features");
    assert_eq!(settled.values[6], 0);
}

#[test]
#[ignore = "requires Linux bwrap and six isolated K2 representation binaries"]
fn learned_hidden_representation_transfers_under_frozen_search_budget() {
    let mut environment = TestEnvironmentV1::new();
    let binaries = ProcessBinariesV1::from_cargo();
    binaries.assert_pairwise_distinct();
    assert_journal_fault_contract_v1(&environment);

    let experiment_id = root("k2-hidden-representation-experiment-v1");
    let train_commitment = root("k2-hidden-representation-train-commitment-v1");
    let confirm_commitment = root("k2-hidden-representation-confirm-commitment-v1");
    let mut journal =
        K2RepresentationJournalV1::create(&environment.journal_store, experiment_id.clone())
            .expect("create representation journal");
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ExperimentFrozen,
        root("k2-hidden-representation-preregistration-v1"),
    );

    let train = train_fixtures_v1();
    assert_eq!(train.len(), K2_REPRESENTATION_TRAIN_TASKS_V1);
    let train_split_root = root_set_v1(
        "k2-hidden-representation-train-split-v1",
        train.iter().map(|fixture| &fixture.task.task_root_sha256),
    );
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::TrainSplitFrozen,
        composition_root_v1(&(train_commitment, train_split_root)).expect("train split root"),
    );

    let train_baselines = train
        .iter()
        .map(|fixture| run_baseline_process_v1(&binaries, &fixture.task))
        .collect::<Vec<_>>();
    assert!(train_baselines.iter().all(|baseline| {
        baseline.complete_programs == K2_REPRESENTATION_COMPLETE_PROGRAMS_V1
            && !baseline.minimum_satisfying_programs.is_empty()
    }));
    let train_baseline_root = root_set_v1(
        "k2-hidden-representation-train-baselines-v1",
        train_baselines
            .iter()
            .map(|baseline| &baseline.outcome_root_sha256),
    );
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::TrainBaselinesFrozen,
        train_baseline_root,
    );

    let train_tasks = train
        .iter()
        .map(|fixture| fixture.task.clone())
        .collect::<Vec<_>>();
    let corpus = project_representation_training_corpus_v1(&train_tasks, &train_baselines)
        .expect("project source-neutral training corpus");
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::TrainCorpusFrozen,
        corpus.corpus_root_sha256.clone(),
    );
    let trainer_request =
        K2RepresentationTrainerRequestV1::seal(binaries.trainer_sha256.clone(), corpus.clone())
            .expect("trainer request");
    let trainer_bytes = representation_bytes_v1(&trainer_request).expect("trainer bytes");
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ModelTrainingDispatched,
        trainer_request.request_root_sha256.clone(),
    );
    let model: K2MeaningPolicySnapshotV1 =
        run_isolated_protocol_v1(&binaries.trainer, &trainer_request);
    model.validate().expect("frozen model valid");
    assert_eq!(model.control_variant, "trained");
    assert!(model.nonzero_parameters >= 16);
    assert!(model.correctly_ranked_pairs * 100 >= model.training_pairs * 95);
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ModelFrozen,
        model.model_root_sha256.clone(),
    );

    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ConfirmSealOpened,
        confirm_commitment,
    );
    let confirm = confirm_fixtures_v1();
    let confirm_tasks = confirm
        .iter()
        .map(|fixture| fixture.task.clone())
        .collect::<Vec<_>>();
    let confirm_seal = K2RepresentationConfirmSealReceiptV1::seal(
        model.model_root_sha256.clone(),
        &train_tasks,
        &confirm_tasks,
        &trainer_bytes,
    )
    .expect("sealed confirm split");
    assert_eq!(
        (
            confirm_seal.root_intersections,
            confirm_seal.trainer_byte_leaks
        ),
        (0, 0)
    );
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ConfirmSplitFrozen,
        confirm_seal.receipt_root_sha256.clone(),
    );

    let confirm_baselines = confirm
        .iter()
        .map(|fixture| run_baseline_process_v1(&binaries, &fixture.task))
        .collect::<Vec<_>>();
    assert!(confirm_baselines.iter().all(|baseline| {
        baseline.complete_programs == K2_REPRESENTATION_COMPLETE_PROGRAMS_V1
            && baseline.minimum_satisfying_depth == K2_REPRESENTATION_MAX_DEPTH_V1
            && baseline.satisfying_strict_prefixes == 0
    }));
    let confirm_baseline_root = root_set_v1(
        "k2-hidden-representation-confirm-baselines-v1",
        confirm_baselines
            .iter()
            .map(|baseline| &baseline.outcome_root_sha256),
    );
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ConfirmBaselinesFrozen,
        confirm_baseline_root,
    );

    let policy_requests = confirm
        .iter()
        .map(|fixture| {
            K2RepresentationPolicyRequestV1::seal(
                binaries.policy_sha256.clone(),
                model.clone(),
                fixture.task.clone(),
            )
            .expect("policy request")
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::PoliciesDispatched,
        root_set_v1(
            "k2-hidden-representation-policy-requests-v1",
            policy_requests
                .iter()
                .map(|request| &request.request_root_sha256),
        ),
    );
    let policies = policy_requests
        .iter()
        .enumerate()
        .map(|(index, request)| {
            let outcome: K2RepresentationPolicyOutcomeV1 =
                run_isolated_protocol_v1(&binaries.policy, request);
            assert!(outcome.action_evaluations <= K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1);
            assert!(
                outcome.action_evaluations * 10_000 <= K2_REPRESENTATION_COMPLETE_PROGRAMS_V1 * 120
            );
            if !outcome.exact_goal_satisfied {
                eprintln!(
                    "{}",
                    serde_json::to_string(&policy_forensic_v1(
                        index,
                        &confirm[index],
                        &confirm_baselines[index],
                        &outcome,
                    ))
                    .expect("policy forensic JSON")
                );
            }
            outcome
        })
        .collect::<Vec<_>>();
    assert_eq!(policies.len(), K2_REPRESENTATION_CONFIRM_TASKS_V1);
    assert!(
        policies.iter().all(|outcome| outcome.exact_goal_satisfied),
        "frozen confirm policy failed: {:?}",
        policies
            .iter()
            .map(|outcome| outcome.exact_goal_satisfied)
            .collect::<Vec<_>>()
    );
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::PoliciesFrozen,
        root_set_v1(
            "k2-hidden-representation-policies-v1",
            policies.iter().map(|outcome| &outcome.outcome_root_sha256),
        ),
    );

    let verifier_requests = (0..confirm.len())
        .map(|index| {
            K2RepresentationVerifierRequestV1::seal(
                binaries.verifier_sha256.clone(),
                policy_requests[index].clone(),
                policies[index].clone(),
                confirm_baselines[index].clone(),
            )
            .expect("verifier request")
        })
        .collect::<Vec<_>>();
    let verifications = verifier_requests
        .iter()
        .map(|request| {
            let receipt: K2RepresentationVerificationReceiptV1 =
                run_isolated_protocol_v1(&binaries.verifier, request);
            assert!(receipt.selected_is_minimum_satisfying);
            assert!(receipt.exact_goal_satisfied);
            receipt
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::IndependentVerificationsFrozen,
        root_set_v1(
            "k2-hidden-representation-verifications-v1",
            verifications
                .iter()
                .map(|receipt| &receipt.verification_root_sha256),
        ),
    );

    let adapter = K2RepresentationSandboxAdapterV1::new(
        binaries.worker.clone(),
        binaries.worker_sha256.clone(),
        environment.workspace_store.clone(),
    )
    .expect("sandbox adapter");
    let mut executions = Vec::new();
    let mut oracles = Vec::new();
    for index in 0..confirm.len() {
        let selected = policies[index]
            .selected_program
            .as_ref()
            .expect("verified selected program");
        let operations = selected
            .action_ids_sha256
            .iter()
            .map(|action_id| {
                confirm[index]
                    .task
                    .law(action_id)
                    .expect("selected law")
                    .effect
                    .clone()
            })
            .collect::<Vec<_>>();
        let request = K2RepresentationSandboxRequestV1::seal(
            confirm[index].task.experiment_id_sha256.clone(),
            binaries.worker_sha256.clone(),
            confirm[index].task.initial.clone(),
            operations,
        )
        .expect("sandbox request");
        append_and_reopen_v1(
            &mut journal,
            &environment,
            &experiment_id,
            if index == 0 {
                K2RepresentationJournalEventKindV1::Execution1Dispatched
            } else {
                K2RepresentationJournalEventKindV1::Execution2Dispatched
            },
            request.request_root_sha256.clone(),
        );
        let execution = adapter
            .execute(&request, &confirm[index].initial_files)
            .expect("isolated representation execution");
        assert!(execution.outcome.success);
        assert!(execution.source_integrity_preserved);
        assert!(execution.workspace_removed);
        assert_eq!(
            execution.adapter_observed_post,
            confirm[index].task.goal.expected_terminal
        );
        append_and_reopen_v1(
            &mut journal,
            &environment,
            &experiment_id,
            if index == 0 {
                K2RepresentationJournalEventKindV1::Execution1Observed
            } else {
                K2RepresentationJournalEventKindV1::Execution2Observed
            },
            execution.outcome.outcome_root_sha256.clone(),
        );
        let oracle_request = K2CompositionOracleRequestV1::seal(
            confirm[index].task.experiment_id_sha256.clone(),
            execution.adapter_observed_post.clone(),
            confirm[index].task.goal.clone(),
        )
        .expect("oracle request");
        let oracle: K2CompositionOracleOutcomeV1 =
            run_isolated_protocol_v1(&binaries.oracle, &oracle_request);
        assert!(oracle.exact_goal_satisfied);
        append_and_reopen_v1(
            &mut journal,
            &environment,
            &experiment_id,
            if index == 0 {
                K2RepresentationJournalEventKindV1::Oracle1Frozen
            } else {
                K2RepresentationJournalEventKindV1::Oracle2Frozen
            },
            oracle.outcome_root_sha256.clone(),
        );
        executions.push(execution);
        oracles.push(oracle);
    }

    let controls = run_controls_v1(
        &trainer_request,
        &model,
        &confirm,
        &confirm_baselines,
        &policy_requests,
        &policies,
        &binaries,
    );
    let ablations = K2CompositionAblationReceiptV1::seal(controls).expect("control receipt");
    if ablations.passed != 18 {
        eprintln!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "schema": "nando.k2-hidden-representation-control-forensic.v1",
                "controls": &ablations.controls,
                "passed": ablations.passed,
                "authority": false
            }))
            .expect("control forensic JSON")
        );
    }
    assert_eq!((ablations.controls.len(), ablations.passed), (18, 18));
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::ControlsFrozen,
        ablations.receipt_root_sha256.clone(),
    );

    let terminal_root = composition_root_v1(&(
        K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_PASS_V1,
        &model.model_root_sha256,
        &confirm_seal.receipt_root_sha256,
        &policies
            .iter()
            .map(|outcome| &outcome.outcome_root_sha256)
            .collect::<Vec<_>>(),
        &verifications
            .iter()
            .map(|receipt| &receipt.verification_root_sha256)
            .collect::<Vec<_>>(),
        &oracles
            .iter()
            .map(|oracle| &oracle.outcome_root_sha256)
            .collect::<Vec<_>>(),
        &ablations.receipt_root_sha256,
        &K2CompositionAuthorityBoundaryV1::denied(),
    ))
    .expect("terminal result root");
    append_and_reopen_v1(
        &mut journal,
        &environment,
        &experiment_id,
        K2RepresentationJournalEventKindV1::TerminalFrozen,
        terminal_root.clone(),
    );
    assert!(journal.projection().terminal);
    let journal_terminal_root = journal
        .projection()
        .last_event_root_sha256
        .clone()
        .expect("terminal journal root");

    let report = serde_json::json!({
        "schema": "nando.k2-hidden-composition-representation-capability-result.v1",
        "verdict": K2_HIDDEN_COMPOSITION_REPRESENTATION_CAPABILITY_PASS_V1,
        "train_tasks": train.len(),
        "train_complete_programs_each": K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
        "training_pairs": model.training_pairs,
        "correctly_ranked_pairs": model.correctly_ranked_pairs,
        "nonzero_parameters": model.nonzero_parameters,
        "model_root_sha256": model.model_root_sha256,
        "confirm_exact_goals": policies.iter().filter(|outcome| outcome.exact_goal_satisfied).count(),
        "confirm_total": confirm.len(),
        "action_evaluations": policies.iter().map(|outcome| outcome.action_evaluations).collect::<Vec<_>>(),
        "complete_programs_each": K2_REPRESENTATION_COMPLETE_PROGRAMS_V1,
        "independent_verifications": verifications.len(),
        "sandbox_executions": executions.len(),
        "exact_oracles": oracles.len(),
        "controls_passed": ablations.passed,
        "controls_total": ablations.controls.len(),
        "confirm_seal_root_sha256": confirm_seal.receipt_root_sha256,
        "ablation_root_sha256": ablations.receipt_root_sha256,
        "terminal_root_sha256": terminal_root,
        "journal_terminal_root_sha256": journal_terminal_root,
        "authority": false
    });
    println!("{}", serde_json::to_string(&report).expect("result JSON"));

    journal.cleanup().expect("journal cleanup");
    environment.cleanup();
    assert!(!environment.root.exists());
}

#[derive(Clone)]
struct FixtureV1 {
    task: K2RepresentationTaskV1,
    initial_files: BTreeMap<String, Vec<u8>>,
    goal_files: BTreeMap<String, Vec<u8>>,
}

fn train_fixtures_v1() -> Vec<FixtureV1> {
    vec![
        fixture_v1(
            "train-t1",
            vec![
                copy("train/t1/s", "train/t1/m"),
                copy("train/t1/m", "train/t1/o"),
                remove("train/t1/m"),
                copy("train/t1/s", "train/t1/j"),
                remove("train/t1/s"),
                copy("train/t1/x", "train/t1/y"),
                remove("train/t1/z"),
            ],
            files(&[("train/t1/s", b"train-t1-value")]),
            files(&[
                ("train/t1/s", b"train-t1-value"),
                ("train/t1/o", b"train-t1-value"),
            ]),
        ),
        fixture_v1(
            "train-t2",
            vec![
                copy("train/t2/s", "train/t2/a"),
                copy("train/t2/a", "train/t2/b"),
                copy("train/t2/b", "train/t2/o"),
                remove("train/t2/a"),
                remove("train/t2/b"),
                copy("train/t2/s", "train/t2/j"),
                remove("train/t2/s"),
            ],
            files(&[("train/t2/s", b"train-t2-value")]),
            files(&[
                ("train/t2/s", b"train-t2-value"),
                ("train/t2/o", b"train-t2-value"),
            ]),
        ),
        fixture_v1(
            "train-t3",
            vec![
                copy("train/t3/s", "train/t3/m"),
                copy("train/t3/m", "train/t3/o1"),
                copy("train/t3/m", "train/t3/o2"),
                remove("train/t3/m"),
                copy("train/t3/s", "train/t3/j"),
                remove("train/t3/s"),
                remove("train/t3/o1"),
            ],
            files(&[("train/t3/s", b"train-t3-value")]),
            files(&[
                ("train/t3/s", b"train-t3-value"),
                ("train/t3/o1", b"train-t3-value"),
                ("train/t3/o2", b"train-t3-value"),
            ]),
        ),
        fixture_v1(
            "train-t4",
            vec![
                copy("train/t4/s1", "train/t4/m1"),
                copy("train/t4/m1", "train/t4/o1"),
                copy("train/t4/s2", "train/t4/m2"),
                copy("train/t4/m2", "train/t4/o2"),
                remove("train/t4/m1"),
                copy("train/t4/s1", "train/t4/j"),
                remove("train/t4/s1"),
            ],
            files(&[
                ("train/t4/s1", b"train-t4-a"),
                ("train/t4/s2", b"train-t4-b"),
            ]),
            files(&[
                ("train/t4/s1", b"train-t4-a"),
                ("train/t4/s2", b"train-t4-b"),
                ("train/t4/o1", b"train-t4-a"),
                ("train/t4/m2", b"train-t4-b"),
                ("train/t4/o2", b"train-t4-b"),
            ]),
        ),
        fixture_v1(
            "train-t5",
            vec![
                copy("train/t5/s", "train/t5/m1"),
                copy("train/t5/m1", "train/t5/m2"),
                copy("train/t5/m2", "train/t5/o"),
                remove("train/t5/m1"),
                copy("train/t5/bad", "train/t5/o"),
                remove("train/t5/s"),
                copy("train/t5/x", "train/t5/y"),
            ],
            files(&[
                ("train/t5/s", b"train-t5-good"),
                ("train/t5/bad", b"train-t5-bad"),
            ]),
            files(&[
                ("train/t5/s", b"train-t5-good"),
                ("train/t5/bad", b"train-t5-bad"),
                ("train/t5/m2", b"train-t5-good"),
                ("train/t5/o", b"train-t5-good"),
            ]),
        ),
        fixture_v1(
            "train-t6",
            vec![
                copy("train/t6/s", "train/t6/m"),
                copy("train/t6/m", "train/t6/o1"),
                copy("train/t6/m", "train/t6/o2"),
                remove("train/t6/m"),
                copy("train/t6/bad", "train/t6/o1"),
                remove("train/t6/s"),
                copy("train/t6/x", "train/t6/y"),
            ],
            files(&[
                ("train/t6/s", b"train-t6-good"),
                ("train/t6/bad", b"train-t6-bad"),
            ]),
            files(&[
                ("train/t6/s", b"train-t6-good"),
                ("train/t6/bad", b"train-t6-bad"),
                ("train/t6/o1", b"train-t6-good"),
                ("train/t6/o2", b"train-t6-good"),
            ]),
        ),
    ]
}

fn confirm_fixtures_v1() -> Vec<FixtureV1> {
    vec![
        fixture_v1(
            "confirm-c1",
            vec![
                copy("confirm/c1/s", "confirm/c1/a"),
                copy("confirm/c1/a", "confirm/c1/b"),
                copy("confirm/c1/b", "confirm/c1/c"),
                copy("confirm/c1/c", "confirm/c1/o"),
                remove("confirm/c1/a"),
                remove("confirm/c1/b"),
                copy("confirm/c1/bad", "confirm/c1/o"),
            ],
            files(&[
                ("confirm/c1/s", b"confirm-c1-good"),
                ("confirm/c1/bad", b"confirm-c1-bad"),
            ]),
            files(&[
                ("confirm/c1/s", b"confirm-c1-good"),
                ("confirm/c1/bad", b"confirm-c1-bad"),
                ("confirm/c1/c", b"confirm-c1-good"),
                ("confirm/c1/o", b"confirm-c1-good"),
            ]),
        ),
        fixture_v1(
            "confirm-c2",
            vec![
                copy("confirm/c2/s", "confirm/c2/h"),
                copy("confirm/c2/h", "confirm/c2/l"),
                copy("confirm/c2/h", "confirm/c2/r"),
                copy("confirm/c2/l", "confirm/c2/o"),
                remove("confirm/c2/h"),
                remove("confirm/c2/l"),
                copy("confirm/c2/bad", "confirm/c2/o"),
            ],
            files(&[
                ("confirm/c2/s", b"confirm-c2-good"),
                ("confirm/c2/bad", b"confirm-c2-bad"),
            ]),
            files(&[
                ("confirm/c2/s", b"confirm-c2-good"),
                ("confirm/c2/bad", b"confirm-c2-bad"),
                ("confirm/c2/r", b"confirm-c2-good"),
                ("confirm/c2/o", b"confirm-c2-good"),
            ]),
        ),
    ]
}

fn fixture_v1(
    label: &str,
    effects: Vec<K2CompositionLearnedEffectV1>,
    initial_files: BTreeMap<String, Vec<u8>>,
    goal_files: BTreeMap<String, Vec<u8>>,
) -> FixtureV1 {
    let seed = root(&format!("k2-hidden-representation-{label}-seed-v1"));
    let action_ids = (0..effects.len())
        .map(|slot| opaque_action_id_v1(&seed, slot as u64).expect("opaque action ID"))
        .collect::<Vec<_>>();
    fixture_with_ids_v1(label, action_ids, effects, initial_files, goal_files)
}

fn action_id_for_effect_v1(
    task: &K2RepresentationTaskV1,
    effect: &K2CompositionLearnedEffectV1,
) -> String {
    task.laws
        .iter()
        .find(|law| &law.effect == effect)
        .expect("action effect")
        .action_id_sha256
        .clone()
}

fn fixture_with_ids_v1(
    label: &str,
    action_ids: Vec<String>,
    effects: Vec<K2CompositionLearnedEffectV1>,
    initial_files: BTreeMap<String, Vec<u8>>,
    goal_files: BTreeMap<String, Vec<u8>>,
) -> FixtureV1 {
    assert_eq!(action_ids.len(), K2_REPRESENTATION_ACTIONS_V1);
    assert_eq!(effects.len(), K2_REPRESENTATION_ACTIONS_V1);
    let experiment_id = root(&format!("k2-hidden-representation-{label}-experiment-v1"));
    let laws = action_ids
        .into_iter()
        .zip(effects)
        .enumerate()
        .map(|(slot, (action_id, effect))| {
            K2RepresentationActionLawV1::seal(
                action_id,
                effect,
                root(&format!(
                    "k2-hidden-representation-{label}-support-{slot}-v1"
                )),
            )
            .expect("representation action law")
        })
        .collect::<Vec<_>>();
    let initial =
        K2CompositionTreeManifestV1::from_files(&initial_files).expect("initial manifest");
    let goal = K2CompositionExactGoalV1::seal(
        K2CompositionTreeManifestV1::from_files(&goal_files).expect("goal manifest"),
    )
    .expect("exact goal");
    let task = K2RepresentationTaskV1::seal(experiment_id, laws, initial, goal)
        .expect("representation task");
    FixtureV1 {
        task,
        initial_files,
        goal_files,
    }
}

fn run_controls_v1(
    trainer_request: &K2RepresentationTrainerRequestV1,
    model: &K2MeaningPolicySnapshotV1,
    confirm: &[FixtureV1],
    baselines: &[K2RepresentationBaselineOutcomeV1],
    policy_requests: &[K2RepresentationPolicyRequestV1],
    policies: &[K2RepresentationPolicyOutcomeV1],
    binaries: &ProcessBinariesV1,
) -> Vec<K2CompositionControlResultV1> {
    let mut controls = Vec::new();
    let zero = mutate_model_v1(model, "zero_parameters", |snapshot| {
        snapshot
            .encoder_weights
            .iter_mut()
            .flatten()
            .for_each(|weight| *weight = 0);
        snapshot.output_weights.fill(0);
    });
    controls.push(control_v1(
        1,
        "zero_parameters",
        exact_count_v1(&zero, confirm, binaries, 3) == 0,
    ));

    let permuted = retrain_with_permuted_labels_control_v1(trainer_request)
        .expect("permuted-label control model");
    controls.push(control_v1(
        2,
        "permuted_train_labels",
        exact_count_v1(&permuted, confirm, binaries, 3) <= 1,
    ));

    let hidden_rows = mutate_model_v1(model, "hidden_row_permutation", |snapshot| {
        snapshot.encoder_weights.rotate_left(1);
    });
    controls.push(control_v1(
        3,
        "hidden_row_permutation",
        exact_count_v1(&hidden_rows, confirm, binaries, 3) <= 1,
    ));

    let output_weights = mutate_model_v1(model, "output_weight_permutation", |snapshot| {
        snapshot.output_weights.rotate_left(1);
    });
    controls.push(control_v1(
        4,
        "output_weight_permutation",
        exact_count_v1(&output_weights, confirm, binaries, 3) <= 1,
    ));

    let goal_mask = mutate_model_v1(model, "goal_feature_mask", |snapshot| {
        mask_columns_v1(snapshot, &[4, 5, 9, 10, 12]);
    });
    controls.push(control_v1(
        5,
        "goal_feature_mask",
        exact_count_v1(&goal_mask, confirm, binaries, 3) <= 1,
    ));

    let state_mask = mutate_model_v1(model, "state_feature_mask", |snapshot| {
        mask_columns_v1(snapshot, &[1, 5, 6, 8]);
    });
    controls.push(control_v1(
        6,
        "state_feature_mask",
        exact_count_v1(&state_mask, confirm, binaries, 3) <= 1,
    ));

    let initial = initial_hidden_representation_control_v1(trainer_request)
        .expect("frozen initialization control model");
    controls.push(control_v1(
        7,
        "frozen_initialization",
        exact_count_v1(&initial, confirm, binaries, 3) <= 1,
    ));
    controls.push(control_v1(
        8,
        "beam_width_one",
        exact_count_v1(model, confirm, binaries, 1) <= 2,
    ));

    let mut trainer_value = serde_json::to_value(trainer_request).expect("trainer JSON");
    trainer_value
        .as_object_mut()
        .expect("trainer object")
        .insert(
            "confirm_expected_program".to_owned(),
            serde_json::json!(["forbidden"]),
        );
    let trainer_injection_rejected = representation_decode_v1::<K2RepresentationTrainerRequestV1>(
        &serde_json::to_vec(&trainer_value).expect("trainer injected bytes"),
    )
    .is_err();
    controls.push(control_v1(
        9,
        "trainer_confirm_program_schema_reject",
        trainer_injection_rejected,
    ));

    let mut policy_value = serde_json::to_value(&policy_requests[0]).expect("policy JSON");
    policy_value.as_object_mut().expect("policy object").insert(
        "complete_baseline_output".to_owned(),
        serde_json::json!({"forbidden": true}),
    );
    let policy_injection_rejected = representation_decode_v1::<K2RepresentationPolicyRequestV1>(
        &serde_json::to_vec(&policy_value).expect("policy injected bytes"),
    )
    .is_err();
    controls.push(control_v1(
        10,
        "policy_baseline_schema_reject",
        policy_injection_rejected,
    ));

    let mut score_tamper = policies[0].clone();
    score_tamper.trace[0].action_score = score_tamper.trace[0].action_score.saturating_add(1);
    score_tamper.reseal().expect("score tamper reseal");
    controls.push(control_v1(
        11,
        "tampered_policy_score_rejected",
        verifier_rejects_v1(
            policy_requests[0].clone(),
            score_tamper,
            baselines[0].clone(),
            binaries,
        ),
    ));

    let mut dropped_expansion = policies[0].clone();
    dropped_expansion.trace.remove(0);
    dropped_expansion
        .reseal()
        .expect("dropped expansion reseal");
    controls.push(control_v1(
        12,
        "dropped_policy_expansion_rejected",
        verifier_rejects_v1(
            policy_requests[0].clone(),
            dropped_expansion,
            baselines[0].clone(),
            binaries,
        ),
    ));

    let mut exceeded = policies[0].clone();
    exceeded.action_evaluations = K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1 + 1;
    exceeded.reseal().expect("budget tamper reseal");
    controls.push(control_v1(
        13,
        "evaluation_budget_exceeded_rejected",
        verifier_rejects_v1(
            policy_requests[0].clone(),
            exceeded,
            baselines[0].clone(),
            binaries,
        ),
    ));

    let mut denominator = baselines[0].clone();
    denominator.complete_programs -= 1;
    denominator.reseal().expect("denominator tamper reseal");
    controls.push(control_v1(
        14,
        "tampered_denominator_rejected",
        verifier_rejects_v1(
            policy_requests[0].clone(),
            policies[0].clone(),
            denominator,
            binaries,
        ),
    ));

    controls.push(control_v1(
        15,
        "cross_task_replay_rejected",
        verifier_rejects_v1(
            policy_requests[1].clone(),
            policies[0].clone(),
            baselines[1].clone(),
            binaries,
        ),
    ));

    let id_permuted = confirm.iter().map(permute_ids_v1).collect::<Vec<_>>();
    controls.push(control_v1(
        16,
        "opaque_action_id_permutation",
        exact_count_v1(model, &id_permuted, binaries, 3) == 2,
    ));
    let path_renamed = confirm.iter().map(rename_paths_v1).collect::<Vec<_>>();
    controls.push(control_v1(
        17,
        "bijective_path_rename",
        exact_count_v1(model, &path_renamed, binaries, 3) == 2,
    ));

    let mut authority_task = confirm[0].task.clone();
    authority_task.authority.natural_k2_authority = true;
    let mut authority_model = model.clone();
    authority_model.authority.product_authority = true;
    let mut authority_trainer = trainer_request.clone();
    authority_trainer.authority.law_certificate_issued = true;
    let mut authority_baseline = K2RepresentationBaselineRequestV1::seal(
        binaries.baseline_sha256.clone(),
        confirm[0].task.clone(),
    )
    .expect("authority baseline request");
    authority_baseline.authority.package_activated = true;
    let mut authority_policy = policy_requests[0].clone();
    authority_policy.authority.phase_memory_mutated = true;
    let mut authority_verifier = K2RepresentationVerifierRequestV1::seal(
        binaries.verifier_sha256.clone(),
        policy_requests[0].clone(),
        policies[0].clone(),
        baselines[0].clone(),
    )
    .expect("authority verifier request");
    authority_verifier.authority.deployment_authority = true;
    let mut authority_sandbox = K2RepresentationSandboxRequestV1::seal(
        confirm[0].task.experiment_id_sha256.clone(),
        binaries.worker_sha256.clone(),
        confirm[0].task.initial.clone(),
        vec![confirm[0].task.laws[0].effect.clone()],
    )
    .expect("authority sandbox request");
    authority_sandbox.authority.product_authority = true;
    let mut authority_oracle = K2CompositionOracleRequestV1::seal(
        confirm[0].task.experiment_id_sha256.clone(),
        confirm[0].task.initial.clone(),
        confirm[0].task.goal.clone(),
    )
    .expect("authority oracle request");
    authority_oracle.authority.natural_k2_authority = true;
    let authority_rejected = authority_task.validate().is_err()
        && authority_model.validate().is_err()
        && train_hidden_representation_v1(&authority_trainer).is_err()
        && complete_representation_baseline_v1(&authority_baseline).is_err()
        && run_hidden_representation_policy_v1(&authority_policy).is_err()
        && verify_hidden_representation_v1(&authority_verifier).is_err()
        && authority_sandbox.validate().is_err()
        && evaluate_exact_composition_goal_v1(&authority_oracle).is_err();
    controls.push(control_v1(
        18,
        "authority_promotion_rejected",
        authority_rejected,
    ));
    controls
}

fn policy_forensic_v1(
    index: usize,
    fixture: &FixtureV1,
    baseline: &K2RepresentationBaselineOutcomeV1,
    outcome: &K2RepresentationPolicyOutcomeV1,
) -> serde_json::Value {
    let final_programs = outcome
        .layers
        .last()
        .map(|layer| {
            layer
                .retained_program_roots_sha256
                .iter()
                .map(|program_root| program_forensic_v1(fixture, outcome, program_root))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let minimum_programs = baseline
        .minimum_satisfying_programs
        .iter()
        .take(12)
        .map(|program| {
            program
                .action_ids_sha256
                .iter()
                .map(|action_id| effect_label_v1(fixture, action_id))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "schema": "nando.k2-hidden-representation-policy-forensic.v1",
        "confirm_index": index,
        "task_root_sha256": &fixture.task.task_root_sha256,
        "exact_goal_satisfied": outcome.exact_goal_satisfied,
        "action_evaluations": outcome.action_evaluations,
        "exact_score_ties": outcome.exact_score_ties,
        "minimum_satisfying_program_count": baseline.minimum_satisfying_programs.len(),
        "minimum_satisfying_programs_first_12": minimum_programs,
        "final_beam": final_programs,
        "authority": false
    })
}

fn program_forensic_v1(
    fixture: &FixtureV1,
    outcome: &K2RepresentationPolicyOutcomeV1,
    final_root: &str,
) -> serde_json::Value {
    let mut current = Some(final_root.to_owned());
    let mut actions = Vec::new();
    let mut scores = Vec::new();
    while let Some(root) = current {
        let trace = outcome
            .trace
            .iter()
            .find(|trace| trace.resulting_program_root_sha256 == root)
            .expect("forensic trace root");
        actions.push(effect_label_v1(fixture, &trace.action_id_sha256));
        scores.push(trace.action_score);
        current = trace.prefix_program_root_sha256.clone();
    }
    actions.reverse();
    scores.reverse();
    let cumulative_score = scores.iter().sum::<i64>();
    serde_json::json!({
        "program_root_sha256": final_root,
        "actions": actions,
        "action_scores": scores,
        "cumulative_score": cumulative_score
    })
}

fn effect_label_v1(fixture: &FixtureV1, action_id: &str) -> String {
    match &fixture.task.law(action_id).expect("forensic law").effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => format!("copy:{source_path}->{target_path}"),
        K2CompositionLearnedEffectV1::RemoveFile { path } => format!("remove:{path}"),
    }
}

fn exact_count_v1(
    model: &K2MeaningPolicySnapshotV1,
    fixtures: &[FixtureV1],
    binaries: &ProcessBinariesV1,
    beam_width: u64,
) -> usize {
    fixtures
        .iter()
        .filter(|fixture| {
            let request = K2RepresentationPolicyRequestV1::seal_with_budget(
                binaries.policy_sha256.clone(),
                model.clone(),
                fixture.task.clone(),
                beam_width,
                K2_REPRESENTATION_MAX_ACTION_EVALUATIONS_V1,
            )
            .expect("control policy request");
            run_hidden_representation_policy_v1(&request)
                .expect("control policy")
                .exact_goal_satisfied
        })
        .count()
}

fn mutate_model_v1(
    model: &K2MeaningPolicySnapshotV1,
    variant: &str,
    mutation: impl FnOnce(&mut K2MeaningPolicySnapshotV1),
) -> K2MeaningPolicySnapshotV1 {
    let mut value = model.clone();
    mutation(&mut value);
    value.control_variant = variant.to_owned();
    value.reseal().expect("control model reseal");
    value.validate().expect("control model validate");
    value
}

fn mask_columns_v1(model: &mut K2MeaningPolicySnapshotV1, columns: &[usize]) {
    for row in &mut model.encoder_weights {
        for column in columns {
            row[*column] = 0;
        }
    }
}

fn verifier_rejects_v1(
    policy_request: K2RepresentationPolicyRequestV1,
    policy_outcome: K2RepresentationPolicyOutcomeV1,
    baseline: K2RepresentationBaselineOutcomeV1,
    binaries: &ProcessBinariesV1,
) -> bool {
    let request = K2RepresentationVerifierRequestV1::seal(
        binaries.verifier_sha256.clone(),
        policy_request,
        policy_outcome,
        baseline,
    )
    .expect("tampered verifier request");
    verify_hidden_representation_v1(&request).is_err()
}

fn permute_ids_v1(fixture: &FixtureV1) -> FixtureV1 {
    let ids = fixture
        .task
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let mut effects = fixture
        .task
        .laws
        .iter()
        .map(|law| law.effect.clone())
        .collect::<Vec<_>>();
    effects.rotate_left(3);
    rebuild_fixture_v1(fixture, "id-permutation", ids, effects, |path| {
        path.to_owned()
    })
}

fn rename_paths_v1(fixture: &FixtureV1) -> FixtureV1 {
    let ids = fixture
        .task
        .laws
        .iter()
        .map(|law| law.action_id_sha256.clone())
        .collect::<Vec<_>>();
    let effects = fixture
        .task
        .laws
        .iter()
        .map(|law| rename_effect_v1(&law.effect, |path| format!("renamed/{path}")))
        .collect::<Vec<_>>();
    rebuild_fixture_v1(fixture, "path-rename", ids, effects, |path| {
        format!("renamed/{path}")
    })
}

fn rebuild_fixture_v1(
    fixture: &FixtureV1,
    label: &str,
    ids: Vec<String>,
    effects: Vec<K2CompositionLearnedEffectV1>,
    rename: impl Fn(&str) -> String,
) -> FixtureV1 {
    let initial = fixture
        .initial_files
        .iter()
        .map(|(path, bytes)| (rename(path), bytes.clone()))
        .collect::<BTreeMap<_, _>>();
    let goal = fixture
        .goal_files
        .iter()
        .map(|(path, bytes)| (rename(path), bytes.clone()))
        .collect::<BTreeMap<_, _>>();
    fixture_with_ids_v1(
        &format!("{}-{label}", &fixture.task.experiment_id_sha256[..12]),
        ids,
        effects,
        initial,
        goal,
    )
}

fn rename_effect_v1(
    effect: &K2CompositionLearnedEffectV1,
    rename: impl Fn(&str) -> String,
) -> K2CompositionLearnedEffectV1 {
    match effect {
        K2CompositionLearnedEffectV1::CopyFile {
            source_path,
            target_path,
        } => copy(&rename(source_path), &rename(target_path)),
        K2CompositionLearnedEffectV1::RemoveFile { path } => remove(&rename(path)),
    }
}

fn control_v1(id: u64, name: &str, passed: bool) -> K2CompositionControlResultV1 {
    K2CompositionControlResultV1 {
        control_id: id,
        name: name.to_owned(),
        expected_verdict: "PASS".to_owned(),
        observed_verdict: if passed { "PASS" } else { "FAIL" }.to_owned(),
        passed,
    }
}

fn run_baseline_process_v1(
    binaries: &ProcessBinariesV1,
    task: &K2RepresentationTaskV1,
) -> K2RepresentationBaselineOutcomeV1 {
    let request =
        K2RepresentationBaselineRequestV1::seal(binaries.baseline_sha256.clone(), task.clone())
            .expect("baseline request");
    run_isolated_protocol_v1(&binaries.baseline, &request)
}

fn append_and_reopen_v1(
    journal: &mut K2RepresentationJournalV1,
    environment: &TestEnvironmentV1,
    experiment_id: &str,
    kind: K2RepresentationJournalEventKindV1,
    payload_root: String,
) {
    journal.append(kind, payload_root).expect("journal append");
    let reopened = K2RepresentationJournalV1::open_existing(
        &environment.journal_store,
        experiment_id.to_owned(),
    )
    .expect("journal prefix reopen");
    assert_eq!(reopened.projection(), journal.projection());
    match kind {
        K2RepresentationJournalEventKindV1::ModelTrainingDispatched => {
            assert!(reopened.projection().indeterminate_model_dispatch);
        }
        K2RepresentationJournalEventKindV1::PoliciesDispatched => {
            assert!(reopened.projection().indeterminate_policy_dispatch);
        }
        K2RepresentationJournalEventKindV1::Execution1Dispatched
        | K2RepresentationJournalEventKindV1::Execution2Dispatched => {
            assert!(reopened.projection().indeterminate_execution_dispatch);
        }
        _ => {}
    }
}

fn assert_journal_fault_contract_v1(environment: &TestEnvironmentV1) {
    let before_id = root("k2-hidden-representation-journal-before-rename-v1");
    let mut before =
        K2RepresentationJournalV1::create(&environment.journal_store, before_id.clone())
            .expect("before-rename journal");
    assert!(
        before
            .append_with_fault(
                K2RepresentationJournalEventKindV1::ExperimentFrozen,
                root("k2-hidden-representation-before-rename-payload-v1"),
                K2RepresentationJournalFaultV1::BeforeRename,
            )
            .is_err()
    );
    let reopened_before =
        K2RepresentationJournalV1::open_existing(&environment.journal_store, before_id)
            .expect("reopen before-rename journal");
    assert_eq!(reopened_before.projection().event_count, 0);
    before.cleanup().expect("before-rename cleanup");

    let after_id = root("k2-hidden-representation-journal-after-rename-v1");
    let mut after = K2RepresentationJournalV1::create(&environment.journal_store, after_id.clone())
        .expect("after-rename journal");
    assert!(
        after
            .append_with_fault(
                K2RepresentationJournalEventKindV1::ExperimentFrozen,
                root("k2-hidden-representation-after-rename-payload-v1"),
                K2RepresentationJournalFaultV1::AfterRename,
            )
            .is_err()
    );
    let reopened_after =
        K2RepresentationJournalV1::open_existing(&environment.journal_store, after_id)
            .expect("reopen after-rename journal");
    assert_eq!(reopened_after.projection().event_count, 1);
    reopened_after.cleanup().expect("after-rename cleanup");
}

fn root_set_v1<'a>(label: &str, roots: impl IntoIterator<Item = &'a String>) -> String {
    let mut values = roots.into_iter().cloned().collect::<Vec<_>>();
    values.sort();
    composition_root_v1(&(label, values)).expect("root set")
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
    composition_root_v1(&("nando.k2-hidden-representation-test-root.v1", label)).expect("test root")
}

struct ProcessBinariesV1 {
    baseline: PathBuf,
    trainer: PathBuf,
    policy: PathBuf,
    verifier: PathBuf,
    worker: PathBuf,
    oracle: PathBuf,
    baseline_sha256: String,
    trainer_sha256: String,
    policy_sha256: String,
    verifier_sha256: String,
    worker_sha256: String,
}

impl ProcessBinariesV1 {
    fn from_cargo() -> Self {
        let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-representation-baseline"));
        let trainer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-representation-trainer"));
        let policy = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-representation-policy"));
        let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-representation-verifier"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-representation-sandbox-worker"));
        let oracle = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-composition-exact-oracle"));
        Self {
            baseline_sha256: composition_sha256_file_v1(&baseline).expect("baseline sha"),
            trainer_sha256: composition_sha256_file_v1(&trainer).expect("trainer sha"),
            policy_sha256: composition_sha256_file_v1(&policy).expect("policy sha"),
            verifier_sha256: composition_sha256_file_v1(&verifier).expect("verifier sha"),
            worker_sha256: composition_sha256_file_v1(&worker).expect("worker sha"),
            baseline,
            trainer,
            policy,
            verifier,
            worker,
            oracle,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let orchestrator = std::env::current_exe().expect("orchestrator executable");
        let roots = [
            composition_sha256_file_v1(&self.baseline).expect("baseline root"),
            composition_sha256_file_v1(&self.trainer).expect("trainer root"),
            composition_sha256_file_v1(&self.policy).expect("policy root"),
            composition_sha256_file_v1(&self.verifier).expect("verifier root"),
            composition_sha256_file_v1(&self.worker).expect("worker root"),
            composition_sha256_file_v1(&self.oracle).expect("oracle root"),
            composition_sha256_file_v1(&orchestrator).expect("orchestrator root"),
        ];
        assert_eq!(roots.iter().collect::<BTreeSet<_>>().len(), roots.len());
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
        .args(["--", "/usr/bin/prlimit", "--cpu=20:20"])
        .args(["--as=536870912:536870912", "--nproc=32:32"])
        .args(["--fsize=33554432:33554432", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn isolated protocol");
    child
        .stdin
        .take()
        .expect("protocol stdin")
        .write_all(&representation_bytes_v1(request).expect("protocol request bytes"))
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
            started.elapsed() < Duration::from_secs(30),
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
    representation_decode_v1(&stdout).expect("protocol outcome")
}

struct TestEnvironmentV1 {
    root: PathBuf,
    workspace_store: PathBuf,
    journal_store: PathBuf,
    cleaned: bool,
}

impl TestEnvironmentV1 {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-hidden-representation-{}-{sequence}",
            std::process::id()
        ));
        let workspace_store = root.join("workspaces");
        let journal_store = root.join("journals");
        fs::create_dir_all(&workspace_store).expect("workspace store");
        fs::create_dir_all(&journal_store).expect("journal store");
        Self {
            root,
            workspace_store,
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
