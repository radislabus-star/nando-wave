//! Reusable cold-path raw-delta learner for autonomous profile induction.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::hypothesis::{
    OperatorSkeleton, RelationEvidenceIndex, RoleHypothesis, discover_action_groups,
    enumerate_hypotheses,
};
use crate::package::{InducedTransition, InducedTransitionPackage};
use crate::synthesis::synthesize_contracts;
use crate::trace::{TransitionTrace, discover_surface, records_for};
use crate::{RelationWaveMemory, WaveContributionMetrics, WaveIdTrainingExample};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseConfig {
    pub cells: usize,
    pub top_k: usize,
    pub split_threshold: f64,
    pub min_cross_surface_support: usize,
    pub min_rate_contrast_milli: usize,
}

impl RawPhaseConfig {
    #[must_use]
    pub const fn live_v1() -> Self {
        Self {
            cells: 16,
            top_k: 48,
            split_threshold: 0.90,
            min_cross_surface_support: 3,
            min_rate_contrast_milli: 50,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseTrainingMetrics {
    pub surfaces: usize,
    pub support_traces: usize,
    pub verifier_positive_candidates: usize,
    pub verifier_negative_candidates: usize,
    pub compact_predicate_candidates: usize,
    pub discovered_predicates: usize,
    pub max_predicate_surface_support: usize,
    pub predicate_confidence_milli: usize,
    pub phase_circuit_ready: bool,
    pub training_cpu_ns: u64,
    pub wave: WaveContributionMetrics,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseInductionMetrics {
    pub action_groups: usize,
    pub hypotheses_generated: usize,
    pub guided_exact_checks: usize,
    pub phase_top_k_hits: usize,
    pub package_bytes: usize,
    pub induction_cpu_ns: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawPhaseTransferMetrics {
    pub tested_surfaces: usize,
    pub passed_surfaces: usize,
    pub adaptation_rows: usize,
    pub query_rows: usize,
    pub correct_executions: usize,
    pub abstains: usize,
    pub wrong_accepts: usize,
    pub leave_one_surface_out_pass: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RawPhaseInductionError {
    EmptyCorpus,
    Surface(String),
    ActionGrouping(String),
    CircuitNotReady,
    WaveTopKMiss(String),
    NonPositiveMargin(String),
    ContractSynthesis(String),
    PackageSerialization,
}

#[derive(Clone, Debug)]
struct CompactCandidateExample {
    surface: usize,
    atom_ids: Vec<u64>,
    verifier_positive: bool,
}

#[derive(Clone, Debug, Default)]
struct CompactPredicateStat {
    positive_examples: usize,
    negative_examples: usize,
    positive_surfaces: BTreeSet<usize>,
    negative_surfaces: BTreeSet<usize>,
}

#[derive(Clone, Debug)]
pub struct RawPhaseInducer {
    config: RawPhaseConfig,
    model_id: u64,
    discovered: BTreeSet<u64>,
    memory: Option<RelationWaveMemory>,
    metrics: RawPhaseTrainingMetrics,
}

pub fn transition_surface_key(before: &Value, _action: &Value) -> Result<String, &'static str> {
    let shape = discover_surface(before)?;
    let mut schema = Vec::new();
    schema.push(match shape.layout {
        crate::LayoutShape::Map => 0,
        crate::LayoutShape::List => 1,
        crate::LayoutShape::Columns => 2,
    });
    append_strings(&shape.root_path, &mut schema);
    append_strings(&shape.record_fields, &mut schema);
    append_strings(&shape.id_sources, &mut schema);
    Ok(format!("{:016x}", stable_hash(&schema)))
}

pub fn transition_family_key(before: &Value) -> Result<String, &'static str> {
    let shape = discover_surface(before)?;
    let records = records_for(before, &shape)?;
    let mut value_kinds = records
        .first()
        .map(|record| {
            record
                .fields
                .values()
                .map(value_kind_id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if shape.layout == crate::LayoutShape::Map {
        value_kinds.push(b's');
    }
    value_kinds.sort_unstable();
    let mut schema = b"raw-family-v1".to_vec();
    schema.extend_from_slice(&(value_kinds.len() as u64).to_le_bytes());
    schema.extend_from_slice(&value_kinds);
    Ok(format!("{:016x}", stable_hash(&schema)))
}

fn value_kind_id(value: &Value) -> u8 {
    match value {
        Value::Null => b'0',
        Value::Bool(_) => b'b',
        Value::Number(_) => b'n',
        Value::String(_) => b's',
        Value::Array(_) => b'a',
        Value::Object(_) => b'o',
    }
}

impl RawPhaseInducer {
    pub fn train(
        meta_batches: &[Vec<TransitionTrace>],
        config: RawPhaseConfig,
    ) -> Result<Self, RawPhaseInductionError> {
        let started = Instant::now();
        if meta_batches.is_empty() {
            return Err(RawPhaseInductionError::EmptyCorpus);
        }
        let examples = collect_examples(meta_batches)?;
        let positive_total = examples
            .iter()
            .filter(|example| example.verifier_positive)
            .count();
        let negative_total = examples.len().saturating_sub(positive_total);
        let mut stats = BTreeMap::<u64, CompactPredicateStat>::new();
        for example in &examples {
            for atom_id in &example.atom_ids {
                let stat = stats.entry(*atom_id).or_default();
                if example.verifier_positive {
                    stat.positive_examples += 1;
                    stat.positive_surfaces.insert(example.surface);
                } else {
                    stat.negative_examples += 1;
                    stat.negative_surfaces.insert(example.surface);
                }
            }
        }
        let compact_predicate_candidates = stats.len();
        let mut contrasts = Vec::new();
        let mut max_predicate_surface_support = 0;
        let discovered = stats
            .into_iter()
            .filter_map(|(atom_id, stat)| {
                let positive_rate = ratio_milli(stat.positive_examples, positive_total);
                let negative_rate = ratio_milli(stat.negative_examples, negative_total);
                let contrast = positive_rate.saturating_sub(negative_rate);
                let cross_surface = stat
                    .positive_surfaces
                    .len()
                    .max(stat.negative_surfaces.len());
                max_predicate_surface_support = max_predicate_surface_support.max(cross_surface);
                if contrast >= config.min_rate_contrast_milli {
                    contrasts.push(contrast);
                }
                (cross_surface >= config.min_cross_surface_support
                    && contrast >= config.min_rate_contrast_milli)
                    .then_some(atom_id)
            })
            .collect::<BTreeSet<_>>();
        let ready = meta_batches.len() >= config.min_cross_surface_support
            && positive_total > 0
            && negative_total > 0
            && !discovered.is_empty();
        let memory = ready.then(|| {
            let wave_examples = examples
                .iter()
                .map(|example| WaveIdTrainingExample {
                    atom_ids: filtered_ids(&example.atom_ids, &discovered),
                    valid: example.verifier_positive,
                })
                .collect::<Vec<_>>();
            RelationWaveMemory::train_ids(&wave_examples, config.cells, config.split_threshold)
        });
        let wave = memory
            .as_ref()
            .map(RelationWaveMemory::metrics)
            .unwrap_or_default();
        let model_id = memory.as_ref().map_or(0, |memory| {
            serde_json::to_vec(&memory.portable_signature())
                .map(|bytes| stable_hash(&bytes))
                .unwrap_or(0)
        });
        let metrics = RawPhaseTrainingMetrics {
            surfaces: meta_batches.len(),
            support_traces: meta_batches.iter().map(Vec::len).sum(),
            verifier_positive_candidates: positive_total,
            verifier_negative_candidates: negative_total,
            compact_predicate_candidates,
            discovered_predicates: discovered.len(),
            max_predicate_surface_support,
            predicate_confidence_milli: if contrasts.is_empty() {
                0
            } else {
                contrasts.iter().sum::<usize>() / contrasts.len()
            },
            phase_circuit_ready: ready,
            training_cpu_ns: nanos_u64(started.elapsed().as_nanos()),
            wave,
        };
        Ok(Self {
            config,
            model_id,
            discovered,
            memory,
            metrics,
        })
    }

    #[must_use]
    pub fn metrics(&self) -> &RawPhaseTrainingMetrics {
        &self.metrics
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.metrics.phase_circuit_ready
    }

    pub fn induce(
        &self,
        support: &[TransitionTrace],
    ) -> Result<(InducedTransitionPackage, RawPhaseInductionMetrics), RawPhaseInductionError> {
        let started = Instant::now();
        let memory = self
            .memory
            .as_ref()
            .filter(|_| self.is_ready())
            .ok_or(RawPhaseInductionError::CircuitNotReady)?;
        let first = support.first().ok_or(RawPhaseInductionError::EmptyCorpus)?;
        let shape = discover_surface(&first.before)
            .map_err(|error| RawPhaseInductionError::Surface(error.to_owned()))?;
        let (kind_path, groups) = discover_action_groups(support)
            .map_err(|error| RawPhaseInductionError::ActionGrouping(error.to_owned()))?;
        let mut metrics = RawPhaseInductionMetrics {
            action_groups: groups.len(),
            ..RawPhaseInductionMetrics::default()
        };
        let mut transitions = Vec::new();
        let mut package_key = self.model_id.to_le_bytes().to_vec();
        for (concrete_kind, group) in groups {
            let evidence = RelationEvidenceIndex::new(&shape, &group)
                .map_err(|error| RawPhaseInductionError::Surface(error.to_owned()))?;
            let hypotheses = enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group);
            metrics.hypotheses_generated += hypotheses.len();
            let mut ranked = hypotheses
                .iter()
                .enumerate()
                .map(|(index, hypothesis)| {
                    let atom_ids = filtered_ids(
                        &candidate_compact_basis(&evidence, hypothesis),
                        &self.discovered,
                    );
                    (memory.score_atom_ids(&atom_ids), index, atom_ids)
                })
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| {
                right
                    .0
                    .cmp(&left.0)
                    .then_with(|| hypotheses[left.1].cmp(&hypotheses[right.1]))
            });
            let mut chosen = None;
            for (margin, index, atom_ids) in ranked.into_iter().take(self.config.top_k.max(1)) {
                metrics.guided_exact_checks += 1;
                if hypotheses[index].exact_on(&group) {
                    metrics.phase_top_k_hits += 1;
                    chosen = Some((margin, hypotheses[index].clone(), atom_ids));
                    break;
                }
            }
            let Some((margin, hypothesis, atom_ids)) = chosen else {
                return Err(RawPhaseInductionError::WaveTopKMiss(concrete_kind));
            };
            if margin <= 0 {
                return Err(RawPhaseInductionError::NonPositiveMargin(concrete_kind));
            }
            let (program, adapter) = hypothesis.compile_actor();
            let (guard, verifier, _) =
                synthesize_contracts(&hypothesis, &group).map_err(|error| {
                    RawPhaseInductionError::ContractSynthesis(format!("{concrete_kind}:{error}"))
                })?;
            package_key.extend_from_slice(hypothesis.stable_key().as_bytes());
            for atom_id in &atom_ids {
                package_key.extend_from_slice(&atom_id.to_le_bytes());
            }
            transitions.push(InducedTransition {
                action_surface: concrete_kind,
                program,
                adapter,
                guard,
                verifier,
                routing_atoms: Vec::new(),
                routing_atom_ids: atom_ids,
                wave_margin_micro: margin,
                support_traces: group.len(),
            });
        }
        let package = InducedTransitionPackage {
            schema: "nando.induced-transition-package.v1".to_owned(),
            package_id: format!("raw-live-v1-{:016x}", stable_hash(&package_key)),
            transitions,
            wave_memory_bytes: memory.bytes_estimate(),
            routing_signature: memory.portable_signature(),
        };
        metrics.package_bytes = package
            .artifact_bytes()
            .map_err(|_| RawPhaseInductionError::PackageSerialization)?
            .len();
        metrics.induction_cpu_ns = nanos_u64(started.elapsed().as_nanos());
        Ok((package, metrics))
    }
}

pub fn evaluate_leave_one_surface_out(
    meta_batches: &[Vec<TransitionTrace>],
    config: RawPhaseConfig,
) -> Result<RawPhaseTransferMetrics, RawPhaseInductionError> {
    let mut metrics = RawPhaseTransferMetrics::default();
    if meta_batches.len() <= config.min_cross_surface_support {
        return Ok(metrics);
    }
    for heldout_index in 0..meta_batches.len() {
        let training = meta_batches
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != heldout_index)
            .map(|(_, traces)| traces.clone())
            .collect::<Vec<_>>();
        let inducer = RawPhaseInducer::train(&training, config)?;
        if !inducer.is_ready() {
            continue;
        }
        let (adaptation, query) = split_adaptation_query(&meta_batches[heldout_index])?;
        metrics.tested_surfaces = metrics.tested_surfaces.saturating_add(1);
        metrics.adaptation_rows = metrics.adaptation_rows.saturating_add(adaptation.len());
        metrics.query_rows = metrics.query_rows.saturating_add(query.len());
        let (package, _) = match inducer.induce(&adaptation) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let mut surface_pass = true;
        for trace in query {
            let execution = package.execute_routed(&trace.before, &trace.action);
            match execution.status {
                crate::InducedExecutionStatus::Executed
                    if execution.after.as_ref() == Some(&trace.after) =>
                {
                    metrics.correct_executions = metrics.correct_executions.saturating_add(1);
                }
                crate::InducedExecutionStatus::Executed => {
                    metrics.wrong_accepts = metrics.wrong_accepts.saturating_add(1);
                    surface_pass = false;
                }
                crate::InducedExecutionStatus::Abstain
                | crate::InducedExecutionStatus::VerifyFailed => {
                    metrics.abstains = metrics.abstains.saturating_add(1);
                    surface_pass = false;
                }
            }
        }
        if surface_pass {
            metrics.passed_surfaces = metrics.passed_surfaces.saturating_add(1);
        }
    }
    metrics.leave_one_surface_out_pass = metrics.tested_surfaces == meta_batches.len()
        && metrics.passed_surfaces == metrics.tested_surfaces
        && metrics.query_rows > 0
        && metrics.wrong_accepts == 0
        && metrics.abstains == 0;
    Ok(metrics)
}

pub fn evaluate_support_query_transfer(
    adaptation_batches: &[Vec<TransitionTrace>],
    query_batches: &[Vec<TransitionTrace>],
    config: RawPhaseConfig,
) -> Result<RawPhaseTransferMetrics, RawPhaseInductionError> {
    let mut metrics = RawPhaseTransferMetrics::default();
    if adaptation_batches.len() < config.min_cross_surface_support
        || adaptation_batches.len() != query_batches.len()
        || adaptation_batches.iter().any(Vec::is_empty)
        || query_batches.iter().any(Vec::is_empty)
    {
        return Ok(metrics);
    }
    let inducer = RawPhaseInducer::train(adaptation_batches, config)?;
    if !inducer.is_ready() {
        return Ok(metrics);
    }
    for (adaptation, query) in adaptation_batches.iter().zip(query_batches) {
        metrics.tested_surfaces = metrics.tested_surfaces.saturating_add(1);
        metrics.adaptation_rows = metrics.adaptation_rows.saturating_add(adaptation.len());
        metrics.query_rows = metrics.query_rows.saturating_add(query.len());
        let (package, _) = match inducer.induce(adaptation) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let mut surface_pass = true;
        for trace in query {
            let execution = package.execute_routed(&trace.before, &trace.action);
            match execution.status {
                crate::InducedExecutionStatus::Executed
                    if execution.after.as_ref() == Some(&trace.after) =>
                {
                    metrics.correct_executions = metrics.correct_executions.saturating_add(1);
                }
                crate::InducedExecutionStatus::Executed => {
                    metrics.wrong_accepts = metrics.wrong_accepts.saturating_add(1);
                    surface_pass = false;
                }
                crate::InducedExecutionStatus::Abstain
                | crate::InducedExecutionStatus::VerifyFailed => {
                    metrics.abstains = metrics.abstains.saturating_add(1);
                    surface_pass = false;
                }
            }
        }
        if surface_pass {
            metrics.passed_surfaces = metrics.passed_surfaces.saturating_add(1);
        }
    }
    metrics.leave_one_surface_out_pass = metrics.tested_surfaces == adaptation_batches.len()
        && metrics.passed_surfaces == metrics.tested_surfaces
        && metrics.query_rows > 0
        && metrics.wrong_accepts == 0
        && metrics.abstains == 0;
    Ok(metrics)
}

pub fn split_forward_adaptation_query(
    traces: &[TransitionTrace],
) -> Result<(Vec<TransitionTrace>, Vec<TransitionTrace>), RawPhaseInductionError> {
    split_adaptation_query(traces)
}

fn split_adaptation_query(
    traces: &[TransitionTrace],
) -> Result<(Vec<TransitionTrace>, Vec<TransitionTrace>), RawPhaseInductionError> {
    let (_, groups) = discover_action_groups(traces)
        .map_err(|error| RawPhaseInductionError::ActionGrouping(error.to_owned()))?;
    let mut adaptation = Vec::new();
    let mut query = Vec::new();
    for group in groups.into_values() {
        if group.len() < 3 {
            return Err(RawPhaseInductionError::EmptyCorpus);
        }
        let split = (group.len() * 2 / 3).clamp(2, group.len() - 1);
        adaptation.extend_from_slice(&group[..split]);
        query.extend_from_slice(&group[split..]);
    }
    Ok((adaptation, query))
}

fn collect_examples(
    meta_batches: &[Vec<TransitionTrace>],
) -> Result<Vec<CompactCandidateExample>, RawPhaseInductionError> {
    let mut examples = Vec::new();
    for (surface, traces) in meta_batches.iter().enumerate() {
        let first = traces.first().ok_or(RawPhaseInductionError::EmptyCorpus)?;
        let shape = discover_surface(&first.before)
            .map_err(|error| RawPhaseInductionError::Surface(error.to_owned()))?;
        let (kind_path, groups) = discover_action_groups(traces)
            .map_err(|error| RawPhaseInductionError::ActionGrouping(error.to_owned()))?;
        for (concrete_kind, group) in groups {
            let evidence = RelationEvidenceIndex::new(&shape, &group)
                .map_err(|error| RawPhaseInductionError::Surface(error.to_owned()))?;
            for hypothesis in enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group) {
                examples.push(CompactCandidateExample {
                    surface,
                    atom_ids: candidate_compact_basis(&evidence, &hypothesis),
                    verifier_positive: hypothesis.exact_on(&group),
                });
            }
        }
    }
    Ok(examples)
}

fn candidate_compact_basis(
    evidence: &RelationEvidenceIndex,
    hypothesis: &RoleHypothesis,
) -> Vec<u64> {
    let mut primitive = evidence
        .raw_delta_atoms(hypothesis, None)
        .into_iter()
        .filter(|atom| atom.starts_with("raw_delta_slot:"))
        .map(|atom| stable_hash(atom.as_bytes()))
        .collect::<Vec<_>>();
    let program_shape = hash_parts(0x20, &[u64::from(program_shape_id(hypothesis.skeleton))]);
    primitive.push(program_shape);
    let mut expanded = primitive.clone();
    for left in 0..primitive.len() {
        for right in (left + 1)..primitive.len() {
            expanded.push(hash_parts(0x21, &[primitive[left], primitive[right]]));
        }
    }
    let slots = &primitive[..primitive.len().saturating_sub(1)];
    for left in 0..slots.len() {
        for right in (left + 1)..slots.len() {
            expanded.push(hash_parts(
                0x22,
                &[program_shape, slots[left], slots[right]],
            ));
        }
    }
    expanded.sort_unstable();
    expanded.dedup();
    expanded
}

fn filtered_ids(atom_ids: &[u64], discovered: &BTreeSet<u64>) -> Vec<u64> {
    atom_ids
        .iter()
        .copied()
        .filter(|atom_id| discovered.contains(atom_id))
        .collect()
}

const fn program_shape_id(skeleton: OperatorSkeleton) -> u8 {
    match skeleton {
        OperatorSkeleton::SetField => 0,
        OperatorSkeleton::IncrementField => 1,
        OperatorSkeleton::AppendRecord => 2,
        OperatorSkeleton::DeleteRecord => 3,
    }
}

fn hash_parts(domain: u8, parts: &[u64]) -> u64 {
    let mut bytes = Vec::with_capacity(1 + parts.len() * 8);
    bytes.push(domain);
    for part in parts {
        bytes.extend_from_slice(&part.to_le_bytes());
    }
    stable_hash(&bytes)
}

fn append_strings(values: &[String], output: &mut Vec<u8>) {
    output.extend_from_slice(&(values.len() as u64).to_le_bytes());
    for value in values {
        output.extend_from_slice(&(value.len() as u64).to_le_bytes());
        output.extend_from_slice(value.as_bytes());
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |state, byte| {
        (state ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (numerator.saturating_mul(1_000) + denominator / 2) / denominator
}

fn nanos_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2_fixture::{A2SurfaceSpec, traces_for, weak_observe};
    use crate::{InducedExecutionStatus, LayoutShape};

    #[test]
    fn compact_raw_phase_inducer_transfers_to_renamed_surface() {
        let meta = (0..6)
            .map(|index| {
                let layout = if index % 2 == 0 {
                    LayoutShape::Map
                } else {
                    LayoutShape::List
                };
                let spec = A2SurfaceSpec::new(index, layout, 0x7a41);
                traces_for(&spec, index * 10_000, 12)
                    .iter()
                    .map(|trace| weak_observe(trace, &spec))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let inducer = RawPhaseInducer::train(&meta, RawPhaseConfig::live_v1()).expect("train");
        assert!(inducer.is_ready(), "{:#?}", inducer.metrics());

        let heldout = A2SurfaceSpec::new(90_000, LayoutShape::Columns, 0x7a41);
        let support = traces_for(&heldout, 1_000_000, 12);
        let query = traces_for(&heldout, 2_000_000, 20);
        let (package, metrics) = inducer.induce(&support).expect("induce");
        assert_eq!(metrics.phase_top_k_hits, 4, "{metrics:#?}");
        assert!(metrics.package_bytes <= 262_144, "{metrics:#?}");
        assert!(package.transitions.iter().all(|transition| {
            transition.routing_atoms.is_empty() && !transition.routing_atom_ids.is_empty()
        }));
        for trace in query {
            let execution = package.execute_routed(&trace.before, &trace.action);
            assert_eq!(execution.status, InducedExecutionStatus::Executed);
            assert_eq!(execution.after, Some(trace.after));
        }
    }
}
