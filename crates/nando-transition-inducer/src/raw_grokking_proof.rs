//! Frozen proof for discovering a relation-phase basis from anonymous deltas.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::a2_fixture::{A2SurfaceSpec, traces_for, weak_observe};
use crate::hypothesis::{
    OperatorSkeleton, RelationEvidenceIndex, RoleHypothesis, discover_action_groups,
    enumerate_hypotheses,
};
use crate::package::{InducedTransition, InducedTransitionPackage};
use crate::synthesis::synthesize_contracts;
use crate::trace::{LayoutShape, TransitionTrace, discover_surface};
use crate::{InducedExecutionStatus, RelationWaveMemory, WaveAblation, WaveTrainingExample};

const CELLS: usize = 16;
const TOP_K: usize = 48;
const SPLIT_THRESHOLD: f64 = 0.90;
const MIN_CROSS_SURFACE_SUPPORT: usize = 3;
const MIN_RATE_CONTRAST_MILLI: usize = 50;
const MAX_PACKAGE_BYTES: usize = 262_144;
const SUPPORT_PER_OPERATOR: usize = 12;
const RELEASE_CORPUS_SEEDS: [u64; 3] = [0x3b9, 0x6e3, 0xa21];
const DEBUG_CORPUS_SEEDS: [u64; 1] = [0x3b9];
const MODES: [WaveAblation; 8] = [
    WaveAblation::Full,
    WaveAblation::NoPhase,
    WaveAblation::ShuffledPhase,
    WaveAblation::MagnitudeOnly,
    WaveAblation::WithoutAntiCenter,
    WaveAblation::SkeletonOnly,
    WaveAblation::RandomCenter,
    WaveAblation::RandomRanking,
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawGrokkingVerdicts {
    pub anonymous_atom_pass: bool,
    pub verifier_feedback_only_pass: bool,
    pub exact_cache_disjoint_pass: bool,
    pub delayed_transfer_pass: bool,
    pub circuit_formation_pass: bool,
    pub cleanup_pass: bool,
    pub phase_causal_pass: bool,
    pub full_execution_pass: bool,
    pub package_budget_pass: bool,
    pub overall_pass: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawGrokkingCheckpoint {
    pub meta_surfaces: usize,
    pub support_traces: usize,
    pub stage: String,
    pub verifier_positive_candidates: usize,
    pub verifier_negative_candidates: usize,
    pub latent_predicate_candidates: usize,
    pub discovered_predicates: usize,
    pub predicate_confidence_milli: usize,
    pub max_predicate_surface_support: usize,
    pub phase_circuit_ready: bool,
    pub exact_memory_entries: usize,
    pub positive_centers: usize,
    pub anti_centers: usize,
    pub wave_memory_bytes: usize,
    pub package_bytes: usize,
    pub max_package_bytes: usize,
    pub package_successes: usize,
    pub package_failures: usize,
    pub correct_cpu_executions: usize,
    pub abstained_queries: usize,
    pub wrong_accepts: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawGrokkingModeReport {
    pub mode: String,
    pub action_groups: usize,
    pub top_k_hits: usize,
    pub exact_rank_sum: usize,
    pub max_exact_rank: usize,
    pub package_successes: usize,
    pub package_failures: usize,
    pub exact_checks: usize,
    pub package_bytes: usize,
    pub max_package_bytes: usize,
    pub route_margins_micro: Vec<i64>,
    pub correct_cpu_executions: usize,
    pub abstained_queries: usize,
    pub wrong_accepts: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseGrokkingProofReport {
    pub report_kind: String,
    pub verdict: String,
    pub verdicts: RawGrokkingVerdicts,
    pub cells: usize,
    pub top_k: usize,
    pub min_cross_surface_support: usize,
    pub min_rate_contrast_milli: usize,
    pub max_package_budget_bytes: usize,
    pub corpus_seeds: usize,
    pub meta_surfaces: usize,
    pub heldout_surfaces: usize,
    pub heldout_queries: usize,
    pub exact_cache_overlap: usize,
    pub raw_atoms_scanned: usize,
    pub concrete_name_leaks: usize,
    pub predicate_name_leaks: usize,
    pub label_leaks: usize,
    pub raw_signature_classes: usize,
    pub ambiguous_signature_classes: usize,
    pub positive_candidates_in_ambiguous_signatures: usize,
    pub checkpoints: Vec<RawGrokkingCheckpoint>,
    pub causal_modes: Vec<RawGrokkingModeReport>,
    pub boundary: String,
}

#[derive(Clone, Debug)]
struct HeldoutSurface {
    support: Vec<TransitionTrace>,
    query: Vec<TransitionTrace>,
}

#[derive(Clone, Debug)]
struct RawCandidateExample {
    surface: usize,
    stable_key: String,
    atoms: Vec<String>,
    verifier_positive: bool,
}

#[derive(Clone, Debug, Default)]
struct RawAtomStat {
    positive_examples: usize,
    negative_examples: usize,
    positive_surfaces: BTreeSet<usize>,
    negative_surfaces: BTreeSet<usize>,
}

#[derive(Clone, Debug)]
struct RawGroundingModel {
    discovered: BTreeSet<String>,
    memory: Option<RelationWaveMemory>,
    ready: bool,
    exact_memory: BTreeSet<String>,
    verifier_positive_candidates: usize,
    verifier_negative_candidates: usize,
    latent_predicate_candidates: usize,
    max_predicate_surface_support: usize,
    confidence_milli: usize,
}

pub fn run_raw_phase_grokking_proof() -> Result<RawPhaseGrokkingProofReport, String> {
    let (meta, concrete_names) = build_meta();
    let heldout = build_heldout();
    let heldout_queries = heldout.iter().map(|surface| surface.query.len()).sum();
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

    let all_examples = collect_examples(&meta)?;
    let (raw_atoms_scanned, concrete_name_leaks, predicate_name_leaks, label_leaks) =
        audit_atoms(&all_examples, &concrete_names);
    let (
        raw_signature_classes,
        ambiguous_signature_classes,
        positive_candidates_in_ambiguous_signatures,
    ) = signature_ambiguity(&all_examples);
    let mut checkpoints = Vec::new();
    let mut final_model = None;
    for count in checkpoint_counts(meta.len()) {
        let examples = all_examples
            .iter()
            .filter(|example| example.surface < count)
            .cloned()
            .collect::<Vec<_>>();
        let model = train_raw_model(&examples, count);
        let mode = evaluate_model(&model, &heldout, WaveAblation::Full)?;
        let wave = model.memory.as_ref().map(RelationWaveMemory::metrics);
        checkpoints.push(RawGrokkingCheckpoint {
            meta_surfaces: count,
            support_traces: meta[..count].iter().map(Vec::len).sum(),
            stage: stage_name(&model, count).to_owned(),
            verifier_positive_candidates: model.verifier_positive_candidates,
            verifier_negative_candidates: model.verifier_negative_candidates,
            latent_predicate_candidates: model.latent_predicate_candidates,
            discovered_predicates: model.discovered.len(),
            predicate_confidence_milli: model.confidence_milli,
            max_predicate_surface_support: model.max_predicate_surface_support,
            phase_circuit_ready: model.ready,
            exact_memory_entries: model.exact_memory.len(),
            positive_centers: wave.map_or(0, |metrics| metrics.center_count),
            anti_centers: wave.map_or(0, |metrics| metrics.negative_center_count),
            wave_memory_bytes: wave.map_or(0, |metrics| metrics.wave_memory_bytes),
            package_bytes: mode.package_bytes,
            max_package_bytes: mode.max_package_bytes,
            package_successes: mode.package_successes,
            package_failures: mode.package_failures,
            correct_cpu_executions: mode.correct_cpu_executions,
            abstained_queries: mode.abstained_queries,
            wrong_accepts: mode.wrong_accepts,
        });
        final_model = Some(model);
    }
    let final_model = final_model.ok_or_else(|| "missing_final_raw_model".to_owned())?;
    let mut causal_modes = Vec::new();
    for mode in MODES {
        causal_modes.push(evaluate_model(&final_model, &heldout, mode)?);
    }

    let first = checkpoints
        .first()
        .ok_or_else(|| "missing_raw_checkpoint".to_owned())?;
    let circuit = checkpoints
        .iter()
        .find(|checkpoint| checkpoint.phase_circuit_ready)
        .ok_or_else(|| "raw_phase_circuit_never_ready".to_owned())?;
    let final_checkpoint = checkpoints
        .last()
        .ok_or_else(|| "missing_raw_checkpoint".to_owned())?;
    let full = mode_report(&causal_modes, WaveAblation::Full)?;
    let anonymous_atom_pass = concrete_name_leaks == 0
        && predicate_name_leaks == 0
        && label_leaks == 0
        && raw_atoms_scanned > 0;
    let verifier_feedback_only_pass = all_examples.iter().any(|example| example.verifier_positive)
        && all_examples
            .iter()
            .any(|example| !example.verifier_positive);
    let exact_cache_disjoint_pass = exact_cache_overlap == 0;
    let delayed_transfer_pass = first.correct_cpu_executions == 0
        && first.abstained_queries == heldout_queries
        && circuit.correct_cpu_executions == heldout_queries;
    let circuit_formation_pass = circuit.discovered_predicates > 0
        && circuit.positive_centers > 0
        && circuit.anti_centers > 0;
    let cleanup_pass = circuit.exact_memory_entries > 0
        && final_checkpoint.exact_memory_entries == 0
        && final_checkpoint.correct_cpu_executions == heldout_queries;
    let phase_causal_pass = MODES
        .iter()
        .copied()
        .filter(|mode| *mode != WaveAblation::Full && *mode != WaveAblation::WithoutAntiCenter)
        .all(|mode| {
            mode_report(&causal_modes, mode).is_ok_and(|candidate| {
                candidate.correct_cpu_executions < full.correct_cpu_executions
                    && candidate.package_failures > full.package_failures
            })
        });
    let full_execution_pass = full.package_successes == heldout.len()
        && full.correct_cpu_executions == heldout_queries
        && full.wrong_accepts == 0
        && full.abstained_queries == 0;
    let package_budget_pass = full.max_package_bytes <= MAX_PACKAGE_BYTES;
    let overall_pass = anonymous_atom_pass
        && verifier_feedback_only_pass
        && exact_cache_disjoint_pass
        && delayed_transfer_pass
        && circuit_formation_pass
        && cleanup_pass
        && phase_causal_pass
        && full_execution_pass
        && package_budget_pass;

    Ok(RawPhaseGrokkingProofReport {
        report_kind: "nando_raw_phase_grokking_proof_v1".to_owned(),
        verdict: if overall_pass {
            "RAW_PHASE_GROKKING_CANDIDATE_PASS"
        } else {
            "RAW_PHASE_GROKKING_CANDIDATE_FAIL"
        }
        .to_owned(),
        verdicts: RawGrokkingVerdicts {
            anonymous_atom_pass,
            verifier_feedback_only_pass,
            exact_cache_disjoint_pass,
            delayed_transfer_pass,
            circuit_formation_pass,
            cleanup_pass,
            phase_causal_pass,
            full_execution_pass,
            package_budget_pass,
            overall_pass,
        },
        cells: CELLS,
        top_k: TOP_K,
        min_cross_surface_support: MIN_CROSS_SURFACE_SUPPORT,
        min_rate_contrast_milli: MIN_RATE_CONTRAST_MILLI,
        max_package_budget_bytes: MAX_PACKAGE_BYTES,
        corpus_seeds: corpus_seeds().len(),
        meta_surfaces: meta.len(),
        heldout_surfaces: heldout.len(),
        heldout_queries,
        exact_cache_overlap,
        raw_atoms_scanned,
        concrete_name_leaks,
        predicate_name_leaks,
        label_leaks,
        raw_signature_classes,
        ambiguous_signature_classes,
        positive_candidates_in_ambiguous_signatures,
        checkpoints,
        causal_modes,
        boundary: "frozen synthetic weak-observation proof; primitive comparison vocabulary and typed actor skeleton library are fixed; relation predicate polarity and combinations are discovered from independent actor-result equality across renamed surfaces; the three-surface evidence threshold and cleanup policy are explicit inductive biases; proves a grokking-like consolidation candidate, not spontaneous neural grokking or live organic traffic".to_owned(),
    })
}

fn collect_examples(meta: &[Vec<TransitionTrace>]) -> Result<Vec<RawCandidateExample>, String> {
    let mut examples = Vec::new();
    for (surface, traces) in meta.iter().enumerate() {
        let first = traces
            .first()
            .ok_or_else(|| "empty_raw_meta_surface".to_owned())?;
        let shape = discover_surface(&first.before).map_err(str::to_owned)?;
        let (kind_path, groups) = discover_action_groups(traces).map_err(str::to_owned)?;
        for (concrete_kind, group) in groups {
            let evidence = RelationEvidenceIndex::new(&shape, &group).map_err(str::to_owned)?;
            for hypothesis in enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group) {
                examples.push(RawCandidateExample {
                    surface,
                    stable_key: hypothesis.stable_key(),
                    atoms: candidate_raw_basis(&evidence, &hypothesis),
                    verifier_positive: hypothesis.exact_on(&group),
                });
            }
        }
    }
    Ok(examples)
}

fn train_raw_model(examples: &[RawCandidateExample], surfaces: usize) -> RawGroundingModel {
    let positive_total = examples
        .iter()
        .filter(|example| example.verifier_positive)
        .count();
    let negative_total = examples.len().saturating_sub(positive_total);
    let mut stats = BTreeMap::<String, RawAtomStat>::new();
    for example in examples {
        for atom in &example.atoms {
            let stat = stats.entry(atom.clone()).or_default();
            if example.verifier_positive {
                stat.positive_examples += 1;
                stat.positive_surfaces.insert(example.surface);
            } else {
                stat.negative_examples += 1;
                stat.negative_surfaces.insert(example.surface);
            }
        }
    }
    let mut contrasts = Vec::new();
    let mut latent_predicate_candidates = 0;
    let mut max_predicate_surface_support = 0;
    let discovered = stats
        .into_iter()
        .filter_map(|(atom, stat)| {
            let positive_rate = ratio_milli(stat.positive_examples, positive_total);
            let negative_rate = ratio_milli(stat.negative_examples, negative_total);
            let contrast = positive_rate.saturating_sub(negative_rate);
            let cross_surface = stat
                .positive_surfaces
                .len()
                .max(stat.negative_surfaces.len());
            max_predicate_surface_support = max_predicate_surface_support.max(cross_surface);
            if contrast >= MIN_RATE_CONTRAST_MILLI {
                contrasts.push(contrast);
                latent_predicate_candidates += 1;
            }
            (cross_surface >= MIN_CROSS_SURFACE_SUPPORT && contrast >= MIN_RATE_CONTRAST_MILLI)
                .then_some(atom)
        })
        .collect::<BTreeSet<_>>();
    let ready = surfaces >= MIN_CROSS_SURFACE_SUPPORT && !discovered.is_empty();
    let memory = ready.then(|| {
        let wave_examples = examples
            .iter()
            .map(|example| WaveTrainingExample {
                atoms: filtered_atoms(example.atoms.clone(), &discovered),
                valid: example.verifier_positive,
            })
            .collect::<Vec<_>>();
        RelationWaveMemory::train(&wave_examples, CELLS, SPLIT_THRESHOLD)
    });
    let exact_memory = if surfaces > MIN_CROSS_SURFACE_SUPPORT {
        BTreeSet::new()
    } else {
        examples
            .iter()
            .filter(|example| example.verifier_positive)
            .map(|example| example.stable_key.clone())
            .collect()
    };
    RawGroundingModel {
        discovered,
        memory,
        ready,
        exact_memory,
        verifier_positive_candidates: positive_total,
        verifier_negative_candidates: negative_total,
        latent_predicate_candidates,
        max_predicate_surface_support,
        confidence_milli: if contrasts.is_empty() {
            0
        } else {
            contrasts.iter().sum::<usize>() / contrasts.len()
        },
    }
}

fn evaluate_model(
    model: &RawGroundingModel,
    heldout: &[HeldoutSurface],
    mode: WaveAblation,
) -> Result<RawGrokkingModeReport, String> {
    let mut report = RawGrokkingModeReport {
        mode: mode_name(mode).to_owned(),
        ..RawGrokkingModeReport::default()
    };
    let Some(memory) = model.memory.as_ref().filter(|_| model.ready) else {
        report.package_failures = heldout.len();
        report.abstained_queries = heldout.iter().map(|surface| surface.query.len()).sum();
        return Ok(report);
    };
    for surface in heldout {
        let Some(package) = induce_raw_package(model, memory, &surface.support, mode, &mut report)?
        else {
            report.package_failures += 1;
            report.abstained_queries += surface.query.len();
            continue;
        };
        report.package_successes += 1;
        let package_bytes = package
            .artifact_bytes()
            .map_err(|error| format!("raw_package_serialize:{error}"))?
            .len();
        report.package_bytes += package_bytes;
        report.max_package_bytes = report.max_package_bytes.max(package_bytes);
        report
            .route_margins_micro
            .extend((0..package.transitions.len()).filter_map(|index| package.route_margin(index)));
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
    Ok(report)
}

fn induce_raw_package(
    model: &RawGroundingModel,
    memory: &RelationWaveMemory,
    support: &[TransitionTrace],
    mode: WaveAblation,
    report: &mut RawGrokkingModeReport,
) -> Result<Option<InducedTransitionPackage>, String> {
    let first = support
        .first()
        .ok_or_else(|| "empty_raw_support".to_owned())?;
    let shape = discover_surface(&first.before).map_err(str::to_owned)?;
    let (kind_path, groups) = discover_action_groups(support).map_err(str::to_owned)?;
    let mut transitions = Vec::new();
    let mut package_key = Vec::new();
    for (concrete_kind, group) in groups {
        report.action_groups += 1;
        let evidence = RelationEvidenceIndex::new(&shape, &group).map_err(str::to_owned)?;
        let hypotheses = enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group);
        let mut ranked = hypotheses
            .iter()
            .enumerate()
            .map(|(index, hypothesis)| {
                let atoms = filtered_atoms(
                    candidate_raw_basis(&evidence, hypothesis),
                    &model.discovered,
                );
                let score = if mode == WaveAblation::RandomRanking {
                    stable_hash(hypothesis.stable_key().as_bytes()) as i64
                } else {
                    memory.score_atoms_with_ablation(&atoms, mode)
                };
                (score, index, atoms)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| hypotheses[left.1].cmp(&hypotheses[right.1]))
        });
        let exact_rank = ranked
            .iter()
            .position(|(_, index, _)| hypotheses[*index].exact_on(&group))
            .map(|rank| rank + 1)
            .ok_or_else(|| format!("raw_exact_hypothesis_missing:{concrete_kind}"))?;
        report.exact_rank_sum += exact_rank;
        report.max_exact_rank = report.max_exact_rank.max(exact_rank);
        report.top_k_hits += usize::from(exact_rank <= TOP_K);
        let mut chosen = None;
        for (score, index, atoms) in ranked.into_iter().take(TOP_K) {
            report.exact_checks += 1;
            if hypotheses[index].exact_on(&group) {
                chosen = Some((score, hypotheses[index].clone(), atoms));
                break;
            }
        }
        let Some((score, hypothesis, atoms)) = chosen else {
            return Ok(None);
        };
        let (program, adapter) = hypothesis.compile_actor();
        let (guard, verifier, _) = synthesize_contracts(&hypothesis, &group)
            .map_err(|error| format!("raw_contract_synthesis:{concrete_kind}:{error}"))?;
        package_key.extend_from_slice(hypothesis.stable_key().as_bytes());
        transitions.push(InducedTransition {
            action_surface: concrete_kind,
            program,
            adapter,
            guard,
            verifier,
            routing_atoms: atoms,
            routing_atom_ids: Vec::new(),
            wave_margin_micro: score,
            support_traces: group.len(),
        });
    }
    Ok(Some(InducedTransitionPackage {
        schema: "nando.induced-transition-package.v1".to_owned(),
        package_id: format!("raw-a4-{:016x}", stable_hash(&package_key)),
        transitions,
        wave_memory_bytes: memory.bytes_estimate(),
        routing_signature: memory.portable_signature(),
    }))
}

fn filtered_atoms(atoms: Vec<String>, discovered: &BTreeSet<String>) -> Vec<String> {
    atoms
        .into_iter()
        .filter(|atom| discovered.contains(atom))
        .collect()
}

fn expand_raw_predicate_basis(atoms: Vec<String>) -> Vec<String> {
    let primitive = atoms
        .iter()
        .filter(|atom| atom.starts_with("raw_delta_slot:") || atom.starts_with("program_shape:"))
        .cloned()
        .collect::<Vec<_>>();
    let mut expanded = atoms;
    for left in 0..primitive.len() {
        for right in (left + 1)..primitive.len() {
            expanded.push(format!(
                "raw_delta_pair:{}&{}",
                primitive[left], primitive[right]
            ));
        }
    }
    if let Some(program_shape) = primitive
        .iter()
        .find(|atom| atom.starts_with("program_shape:"))
    {
        let slots = primitive
            .iter()
            .filter(|atom| atom.starts_with("raw_delta_slot:"))
            .collect::<Vec<_>>();
        for left in 0..slots.len() {
            for right in (left + 1)..slots.len() {
                expanded.push(format!(
                    "raw_delta_triplet:{}&{}&{}",
                    program_shape, slots[left], slots[right]
                ));
            }
        }
    }
    expanded
}

fn candidate_raw_basis(
    evidence: &RelationEvidenceIndex,
    hypothesis: &RoleHypothesis,
) -> Vec<String> {
    let mut atoms = evidence.raw_delta_atoms(hypothesis, None);
    atoms.push(format!(
        "program_shape:{}",
        program_shape_id(hypothesis.skeleton)
    ));
    expand_raw_predicate_basis(atoms)
}

const fn program_shape_id(skeleton: OperatorSkeleton) -> u8 {
    match skeleton {
        OperatorSkeleton::SetField => 0,
        OperatorSkeleton::IncrementField => 1,
        OperatorSkeleton::AppendRecord => 2,
        OperatorSkeleton::DeleteRecord => 3,
    }
}

fn audit_atoms(
    examples: &[RawCandidateExample],
    concrete_names: &BTreeSet<String>,
) -> (usize, usize, usize, usize) {
    let predicate_names = [
        "root_valid",
        "target_unique",
        "target_field_changed",
        "only_target_changed",
        "set_effect",
        "increment_effect",
        "append_effect",
        "delete_effect",
        "operand_type_matches",
        "no_op",
    ];
    let label_names = ["valid", "invalid", "exact", "accept", "reject", "correct"];
    let mut scanned = 0;
    let mut concrete_leaks = 0;
    let mut predicate_leaks = 0;
    let mut label_leaks = 0;
    for atom in examples.iter().flat_map(|example| example.atoms.iter()) {
        scanned += 1;
        concrete_leaks += usize::from(concrete_names.iter().any(|name| atom.contains(name)));
        predicate_leaks += usize::from(predicate_names.iter().any(|name| atom.contains(name)));
        label_leaks += usize::from(label_names.iter().any(|name| atom.contains(name)));
    }
    (scanned, concrete_leaks, predicate_leaks, label_leaks)
}

fn signature_ambiguity(examples: &[RawCandidateExample]) -> (usize, usize, usize) {
    let mut classes = BTreeMap::<Vec<String>, (usize, usize)>::new();
    for example in examples {
        let counts = classes.entry(example.atoms.clone()).or_default();
        if example.verifier_positive {
            counts.0 += 1;
        } else {
            counts.1 += 1;
        }
    }
    let ambiguous = classes
        .values()
        .filter(|(positive, negative)| *positive > 0 && *negative > 0)
        .count();
    let positive_ambiguous = classes
        .values()
        .filter(|(positive, negative)| *positive > 0 && *negative > 0)
        .map(|(positive, _)| *positive)
        .sum();
    (classes.len(), ambiguous, positive_ambiguous)
}

fn build_meta() -> (Vec<Vec<TransitionTrace>>, BTreeSet<String>) {
    let mut names = BTreeSet::new();
    let surfaces = corpus_seeds()
        .iter()
        .enumerate()
        .flat_map(|(seed_index, corpus_seed)| {
            (0..meta_surfaces_per_seed()).map(move |index| {
                let layout = if index % 2 == 0 {
                    LayoutShape::Map
                } else {
                    LayoutShape::List
                };
                A2SurfaceSpec::new(seed_index * 1_000 + index, layout, *corpus_seed)
            })
        })
        .collect::<Vec<_>>();
    for spec in &surfaces {
        names.extend([
            spec.outer.clone(),
            spec.root.clone(),
            spec.id.clone(),
            spec.status.clone(),
            spec.count.clone(),
            spec.owner.clone(),
            spec.note.clone(),
            spec.command.clone(),
            spec.kind.clone(),
            spec.target.clone(),
            spec.value.clone(),
            spec.amount.clone(),
            spec.set_kind.clone(),
            spec.increment_kind.clone(),
            spec.append_kind.clone(),
            spec.delete_kind.clone(),
        ]);
    }
    let meta = surfaces
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            traces_for(spec, index * 100_000, SUPPORT_PER_OPERATOR)
                .iter()
                .map(|trace| weak_observe(trace, spec))
                .collect()
        })
        .collect();
    (meta, names)
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
                        40_000 + seed_index * 10 + layout_index,
                        layout,
                        *corpus_seed,
                    );
                    let support = traces_for(
                        &spec,
                        7_000_000 + seed_index * 100_000,
                        SUPPORT_PER_OPERATOR,
                    );
                    HeldoutSurface {
                        support: support
                            .iter()
                            .map(|trace| weak_observe(trace, &spec))
                            .collect(),
                        query: traces_for(
                            &spec,
                            8_000_000 + seed_index * 100_000,
                            query_per_operator(),
                        ),
                    }
                })
        })
        .collect()
}

fn checkpoint_counts(meta_len: usize) -> Vec<usize> {
    let mut counts = [1usize, 2, 3, 4, 8, 12, meta_len]
        .into_iter()
        .filter(|count| *count <= meta_len)
        .collect::<Vec<_>>();
    counts.sort_unstable();
    counts.dedup();
    counts
}

fn stage_name(model: &RawGroundingModel, surfaces: usize) -> &'static str {
    if !model.ready {
        "memorization"
    } else if surfaces == MIN_CROSS_SURFACE_SUPPORT {
        "circuit_formation"
    } else {
        "cleanup"
    }
}

fn mode_report(
    reports: &[RawGrokkingModeReport],
    mode: WaveAblation,
) -> Result<&RawGrokkingModeReport, String> {
    let name = mode_name(mode);
    reports
        .iter()
        .find(|report| report.mode == name)
        .ok_or_else(|| format!("missing_raw_mode:{name}"))
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

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
        (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
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
