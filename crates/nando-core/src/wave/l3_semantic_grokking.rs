//! L3 semantic grokking over L2 motif fields.
//!
//! L3 is the first layer that may promote semantic atoms. It does not parse raw
//! text directly. It learns frame centers from L2 motif tokens, then uses the
//! semantic relation operator to solve heldout role bindings.

use std::collections::{HashMap, HashSet};

use super::{
    L1CenterMemoryConfig, L2CenterMemory, L2CenterMemoryConfig, SemanticAtom, SemanticCandidate,
    SemanticEquationForm, SemanticSchemaKey, SemanticWaveMemory, semantic_label_slot,
};

pub const L3_FRAME_CENTER_BYTES: usize = 64;
pub const L3_FRAME_FEATURE_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L3SemanticGrokkingVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L3SemanticGrokkingConfig {
    pub l2_config: L2CenterMemoryConfig,
    pub min_frame_accuracy: f32,
    pub min_answer_accuracy: f32,
    pub min_frame_gap: f32,
    pub min_frame_ablation_drop: f32,
    pub max_model_to_naive_ratio: f32,
}

impl Default for L3SemanticGrokkingConfig {
    fn default() -> Self {
        Self {
            l2_config: L2CenterMemoryConfig {
                l1_config: L1CenterMemoryConfig {
                    min_center_support: 2,
                    min_heldout_ngram_coverage: 0.70,
                    min_average_reconstruction_similarity: 0.68,
                    min_average_fourier_similarity: 0.64,
                    min_fourier_ablation_drop: 0.03,
                    min_real_vs_corrupt_coverage_gap: 0.12,
                    max_model_to_naive_ratio: 0.20,
                    max_corrupt_eval_words: 1_024,
                    max_fourier_eval_words: 512,
                    ..L1CenterMemoryConfig::default()
                },
                motif_len: 4,
                min_motif_support: 4,
                min_heldout_ref_coverage: 0.45,
                min_heldout_word_coverage: 0.35,
                min_average_sequence_similarity: 0.45,
                min_average_fourier_similarity: 0.45,
                min_fourier_ablation_drop: 0.10,
                min_real_vs_corrupt_coverage_gap: 0.10,
                max_model_to_naive_ratio: 1.20,
                max_corrupt_eval_words: 1_024,
                max_fourier_eval_words: 512,
                ..L2CenterMemoryConfig::default()
            },
            min_frame_accuracy: 0.99,
            min_answer_accuracy: 0.99,
            min_frame_gap: 0.04,
            min_frame_ablation_drop: 0.05,
            max_model_to_naive_ratio: 0.25,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L3SemanticExample {
    pub query_surface: String,
    pub fact: super::SemanticFact,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L3FrameCenter {
    pub schema: SemanticSchemaKey,
    pub unknown_role: String,
    pub object_anchor: String,
    pub support: u32,
    pub features: Vec<(u32, i16)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L3FrameSelection {
    pub schema: SemanticSchemaKey,
    pub unknown_role: String,
    pub object_anchor: String,
    pub score: f32,
    pub runner_up_score: f32,
    pub gap: f32,
}

#[derive(Clone, Debug)]
pub struct L3SemanticGrokkingMemory {
    config: L3SemanticGrokkingConfig,
    l2: L2CenterMemory,
    frames: Vec<L3FrameCenter>,
    semantic: SemanticWaveMemory,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L3SemanticGrokkingProof {
    pub verdict: L3SemanticGrokkingVerdict,
    pub train_examples: usize,
    pub heldout_examples: usize,
    pub frame_count: usize,
    pub l2_center_count: usize,
    pub operator_count: usize,
    pub frame_accuracy: f32,
    pub answer_accuracy: f32,
    pub average_frame_gap: f32,
    pub frame_ablation_drop: f32,
    pub role_swap_rejected: bool,
    pub route_splice_rejected: bool,
    pub exact_lookup_heldout_hits: usize,
    pub model_hot_bytes: usize,
    pub naive_semantic_fact_bytes: usize,
    pub model_to_naive_ratio: f32,
    pub frame_pass: bool,
    pub answer_pass: bool,
    pub ablation_pass: bool,
    pub anti_lookup_pass: bool,
    pub compression_pass: bool,
    pub semantic_grokking_ready: bool,
}

impl L3SemanticGrokkingMemory {
    #[must_use]
    pub fn train(examples: &[L3SemanticExample], config: L3SemanticGrokkingConfig) -> Self {
        let surfaces = examples
            .iter()
            .map(|example| example.query_surface.as_str())
            .collect::<Vec<_>>();
        let l2 = L2CenterMemory::build(surfaces.iter().copied(), config.l2_config);

        let mut semantic = SemanticWaveMemory::new();
        semantic.train(examples.iter().map(|example| &example.fact));

        let mut builders: HashMap<SemanticSchemaKey, FrameBuilder> = HashMap::new();
        for example in examples {
            let motif_tokens = motif_tokens(&l2, &example.query_surface);
            let builder = builders
                .entry(example.fact.schema.clone())
                .or_insert_with(|| FrameBuilder {
                    unknown_role: example.fact.schema.subject_role.clone(),
                    object_anchor: example.fact.schema.object_role.clone(),
                    support: 0,
                    weights: HashMap::new(),
                });
            builder.support += 1;
            for token in motif_tokens {
                *builder.weights.entry(token).or_default() += 1;
            }
        }

        let frames = builders
            .into_iter()
            .map(|(schema, builder)| L3FrameCenter {
                schema,
                unknown_role: builder.unknown_role,
                object_anchor: builder.object_anchor,
                support: builder.support,
                features: compact_features(builder.weights),
            })
            .collect();

        Self {
            config,
            l2,
            frames,
            semantic,
        }
    }

    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    #[must_use]
    pub fn l2_center_count(&self) -> usize {
        self.l2.center_count()
    }

    #[must_use]
    pub fn operator_count(&self) -> usize {
        self.semantic.operator_count()
    }

    #[must_use]
    pub fn hot_bytes(&self) -> usize {
        self.l2.hot_bytes()
            + self.semantic.hot_operator_bytes()
            + self.frames.len() * L3_FRAME_CENTER_BYTES
            + self
                .frames
                .iter()
                .map(|frame| frame.features.len() * L3_FRAME_FEATURE_BYTES)
                .sum::<usize>()
    }

    #[must_use]
    pub fn select_frame(&self, text: &str) -> Option<L3FrameSelection> {
        self.select_frame_with_ablation(text, None)
    }

    #[must_use]
    pub fn compile_equation(&self, text: &str) -> Option<SemanticEquationForm> {
        let selection = self.select_frame(text)?;
        if selection.gap < self.config.min_frame_gap {
            return None;
        }
        let object_label = copy_object_label_after_anchor(text, &selection.object_anchor)?;
        let object_slot = semantic_label_slot(
            &selection.schema.route,
            &selection.schema.relation,
            &selection.schema.object_role,
            &object_label,
        );
        let object = SemanticAtom::new(
            selection.schema.object_role.clone(),
            route_family(&selection.schema.route),
            object_slot,
            object_label,
        );

        Some(SemanticEquationForm {
            subject: None,
            schema: selection.schema,
            object: Some(object),
            unknown_role: Some(selection.unknown_role),
        })
    }

    #[must_use]
    pub fn solve_query(
        &self,
        text: &str,
        candidates: &[SemanticCandidate],
    ) -> Option<super::SemanticEquationPrediction> {
        let equation = self.compile_equation(text)?;
        self.semantic.solve_equation(&equation, candidates)
    }

    fn select_frame_with_ablation(
        &self,
        text: &str,
        ablate_top: Option<usize>,
    ) -> Option<L3FrameSelection> {
        if self.frames.is_empty() {
            return None;
        }
        let query_tokens = motif_tokens(&self.l2, text)
            .into_iter()
            .collect::<HashSet<_>>();
        if query_tokens.is_empty() {
            return None;
        }

        let mut best_index = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        let mut runner_up_score = f32::NEG_INFINITY;
        for (index, frame) in self.frames.iter().enumerate() {
            let score = score_frame(frame, &query_tokens, ablate_top);
            if score > best_score {
                runner_up_score = best_score;
                best_score = score;
                best_index = index;
            } else if score > runner_up_score {
                runner_up_score = score;
            }
        }

        if runner_up_score == f32::NEG_INFINITY {
            runner_up_score = 0.0;
        }
        let frame = &self.frames[best_index];
        Some(L3FrameSelection {
            schema: frame.schema.clone(),
            unknown_role: frame.unknown_role.clone(),
            object_anchor: frame.object_anchor.clone(),
            score: best_score,
            runner_up_score,
            gap: best_score - runner_up_score,
        })
    }
}

impl L3SemanticGrokkingProof {
    #[must_use]
    pub fn prove_linux_command_provider_profile() -> Self {
        Self::prove_profile(&L3SemanticGrokkingConfig::default(), 8_000, 2_000)
    }

    #[must_use]
    pub fn prove_profile(
        config: &L3SemanticGrokkingConfig,
        train_count: usize,
        heldout_count: usize,
    ) -> Self {
        let train = semantic_profile_examples(0, train_count);
        let heldout = semantic_profile_examples(train_count as u32, heldout_count);
        let memory = L3SemanticGrokkingMemory::train(&train, config.clone());

        let mut frame_correct = 0usize;
        let mut answer_correct = 0usize;
        let mut frame_gap_sum = 0.0;
        let mut ablated_gap_sum = 0.0;
        let mut role_swap_rejected = true;
        let mut route_splice_rejected = true;

        for example in &heldout {
            let selection = memory
                .select_frame(&example.query_surface)
                .expect("heldout frame should select");
            if selection.schema == example.fact.schema {
                frame_correct += 1;
            }
            frame_gap_sum += selection.gap;
            let ablated = memory
                .select_frame_with_ablation(&example.query_surface, Some(8))
                .expect("ablated frame should still produce a score");
            ablated_gap_sum += ablated.gap;

            let candidates = candidates_for_fact(&example.fact);
            let prediction = memory
                .solve_query(&example.query_surface, &candidates)
                .expect("heldout query should solve");
            if prediction.resolved_label == example.fact.subject.label {
                answer_correct += 1;
            }

            let role_swap_text = format!(
                "which command provides package {}",
                example.fact.subject.label
            );
            role_swap_rejected &= memory.compile_equation(&role_swap_text).is_none();

            let route_splice_text = format!(
                "which service provides command {}",
                example.fact.object.label
            );
            route_splice_rejected &= memory.compile_equation(&route_splice_text).is_none();
        }

        let train_set = train
            .iter()
            .map(|example| fact_key(&example.fact))
            .collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|example| train_set.contains(&fact_key(&example.fact)))
            .count();

        let frame_accuracy = ratio(frame_correct, heldout.len());
        let answer_accuracy = ratio(answer_correct, heldout.len());
        let average_frame_gap = ratio_f32(frame_gap_sum, heldout.len());
        let average_ablated_gap = ratio_f32(ablated_gap_sum, heldout.len());
        let frame_ablation_drop = average_frame_gap - average_ablated_gap;
        let model_hot_bytes = memory.hot_bytes();
        let naive_semantic_fact_bytes = (train.len() + heldout.len()) * 8_192;
        let model_to_naive_ratio = ratio(model_hot_bytes, naive_semantic_fact_bytes);

        let frame_pass = frame_accuracy >= config.min_frame_accuracy
            && average_frame_gap >= config.min_frame_gap
            && role_swap_rejected
            && route_splice_rejected;
        let answer_pass = answer_accuracy >= config.min_answer_accuracy;
        let ablation_pass = frame_ablation_drop >= config.min_frame_ablation_drop;
        let anti_lookup_pass = exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let semantic_grokking_ready =
            frame_pass && answer_pass && ablation_pass && anti_lookup_pass && compression_pass;
        let verdict = if semantic_grokking_ready {
            L3SemanticGrokkingVerdict::Proven
        } else {
            L3SemanticGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_examples: train.len(),
            heldout_examples: heldout.len(),
            frame_count: memory.frame_count(),
            l2_center_count: memory.l2_center_count(),
            operator_count: memory.operator_count(),
            frame_accuracy,
            answer_accuracy,
            average_frame_gap,
            frame_ablation_drop,
            role_swap_rejected,
            route_splice_rejected,
            exact_lookup_heldout_hits,
            model_hot_bytes,
            naive_semantic_fact_bytes,
            model_to_naive_ratio,
            frame_pass,
            answer_pass,
            ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_grokking_ready,
        }
    }
}

#[derive(Clone, Debug)]
struct FrameBuilder {
    unknown_role: String,
    object_anchor: String,
    support: u32,
    weights: HashMap<u32, i32>,
}

fn motif_tokens(l2: &L2CenterMemory, text: &str) -> Vec<u32> {
    l2.token_sequence_for_text(text)
        .tokens
        .into_iter()
        .filter(|token| token & (1 << 31) == 0)
        .collect()
}

fn compact_features(weights: HashMap<u32, i32>) -> Vec<(u32, i16)> {
    let mut features = weights
        .into_iter()
        .map(|(token, weight)| (token, weight.min(i32::from(i16::MAX)) as i16))
        .collect::<Vec<_>>();
    features.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    features
}

fn score_frame(
    frame: &L3FrameCenter,
    query_tokens: &HashSet<u32>,
    ablate_top: Option<usize>,
) -> f32 {
    let skip = ablate_top.unwrap_or(0);
    let mut matched = 0.0;
    let mut total = 0.0;
    for (index, (token, weight)) in frame.features.iter().enumerate() {
        let weight = f32::from(*weight).max(0.0);
        if index >= skip && query_tokens.contains(token) {
            matched += weight;
        }
        total += weight;
    }
    if total == 0.0 { 0.0 } else { matched / total }
}

fn copy_object_label_after_anchor(text: &str, anchor: &str) -> Option<String> {
    let tokens = text
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| matches!(ch, '?' | '.' | ',' | ';' | ':'))
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let anchor = anchor.to_ascii_lowercase();
    tokens
        .windows(2)
        .find_map(|window| (window[0] == anchor).then(|| window[1].clone()))
}

fn candidates_for_fact(fact: &super::SemanticFact) -> [SemanticCandidate; 3] {
    let family = fact.subject.family.clone();
    [
        SemanticCandidate::new(fact.subject.clone()),
        SemanticCandidate::new(SemanticAtom::new(
            fact.schema.object_role.clone(),
            family.clone(),
            fact.subject.slot,
            fact.object.label.clone(),
        )),
        SemanticCandidate::new(SemanticAtom::new(
            fact.schema.subject_role.clone(),
            family,
            fact.subject.slot.wrapping_add(1),
            next_label(&fact.subject.label),
        )),
    ]
}

fn semantic_profile_examples(start_slot: u32, count: usize) -> Vec<L3SemanticExample> {
    let mut examples = Vec::with_capacity(count * 2);
    for offset in 0..count as u32 {
        let slot = start_slot + offset;
        examples.push(package_provider_example(slot));
        examples.push(service_executor_example(slot));
    }
    examples
}

fn fact_key(fact: &super::SemanticFact) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        fact.subject.role,
        fact.subject.label,
        fact.schema.relation,
        fact.schema.route,
        fact.schema.polarity,
        fact.schema.evidence_kind,
        fact.object.role,
        fact.object.label
    )
}

fn package_provider_example(slot: u32) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        "package",
        "provides_command",
        "command",
        "linux.command.provider",
        "positive",
        "package_metadata",
    );
    let package = format!("pkg{slot:05}");
    let command = format!("cmd{slot:05}");
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &command,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: format!("which package provides command {command}"),
        fact: super::SemanticFact::new(
            SemanticAtom::new("package", family.clone(), atom_slot, package),
            schema,
            SemanticAtom::new("command", family, atom_slot, command),
        ),
    }
}

