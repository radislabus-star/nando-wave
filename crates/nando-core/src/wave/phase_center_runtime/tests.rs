use super::*;

fn passing_threshold_policy() -> PhaseCenterThresholdPolicyEvidence {
    PhaseCenterThresholdPolicyEvidence {
        candidate_bucket_count: 1,
        auto_calibrated_bucket_count: 1,
        calibration_window_before_shadow: true,
        shadow_window_after_calibration: true,
        per_bucket_thresholds_reported: true,
        fixed_policy_shadow_replay: true,
    }
}

fn test_verifier_binding() -> PhaseCenterVerifierBinding {
    PhaseCenterVerifierBinding {
        verifier_id: 11,
        verifier_version: 1,
        verifier_input_kind_id: 22,
        verifier_evidence_source_id: 33,
        false_accept_threshold: 0,
    }
}

fn promotion_evidence(
    future_shadow_events: usize,
    unique_accepts: usize,
    tokens_saved: u64,
    cost_saved_microusd: u64,
) -> PhaseCenterPromotionEvidence {
    PhaseCenterPromotionEvidence {
        future_shadow_events,
        unique_cpu_accepts_over_exact_cache: unique_accepts,
        tokens_saved,
        cost_saved_microusd,
        false_accepts: 0,
        runtime_margin_parity_mismatches: 0,
        verifier_binding: test_verifier_binding(),
        threshold_policy: passing_threshold_policy(),
        exact_cache_overlap_excluded: true,
        token_cost_denominator_present: true,
        local_accept_enabled: false,
    }
}

#[test]
fn phase_hash_is_unit_and_deterministic() {
    let a = stable_phase_cell("rel:o0:s1", 7);
    let b = stable_phase_cell("rel:o0:s1", 7);
    let magnitude = (a.re * a.re + a.im * a.im).sqrt();
    assert_eq!(a, b);
    assert!((magnitude - 1.0).abs() < 1e-12);
}

#[test]
fn runtime_scores_correct_transition_above_wrong() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: positive.into_boxed_slice(),
        wrong_vec: negative.into_boxed_slice(),
    };
    assert!(runtime.margin(&task).expect("valid task") > 0.0);
}

#[test]
fn offload_policy_rejects_invalid_threshold() {
    assert_eq!(
        PhaseCenterOffloadPolicy::new(0),
        Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
    );
    assert_eq!(
        PhaseCenterOffloadPolicy::new(-1),
        Err(PhaseCenterRuntimeError::InvalidOffloadThreshold)
    );
}

#[test]
fn offload_policy_routes_by_margin_micro_threshold() {
    let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid threshold");
    let local = policy.decide_margin(0.3004).expect("finite margin");
    let fallback = policy.decide_margin(0.2994).expect("finite margin");
    assert_eq!(local.margin_micro, 300_400);
    assert_eq!(local.action, PhaseCenterOffloadAction::LocalOperator);
    assert!(local.is_local_operator());
    assert_eq!(fallback.margin_micro, 299_400);
    assert_eq!(fallback.action, PhaseCenterOffloadAction::FallbackToLlm);
    assert!(fallback.is_fallback_to_llm());
}

#[test]
fn offload_policy_rejects_nonfinite_margin() {
    let policy = PhaseCenterOffloadPolicy::default_conservative();
    assert_eq!(
        policy.decide_margin(f64::NAN),
        Err(PhaseCenterRuntimeError::InvalidMargin)
    );
    assert_eq!(
        phase_margin_to_micro(f64::INFINITY),
        Err(PhaseCenterRuntimeError::InvalidMargin)
    );
}

#[test]
fn runtime_offload_decision_uses_packaged_margin() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let task = PhaseCenterEvalTask {
        center_index: 0,
        correct_vec: positive.into_boxed_slice(),
        wrong_vec: negative.into_boxed_slice(),
    };
    let decision = runtime
        .offload_decision(&task, policy)
        .expect("valid offload decision");
    assert!(decision.is_local_operator());
    assert!(decision.margin_micro > 0);
}

#[test]
fn runtime_offload_decisions_batch_matches_per_task_decisions() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let tasks = vec![
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.clone().into_boxed_slice(),
            wrong_vec: negative.clone().into_boxed_slice(),
        },
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: negative.clone().into_boxed_slice(),
            wrong_vec: positive.clone().into_boxed_slice(),
        },
    ];
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let batch = runtime
        .offload_decisions(&tasks, policy)
        .expect("valid batch decisions");
    let per_task = tasks
        .iter()
        .map(|task| {
            runtime
                .offload_decision(task, policy)
                .expect("valid per-task decision")
        })
        .collect::<Vec<_>>();
    assert_eq!(batch, per_task);
    assert!(batch[0].is_local_operator());
    assert!(batch[1].is_fallback_to_llm());
}

#[test]
fn runtime_offload_decisions_into_reuses_caller_buffer() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let tasks = vec![
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.clone().into_boxed_slice(),
            wrong_vec: negative.clone().into_boxed_slice(),
        },
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: negative.clone().into_boxed_slice(),
            wrong_vec: positive.clone().into_boxed_slice(),
        },
    ];
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let expected = runtime
        .offload_decisions(&tasks, policy)
        .expect("valid batch decisions");
    let mut out = Vec::with_capacity(8);
    let original_capacity = out.capacity();
    runtime
        .offload_decisions_into(&tasks, policy, &mut out)
        .expect("valid reused-buffer batch decisions");
    assert_eq!(out, expected);
    assert_eq!(out.capacity(), original_capacity);

    runtime
        .offload_decisions_into(tasks.iter().take(1), policy, &mut out)
        .expect("valid shorter reused-buffer batch decisions");
    assert_eq!(out.len(), 1);
    assert_eq!(out.capacity(), original_capacity);
}

#[test]
fn runtime_offload_decisions_for_batch_reports_first_error() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let valid_task = (0, positive.as_slice(), negative.as_slice());
    let invalid_width = (0, positive[..7].as_ref(), negative.as_slice());
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    assert_eq!(
        runtime.offload_decisions_for([valid_task, invalid_width], policy),
        Err(PhaseCenterRuntimeError::VectorWidthMismatch)
    );
}

#[test]
fn runtime_offload_decisions_for_into_reuses_buffer_and_reports_error() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let mut out = Vec::with_capacity(4);
    let original_capacity = out.capacity();
    runtime
        .offload_decisions_for_into(
            [(0, positive.as_slice(), negative.as_slice())],
            policy,
            &mut out,
        )
        .expect("valid raw-slice batch decisions");
    assert_eq!(out.len(), 1);
    assert_eq!(out.capacity(), original_capacity);
    assert!(out[0].is_local_operator());

    assert_eq!(
        runtime.offload_decisions_for_into(
            [(0, positive[..7].as_ref(), negative.as_slice())],
            policy,
            &mut out,
        ),
        Err(PhaseCenterRuntimeError::VectorWidthMismatch)
    );
    assert!(out.is_empty());
    assert_eq!(out.capacity(), original_capacity);
}

#[test]
fn runtime_offload_summary_into_reuses_caller_buffers() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let tasks = vec![
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.clone().into_boxed_slice(),
            wrong_vec: negative.clone().into_boxed_slice(),
        },
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: negative.clone().into_boxed_slice(),
            wrong_vec: positive.clone().into_boxed_slice(),
        },
    ];
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let expected_decisions = runtime
        .offload_decisions(&tasks, policy)
        .expect("valid batch decisions");
    let expected_summary = PhaseCenterOffloadSummary::from_decision_slice(&expected_decisions);
    let mut decision_scratch = Vec::with_capacity(8);
    let mut margin_scratch = Vec::with_capacity(8);
    let decision_capacity = decision_scratch.capacity();
    let margin_capacity = margin_scratch.capacity();

    let summary = runtime
        .offload_summary_into(&tasks, policy, &mut decision_scratch, &mut margin_scratch)
        .expect("valid summary");

    assert_eq!(decision_scratch, expected_decisions);
    assert_eq!(summary, expected_summary);
    assert_eq!(decision_scratch.capacity(), decision_capacity);
    assert_eq!(margin_scratch.capacity(), margin_capacity);
}

#[test]
fn runtime_offload_summary_for_into_reuses_caller_buffers() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let mut decision_scratch = Vec::with_capacity(4);
    let mut margin_scratch = Vec::with_capacity(4);
    let decision_capacity = decision_scratch.capacity();
    let margin_capacity = margin_scratch.capacity();
    let summary = runtime
        .offload_summary_for_into(
            [
                (0, positive.as_slice(), negative.as_slice()),
                (0, negative.as_slice(), positive.as_slice()),
            ],
            policy,
            &mut decision_scratch,
            &mut margin_scratch,
        )
        .expect("valid raw-slice summary");

    assert_eq!(summary.calls, 2);
    assert_eq!(summary.local_operator_calls, 1);
    assert_eq!(summary.fallback_to_llm_calls, 1);
    assert_eq!(decision_scratch.capacity(), decision_capacity);
    assert_eq!(margin_scratch.capacity(), margin_capacity);
}

#[test]
fn offload_runtime_from_package_bytes_reuses_caller_buffers() {
    let positive = phase_vector_from_atoms(["class:order", "rel:o0:s1"], 8);
    let negative = phase_vector_from_atoms(["class:order", "rel:o0:s0"], 8);
    let runtime = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid runtime");
    let bytes = runtime.to_bytes().expect("runtime serializes");
    let package_info =
        PhaseCenterOffloadRuntime::inspect_package_bytes(&bytes).expect("sdk inspects");
    assert_eq!(
        package_info,
        PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects")
    );
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let offload_runtime =
        PhaseCenterOffloadRuntime::from_package_bytes(&bytes, policy).expect("sdk loads");
    assert_eq!(offload_runtime.package_info(), package_info);
    assert_eq!(offload_runtime.policy(), policy);
    assert_eq!(offload_runtime.cells(), 8);
    assert_eq!(offload_runtime.record_count(), 1);
    assert_eq!(offload_runtime.bytes_estimate(), runtime.bytes_estimate());
    assert_eq!(
        offload_runtime.runtime().record_count(),
        runtime.record_count()
    );

    let tasks = vec![
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: positive.clone().into_boxed_slice(),
            wrong_vec: negative.clone().into_boxed_slice(),
        },
        PhaseCenterEvalTask {
            center_index: 0,
            correct_vec: negative.clone().into_boxed_slice(),
            wrong_vec: positive.clone().into_boxed_slice(),
        },
    ];
    let expected_summary = runtime
        .offload_summary_into(
            &tasks,
            policy,
            &mut Vec::with_capacity(2),
            &mut Vec::with_capacity(2),
        )
        .expect("runtime summary");
    let mut decision_scratch = Vec::with_capacity(4);
    let mut margin_scratch = Vec::with_capacity(4);
    let decision_capacity = decision_scratch.capacity();
    let margin_capacity = margin_scratch.capacity();
    let summary = offload_runtime
        .offload_summary_into(&tasks, &mut decision_scratch, &mut margin_scratch)
        .expect("sdk summary");
    assert_eq!(summary, expected_summary);
    assert_eq!(decision_scratch.capacity(), decision_capacity);
    assert_eq!(margin_scratch.capacity(), margin_capacity);
}

#[test]
fn offload_runtime_rejects_bad_package_bytes() {
    let policy = PhaseCenterOffloadPolicy::default_conservative();
    assert_eq!(
        PhaseCenterOffloadRuntime::inspect_package_bytes(b"bad"),
        Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
    );
    assert_eq!(
        PhaseCenterOffloadRuntime::from_package_bytes(b"bad", policy),
        Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
    );
}

#[test]
fn offload_summary_counts_unique_decisions_and_false_local_accepts() {
    let policy = PhaseCenterOffloadPolicy::new(1).expect("valid policy");
    let local = policy.decide_margin(0.5).expect("finite margin");
    let fallback = policy.decide_margin(-0.1).expect("finite margin");
    let false_local = PhaseCenterOffloadDecision {
        action: PhaseCenterOffloadAction::LocalOperator,
        margin_micro: 0,
        margin_threshold_micro: 1,
    };
    let summary = PhaseCenterOffloadSummary::from_decisions([local, fallback, false_local]);
    assert_eq!(
        summary,
        PhaseCenterOffloadSummary {
            calls: 3,
            local_operator_calls: 2,
            fallback_to_llm_calls: 1,
            offload_rate_milli: 667,
            local_accuracy_milli: 500,
            false_local_accepts: 1,
            median_margin_micro: 0,
            p10_margin_micro: -100_000,
        }
    );
}

