use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::*;
use serde::Serialize;

const CONFIRM_COMMITMENT_V1: &str =
    "0a48670dbb2035c0502f064ee10c41c20b5c6391743641b814af98892efba6f4";
const DEVELOPMENT_COMMITMENT_V1: &str =
    "2fbfa252f13d5191024a9ae5d53eae293bd39ab458445808d2414638840a53e7";
const GENERATOR_SCHEMA_ROOT_V1: &str =
    "ad591e3c1a7826295ea93056049dd3759f37c6502b86a542e27dd67fb68a0286";

static TEST_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

#[test]
fn development_active_inquiry_contract_is_complete_and_fail_closed() {
    let fixture = build_fixture_v1(0, DEVELOPMENT_COMMITMENT_V1, CaseVariantV1::default());
    let artifacts = direct_artifacts_v1(&fixture);
    assert_core_contract_v1(&fixture, &artifacts);

    let environment = TestEnvironmentV1::new("development-controls");
    let controls = assert_negative_controls_v1(&fixture, &artifacts, &environment);
    assert_eq!(controls.len(), 18);
    assert_generated_provenance_control_v1(&fixture);
    assert!(directory_is_empty_v1(&environment.journal_store));
}

#[test]
#[ignore = "exactly one sealed run; requires Linux bwrap and five isolated inquiry binaries"]
fn sealed_model_guided_active_inquiry_uses_one_safe_probe_per_case() {
    let binaries = ProcessBinariesV1::from_cargo();
    binaries.assert_pairwise_distinct();
    let environment = TestEnvironmentV1::new("sealed-confirm");
    let fixtures = (0..K2_INQUIRY_CONFIRM_CASES_V1)
        .map(|case_index| {
            build_fixture_v1(case_index, CONFIRM_COMMITMENT_V1, CaseVariantV1::default())
        })
        .collect::<Vec<_>>();
    assert_confirm_disjointness_v1(&fixtures);

    let experiment_id_sha256 = root_v1(&(
        "sealed-experiment",
        fixtures
            .iter()
            .map(|fixture| &fixture.public_case.case_root_sha256)
            .collect::<Vec<_>>(),
    ));
    let mut journal =
        K2InquiryJournalV1::create(&environment.journal_store, experiment_id_sha256.clone())
            .expect("create sealed inquiry journal");
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ExperimentFrozen,
        root_set_v1(
            "sealed-public-cases",
            fixtures
                .iter()
                .map(|fixture| fixture.public_case.case_root_sha256.clone()),
        ),
    );

    let baseline_requests = fixtures
        .iter()
        .map(|fixture| {
            K2InquiryBaselineRequestV1::seal(
                binaries.baseline_sha256.clone(),
                fixture.public_case.clone(),
            )
            .expect("seal baseline request")
        })
        .collect::<Vec<_>>();
    let baselines = baseline_requests
        .iter()
        .map(|request| run_isolated_protocol_v1(&binaries.baseline, request))
        .collect::<Vec<K2InquiryBaselinesV1>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::BaselinesFrozen,
        root_set_v1(
            "sealed-baselines",
            baselines
                .iter()
                .map(|value| value.baselines_root_sha256.clone()),
        ),
    );

    let selector_requests = fixtures
        .iter()
        .map(|fixture| {
            K2InquirySelectorRequestV1::seal(
                binaries.selector_sha256.clone(),
                fixture.public_case.clone(),
            )
            .expect("seal selector request")
        })
        .collect::<Vec<_>>();
    assert_choice_invariant_public_inputs_v1(
        &fixtures[0],
        &selector_requests[0],
        &baseline_requests[0],
    );
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionDispatched,
        root_set_v1(
            "sealed-selector-dispatches",
            selector_requests
                .iter()
                .map(|request| request.request_root_sha256.clone()),
        ),
    );
    let precommits = selector_requests
        .iter()
        .map(|request| run_isolated_protocol_v1(&binaries.selector, request))
        .collect::<Vec<K2InquirySelectionPrecommitV1>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionPrecommitted,
        root_set_v1(
            "sealed-selection-precommits",
            precommits
                .iter()
                .map(|value| value.precommit_root_sha256.clone()),
        ),
    );

    let selection_verifications = selector_requests
        .iter()
        .zip(&precommits)
        .map(|(selector_request, precommit)| {
            let command = K2InquiryVerifierCommandV1::VerifySelection {
                verifier_executable_sha256: binaries.verifier_sha256.clone(),
                selector_request: selector_request.clone(),
                precommit: precommit.clone(),
            };
            match run_isolated_protocol_v1(&binaries.verifier, &command) {
                K2InquiryVerifierReceiptV1::Selection { value } => value,
                K2InquiryVerifierReceiptV1::Outcome { .. } => {
                    panic!("selection verifier returned outcome receipt")
                }
            }
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::SelectionVerified,
        root_set_v1(
            "sealed-selection-verifications",
            selection_verifications
                .iter()
                .map(|value| value.receipt_root_sha256.clone()),
        ),
    );

    let worker_requests = fixtures
        .iter()
        .zip(&precommits)
        .zip(&selection_verifications)
        .map(|((fixture, precommit), selection)| {
            let probe = fixture
                .public_case
                .probe(&precommit.selected_probe_root_sha256)
                .expect("selected probe exists");
            let true_model = fixture
                .public_case
                .model(&fixture.true_model_root_sha256)
                .expect("true model exists");
            let resolved_effect = true_model
                .effect(&probe.action_id_sha256)
                .expect("selected action known")
                .clone();
            K2InquiryWorkerRequestV1::seal(
                fixture.public_case.experiment_id_sha256.clone(),
                selection.receipt_root_sha256.clone(),
                probe.probe_root_sha256.clone(),
                probe.action_id_sha256.clone(),
                binaries.worker_sha256.clone(),
                probe.initial_manifest.clone(),
                resolved_effect,
            )
            .expect("seal worker request")
        })
        .collect::<Vec<_>>();
    let observer_requests = fixtures
        .iter()
        .zip(&precommits)
        .map(|(fixture, precommit)| {
            K2InquiryObserverRequestV1::seal(
                fixture.public_case.experiment_id_sha256.clone(),
                precommit.selected_probe_root_sha256.clone(),
                binaries.observer_sha256.clone(),
            )
            .expect("seal observer request")
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ProbeDispatched,
        root_set_v1(
            "sealed-probe-dispatches",
            worker_requests
                .iter()
                .map(|request| request.request_root_sha256.clone()),
        ),
    );

    let sandbox = K2InquirySandboxAdapterV1::new(
        binaries.worker.clone(),
        binaries.worker_sha256.clone(),
        binaries.observer.clone(),
        binaries.observer_sha256.clone(),
        environment.workspace_store.clone(),
    )
    .expect("create inquiry sandbox");
    let executions = worker_requests
        .iter()
        .zip(&observer_requests)
        .zip(&fixtures)
        .map(|((worker_request, observer_request), fixture)| {
            sandbox
                .execute(worker_request, observer_request, &fixture.initial_files)
                .expect("execute one isolated inquiry probe")
        })
        .collect::<Vec<_>>();
    assert!(
        executions.iter().all(|execution| {
            execution.source_integrity_preserved && execution.workspace_removed
        })
    );
    assert!(directory_is_empty_v1(&environment.workspace_store));
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ProbeObserved,
        root_set_v1(
            "sealed-observations",
            executions
                .iter()
                .map(|execution| execution.observation.receipt_root_sha256.clone()),
        ),
    );

    let outcome_requests = (0..fixtures.len())
        .map(|index| {
            K2InquiryOutcomeVerificationRequestV1::seal(
                binaries.verifier_sha256.clone(),
                selector_requests[index].clone(),
                precommits[index].clone(),
                selection_verifications[index].clone(),
                baseline_requests[index].clone(),
                baselines[index].clone(),
                executions[index].observation.clone(),
                fixtures[index].true_model_root_sha256.clone(),
            )
            .expect("seal outcome verification request")
        })
        .collect::<Vec<_>>();
    let outcomes = outcome_requests
        .iter()
        .map(|request| {
            let command = K2InquiryVerifierCommandV1::VerifyOutcome {
                request: Box::new(request.clone()),
            };
            match run_isolated_protocol_v1(&binaries.verifier, &command) {
                K2InquiryVerifierReceiptV1::Outcome { value } => value,
                K2InquiryVerifierReceiptV1::Selection { .. } => {
                    panic!("outcome verifier returned selection receipt")
                }
            }
        })
        .collect::<Vec<_>>();
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ModelsUpdated,
        root_set_v1(
            "sealed-model-updates",
            outcomes
                .iter()
                .map(|outcome| outcome.receipt_root_sha256.clone()),
        ),
    );

    let first_artifacts = InquiryArtifactsV1 {
        selector_request: selector_requests[0].clone(),
        precommit: precommits[0].clone(),
        baseline_request: baseline_requests[0].clone(),
        baselines: baselines[0].clone(),
        selection_verification: selection_verifications[0].clone(),
        observation: executions[0].observation.clone(),
        outcome_request: outcome_requests[0].clone(),
        outcome_receipt: outcomes[0].clone(),
    };
    let controls = assert_negative_controls_v1(&fixtures[0], &first_artifacts, &environment);
    assert_eq!(controls.len(), 18);
    assert_generated_provenance_control_v1(&fixtures[0]);
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::ControlsFrozen,
        root_set_v1("sealed-controls", controls.iter().cloned()),
    );

    let totals = InquiryTotalsV1::from_results(&precommits, &outcomes);
    totals.assert_pass();
    let terminal_payload_root_sha256 =
        composition_root_v1(&(K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS_V1, &totals, &controls))
            .expect("terminal payload root");
    append_and_reopen_v1(
        &mut journal,
        &environment.journal_store,
        &experiment_id_sha256,
        K2InquiryJournalEventKindV1::TerminalFrozen,
        terminal_payload_root_sha256,
    );
    let projection = journal.projection();
    assert!(projection.terminal);
    let terminal_event_root_sha256 = projection
        .last_event_root_sha256
        .clone()
        .expect("terminal event root");

    let mut receipt = SealedInquiryReceiptV1 {
        schema: "nando.k2-self-chosen-safe-inquiry-receipt.v1".to_owned(),
        disposition: K2_MODEL_GUIDED_ACTIVE_INQUIRY_PASS_V1.to_owned(),
        confirm_commitment_sha256: CONFIRM_COMMITMENT_V1.to_owned(),
        generator_schema_sha256: GENERATOR_SCHEMA_ROOT_V1.to_owned(),
        cases: fixtures.len() as u64,
        totals,
        negative_controls_passed: controls.len() as u64,
        forbidden_probe_executions: 0,
        terminal_event_root_sha256,
        selector_executable_sha256: binaries.selector_sha256.clone(),
        baseline_executable_sha256: binaries.baseline_sha256.clone(),
        verifier_executable_sha256: binaries.verifier_sha256.clone(),
        worker_executable_sha256: binaries.worker_sha256.clone(),
        observer_executable_sha256: binaries.observer_sha256.clone(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 = composition_root_v1(&(
        &receipt.schema,
        &receipt.disposition,
        &receipt.confirm_commitment_sha256,
        &receipt.generator_schema_sha256,
        receipt.cases,
        &receipt.totals,
        receipt.negative_controls_passed,
        receipt.forbidden_probe_executions,
        &receipt.terminal_event_root_sha256,
        &receipt.selector_executable_sha256,
        &receipt.baseline_executable_sha256,
        &receipt.verifier_executable_sha256,
        &receipt.worker_executable_sha256,
        &receipt.observer_executable_sha256,
        &receipt.authority,
    ))
    .expect("receipt root");

    journal.cleanup().expect("cleanup sealed journal");
    assert!(directory_is_empty_v1(&environment.journal_store));
    assert!(directory_is_empty_v1(&environment.workspace_store));
    println!(
        "NANDO_K2_INQUIRY_SEALED_RESULT={}",
        String::from_utf8(composition_bytes_v1(&receipt).expect("receipt bytes"))
            .expect("receipt utf8")
    );
}

#[derive(Clone, Copy, Default)]
struct CaseVariantV1 {
    action_permutation: u64,
    path_bijection: u64,
    reverse_candidates: bool,
    collapse_optimal_predictions: bool,
    rotate_model_effect_bindings: bool,
}

#[derive(Clone)]
struct ProbeRootsV1 {
    optimal: String,
    stable: String,
    cheapest: String,
    heuristic: String,
    unsafe_high_information: String,
    ambiguous: String,
    delayed: String,
    unknown: String,
}

struct InquiryFixtureV1 {
    case_index: usize,
    split_commitment_root_sha256: String,
    public_case: K2InquiryPublicCaseV1,
    initial_files: BTreeMap<String, Vec<u8>>,
    true_model_id_sha256: String,
    true_model_root_sha256: String,
    roles: ProbeRootsV1,
}

struct InquiryArtifactsV1 {
    selector_request: K2InquirySelectorRequestV1,
    precommit: K2InquirySelectionPrecommitV1,
    baseline_request: K2InquiryBaselineRequestV1,
    baselines: K2InquiryBaselinesV1,
    selection_verification: K2InquirySelectionVerificationReceiptV1,
    observation: K2InquiryObservationReceiptV1,
    outcome_request: K2InquiryOutcomeVerificationRequestV1,
    outcome_receipt: K2InquiryOutcomeVerificationReceiptV1,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InquiryTotalsV1 {
    model_guided_survivors: u64,
    passive_survivors: u64,
    stable_hash_survivors: u64,
    cheapest_first_survivors: u64,
    explicit_heuristic_survivors: u64,
    oracle_survivors: u64,
    oracle_matches: u64,
    complete_predictions: u64,
    exact_best_ties: u64,
}

impl InquiryTotalsV1 {
    fn from_results(
        precommits: &[K2InquirySelectionPrecommitV1],
        outcomes: &[K2InquiryOutcomeVerificationReceiptV1],
    ) -> Self {
        let mut totals = Self {
            model_guided_survivors: 0,
            passive_survivors: 0,
            stable_hash_survivors: 0,
            cheapest_first_survivors: 0,
            explicit_heuristic_survivors: 0,
            oracle_survivors: 0,
            oracle_matches: 0,
            complete_predictions: 0,
            exact_best_ties: precommits
                .iter()
                .map(|precommit| precommit.exact_best_ties)
                .sum(),
        };
        for outcome in outcomes {
            totals.model_guided_survivors += outcome.surviving_model_roots_sha256.len() as u64;
            totals.oracle_survivors += outcome.oracle_survivors;
            totals.oracle_matches += u64::from(outcome.selector_matches_oracle);
            totals.complete_predictions += outcome.complete_prediction_count;
            for baseline in &outcome.baseline_survivors {
                match baseline.kind {
                    K2InquiryBaselineKindV1::Passive => {
                        totals.passive_survivors += baseline.survivors;
                    }
                    K2InquiryBaselineKindV1::StableHash => {
                        totals.stable_hash_survivors += baseline.survivors;
                    }
                    K2InquiryBaselineKindV1::CheapestFirst => {
                        totals.cheapest_first_survivors += baseline.survivors;
                    }
                    K2InquiryBaselineKindV1::ExplicitHeuristic => {
                        totals.explicit_heuristic_survivors += baseline.survivors;
                    }
                }
            }
        }
        totals
    }

    fn assert_pass(&self) {
        assert_eq!(self.model_guided_survivors, 8);
        assert_eq!(self.passive_survivors, 32);
        assert!(self.stable_hash_survivors > 8);
        assert!(self.cheapest_first_survivors > 8);
        assert!(self.explicit_heuristic_survivors > 8);
        assert_eq!(self.oracle_survivors, 8);
        assert_eq!(self.oracle_matches, 8);
        assert_eq!(self.complete_predictions, 8 * 4 * 8);
        assert_eq!(self.exact_best_ties, 8);
    }
}

#[derive(Serialize)]
struct SealedInquiryReceiptV1 {
    schema: String,
    disposition: String,
    confirm_commitment_sha256: String,
    generator_schema_sha256: String,
    cases: u64,
    totals: InquiryTotalsV1,
    negative_controls_passed: u64,
    forbidden_probe_executions: u64,
    terminal_event_root_sha256: String,
    selector_executable_sha256: String,
    baseline_executable_sha256: String,
    verifier_executable_sha256: String,
    worker_executable_sha256: String,
    observer_executable_sha256: String,
    authority: K2CompositionAuthorityBoundaryV1,
    receipt_root_sha256: String,
}

fn build_fixture_v1(
    case_index: usize,
    split_commitment_root_sha256: &str,
    variant: CaseVariantV1,
) -> InquiryFixtureV1 {
    let experiment_id_sha256 =
        root_v1(&("case-experiment", split_commitment_root_sha256, case_index));
    let path_prefix = format!(
        "generated/case-{case_index}/paths-{}",
        variant.path_bijection
    );
    let source_paths = (0..4)
        .map(|index| format!("{path_prefix}/source-{index}.txt"))
        .collect::<Vec<_>>();
    let target_path = format!("{path_prefix}/observed-target.txt");
    let mut initial_files = BTreeMap::new();
    for (index, path) in source_paths.iter().enumerate() {
        initial_files.insert(
            path.clone(),
            format!("case={case_index};source={index};payload=v1\n").into_bytes(),
        );
    }
    initial_files.insert(
        target_path.clone(),
        format!("case={case_index};initial-target=v1\n").into_bytes(),
    );
    let initial_manifest =
        K2CompositionTreeManifestV1::from_files(&initial_files).expect("initial manifest");

    let action_id = |role: &str| {
        root_v1(&(
            "opaque-action",
            split_commitment_root_sha256,
            case_index,
            variant.action_permutation,
            role,
        ))
    };
    let optimal_action = action_id("a0");
    let stable_action = action_id("a1");
    let cheapest_action = action_id("a2");
    let heuristic_action = action_id("a3");
    let unsafe_action = action_id("a4");
    let ambiguous_action = action_id("a5");
    let delayed_action = action_id("a6");
    let unknown_action = action_id("a7");

    let distinguishing = |model_index: usize| match model_index {
        0 => copy_v1(&source_paths[0], &target_path),
        1 => copy_v1(&source_paths[1], &target_path),
        2 => remove_v1(&target_path),
        3 => copy_v1(&source_paths[3], &target_path),
        _ => unreachable!(),
    };
    let stable_effect = |model_index: usize| {
        if model_index < 2 {
            copy_v1(&source_paths[0], &target_path)
        } else {
            copy_v1(&source_paths[2], &target_path)
        }
    };
    let heuristic_effect = |model_index: usize| {
        if model_index < 2 {
            remove_v1(&target_path)
        } else {
            copy_v1(&source_paths[3], &target_path)
        }
    };

    let model_ids = (0..4)
        .map(|model_index| {
            root_v1(&(
                "opaque-model",
                split_commitment_root_sha256,
                case_index,
                model_index,
            ))
        })
        .collect::<Vec<_>>();
    let common_evidence_root_sha256 =
        root_v1(&("common-evidence", split_commitment_root_sha256, case_index));
    let models = (0..4)
        .map(|model_index| {
            let optimal_index = if variant.rotate_model_effect_bindings {
                (model_index + 1) % 4
            } else {
                model_index
            };
            let optimal_effect = if variant.collapse_optimal_predictions {
                copy_v1(&source_paths[0], &target_path)
            } else {
                distinguishing(optimal_index)
            };
            let actions = vec![
                K2InquiryModelActionV1 {
                    action_id_sha256: optimal_action.clone(),
                    effect: optimal_effect,
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: stable_action.clone(),
                    effect: stable_effect(model_index),
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: cheapest_action.clone(),
                    effect: copy_v1(&source_paths[1], &target_path),
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: heuristic_action.clone(),
                    effect: heuristic_effect(model_index),
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: unsafe_action.clone(),
                    effect: distinguishing(model_index),
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: ambiguous_action.clone(),
                    effect: distinguishing(model_index),
                },
                K2InquiryModelActionV1 {
                    action_id_sha256: delayed_action.clone(),
                    effect: distinguishing(model_index),
                },
            ];
            K2InquiryWorldModelV1::seal(
                experiment_id_sha256.clone(),
                model_ids[model_index].clone(),
                common_evidence_root_sha256.clone(),
                root_v1(&(
                    "source-neutral-model-provenance",
                    split_commitment_root_sha256,
                    case_index,
                    model_index,
                )),
                actions,
            )
            .expect("seal world model")
        })
        .collect::<Vec<_>>();

    let make_probe = |role: &str,
                      nonce: u64,
                      action_id_sha256: String,
                      reversible: bool,
                      observation_mode: K2InquiryObservationModeV1,
                      risk_units: u64,
                      cost_units: u64,
                      hints: (bool, bool, bool)| {
        let probe_id_sha256 = root_v1(&(
            "opaque-probe",
            split_commitment_root_sha256,
            case_index,
            role,
            nonce,
        ));
        let provenance = inquiry_generated_probe_provenance_root_v1(
            &experiment_id_sha256,
            GENERATOR_SCHEMA_ROOT_V1,
            split_commitment_root_sha256,
            &probe_id_sha256,
            &action_id_sha256,
        )
        .expect("generated probe provenance");
        K2InquiryProbeV1::seal(
            experiment_id_sha256.clone(),
            probe_id_sha256,
            action_id_sha256,
            initial_manifest.clone(),
            reversible,
            observation_mode,
            risk_units,
            cost_units,
            hints.0,
            hints.1,
            hints.2,
            provenance,
        )
        .expect("seal inquiry probe")
    };

    let optimal = make_probe(
        "p0",
        0,
        optimal_action,
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        2,
        5,
        (false, false, false),
    );
    let cheapest = make_probe(
        "p2",
        0,
        cheapest_action,
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        1,
        1,
        (false, false, false),
    );
    let heuristic = make_probe(
        "p3",
        0,
        heuristic_action,
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        2,
        4,
        (true, true, true),
    );
    let unsafe_high_information = make_probe(
        "p4",
        0,
        unsafe_action,
        false,
        K2InquiryObservationModeV1::ExactImmediate,
        1,
        1,
        (false, false, false),
    );
    let ambiguous = make_probe(
        "p5",
        0,
        ambiguous_action,
        true,
        K2InquiryObservationModeV1::Ambiguous,
        1,
        1,
        (false, false, false),
    );
    let delayed = make_probe(
        "p6",
        0,
        delayed_action,
        true,
        K2InquiryObservationModeV1::Delayed,
        1,
        1,
        (false, false, false),
    );
    let unknown = make_probe(
        "p7",
        0,
        unknown_action,
        true,
        K2InquiryObservationModeV1::ExactImmediate,
        1,
        1,
        (false, false, false),
    );
    let other_eligible_min = [&optimal, &cheapest, &heuristic]
        .iter()
        .map(|probe| &probe.probe_root_sha256)
        .min()
        .expect("eligible probe root");
    let stable = (0..100_000)
        .map(|nonce| {
            make_probe(
                "p1",
                nonce,
                stable_action.clone(),
                true,
                K2InquiryObservationModeV1::ExactImmediate,
                3,
                6,
                (false, false, true),
            )
        })
        .find(|probe| &probe.probe_root_sha256 < other_eligible_min)
        .expect("find stable-hash decoy root");

    let roles = ProbeRootsV1 {
        optimal: optimal.probe_root_sha256.clone(),
        stable: stable.probe_root_sha256.clone(),
        cheapest: cheapest.probe_root_sha256.clone(),
        heuristic: heuristic.probe_root_sha256.clone(),
        unsafe_high_information: unsafe_high_information.probe_root_sha256.clone(),
        ambiguous: ambiguous.probe_root_sha256.clone(),
        delayed: delayed.probe_root_sha256.clone(),
        unknown: unknown.probe_root_sha256.clone(),
    };
    let mut probes = vec![
        optimal,
        stable,
        cheapest,
        heuristic,
        unsafe_high_information,
        ambiguous,
        delayed,
        unknown,
    ];
    let probe_count = probes.len();
    probes.rotate_left((case_index * 3 + 1) % probe_count);
    if variant.reverse_candidates {
        probes.reverse();
    }
    let public_case = K2InquiryPublicCaseV1::seal(
        experiment_id_sha256,
        GENERATOR_SCHEMA_ROOT_V1.to_owned(),
        split_commitment_root_sha256.to_owned(),
        models,
        probes,
    )
    .expect("seal public case");
    let true_model_id_sha256 = model_ids[case_index % 4].clone();
    let true_model_root_sha256 = public_case
        .models
        .iter()
        .find(|model| model.model_id_sha256 == true_model_id_sha256)
        .expect("true model by opaque id")
        .model_root_sha256
        .clone();
    InquiryFixtureV1 {
        case_index,
        split_commitment_root_sha256: split_commitment_root_sha256.to_owned(),
        public_case,
        initial_files,
        true_model_id_sha256,
        true_model_root_sha256,
        roles,
    }
}

fn direct_artifacts_v1(fixture: &InquiryFixtureV1) -> InquiryArtifactsV1 {
    let selector_request = K2InquirySelectorRequestV1::seal(
        root_v1(&"development-selector-executable"),
        fixture.public_case.clone(),
    )
    .expect("development selector request");
    let precommit = select_model_guided_probe_v1(&selector_request).expect("development selection");
    let baseline_request = K2InquiryBaselineRequestV1::seal(
        root_v1(&"development-baseline-executable"),
        fixture.public_case.clone(),
    )
    .expect("development baseline request");
    let baselines =
        evaluate_inquiry_baselines_v1(&baseline_request).expect("development baselines");
    let verifier_executable_sha256 = root_v1(&"development-verifier-executable");
    let selection_verification = verify_inquiry_selection_v1(
        verifier_executable_sha256.clone(),
        &selector_request,
        &precommit,
    )
    .expect("development selection verification");
    let selected_evaluation = evaluation_v1(&precommit, &precommit.selected_probe_root_sha256);
    let true_prediction = selected_evaluation
        .predictions
        .iter()
        .find(|prediction| prediction.model_root_sha256 == fixture.true_model_root_sha256)
        .expect("true-model prediction");
    let observer_request = K2InquiryObserverRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        precommit.selected_probe_root_sha256.clone(),
        root_v1(&"development-observer-executable"),
    )
    .expect("development observer request");
    let mut observation = K2InquiryObservationReceiptV1 {
        schema: K2_INQUIRY_OBSERVATION_SCHEMA_V1.to_owned(),
        observer_request_root_sha256: observer_request.request_root_sha256,
        observer_executable_sha256: observer_request.observer_executable_sha256,
        selected_probe_root_sha256: precommit.selected_probe_root_sha256.clone(),
        post_manifest: true_prediction.predicted_post_manifest.clone(),
        observable_outcome_root_sha256: String::new(),
        authority: K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    observation.reseal().expect("development observation");
    let outcome_request = K2InquiryOutcomeVerificationRequestV1::seal(
        verifier_executable_sha256,
        selector_request.clone(),
        precommit.clone(),
        selection_verification.clone(),
        baseline_request.clone(),
        baselines.clone(),
        observation.clone(),
        fixture.true_model_root_sha256.clone(),
    )
    .expect("development outcome request");
    let outcome_receipt =
        verify_inquiry_outcome_v1(&outcome_request).expect("development outcome verification");
    InquiryArtifactsV1 {
        selector_request,
        precommit,
        baseline_request,
        baselines,
        selection_verification,
        observation,
        outcome_request,
        outcome_receipt,
    }
}

fn assert_core_contract_v1(fixture: &InquiryFixtureV1, artifacts: &InquiryArtifactsV1) {
    assert_eq!(fixture.public_case.models.len(), 4);
    assert_eq!(fixture.public_case.probes.len(), 8);
    assert_eq!(artifacts.precommit.evaluations.len(), 8);
    assert!(
        artifacts
            .precommit
            .evaluations
            .iter()
            .all(|evaluation| evaluation.predictions.len() == 4)
    );
    assert_eq!(
        artifacts.precommit.selected_probe_root_sha256,
        fixture.roles.optimal
    );
    let selected = evaluation_v1(&artifacts.precommit, &fixture.roles.optimal);
    assert_eq!(selected.partition_sizes, vec![1, 1, 1, 1]);
    assert_eq!(
        (selected.minimax_eliminated, selected.pair_separation),
        (3, 12)
    );
    assert_eq!(artifacts.precommit.exact_best_ties, 1);
    assert_eq!(
        artifacts
            .precommit
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.eligibility.eligible)
            .count(),
        4
    );
    let decisions = artifacts
        .baselines
        .decisions
        .iter()
        .map(|decision| (decision.kind, decision.selected_probe_root_sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(decisions[&K2InquiryBaselineKindV1::Passive], None);
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::StableHash],
        Some(fixture.roles.stable.clone())
    );
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::CheapestFirst],
        Some(fixture.roles.cheapest.clone())
    );
    assert_eq!(
        decisions[&K2InquiryBaselineKindV1::ExplicitHeuristic],
        Some(fixture.roles.heuristic.clone())
    );
    assert_eq!(
        artifacts.outcome_receipt.surviving_model_roots_sha256.len(),
        1
    );
    assert!(artifacts.outcome_receipt.selector_matches_oracle);
    assert_eq!(artifacts.outcome_receipt.oracle_survivors, 1);
    assert_eq!(artifacts.outcome_receipt.complete_prediction_count, 32);
    artifacts
        .outcome_receipt
        .authority
        .validate()
        .expect("authority denied");
}

