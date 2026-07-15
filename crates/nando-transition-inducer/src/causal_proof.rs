//! Frozen causal proof for the Wave ranking contribution to program induction.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::a2_fixture::{A2SurfaceSpec, traces_for, weak_observe};
use crate::hypothesis::{RelationEvidenceIndex, discover_action_groups, enumerate_hypotheses};
use crate::trace::{LayoutShape, TransitionTrace, discover_surface};
use crate::{InducedExecutionStatus, RoleHypothesis, TransitionInducer, WaveAblation};

const CELLS: usize = 16;
const SPLIT_THRESHOLD: f64 = 0.90;
const TOP_K: usize = 16;
const SUPPORT_PER_OPERATOR: usize = 12;
const RELEASE_CORPUS_SEEDS: [u64; 3] = [0x2a7, 0x5d1, 0x91f];
const DEBUG_CORPUS_SEEDS: [u64; 1] = [0x2a7];
const MODES: [WaveAblation; 9] = [
    WaveAblation::Full,
    WaveAblation::NoPhase,
    WaveAblation::ShuffledPhase,
    WaveAblation::MagnitudeOnly,
    WaveAblation::WithoutAntiCenter,
    WaveAblation::LegacySingleAntiCenter,
    WaveAblation::SkeletonOnly,
    WaveAblation::RandomCenter,
    WaveAblation::RandomRanking,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveCausalVerdicts {
    pub full_execution_pass: bool,
    pub phase_causal_pass: bool,
    pub relational_atoms_causal_pass: bool,
    pub anti_center_causal_pass: bool,
    pub clustered_anti_center_pass: bool,
    pub core_causal_pass: bool,
    pub strict_all_ablation_pass: bool,
    pub formation_final_pass: bool,
    pub delayed_transfer_observed: bool,
    pub compression_cleanup_observed: bool,
    pub emergent_grokking_candidate: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveCausalModeReport {
    pub mode: String,
    pub action_groups: usize,
    pub hypotheses_generated: usize,
    pub exact_checks: usize,
    pub top_k_hits: usize,
    pub top_k_recall_milli: usize,
    pub median_exact_rank: usize,
    pub p90_exact_rank: usize,
    pub max_exact_rank: usize,
    pub search_cost_vs_full_milli: usize,
    pub invalid_hypotheses: usize,
    pub dangerous_invalid_hypotheses: usize,
    pub role_binding_decoys: usize,
    pub dangerous_role_binding_decoys: usize,
    pub package_successes: usize,
    pub package_failures: usize,
    pub induced_package_bytes: usize,
    pub correct_cpu_executions: usize,
    pub abstained_queries: usize,
    pub wrong_accepts: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveFormationCheckpoint {
    pub meta_batches: usize,
    pub support_traces: usize,
    pub positive_examples: usize,
    pub negative_examples: usize,
    pub positive_centers: usize,
    pub anti_centers: usize,
    pub positive_center_mean_support_milli: usize,
    pub wave_memory_bytes: usize,
    pub memory_bytes_per_example_milli: usize,
    pub exact_checks: usize,
    pub top_k_recall_milli: usize,
    pub package_successes: usize,
    pub package_failures: usize,
    pub correct_cpu_executions: usize,
    pub abstained_queries: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct WaveCausalProofReport {
    pub report_kind: String,
    pub verdict: String,
    pub verdicts: WaveCausalVerdicts,
    pub cells: usize,
    pub top_k: usize,
    pub corpus_seeds: usize,
    pub meta_surfaces: usize,
    pub heldout_surfaces: usize,
    pub support_traces: usize,
    pub heldout_queries: usize,
    pub exact_cache_overlap: usize,
    pub structural_exact_parity_checks: usize,
    pub structural_exact_parity_failures: usize,
    pub correct_cpu_executions: usize,
    pub abstained_queries: usize,
    pub wrong_accepts: usize,
    pub package_failures: usize,
    pub induced_package_bytes: usize,
    pub modes: Vec<WaveCausalModeReport>,
    pub formation_verdict: String,
    pub formation_curve: Vec<WaveFormationCheckpoint>,
    pub boundary: String,
}

#[derive(Clone, Debug)]
struct HeldoutSurface {
    weak_support: Vec<TransitionTrace>,
    query: Vec<TransitionTrace>,
}

#[derive(Clone, Debug)]
struct PreparedRankingCandidate {
    hypothesis: RoleHypothesis,
    atoms: Vec<String>,
    stable_key: String,
    exact: bool,
}

#[derive(Clone, Debug)]
struct PreparedRankingGroup {
    concrete_kind: String,
    canonical_skeleton: crate::OperatorSkeleton,
    candidates: Vec<PreparedRankingCandidate>,
    structural_exact_parity_checks: usize,
    structural_exact_parity_failures: usize,
}

pub fn run_wave_causal_proof() -> Result<WaveCausalProofReport, String> {
    let meta = build_meta();
    let heldout = build_heldout();
    let base_inducer = TransitionInducer::train(&meta, CELLS, SPLIT_THRESHOLD, TOP_K)
        .map_err(|error| format!("train:{error:?}"))?;
    let ranking_groups = prepare_ranking_groups(base_inducer.wave_memory(), &heldout)?;
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

    let mut report = WaveCausalProofReport {
        report_kind: "nando_wave_causal_operator_proof_v2".to_owned(),
        cells: CELLS,
        top_k: TOP_K,
        corpus_seeds: corpus_seeds().len(),
        meta_surfaces: meta.len(),
        heldout_surfaces: heldout.len(),
        support_traces: heldout
            .iter()
            .map(|surface| surface.weak_support.len())
            .sum(),
        heldout_queries: heldout.iter().map(|surface| surface.query.len()).sum(),
        exact_cache_overlap: support_keys.intersection(&query_keys).count(),
        structural_exact_parity_checks: ranking_groups
            .iter()
            .map(|group| group.structural_exact_parity_checks)
            .sum(),
        structural_exact_parity_failures: ranking_groups
            .iter()
            .map(|group| group.structural_exact_parity_failures)
            .sum(),
        boundary: "frozen synthetic A2 fixed-budget causal proof; phase ablations change only candidate scoring; actor, guard, verifier, corpus, hypotheses, top-k, and tie-break stay fixed; proves search and heldout-package contribution, not live traffic, natural training dynamics, or composed programs".to_owned(),
        ..WaveCausalProofReport::default()
    };

    for mode in MODES {
        let mut mode_report = evaluate_ranking(base_inducer.wave_memory(), &ranking_groups, mode)?;
        evaluate_mode_execution(&mut mode_report, &base_inducer, &heldout, mode);
        report.modes.push(mode_report);
    }
    report.formation_curve = evaluate_formation_curve(&meta, &heldout)?;
    let full_checks = mode_report(&report.modes, WaveAblation::Full)?.exact_checks;
    for mode in &mut report.modes {
        mode.search_cost_vs_full_milli = ratio_milli(mode.exact_checks, full_checks);
    }
    let full = mode_report(&report.modes, WaveAblation::Full)?;
    let shuffled = mode_report(&report.modes, WaveAblation::ShuffledPhase)?;
    let magnitude = mode_report(&report.modes, WaveAblation::MagnitudeOnly)?;
    let no_phase = mode_report(&report.modes, WaveAblation::NoPhase)?;
    let without_anti = mode_report(&report.modes, WaveAblation::WithoutAntiCenter)?;
    let legacy_anti = mode_report(&report.modes, WaveAblation::LegacySingleAntiCenter)?;
    let skeleton = mode_report(&report.modes, WaveAblation::SkeletonOnly)?;
    let random_center = mode_report(&report.modes, WaveAblation::RandomCenter)?;
    let random = mode_report(&report.modes, WaveAblation::RandomRanking)?;
    let full_execution_pass = full.correct_cpu_executions == report.heldout_queries
        && full.wrong_accepts == 0
        && full.abstained_queries == 0
        && full.package_failures == 0
        && report.exact_cache_overlap == 0
        && report.structural_exact_parity_checks >= report.modes[0].action_groups
        && report.structural_exact_parity_failures == 0;
    let phase_causal_pass = full.top_k_recall_milli == 1000
        && ranking_degraded(shuffled, full)
        && ranking_degraded(magnitude, full)
        && ranking_degraded(no_phase, full)
        && ranking_degraded(random_center, full)
        && ranking_degraded(random, full)
        && transfer_degraded(shuffled, full)
        && transfer_degraded(magnitude, full)
        && transfer_degraded(no_phase, full)
        && transfer_degraded(random_center, full)
        && transfer_degraded(random, full);
    let relational_atoms_causal_pass = ranking_degraded(skeleton, full);
    let anti_center_causal_pass = without_anti.dangerous_role_binding_decoys
        > full.dangerous_role_binding_decoys.saturating_mul(2).max(1);
    let clustered_anti_center_pass = legacy_anti.dangerous_role_binding_decoys
        > full.dangerous_role_binding_decoys.saturating_mul(2).max(1);
    let core_causal_pass = full_execution_pass && phase_causal_pass && relational_atoms_causal_pass;
    let strict_all_ablation_pass =
        core_causal_pass && anti_center_causal_pass && clustered_anti_center_pass;
    let first_checkpoint = report
        .formation_curve
        .first()
        .ok_or_else(|| "missing_formation_checkpoint".to_owned())?;
    let final_checkpoint = report
        .formation_curve
        .last()
        .ok_or_else(|| "missing_formation_checkpoint".to_owned())?;
    let formation_final_pass = final_checkpoint.correct_cpu_executions == report.heldout_queries
        && final_checkpoint.package_failures == 0
        && final_checkpoint.abstained_queries == 0;
    let delayed_transfer_observed =
        first_checkpoint.correct_cpu_executions < final_checkpoint.correct_cpu_executions;
    let compression_cleanup_observed = final_checkpoint.memory_bytes_per_example_milli
        < first_checkpoint.memory_bytes_per_example_milli;
    let emergent_grokking_candidate =
        formation_final_pass && delayed_transfer_observed && compression_cleanup_observed;
    report.correct_cpu_executions = full.correct_cpu_executions;
    report.abstained_queries = full.abstained_queries;
    report.wrong_accepts = full.wrong_accepts;
    report.package_failures = full.package_failures;
    report.induced_package_bytes = full.induced_package_bytes;
    report.verdicts = WaveCausalVerdicts {
        full_execution_pass,
        phase_causal_pass,
        relational_atoms_causal_pass,
        anti_center_causal_pass,
        clustered_anti_center_pass,
        core_causal_pass,
        strict_all_ablation_pass,
        formation_final_pass,
        delayed_transfer_observed,
        compression_cleanup_observed,
        emergent_grokking_candidate,
    };
    report.formation_verdict = if emergent_grokking_candidate {
        "EMERGENT_GROKKING_CANDIDATE"
    } else if formation_final_pass {
        "IMMEDIATE_RELATIONAL_INDUCTION_NO_EMERGENT_STAGE"
    } else {
        "PHASE_FORMATION_FAIL"
    }
    .to_owned();
    report.verdict = if strict_all_ablation_pass {
        "WAVE_CAUSAL_OPERATOR_PROOF_PASS"
    } else if core_causal_pass {
        "WAVE_CAUSAL_CORE_PASS_ANTI_CENTER_WATCH"
    } else {
        "WAVE_CAUSAL_OPERATOR_PROOF_FAIL"
    }
    .to_owned();
    Ok(report)
}

fn build_meta() -> Vec<Vec<TransitionTrace>> {
    corpus_seeds()
        .iter()
        .enumerate()
        .flat_map(|(seed_index, corpus_seed)| {
            (0..meta_surfaces_per_seed()).map(move |index| {
                let layout = if index % 2 == 0 {
                    LayoutShape::Map
                } else {
                    LayoutShape::List
                };
                let spec = A2SurfaceSpec::new(seed_index * 1_000 + index, layout, *corpus_seed);
                traces_for(&spec, seed_index * 100_000, SUPPORT_PER_OPERATOR)
                    .iter()
                    .map(|trace| weak_observe(trace, &spec))
                    .collect()
            })
        })
        .collect()
}

fn build_heldout() -> Vec<HeldoutSurface> {
    corpus_seeds()
        .iter()
        .enumerate()
        .flat_map(|(seed_index, corpus_seed)| {
            [LayoutShape::Map, LayoutShape::List, LayoutShape::Columns]
                .into_iter()
                .enumerate()
                .map(move |(layout_index, layout)| {
                    let spec = A2SurfaceSpec::new(
                        30_000 + seed_index * 10 + layout_index,
                        layout,
                        *corpus_seed,
                    );
                    let support = traces_for(
                        &spec,
                        5_000_000 + seed_index * 100_000,
                        SUPPORT_PER_OPERATOR,
                    );
                    HeldoutSurface {
                        weak_support: support
                            .iter()
                            .map(|trace| weak_observe(trace, &spec))
                            .collect(),
                        query: traces_for(
                            &spec,
                            6_000_000 + seed_index * 100_000,
                            query_per_operator(),
                        ),
                    }
                })
        })
        .collect()
}

fn prepare_ranking_groups(
    memory: &crate::RelationWaveMemory,
    heldout: &[HeldoutSurface],
) -> Result<Vec<PreparedRankingGroup>, String> {
    let mut prepared = Vec::new();
    for surface in heldout {
        let first = surface
            .weak_support
            .first()
            .ok_or_else(|| "empty_heldout_support".to_owned())?;
        let shape = discover_surface(&first.before).map_err(str::to_owned)?;
        let (kind_path, groups) =
            discover_action_groups(&surface.weak_support).map_err(str::to_owned)?;
        for (concrete_kind, group) in groups {
            let evidence = RelationEvidenceIndex::new(&shape, &group).map_err(str::to_owned)?;
            let hypotheses = enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group);
            let mut candidates = hypotheses
                .into_iter()
                .map(|hypothesis| {
                    let atoms = evidence.signature_and_valid(&hypothesis, Some(1)).0.atoms();
                    let stable_key = hypothesis.stable_key();
                    PreparedRankingCandidate {
                        hypothesis,
                        atoms,
                        stable_key,
                        exact: false,
                    }
                })
                .collect::<Vec<_>>();
            let mut full_ranked = candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| {
                    (
                        memory.score_atoms_with_ablation(&candidate.atoms, WaveAblation::Full),
                        index,
                    )
                })
                .collect::<Vec<_>>();
            full_ranked.sort_by(|left, right| {
                right.0.cmp(&left.0).then_with(|| {
                    candidates[left.1]
                        .hypothesis
                        .cmp(&candidates[right.1].hypothesis)
                })
            });
            let canonical_index = full_ranked
                .iter()
                .find_map(|(_, index)| {
                    candidates[*index]
                        .hypothesis
                        .exact_on(&group)
                        .then_some(*index)
                })
                .ok_or_else(|| format!("exact_hypothesis_missing:{concrete_kind}"))?;
            let canonical_actor_key = actor_key(&candidates[canonical_index].hypothesis)?;
            for candidate in &mut candidates {
                candidate.exact = actor_key(&candidate.hypothesis)? == canonical_actor_key;
            }
            let structural_exact_parity_checks = candidates
                .iter()
                .filter(|candidate| candidate.exact)
                .count();
            let structural_exact_parity_failures = candidates
                .iter()
                .filter(|candidate| candidate.exact)
                .filter(|candidate| !candidate.hypothesis.exact_on(&group))
                .count();
            prepared.push(PreparedRankingGroup {
                concrete_kind,
                canonical_skeleton: candidates[canonical_index].hypothesis.skeleton,
                candidates,
                structural_exact_parity_checks,
                structural_exact_parity_failures,
            });
        }
    }
    Ok(prepared)
}

fn actor_key(hypothesis: &RoleHypothesis) -> Result<String, String> {
    let (program, adapter) = hypothesis.compile_actor();
    serde_json::to_string(&(program, adapter)).map_err(|error| error.to_string())
}

fn evaluate_ranking(
    memory: &crate::RelationWaveMemory,
    groups: &[PreparedRankingGroup],
    mode: WaveAblation,
) -> Result<WaveCausalModeReport, String> {
    let mut report = WaveCausalModeReport {
        mode: mode_name(mode).to_owned(),
        action_groups: groups.len(),
        hypotheses_generated: groups.iter().map(|group| group.candidates.len()).sum(),
        ..WaveCausalModeReport::default()
    };
    let mut exact_ranks = Vec::new();
    for group in groups {
        let mut ranked = group
            .candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let score = if mode == WaveAblation::RandomRanking {
                    stable_rank_hash(candidate.stable_key.as_bytes())
                } else {
                    memory.score_atoms_with_ablation(&candidate.atoms, mode)
                };
                (score, index)
            })
            .collect::<Vec<_>>();
        for (score, index) in &ranked {
            if !group.candidates[*index].exact {
                report.invalid_hypotheses += 1;
                report.dangerous_invalid_hypotheses += usize::from(*score > 0);
                if group.candidates[*index].hypothesis.skeleton == group.canonical_skeleton {
                    report.role_binding_decoys += 1;
                    report.dangerous_role_binding_decoys += usize::from(*score > 0);
                }
            }
        }
        ranked.sort_by(|left, right| {
            right.0.cmp(&left.0).then_with(|| {
                group.candidates[left.1]
                    .hypothesis
                    .cmp(&group.candidates[right.1].hypothesis)
            })
        });
        let exact_rank = ranked
            .iter()
            .position(|(_, index)| group.candidates[*index].exact)
            .map(|rank| rank + 1)
            .ok_or_else(|| format!("exact_hypothesis_missing:{}", group.concrete_kind))?;
        report.exact_checks += exact_rank;
        report.top_k_hits += usize::from(exact_rank <= TOP_K);
        exact_ranks.push(exact_rank);
    }
    exact_ranks.sort_unstable();
    report.top_k_recall_milli = ratio_milli(report.top_k_hits, report.action_groups);
    report.median_exact_rank = percentile(&exact_ranks, 50);
    report.p90_exact_rank = percentile(&exact_ranks, 90);
    report.max_exact_rank = exact_ranks.last().copied().unwrap_or(0);
    Ok(report)
}

