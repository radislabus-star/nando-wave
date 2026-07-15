//! Frozen synthetic A1 proof. Generator truth is used only by this evaluator.

use std::collections::BTreeSet;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    InducedExecutionStatus, InducedTransitionPackage, InductionMetrics, LayoutShape,
    TransitionInducer, TransitionTrace,
};

const META_SURFACES: usize = 8;
const SUPPORT_PER_OPERATOR: usize = 16;
const QUERY_PER_OPERATOR: usize = 100;
const RELEASE_CORPUS_SEEDS: [u64; 5] = [0x12d, 0x4a3, 0x8f1, 0xc07, 0x10b9];
const DEBUG_CORPUS_SEEDS: [u64; 2] = [0x12d, 0x4a3];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct A1Verdicts {
    pub core_pass: bool,
    pub full_contract_pass: bool,
    pub correctness_pass: bool,
    pub wave_contribution_pass: bool,
    pub portability_pass: bool,
    pub overall_pass: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct A1ProofReport {
    pub report_kind: String,
    pub verdict: String,
    pub core_verdict: String,
    pub full_contract_verdict: String,
    pub verdicts: A1Verdicts,
    pub corpus_seeds: usize,
    pub meta_surfaces: usize,
    pub heldout_surfaces: usize,
    pub heldout_new_layout_surfaces: usize,
    pub support_traces: usize,
    pub wave_training_cpu_ns: u64,
    pub induction_benchmark_repeats: usize,
    pub frozen_positive_traces: usize,
    pub correct_cpu_executions: usize,
    pub wrong_accepts: usize,
    pub hard_negatives: usize,
    pub correct_abstains: usize,
    pub negative_accepts: usize,
    pub role_swap_negatives: usize,
    pub role_swap_accepts: usize,
    pub route_splice_negatives: usize,
    pub route_splice_accepts: usize,
    pub adapter_roundtrip_checks: usize,
    pub adapter_roundtrip_failures: usize,
    pub commuting_checks: usize,
    pub commuting_failures: usize,
    pub frame_preservation_checks: usize,
    pub frame_preservation_failures: usize,
    pub metamorphic_checks: usize,
    pub metamorphic_failures: usize,
    pub verifier_mutation_checks: usize,
    pub verifier_mutation_survivors: usize,
    pub guard_cegis_candidates_checked: usize,
    pub guard_cegis_counterexamples: usize,
    pub verifier_cegis_candidates_checked: usize,
    pub verifier_cegis_counterexamples: usize,
    pub exact_cache_overlap: usize,
    pub hypotheses_generated: usize,
    pub guided_exact_checks: usize,
    pub unguided_exact_checks: usize,
    pub exact_check_reduction_milli: usize,
    pub guided_induction_cpu_ns: u64,
    pub unguided_induction_cpu_ns: u64,
    pub cpu_speedup_milli: usize,
    pub positive_coverage_milli: usize,
    pub accepted_transition_denominator: usize,
    pub accepted_error_upper_bound_ppm_95: u64,
    pub runtime_execution_samples: usize,
    pub runtime_p99_ns: u64,
    pub process_rss_kib: u64,
    pub phase_top_k_recall_milli: usize,
    pub positive_wave_examples: usize,
    pub negative_anti_center_examples: usize,
    pub phase_center_count: usize,
    pub center_split_count: usize,
    pub wave_memory_bytes: usize,
    pub induced_package_bytes: usize,
    pub routing_signature_checks: usize,
    pub routing_signature_failures: usize,
    pub induced_packages: Vec<InducedTransitionPackage>,
    pub manual_role_labels_used: bool,
    pub manual_adapters_used: bool,
    pub operator_labels_used: bool,
    pub verifier_rules_used_as_training_authority: bool,
    pub exact_lookup_used: bool,
    pub boundary: String,
}

#[derive(Clone, Debug)]
struct SurfaceSpec {
    index: usize,
    corpus_seed: u64,
    layout: LayoutShape,
    outer: String,
    root: String,
    id: String,
    status: String,
    count: String,
    owner: String,
    note: String,
    command: String,
    kind: String,
    target: String,
    set_value: String,
    amount: String,
    noise: String,
    set_kind: String,
    increment_kind: String,
}

#[derive(Clone, Debug)]
struct HeldoutSurface {
    spec: SurfaceSpec,
    support: Vec<TransitionTrace>,
    query: Vec<TransitionTrace>,
}

pub fn run_a1_proof() -> Result<A1ProofReport, String> {
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
                let spec = SurfaceSpec::new(seed_index * 1_000 + index, layout, *corpus_seed);
                traces_for(&spec, seed_index * 100_000, SUPPORT_PER_OPERATOR)
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
                    let spec = SurfaceSpec::new(
                        10_000 + seed_index * 10 + layout_index,
                        layout,
                        *corpus_seed,
                    );
                    HeldoutSurface {
                        support: traces_for(
                            &spec,
                            1_000_000 + seed_index * 100_000,
                            SUPPORT_PER_OPERATOR,
                        ),
                        query: traces_for(
                            &spec,
                            2_000_000 + seed_index * 100_000,
                            QUERY_PER_OPERATOR,
                        ),
                        spec,
                    }
                })
        })
        .collect::<Vec<_>>();

    let training_started = std::time::Instant::now();
    let mut inducer = TransitionInducer::train(&meta, 16, 0.90, 16)
        .map_err(|error| format!("train:{error:?}"))?;
    let wave_training_cpu_ns =
        u64::try_from(training_started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let support_keys = heldout
        .iter()
        .flat_map(|surface| surface.support.iter())
        .map(trace_key)
        .collect::<BTreeSet<_>>();
    let query_keys = heldout
        .iter()
        .flat_map(|surface| surface.query.iter())
        .map(trace_key)
        .collect::<BTreeSet<_>>();
    let exact_cache_overlap = support_keys.intersection(&query_keys).count();

    let mut report = A1ProofReport {
        report_kind: "nando_transition_program_induction_a1_v1".to_owned(),
        corpus_seeds: corpus_seeds.len(),
        meta_surfaces: meta.len(),
        heldout_surfaces: heldout.len(),
        heldout_new_layout_surfaces: heldout
            .iter()
            .filter(|surface| surface.spec.layout == LayoutShape::Columns)
            .count(),
        support_traces: heldout.iter().map(|surface| surface.support.len()).sum(),
        wave_training_cpu_ns,
        induction_benchmark_repeats: if cfg!(debug_assertions) { 3 } else { 7 },
        frozen_positive_traces: heldout.iter().map(|surface| surface.query.len()).sum(),
        exact_cache_overlap,
        manual_role_labels_used: false,
        manual_adapters_used: false,
        operator_labels_used: false,
        verifier_rules_used_as_training_authority: false,
        exact_lookup_used: false,
        boundary: "frozen synthetic multi-seed A1 proof: support/query few-shot adaptation with full before/action/after traces; set/increment only; live weak observation and production savings remain unproven".to_owned(),
        ..A1ProofReport::default()
    };
    let mut runtime_samples_ns = Vec::new();

    for surface in &heldout {
        let mut canonical = None;
        let mut guided_times = Vec::with_capacity(report.induction_benchmark_repeats);
        let mut unguided_times = Vec::with_capacity(report.induction_benchmark_repeats);
        for _ in 0..report.induction_benchmark_repeats {
            let mut trial_inducer = inducer.clone();
            let (package, metrics) = trial_inducer
                .induce(&surface.support)
                .map_err(|error| format!("induce surface {}:{error:?}", surface.spec.index))?;
            guided_times.push(metrics.guided_induction_cpu_ns);
            unguided_times.push(metrics.unguided_induction_cpu_ns);
            if canonical.is_none() {
                canonical = Some((trial_inducer, package, metrics));
            }
        }
        let Some((next_inducer, package, mut metrics)) = canonical else {
            return Err("induction benchmark produced no trials".to_owned());
        };
        inducer = next_inducer;
        metrics.guided_induction_cpu_ns = median_u64(&mut guided_times);
        metrics.unguided_induction_cpu_ns = median_u64(&mut unguided_times);
        accumulate_induction(&mut report, metrics);
        let package_artifact = package
            .artifact_bytes()
            .map_err(|error| format!("package:{error}"))?;
        report.induced_package_bytes += package_artifact.len();
        let roundtripped_package: InducedTransitionPackage =
            serde_json::from_slice(&package_artifact)
                .map_err(|error| format!("package_roundtrip:{error}"))?;
        report.routing_signature_checks += 1;
        if roundtripped_package != package {
            report.routing_signature_failures += 1;
        }
        for index in 0..roundtripped_package.transitions.len() {
            report.routing_signature_checks += 1;
            if roundtripped_package
                .route_margin(index)
                .is_none_or(|margin| margin <= 0)
            {
                report.routing_signature_failures += 1;
            }
        }
        evaluate_positive(
            &mut report,
            &package,
            &surface.query,
            &mut runtime_samples_ns,
        );
        evaluate_negatives(&mut report, &package, &surface.spec, &surface.query);
        evaluate_roundtrip(&mut report, &package, &surface.query);
        evaluate_metamorphic(&mut report, &package, &surface.spec, &surface.query);
        evaluate_verifier_mutations(&mut report, &package, &surface.spec, &surface.query);
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
    runtime_samples_ns.sort_unstable();
    report.runtime_execution_samples = runtime_samples_ns.len();
    report.runtime_p99_ns = percentile_u64(&runtime_samples_ns, 99);
    report.process_rss_kib = process_rss_kib();
    report.positive_coverage_milli =
        ratio_milli(report.correct_cpu_executions, report.frozen_positive_traces);
    report.accepted_transition_denominator = report
        .correct_cpu_executions
        .saturating_add(report.wrong_accepts)
        .saturating_add(report.negative_accepts);
    report.accepted_error_upper_bound_ppm_95 =
        zero_error_upper_bound_ppm_95(report.accepted_transition_denominator);

    let core_pass = report.correct_cpu_executions == report.frozen_positive_traces
        && report.wrong_accepts == 0
        && report.correct_abstains == report.hard_negatives
        && report.negative_accepts == 0
        && report.verifier_mutation_survivors == 0;
    let correctness_pass = core_pass
        && report.role_swap_negatives > 0
        && report.role_swap_accepts == 0
        && report.route_splice_negatives > 0
        && report.route_splice_accepts == 0
        && report.frame_preservation_checks == report.frozen_positive_traces
        && report.frame_preservation_failures == 0
        && report.guard_cegis_candidates_checked > 0
        && report.guard_cegis_counterexamples > 0
        && report.verifier_cegis_candidates_checked > 0
        && report.verifier_cegis_counterexamples > 0;
    let portability_pass = report.heldout_new_layout_surfaces > 0
        && report.corpus_seeds > 1
        && report.adapter_roundtrip_failures == 0
        && report.commuting_failures == 0
        && report.metamorphic_failures == 0
        && report.exact_cache_overlap == 0
        && report.routing_signature_checks > 0
        && report.routing_signature_failures == 0;
    let wave_contribution_pass = report.phase_top_k_recall_milli == 1000
        && report.positive_wave_examples > 0
        && report.negative_anti_center_examples > 0
        && report.phase_center_count >= 2
        && report.center_split_count > 0
        && report.unguided_exact_checks >= report.guided_exact_checks.saturating_mul(5)
        && report.unguided_induction_cpu_ns >= report.guided_induction_cpu_ns.saturating_mul(2);
    let full_contract_pass = correctness_pass
        && portability_pass
        && wave_contribution_pass
        && report.positive_coverage_milli > 0
        && report.accepted_transition_denominator > 0
        && report.accepted_error_upper_bound_ppm_95 > 0
        && report.induced_package_bytes > 0
        && report.guided_induction_cpu_ns > 0
        && report.runtime_execution_samples == report.frozen_positive_traces
        && report.runtime_p99_ns > 0
        && report.process_rss_kib > 0;
    let overall_pass = full_contract_pass;
    report.verdicts = A1Verdicts {
        core_pass,
        full_contract_pass,
        correctness_pass,
        wave_contribution_pass,
        portability_pass,
        overall_pass,
    };
    report.core_verdict = if core_pass {
        "A1_CORE_PASS"
    } else {
        "A1_CORE_FAIL"
    }
    .to_owned();
    report.full_contract_verdict = if full_contract_pass {
        "A1_PASS"
    } else {
        "A1_FULL_CONTRACT_WATCH"
    }
    .to_owned();
    report.verdict = if full_contract_pass {
        "A1_PASS"
    } else if core_pass {
        "A1_CORE_PASS"
    } else {
        "A1_FAIL"
    }
    .to_owned();
    Ok(report)
}

impl SurfaceSpec {
    fn new(index: usize, layout: LayoutShape, corpus_seed: u64) -> Self {
        Self {
            index,
            corpus_seed,
            layout,
            outer: seeded_name("space", index, corpus_seed),
            root: seeded_name("rows", index, corpus_seed),
            id: seeded_name("identity", index, corpus_seed),
            status: seeded_name("condition", index, corpus_seed),
            count: seeded_name("quantity", index, corpus_seed),
            owner: seeded_name("holder", index, corpus_seed),
            note: seeded_name("memo", index, corpus_seed),
            command: seeded_name("request", index, corpus_seed),
            kind: seeded_name("verb", index, corpus_seed),
            target: seeded_name("subject", index, corpus_seed),
            set_value: seeded_name("destination", index, corpus_seed),
            amount: seeded_name("step", index, corpus_seed),
            noise: seeded_name("channel", index, corpus_seed),
            set_kind: seeded_name("commit", index, corpus_seed),
            increment_kind: seeded_name("raise", index, corpus_seed),
        }
    }
}

fn seeded_name(role: &str, index: usize, seed: u64) -> String {
    let salt = mix64(seed ^ stable_role_hash(role));
    match seed % 3 {
        0 => format!("{role}_{index}_{:x}", salt & 0xfff),
        1 => format!("{:x}_{role}_{index}", salt & 0xfff),
        _ => format!("{role}{:x}_{index}", salt & 0xfff),
    }
}

fn traces_for(spec: &SurfaceSpec, seed_start: usize, per_operator: usize) -> Vec<TransitionTrace> {
    let mut traces = Vec::with_capacity(per_operator * 2);
    for offset in 0..per_operator {
        traces.push(make_trace(spec, seed_start + offset, false));
        traces.push(make_trace(spec, seed_start + per_operator + offset, true));
    }
    traces
}

fn make_trace(spec: &SurfaceSpec, seed: usize, increment: bool) -> TransitionTrace {
    let selected_original = seed % 4;
    let base_ids = (0..4)
        .map(|row| format!("entity_{}_{}_{}", spec.index, seed, row))
        .collect::<Vec<_>>();
    let order = seeded_row_order(spec.corpus_seed, seed);
    let ids = order
        .iter()
        .map(|row| base_ids[*row].clone())
        .collect::<Vec<_>>();
    let mut rows = order
        .iter()
        .map(|row| {
            let row = *row;
            Map::from_iter([
                (spec.id.clone(), Value::String(base_ids[row].clone())),
                (
                    spec.status.clone(),
                    Value::String(format!("open_{}", (seed + row) % 5)),
                ),
                (spec.count.clone(), Value::from((seed + row + 10) as u64)),
                (
                    spec.owner.clone(),
                    Value::String(format!("owner_{}", (seed + row) % 7)),
                ),
                (
                    spec.note.clone(),
                    Value::String(format!("frame_{}_{}", seed, row)),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let selected = order
        .iter()
        .position(|row| *row == selected_original)
        .unwrap_or(0);
    let selected_id = base_ids[selected_original].clone();
    let before = surface_state(spec, &ids, &rows, seed);
    let mut action_body = Map::new();
    action_body.insert(
        spec.kind.clone(),
        Value::String(if increment {
            spec.increment_kind.clone()
        } else {
            spec.set_kind.clone()
        }),
    );
    action_body.insert(spec.target.clone(), Value::String(selected_id));
    action_body.insert(
        spec.noise.clone(),
        Value::String("weak_trace_channel".to_owned()),
    );
    if increment {
        let amount = (seed % 3 + 1) as u64;
        action_body.insert(spec.amount.clone(), Value::from(amount));
        let current = rows[selected]
            .get(&spec.count)
            .and_then(Value::as_u64)
            .unwrap_or(0);
        rows[selected].insert(spec.count.clone(), Value::from(current + amount));
    } else {
        let value = format!("done_{}", seed % 3);
        action_body.insert(spec.set_value.clone(), Value::String(value.clone()));
        rows[selected].insert(spec.status.clone(), Value::String(value));
    }
    TransitionTrace {
        before,
        action: Value::Object(Map::from_iter([(
            spec.command.clone(),
            Value::Object(action_body),
        )])),
        after: surface_state(spec, &ids, &rows, seed),
    }
}

fn surface_state(
    spec: &SurfaceSpec,
    ids: &[String],
    rows: &[Map<String, Value>],
    seed: usize,
) -> Value {
    let collection = match spec.layout {
        LayoutShape::Map => Value::Object(Map::from_iter(rows.iter().enumerate().map(
            |(index, row)| {
                let mut row = row.clone();
                row.remove(&spec.id);
                (ids[index].clone(), Value::Object(row))
            },
        ))),
        LayoutShape::List => Value::Array(rows.iter().cloned().map(Value::Object).collect()),
        LayoutShape::Columns => {
            let mut columns = Map::new();
            for field in [&spec.id, &spec.status, &spec.count, &spec.owner, &spec.note] {
                columns.insert(
                    field.clone(),
                    Value::Array(
                        rows.iter()
                            .filter_map(|row| row.get(field).cloned())
                            .collect(),
                    ),
                );
            }
            Value::Object(columns)
        }
    };
    json!({
        spec.outer.clone(): {
            spec.root.clone(): collection,
            "surface_frame": format!("surface-{}", spec.index)
        },
        "trace_frame": format!("trace-{seed}")
    })
}

fn evaluate_positive(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
    runtime_samples_ns: &mut Vec<u64>,
) {
    for trace in query {
        let started = Instant::now();
        let result = package.execute(&trace.before, &trace.action);
        runtime_samples_ns.push(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        if result.status == InducedExecutionStatus::Executed
            && result.after.as_ref() == Some(&trace.after)
        {
            report.correct_cpu_executions += 1;
        } else if result.status == InducedExecutionStatus::Executed {
            report.wrong_accepts += 1;
        }
        report.commuting_checks += 1;
        if result.after.as_ref() != Some(&trace.after) {
            report.commuting_failures += 1;
        }
        report.frame_preservation_checks += 1;
        if result.after.as_ref() != Some(&trace.after) {
            report.frame_preservation_failures += 1;
        }
    }
}

fn evaluate_negatives(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    spec: &SurfaceSpec,
    query: &[TransitionTrace],
) {
    for trace in query.iter().take(40) {
        let mut missing = trace.action.clone();
        set_action_field(
            &mut missing,
            spec,
            &spec.target,
            Value::String("missing".to_owned()),
        );
        check_negative(report, package, &trace.before, &missing);

        let mut wrong_type = trace.action.clone();
        let is_set = action_kind(&wrong_type, spec) == Some(spec.set_kind.as_str());
        if is_set {
            set_action_field(&mut wrong_type, spec, &spec.set_value, Value::from(42));
        } else {
            set_action_field(
                &mut wrong_type,
                spec,
                &spec.amount,
                Value::String("not-a-number".to_owned()),
            );
        }
        check_negative(report, package, &trace.before, &wrong_type);

        if is_set {
            check_negative(report, package, &trace.after, &trace.action);
        } else {
            let mut zero = trace.action.clone();
            set_action_field(&mut zero, spec, &spec.amount, Value::from(0));
            check_negative(report, package, &trace.before, &zero);
        }

        let operand_field = if is_set {
            &spec.set_value
        } else {
            &spec.amount
        };
        if let (Some(target), Some(operand)) = (
            action_field(&trace.action, spec, &spec.target).cloned(),
            action_field(&trace.action, spec, operand_field).cloned(),
        ) {
            let mut role_swap = trace.action.clone();
            set_action_field(&mut role_swap, spec, &spec.target, operand);
            set_action_field(&mut role_swap, spec, operand_field, target);
            check_role_swap(report, package, &trace.before, &role_swap);
        }
    }
}

fn check_role_swap(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    before: &Value,
    action: &Value,
) {
    report.role_swap_negatives += 1;
    report.hard_negatives += 1;
    let result = package.execute(before, action);
    if result.status == InducedExecutionStatus::Executed {
        report.role_swap_accepts += 1;
        report.negative_accepts += 1;
    } else {
        report.correct_abstains += 1;
    }
}

fn check_negative(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    before: &Value,
    action: &Value,
) {
    report.hard_negatives += 1;
    let result = package.execute(before, action);
    if result.status == InducedExecutionStatus::Executed {
        report.negative_accepts += 1;
    } else {
        report.correct_abstains += 1;
    }
}

fn evaluate_route_splices(
    report: &mut A1ProofReport,
    packages: &[InducedTransitionPackage],
    heldout: &[HeldoutSurface],
) {
    if heldout.len() < 2 || packages.len() != heldout.len() {
        return;
    }
    for (index, package) in packages.iter().enumerate() {
        let local = &heldout[index];
        let foreign = &heldout[(index + 1) % heldout.len()];
        for (local_trace, foreign_trace) in local.query.iter().zip(foreign.query.iter()).take(40) {
            report.route_splice_negatives += 1;
            report.hard_negatives += 1;
            let result = package.execute(&local_trace.before, &foreign_trace.action);
            if result.status == InducedExecutionStatus::Executed {
                report.route_splice_accepts += 1;
                report.negative_accepts += 1;
            } else {
                report.correct_abstains += 1;
            }
        }
    }
}

fn evaluate_roundtrip(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    query: &[TransitionTrace],
) {
    for trace in query.iter().take(40) {
        for transition in &package.transitions {
            if transition.adapter.adapt_action(&trace.action).is_err() {
                continue;
            }
            report.adapter_roundtrip_checks += 2;
            let state_ok = transition
                .adapter
                .adapt_state(&trace.before)
                .and_then(|adapted| transition.adapter.project(&adapted.records, &adapted))
                .is_ok_and(|projected| projected == trace.before);
            if !state_ok {
                report.adapter_roundtrip_failures += 1;
            }
            let action_ok = transition
                .adapter
                .adapt_action(&trace.action)
                .and_then(|canonical| {
                    transition
                        .adapter
                        .encode_action(&canonical)
                        .and_then(|encoded| transition.adapter.adapt_action(&encoded))
                        .map(|roundtrip| roundtrip == canonical)
                })
                .unwrap_or(false);
            if !action_ok {
                report.adapter_roundtrip_failures += 1;
            }
        }
    }
}

fn evaluate_metamorphic(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    spec: &SurfaceSpec,
    query: &[TransitionTrace],
) {
    for trace in query.iter().take(40) {
        let mut injected = trace.clone();
        inject_irrelevant(&mut injected.before, spec, "injected_before_after");
        inject_irrelevant(&mut injected.after, spec, "injected_before_after");
        report.metamorphic_checks += 1;
        let result = package.execute(&injected.before, &injected.action);
        if result.status != InducedExecutionStatus::Executed
            || result.after.as_ref() != Some(&injected.after)
        {
            report.metamorphic_failures += 1;
        }

        if spec.layout == LayoutShape::List {
            let mut permuted = trace.clone();
            reverse_list_root(&mut permuted.before, spec);
            reverse_list_root(&mut permuted.after, spec);
            report.metamorphic_checks += 1;
            let result = package.execute(&permuted.before, &permuted.action);
            if result.status != InducedExecutionStatus::Executed
                || result.after.as_ref() != Some(&permuted.after)
            {
                report.metamorphic_failures += 1;
            }
        }
    }
}

fn evaluate_verifier_mutations(
    report: &mut A1ProofReport,
    package: &InducedTransitionPackage,
    spec: &SurfaceSpec,
    query: &[TransitionTrace],
) {
    for trace in query.iter().take(40) {
        let Some(transition) = package
            .transitions
            .iter()
            .find(|transition| transition.adapter.adapt_action(&trace.action).is_ok())
        else {
            continue;
        };
        let mutations = [
            trace.before.clone(),
            mutate_frame(&trace.after),
            mutate_record_noise(&trace.after, spec),
        ];
        for mutation in mutations {
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

fn accumulate_induction(report: &mut A1ProofReport, metrics: InductionMetrics) {
    report.hypotheses_generated += metrics.hypotheses_generated;
    report.guided_exact_checks += metrics.guided_exact_checks;
    report.unguided_exact_checks += metrics.unguided_exact_checks;
    report.guided_induction_cpu_ns = report
        .guided_induction_cpu_ns
        .saturating_add(metrics.guided_induction_cpu_ns);
    report.unguided_induction_cpu_ns = report
        .unguided_induction_cpu_ns
        .saturating_add(metrics.unguided_induction_cpu_ns);
    report.phase_top_k_recall_milli = report
        .phase_top_k_recall_milli
        .saturating_add(metrics.phase_top_k_recall_milli);
    report.positive_wave_examples = metrics.wave.positive_examples;
    report.negative_anti_center_examples = metrics.wave.negative_examples;
    report.phase_center_count = metrics.wave.center_count;
    report.center_split_count = metrics.wave.center_split_count;
    report.wave_memory_bytes = metrics.wave.wave_memory_bytes;
    report.guard_cegis_candidates_checked += metrics.synthesis.guard_candidates_checked;
    report.guard_cegis_counterexamples += metrics.synthesis.guard_counterexamples;
    report.verifier_cegis_candidates_checked += metrics.synthesis.verifier_candidates_checked;
    report.verifier_cegis_counterexamples += metrics.synthesis.verifier_counterexamples;
}

fn trace_key(trace: &TransitionTrace) -> String {
    serde_json::to_string(&(&trace.before, &trace.action)).unwrap_or_default()
}

fn reduction_milli(guided: usize, unguided: usize) -> usize {
    if unguided == 0 {
        return 0;
    }
    unguided.saturating_sub(guided) * 1000 / unguided
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

fn speedup_milli(unguided_ns: u64, guided_ns: u64) -> usize {
    if guided_ns == 0 {
        return 0;
    }
    usize::try_from(unguided_ns.saturating_mul(1000) / guided_ns).unwrap_or(usize::MAX)
}

fn median_u64(values: &mut [u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn corpus_seeds() -> &'static [u64] {
    if cfg!(debug_assertions) {
        &DEBUG_CORPUS_SEEDS
    } else {
        &RELEASE_CORPUS_SEEDS
    }
}

fn seeded_row_order(corpus_seed: u64, trace_seed: usize) -> Vec<usize> {
    let mut order = vec![0, 1, 2, 3];
    let mut state = mix64(corpus_seed ^ trace_seed as u64);
    for index in (1..order.len()).rev() {
        state = mix64(state);
        let swap = usize::try_from(state % (index as u64 + 1)).unwrap_or(0);
        order.swap(index, swap);
    }
    order
}

fn stable_role_hash(role: &str) -> u64 {
    role.as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
            (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn action_kind<'a>(action: &'a Value, spec: &SurfaceSpec) -> Option<&'a str> {
    action.get(&spec.command)?.get(&spec.kind)?.as_str()
}

fn action_field<'a>(action: &'a Value, spec: &SurfaceSpec, field: &str) -> Option<&'a Value> {
    action.get(&spec.command)?.get(field)
}

fn set_action_field(action: &mut Value, spec: &SurfaceSpec, field: &str, value: Value) {
    if let Some(body) = action
        .as_object_mut()
        .and_then(|root| root.get_mut(&spec.command))
        .and_then(Value::as_object_mut)
    {
        body.insert(field.to_owned(), value);
    }
}

fn inject_irrelevant(state: &mut Value, spec: &SurfaceSpec, value: &str) {
    if let Some(outer) = state
        .as_object_mut()
        .and_then(|root| root.get_mut(&spec.outer))
        .and_then(Value::as_object_mut)
    {
        outer.insert(
            "metamorphic_frame".to_owned(),
            Value::String(value.to_owned()),
        );
    }
}

fn reverse_list_root(state: &mut Value, spec: &SurfaceSpec) {
    if let Some(rows) = state
        .as_object_mut()
        .and_then(|root| root.get_mut(&spec.outer))
        .and_then(Value::as_object_mut)
        .and_then(|outer| outer.get_mut(&spec.root))
        .and_then(Value::as_array_mut)
    {
        rows.reverse();
    }
}

fn mutate_frame(after: &Value) -> Value {
    let mut mutation = after.clone();
    if let Some(root) = mutation.as_object_mut() {
        root.insert("forbidden_frame_change".to_owned(), Value::Bool(true));
    }
    mutation
}

fn mutate_record_noise(after: &Value, spec: &SurfaceSpec) -> Value {
    let mut mutation = after.clone();
    let Some(root) = mutation
        .as_object_mut()
        .and_then(|root| root.get_mut(&spec.outer))
        .and_then(Value::as_object_mut)
        .and_then(|outer| outer.get_mut(&spec.root))
    else {
        return mutation;
    };
    match spec.layout {
        LayoutShape::Map => {
            if let Some(record) = root
                .as_object_mut()
                .and_then(|rows| rows.values_mut().next())
                .and_then(Value::as_object_mut)
            {
                record.insert(spec.note.clone(), Value::String("mutated".to_owned()));
            }
        }
        LayoutShape::List => {
            if let Some(record) = root
                .as_array_mut()
                .and_then(|rows| rows.first_mut())
                .and_then(Value::as_object_mut)
            {
                record.insert(spec.note.clone(), Value::String("mutated".to_owned()));
            }
        }
        LayoutShape::Columns => {
            if let Some(value) = root
                .as_object_mut()
                .and_then(|columns| columns.get_mut(&spec.note))
                .and_then(Value::as_array_mut)
                .and_then(|values| values.first_mut())
            {
                *value = Value::String("mutated".to_owned());
            }
        }
    }
    mutation
}