fn assert_negative_controls_v1(
    fixture: &InquiryFixtureV1,
    artifacts: &InquiryArtifactsV1,
    environment: &TestEnvironmentV1,
) -> BTreeSet<String> {
    let mut passed = BTreeSet::new();
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.unsafe_high_information,
        K2InquiryEligibilityReasonV1::NonReversible,
    );
    passed.insert("01-unsafe-high-information-veto".to_owned());
    assert_ne!(
        artifacts.precommit.selected_probe_root_sha256,
        fixture.roles.cheapest
    );
    passed.insert("02-cheapest-useless-not-selected".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.ambiguous,
        K2InquiryEligibilityReasonV1::AmbiguousObservation,
    );
    passed.insert("03-ambiguous-observation-veto".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.delayed,
        K2InquiryEligibilityReasonV1::DelayedObservation,
    );
    passed.insert("04-delayed-observation-veto".to_owned());
    assert_reason_v1(
        &artifacts.precommit,
        &fixture.roles.unknown,
        K2InquiryEligibilityReasonV1::UnknownAction,
    );
    passed.insert("05-unknown-action-veto".to_owned());

    assert_choice_invariant_public_inputs_v1(
        fixture,
        &artifacts.selector_request,
        &artifacts.baseline_request,
    );
    let mut selector_value =
        serde_json::to_value(&artifacts.selector_request).expect("selector value");
    selector_value["private_true_model_root_sha256"] =
        serde_json::Value::String(fixture.true_model_root_sha256.clone());
    let selector_bytes = composition_bytes_v1(&selector_value).expect("injected selector bytes");
    assert!(composition_decode_v1::<K2InquirySelectorRequestV1>(&selector_bytes).is_err());
    passed.insert("06-private-true-choice-excluded".to_owned());

    let mut outcome_value =
        serde_json::to_value(&artifacts.selector_request).expect("outcome value");
    outcome_value["post_outcome_root_sha256"] =
        serde_json::Value::String(artifacts.observation.observable_outcome_root_sha256.clone());
    let outcome_bytes = composition_bytes_v1(&outcome_value).expect("injected outcome bytes");
    assert!(composition_decode_v1::<K2InquirySelectorRequestV1>(&outcome_bytes).is_err());
    passed.insert("07-post-outcome-input-rejected".to_owned());

    let mut tampered_prediction = artifacts.precommit.clone();
    let selected_probe_root_sha256 = tampered_prediction.selected_probe_root_sha256.clone();
    let selected = tampered_prediction
        .evaluations
        .iter_mut()
        .find(|evaluation| evaluation.probe_root_sha256 == selected_probe_root_sha256)
        .expect("selected evaluation mutable");
    let old_prediction = selected.predictions[0].clone();
    selected.predictions[0] = K2InquiryPredictionV1::seal(
        old_prediction.model_root_sha256,
        old_prediction.probe_root_sha256,
        false,
        "tampered".to_owned(),
        fixture.public_case.probes[0].initial_manifest.clone(),
        K2InquiryObservationModeV1::ExactImmediate,
    )
    .expect("tampered prediction");
    selected.reseal().expect("reseal tampered evaluation");
    tampered_prediction
        .reseal()
        .expect("reseal tampered precommit");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &tampered_prediction,
        )
        .is_err()
    );
    passed.insert("08-tampered-prediction-rejected".to_owned());

    let mut tampered_selected = artifacts.precommit.clone();
    tampered_selected.selected_probe_root_sha256 = fixture.roles.cheapest.clone();
    tampered_selected.reseal().expect("reseal selected tamper");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &tampered_selected,
        )
        .is_err()
    );
    passed.insert("09-tampered-selected-root-rejected".to_owned());

    let mut tampered_observation = artifacts.observation.clone();
    tampered_observation.post_manifest = fixture.public_case.probes[0].initial_manifest.clone();
    let tampered_observation_request = K2InquiryOutcomeVerificationRequestV1::seal(
        artifacts
            .selection_verification
            .verifier_executable_sha256
            .clone(),
        artifacts.selector_request.clone(),
        artifacts.precommit.clone(),
        artifacts.selection_verification.clone(),
        artifacts.baseline_request.clone(),
        artifacts.baselines.clone(),
        tampered_observation,
        fixture.true_model_root_sha256.clone(),
    )
    .expect("tampered observation request");
    assert!(verify_inquiry_outcome_v1(&tampered_observation_request).is_err());
    passed.insert("10-tampered-observer-manifest-rejected".to_owned());

    let action_permuted = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            action_permutation: 1,
            ..CaseVariantV1::default()
        },
    );
    let action_permuted_artifacts = direct_artifacts_v1(&action_permuted);
    assert_eq!(
        action_permuted_artifacts
            .precommit
            .selected_probe_root_sha256,
        action_permuted.roles.optimal
    );
    assert_selected_partition_v1(
        &action_permuted_artifacts.precommit,
        &action_permuted.roles.optimal,
    );
    passed.insert("11-action-id-permutation-invariant".to_owned());

    let path_bijected = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            path_bijection: 1,
            ..CaseVariantV1::default()
        },
    );
    let path_bijected_artifacts = direct_artifacts_v1(&path_bijected);
    assert_eq!(
        path_bijected_artifacts.precommit.selected_probe_root_sha256,
        path_bijected.roles.optimal
    );
    assert_selected_partition_v1(
        &path_bijected_artifacts.precommit,
        &path_bijected.roles.optimal,
    );
    passed.insert("12-path-bijection-invariant".to_owned());

    let reversed = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            reverse_candidates: true,
            ..CaseVariantV1::default()
        },
    );
    let reversed_artifacts = direct_artifacts_v1(&reversed);
    assert_eq!(
        reversed_artifacts.precommit.selected_probe_root_sha256,
        artifacts.precommit.selected_probe_root_sha256
    );
    passed.insert("13-candidate-order-shuffle-invariant".to_owned());

    let collapsed = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            collapse_optimal_predictions: true,
            ..CaseVariantV1::default()
        },
    );
    let collapsed_artifacts = direct_artifacts_v1(&collapsed);
    let collapsed_selected = evaluation_v1(
        &collapsed_artifacts.precommit,
        &collapsed_artifacts.precommit.selected_probe_root_sha256,
    );
    assert!(collapsed_selected.minimax_eliminated < 3);
    assert!(
        collapsed_artifacts
            .outcome_receipt
            .surviving_model_roots_sha256
            .len()
            > 1
    );
    passed.insert("14-collapsed-predictions-destroy-unique-id".to_owned());

    let rotated = build_fixture_v1(
        fixture.case_index,
        &fixture.split_commitment_root_sha256,
        CaseVariantV1 {
            rotate_model_effect_bindings: true,
            ..CaseVariantV1::default()
        },
    );
    let rotated_artifacts = direct_artifacts_v1(&rotated);
    let original_outcome = predicted_outcome_for_model_id_v1(
        fixture,
        &artifacts.precommit,
        &fixture.true_model_id_sha256,
    );
    let rotated_outcome = predicted_outcome_for_model_id_v1(
        &rotated,
        &rotated_artifacts.precommit,
        &rotated.true_model_id_sha256,
    );
    assert_ne!(original_outcome, rotated_outcome);
    passed.insert("15-shuffled-model-effect-binding-changes-result".to_owned());

    assert_same_identity_redispatch_rejected_v1(environment);
    passed.insert("16-same-identity-redispatch-rejected".to_owned());
    assert_journal_restart_and_fault_parity_v1(environment);
    passed.insert("17-journal-prefix-restart-parity".to_owned());
    assert_authority_promotion_rejected_v1(fixture, artifacts);
    passed.insert("18-authority-promotion-rejected".to_owned());
    passed
}