#[test]
fn offload_summary_repeats_decision_ring_for_simulated_calls() {
    let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
    let local = policy.decide_margin(0.4).expect("finite margin");
    let fallback = policy.decide_margin(0.2).expect("finite margin");
    let summary = PhaseCenterOffloadSummary::from_repeated_decisions([local, fallback], 5);
    assert_eq!(summary.calls, 5);
    assert_eq!(summary.local_operator_calls, 3);
    assert_eq!(summary.fallback_to_llm_calls, 2);
    assert_eq!(summary.offload_rate_milli, 600);
    assert_eq!(summary.local_accuracy_milli, 1000);
    assert_eq!(summary.false_local_accepts, 0);
    assert_eq!(summary.median_margin_micro, 400_000);
    assert_eq!(summary.p10_margin_micro, 200_000);
}

#[test]
fn offload_summary_into_reuses_caller_margin_scratch() {
    let policy = PhaseCenterOffloadPolicy::new(300_000).expect("valid policy");
    let local = policy.decide_margin(0.4).expect("finite margin");
    let fallback = policy.decide_margin(0.2).expect("finite margin");
    let decisions = [local, fallback];
    let mut margin_scratch = Vec::with_capacity(8);
    let original_capacity = margin_scratch.capacity();
    let unique =
        PhaseCenterOffloadSummary::from_decision_slice_into(&decisions, &mut margin_scratch);
    assert_eq!(unique.calls, 2);
    assert_eq!(unique.local_operator_calls, 1);
    assert_eq!(unique.fallback_to_llm_calls, 1);
    assert_eq!(margin_scratch, [200_000, 400_000]);
    assert_eq!(margin_scratch.capacity(), original_capacity);

    let repeated = PhaseCenterOffloadSummary::from_repeated_decision_fn_into(
        decisions.len(),
        5,
        |index| decisions[index],
        &mut margin_scratch,
    );
    assert_eq!(repeated.calls, 5);
    assert_eq!(repeated.local_operator_calls, 3);
    assert_eq!(repeated.fallback_to_llm_calls, 2);
    assert_eq!(repeated.median_margin_micro, 400_000);
    assert_eq!(
        margin_scratch,
        [200_000, 200_000, 400_000, 400_000, 400_000]
    );
    assert_eq!(margin_scratch.capacity(), original_capacity);
}

#[test]
fn atom_encoder_matches_allocating_phase_vector_and_reuses_scratch() {
    let expected = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let capacity_before = encoder.scratch_capacity();
    let encoded = encoder
        .encode_atoms(["family:test_output_parse", "state:exit0", "result:pass"])
        .expect("atoms encode")
        .to_vec();
    assert_eq!(encoded, expected);
    assert_eq!(encoder.cells(), 16);
    assert_eq!(encoder.scratch_capacity(), capacity_before);

    let other = encoder
        .encode_atoms(["family:test_output_parse", "state:panic", "result:fail"])
        .expect("second atoms encode")
        .to_vec();
    let other_expected = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );
    assert_eq!(other, other_expected);
    assert_eq!(encoder.scratch_capacity(), capacity_before);
    assert_eq!(
        PhaseCenterAtomEncoder::new(0),
        Err(PhaseCenterRuntimeError::EmptyRuntime)
    );
}

#[test]
fn atom_id_encoder_matches_allocating_phase_vector_and_reuses_scratch() {
    let expected = phase_vector_from_atom_ids([101, 202, 303], 16);
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let capacity_before = encoder.scratch_capacity();
    let encoded = encoder
        .encode_atom_ids([101, 202, 303])
        .expect("atom ids encode")
        .to_vec();
    assert_eq!(encoded, expected);
    assert_eq!(encoder.scratch_capacity(), capacity_before);

    let other = encoder
        .encode_atom_ids([101, 404, 505])
        .expect("second atom ids encode")
        .to_vec();
    assert_ne!(other, expected);
    assert_eq!(other, phase_vector_from_atom_ids([101, 404, 505], 16));
    assert_eq!(encoder.scratch_capacity(), capacity_before);

    let cell = stable_phase_atom_id_cell(101, 7);
    let magnitude = (cell.re * cell.re + cell.im * cell.im).sqrt();
    assert!((magnitude - 1.0).abs() < 0.000_000_001);
    assert_eq!(
        stable_phase_atom_id_cell(101, 7),
        stable_phase_atom_id_cell(101, 7)
    );
    assert_ne!(
        stable_phase_atom_id_cell(101, 7),
        stable_phase_atom_id_cell(101, 8)
    );
}

#[test]
fn online_miner_learns_then_scores_future_events() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );

    let first = miner
        .observe(7, &positive, true, false, 100, 300)
        .expect("first event accepted");
    let second = miner
        .observe(7, &negative, false, false, 100, 300)
        .expect("second event accepted");
    assert!(!first.active_before_update);
    assert!(!second.active_before_update);

    let calibration_positive = miner
        .observe(7, &positive, true, false, 100, 300)
        .expect("positive calibration event accepted");
    let calibration_negative = miner
        .observe(7, &negative, false, false, 100, 300)
        .expect("negative calibration event accepted");
    assert!(calibration_positive.calibration_event);
    assert!(calibration_negative.calibration_event);
    assert!(!calibration_positive.local_operator_shadow_decision);
    assert!(!calibration_negative.local_operator_shadow_decision);

    let accepted = miner
        .observe(7, &positive, true, false, 123, 456)
        .expect("future positive event scored");
    let rejected_wrong = miner
        .observe(7, &negative, false, false, 123, 456)
        .expect("future negative event scored");
    assert!(accepted.active_before_update);
    assert!(!accepted.calibration_event);
    assert!(accepted.raw_local_operator);
    assert!(accepted.local_operator_shadow_decision);
    assert!(accepted.unique_cpu_accept_over_exact_cache);
    assert!(!accepted.false_accept);
    assert!(!rejected_wrong.raw_local_operator);
    assert!(!rejected_wrong.false_accept);

    let summary = miner.summary();
    assert_eq!(summary.bucket_count, 1);
    assert_eq!(summary.active_bucket_count, 1);
    assert_eq!(summary.candidate_bucket_count, 1);
    assert_eq!(summary.rejected_bucket_count, 0);
    assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(summary.tokens_saved, 123);
    assert_eq!(summary.cost_saved_microusd, 456);
    assert_eq!(summary.false_accepts, 0);

    let runtime = miner
        .candidate_runtime(7)
        .expect("candidate runtime builds")
        .expect("safe bucket emits candidate runtime");
    assert_eq!(runtime.record_count(), 1);
    assert!(runtime.margin_for(0, &positive, &negative).expect("margin") > 0.0);

    let bucket = miner.bucket(7).expect("bucket exists");
    assert!(bucket.trust_quality_micro > 0);
    assert_eq!(bucket.trust_false_risk_micro, 0);
    assert!(bucket.trust_drift_micro > 0);
    assert!(bucket.trust_token_value_micro > 0);
}

#[test]
fn online_miner_decay_prevents_old_phase_energy_from_becoming_permanent() {
    let config = PhaseCenterOnlineMinerConfig {
        cells: 1,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 1,
    };
    let mut bucket = PhaseCenterOnlineBucket::new(9, config);
    let vector = [PhaseCenterCell { re: 1.0, im: 0.0 }];
    for _ in 0..=PHASE_CENTER_ONLINE_DECAY_INTERVAL {
        bucket.add(&vector, true);
    }
    let expected =
        PHASE_CENTER_ONLINE_DECAY_INTERVAL as f64 * PHASE_CENTER_ONLINE_DECAY_FACTOR + 1.0;
    assert!((bucket.positive_sum[0].re - expected).abs() < f64::EPSILON);
    assert!(bucket.positive_sum[0].re < bucket.positive_events as f64);
}

#[test]
fn online_miner_waits_for_false_margin_before_shadow_accept() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );

    miner
        .observe(11, &positive, true, false, 0, 0)
        .expect("seed positive");
    miner
        .observe(11, &negative, false, false, 0, 0)
        .expect("seed negative");
    let calibration_positive = miner
        .observe(11, &positive, true, false, 100, 300)
        .expect("positive-only calibration");
    assert!(calibration_positive.active_before_update);
    assert!(calibration_positive.calibration_event);
    assert!(!calibration_positive.raw_local_operator);

    let first_false_margin = miner
        .observe(11, &negative, false, false, 100, 300)
        .expect("first false margin calibrates threshold");
    assert!(first_false_margin.active_before_update);
    assert!(first_false_margin.calibration_event);
    assert!(!first_false_margin.raw_local_operator);
    assert!(!first_false_margin.false_accept);

    let accepted = miner
        .observe(11, &positive, true, false, 123, 456)
        .expect("future positive scored after false-margin calibration");
    assert!(!accepted.calibration_event);
    assert!(accepted.raw_local_operator);
    assert!(accepted.unique_cpu_accept_over_exact_cache);

    let summary = miner.summary();
    assert_eq!(summary.false_accepts, 0);
    assert_eq!(summary.rejected_bucket_count, 0);
    assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
}

#[test]
fn online_miner_quarantines_bucket_after_verified_false_accept() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );

    miner
        .observe(9, &positive, true, false, 0, 0)
        .expect("seed positive");
    miner
        .observe(9, &negative, false, false, 0, 0)
        .expect("seed negative");
    miner
        .observe(9, &positive, true, false, 0, 0)
        .expect("calibration positive");
    miner
        .observe(9, &negative, false, false, 0, 0)
        .expect("calibration negative");

    let unsafe_decision = miner
        .observe(9, &positive, false, false, 500, 700)
        .expect("unsafe event scored");
    assert!(unsafe_decision.raw_local_operator);
    assert!(unsafe_decision.false_accept);
    assert!(!unsafe_decision.local_operator_shadow_decision);
    assert!(!unsafe_decision.unique_cpu_accept_over_exact_cache);

    let summary = miner.summary();
    assert_eq!(summary.rejected_bucket_count, 1);
    assert_eq!(summary.candidate_bucket_count, 0);
    assert_eq!(summary.false_accepts, 1);
    let bucket = miner.bucket(9).expect("bucket exists");
    assert!(bucket.rejected);
    assert!(bucket.trust_false_risk_micro > 0);
    assert_eq!(
        bucket.learned_threshold_micro,
        unsafe_decision.margin_micro.saturating_add(1)
    );
    let after_quarantine_positive = miner
        .observe(9, &positive, true, false, 500, 700)
        .expect("quarantined bucket still learns but does not accept");
    assert!(after_quarantine_positive.active_before_update);
    assert!(!after_quarantine_positive.raw_local_operator);
    assert!(!after_quarantine_positive.local_operator_shadow_decision);
    assert!(!after_quarantine_positive.unique_cpu_accept_over_exact_cache);
    assert!(
        miner
            .candidate_runtime(9)
            .expect("candidate check")
            .is_none()
    );
}

#[test]
fn live_operator_store_tracks_mutable_budget_and_verifier_bound_export() {
    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 4,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .expect("valid live operator store");
    let positive = phase_vector_from_atom_ids([101, 201, 301], 16);
    let negative = phase_vector_from_atom_ids([101, 999, 998], 16);

    for (verified_safe_accept, vector) in [
        (true, &positive),
        (false, &negative),
        (true, &positive),
        (false, &negative),
    ] {
        store
            .observe(71, vector, verified_safe_accept, false, 10, 30)
            .expect("live event observed");
    }

    let future = store
        .observe(71, &positive, true, false, 55, 165)
        .expect("future event scored");
    assert!(future.active_before_update);
    assert!(future.local_operator_shadow_decision);
    assert!(future.unique_cpu_accept_over_exact_cache);
    assert!(!future.false_accept);

    let summary = store.summary();
    assert_eq!(summary.bucket_count, 1);
    assert_eq!(summary.candidate_bucket_count, 1);
    assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(summary.tokens_saved, 55);
    assert_eq!(summary.cost_saved_microusd, 165);
    assert_eq!(summary.false_accepts, 0);

    let snapshot = store.runtime_budget_snapshot();
    assert_eq!(snapshot.warm_route_count, 0);
    assert_eq!(snapshot.warm_profile_count, 1);
    assert_eq!(snapshot.hot_route_count, 0);
    assert_eq!(snapshot.hot_profile_count, 1);
    assert_eq!(snapshot.hot_route_profile_edges, 0);
    assert!(snapshot.warm_metadata_bytes_estimate > 0);
    assert!(snapshot.hot_runtime_bytes_estimate > 0);
    assert!(snapshot.hot_budget_passed());
    assert!(snapshot.warm_budget_passed());
    assert!(snapshot.product_runtime_budget_passed());

    let verifier_binding = test_verifier_binding();
    let mut packages = Vec::with_capacity(2);
    let capacity = packages.capacity();
    store
        .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
        .expect("verifier-bound candidates exported");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages.capacity(), capacity);
    assert_eq!(packages[0].bucket_id, 71);
    assert_eq!(packages[0].verifier_binding, verifier_binding);
    assert!(packages[0].verifier_binding.is_bound());
}

