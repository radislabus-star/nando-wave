use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::hypothesis::{
    RelationEvidenceIndex, RelationSignature, RoleHypothesis, discover_action_groups,
    enumerate_hypotheses,
};
use crate::package::{InducedTransition, InducedTransitionPackage};
use crate::synthesis::{SynthesisMetrics, synthesize_contracts};
use crate::trace::{TransitionTrace, discover_surface};
use crate::wave::{RelationWaveMemory, WaveAblation, WaveContributionMetrics, WaveTrainingExample};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InductionError {
    EmptyTrainingCorpus,
    Surface(String),
    ActionGrouping(String),
    WaveTopKMiss(String),
    GuardSynthesis(String),
    VerifierSynthesis(String),
    PackageSerialization,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InductionMetrics {
    pub action_groups: usize,
    pub hypotheses_generated: usize,
    pub guided_exact_checks: usize,
    pub unguided_exact_checks: usize,
    pub phase_top_k_hits: usize,
    pub phase_top_k_recall_milli: usize,
    pub guided_induction_cpu_ns: u64,
    pub unguided_induction_cpu_ns: u64,
    pub package_bytes: usize,
    pub wave: WaveContributionMetrics,
    pub synthesis: SynthesisMetrics,
}

#[derive(Clone, Debug)]
pub struct TransitionInducer {
    memory: RelationWaveMemory,
    top_k: usize,
    ablation: WaveAblation,
}

impl TransitionInducer {
    pub fn train(
        meta_batches: &[Vec<TransitionTrace>],
        cells: usize,
        split_threshold: f64,
        top_k: usize,
    ) -> Result<Self, InductionError> {
        if meta_batches.is_empty() {
            return Err(InductionError::EmptyTrainingCorpus);
        }
        let mut examples = Vec::new();
        for traces in meta_batches {
            let first = traces.first().ok_or(InductionError::EmptyTrainingCorpus)?;
            let shape = discover_surface(&first.before)
                .map_err(|error| InductionError::Surface(error.to_owned()))?;
            let (kind_path, groups) = discover_action_groups(traces)
                .map_err(|error| InductionError::ActionGrouping(error.to_owned()))?;
            for (concrete_kind, group) in groups {
                let evidence = RelationEvidenceIndex::new(&shape, &group)
                    .map_err(|error| InductionError::Surface(error.to_owned()))?;
                for hypothesis in enumerate_hypotheses(&shape, &kind_path, &concrete_kind, &group) {
                    let signature = evidence.signature_and_valid(&hypothesis, Some(1)).0;
                    let valid = evidence.signature_and_valid(&hypothesis, None).1;
                    let atoms = signature.atoms();
                    examples.push(WaveTrainingExample { atoms, valid });
                }
            }
        }
        let memory = RelationWaveMemory::train(&examples, cells, split_threshold);
        Ok(Self {
            memory,
            top_k: top_k.max(1),
            ablation: WaveAblation::Full,
        })
    }

    #[must_use]
    pub fn with_ablation(mut self, ablation: WaveAblation) -> Self {
        self.ablation = ablation;
        self
    }

    pub fn induce(
        &mut self,
        support: &[TransitionTrace],
    ) -> Result<(InducedTransitionPackage, InductionMetrics), InductionError> {
        self.induce_internal(support, true)
    }

    pub(crate) fn induce_without_unguided_benchmark(
        &mut self,
        support: &[TransitionTrace],
    ) -> Result<(InducedTransitionPackage, InductionMetrics), InductionError> {
        self.induce_internal(support, false)
    }

    fn induce_internal(
        &mut self,
        support: &[TransitionTrace],
        benchmark_unguided: bool,
    ) -> Result<(InducedTransitionPackage, InductionMetrics), InductionError> {
        let first = support.first().ok_or(InductionError::EmptyTrainingCorpus)?;
        let shape = discover_surface(&first.before)
            .map_err(|error| InductionError::Surface(error.to_owned()))?;
        let (kind_path, groups) = discover_action_groups(support)
            .map_err(|error| InductionError::ActionGrouping(error.to_owned()))?;
        let mut metrics = InductionMetrics {
            action_groups: groups.len(),
            wave: self.memory.metrics(),
            ..InductionMetrics::default()
        };
        let mut transitions = Vec::new();
        let mut work = Vec::new();
        for (concrete_kind, group) in &groups {
            let guided_started = Instant::now();
            let hypotheses = enumerate_hypotheses(&shape, &kind_path, concrete_kind, group);
            let evidence = RelationEvidenceIndex::new(&shape, group)
                .map_err(|error| InductionError::Surface(error.to_owned()))?;
            metrics.hypotheses_generated += hypotheses.len();
            let mut score_cache: HashMap<RelationSignature, (i64, Vec<String>)> = HashMap::new();
            let mut ranked = hypotheses
                .iter()
                .enumerate()
                .map(|(index, hypothesis)| {
                    let signature = evidence.signature_and_valid(hypothesis, Some(1)).0;
                    if self.ablation == WaveAblation::RandomRanking {
                        let margin = i64::from_ne_bytes(
                            stable_hash(hypothesis.stable_key().as_bytes()).to_ne_bytes(),
                        );
                        return (margin, index, signature);
                    }
                    let (margin, _) = score_cache.entry(signature).or_insert_with(|| {
                        let atoms = signature.atoms();
                        (
                            self.memory.score_atoms_with_ablation(&atoms, self.ablation),
                            atoms,
                        )
                    });
                    (*margin, index, signature)
                })
                .collect::<Vec<_>>();
            ranked.sort_by(|left, right| {
                right
                    .0
                    .cmp(&left.0)
                    .then_with(|| hypotheses[left.1].cmp(&hypotheses[right.1]))
            });
            let mut chosen = None;
            for (margin, index, signature) in ranked.into_iter().take(self.top_k) {
                metrics.guided_exact_checks += 1;
                let hypothesis = &hypotheses[index];
                if hypothesis.exact_on(group) {
                    metrics.phase_top_k_hits += 1;
                    let atoms = score_cache
                        .get(&signature)
                        .map(|(_, atoms)| atoms.clone())
                        .unwrap_or_else(|| signature.atoms());
                    chosen = Some((margin, hypothesis.clone(), atoms));
                    break;
                }
            }
            let Some((margin, hypothesis, atoms)) = chosen else {
                return Err(InductionError::WaveTopKMiss(concrete_kind.clone()));
            };
            metrics.guided_induction_cpu_ns = metrics
                .guided_induction_cpu_ns
                .saturating_add(nanos_u64(guided_started.elapsed().as_nanos()));
            let (program, adapter) = hypothesis.compile_actor();
            let (guard, verifier, synthesis) =
                synthesize_contracts(&hypothesis, group).map_err(|error| {
                    if error.starts_with("guard_") {
                        InductionError::GuardSynthesis(format!("{concrete_kind}:{error}"))
                    } else {
                        InductionError::VerifierSynthesis(format!("{concrete_kind}:{error}"))
                    }
                })?;
            metrics.synthesis.guard_candidates_checked += synthesis.guard_candidates_checked;
            metrics.synthesis.guard_counterexamples += synthesis.guard_counterexamples;
            metrics.synthesis.verifier_candidates_checked += synthesis.verifier_candidates_checked;
            metrics.synthesis.verifier_counterexamples += synthesis.verifier_counterexamples;
            work.push((hypothesis, atoms.clone()));
            transitions.push(InducedTransition {
                action_surface: concrete_kind.clone(),
                program,
                adapter,
                guard,
                verifier,
                routing_atoms: atoms.clone(),
                routing_atom_ids: Vec::new(),
                wave_margin_micro: margin,
                support_traces: group.len(),
            });
        }
        if benchmark_unguided {
            let unguided_started = Instant::now();
            for (concrete_kind, group) in &groups {
                let mut hypotheses = enumerate_hypotheses(&shape, &kind_path, concrete_kind, group);
                hypotheses
                    .sort_by_key(|hypothesis| stable_hash(hypothesis.stable_key().as_bytes()));
                for hypothesis in hypotheses {
                    metrics.unguided_exact_checks += 1;
                    if hypothesis.exact_on(group) {
                        break;
                    }
                }
            }
            metrics.unguided_induction_cpu_ns = nanos_u64(unguided_started.elapsed().as_nanos());
        }
        metrics.phase_top_k_recall_milli = ratio_milli(metrics.phase_top_k_hits, groups.len());

        let package_id = package_id(&work);
        for (_, atoms) in &work {
            self.memory.consolidate(atoms);
        }
        metrics.wave = self.memory.metrics();
        let routing_signature = self.memory.portable_signature();
        let mut package = InducedTransitionPackage {
            schema: "nando.induced-transition-package.v1".to_owned(),
            package_id,
            transitions,
            wave_memory_bytes: metrics.wave.wave_memory_bytes,
            routing_signature,
        };
        metrics.package_bytes = package
            .artifact_bytes()
            .map_err(|_| InductionError::PackageSerialization)?
            .len();
        package.wave_memory_bytes = metrics.wave.wave_memory_bytes;
        Ok((package, metrics))
    }

    #[must_use]
    pub fn wave_memory(&self) -> &RelationWaveMemory {
        &self.memory
    }
}

fn package_id(work: &[(RoleHypothesis, Vec<String>)]) -> String {
    let mut bytes = Vec::new();
    for (hypothesis, atoms) in work {
        bytes.extend_from_slice(hypothesis.stable_key().as_bytes());
        for atom in atoms {
            bytes.extend_from_slice(atom.as_bytes());
        }
    }
    format!("a1-{:016x}", stable_hash(&bytes))
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut state = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        state ^= u64::from(*byte);
        state = state.wrapping_mul(0x0000_0100_0000_01b3);
    }
    state
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    (numerator * 1000 + denominator / 2) / denominator
}

fn nanos_u64(value: u128) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