fn evaluate_mode_execution(
    report: &mut WaveCausalModeReport,
    base_inducer: &TransitionInducer,
    heldout: &[HeldoutSurface],
    mode: WaveAblation,
) {
    for surface in heldout {
        let mut inducer = base_inducer.clone().with_ablation(mode);
        let Ok((package, _)) = inducer.induce_without_unguided_benchmark(&surface.weak_support)
        else {
            report.package_failures += 1;
            report.abstained_queries += surface.query.len();
            continue;
        };
        report.package_successes += 1;
        report.induced_package_bytes += package.artifact_bytes().map_or(0, |bytes| bytes.len());
        for trace in &surface.query {
            let result = package.execute_routed(&trace.before, &trace.action);
            if result.status == InducedExecutionStatus::Executed
                && result.after.as_ref() == Some(&trace.after)
            {
                report.correct_cpu_executions += 1;
            } else if result.status == InducedExecutionStatus::Executed {
                report.wrong_accepts += 1;
            } else {
                report.abstained_queries += 1;
            }
        }
    }
}

fn evaluate_formation_curve(
    meta: &[Vec<TransitionTrace>],
    heldout: &[HeldoutSurface],
) -> Result<Vec<WaveFormationCheckpoint>, String> {
    let mut checkpoint_counts = [1usize, 2, 4, 8, 12, meta.len()]
        .into_iter()
        .filter(|count| *count <= meta.len())
        .collect::<Vec<_>>();
    checkpoint_counts.sort_unstable();
    checkpoint_counts.dedup();
    let mut checkpoints = Vec::new();
    for count in checkpoint_counts {
        let inducer = TransitionInducer::train(&meta[..count], CELLS, SPLIT_THRESHOLD, TOP_K)
            .map_err(|error| format!("formation_train:{count}:{error:?}"))?;
        let groups = prepare_ranking_groups(inducer.wave_memory(), heldout)?;
        let mut mode = evaluate_ranking(inducer.wave_memory(), &groups, WaveAblation::Full)?;
        evaluate_mode_execution(&mut mode, &inducer, heldout, WaveAblation::Full);
        let wave = inducer.wave_memory().metrics();
        let total_examples = wave
            .positive_examples
            .saturating_add(wave.negative_examples);
        checkpoints.push(WaveFormationCheckpoint {
            meta_batches: count,
            support_traces: meta[..count].iter().map(Vec::len).sum(),
            positive_examples: wave.positive_examples,
            negative_examples: wave.negative_examples,
            positive_centers: wave.center_count,
            anti_centers: wave.negative_center_count,
            positive_center_mean_support_milli: wave.positive_center_mean_support_milli,
            wave_memory_bytes: wave.wave_memory_bytes,
            memory_bytes_per_example_milli: ratio_milli(wave.wave_memory_bytes, total_examples),
            exact_checks: mode.exact_checks,
            top_k_recall_milli: mode.top_k_recall_milli,
            package_successes: mode.package_successes,
            package_failures: mode.package_failures,
            correct_cpu_executions: mode.correct_cpu_executions,
            abstained_queries: mode.abstained_queries,
        });
    }
    Ok(checkpoints)
}