#[test]
fn online_miner_ranks_candidate_recovery_by_tokens_before_call_count() {
    let config = PhaseCenterOnlineMinerConfig {
        cells: 4,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 1,
        max_buckets: 4,
    };
    let mut miner = PhaseCenterOnlineMiner::new(config).expect("valid miner");

    let mut call_heavy = PhaseCenterOnlineBucket::new(10, config);
    call_heavy.positive_events = 1;
    call_heavy.negative_events = 1;
    call_heavy.events_seen = 2;
    call_heavy.unique_cpu_accepts_over_exact_cache = 50;
    call_heavy.tokens_saved = 1_000;
    call_heavy.cost_saved_microusd = 1_000;

    let mut token_heavy = PhaseCenterOnlineBucket::new(20, config);
    token_heavy.positive_events = 1;
    token_heavy.negative_events = 1;
    token_heavy.events_seen = 2;
    token_heavy.unique_cpu_accepts_over_exact_cache = 2;
    token_heavy.tokens_saved = 10_000;
    token_heavy.cost_saved_microusd = 10_000;

    let mut token_tie_accept_heavy = PhaseCenterOnlineBucket::new(30, config);
    token_tie_accept_heavy.positive_events = 1;
    token_tie_accept_heavy.negative_events = 1;
    token_tie_accept_heavy.events_seen = 2;
    token_tie_accept_heavy.unique_cpu_accepts_over_exact_cache = 3;
    token_tie_accept_heavy.tokens_saved = 10_000;
    token_tie_accept_heavy.cost_saved_microusd = 9_000;

    miner
        .buckets
        .extend([call_heavy, token_heavy, token_tie_accept_heavy]);

    assert_eq!(miner.candidate_bucket_ids_limited(3), vec![30, 20, 10]);
}

#[test]
fn live_operator_store_observes_numeric_route_atom_events() {
    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 4,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .expect("valid live operator store");
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let positive_atoms = [10_001, 20_001, 30_001];
    let negative_atoms = [10_001, 20_999, 30_999];
    let evidence = PhaseCenterHotRequestEvidence {
        verified_safe_accept: true,
        exact_cache_hit: false,
        tokens: 10,
        cost_microusd: 30,
    };
    assert_eq!(
        PhaseCenterLiveOperatorAtomEvent::new(700, 701, &positive_atoms, evidence).evidence(),
        evidence
    );

    for (verified_safe_accept, atom_ids, tokens, cost_microusd) in [
        (true, positive_atoms.as_slice(), 10, 30),
        (false, negative_atoms.as_slice(), 10, 30),
        (true, positive_atoms.as_slice(), 10, 30),
        (false, negative_atoms.as_slice(), 10, 30),
        (true, positive_atoms.as_slice(), 55, 165),
    ] {
        store
            .observe_atom_event(
                &mut encoder,
                PhaseCenterLiveOperatorAtomEvent::new(
                    700,
                    701,
                    atom_ids,
                    PhaseCenterHotRequestEvidence {
                        verified_safe_accept,
                        exact_cache_hit: false,
                        tokens,
                        cost_microusd,
                    },
                ),
            )
            .expect("numeric atom event observed");
    }

    let route = store.route_stats(700).expect("route stats exist");
    assert_eq!(store.route_count(), 1);
    assert_eq!(store.route_bucket_count(), 1);
    assert_eq!(route.route_bucket_count, 1);
    assert_eq!(route.events_seen, 5);
    assert_eq!(route.scored_events, 3);
    assert_eq!(route.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(route.tokens_saved, 55);
    assert_eq!(route.cost_saved_microusd, 165);
    assert_eq!(route.false_accepts, 0);

    let snapshot = store.runtime_budget_snapshot();
    assert_eq!(snapshot.warm_route_count, 1);
    assert_eq!(snapshot.warm_profile_count, 1);
    assert_eq!(snapshot.hot_route_count, 1);
    assert_eq!(snapshot.hot_profile_count, 1);
    assert_eq!(snapshot.hot_route_profile_edges, 1);
    assert!(snapshot.product_runtime_budget_passed());
}

#[test]
fn live_operator_store_exports_product_hot_runtime_without_package_roundtrip() {
    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 4,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .expect("valid live operator store");
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let positive_atoms = [80_001, 80_002, 80_003];
    let negative_atoms = [80_001, 80_999, 80_998];

    for (verified_safe_accept, atom_ids, tokens, cost_microusd) in [
        (true, positive_atoms.as_slice(), 10, 30),
        (false, negative_atoms.as_slice(), 10, 30),
        (true, positive_atoms.as_slice(), 10, 30),
        (false, negative_atoms.as_slice(), 10, 30),
        (true, positive_atoms.as_slice(), 55, 165),
    ] {
        store
            .observe_atom_event(
                &mut encoder,
                PhaseCenterLiveOperatorAtomEvent::new(
                    8_000,
                    8_001,
                    atom_ids,
                    PhaseCenterHotRequestEvidence {
                        verified_safe_accept,
                        exact_cache_hit: false,
                        tokens,
                        cost_microusd,
                    },
                ),
            )
            .expect("numeric atom event observed");
    }

    let (hot_runtime, route_table) = store
        .candidate_hot_runtime_and_route_table()
        .expect("direct product hot runtime builds")
        .expect("candidate hot runtime exists");
    assert_eq!(hot_runtime.profile_count(), 1);
    assert_eq!(route_table.route_count(), 1);
    assert_eq!(route_table.profile_edge_count(), 1);
    let route_index = route_table
        .resolve_route_index(8_000)
        .expect("route index exists");
    let mut scratch = PhaseCenterHotScratch::new(16, 1).expect("valid scratch");
    let decisions = hot_runtime
        .score_hot_request_candidates(
            &route_table,
            PhaseCenterHotRequest::new(route_index, &positive_atoms),
            &mut scratch,
        )
        .expect("product hot request scores");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].profile_id, 8_001);
    assert!(decisions[0].score_candidate);
    assert!(decisions[0].verifier_required);
    assert!(!decisions[0].local_accept);

    let snapshot = store.runtime_budget_snapshot();
    assert_eq!(snapshot.hot_route_count, 1);
    assert_eq!(snapshot.hot_profile_count, 1);
    assert_eq!(snapshot.hot_route_profile_edges, 1);
    assert!(snapshot.product_runtime_budget_passed());
}

#[test]
fn live_operator_store_rejects_false_accept_before_export() {
    let mut store = PhaseCenterLiveOperatorStore::new(PhaseCenterLiveOperatorStoreConfig {
        miner: PhaseCenterOnlineMinerConfig {
            cells: 16,
            min_bucket_events: 2,
            threshold_floor_micro: 1,
            calibration_events: 2,
            max_buckets: 4,
        },
        memory: PhaseCenterOperatorMemoryConfig {
            max_hot_profiles_per_worker: 4,
            max_hot_bytes_per_worker: 64 * 1024,
            max_warm_profiles_per_process: 4,
            max_profiles_per_route: 2,
            max_route_top_k: 2,
            min_tokens_saved: 1,
            min_accept_rate_milli: 1,
            false_accepts_must_be_zero: true,
        },
    })
    .expect("valid live operator store");
    let positive = phase_vector_from_atom_ids([401, 501, 601], 16);
    let negative = phase_vector_from_atom_ids([401, 999, 998], 16);

    for (verified_safe_accept, vector) in [
        (true, &positive),
        (false, &negative),
        (true, &positive),
        (false, &negative),
    ] {
        store
            .observe(72, vector, verified_safe_accept, false, 0, 0)
            .expect("live event observed");
    }

    let false_accept = store
        .observe(72, &positive, false, false, 55, 165)
        .expect("unsafe event scored");
    assert!(false_accept.raw_local_operator);
    assert!(false_accept.false_accept);
    assert!(!false_accept.local_operator_shadow_decision);

    let summary = store.summary();
    assert_eq!(summary.rejected_bucket_count, 1);
    assert_eq!(summary.candidate_bucket_count, 0);
    assert_eq!(summary.false_accepts, 1);

    let snapshot = store.runtime_budget_snapshot();
    assert_eq!(snapshot.warm_profile_count, 1);
    assert_eq!(snapshot.hot_profile_count, 0);
    assert_eq!(snapshot.hot_bytes_estimate, 0);

    let mut packages = Vec::new();
    store
        .candidate_packages_into_with_verifier(test_verifier_binding(), &mut packages)
        .expect("export stays safe");
    assert!(packages.is_empty());
}

#[test]
fn online_event_adapter_emits_verifier_bound_nwpc_package() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );
    let positive_event = PhaseCenterOnlineEvent {
        bucket_id: 11,
        vector: &positive,
        verified_safe_accept: true,
        exact_cache_hit: false,
        tokens: 100,
        cost_microusd: 300,
    };
    let negative_event = PhaseCenterOnlineEvent {
        bucket_id: 11,
        vector: &negative,
        verified_safe_accept: false,
        exact_cache_hit: false,
        tokens: 100,
        cost_microusd: 300,
    };

    miner
        .observe_event(positive_event)
        .expect("seed positive event");
    miner
        .observe_event(negative_event)
        .expect("seed negative event");
    miner
        .observe_event(positive_event)
        .expect("calibration positive event");
    miner
        .observe_event(negative_event)
        .expect("calibration negative event");
    let decision = miner
        .observe_event(positive_event)
        .expect("future positive event scored");
    assert!(decision.local_operator_shadow_decision);
    assert!(decision.unique_cpu_accept_over_exact_cache);

    let verifier_binding = test_verifier_binding();
    let package = miner
        .candidate_package_bytes_with_verifier(11, verifier_binding)
        .expect("candidate package builds")
        .expect("safe bucket emits package");
    assert_eq!(package.bucket_id, 11);
    assert!(package.threshold_micro > 0);
    assert_eq!(package.verifier_binding, verifier_binding);
    assert!(package.verifier_binding.is_bound());
    assert!(
        package
            .package_bytes
            .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
    );
    assert_eq!(package.package_info.cells, 16);
    assert_eq!(package.package_info.record_count, 1);
    assert_eq!(
        package.package_info.serialized_len,
        package.package_bytes.len()
    );
    let loaded = PhaseCenterFlatRuntime::from_bytes(&package.package_bytes)
        .expect("candidate package loads");
    assert!(loaded.margin_for(0, &positive, &negative).expect("margin") > 0.0);
}

#[test]
fn shadow_ready_package_can_be_handed_to_external_admission_before_accept() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(["family:wait", "state:yielded"], 16);
    let negative = phase_vector_from_atoms(["family:write", "state:running"], 16);

    miner
        .observe(7, &positive, true, false, 0, 0)
        .expect("seed positive");
    miner
        .observe(7, &negative, false, false, 0, 0)
        .expect("seed negative");
    miner
        .observe(7, &positive, true, false, 0, 0)
        .expect("calibration positive");
    miner
        .observe(7, &negative, false, false, 0, 0)
        .expect("calibration negative");

    assert!(
        miner
            .candidate_package_bytes(7)
            .expect("candidate query")
            .is_none()
    );
    let package = miner
        .shadow_ready_package_bytes(7)
        .expect("shadow package query")
        .expect("shadow-ready package");
    assert_eq!(package.bucket_id, 7);
    assert_eq!(package.package_info.record_count, 1);
}

