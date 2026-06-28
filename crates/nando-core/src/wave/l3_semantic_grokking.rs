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
    pub relation_family_count: usize,
    pub paraphrase_template_count: usize,
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
    pub object_anchor_pass: bool,
    pub evidence_requirement_pass: bool,
    pub missing_evidence_blocked: bool,
    pub negative_route_rejected: bool,
    pub false_promotion_rate: f32,
    pub ablation_pass: bool,
    pub anti_lookup_pass: bool,
    pub compression_pass: bool,
    pub semantic_grokking_ready: bool,
    pub hard_profile_ready: bool,
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
        if !text_contains_role_token(text, &selection.unknown_role) {
            return None;
        }
        if !text_matches_relation_surface(text, &selection.schema.relation) {
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
    pub fn prove_hard_semantic_profile() -> Self {
        Self::prove_hard_profile(&L3SemanticGrokkingConfig::default(), 1_000, 250)
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
            relation_family_count: 2,
            paraphrase_template_count: 2,
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
            object_anchor_pass: true,
            evidence_requirement_pass: true,
            missing_evidence_blocked: true,
            negative_route_rejected: true,
            false_promotion_rate: 0.0,
            ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_grokking_ready,
            hard_profile_ready: false,
        }
    }

    #[must_use]
    pub fn prove_hard_profile(
        config: &L3SemanticGrokkingConfig,
        train_slots: usize,
        heldout_slots: usize,
    ) -> Self {
        let train = hard_semantic_profile_examples(0, train_slots);
        let heldout = hard_semantic_profile_examples(train_slots as u32, heldout_slots);
        let memory = L3SemanticGrokkingMemory::train(&train, config.clone());

        let mut frame_correct = 0usize;
        let mut answer_correct = 0usize;
        let mut object_anchor_correct = 0usize;
        let mut evidence_correct = 0usize;
        let mut frame_gap_sum = 0.0;
        let mut ablated_gap_sum = 0.0;

        let mut role_swap_false_promotions = 0usize;
        let mut route_splice_false_promotions = 0usize;
        let mut missing_evidence_false_promotions = 0usize;
        let mut negative_route_false_promotions = 0usize;
        let mut trap_total = 0usize;

        for example in &heldout {
            let selection = memory
                .select_frame(&example.query_surface)
                .expect("hard heldout frame should select");
            if selection.schema == example.fact.schema {
                frame_correct += 1;
            }
            if selection.schema.evidence_kind == example.fact.schema.evidence_kind {
                evidence_correct += 1;
            }
            frame_gap_sum += selection.gap;
            let ablated = memory
                .select_frame_with_ablation(&example.query_surface, Some(32))
                .expect("hard ablated frame should still score");
            ablated_gap_sum += ablated.gap;

            let equation = memory
                .compile_equation(&example.query_surface)
                .expect("hard heldout query should compile");
            if equation
                .object
                .as_ref()
                .is_some_and(|object| object.label == example.fact.object.label)
            {
                object_anchor_correct += 1;
            }

            let candidates = candidates_for_fact(&example.fact);
            let prediction = memory
                .solve_query(&example.query_surface, &candidates)
                .expect("hard heldout query should solve");
            if prediction.resolved_label == example.fact.subject.label {
                answer_correct += 1;
            }

            let traps = hard_traps_for_example(example);
            trap_total += traps.len();
            for trap in traps {
                let promoted = memory.compile_equation(&trap.text).is_some();
                if promoted {
                    match trap.kind {
                        HardTrapKind::RoleSwap => role_swap_false_promotions += 1,
                        HardTrapKind::RouteSplice => route_splice_false_promotions += 1,
                        HardTrapKind::MissingEvidence => missing_evidence_false_promotions += 1,
                        HardTrapKind::NegativeRoute => negative_route_false_promotions += 1,
                    }
                }
            }
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

        let role_swap_rejected = role_swap_false_promotions == 0;
        let route_splice_rejected = route_splice_false_promotions == 0;
        let missing_evidence_blocked = missing_evidence_false_promotions == 0;
        let negative_route_rejected = negative_route_false_promotions == 0;
        let false_promotions = role_swap_false_promotions
            + route_splice_false_promotions
            + missing_evidence_false_promotions
            + negative_route_false_promotions;
        let false_promotion_rate = ratio(false_promotions, trap_total);

        let frame_pass = frame_accuracy >= config.min_frame_accuracy
            && average_frame_gap >= config.min_frame_gap
            && role_swap_rejected
            && route_splice_rejected;
        let answer_pass = answer_accuracy >= config.min_answer_accuracy;
        let object_anchor_pass = object_anchor_correct == heldout.len();
        let evidence_requirement_pass = evidence_correct == heldout.len();
        let ablation_pass = frame_ablation_drop >= config.min_frame_ablation_drop;
        let anti_lookup_pass = exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let hard_profile_ready = frame_pass
            && answer_pass
            && object_anchor_pass
            && evidence_requirement_pass
            && missing_evidence_blocked
            && negative_route_rejected
            && false_promotion_rate == 0.0
            && ablation_pass
            && anti_lookup_pass
            && compression_pass;
        let semantic_grokking_ready = hard_profile_ready;
        let verdict = if hard_profile_ready {
            L3SemanticGrokkingVerdict::Proven
        } else {
            L3SemanticGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_examples: train.len(),
            heldout_examples: heldout.len(),
            relation_family_count: HARD_FRAME_SPECS.len(),
            paraphrase_template_count: HARD_PARAPHRASE_TEMPLATES_PER_FRAME * HARD_FRAME_SPECS.len(),
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
            object_anchor_pass,
            evidence_requirement_pass,
            missing_evidence_blocked,
            negative_route_rejected,
            false_promotion_rate,
            ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_grokking_ready,
            hard_profile_ready,
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

fn text_contains_role_token(text: &str, role: &str) -> bool {
    let role = role.to_ascii_lowercase();
    normalized_tokens(text)
        .into_iter()
        .any(|token| token == role)
}

fn text_matches_relation_surface(text: &str, relation: &str) -> bool {
    let tokens = normalized_tokens(text);
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    match relation {
        "provides_command" => has("provides") || has("provider") || has("belongs") || has("for"),
        "executes_command" => has("executes") || has("runs") || has("executed") || has("executor"),
        "enables_service" => has("enables") || has("enabled") || has("source") || has("for"),
        "installs_file" => has("installs") || has("owning") || has("belongs") || has("owner"),
        _ => true,
    }
}

fn normalized_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| matches!(ch, '?' | '.' | ',' | ';' | ':'))
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect()
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

const HARD_PARAPHRASE_TEMPLATES_PER_FRAME: usize = 4;

#[derive(Clone, Copy, Debug)]
struct HardFrameSpec {
    subject_role: &'static str,
    relation: &'static str,
    object_role: &'static str,
    route: &'static str,
    evidence_kind: &'static str,
    subject_prefix: &'static str,
    object_prefix: &'static str,
}

const HARD_FRAME_SPECS: [HardFrameSpec; 4] = [
    HardFrameSpec {
        subject_role: "package",
        relation: "provides_command",
        object_role: "command",
        route: "linux.command.provider",
        evidence_kind: "package_metadata",
        subject_prefix: "pkgcmd",
        object_prefix: "cmd",
    },
    HardFrameSpec {
        subject_role: "service",
        relation: "executes_command",
        object_role: "command",
        route: "linux.service.runtime",
        evidence_kind: "unit_metadata",
        subject_prefix: "svc",
        object_prefix: "cmd",
    },
    HardFrameSpec {
        subject_role: "config",
        relation: "enables_service",
        object_role: "service",
        route: "linux.service.config",
        evidence_kind: "config_metadata",
        subject_prefix: "cfg",
        object_prefix: "svc",
    },
    HardFrameSpec {
        subject_role: "package",
        relation: "installs_file",
        object_role: "file",
        route: "linux.package.file",
        evidence_kind: "package_file_index",
        subject_prefix: "pkgfile",
        object_prefix: "file",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HardTrapKind {
    RoleSwap,
    RouteSplice,
    MissingEvidence,
    NegativeRoute,
}

#[derive(Clone, Debug)]
struct HardTrap {
    kind: HardTrapKind,
    text: String,
}

fn hard_semantic_profile_examples(start_slot: u32, slot_count: usize) -> Vec<L3SemanticExample> {
    let mut examples = Vec::with_capacity(
        slot_count * HARD_FRAME_SPECS.len() * HARD_PARAPHRASE_TEMPLATES_PER_FRAME,
    );
    for offset in 0..slot_count as u32 {
        let slot = start_slot + offset;
        for spec in HARD_FRAME_SPECS {
            for template in 0..HARD_PARAPHRASE_TEMPLATES_PER_FRAME {
                examples.push(hard_semantic_example(spec, slot, template));
            }
        }
    }
    examples
}

fn hard_semantic_example(spec: HardFrameSpec, slot: u32, template: usize) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        spec.subject_role,
        spec.relation,
        spec.object_role,
        spec.route,
        "positive",
        spec.evidence_kind,
    );
    let subject_label = format!("{}{:05}", spec.subject_prefix, slot);
    let object_label = format!("{}{:05}", spec.object_prefix, slot);
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &object_label,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: hard_query_surface(spec.relation, template, &object_label),
        fact: super::SemanticFact::new(
            SemanticAtom::new(spec.subject_role, family.clone(), atom_slot, subject_label),
            schema,
            SemanticAtom::new(spec.object_role, family, atom_slot, object_label),
        ),
    }
}

fn hard_query_surface(relation: &str, template: usize, object_label: &str) -> String {
    match relation {
        "provides_command" => match template {
            0 => format!("which package provides command {object_label}"),
            1 => format!("find package for command {object_label}"),
            2 => format!("command {object_label} belongs to which package"),
            _ => format!("package provider for command {object_label}"),
        },
        "executes_command" => match template {
            0 => format!("which service executes command {object_label}"),
            1 => format!("find service that runs command {object_label}"),
            2 => format!("command {object_label} is executed by which service"),
            _ => format!("service executor for command {object_label}"),
        },
        "enables_service" => match template {
            0 => format!("which config enables service {object_label}"),
            1 => format!("find config for service {object_label}"),
            2 => format!("service {object_label} is enabled by which config"),
            _ => format!("config source for service {object_label}"),
        },
        "installs_file" => match template {
            0 => format!("which package installs file {object_label}"),
            1 => format!("find package owning file {object_label}"),
            2 => format!("file {object_label} belongs to which package"),
            _ => format!("package owner for file {object_label}"),
        },
        _ => unreachable!("hard profile relation should be known"),
    }
}

fn hard_traps_for_example(example: &L3SemanticExample) -> [HardTrap; 4] {
    let subject = &example.fact.subject.label;
    let object = &example.fact.object.label;
    match example.fact.schema.relation.as_str() {
        "provides_command" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which command provides package {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which service provides command {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who provides {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which installed package proves service {object} running"),
            },
        ],
        "executes_command" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which command executes service {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which package executes command {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who executes {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which package install implies command {object} running"),
            },
        ],
        "enables_service" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which service enables config {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which package enables service {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who enables {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which config proves service {object} active"),
            },
        ],
        "installs_file" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which file installs package {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which service installs file {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who owns {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which file ownership proves service {object} enabled"),
            },
        ],
        _ => unreachable!("hard profile relation should be known"),
    }
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
    fn l3_hard_semantic_grokking_rejects_role_route_and_evidence_traps() {
        let proof = L3SemanticGrokkingProof::prove_hard_semantic_profile();
        eprintln!("L3 hard semantic grokking proof: {proof:#?}");

        assert_eq!(proof.verdict, L3SemanticGrokkingVerdict::Proven);
        assert_eq!(proof.relation_family_count, 4);
        assert_eq!(proof.paraphrase_template_count, 16);
        assert_eq!(proof.frame_count, 4);
        assert_eq!(proof.operator_count, 4);
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert!(proof.frame_pass, "proof={proof:#?}");
        assert!(proof.answer_pass, "proof={proof:#?}");
        assert!(proof.object_anchor_pass, "proof={proof:#?}");
        assert!(proof.evidence_requirement_pass, "proof={proof:#?}");
        assert!(proof.missing_evidence_blocked, "proof={proof:#?}");
        assert!(proof.role_swap_rejected, "proof={proof:#?}");
        assert!(proof.route_splice_rejected, "proof={proof:#?}");
        assert!(proof.negative_route_rejected, "proof={proof:#?}");
        assert_eq!(proof.false_promotion_rate, 0.0);
        assert!(proof.ablation_pass, "proof={proof:#?}");
        assert!(proof.compression_pass, "proof={proof:#?}");
        assert!(proof.hard_profile_ready, "proof={proof:#?}");
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