fn assert_generated_provenance_control_v1(fixture: &InquiryFixtureV1) {
    let original = fixture
        .public_case
        .probe(&fixture.roles.optimal)
        .expect("optimal probe");
    let foreign = K2InquiryProbeV1::seal(
        original.experiment_id_sha256.clone(),
        original.probe_id_sha256.clone(),
        original.action_id_sha256.clone(),
        original.initial_manifest.clone(),
        original.reversible,
        original.observation_mode,
        original.risk_units,
        original.cost_units,
        original.applicability_hint,
        original.dependency_hint,
        original.cleanup_hint,
        root_v1(&"foreign-generated-provenance"),
    )
    .expect("foreign-provenance probe");
    let mut probes = fixture.public_case.probes.clone();
    probes.retain(|probe| probe.probe_root_sha256 != fixture.roles.optimal);
    let foreign_root = foreign.probe_root_sha256.clone();
    probes.push(foreign);
    let case = K2InquiryPublicCaseV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        fixture.public_case.generator_schema_root_sha256.clone(),
        fixture.public_case.split_commitment_root_sha256.clone(),
        fixture.public_case.models.clone(),
        probes,
    )
    .expect("foreign-provenance case");
    let request = K2InquirySelectorRequestV1::seal(root_v1(&"provenance-selector"), case)
        .expect("provenance selector request");
    let precommit = select_model_guided_probe_v1(&request).expect("provenance selection");
    assert_reason_v1(
        &precommit,
        &foreign_root,
        K2InquiryEligibilityReasonV1::NonGeneratedProvenance,
    );
}