#[test]
fn online_stream_api_reuses_caller_buffers() {
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );
    let positive_event = PhaseCenterOnlineEvent {
        bucket_id: 13,
        vector: &positive,
        verified_safe_accept: true,
        exact_cache_hit: false,
        tokens: 17,
        cost_microusd: 23,
    };
    let negative_event = PhaseCenterOnlineEvent {
        bucket_id: 13,
        vector: &negative,
        verified_safe_accept: false,
        exact_cache_hit: false,
        tokens: 17,
        cost_microusd: 23,
    };
    let mut decisions = Vec::with_capacity(8);
    let decision_capacity = decisions.capacity();
    miner
        .observe_events_into(
            [
                positive_event,
                negative_event,
                positive_event,
                negative_event,
                positive_event,
            ],
            &mut decisions,
        )
        .expect("stream events accepted");

    assert_eq!(decisions.len(), 5);
    assert_eq!(decisions.capacity(), decision_capacity);
    assert!(decisions[4].local_operator_shadow_decision);
    assert!(decisions[4].unique_cpu_accept_over_exact_cache);

    let mut packages = Vec::with_capacity(4);
    let package_capacity = packages.capacity();
    miner
        .candidate_packages_into(&mut packages)
        .expect("candidate packages emitted into caller buffer");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages.capacity(), package_capacity);
    assert_eq!(packages[0].bucket_id, 13);
    assert!(!packages[0].verifier_binding.is_bound());
    let verifier_binding = test_verifier_binding();
    miner
        .candidate_packages_into_with_verifier(verifier_binding, &mut packages)
        .expect("verifier-bound candidate packages emitted into caller buffer");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages.capacity(), package_capacity);
    assert_eq!(packages[0].bucket_id, 13);
    assert_eq!(packages[0].verifier_binding, verifier_binding);
    assert!(packages[0].verifier_binding.is_bound());
    assert!(
        packages[0]
            .package_bytes
            .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
    );
}

#[test]
fn online_atom_adapter_learns_then_emits_candidate_package() {
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let capacity_before = encoder.scratch_capacity();

    miner
        .observe_atoms(
            &mut encoder,
            17,
            ["family:test_output_parse", "state:exit0", "result:pass"],
            true,
            false,
            10,
            30,
        )
        .expect("seed positive atoms");
    miner
        .observe_atoms(
            &mut encoder,
            17,
            ["family:test_output_parse", "state:panic", "result:fail"],
            false,
            false,
            10,
            30,
        )
        .expect("seed negative atoms");
    miner
        .observe_atoms(
            &mut encoder,
            17,
            ["family:test_output_parse", "state:exit0", "result:pass"],
            true,
            false,
            10,
            30,
        )
        .expect("calibration positive atoms");
    miner
        .observe_atoms(
            &mut encoder,
            17,
            ["family:test_output_parse", "state:panic", "result:fail"],
            false,
            false,
            10,
            30,
        )
        .expect("calibration negative atoms");
    let decision = miner
        .observe_atoms(
            &mut encoder,
            17,
            ["family:test_output_parse", "state:exit0", "result:pass"],
            true,
            false,
            25,
            75,
        )
        .expect("future positive atoms");

    assert_eq!(encoder.scratch_capacity(), capacity_before);
    assert!(decision.local_operator_shadow_decision);
    assert!(decision.unique_cpu_accept_over_exact_cache);
    assert!(!decision.false_accept);
    let summary = miner.summary();
    assert_eq!(summary.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(summary.tokens_saved, 25);
    assert_eq!(summary.cost_saved_microusd, 75);
    assert_eq!(summary.false_accepts, 0);

    let verifier_binding = test_verifier_binding();
    let package = miner
        .candidate_package_bytes_with_verifier(17, verifier_binding)
        .expect("candidate package builds")
        .expect("safe atom bucket emits package");
    assert_eq!(package.bucket_id, 17);
    assert_eq!(package.verifier_binding, verifier_binding);
    assert_eq!(package.package_info.record_count, 1);
    assert!(
        package
            .package_bytes
            .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
    );
}

#[test]
fn online_atom_id_adapter_learns_then_emits_verifier_bound_candidate_package() {
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");
    let capacity_before = encoder.scratch_capacity();

    for (verified_safe_accept, atom_ids) in [
        (true, [10_001, 20_001, 30_001]),
        (false, [10_001, 20_999, 30_999]),
        (true, [10_001, 20_001, 30_001]),
        (false, [10_001, 20_999, 30_999]),
    ] {
        miner
            .observe_atom_ids(
                &mut encoder,
                19,
                atom_ids,
                verified_safe_accept,
                false,
                10,
                30,
            )
            .expect("atom ids observed");
    }

    let decision = miner
        .observe_atom_ids(
            &mut encoder,
            19,
            [10_001, 20_001, 30_001],
            true,
            false,
            25,
            75,
        )
        .expect("future positive atom ids");

    assert_eq!(encoder.scratch_capacity(), capacity_before);
    assert!(decision.local_operator_shadow_decision);
    assert!(decision.unique_cpu_accept_over_exact_cache);
    assert!(!decision.false_accept);

    let verifier_binding = test_verifier_binding();
    let package = miner
        .candidate_package_bytes_with_verifier(19, verifier_binding)
        .expect("candidate package builds")
        .expect("safe atom-id bucket emits package");
    assert_eq!(package.bucket_id, 19);
    assert_eq!(package.verifier_binding, verifier_binding);
    assert!(package.verifier_binding.is_bound());
    assert!(
        package
            .package_bytes
            .starts_with(&PHASE_CENTER_RUNTIME_PACKAGE_MAGIC)
    );
}

#[test]
fn online_miner_exports_only_safe_buckets_to_hot_runtime() {
    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let mut miner = PhaseCenterOnlineMiner::new(PhaseCenterOnlineMinerConfig {
        cells: 16,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    })
    .expect("valid online miner");

    for (bucket_id, verified_safe_accept, atoms) in [
        (
            21,
            true,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
        (
            21,
            false,
            ["family:test_output_parse", "state:panic", "result:fail"],
        ),
        (
            21,
            true,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
        (
            21,
            false,
            ["family:test_output_parse", "state:panic", "result:fail"],
        ),
        (
            21,
            true,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
        (
            22,
            true,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
        (
            22,
            false,
            ["family:test_output_parse", "state:panic", "result:fail"],
        ),
        (
            22,
            true,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
        (
            22,
            false,
            ["family:test_output_parse", "state:panic", "result:fail"],
        ),
        (
            22,
            false,
            ["family:test_output_parse", "state:exit0", "result:pass"],
        ),
    ] {
        miner
            .observe_atoms(
                &mut encoder,
                bucket_id,
                atoms,
                verified_safe_accept,
                false,
                10,
                30,
            )
            .expect("atoms observed");
    }

    assert_eq!(miner.summary().candidate_bucket_count, 1);
    assert_eq!(miner.summary().rejected_bucket_count, 1);
    let hot = miner
        .candidate_hot_runtime()
        .expect("hot runtime builds")
        .expect("safe bucket exported");
    assert_eq!(hot.profile_count(), 1);
    assert_eq!(hot.profile_id_at(0), Some(21));
    assert_eq!(hot.profile_id_at(1), None);
    assert_eq!(hot.resolve_profile_index(21), Some(0));
    assert_eq!(hot.resolve_profile_index(22), None);

    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let profile_index = hot.resolve_profile_index(21).expect("resolved profile");
    let decision = hot
        .score_profile(profile_index, &positive)
        .expect("hot score");
    assert_eq!(decision.profile_id, 21);
    assert!(decision.local_operator);
}

#[test]
fn promotion_gate_allows_only_verified_future_shadow_savings() {
    let summary = PhaseCenterOnlineSummary {
        unique_cpu_accepts_over_exact_cache: 3,
        tokens_saved: 1200,
        cost_saved_microusd: 3600,
        false_accepts: 0,
        ..PhaseCenterOnlineSummary::default()
    };
    let evidence =
        PhaseCenterPromotionEvidence::from_online_summary(summary, 10, 0, true, true, false)
            .with_verifier_binding(test_verifier_binding())
            .with_threshold_policy(passing_threshold_policy());
    assert_eq!(
        evidence.evaluate(),
        PhaseCenterPromotionDecision {
            eligible: true,
            blocker: None,
        }
    );
}

#[test]
fn promotion_gate_blocks_unsafe_or_unproven_candidates() {
    let safe = PhaseCenterPromotionEvidence {
        future_shadow_events: 10,
        unique_cpu_accepts_over_exact_cache: 3,
        tokens_saved: 1200,
        cost_saved_microusd: 3600,
        false_accepts: 0,
        runtime_margin_parity_mismatches: 0,
        verifier_binding: test_verifier_binding(),
        threshold_policy: passing_threshold_policy(),
        exact_cache_overlap_excluded: true,
        token_cost_denominator_present: true,
        local_accept_enabled: false,
    };
    assert_eq!(
        PhaseCenterPromotionEvidence {
            verifier_binding: PhaseCenterVerifierBinding::default(),
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(PhaseCenterPromotionBlocker::MissingVerifierBinding)
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            threshold_policy: PhaseCenterThresholdPolicyEvidence::default(),
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(
            PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration
        )
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            false_accepts: 1,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(PhaseCenterPromotionBlocker::FalseAccepts)
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            runtime_margin_parity_mismatches: 1,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(PhaseCenterPromotionBlocker::RuntimeParityMismatch)
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            exact_cache_overlap_excluded: false,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(
            PhaseCenterPromotionBlocker::ExactCacheOverlapNotExcluded
        )
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            token_cost_denominator_present: false,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(
            PhaseCenterPromotionBlocker::MissingTokenCostDenominator
        )
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            unique_cpu_accepts_over_exact_cache: 0,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(
            PhaseCenterPromotionBlocker::NoUniqueAcceptsOverExactCache
        )
    );
    assert_eq!(
        PhaseCenterPromotionEvidence {
            local_accept_enabled: true,
            ..safe
        }
        .evaluate(),
        PhaseCenterPromotionDecision::blocked(
            PhaseCenterPromotionBlocker::LocalAcceptAlreadyEnabled
        )
    );
}

#[test]
fn online_miner_reports_threshold_policy_evidence() {
    let config = PhaseCenterOnlineMinerConfig {
        cells: 4,
        min_bucket_events: 2,
        threshold_floor_micro: 1,
        calibration_events: 2,
        max_buckets: 4,
    };
    let miner = PhaseCenterOnlineMiner {
        config,
        buckets: vec![
            PhaseCenterOnlineBucket {
                bucket_id: 10,
                positive_sum: vec![PhaseCenterCell::default(); config.cells],
                negative_sum: vec![PhaseCenterCell::default(); config.cells],
                positive_events: 3,
                negative_events: 2,
                events_seen: 6,
                scored_events: 4,
                calibration_events_seen: 2,
                learned_threshold_micro: 77,
                max_calibration_false_margin_micro: Some(76),
                local_operator_shadow_decisions: 1,
                unique_cpu_accepts_over_exact_cache: 1,
                tokens_saved: 120,
                cost_saved_microusd: 360,
                false_accepts: 0,
                rejected: false,
                trust_quality_micro: 0,
                trust_false_risk_micro: 0,
                trust_drift_micro: 0,
                trust_token_value_micro: 0,
            },
            PhaseCenterOnlineBucket {
                bucket_id: 11,
                positive_sum: vec![PhaseCenterCell::default(); config.cells],
                negative_sum: vec![PhaseCenterCell::default(); config.cells],
                positive_events: 3,
                negative_events: 2,
                events_seen: 6,
                scored_events: 4,
                calibration_events_seen: 2,
                learned_threshold_micro: 77,
                max_calibration_false_margin_micro: None,
                local_operator_shadow_decisions: 1,
                unique_cpu_accepts_over_exact_cache: 1,
                tokens_saved: 120,
                cost_saved_microusd: 360,
                false_accepts: 0,
                rejected: false,
                trust_quality_micro: 0,
                trust_false_risk_micro: 0,
                trust_drift_micro: 0,
                trust_token_value_micro: 0,
            },
        ],
    };

    let evidence = miner.threshold_policy_evidence();
    assert_eq!(evidence.candidate_bucket_count, 2);
    assert_eq!(evidence.auto_calibrated_bucket_count, 1);
    assert!(!evidence.automatic_calibration_passed());
    assert!(!evidence.promotion_policy_passed());

    let miner = PhaseCenterOnlineMiner {
        buckets: vec![PhaseCenterOnlineBucket {
            max_calibration_false_margin_micro: Some(76),
            ..miner.buckets[0].clone()
        }],
        ..miner
    };
    let evidence = miner.threshold_policy_evidence();
    assert_eq!(evidence.candidate_bucket_count, 1);
    assert_eq!(evidence.auto_calibrated_bucket_count, 1);
    assert!(evidence.automatic_calibration_passed());
    assert!(evidence.promotion_policy_passed());
}

#[test]
fn operator_memory_admits_only_promoted_profiles() {
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 4,
        max_profiles_per_route: 2,
        max_route_top_k: 1,
        min_tokens_saved: 100,
        min_accept_rate_milli: 100,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");

    let unsafe_decision = memory.admit(PhaseCenterOperatorAdmission {
        route_id: 7,
        profile_id: 1,
        evidence: PhaseCenterPromotionEvidence {
            threshold_policy: PhaseCenterThresholdPolicyEvidence::default(),
            ..promotion_evidence(10, 3, 120, 360)
        },
        runtime_bytes_estimate: 256,
        last_seen_tick: 1,
    });
    assert_eq!(
        unsafe_decision,
        PhaseCenterOperatorAdmissionDecision::blocked(
            PhaseCenterOperatorAdmissionBlocker::PromotionBlocked(
                PhaseCenterPromotionBlocker::MissingAutomaticThresholdCalibration
            )
        )
    );
    assert_eq!(memory.warm_profile_count(), 0);

    let low_value_decision = memory.admit(PhaseCenterOperatorAdmission {
        route_id: 7,
        profile_id: 2,
        evidence: promotion_evidence(10, 3, 99, 360),
        runtime_bytes_estimate: 256,
        last_seen_tick: 2,
    });
    assert_eq!(
        low_value_decision,
        PhaseCenterOperatorAdmissionDecision::blocked(
            PhaseCenterOperatorAdmissionBlocker::BelowMinTokensSaved
        )
    );
    assert_eq!(memory.warm_profile_count(), 0);

    let admitted = memory.admit(PhaseCenterOperatorAdmission {
        route_id: 7,
        profile_id: 3,
        evidence: promotion_evidence(10, 3, 120, 360),
        runtime_bytes_estimate: 256,
        last_seen_tick: 3,
    });
    assert_eq!(
        admitted,
        PhaseCenterOperatorAdmissionDecision {
            admitted: true,
            blocker: None
        }
    );
    assert_eq!(memory.warm_profile_count(), 1);
    assert_eq!(memory.route(7).expect("route exists").profile_count(), 1);
}

#[test]
fn operator_memory_bounds_route_top_k_and_warm_profiles() {
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 2,
        max_profiles_per_route: 2,
        max_route_top_k: 1,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");

    for (profile_id, route_id, tokens_saved, tick) in [
        (1, 10, 100, 1),
        (2, 10, 300, 2),
        (3, 10, 200, 3),
        (4, 20, 400, 4),
    ] {
        memory.admit(PhaseCenterOperatorAdmission {
            route_id,
            profile_id,
            evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
            runtime_bytes_estimate: 256,
            last_seen_tick: tick,
        });
    }

    assert_eq!(memory.warm_profile_count(), 2);
    assert_eq!(
        memory.route(10).expect("route 10 exists").profile_count(),
        1
    );
    assert_eq!(
        memory.route(20).expect("route 20 exists").profile_count(),
        1
    );

    let mut top = Vec::new();
    memory.route_top_k_into(10, &mut top);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].profile_id, 2);
    memory.route_top_k_into(20, &mut top);
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].profile_id, 4);
    memory.route_top_k_into(30, &mut top);
    assert!(top.is_empty());
}

#[test]
fn hot_route_plan_scores_bounded_top_k_without_registry_scan() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let p3 = phase_vector_from_atom_ids([3, 33], 16);
    let n3 = phase_vector_from_atom_ids([3, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.clone().into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.clone().into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p3.clone().into_boxed_slice(),
                negative_center: n3.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2, 4], &[1, 1, 1])
        .expect("valid hot runtime");
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 4,
        max_profiles_per_route: 3,
        max_route_top_k: 2,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");

    for (profile_id, route_id, tokens_saved, tick) in
        [(1, 10, 100, 1), (2, 10, 300, 2), (4, 20, 400, 3)]
    {
        assert!(
            memory
                .admit(PhaseCenterOperatorAdmission {
                    route_id,
                    profile_id,
                    evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                    runtime_bytes_estimate: 256,
                    last_seen_tick: tick,
                })
                .admitted
        );
    }

    let plan = memory
        .hot_route_plan(&hot, 10)
        .expect("route plan builds")
        .expect("route has profiles");
    assert_eq!(plan.route_id(), 10);
    assert_eq!(plan.profile_count(), 2);
    assert_eq!(plan.profile_indexes(), &[1, 0]);
    assert!(plan.bytes_estimate() >= 2 * std::mem::size_of::<usize>());
    assert!(
        memory
            .hot_route_plan(&hot, 30)
            .expect("missing route")
            .is_none()
    );

    let mut decisions = Vec::with_capacity(8);
    let decision_capacity = decisions.capacity();
    hot.score_route_plan_into(&plan, &p2, &mut decisions)
        .expect("route plan scores");
    assert_eq!(decisions.capacity(), decision_capacity);
    assert_eq!(decisions.len(), 2);
    assert_eq!(
        decisions[0],
        hot.score_profile(1, &p2).expect("manual profile 2 score")
    );
    assert_eq!(
        decisions[1],
        hot.score_profile(0, &p2).expect("manual profile 1 score")
    );
    assert_eq!(decisions[0].profile_id, 2);
    assert!(decisions[0].local_operator);
}

#[test]
fn hot_route_plan_rejects_profile_missing_from_hot_runtime() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: p1.into_boxed_slice(),
            negative_center: n1.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1], &[1]).expect("valid hot runtime");
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 2,
        max_profiles_per_route: 2,
        max_route_top_k: 1,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");
    assert!(
        memory
            .admit(PhaseCenterOperatorAdmission {
                route_id: 10,
                profile_id: 2,
                evidence: promotion_evidence(10, 5, 100, 300),
                runtime_bytes_estimate: 256,
                last_seen_tick: 1,
            })
            .admitted
    );

    assert_eq!(
        memory.hot_route_plan(&hot, 10),
        Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
    );
}

