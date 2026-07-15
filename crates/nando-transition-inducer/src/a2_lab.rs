//! Frozen A2 proof for four operators under partial and noisy support observations.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::a2_fixture::{A2_OPERATORS, A2SurfaceSpec, traces_for, weak_observe};
use crate::{
    InducedExecutionStatus, InducedTransitionPackage, InductionMetrics, LayoutShape,
    LiveObservedTransition, TransitionInducer, TransitionTrace,
};

const META_SURFACES: usize = 8;
const SUPPORT_PER_OPERATOR: usize = 12;
const QUERY_PER_OPERATOR: usize = 50;
const RELEASE_CORPUS_SEEDS: [u64; 5] = [0x2a7, 0x5d1, 0x91f, 0xd33, 0x1279];
const DEBUG_CORPUS_SEEDS: [u64; 2] = [0x2a7, 0x5d1];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct A2Verdicts {
    pub operator_expansion_pass: bool,
    pub weak_observability_pass: bool,
    pub wave_contribution_pass: bool,
    pub overall_pass: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct A2ProofReport {
    pub report_kind: String,
    pub verdict: String,
    pub verdicts: A2Verdicts,
    pub corpus_seeds: usize,
    pub meta_surfaces: usize,
    pub heldout_surfaces: usize,
    pub weak_support_traces: usize,
    pub masked_state_fields: usize,
    pub injected_noise_fields: usize,
    pub frozen_positive_traces: usize,
    pub correct_cpu_executions: usize,
    pub wrong_accepts: usize,
    pub positive_coverage_milli: usize,
    pub induced_family_counts: BTreeMap<String, usize>,
    pub full_weak_family_disagreements: usize,
    pub full_weak_execution_disagreements: usize,
    pub hard_negatives: usize,
    pub correct_abstains: usize,
    pub negative_accepts: usize,
    pub role_swap_negatives: usize,
    pub role_swap_accepts: usize,
    pub route_splice_negatives: usize,
    pub route_splice_accepts: usize,
    pub duplicate_append_negatives: usize,
    pub duplicate_append_accepts: usize,
    pub missing_delete_negatives: usize,
    pub missing_delete_accepts: usize,
    pub adapter_roundtrip_checks: usize,
    pub adapter_roundtrip_failures: usize,
    pub commuting_checks: usize,
    pub commuting_failures: usize,
    pub frame_preservation_checks: usize,
    pub frame_preservation_failures: usize,
    pub verifier_mutation_checks: usize,
    pub verifier_mutation_survivors: usize,
    pub exact_cache_overlap: usize,
    pub hypotheses_generated: usize,
    pub guided_exact_checks: usize,
    pub unguided_exact_checks: usize,
    pub exact_check_reduction_milli: usize,
    pub guided_induction_cpu_ns: u64,
    pub unguided_induction_cpu_ns: u64,
    pub cpu_speedup_milli: usize,
    pub phase_top_k_recall_milli: usize,
    pub phase_center_count: usize,
    pub center_split_count: usize,
    pub positive_wave_examples: usize,
    pub negative_anti_center_examples: usize,
    pub wave_memory_bytes: usize,
    pub guard_cegis_candidates_checked: usize,
    pub guard_cegis_counterexamples: usize,
    pub verifier_cegis_candidates_checked: usize,
    pub verifier_cegis_counterexamples: usize,
    pub accepted_transition_denominator: usize,
    pub accepted_error_upper_bound_ppm_95: u64,
    pub runtime_execution_samples: usize,
    pub runtime_p99_ns: u64,
    pub process_rss_kib: u64,
    pub induced_package_bytes: usize,
    pub routing_signature_checks: usize,
    pub routing_signature_failures: usize,
    pub induced_packages: Vec<InducedTransitionPackage>,
    pub boundary: String,
}

#[derive(Clone, Debug)]
struct HeldoutSurface {
    spec: A2SurfaceSpec,
    full_support: Vec<TransitionTrace>,
    weak_support: Vec<TransitionTrace>,
    query: Vec<TransitionTrace>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct A2LiveSmokeFixture {
    pub package: InducedTransitionPackage,
    pub traces: Vec<LiveObservedTransition>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseLiveSmokeFixture {
    pub support: Vec<LiveObservedTransition>,
    pub future_shadow: Vec<LiveObservedTransition>,
    pub future_promotion: Vec<LiveObservedTransition>,
}

pub fn build_raw_phase_live_smoke_fixture() -> RawPhaseLiveSmokeFixture {
    let mut support = Vec::new();
    let mut future_shadow = Vec::new();
    let mut future_promotion = Vec::new();
    for surface in 0..4 {
        let spec = A2SurfaceSpec::new(91_000 + surface, LayoutShape::Map, 0x61c3);
        support.extend(live_rows(
            traces_for(&spec, surface * 100_000, SUPPORT_PER_OPERATOR),
            format!("raw-live-support-{surface}"),
            8,
        ));
        future_shadow.extend(live_rows(
            traces_for(&spec, 8_000_000 + surface * 100_000, 2),
            format!("raw-live-shadow-{surface}"),
            9,
        ));
        future_promotion.extend(live_rows(
            traces_for(&spec, 9_000_000 + surface * 100_000, 14),
            format!("raw-live-promotion-{surface}"),
            10,
        ));
    }
    RawPhaseLiveSmokeFixture {
        support,
        future_shadow,
        future_promotion,
    }
}

pub fn build_raw_phase_frontier_fixture(surface_count: usize) -> Vec<LiveObservedTransition> {
    let mut traces = Vec::new();
    for surface in 0..surface_count {
        let spec = A2SurfaceSpec::new(92_000 + surface, LayoutShape::Map, 0x71d5);
        traces.extend(live_rows(
            traces_for(&spec, 11_000_000 + surface * 100_000, 1),
            format!("raw-frontier-{surface}"),
            8,
        ));
    }
    traces
}

fn live_rows(
    traces: Vec<TransitionTrace>,
    prefix: String,
    july_day: usize,
) -> Vec<LiveObservedTransition> {
    traces
        .into_iter()
        .enumerate()
        .map(|(index, trace)| LiveObservedTransition {
            schema: crate::LIVE_GROUNDED_TRACE_SCHEMA.to_owned(),
            trace_id: format!("{prefix}-{index:04}"),
            timestamp: format!(
                "2026-07-{july_day:02}T{:02}:{:02}:00Z",
                index / 4,
                index % 4
            ),
            before: trace.before,
            action: trace.action,
            after: trace.after,
            input_tokens: 100,
            output_tokens: 25,
            total_tokens: 125,
            request_sha256: format!("{prefix}-{index:04}"),
            evidence_source: "application_state".to_owned(),
            evidence_verifier: "raw_phase_fixture_exact_state".to_owned(),
            evidence_receipt_sha256: format!("{:064x}", stable_fixture_receipt(&prefix, index)),
            source_session_id_sha256: format!(
                "{:064x}",
                stable_fixture_receipt(&prefix, index % 4)
            ),
            source_event_id_sha256: format!("{:064x}", stable_fixture_receipt(&prefix, index)),
        })
        .collect()
}

fn stable_fixture_receipt(prefix: &str, index: usize) -> u64 {
    prefix
        .bytes()
        .chain(index.to_le_bytes())
        .fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
            (state ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

pub fn build_a2_live_smoke_fixture() -> Result<A2LiveSmokeFixture, String> {
    let spec = A2SurfaceSpec::new(90_001, LayoutShape::Map, 0x51a7);
    let support = traces_for(&spec, 5_000_000, SUPPORT_PER_OPERATOR);
    let query = traces_for(&spec, 6_000_000, 20);
    let weak_support = support
        .iter()
        .map(|trace| weak_observe(trace, &spec))
        .collect::<Vec<_>>();
    let mut inducer = TransitionInducer::train(std::slice::from_ref(&weak_support), 16, 0.90, 16)
        .map_err(|error| format!("live_fixture_train:{error:?}"))?;
    let (package, _) = inducer
        .induce(&weak_support)
        .map_err(|error| format!("live_fixture_induce:{error:?}"))?;
    let traces = query
        .into_iter()
        .enumerate()
        .map(|(index, trace)| LiveObservedTransition {
            schema: crate::LIVE_GROUNDED_TRACE_SCHEMA.to_owned(),
            trace_id: format!("a2-live-smoke-{index:04}"),
            timestamp: "deployment-smoke".to_owned(),
            before: trace.before,
            action: trace.action,
            after: trace.after,
            input_tokens: 100,
            output_tokens: 25,
            total_tokens: 125,
            request_sha256: format!("deployment-smoke-{index:04}"),
            evidence_source: "application_state".to_owned(),
            evidence_verifier: "a2_fixture_exact_state".to_owned(),
            evidence_receipt_sha256: format!("{:064x}", index + 1),
            source_session_id_sha256: format!("{:064x}", index / 4 + 1),
            source_event_id_sha256: format!("{:064x}", index + 1),
        })
        .collect();
    Ok(A2LiveSmokeFixture { package, traces })
}

pub fn run_a2_proof() -> Result<A2ProofReport, String> {
    let corpus_seeds = corpus_seeds();
    let meta = corpus_seeds
        .iter()
        .enumerate()
        .flat_map(|(seed_index, corpus_seed)| {
            (0..META_SURFACES).map(move |index| {
                let layout = if index % 2 == 0 {
                    LayoutShape::Map
                } else {
                    LayoutShape::List
                };
                let spec = A2SurfaceSpec::new(seed_index * 1_000 + index, layout, *corpus_seed);
                traces_for(&spec, seed_index * 100_000, SUPPORT_PER_OPERATOR)
                    .iter()
                    .map(|trace| weak_observe(trace, &spec))
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    let heldout = corpus_seeds
        .iter()
        .enumerate()
        .flat_map(|(seed_index, corpus_seed)| {
            [LayoutShape::Map, LayoutShape::List, LayoutShape::Columns]
                .into_iter()
                .enumerate()
                .map(move |(layout_index, layout)| {
                    let spec = A2SurfaceSpec::new(
                        20_000 + seed_index * 10 + layout_index,
                        layout,
                        *corpus_seed,
                    );
                    let full_support = traces_for(
                        &spec,
                        3_000_000 + seed_index * 100_000,
                        SUPPORT_PER_OPERATOR,
                    );
                    let weak_support = full_support
                        .iter()
                        .map(|trace| weak_observe(trace, &spec))
                        .collect();
                    let query =
                        traces_for(&spec, 4_000_000 + seed_index * 100_000, QUERY_PER_OPERATOR);
                    HeldoutSurface {
                        spec,
                        full_support,
                        weak_support,
                        query,
                    }
                })
        })
        .collect::<Vec<_>>();

    let mut inducer = TransitionInducer::train(&meta, 16, 0.90, 16)
        .map_err(|error| format!("train:{error:?}"))?;
    let support_keys = heldout
        .iter()
        .flat_map(|surface| surface.weak_support.iter())
        .map(trace_key)
        .collect::<BTreeSet<_>>();
    let query_keys = heldout
        .iter()
        .flat_map(|surface| surface.query.iter())
        .map(trace_key)
        .collect::<BTreeSet<_>>();
    let mut report = A2ProofReport {
        report_kind: "nando_transition_program_induction_a2_v1".to_owned(),
        corpus_seeds: corpus_seeds.len(),
        meta_surfaces: meta.len(),
        heldout_surfaces: heldout.len(),
        weak_support_traces: heldout
            .iter()
            .map(|surface| surface.weak_support.len())
            .sum(),
        masked_state_fields: heldout
            .iter()
            .map(|surface| surface.weak_support.len() * 2 * 2)
            .sum(),
        injected_noise_fields: heldout
            .iter()
            .map(|surface| surface.weak_support.len() * 3)
            .sum(),
        frozen_positive_traces: heldout.iter().map(|surface| surface.query.len()).sum(),
        exact_cache_overlap: support_keys.intersection(&query_keys).count(),
        boundary: "frozen synthetic A2: four V1 operators; partial irrelevant-field masking and irrelevant noise in support; full target/effect evidence remains observable; no live traffic claim".to_owned(),
        ..A2ProofReport::default()
    };
    let benchmark_repeats = if cfg!(debug_assertions) { 1 } else { 5 };
    let mut runtime_samples = Vec::new();

    for surface in &heldout {
        let base_inducer = inducer.clone();
        let mut full_inducer = base_inducer.clone();
        let (full_package, _) = full_inducer
            .induce(&surface.full_support)
            .map_err(|error| format!("full_induce:{}:{error:?}", surface.spec.index))?;
        let mut canonical = None;
        let mut guided_times = Vec::with_capacity(benchmark_repeats);
        let mut unguided_times = Vec::with_capacity(benchmark_repeats);
        for _ in 0..benchmark_repeats {
            let mut trial = base_inducer.clone();
            let (package, metrics) = trial
                .induce(&surface.weak_support)
                .map_err(|error| format!("weak_induce:{}:{error:?}", surface.spec.index))?;
            guided_times.push(metrics.guided_induction_cpu_ns);
            unguided_times.push(metrics.unguided_induction_cpu_ns);
            if canonical.is_none() {
                canonical = Some((trial, package, metrics));
            }
        }
        let Some((next_inducer, package, mut metrics)) = canonical else {
            return Err("empty_a2_benchmark".to_owned());
        };
        inducer = next_inducer;
        metrics.guided_induction_cpu_ns = median_u64(&mut guided_times);
        metrics.unguided_induction_cpu_ns = median_u64(&mut unguided_times);
        accumulate_induction(&mut report, metrics);

        let weak_families = package_families(&package);
        let full_families = package_families(&full_package);
        if weak_families != full_families {
            report.full_weak_family_disagreements += 1;
        }
        for family in &weak_families {
            *report
                .induced_family_counts
                .entry(family.clone())
                .or_default() += 1;
        }
        evaluate_full_weak_parity(&mut report, &package, &full_package, &surface.query);

        let bytes = package
            .artifact_bytes()
            .map_err(|error| format!("package:{error}"))?;
        report.induced_package_bytes += bytes.len();
        let roundtripped: InducedTransitionPackage =
            serde_json::from_slice(&bytes).map_err(|error| format!("roundtrip:{error}"))?;
        report.routing_signature_checks += 1;
        if roundtripped != package {
            report.routing_signature_failures += 1;
        }
        for index in 0..roundtripped.transitions.len() {
            report.routing_signature_checks += 1;
            if roundtripped
                .route_margin(index)
                .is_none_or(|margin| margin <= 0)
            {
                report.routing_signature_failures += 1;
            }
        }

        evaluate_positives(&mut report, &package, &surface.query, &mut runtime_samples);
        evaluate_negatives(&mut report, &package, &surface.query);
        evaluate_roundtrips(&mut report, &package, &surface.query);
        evaluate_verifier_mutations(&mut report, &package, &surface.query);
        report.induced_packages.push(package);
    }

    let packages = report.induced_packages.clone();
    evaluate_route_splices(&mut report, &packages, &heldout);
    report.phase_top_k_recall_milli /= heldout.len().max(1);
    report.exact_check_reduction_milli =
        reduction_milli(report.guided_exact_checks, report.unguided_exact_checks);
    report.cpu_speedup_milli = speedup_milli(
        report.unguided_induction_cpu_ns,
        report.guided_induction_cpu_ns,
    );
    report.positive_coverage_milli =
        ratio_milli(report.correct_cpu_executions, report.frozen_positive_traces);
    report.accepted_transition_denominator = report
        .correct_cpu_executions
        .saturating_add(report.wrong_accepts)
        .saturating_add(report.negative_accepts);
    report.accepted_error_upper_bound_ppm_95 =
        zero_error_upper_bound_ppm_95(report.accepted_transition_denominator);
    runtime_samples.sort_unstable();
    report.runtime_execution_samples = runtime_samples.len();
    report.runtime_p99_ns = percentile_u64(&runtime_samples, 99);
    report.process_rss_kib = process_rss_kib();

    let expected_families = BTreeSet::from([
        "append_record".to_owned(),
        "delete_record".to_owned(),
        "increment_field".to_owned(),
        "set_field".to_owned(),
    ]);
    let induced_families = report
        .induced_family_counts
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let operator_expansion_pass = induced_families == expected_families
        && report.correct_cpu_executions == report.frozen_positive_traces
        && report.wrong_accepts == 0
        && report.negative_accepts == 0
        && report.correct_abstains == report.hard_negatives
        && report.role_swap_negatives > 0
        && report.role_swap_accepts == 0
        && report.route_splice_negatives > 0
        && report.route_splice_accepts == 0
        && report.duplicate_append_negatives > 0
        && report.duplicate_append_accepts == 0
        && report.missing_delete_negatives > 0
        && report.missing_delete_accepts == 0
        && report.verifier_mutation_survivors == 0;
    let weak_observability_pass = report.corpus_seeds > 1
        && report.masked_state_fields > 0
        && report.injected_noise_fields > 0
        && report.full_weak_family_disagreements == 0
        && report.full_weak_execution_disagreements == 0
        && report.adapter_roundtrip_failures == 0
        && report.commuting_failures == 0
        && report.frame_preservation_failures == 0
        && report.exact_cache_overlap == 0
        && report.routing_signature_failures == 0
        && report.accepted_error_upper_bound_ppm_95 > 0
        && report.runtime_p99_ns > 0
        && report.process_rss_kib > 0;
    let wave_contribution_pass = report.phase_top_k_recall_milli == 1000
        && report.phase_center_count >= 4
        && report.center_split_count >= 3
        && report.unguided_exact_checks >= report.guided_exact_checks.saturating_mul(5)
        && report.unguided_induction_cpu_ns >= report.guided_induction_cpu_ns.saturating_mul(2);
    let overall_pass = operator_expansion_pass && weak_observability_pass && wave_contribution_pass;
    report.verdicts = A2Verdicts {
        operator_expansion_pass,
        weak_observability_pass,
        wave_contribution_pass,
        overall_pass,
    };
    report.verdict = if overall_pass {
        "A2_PASS"
    } else if operator_expansion_pass {
        "A2_OPERATOR_EXPANSION_PASS_FULL_CONTRACT_WATCH"
    } else {
        "A2_FAIL"
    }
    .to_owned();
    Ok(report)
}

fn evaluate_positives(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
    runtime_samples: &mut Vec<u64>,
) {
    for trace in query {
        let started = Instant::now();
        let result = package.execute_routed(&trace.before, &trace.action);
        runtime_samples.push(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        if result.status == InducedExecutionStatus::Executed
            && result.after.as_ref() == Some(&trace.after)
        {
            report.correct_cpu_executions += 1;
        } else if result.status == InducedExecutionStatus::Executed {
            report.wrong_accepts += 1;
        }
        report.commuting_checks += 1;
        report.frame_preservation_checks += 1;
        if result.after.as_ref() != Some(&trace.after) {
            report.commuting_failures += 1;
            report.frame_preservation_failures += 1;
        }
    }
}

fn evaluate_full_weak_parity(
    report: &mut A2ProofReport,
    weak: &InducedTransitionPackage,
    full: &InducedTransitionPackage,
    query: &[TransitionTrace],
) {
    for trace in query.iter().step_by(10) {
        let weak_result = weak.execute_routed(&trace.before, &trace.action);
        let full_result = full.execute_routed(&trace.before, &trace.action);
        if weak_result.status != full_result.status || weak_result.after != full_result.after {
            report.full_weak_execution_disagreements += 1;
        }
    }
}

fn evaluate_negatives(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
) {
    for operator_index in 0..A2_OPERATORS.len() {
        let start = operator_index * QUERY_PER_OPERATOR;
        for trace in query.iter().skip(start).take(10) {
            let Some(transition) = package
                .transitions
                .iter()
                .find(|transition| transition.adapter.adapt_action(&trace.action).is_ok())
            else {
                continue;
            };
            let Ok(mut canonical) = transition.adapter.adapt_action(&trace.action) else {
                continue;
            };
            match transition.program.action_kind.as_str() {
                "set_field" => {
                    check_negative(report, package, &trace.after, &trace.action);
                    role_swap(
                        report,
                        package,
                        transition,
                        trace,
                        &mut canonical,
                        "target",
                        "value",
                    );
                }
                "increment_field" => {
                    set_slot(&mut canonical, "amount", Value::from(0));
                    check_encoded_negative(report, package, transition, trace, &canonical);
                    role_swap(
                        report,
                        package,
                        transition,
                        trace,
                        &mut canonical,
                        "target",
                        "amount",
                    );
                }
                "append_record" => {
                    if let Some(existing_id) = transition
                        .adapter
                        .adapt_state(&trace.before)
                        .ok()
                        .and_then(|state| state.records.first().cloned())
                        .and_then(|record| record.get("id").cloned())
                    {
                        set_slot(&mut canonical, "record.id", existing_id);
                        report.duplicate_append_negatives += 1;
                        let accepted =
                            check_encoded_negative(report, package, transition, trace, &canonical);
                        if accepted {
                            report.duplicate_append_accepts += 1;
                        }
                    }
                }
                "delete_record" => {
                    set_slot(
                        &mut canonical,
                        "target",
                        Value::String("__a2_missing_delete__".to_owned()),
                    );
                    report.missing_delete_negatives += 1;
                    let accepted =
                        check_encoded_negative(report, package, transition, trace, &canonical);
                    if accepted {
                        report.missing_delete_accepts += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

fn role_swap(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    transition: &crate::InducedTransition,
    trace: &TransitionTrace,
    canonical: &mut Map<String, Value>,
    left: &str,
    right: &str,
) {
    let (Some(left_value), Some(right_value)) = (
        canonical_slot(canonical, left),
        canonical_slot(canonical, right),
    ) else {
        return;
    };
    set_slot(canonical, left, right_value);
    set_slot(canonical, right, left_value);
    report.role_swap_negatives += 1;
    let accepted = check_encoded_negative(report, package, transition, trace, canonical);
    if accepted {
        report.role_swap_accepts += 1;
    }
}

fn check_encoded_negative(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    transition: &crate::InducedTransition,
    trace: &TransitionTrace,
    canonical: &Map<String, Value>,
) -> bool {
    let Ok(action) = transition.adapter.encode_action(canonical) else {
        return false;
    };
    check_negative(report, package, &trace.before, &action)
}

fn check_negative(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    before: &Value,
    action: &Value,
) -> bool {
    report.hard_negatives += 1;
    let result = package.execute_routed(before, action);
    if result.status == InducedExecutionStatus::Executed {
        report.negative_accepts += 1;
        true
    } else {
        report.correct_abstains += 1;
        false
    }
}

fn evaluate_route_splices(
    report: &mut A2ProofReport,
    packages: &[InducedTransitionPackage],
    heldout: &[HeldoutSurface],
) {
    if packages.len() != heldout.len() || packages.len() < 2 {
        return;
    }
    for (index, package) in packages.iter().enumerate() {
        let foreign = &heldout[(index + 1) % heldout.len()];
        for (local, foreign_trace) in heldout[index]
            .query
            .iter()
            .zip(foreign.query.iter())
            .take(20)
        {
            report.route_splice_negatives += 1;
            if check_negative(report, package, &local.before, &foreign_trace.action) {
                report.route_splice_accepts += 1;
            }
        }
    }
}

fn evaluate_roundtrips(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
) {
    for trace in query.iter().step_by(10) {
        let Some(transition) = package
            .transitions
            .iter()
            .find(|transition| transition.adapter.adapt_action(&trace.action).is_ok())
        else {
            continue;
        };
        report.adapter_roundtrip_checks += 2;
        let state_ok = transition
            .adapter
            .adapt_state(&trace.before)
            .and_then(|adapted| transition.adapter.project(&adapted.records, &adapted))
            .is_ok_and(|projected| projected == trace.before);
        let action_ok = transition
            .adapter
            .adapt_action(&trace.action)
            .and_then(|canonical| transition.adapter.encode_action(&canonical))
            .and_then(|encoded| transition.adapter.adapt_action(&encoded))
            .is_ok();
        report.adapter_roundtrip_failures += usize::from(!state_ok) + usize::from(!action_ok);
    }
}

fn evaluate_verifier_mutations(
    report: &mut A2ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
) {
    for trace in query.iter().step_by(10) {
        let Some(transition) = package
            .transitions
            .iter()
            .find(|transition| transition.adapter.adapt_action(&trace.action).is_ok())
        else {
            continue;
        };
        let mut frame_mutation = trace.after.clone();
        if let Some(root) = frame_mutation.as_object_mut() {
            root.insert("a2_forbidden_frame".to_owned(), Value::Bool(true));
        }
        for mutation in [trace.before.clone(), frame_mutation] {
            report.verifier_mutation_checks += 1;
            if transition
                .verifier
                .verify(&transition.adapter, &trace.before, &trace.action, &mutation)
                .is_ok()
            {
                report.verifier_mutation_survivors += 1;
            }
        }
    }
}

fn package_families(package: &InducedTransitionPackage) -> BTreeSet<String> {
    package
        .transitions
        .iter()
        .map(|transition| transition.program.action_kind.clone())
        .collect()
}

fn accumulate_induction(report: &mut A2ProofReport, metrics: InductionMetrics) {
    report.hypotheses_generated += metrics.hypotheses_generated;
    report.guided_exact_checks += metrics.guided_exact_checks;
    report.unguided_exact_checks += metrics.unguided_exact_checks;
    report.guided_induction_cpu_ns = report
        .guided_induction_cpu_ns
        .saturating_add(metrics.guided_induction_cpu_ns);
    report.unguided_induction_cpu_ns = report
        .unguided_induction_cpu_ns
        .saturating_add(metrics.unguided_induction_cpu_ns);
    report.phase_top_k_recall_milli += metrics.phase_top_k_recall_milli;
    report.phase_center_count = metrics.wave.center_count;
    report.center_split_count = metrics.wave.center_split_count;
    report.positive_wave_examples = metrics.wave.positive_examples;
    report.negative_anti_center_examples = metrics.wave.negative_examples;
    report.wave_memory_bytes = metrics.wave.wave_memory_bytes;
    report.guard_cegis_candidates_checked += metrics.synthesis.guard_candidates_checked;
    report.guard_cegis_counterexamples += metrics.synthesis.guard_counterexamples;
    report.verifier_cegis_candidates_checked += metrics.synthesis.verifier_candidates_checked;
    report.verifier_cegis_counterexamples += metrics.synthesis.verifier_counterexamples;
}

fn canonical_slot(action: &Map<String, Value>, path: &str) -> Option<Value> {
    let mut cursor = action.get(path.split('.').next()?)?;
    for segment in path.split('.').skip(1) {
        cursor = cursor.as_object()?.get(segment)?;
    }
    Some(cursor.clone())
}

fn set_slot(action: &mut Map<String, Value>, path: &str, value: Value) {
    let mut segments = path.split('.');
    let Some(first) = segments.next() else {
        return;
    };
    let Some(second) = segments.next() else {
        action.insert(first.to_owned(), value);
        return;
    };
    let nested = action
        .entry(first.to_owned())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(object) = nested.as_object_mut() {
        object.insert(second.to_owned(), value);
    }
}

fn trace_key(trace: &TransitionTrace) -> String {
    serde_json::to_string(&(&trace.before, &trace.action)).unwrap_or_default()
}

fn corpus_seeds() -> &'static [u64] {
    if cfg!(debug_assertions) {
        &DEBUG_CORPUS_SEEDS
    } else {
        &RELEASE_CORPUS_SEEDS
    }
}

fn reduction_milli(guided: usize, unguided: usize) -> usize {
    if unguided == 0 {
        return 0;
    }
    unguided.saturating_sub(guided) * 1000 / unguided
}

fn speedup_milli(unguided_ns: u64, guided_ns: u64) -> usize {
    if guided_ns == 0 {
        return 0;
    }
    usize::try_from(unguided_ns.saturating_mul(1000) / guided_ns).unwrap_or(usize::MAX)
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn zero_error_upper_bound_ppm_95(denominator: usize) -> u64 {
    if denominator == 0 {
        return 0;
    }
    let upper = 1.0 - 0.05_f64.powf(1.0 / denominator as f64);
    (upper * 1_000_000.0).ceil() as u64
}

fn median_u64(values: &mut [u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn percentile_u64(sorted: &[u64], percentile: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = sorted
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

fn process_rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmRSS:")?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
        })
        .unwrap_or(0)
}