fn assert_choice_invariant_public_inputs_v1(
    fixture: &InquiryFixtureV1,
    selector_request: &K2InquirySelectorRequestV1,
    baseline_request: &K2InquiryBaselineRequestV1,
) {
    let selector_bytes = composition_bytes_v1(selector_request).expect("selector bytes");
    let baseline_bytes = composition_bytes_v1(baseline_request).expect("baseline bytes");
    let selector_variants = fixture
        .public_case
        .models
        .iter()
        .map(|_private_choice| selector_bytes.clone())
        .collect::<BTreeSet<_>>();
    let baseline_variants = fixture
        .public_case
        .models
        .iter()
        .map(|_private_choice| baseline_bytes.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(selector_variants.len(), 1);
    assert_eq!(baseline_variants.len(), 1);
    let selector_value = serde_json::to_value(selector_request).expect("selector value");
    let baseline_value = serde_json::to_value(baseline_request).expect("baseline value");
    assert!(
        selector_value
            .get("private_true_model_root_sha256")
            .is_none()
    );
    assert!(
        baseline_value
            .get("private_true_model_root_sha256")
            .is_none()
    );
    assert!(selector_value.get("observation").is_none());
    assert!(baseline_value.get("observation").is_none());
}

fn assert_authority_promotion_rejected_v1(
    fixture: &InquiryFixtureV1,
    artifacts: &InquiryArtifactsV1,
) {
    let mut selector = artifacts.selector_request.clone();
    selector.authority.natural_k2_authority = true;
    assert!(selector.validate().is_err());
    let mut baseline = artifacts.baseline_request.clone();
    baseline.authority.product_authority = true;
    assert!(baseline.validate().is_err());
    let selected_probe = fixture
        .public_case
        .probe(&artifacts.precommit.selected_probe_root_sha256)
        .expect("selected probe");
    let true_model = fixture
        .public_case
        .model(&fixture.true_model_root_sha256)
        .expect("true model");
    let mut worker = K2InquiryWorkerRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        artifacts.selection_verification.receipt_root_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        selected_probe.action_id_sha256.clone(),
        root_v1(&"authority-worker"),
        selected_probe.initial_manifest.clone(),
        true_model
            .effect(&selected_probe.action_id_sha256)
            .expect("true effect")
            .clone(),
    )
    .expect("authority worker request");
    worker.authority.package_activated = true;
    assert!(worker.validate().is_err());
    let mut observer = K2InquiryObserverRequestV1::seal(
        fixture.public_case.experiment_id_sha256.clone(),
        selected_probe.probe_root_sha256.clone(),
        root_v1(&"authority-observer"),
    )
    .expect("authority observer request");
    observer.authority.phase_memory_mutated = true;
    assert!(observer.validate().is_err());
    let mut outcome_request = artifacts.outcome_request.clone();
    outcome_request.authority.law_certificate_issued = true;
    assert!(verify_inquiry_outcome_v1(&outcome_request).is_err());
    let mut precommit = artifacts.precommit.clone();
    precommit.authority.k1_registry_mutated = true;
    precommit.reseal().expect("reseal promoted precommit");
    assert!(
        verify_inquiry_selection_v1(
            artifacts
                .selection_verification
                .verifier_executable_sha256
                .clone(),
            &artifacts.selector_request,
            &precommit,
        )
        .is_err()
    );
    let mut observation = artifacts.observation.clone();
    observation.authority.deployment_authority = true;
    observation.reseal().expect("reseal promoted observation");
    let request = K2InquiryOutcomeVerificationRequestV1::seal(
        artifacts
            .selection_verification
            .verifier_executable_sha256
            .clone(),
        artifacts.selector_request.clone(),
        artifacts.precommit.clone(),
        artifacts.selection_verification.clone(),
        artifacts.baseline_request.clone(),
        artifacts.baselines.clone(),
        observation,
        fixture.true_model_root_sha256.clone(),
    )
    .expect("promoted observation request");
    assert!(verify_inquiry_outcome_v1(&request).is_err());
    let mut terminal = artifacts.outcome_receipt.clone();
    terminal.authority.product_authority = true;
    assert!(terminal.authority.validate().is_err());
}