#[test]
fn hot_route_table_scores_route_index_without_warm_memory() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let p4 = phase_vector_from_atom_ids([4, 44], 16);
    let n4 = phase_vector_from_atom_ids([4, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.clone().into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.clone().into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p4.clone().into_boxed_slice(),
                negative_center: n4.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2, 4], &[1, 1, 1])
        .expect("valid hot runtime");
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 4,
        max_profiles_per_route: 3,
        max_route_top_k: 2,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");
    for (profile_id, route_id, tokens_saved, tick) in
        [(4, 20, 400, 1), (1, 10, 100, 2), (2, 10, 300, 3)]
    {
        assert!(
            memory
                .admit(PhaseCenterOperatorAdmission {
                    route_id,
                    profile_id,
                    evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                    runtime_bytes_estimate: 256,
                    last_seen_tick: tick,
                })
                .admitted
        );
    }

    let table = memory.hot_route_table(&hot).expect("route table builds");
    assert_eq!(table.route_count(), 2);
    assert_eq!(table.route_id_at(0), Some(10));
    assert_eq!(table.route_id_at(1), Some(20));
    assert_eq!(table.route_id_at(2), None);
    assert_eq!(table.resolve_route_index(10), Some(0));
    assert_eq!(table.resolve_route_index(20), Some(1));
    assert_eq!(table.resolve_route_index(30), None);
    assert!(table.bytes_estimate() >= table.route_count() * std::mem::size_of::<usize>());

    let route_index = table.resolve_route_index(10).expect("route 10 index");
    let plan = table.route_plan_at(route_index).expect("route 10 plan");
    let mut by_plan = Vec::with_capacity(4);
    let mut by_route_index = Vec::with_capacity(4);
    let route_capacity = by_route_index.capacity();
    hot.score_route_plan_into(plan, &p2, &mut by_plan)
        .expect("plan scores");
    hot.score_route_index_into(&table, route_index, &p2, &mut by_route_index)
        .expect("route index scores");
    assert_eq!(by_route_index.capacity(), route_capacity);
    assert_eq!(by_route_index, by_plan);
    assert_eq!(by_route_index[0].profile_id, 2);
    assert!(by_route_index[0].local_operator);

    assert_eq!(
        hot.score_route_index_into(&table, 99, &p2, &mut by_route_index),
        Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
    );
}

#[test]
fn operator_memory_runtime_budget_snapshot_reports_hot_and_warm_bounds() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
        .expect("valid hot runtime");
    let mut memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 2,
        max_hot_bytes_per_worker: 64 * 1024,
        max_warm_profiles_per_process: 4,
        max_profiles_per_route: 2,
        max_route_top_k: 2,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid operator memory");

    for (profile_id, route_id, tokens_saved, tick) in [(1, 10, 100, 1), (2, 10, 200, 2)] {
        assert!(
            memory
                .admit(PhaseCenterOperatorAdmission {
                    route_id,
                    profile_id,
                    evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                    runtime_bytes_estimate: 512,
                    last_seen_tick: tick,
                })
                .admitted
        );
    }

    let table = memory.hot_route_table(&hot).expect("route table builds");
    let snapshot = memory.runtime_budget_snapshot(&hot, &table);
    assert_eq!(snapshot.max_hot_profiles_per_worker, 2);
    assert_eq!(snapshot.max_warm_profiles_per_process, 4);
    assert_eq!(snapshot.warm_route_count, 1);
    assert_eq!(snapshot.warm_profile_count, 2);
    assert_eq!(snapshot.warm_runtime_bytes_estimate, 1024);
    assert!(snapshot.warm_bytes_estimate >= snapshot.warm_runtime_bytes_estimate);
    assert_eq!(snapshot.hot_route_count, 1);
    assert_eq!(snapshot.hot_profile_count, 2);
    assert_eq!(snapshot.hot_route_profile_edges, 2);
    assert_eq!(
        snapshot.hot_bytes_estimate,
        snapshot
            .hot_runtime_bytes_estimate
            .saturating_add(snapshot.hot_route_table_bytes_estimate)
    );
    assert!(snapshot.hot_budget_passed());
    assert!(snapshot.warm_budget_passed());
    assert!(snapshot.product_runtime_budget_passed());

    let mut tight_memory = PhaseCenterOperatorMemory::new(PhaseCenterOperatorMemoryConfig {
        max_hot_profiles_per_worker: 1,
        max_hot_bytes_per_worker: 1,
        max_warm_profiles_per_process: 4,
        max_profiles_per_route: 2,
        max_route_top_k: 2,
        min_tokens_saved: 1,
        min_accept_rate_milli: 1,
        false_accepts_must_be_zero: true,
    })
    .expect("valid tight operator memory");
    for (profile_id, route_id, tokens_saved, tick) in [(1, 10, 100, 1), (2, 10, 200, 2)] {
        assert!(
            tight_memory
                .admit(PhaseCenterOperatorAdmission {
                    route_id,
                    profile_id,
                    evidence: promotion_evidence(10, 5, tokens_saved, tokens_saved * 3),
                    runtime_bytes_estimate: 512,
                    last_seen_tick: tick,
                })
                .admitted
        );
    }
    let tight_table = tight_memory
        .hot_route_table(&hot)
        .expect("tight route table still builds for explicit audit");
    let tight_snapshot = tight_memory.runtime_budget_snapshot(&hot, &tight_table);
    assert!(!tight_snapshot.hot_profile_budget_passed);
    assert!(!tight_snapshot.hot_byte_budget_passed);
    assert!(!tight_snapshot.hot_budget_passed());
    assert!(!tight_snapshot.product_runtime_budget_passed());
}

#[test]
fn hot_route_table_rejects_duplicate_route_plans() {
    let table = PhaseCenterHotRouteTable::from_plans([
        PhaseCenterHotRoutePlan::new(7, vec![0])
            .expect("plan builds")
            .expect("plan exists"),
        PhaseCenterHotRoutePlan::new(7, vec![1])
            .expect("plan builds")
            .expect("plan exists"),
    ]);
    assert_eq!(table, Err(PhaseCenterRuntimeError::InvalidRuntimePackage));
}

#[test]
fn hot_route_table_scores_atom_ids_with_reused_buffers() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.clone().into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
        .expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(10, [2, 1])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let route_index = table.resolve_route_index(10).expect("route index");

    let mut expected = Vec::with_capacity(4);
    hot.score_route_index_into(&table, route_index, &p2, &mut expected)
        .expect("vector route score");

    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let encoder_capacity = encoder.scratch_capacity();
    let mut decisions = Vec::with_capacity(4);
    let decision_capacity = decisions.capacity();
    hot.score_route_atom_ids_into(&table, route_index, &mut encoder, [2, 22], &mut decisions)
        .expect("atom-id route score");

    assert_eq!(encoder.scratch_capacity(), encoder_capacity);
    assert_eq!(decisions.capacity(), decision_capacity);
    assert_eq!(decisions, expected);
    assert_eq!(decisions[0].profile_id, 2);
    assert!(decisions[0].local_operator);

    assert_eq!(
        hot.score_route_atom_ids_into(&table, 99, &mut encoder, [2, 22], &mut decisions,),
        Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
    );
}