fn service_executor_example(slot: u32) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        "service",
        "executes_command",
        "command",
        "linux.service.runtime",
        "positive",
        "unit_metadata",
    );
    let service = format!("svc{slot:05}");
    let command = format!("cmd{slot:05}");
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &command,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: format!("which service executes command {command}"),
        fact: super::SemanticFact::new(
            SemanticAtom::new("service", family.clone(), atom_slot, service),
            schema,
            SemanticAtom::new("command", family, atom_slot, command),
        ),
    }
}

fn route_family(route: &str) -> String {
    route.replace('.', "-")
}

fn next_label(label: &str) -> String {
    format!("{label}_near_miss")
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn ratio_f32(numerator: f32, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l3_semantic_grokking_learns_frame_from_l2_and_solves_heldout() {
        let proof = L3SemanticGrokkingProof::prove_linux_command_provider_profile();
        eprintln!("L3 semantic grokking proof: {proof:#?}");

        assert_eq!(proof.verdict, L3SemanticGrokkingVerdict::Proven);
        assert_eq!(proof.train_examples, 16_000);
        assert_eq!(proof.heldout_examples, 4_000);
        assert_eq!(proof.frame_count, 2);
        assert_eq!(proof.operator_count, 2);
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert!(proof.frame_pass, "proof={proof:#?}");
        assert!(proof.answer_pass, "proof={proof:#?}");
        assert!(proof.ablation_pass, "proof={proof:#?}");
        assert!(proof.role_swap_rejected, "proof={proof:#?}");
        assert!(proof.route_splice_rejected, "proof={proof:#?}");
        assert!(proof.compression_pass, "proof={proof:#?}");
        assert!(proof.semantic_grokking_ready, "proof={proof:#?}");
    }

    #[test]
    fn l3_unknown_surface_has_no_semantic_authority() {
        let train = semantic_profile_examples(0, 256);
        let memory = L3SemanticGrokkingMemory::train(&train, L3SemanticGrokkingConfig::default());

        assert!(memory.compile_equation("bash maybe thing").is_none());
        assert!(
            memory
                .compile_equation("which service provides command cmd00999")
                .is_none()
        );
    }
}