fn assert_same_identity_redispatch_rejected_v1(environment: &TestEnvironmentV1) {
    let experiment = root_v1(&(
        "redispatch-control",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut journal = K2InquiryJournalV1::create(&environment.journal_store, experiment.clone())
        .expect("redispatch journal");
    for (index, kind) in journal_kinds_v1().into_iter().take(6).enumerate() {
        journal
            .append(kind, root_v1(&(experiment.as_str(), index)))
            .expect("append redispatch prefix");
    }
    assert!(
        journal
            .append(
                K2InquiryJournalEventKindV1::ProbeDispatched,
                root_v1(&"duplicate-probe-dispatch"),
            )
            .is_err()
    );
    let reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, experiment)
        .expect("reopen redispatch journal");
    assert_eq!(reopened.projection().event_count, 6);
    assert!(reopened.projection().indeterminate_probe_dispatch);
    reopened.cleanup().expect("cleanup redispatch journal");
}

fn assert_journal_restart_and_fault_parity_v1(environment: &TestEnvironmentV1) {
    for prefix in 0..=10 {
        let experiment = root_v1(&(
            "journal-prefix-control",
            prefix,
            TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
        ));
        let mut journal =
            K2InquiryJournalV1::create(&environment.journal_store, experiment.clone())
                .expect("prefix journal");
        for (index, kind) in journal_kinds_v1().into_iter().take(prefix).enumerate() {
            journal
                .append(kind, root_v1(&(experiment.as_str(), index)))
                .expect("append prefix event");
            let reopened =
                K2InquiryJournalV1::open_existing(&environment.journal_store, experiment.clone())
                    .expect("reopen legal prefix");
            assert_eq!(reopened.projection(), journal.projection());
        }
        let reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, experiment)
            .expect("reopen final prefix");
        assert_eq!(reopened.projection(), journal.projection());
        reopened.cleanup().expect("cleanup prefix journal");
    }

    let before_id = root_v1(&(
        "journal-before-rename",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut before = K2InquiryJournalV1::create(&environment.journal_store, before_id.clone())
        .expect("before-rename journal");
    assert!(
        before
            .append_with_fault(
                K2InquiryJournalEventKindV1::ExperimentFrozen,
                root_v1(&"before-rename-payload"),
                K2InquiryJournalFaultV1::BeforeRename,
            )
            .is_err()
    );
    let before_reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, before_id)
        .expect("reopen before-rename journal");
    assert_eq!(before_reopened.projection().event_count, 0);
    before_reopened
        .cleanup()
        .expect("cleanup before-rename journal");

    let after_id = root_v1(&(
        "journal-after-rename",
        TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed),
    ));
    let mut after = K2InquiryJournalV1::create(&environment.journal_store, after_id.clone())
        .expect("after-rename journal");
    assert!(
        after
            .append_with_fault(
                K2InquiryJournalEventKindV1::ExperimentFrozen,
                root_v1(&"after-rename-payload"),
                K2InquiryJournalFaultV1::AfterRename,
            )
            .is_err()
    );
    let after_reopened = K2InquiryJournalV1::open_existing(&environment.journal_store, after_id)
        .expect("reopen after-rename journal");
    assert_eq!(after_reopened.projection().event_count, 1);
    after_reopened
        .cleanup()
        .expect("cleanup after-rename journal");
}