#[test]
fn hot_candidate_scoring_requires_verifier_and_never_local_accepts() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
        .expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(10, [2, 1])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let route_index = table.resolve_route_index(10).expect("route index");

    let mut encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let encoder_capacity = encoder.scratch_capacity();
    let mut candidates = Vec::with_capacity(4);
    let candidate_capacity = candidates.capacity();
    hot.score_route_atom_id_candidates_into(
        &table,
        route_index,
        &mut encoder,
        [2, 22],
        &mut candidates,
    )
    .expect("candidate score");

    assert_eq!(encoder.scratch_capacity(), encoder_capacity);
    assert_eq!(candidates.capacity(), candidate_capacity);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].profile_id, 2);
    assert!(candidates[0].score_candidate);
    assert!(candidates[0].verifier_required);
    assert!(!candidates[0].local_accept);

    hot.score_route_atom_id_candidates_into(
        &table,
        route_index,
        &mut encoder,
        [2, 99],
        &mut candidates,
    )
    .expect("negative candidate score");
    assert_eq!(candidates[0].profile_id, 2);
    assert!(!candidates[0].score_candidate);
    assert!(!candidates[0].verifier_required);
    assert!(!candidates[0].local_accept);

    assert_eq!(
            hot.score_route_atom_id_candidates_into(
                &table,
                99,
                &mut encoder,
                [2, 22],
                &mut candidates,
            ),
            Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
        );
}

#[test]
fn hot_request_adapter_scores_candidates_with_reused_scratch() {
    let p1 = phase_vector_from_atom_ids([1, 11], 16);
    let n1 = phase_vector_from_atom_ids([1, 99], 16);
    let p2 = phase_vector_from_atom_ids([2, 22], 16);
    let n2 = phase_vector_from_atom_ids([2, 99], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: p1.into_boxed_slice(),
                negative_center: n1.into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: p2.into_boxed_slice(),
                negative_center: n2.into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[1, 2], &[1, 1])
        .expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(10, [2, 1])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let route_index = table.resolve_route_index(10).expect("route index");
    let positive_atoms = [2, 22];
    let negative_atoms = [2, 99];

    let mut scratch = PhaseCenterHotScratch::new(16, 4).expect("valid scratch");
    let encoder_capacity = scratch.encoder_scratch_capacity();
    let candidate_capacity = scratch.candidate_capacity();
    let score_capacity = scratch.score_capacity();
    let atom_cache_capacity = scratch.atom_cache_capacity();
    let mut reference_encoder = PhaseCenterAtomEncoder::new(16).expect("valid encoder");
    let mut reference = Vec::with_capacity(4);
    hot.score_route_atom_id_candidates_into(
        &table,
        route_index,
        &mut reference_encoder,
        positive_atoms,
        &mut reference,
    )
    .expect("reference candidate score");

    {
        let candidates = hot
            .score_hot_request_candidates(
                &table,
                PhaseCenterHotRequest::new(route_index, &positive_atoms),
                &mut scratch,
            )
            .expect("hot request scores");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].profile_id, 2);
        assert!(candidates[0].score_candidate);
        assert!(candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);
        assert_eq!(candidates, reference);
    }
    assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
    assert_eq!(scratch.candidate_capacity(), candidate_capacity);
    assert_eq!(scratch.score_capacity(), score_capacity);
    assert_eq!(scratch.atom_cache_capacity(), atom_cache_capacity);
    assert_eq!(scratch.cached_atom_rows(), positive_atoms.len());

    let positive_vector = phase_vector_from_atom_ids(positive_atoms, 16);
    {
        let candidates = hot
            .score_prepared_hot_request_candidates(
                &table,
                PhaseCenterPreparedHotRequest::new(route_index, &positive_vector),
                &mut scratch,
            )
            .expect("prepared hot request scores");
        assert_eq!(candidates, reference);
        assert!(candidates[0].score_candidate);
        assert!(!candidates[0].local_accept);
    }
    assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
    assert_eq!(scratch.candidate_capacity(), candidate_capacity);
    assert_eq!(scratch.score_capacity(), score_capacity);

    {
        let candidates = hot
            .score_hot_request_candidates(
                &table,
                PhaseCenterHotRequest::new(route_index, &negative_atoms),
                &mut scratch,
            )
            .expect("negative hot request scores");
        assert_eq!(candidates[0].profile_id, 2);
        assert!(!candidates[0].score_candidate);
        assert!(!candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);
    }
    assert_eq!(scratch.encoder_scratch_capacity(), encoder_capacity);
    assert_eq!(scratch.candidate_capacity(), candidate_capacity);
    assert_eq!(scratch.score_capacity(), score_capacity);
    assert_eq!(scratch.atom_cache_capacity(), atom_cache_capacity);
    assert_eq!(scratch.cached_atom_rows(), 3);

    assert!(matches!(
        hot.score_hot_request_candidates(
            &table,
            PhaseCenterHotRequest::new(99, &positive_atoms),
            &mut scratch,
        ),
        Err(PhaseCenterRuntimeError::ProgramIndexOutOfBounds)
    ));

    let mut wrong_width_scratch = PhaseCenterHotScratch::new(8, 4).expect("valid scratch");
    assert!(matches!(
        hot.score_hot_request_candidates(
            &table,
            PhaseCenterHotRequest::new(route_index, &positive_atoms),
            &mut wrong_width_scratch,
        ),
        Err(PhaseCenterRuntimeError::VectorWidthMismatch)
    ));

    let too_many_atoms = [1_u64, 2, 3];
    let mut tiny_cache_scratch =
        PhaseCenterHotScratch::with_atom_cache_capacity(16, 4, 2).expect("valid scratch");
    assert!(matches!(
        hot.score_hot_request_candidates(
            &table,
            PhaseCenterHotRequest::new(route_index, &too_many_atoms),
            &mut tiny_cache_scratch,
        ),
        Err(PhaseCenterRuntimeError::RuntimePackageTooLarge)
    ));
}

#[test]
fn local_accept_gate_requires_candidate_verifier_and_promotion() {
    let candidate = PhaseCenterHotCandidateDecision {
        profile_id: 7,
        margin_micro: 400_000,
        score_candidate: true,
        verifier_required: true,
        local_accept: false,
    };
    let promotion = promotion_evidence(10, 5, 120, 360);

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate,
            verifier_passed: true,
            promotion,
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision {
            local_accept: true,
            blocker: None,
        }
    );

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate,
            verifier_passed: false,
            promotion,
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision::blocked(PhaseCenterLocalAcceptBlocker::VerifierRequired)
    );

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate: PhaseCenterHotCandidateDecision {
                score_candidate: false,
                verifier_required: false,
                ..candidate
            },
            verifier_passed: true,
            promotion,
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision::blocked(PhaseCenterLocalAcceptBlocker::ScoreNotCandidate)
    );

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate: PhaseCenterHotCandidateDecision {
                local_accept: true,
                ..candidate
            },
            verifier_passed: true,
            promotion,
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision::blocked(
            PhaseCenterLocalAcceptBlocker::CandidateAlreadyClaimsLocalAccept
        )
    );

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate,
            verifier_passed: true,
            promotion: PhaseCenterPromotionEvidence {
                verifier_binding: PhaseCenterVerifierBinding::default(),
                ..promotion
            },
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision::blocked(PhaseCenterLocalAcceptBlocker::PromotionBlocked(
            PhaseCenterPromotionBlocker::MissingVerifierBinding
        ))
    );

    assert_eq!(
        PhaseCenterLocalAcceptEvidence {
            candidate,
            verifier_passed: true,
            promotion: PhaseCenterPromotionEvidence {
                false_accepts: 1,
                ..promotion
            },
        }
        .evaluate(),
        PhaseCenterLocalAcceptDecision::blocked(PhaseCenterLocalAcceptBlocker::PromotionBlocked(
            PhaseCenterPromotionBlocker::FalseAccepts
        ))
    );
}

#[test]
fn savings_report_requires_real_denominator_and_provider_costs() {
    let evidence = PhaseCenterSavingsEvidence {
        denominator: PhaseCenterSavingsDenominator {
            total_calls: 1000,
            total_tokens: 1_000_000,
            total_cost_microusd: 3_000_000,
            exact_cache_hits: 50,
            exact_cache_tokens_saved: 50_000,
            exact_cache_cost_saved_microusd: 150_000,
            synthetic_trace_used: false,
            provider_billing_evidence_present: true,
        },
        nando_unique_accepts_over_exact_cache: 100,
        nando_tokens_saved: 200_000,
        nando_cost_saved_microusd: 600_000,
        false_accepts: 0,
    };
    let report = evidence.report();
    assert!(report.market_money_claim_allowed);
    assert_eq!(report.blocker, None);
    assert_eq!(report.exact_cache_calls_saved_milli, 50);
    assert_eq!(report.nando_calls_saved_milli, 100);
    assert_eq!(report.combined_calls_saved_milli, 150);
    assert_eq!(report.exact_cache_tokens_saved_milli, 50);
    assert_eq!(report.nando_tokens_saved_milli, 200);
    assert_eq!(report.combined_tokens_saved_milli, 250);
    assert_eq!(report.exact_cache_cost_saved_milli, 50);
    assert_eq!(report.nando_cost_saved_milli, 200);
    assert_eq!(report.combined_cost_saved_milli, 250);
}

#[test]
fn savings_report_blocks_synthetic_or_unsafe_claims() {
    let safe = PhaseCenterSavingsEvidence {
        denominator: PhaseCenterSavingsDenominator {
            total_calls: 100,
            total_tokens: 1000,
            total_cost_microusd: 3000,
            exact_cache_hits: 5,
            exact_cache_tokens_saved: 50,
            exact_cache_cost_saved_microusd: 150,
            synthetic_trace_used: false,
            provider_billing_evidence_present: true,
        },
        nando_unique_accepts_over_exact_cache: 10,
        nando_tokens_saved: 100,
        nando_cost_saved_microusd: 300,
        false_accepts: 0,
    };

    assert_eq!(
        PhaseCenterSavingsEvidence {
            denominator: PhaseCenterSavingsDenominator {
                synthetic_trace_used: true,
                ..safe.denominator
            },
            ..safe
        }
        .report()
        .blocker,
        Some(PhaseCenterSavingsBlocker::SyntheticTrace)
    );
    assert_eq!(
        PhaseCenterSavingsEvidence {
            denominator: PhaseCenterSavingsDenominator {
                provider_billing_evidence_present: false,
                ..safe.denominator
            },
            ..safe
        }
        .report()
        .blocker,
        Some(PhaseCenterSavingsBlocker::MissingProviderBillingEvidence)
    );
    assert_eq!(
        PhaseCenterSavingsEvidence {
            false_accepts: 1,
            ..safe
        }
        .report()
        .blocker,
        Some(PhaseCenterSavingsBlocker::FalseAccepts)
    );
    assert_eq!(
        PhaseCenterSavingsEvidence {
            nando_unique_accepts_over_exact_cache: 0,
            ..safe
        }
        .report()
        .blocker,
        Some(PhaseCenterSavingsBlocker::NoUniqueAcceptsOverExactCache)
    );
    assert_eq!(
        PhaseCenterSavingsEvidence {
            nando_unique_accepts_over_exact_cache: 96,
            ..safe
        }
        .report()
        .blocker,
        Some(PhaseCenterSavingsBlocker::CombinedCallsExceedTotalCalls)
    );
}

#[test]
fn hot_shadow_eval_counts_candidate_savings_and_false_accepts() {
    let decisions = [
        PhaseCenterHotCandidateDecision {
            profile_id: 7,
            margin_micro: 10,
            score_candidate: false,
            verifier_required: true,
            local_accept: false,
        },
        PhaseCenterHotCandidateDecision {
            profile_id: 9,
            margin_micro: 1000,
            score_candidate: true,
            verifier_required: true,
            local_accept: false,
        },
    ];
    let mut eval = PhaseCenterHotShadowEval::default();
    eval.observe_candidate_decisions(
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: true,
            tokens: 10,
            cost_microusd: 30,
        },
        &decisions,
    );
    eval.observe_candidate_decisions(
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 20,
            cost_microusd: 60,
        },
        &decisions,
    );
    eval.observe_candidate_decisions(
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: false,
            exact_cache_hit: false,
            tokens: 40,
            cost_microusd: 120,
        },
        &decisions,
    );

    assert_eq!(eval.score_events, 3);
    assert_eq!(eval.score_candidate_events, 3);
    assert_eq!(eval.verifier_required_events, 3);
    assert_eq!(eval.local_accept_events, 0);
    assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(eval.tokens_saved, 20);
    assert_eq!(eval.cost_saved_microusd, 60);
    assert_eq!(eval.false_accepts, 1);
}