fn mode_report(
    reports: &[WaveCausalModeReport],
    mode: WaveAblation,
) -> Result<&WaveCausalModeReport, String> {
    let name = mode_name(mode);
    reports
        .iter()
        .find(|report| report.mode == name)
        .ok_or_else(|| format!("missing_mode:{name}"))
}

fn ranking_degraded(candidate: &WaveCausalModeReport, full: &WaveCausalModeReport) -> bool {
    candidate.top_k_recall_milli < full.top_k_recall_milli
        || candidate.exact_checks.saturating_mul(1000) >= full.exact_checks.saturating_mul(1200)
}

fn transfer_degraded(candidate: &WaveCausalModeReport, full: &WaveCausalModeReport) -> bool {
    candidate.correct_cpu_executions < full.correct_cpu_executions
        || candidate.package_failures > full.package_failures
}

const fn mode_name(mode: WaveAblation) -> &'static str {
    match mode {
        WaveAblation::Full => "full",
        WaveAblation::NoPhase => "no_phase",
        WaveAblation::ShuffledPhase => "shuffled_phase",
        WaveAblation::MagnitudeOnly => "magnitude_only",
        WaveAblation::WithoutAntiCenter => "without_anti_center",
        WaveAblation::LegacySingleAntiCenter => "legacy_single_anti_center",
        WaveAblation::SkeletonOnly => "skeleton_only",
        WaveAblation::RandomCenter => "random_center",
        WaveAblation::RandomRanking => "random_ranking",
    }
}

fn trace_key(trace: &TransitionTrace) -> String {
    serde_json::to_string(&(&trace.before, &trace.action)).unwrap_or_default()
}

fn stable_rank_hash(bytes: &[u8]) -> i64 {
    let state = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
        (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    i64::from_ne_bytes(state.to_ne_bytes())
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn corpus_seeds() -> &'static [u64] {
    if cfg!(debug_assertions) {
        &DEBUG_CORPUS_SEEDS
    } else {
        &RELEASE_CORPUS_SEEDS
    }
}

const fn meta_surfaces_per_seed() -> usize {
    if cfg!(debug_assertions) { 4 } else { 8 }
}

const fn query_per_operator() -> usize {
    if cfg!(debug_assertions) { 12 } else { 40 }
}

fn percentile(sorted: &[usize], percentile: usize) -> usize {
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