fn journal_kinds_v1() -> [K2InquiryJournalEventKindV1; 10] {
    [
        K2InquiryJournalEventKindV1::ExperimentFrozen,
        K2InquiryJournalEventKindV1::BaselinesFrozen,
        K2InquiryJournalEventKindV1::SelectionDispatched,
        K2InquiryJournalEventKindV1::SelectionPrecommitted,
        K2InquiryJournalEventKindV1::SelectionVerified,
        K2InquiryJournalEventKindV1::ProbeDispatched,
        K2InquiryJournalEventKindV1::ProbeObserved,
        K2InquiryJournalEventKindV1::ModelsUpdated,
        K2InquiryJournalEventKindV1::ControlsFrozen,
        K2InquiryJournalEventKindV1::TerminalFrozen,
    ]
}

fn assert_confirm_disjointness_v1(fixtures: &[InquiryFixtureV1]) {
    assert_eq!(fixtures.len(), 8);
    let experiment_roots = fixtures
        .iter()
        .map(|fixture| &fixture.public_case.experiment_id_sha256)
        .collect::<BTreeSet<_>>();
    let case_roots = fixtures
        .iter()
        .map(|fixture| &fixture.public_case.case_root_sha256)
        .collect::<BTreeSet<_>>();
    let model_roots = fixtures
        .iter()
        .flat_map(|fixture| {
            fixture
                .public_case
                .models
                .iter()
                .map(|model| &model.model_root_sha256)
        })
        .collect::<BTreeSet<_>>();
    let probe_roots = fixtures
        .iter()
        .flat_map(|fixture| {
            fixture
                .public_case
                .probes
                .iter()
                .map(|probe| &probe.probe_root_sha256)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(experiment_roots.len(), 8);
    assert_eq!(case_roots.len(), 8);
    assert_eq!(model_roots.len(), 8 * 4);
    assert_eq!(probe_roots.len(), 8 * 8);
    assert!(fixtures.iter().all(|fixture| {
        fixture.public_case.split_commitment_root_sha256 == CONFIRM_COMMITMENT_V1
            && fixture.public_case.generator_schema_root_sha256 == GENERATOR_SCHEMA_ROOT_V1
    }));
}

fn append_and_reopen_v1(
    journal: &mut K2InquiryJournalV1,
    store: &Path,
    experiment_id_sha256: &str,
    kind: K2InquiryJournalEventKindV1,
    payload_root_sha256: String,
) {
    journal
        .append(kind, payload_root_sha256)
        .expect("append inquiry journal event");
    let reopened = K2InquiryJournalV1::open_existing(store, experiment_id_sha256.to_owned())
        .expect("reopen inquiry journal");
    assert_eq!(reopened.projection(), journal.projection());
}

fn evaluation_v1<'a>(
    precommit: &'a K2InquirySelectionPrecommitV1,
    probe_root: &str,
) -> &'a K2InquiryProbeEvaluationV1 {
    precommit
        .evaluations
        .iter()
        .find(|evaluation| evaluation.probe_root_sha256 == probe_root)
        .expect("probe evaluation")
}

