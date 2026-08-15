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