#[test]
fn prepared_hot_evidence_row_exposes_source_neutral_requests_and_denominator() {
    let vector = phase_vector_from_atoms(["route:run_check", "result:pass"], 8);
    let row = PhaseCenterPreparedHotEvidenceRow::new(
        3,
        vec![11, 22, 33],
        vector.clone(),
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 44,
            cost_microusd: 132,
        },
    );

    let atom_request = row.hot_evidence_request();
    assert_eq!(atom_request.request.route_index, 3);
    assert_eq!(atom_request.request.atom_ids, &[11, 22, 33]);
    assert_eq!(atom_request.evidence, row.evidence());

    let prepared_request = row.prepared_evidence_request();
    assert_eq!(prepared_request.request.route_index, 3);
    assert_eq!(prepared_request.request.phase_vector, vector.as_slice());
    assert_eq!(prepared_request.evidence, row.evidence());

    let mut denominator = PhaseCenterPreparedHotDenominator::default();
    denominator.observe_evidence(row.evidence());
    denominator.observe_evidence(PhaseCenterHotRequestEvidence {
        verified_safe_accept: true,
        exact_cache_hit: true,
        tokens: 10,
        cost_microusd: 30,
    });

    assert_eq!(denominator.total_tokens, 54);
    assert_eq!(denominator.total_cost_microusd, 162);
    assert_eq!(denominator.exact_cache_hits, 1);
    assert_eq!(denominator.exact_cache_tokens, 10);
    assert_eq!(denominator.exact_cache_cost_microusd, 30);
    assert_eq!(denominator.non_exact_rows, 1);
}

#[test]
fn hot_row_preparer_converts_live_atom_event_without_source_strings() {
    let positive = phase_vector_from_atoms(["route:run_check", "result:pass"], 8);
    let negative = phase_vector_from_atoms(["route:run_check", "result:fail"], 8);
    let flat = PhaseCenterFlatRuntime::new(
        8,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[900], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(700, [900])
        .expect("valid route plan")
        .expect("non-empty route plan");
    let routes = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid routes");
    let route_index = routes.resolve_route_index(700).expect("route exists");
    let atoms = [11_u64, 22, 33];
    let evidence = PhaseCenterHotRequestEvidence {
        verified_safe_accept: true,
        exact_cache_hit: false,
        tokens: 44,
        cost_microusd: 132,
    };
    let mut preparer = PhaseCenterHotRowPreparer::new(8).expect("preparer");

    let row = preparer
        .prepare_live_atom_event(
            &routes,
            PhaseCenterLiveOperatorAtomEvent::new(700, 701, &atoms, evidence),
        )
        .expect("prepare succeeds")
        .expect("route is known");

    assert_eq!(preparer.cells(), 8);
    assert_eq!(row.route_index, route_index);
    assert_eq!(row.atom_ids.as_slice(), atoms.as_slice());
    assert_eq!(row.phase_vector.len(), 8);
    assert_eq!(row.evidence(), evidence);

    let missing = preparer
        .prepare_live_atom_event(
            &routes,
            PhaseCenterLiveOperatorAtomEvent::new(701, 701, &atoms, evidence),
        )
        .expect("missing route is not an error");
    assert!(missing.is_none());
}

#[test]
fn hot_worker_scores_prepared_evidence_request_into_shadow_eval() {
    let positive = phase_vector_from_atoms(["route:tool_status", "result:ok"], 16);
    let negative = phase_vector_from_atoms(["route:tool_status", "result:error"], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("valid route plan")
        .expect("non-empty route plan");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid route table");
    let route_index = table.resolve_route_index(11).expect("route exists");
    let mut worker = PhaseCenterHotWorker::new(hot.clone(), table.clone()).expect("valid worker");
    let mut eval = PhaseCenterHotShadowEval::default();

    let positive_row = PhaseCenterPreparedHotEvidenceRow::new(
        route_index,
        vec![100, 200],
        positive.clone(),
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: true,
            exact_cache_hit: false,
            tokens: 12,
            cost_microusd: 36,
        },
    );
    let false_row = PhaseCenterPreparedHotEvidenceRow::new(
        route_index,
        vec![100, 200],
        positive.clone(),
        PhaseCenterHotRequestEvidence {
            verified_safe_accept: false,
            exact_cache_hit: false,
            tokens: 12,
            cost_microusd: 36,
        },
    );

    let decisions = worker
        .score_prepared_row_with_evidence(&positive_row, &mut eval)
        .expect("prepared evidence request scores");
    assert_eq!(decisions.len(), 1);
    assert!(decisions[0].score_candidate);

    worker
        .score_prepared_rows_with_evidence(std::slice::from_ref(&false_row), &mut eval)
        .expect("false evidence row scores");

    assert_eq!(eval.score_events, 2);
    assert_eq!(eval.score_candidate_events, 2);
    assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(eval.tokens_saved, 12);
    assert_eq!(eval.cost_saved_microusd, 36);
    assert_eq!(eval.false_accepts, 1);

    let mut runtime_eval = PhaseCenterHotShadowEval::default();
    let mut scratch = PhaseCenterHotScratch::new(16, 1).expect("scratch");
    hot.score_prepared_hot_rows_into(
        &table,
        &[positive_row, false_row],
        &mut scratch,
        &mut runtime_eval,
    )
    .expect("runtime scores prepared rows");
    assert_eq!(runtime_eval, eval);
}

#[test]
fn hot_worker_scores_live_atom_event_without_prepared_row_or_local_accept() {
    let atoms = [100_u64, 200, 300];
    let positive = phase_vector_from_atom_ids(atoms, 16);
    let negative = phase_vector_from_atoms(["route:tool_status", "result:error"], 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("valid route plan")
        .expect("non-empty route plan");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("valid route table");
    let mut worker = PhaseCenterHotWorker::new(hot, table).expect("valid worker");
    let mut eval = PhaseCenterHotShadowEval::default();

    let decisions = worker
        .score_live_atom_event_with_evidence(
            PhaseCenterLiveOperatorAtomEvent::new(
                11,
                99,
                &atoms,
                PhaseCenterHotRequestEvidence {
                    verified_safe_accept: true,
                    exact_cache_hit: false,
                    tokens: 15,
                    cost_microusd: 45,
                },
            ),
            &mut eval,
        )
        .expect("live atom event scores")
        .expect("route exists");

    assert_eq!(decisions.len(), 1);
    assert!(decisions[0].score_candidate);
    assert!(decisions[0].verifier_required);
    assert!(!decisions[0].local_accept);
    assert_eq!(eval.unique_cpu_accepts_over_exact_cache, 1);
    assert_eq!(eval.tokens_saved, 15);
    assert_eq!(eval.cost_saved_microusd, 45);
    assert_eq!(eval.false_accepts, 0);

    let missing = worker
        .score_live_atom_event_with_evidence(
            PhaseCenterLiveOperatorAtomEvent::new(
                12,
                99,
                &atoms,
                PhaseCenterHotRequestEvidence {
                    verified_safe_accept: false,
                    exact_cache_hit: false,
                    tokens: 15,
                    cost_microusd: 45,
                },
            ),
            &mut eval,
        )
        .expect("missing route is not an error");
    assert!(missing.is_none());
    assert_eq!(eval.false_accepts, 0);
}

#[test]
fn hot_runtime_scores_numeric_profile_without_cold_path() {
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");

    let positive_decision = hot.score_profile(0, &positive).expect("positive score");
    let negative_decision = hot.score_profile(0, &negative).expect("negative score");

    assert_eq!(hot.cells(), 16);
    assert_eq!(hot.profile_count(), 1);
    assert_eq!(hot.profile_id_at(0), Some(42));
    assert_eq!(hot.resolve_profile_index(42), Some(0));
    assert_eq!(hot.resolve_profile_index(7), None);
    assert_eq!(
        flat.record(0).expect("record exists").positive_center.len(),
        16
    );
    assert_eq!(
        flat.record(1),
        Err(PhaseCenterRuntimeError::CenterIndexOutOfBounds)
    );
    assert_eq!(positive_decision.profile_id, 42);
    assert_eq!(
        flat.score_vector_margin_micro(0, &positive)
            .expect("flat positive score"),
        positive_decision.margin_micro
    );
    assert_eq!(
        flat.score_vector_margin_micro(0, &negative)
            .expect("flat negative score"),
        negative_decision.margin_micro
    );
    assert!(positive_decision.local_operator);
    assert!(positive_decision.margin_micro > 0);
    assert!(!negative_decision.local_operator);
    assert!(negative_decision.margin_micro < positive_decision.margin_micro);
}

#[test]
fn hot_runtime_portable_package_roundtrips_into_worker_without_cold_path() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![
            PhaseCenterFlatRecord {
                positive_center: positive.clone().into_boxed_slice(),
                negative_center: negative.clone().into_boxed_slice(),
            },
            PhaseCenterFlatRecord {
                positive_center: phase_vector_from_atoms(
                    ["family:git_status", "state:clean", "result:summary"],
                    16,
                )
                .into_boxed_slice(),
                negative_center: phase_vector_from_atoms(
                    ["family:git_status", "state:dirty", "result:summary"],
                    16,
                )
                .into_boxed_slice(),
            },
        ],
    )
    .expect("valid flat runtime");
    let hot = PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42, 77], &[1, 1])
        .expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let second_route_plan = hot
        .route_plan_from_profile_ids(12, [77])
        .expect("route plan builds")
        .expect("route plan exists");
    let route_table = PhaseCenterHotRouteTable::from_plans([route_plan, second_route_plan])
        .expect("route table builds");
    let policy = PhaseCenterHotPackagePolicyDefaults {
        local_accept_enabled: false,
        require_verifier: true,
        require_false_accepts_zero: true,
        shadow_only: true,
        min_margin_threshold_micro: 1,
    };
    let package = PhaseCenterHotRuntimePackage::from_runtime(
        hot,
        route_table,
        test_verifier_binding(),
        policy,
    )
    .expect("package builds");
    let bytes = package.to_bytes().expect("package serializes");
    let info = PhaseCenterHotRuntimePackage::inspect_bytes(&bytes).expect("package inspects");

    assert_eq!(info.magic, PHASE_CENTER_HOT_RUNTIME_PACKAGE_MAGIC);
    assert_eq!(info.cells, 16);
    assert_eq!(info.profile_count, 2);
    assert_eq!(info.route_count, 2);
    assert_eq!(info.route_profile_edges, 2);
    assert_eq!(info.serialized_len, bytes.len());
    assert_ne!(info.fingerprint64, 0);
    assert_eq!(info.verifier_binding, test_verifier_binding());
    assert_eq!(info.policy_defaults, policy);
    assert!(info.hot_runtime_bytes_estimate > 0);
    assert!(info.hot_route_table_bytes_estimate > 0);
    assert!(info.hot_scratch_bytes_estimate > 0);
    assert!(info.hot_bytes_estimate >= info.hot_runtime_bytes_estimate);
    assert!(!info.server_policy_allows_local_accept());

    let loaded =
        PhaseCenterHotRuntimePackage::from_bytes(&bytes).expect("package loads from bytes");
    assert_eq!(loaded.info, info);
    let mut worker = loaded.into_worker().expect("worker owns loaded package");
    let route_index = worker.resolve_route_index(11).expect("route index");
    assert_eq!(worker.cells(), 16);
    assert_eq!(worker.profile_count(), 2);
    assert_eq!(worker.route_count(), 2);
    assert_eq!(worker.route_profile_edge_count(), 2);
    let decisions = worker
        .score_atom_ids(PhaseCenterHotRequest::new(route_index, &atom_ids))
        .expect("loaded worker scores");
    assert_eq!(decisions.len(), 1);
    let decision = decisions[0];

    assert_eq!(decision.profile_id, 42);
    assert!(decision.score_candidate);
    assert!(decision.verifier_required);
    assert!(!decision.local_accept);

    let wrong_decision = worker
        .score_atom_ids(PhaseCenterHotRequest::new(route_index, &wrong_atom_ids))
        .expect("loaded worker scores wrong")
        .first()
        .copied()
        .expect("wrong decision exists");
    assert!(!wrong_decision.score_candidate);
}