fn assert_reason_v1(
    precommit: &K2InquirySelectionPrecommitV1,
    probe_root: &str,
    reason: K2InquiryEligibilityReasonV1,
) {
    let evaluation = evaluation_v1(precommit, probe_root);
    assert!(!evaluation.eligibility.eligible);
    assert_eq!(evaluation.eligibility.reason, reason);
}

fn assert_selected_partition_v1(precommit: &K2InquirySelectionPrecommitV1, optimal_root: &str) {
    let evaluation = evaluation_v1(precommit, optimal_root);
    assert_eq!(evaluation.partition_sizes, vec![1, 1, 1, 1]);
    assert_eq!(
        (evaluation.minimax_eliminated, evaluation.pair_separation),
        (3, 12)
    );
}

fn predicted_outcome_for_model_id_v1(
    fixture: &InquiryFixtureV1,
    precommit: &K2InquirySelectionPrecommitV1,
    model_id_sha256: &str,
) -> String {
    let model_root = &fixture
        .public_case
        .models
        .iter()
        .find(|model| model.model_id_sha256 == model_id_sha256)
        .expect("model by opaque id")
        .model_root_sha256;
    evaluation_v1(precommit, &fixture.roles.optimal)
        .predictions
        .iter()
        .find(|prediction| &prediction.model_root_sha256 == model_root)
        .expect("model prediction")
        .observable_outcome_root_sha256
        .clone()
}

fn copy_v1(source: &str, target: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::CopyFile {
        source_path: source.to_owned(),
        target_path: target.to_owned(),
    }
}

fn remove_v1(path: &str) -> K2CompositionLearnedEffectV1 {
    K2CompositionLearnedEffectV1::RemoveFile {
        path: path.to_owned(),
    }
}

fn root_v1<T: Serialize>(value: &T) -> String {
    composition_root_v1(&("nando.k2-inquiry-test-root.v1", value)).expect("test root")
}

fn root_set_v1(label: &str, roots: impl IntoIterator<Item = String>) -> String {
    let mut roots = roots.into_iter().collect::<Vec<_>>();
    roots.sort();
    composition_root_v1(&(label, roots)).expect("root set")
}

fn directory_is_empty_v1(path: &Path) -> bool {
    fs::read_dir(path)
        .expect("read generated directory")
        .next()
        .is_none()
}

struct TestEnvironmentV1 {
    root: PathBuf,
    journal_store: PathBuf,
    workspace_store: PathBuf,
}

impl TestEnvironmentV1 {
    fn new(label: &str) -> Self {
        let sequence = TEST_SEQUENCE_V1.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nando-k2-inquiry-{label}-{}-{sequence}",
            std::process::id()
        ));
        let journal_store = root.join("journals");
        let workspace_store = root.join("workspaces");
        fs::create_dir_all(&journal_store).expect("create journal store");
        fs::create_dir_all(&workspace_store).expect("create workspace store");
        Self {
            root,
            journal_store,
            workspace_store,
        }
    }
}

impl Drop for TestEnvironmentV1 {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct ProcessBinariesV1 {
    selector: PathBuf,
    baseline: PathBuf,
    verifier: PathBuf,
    worker: PathBuf,
    observer: PathBuf,
    selector_sha256: String,
    baseline_sha256: String,
    verifier_sha256: String,
    worker_sha256: String,
    observer_sha256: String,
}

impl ProcessBinariesV1 {
    fn from_cargo() -> Self {
        let selector = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-selector"));
        let baseline = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-baseline"));
        let verifier = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-verifier"));
        let worker = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-worker"));
        let observer = PathBuf::from(env!("CARGO_BIN_EXE_nando-k2-inquiry-observer"));
        Self {
            selector_sha256: composition_sha256_file_v1(&selector).expect("selector sha"),
            baseline_sha256: composition_sha256_file_v1(&baseline).expect("baseline sha"),
            verifier_sha256: composition_sha256_file_v1(&verifier).expect("verifier sha"),
            worker_sha256: composition_sha256_file_v1(&worker).expect("worker sha"),
            observer_sha256: composition_sha256_file_v1(&observer).expect("observer sha"),
            selector,
            baseline,
            verifier,
            worker,
            observer,
        }
    }

    fn assert_pairwise_distinct(&self) {
        let orchestrator = std::env::current_exe().expect("orchestrator executable");
        let roots = [
            self.selector_sha256.clone(),
            self.baseline_sha256.clone(),
            self.verifier_sha256.clone(),
            self.worker_sha256.clone(),
            self.observer_sha256.clone(),
            composition_sha256_file_v1(&orchestrator).expect("orchestrator sha"),
        ];
        assert_eq!(roots.iter().collect::<BTreeSet<_>>().len(), roots.len());
    }
}

fn run_isolated_protocol_v1<T, U>(executable: &Path, request: &T) -> U
where
    T: Serialize,
    U: serde::de::DeserializeOwned + Serialize,
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
        .args(["--", "/usr/bin/prlimit", "--cpu=10:10"])
        .args(["--as=536870912:536870912", "--nproc=32:32"])
        .args(["--fsize=33554432:33554432", "--", guest])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn isolated inquiry process");
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
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll inquiry process") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "isolated inquiry process timed out"
        );
        thread::sleep(Duration::from_millis(5));
    };
    let stdout = stdout_reader.join().expect("join stdout reader");
    let stderr = stderr_reader.join().expect("join stderr reader");
    assert!(
        status.success(),
        "isolated inquiry process failed: {}",
        String::from_utf8_lossy(&stderr)
    );
    composition_decode_v1(&stdout).expect("decode isolated inquiry output")
}