#[test]
fn hot_runtime_fixed_point_score_matches_float_decision_for_clear_margin() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.clone().into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");

    let float_positive = hot.score_profile(0, &positive).expect("float positive");
    let fixed_positive = hot
        .score_profile_fixed(0, &positive)
        .expect("fixed positive");
    let float_negative = hot.score_profile(0, &negative).expect("float negative");
    let fixed_negative = hot
        .score_profile_fixed(0, &negative)
        .expect("fixed negative");

    assert_eq!(fixed_positive.profile_id, float_positive.profile_id);
    assert_eq!(fixed_positive.local_operator, float_positive.local_operator);
    assert_eq!(fixed_negative.local_operator, float_negative.local_operator);
    assert!((fixed_positive.margin_micro - float_positive.margin_micro).abs() <= 50);
    assert!((fixed_negative.margin_micro - float_negative.margin_micro).abs() <= 50);
    assert!(fixed_positive.margin_micro > 0);
    assert!(fixed_negative.margin_micro < fixed_positive.margin_micro);
}

#[test]
#[ignore]
fn hot_runtime_numeric_score_path_p99_budget() {
    let positive = phase_vector_from_atoms(
        ["family:test_output_parse", "state:exit0", "result:pass"],
        16,
    );
    let negative = phase_vector_from_atoms(
        ["family:test_output_parse", "state:panic", "result:fail"],
        16,
    );
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let mut latencies = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let start = std::time::Instant::now();
        let decision = hot.score_profile(0, &positive).expect("hot score");
        latencies.push(start.elapsed().as_nanos());
        assert!(decision.local_operator);
    }
    latencies.sort_unstable();
    let p99 = latencies[latencies.len() * 99 / 100];
    println!("phase_center_hot_runtime_numeric_score_path_p99_ns={p99}");
    assert!(p99 <= 1_000, "hot path p99 budget exceeded: p99_ns={p99}");
}

#[test]
#[ignore]
fn hot_atom_request_candidate_path_adapter_cost_smoke() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let route_index = table.resolve_route_index(11).expect("route index");
    let request = PhaseCenterHotRequest::new(route_index, &atom_ids);
    let mut scratch = PhaseCenterHotScratch::new(16, 2).expect("valid scratch");

    let mut latencies = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let start = std::time::Instant::now();
        let candidates = hot
            .score_hot_request_candidates(&table, request, &mut scratch)
            .expect("hot request score");
        latencies.push(start.elapsed().as_nanos());
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].score_candidate);
        assert!(candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);
    }
    latencies.sort_unstable();
    let p99 = latencies[latencies.len() * 99 / 100];
    println!("phase_center_hot_atom_request_candidate_path_p99_ns={p99}");
    assert!(
        p99 <= 20_000,
        "hot atom request adapter path regression: p99_ns={p99}"
    );
}

#[test]
#[ignore]
fn prepared_hot_request_candidate_path_p99_budget() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let route_index = table.resolve_route_index(11).expect("route index");
    let request = PhaseCenterPreparedHotRequest::new(route_index, &positive);
    let mut scratch = PhaseCenterHotScratch::new(16, 2).expect("valid scratch");

    let mut latencies = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let start = std::time::Instant::now();
        let candidates = hot
            .score_prepared_hot_request_candidates(&table, request, &mut scratch)
            .expect("prepared hot request score");
        latencies.push(start.elapsed().as_nanos());
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].score_candidate);
        assert!(candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);
    }
    latencies.sort_unstable();
    let p99 = latencies[latencies.len() * 99 / 100];
    println!("phase_center_prepared_hot_request_candidate_path_p99_ns={p99}");
    assert!(
        p99 <= 1_000,
        "prepared hot request candidate path p99 budget exceeded: p99_ns={p99}"
    );
}

#[test]
fn hot_worker_owns_runtime_route_table_and_scratch() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let mut worker = PhaseCenterHotWorker::new(hot, table).expect("worker builds");
    let route_index = worker.resolve_route_index(11).expect("route index");
    assert_eq!(worker.cells(), 16);
    assert_eq!(worker.profile_count(), 1);
    assert_eq!(worker.route_count(), 1);
    assert_eq!(worker.route_profile_edge_count(), 1);
    assert!(worker.bytes_estimate() > 0);

    let prepared = worker
        .score_prepared(PhaseCenterPreparedHotRequest::new(route_index, &positive))
        .expect("prepared worker score");
    assert_eq!(prepared.len(), 1);
    assert!(prepared[0].score_candidate);
    assert!(prepared[0].verifier_required);
    assert!(!prepared[0].local_accept);
    let prepared_profile_id = prepared[0].profile_id;
    let prepared_score_candidate = prepared[0].score_candidate;
    let prepared_local_accept = prepared[0].local_accept;

    let atom = worker
        .score_atom_ids(PhaseCenterHotRequest::new(route_index, &atom_ids))
        .expect("atom worker score");
    assert_eq!(atom.len(), 1);
    assert_eq!(atom[0].profile_id, prepared_profile_id);
    assert_eq!(atom[0].score_candidate, prepared_score_candidate);
    assert_eq!(atom[0].local_accept, prepared_local_accept);
}

#[test]
#[ignore]
fn hot_worker_prepared_request_p99_budget() {
    let atom_ids = [42_u64, 7, 9];
    let wrong_atom_ids = [42_u64, 7, 99];
    let positive = phase_vector_from_atom_ids(atom_ids, 16);
    let negative = phase_vector_from_atom_ids(wrong_atom_ids, 16);
    let flat = PhaseCenterFlatRuntime::new(
        16,
        vec![PhaseCenterFlatRecord {
            positive_center: positive.clone().into_boxed_slice(),
            negative_center: negative.into_boxed_slice(),
        }],
    )
    .expect("valid flat runtime");
    let hot =
        PhaseCenterHotRuntime::from_flat_runtime(&flat, &[42], &[1]).expect("valid hot runtime");
    let route_plan = hot
        .route_plan_from_profile_ids(11, [42])
        .expect("route plan builds")
        .expect("route plan exists");
    let table = PhaseCenterHotRouteTable::from_plans([route_plan]).expect("route table builds");
    let mut worker = PhaseCenterHotWorker::new(hot, table).expect("worker builds");
    let route_index = worker.resolve_route_index(11).expect("route index");

    let mut latencies = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let start = std::time::Instant::now();
        let candidates = worker
            .score_prepared(PhaseCenterPreparedHotRequest::new(route_index, &positive))
            .expect("worker prepared score");
        latencies.push(start.elapsed().as_nanos());
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].score_candidate);
        assert!(candidates[0].verifier_required);
        assert!(!candidates[0].local_accept);
    }
    latencies.sort_unstable();
    let p99 = latencies[latencies.len() * 99 / 100];
    println!("phase_center_hot_worker_prepared_request_p99_ns={p99}");
    assert!(
        p99 <= 1_000,
        "hot worker prepared request p99 budget exceeded: p99_ns={p99}"
    );
}

#[test]
fn compiler_builds_runtime_from_relation_atoms() {
    let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
    compiler
        .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
        .expect("positive atoms accepted");
    compiler
        .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
        .expect("negative atoms accepted");
    let runtime = compiler.compile().expect("complete compiler");
    let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
    let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
    assert_eq!(runtime.record_count(), 1);
    assert!(
        runtime
            .margin_for(0, &correct, &wrong)
            .expect("valid compiled runtime")
            > 0.0
    );
}

#[test]
fn compiler_rejects_incomplete_programs() {
    let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
    compiler
        .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
        .expect("positive atoms accepted");
    assert_eq!(
        compiler.compile(),
        Err(PhaseCenterRuntimeError::IncompleteProgram)
    );
}

#[test]
fn runtime_package_roundtrip_preserves_margin() {
    let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
    compiler
        .add_positive_atoms(0, ["class:order", "rel:o0:s1", "out:o0", "src:s1"])
        .expect("positive atoms accepted");
    compiler
        .add_negative_atoms(0, ["class:order", "rel:o0:s0", "out:o0", "src:s0"])
        .expect("negative atoms accepted");
    let runtime = compiler.compile().expect("complete compiler");
    let bytes = runtime.to_bytes().expect("runtime serializes");
    let loaded = PhaseCenterFlatRuntime::from_bytes(&bytes).expect("runtime loads");
    let correct = phase_vector_from_atoms(["class:order", "rel:o0:s1", "out:o0", "src:s1"], 8);
    let wrong = phase_vector_from_atoms(["class:order", "rel:o0:s0", "out:o0", "src:s0"], 8);
    assert_eq!(bytes.len(), runtime.serialized_len());
    assert_eq!(loaded.cells(), runtime.cells());
    assert_eq!(loaded.record_count(), runtime.record_count());
    assert_eq!(
        loaded.margin_for(0, &correct, &wrong),
        runtime.margin_for(0, &correct, &wrong)
    );
}

#[test]
fn runtime_package_inspect_reports_header_without_loading_scores() {
    let mut compiler = PhaseCenterCompiler::new(8, 2).expect("valid compiler");
    compiler
        .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
        .expect("positive atoms accepted");
    compiler
        .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
        .expect("negative atoms accepted");
    compiler
        .add_positive_atoms(1, ["class:edit", "rel:o1:s2"])
        .expect("positive atoms accepted");
    compiler
        .add_negative_atoms(1, ["class:edit", "rel:o1:s1"])
        .expect("negative atoms accepted");
    let runtime = compiler.compile().expect("complete compiler");
    let bytes = runtime.to_bytes().expect("runtime serializes");
    let info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
    let repeat_info = PhaseCenterFlatRuntime::inspect_bytes(&bytes).expect("runtime inspects");
    let mut mutated_bytes = bytes.clone();
    let last = mutated_bytes.last_mut().expect("package has payload");
    *last ^= 0x01;
    let mutated_info =
        PhaseCenterFlatRuntime::inspect_bytes(&mutated_bytes).expect("runtime inspects");
    assert_eq!(
        info,
        PhaseCenterRuntimePackageInfo {
            magic: PHASE_CENTER_RUNTIME_PACKAGE_MAGIC,
            cells: 8,
            record_count: 2,
            serialized_len: bytes.len(),
            payload_bytes: bytes.len() - PHASE_CENTER_RUNTIME_PACKAGE_HEADER_BYTES,
            fingerprint64: runtime_package_fingerprint64(&bytes),
        }
    );
    assert_ne!(info.fingerprint64, 0);
    assert_eq!(info.fingerprint64, repeat_info.fingerprint64);
    assert_ne!(info.fingerprint64, mutated_info.fingerprint64);
}

#[test]
fn runtime_package_rejects_bad_magic() {
    let mut compiler = PhaseCenterCompiler::new(8, 1).expect("valid compiler");
    compiler
        .add_positive_atoms(0, ["class:order", "rel:o0:s1"])
        .expect("positive atoms accepted");
    compiler
        .add_negative_atoms(0, ["class:order", "rel:o0:s0"])
        .expect("negative atoms accepted");
    let runtime = compiler.compile().expect("complete compiler");
    let mut bytes = runtime.to_bytes().expect("runtime serializes");
    bytes[0] = b'X';
    assert_eq!(
        PhaseCenterFlatRuntime::from_bytes(&bytes),
        Err(PhaseCenterRuntimeError::InvalidRuntimePackage)
    );
}

#[test]
fn online_miner_checkpoint_roundtrip_preserves_mutable_centers() {
    let config = PhaseCenterOnlineMinerConfig {
        cells: 8,
        min_bucket_events: 2,
        threshold_floor_micro: 50_000,
        calibration_events: 2,
        max_buckets: 8,
    };
    let mut miner = PhaseCenterOnlineMiner::new(config).expect("online miner");
    let mut encoder = PhaseCenterAtomEncoder::new(config.cells).expect("encoder");
    for index in 0..12 {
        miner
            .observe_atom_ids(
                &mut encoder,
                7,
                [1, 2, index % 3],
                index % 4 != 0,
                false,
                100,
                0,
            )
            .expect("online observation");
    }
    let bytes = miner.to_checkpoint_bytes().expect("checkpoint");
    let restored =
        PhaseCenterOnlineMiner::from_checkpoint_bytes(&bytes).expect("checkpoint restore");
    assert_eq!(restored, miner);
    assert_eq!(restored.summary(), miner.summary());
}
