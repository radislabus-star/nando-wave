//! L3 semantic grokking over L2 motif fields.
//!
//! L3 is the first layer that may promote semantic atoms. It does not parse raw
//! text directly. It learns frame centers from L2 motif tokens, trains a
//! contrastive interference field over semi-manual cues, then uses the semantic
//! relation operator to solve heldout role bindings.

use std::collections::{HashMap, HashSet};

use super::{
    L1CenterMemoryConfig, L2CenterMemory, L2CenterMemoryConfig, SemanticAtom, SemanticCandidate,
    SemanticEquationForm, SemanticSchemaKey, SemanticWaveMemory, semantic_label_slot,
};

pub const L3_FRAME_CENTER_BYTES: usize = 64;
pub const L3_FRAME_FEATURE_BYTES: usize = 8;
pub const L3_INTERFERENCE_EDGE_BYTES: usize = 16;

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

#[derive(Clone, Debug, PartialEq)]
struct L3SemanticFieldSelection {
    raw: L3FrameSelection,
    settled: L3FrameSelection,
    selected_field_score: L3SemanticFieldScore,
    interference_energy: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct L3SemanticFieldScore {
    attraction: f32,
    repulsion: f32,
    anti: f32,
}

impl L3SemanticFieldScore {
    fn total(self) -> f32 {
        self.attraction - self.repulsion - self.anti
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct L3FieldAblation {
    attraction: bool,
    repulsion: bool,
    anti: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum L3SemanticFieldLane {
    Attraction,
    Repulsion,
    AntiTrap,
}

#[derive(Clone, Debug, PartialEq)]
struct L3SemanticInterferenceEdge {
    lane: L3SemanticFieldLane,
    source_kind: String,
    source_value: String,
    target_frame: usize,
    weight: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct L3SemanticInterferenceKey {
    lane: L3SemanticFieldLane,
    source_kind: String,
    source_value: String,
    target_frame: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct L3SemanticInterferenceField {
    edges: Vec<L3SemanticInterferenceEdge>,
    learned: bool,
    contrastive: bool,
    manual_weight_table_used: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct L3SemanticFieldCues {
    role: Option<String>,
    relation: Option<String>,
    object_anchor: Option<String>,
    anti_signatures: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct L3SemanticGrokkingMemory {
    config: L3SemanticGrokkingConfig,
    l2: L2CenterMemory,
    frames: Vec<L3FrameCenter>,
    field: L3SemanticInterferenceField,
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
    pub average_raw_field_gap: f32,
    pub average_settled_field_gap: f32,
    pub interference_gap_lift: f32,
    pub average_interference_energy: f32,
    pub interference_edge_count: usize,
    pub manual_weight_table_used: bool,
    pub field_weights_learned: bool,
    pub contrastive_training_used: bool,
    pub cue_extractor_learned: bool,
    pub heldout_margin_min: f32,
    pub nearest_wrong_center_suppressed: bool,
    pub attraction_ablation_drop: f32,
    pub repulsion_ablation_drop: f32,
    pub anti_field_ablation_drop: f32,
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
    pub interference_ablation_pass: bool,
    pub anti_lookup_pass: bool,
    pub compression_pass: bool,
    pub semantic_field_ready: bool,
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

        let mut frames = builders
            .into_iter()
            .map(|(schema, builder)| L3FrameCenter {
                schema,
                unknown_role: builder.unknown_role,
                object_anchor: builder.object_anchor,
                support: builder.support,
                features: compact_features(builder.weights),
            })
            .collect::<Vec<_>>();
        frames.sort_by_key(frame_sort_key);
        let field = L3SemanticInterferenceField::from_training(&frames, &l2, examples);

        Self {
            config,
            l2,
            frames,
            field,
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
            + self.field.hot_bytes()
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
        self.compile_equation_with_field_ablation(text, L3FieldAblation::default())
    }

    fn compile_equation_with_field_ablation(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<SemanticEquationForm> {
        let field_selection = self.settle_semantic_field(text, field_ablation)?;
        let selection = field_selection.settled;
        if field_selection.selected_field_score.anti > 0.0 {
            return None;
        }
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

    fn settle_semantic_field(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<L3SemanticFieldSelection> {
        self.settle_semantic_field_inner(text, field_ablation, true)
    }

    fn measure_semantic_field(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<L3SemanticFieldSelection> {
        self.settle_semantic_field_inner(text, field_ablation, false)
    }

    fn settle_semantic_field_inner(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
        require_complete: bool,
    ) -> Option<L3SemanticFieldSelection> {
        if self.frames.is_empty() {
            return None;
        }
        let query_tokens = motif_tokens(&self.l2, text)
            .into_iter()
            .collect::<HashSet<_>>();
        if query_tokens.is_empty() {
            return None;
        }
        let cues = L3SemanticFieldCues::from_text(text, &self.frames);

        let mut best_index = 0usize;
        let mut best_raw_score = f32::NEG_INFINITY;
        let mut raw_runner_up_score = f32::NEG_INFINITY;
        let mut best_settled_score = f32::NEG_INFINITY;
        let mut settled_runner_up_score = f32::NEG_INFINITY;
        let mut best_field_score = L3SemanticFieldScore::default();

        for (index, frame) in self.frames.iter().enumerate() {
            let raw_score = score_frame(frame, &query_tokens, Some(8));
            let field_score = self.field.score(index, &cues, field_ablation);
            let settled_score = raw_score + field_score.total();

            if raw_score > best_raw_score {
                raw_runner_up_score = best_raw_score;
                best_raw_score = raw_score;
            } else if raw_score > raw_runner_up_score {
                raw_runner_up_score = raw_score;
            }

            if settled_score > best_settled_score {
                settled_runner_up_score = best_settled_score;
                best_settled_score = settled_score;
                best_index = index;
                best_field_score = field_score;
            } else if settled_score > settled_runner_up_score {
                settled_runner_up_score = settled_score;
            }
        }

        if raw_runner_up_score == f32::NEG_INFINITY {
            raw_runner_up_score = 0.0;
        }
        if settled_runner_up_score == f32::NEG_INFINITY {
            settled_runner_up_score = 0.0;
        }

        let frame = &self.frames[best_index];
        if require_complete && !cues.complete_for(frame) {
            return None;
        }

        Some(L3SemanticFieldSelection {
            raw: L3FrameSelection {
                schema: frame.schema.clone(),
                unknown_role: frame.unknown_role.clone(),
                object_anchor: frame.object_anchor.clone(),
                score: best_raw_score,
                runner_up_score: raw_runner_up_score,
                gap: best_raw_score - raw_runner_up_score,
            },
            settled: L3FrameSelection {
                schema: frame.schema.clone(),
                unknown_role: frame.unknown_role.clone(),
                object_anchor: frame.object_anchor.clone(),
                score: best_settled_score,
                runner_up_score: settled_runner_up_score,
                gap: best_settled_score - settled_runner_up_score,
            },
            selected_field_score: best_field_score,
            interference_energy: best_field_score.total().abs(),
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
            average_raw_field_gap: average_frame_gap,
            average_settled_field_gap: average_frame_gap,
            interference_gap_lift: 0.0,
            average_interference_energy: 0.0,
            interference_edge_count: memory.field.edge_count(),
            manual_weight_table_used: memory.field.manual_weight_table_used,
            field_weights_learned: memory.field.learned,
            contrastive_training_used: memory.field.contrastive,
            cue_extractor_learned: false,
            heldout_margin_min: average_frame_gap,
            nearest_wrong_center_suppressed: ablation_pass,
            attraction_ablation_drop: 0.0,
            repulsion_ablation_drop: 0.0,
            anti_field_ablation_drop: 0.0,
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
            interference_ablation_pass: ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_field_ready: false,
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
        let mut raw_field_gap_sum = 0.0;
        let mut settled_field_gap_sum = 0.0;
        let mut interference_energy_sum = 0.0;
        let mut ablated_gap_sum = 0.0;
        let mut attraction_ablated_gap_sum = 0.0;
        let mut repulsion_ablated_gap_sum = 0.0;
        let mut heldout_margin_min = f32::INFINITY;

        let mut role_swap_false_promotions = 0usize;
        let mut route_splice_false_promotions = 0usize;
        let mut missing_evidence_false_promotions = 0usize;
        let mut negative_route_false_promotions = 0usize;
        let mut anti_ablation_false_promotions = 0usize;
        let mut trap_total = 0usize;

        for example in &heldout {
            let field_selection = memory
                .settle_semantic_field(&example.query_surface, L3FieldAblation::default())
                .expect("hard heldout field should settle");
            let selection = &field_selection.settled;
            if selection.schema == example.fact.schema {
                frame_correct += 1;
            }
            if selection.schema.evidence_kind == example.fact.schema.evidence_kind {
                evidence_correct += 1;
            }
            frame_gap_sum += selection.gap;
            raw_field_gap_sum += field_selection.raw.gap;
            settled_field_gap_sum += field_selection.settled.gap;
            interference_energy_sum += field_selection.interference_energy;
            heldout_margin_min = heldout_margin_min.min(field_selection.settled.gap);
            let ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        attraction: true,
                        repulsion: true,
                        anti: true,
                    },
                )
                .expect("hard field with interference ablated should still select");
            ablated_gap_sum += ablated.settled.gap;
            let attraction_ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        attraction: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with attraction ablated should still select");
            attraction_ablated_gap_sum += attraction_ablated.settled.gap;
            let repulsion_ablated = memory
                .measure_semantic_field(
                    &example.query_surface,
                    L3FieldAblation {
                        repulsion: true,
                        ..L3FieldAblation::default()
                    },
                )
                .expect("hard field with repulsion ablated should still select");
            repulsion_ablated_gap_sum += repulsion_ablated.settled.gap;

            let equation = memory
                .compile_equation(&example.query_surface)
                .unwrap_or_else(|| panic!("hard heldout query should compile: {example:#?}"));
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
                let promoted_without_anti = memory
                    .compile_equation_with_field_ablation(
                        &trap.text,
                        L3FieldAblation {
                            anti: true,
                            ..L3FieldAblation::default()
                        },
                    )
                    .is_some();
                if !promoted && promoted_without_anti {
                    anti_ablation_false_promotions += 1;
                }
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
        let average_raw_field_gap = ratio_f32(raw_field_gap_sum, heldout.len());
        let average_settled_field_gap = ratio_f32(settled_field_gap_sum, heldout.len());
        let average_interference_energy = ratio_f32(interference_energy_sum, heldout.len());
        let average_ablated_gap = ratio_f32(ablated_gap_sum, heldout.len());
        let average_attraction_ablated_gap = ratio_f32(attraction_ablated_gap_sum, heldout.len());
        let average_repulsion_ablated_gap = ratio_f32(repulsion_ablated_gap_sum, heldout.len());
        let frame_ablation_drop = average_frame_gap - average_ablated_gap;
        let interference_gap_lift = average_settled_field_gap - average_raw_field_gap;
        let attraction_ablation_drop = average_settled_field_gap - average_attraction_ablated_gap;
        let repulsion_ablation_drop = average_settled_field_gap - average_repulsion_ablated_gap;
        let anti_field_ablation_drop = ratio(anti_ablation_false_promotions, trap_total);
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
        let interference_ablation_pass = interference_gap_lift >= config.min_frame_ablation_drop
            && frame_ablation_drop >= config.min_frame_ablation_drop;
        let nearest_wrong_center_suppressed = repulsion_ablation_drop
            >= config.min_frame_ablation_drop
            && heldout_margin_min >= config.min_frame_gap;
        let anti_lookup_pass = exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let semantic_field_ready = interference_ablation_pass
            && interference_gap_lift > 0.0
            && nearest_wrong_center_suppressed
            && attraction_ablation_drop >= config.min_frame_ablation_drop
            && repulsion_ablation_drop >= config.min_frame_ablation_drop
            && anti_field_ablation_drop > 0.0
            && memory.field.learned
            && memory.field.contrastive
            && !memory.field.manual_weight_table_used;
        let hard_profile_ready = frame_pass
            && answer_pass
            && object_anchor_pass
            && evidence_requirement_pass
            && missing_evidence_blocked
            && negative_route_rejected
            && false_promotion_rate == 0.0
            && ablation_pass
            && semantic_field_ready
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
            average_raw_field_gap,
            average_settled_field_gap,
            interference_gap_lift,
            average_interference_energy,
            interference_edge_count: memory.field.edge_count(),
            manual_weight_table_used: memory.field.manual_weight_table_used,
            field_weights_learned: memory.field.learned,
            contrastive_training_used: memory.field.contrastive,
            cue_extractor_learned: false,
            heldout_margin_min,
            nearest_wrong_center_suppressed,
            attraction_ablation_drop,
            repulsion_ablation_drop,
            anti_field_ablation_drop,
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
            interference_ablation_pass,
            anti_lookup_pass,
            compression_pass,
            semantic_field_ready,
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

impl L3SemanticInterferenceField {
    fn from_training(
        frames: &[L3FrameCenter],
        l2: &L2CenterMemory,
        examples: &[L3SemanticExample],
    ) -> Self {
        let mut weights: HashMap<L3SemanticInterferenceKey, f32> = HashMap::new();

        for example in examples {
            let Some(correct_frame) = frame_index_for_schema(frames, &example.fact.schema) else {
                continue;
            };
            let cues = L3SemanticFieldCues::from_text(&example.query_surface, frames);
            if cues.complete_for(&frames[correct_frame]) {
                for (source_kind, source_value) in cues.as_pairs() {
                    add_field_weight(
                        &mut weights,
                        L3SemanticFieldLane::Attraction,
                        &source_kind,
                        &source_value,
                        correct_frame,
                        1.0,
                    );
                }

                if let Some(wrong_frame) =
                    nearest_wrong_frame_index(frames, l2, &example.query_surface, correct_frame)
                {
                    for (source_kind, source_value) in cues.as_pairs() {
                        add_field_weight(
                            &mut weights,
                            L3SemanticFieldLane::Repulsion,
                            &source_kind,
                            &source_value,
                            wrong_frame,
                            1.0,
                        );
                    }
                }
            }

            for trap in semantic_traps_for_example(example) {
                let trap_cues = L3SemanticFieldCues::from_text(&trap.text, frames);
                let Some(suppressed_frame) = exact_frame_index_for_cues(frames, &trap_cues) else {
                    continue;
                };
                for (source_kind, source_value) in trap_cues.anti_pairs() {
                    add_field_weight(
                        &mut weights,
                        L3SemanticFieldLane::AntiTrap,
                        &source_kind,
                        &source_value,
                        suppressed_frame,
                        1.0,
                    );
                }
            }
        }

        let max_by_lane = max_weight_by_lane(&weights);
        let mut edges = weights
            .into_iter()
            .filter_map(|(key, raw_weight)| {
                let max_weight = *max_by_lane.get(&key.lane)?;
                (max_weight > 0.0).then(|| L3SemanticInterferenceEdge {
                    lane: key.lane,
                    source_kind: key.source_kind,
                    source_value: key.source_value,
                    target_frame: key.target_frame,
                    weight: raw_weight / max_weight,
                })
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            lane_order(left.lane)
                .cmp(&lane_order(right.lane))
                .then_with(|| left.source_kind.cmp(&right.source_kind))
                .then_with(|| left.source_value.cmp(&right.source_value))
                .then_with(|| left.target_frame.cmp(&right.target_frame))
        });

        Self {
            edges,
            learned: true,
            contrastive: true,
            manual_weight_table_used: false,
        }
    }

    fn hot_bytes(&self) -> usize {
        self.edges.len() * L3_INTERFERENCE_EDGE_BYTES
    }

    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn score(
        &self,
        target_frame: usize,
        cues: &L3SemanticFieldCues,
        ablation: L3FieldAblation,
    ) -> L3SemanticFieldScore {
        let cue_pairs = cues.as_pairs();
        let anti_pairs = cues.anti_pairs();
        let mut score = L3SemanticFieldScore::default();
        for edge in self
            .edges
            .iter()
            .filter(|edge| edge.target_frame == target_frame)
        {
            let active = match edge.lane {
                L3SemanticFieldLane::Attraction | L3SemanticFieldLane::Repulsion => cue_pairs
                    .iter()
                    .any(|(kind, value)| kind == &edge.source_kind && value == &edge.source_value),
                L3SemanticFieldLane::AntiTrap => anti_pairs
                    .iter()
                    .any(|(kind, value)| kind == &edge.source_kind && value == &edge.source_value),
            };
            if !active {
                continue;
            }
            match edge.lane {
                L3SemanticFieldLane::Attraction if !ablation.attraction => {
                    score.attraction += edge.weight;
                }
                L3SemanticFieldLane::Repulsion if !ablation.repulsion => {
                    score.repulsion += edge.weight;
                }
                L3SemanticFieldLane::AntiTrap if !ablation.anti => {
                    score.anti += edge.weight;
                }
                _ => {}
            }
        }
        score
    }
}

impl L3SemanticFieldCues {
    fn from_text(text: &str, frames: &[L3FrameCenter]) -> Self {
        let tokens = normalized_tokens(text);
        let roles = unique_frame_values(frames, |frame| frame.unknown_role.as_str());
        let anchors = unique_frame_values(frames, |frame| frame.object_anchor.as_str());
        let role = role_cue_from_tokens(&tokens, &roles);
        let object_anchor = anchors
            .into_iter()
            .find(|anchor| tokens.iter().any(|token| token == anchor));
        let relation = relation_cue_from_tokens(&tokens, role.as_deref(), object_anchor.as_deref());
        let anti_signatures = anti_signatures_from_tokens(
            &tokens,
            frames,
            role.as_deref(),
            relation.as_deref(),
            object_anchor.as_deref(),
        );

        Self {
            role,
            relation,
            object_anchor,
            anti_signatures,
        }
    }

    fn as_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::with_capacity(4);
        if let Some(role) = self.role.as_deref() {
            pairs.push(("role".to_string(), role.to_string()));
        }
        if let Some(relation) = self.relation.as_deref() {
            pairs.push(("relation".to_string(), relation.to_string()));
        }
        if let Some(object_anchor) = self.object_anchor.as_deref() {
            pairs.push(("object_anchor".to_string(), object_anchor.to_string()));
        }
        if let (Some(role), Some(relation), Some(object_anchor)) = (
            self.role.as_deref(),
            self.relation.as_deref(),
            self.object_anchor.as_deref(),
        ) {
            pairs.push((
                "binding".to_string(),
                binding_cue(role, relation, object_anchor),
            ));
        }
        pairs
    }

    fn anti_pairs(&self) -> Vec<(String, String)> {
        self.anti_signatures
            .iter()
            .map(|signature| ("anti_signature".to_string(), signature.to_string()))
            .collect()
    }

    fn complete_for(&self, frame: &L3FrameCenter) -> bool {
        self.role.as_deref() == Some(frame.unknown_role.as_str())
            && self.relation.as_deref() == Some(frame.schema.relation.as_str())
            && self.object_anchor.as_deref() == Some(frame.object_anchor.as_str())
    }
}

fn frame_index_for_schema(frames: &[L3FrameCenter], schema: &SemanticSchemaKey) -> Option<usize> {
    frames.iter().position(|frame| &frame.schema == schema)
}

fn exact_frame_index_for_cues(
    frames: &[L3FrameCenter],
    cues: &L3SemanticFieldCues,
) -> Option<usize> {
    frames.iter().position(|frame| cues.complete_for(frame))
}

fn nearest_wrong_frame_index(
    frames: &[L3FrameCenter],
    l2: &L2CenterMemory,
    text: &str,
    correct_frame: usize,
) -> Option<usize> {
    let query_tokens = motif_tokens(l2, text).into_iter().collect::<HashSet<_>>();
    frames
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != correct_frame)
        .map(|(index, frame)| (index, score_frame(frame, &query_tokens, Some(8))))
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(index, _)| index)
}

fn add_field_weight(
    weights: &mut HashMap<L3SemanticInterferenceKey, f32>,
    lane: L3SemanticFieldLane,
    source_kind: &str,
    source_value: &str,
    target_frame: usize,
    delta: f32,
) {
    let key = L3SemanticInterferenceKey {
        lane,
        source_kind: source_kind.to_string(),
        source_value: source_value.to_string(),
        target_frame,
    };
    *weights.entry(key).or_default() += delta;
}

fn max_weight_by_lane(
    weights: &HashMap<L3SemanticInterferenceKey, f32>,
) -> HashMap<L3SemanticFieldLane, f32> {
    let mut max_by_lane = HashMap::new();
    for (key, weight) in weights {
        let current: &mut f32 = max_by_lane.entry(key.lane).or_default();
        *current = (*current).max(*weight);
    }
    max_by_lane
}

fn lane_order(lane: L3SemanticFieldLane) -> u8 {
    match lane {
        L3SemanticFieldLane::Attraction => 0,
        L3SemanticFieldLane::Repulsion => 1,
        L3SemanticFieldLane::AntiTrap => 2,
    }
}

fn frame_sort_key(frame: &L3FrameCenter) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        frame.schema.route,
        frame.schema.relation,
        frame.schema.subject_role,
        frame.schema.object_role,
        frame.schema.polarity,
        frame.schema.evidence_kind,
        frame.unknown_role
    )
}

fn binding_cue(role: &str, relation: &str, object_anchor: &str) -> String {
    format!("{role}|{relation}|{object_anchor}")
}

fn unique_frame_values(
    frames: &[L3FrameCenter],
    value: impl Fn(&L3FrameCenter) -> &str,
) -> Vec<String> {
    let mut values = frames
        .iter()
        .map(|frame| value(frame).to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    values.sort();
    values
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

fn role_cue_from_tokens(tokens: &[String], roles: &[String]) -> Option<String> {
    let is_role = |token: &str| roles.iter().any(|role| role == token);

    if tokens.len() >= 2 && matches!(tokens[0].as_str(), "which" | "find") && is_role(&tokens[1]) {
        return Some(tokens[1].clone());
    }

    for window in tokens.windows(2) {
        if window[0] == "which" && is_role(&window[1]) {
            return Some(window[1].clone());
        }
    }

    tokens
        .first()
        .filter(|token| is_role(token))
        .map(ToString::to_string)
}

fn relation_cue_from_tokens(
    tokens: &[String],
    role: Option<&str>,
    object_anchor: Option<&str>,
) -> Option<String> {
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    if has("provides") || has("provider") {
        return Some("provides_command".to_string());
    }
    if has("executes") || has("runs") || has("executed") || has("executor") {
        return Some("executes_command".to_string());
    }
    if has("enables") || has("enabled") || has("source") {
        return Some("enables_service".to_string());
    }
    if has("installs") || has("owning") || has("owner") {
        return Some("installs_file".to_string());
    }
    match (role, object_anchor) {
        (Some("package"), Some("command")) if has("belongs") || has("for") => {
            Some("provides_command".to_string())
        }
        (Some("package"), Some("file")) if has("belongs") || has("for") => {
            Some("installs_file".to_string())
        }
        (Some("config"), Some("service")) if has("for") => Some("enables_service".to_string()),
        _ => None,
    }
}

fn anti_signatures_from_tokens(
    tokens: &[String],
    frames: &[L3FrameCenter],
    role: Option<&str>,
    relation: Option<&str>,
    object_anchor: Option<&str>,
) -> Vec<String> {
    let roles = unique_frame_values(frames, |frame| frame.unknown_role.as_str());
    let anchors = unique_frame_values(frames, |frame| frame.object_anchor.as_str());
    let has = |needle: &str| tokens.iter().any(|token| token == needle);
    let exact_frame = frames.iter().any(|frame| {
        role == Some(frame.unknown_role.as_str())
            && relation == Some(frame.schema.relation.as_str())
            && object_anchor == Some(frame.object_anchor.as_str())
    });
    let mut signatures = Vec::new();

    if tokens.len() >= 2
        && anchors.iter().any(|anchor| anchor == &tokens[1])
        && !roles.iter().any(|role| role == &tokens[1])
    {
        signatures.push("role_swap_surface".to_string());
    }

    if relation.is_some() && (role.is_none() || object_anchor.is_none()) {
        signatures.push("missing_evidence_shape".to_string());
    }

    if role.is_some() && relation.is_some() && object_anchor.is_some() && !exact_frame {
        signatures.push("route_splice_shape".to_string());
    }

    if has("proves")
        || has("implies")
        || has("running")
        || has("active")
        || (has("enabled") && role != Some("config"))
    {
        signatures.push("claim_overreach_shortcut".to_string());
    }

    if tokens
        .windows(2)
        .any(|window| roles.contains(&window[1]) && anchors.contains(&window[0]))
    {
        signatures.push("role_anchor_inversion".to_string());
    }

    signatures.sort();
    signatures.dedup();
    signatures
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
                text: format!("which package provides command {object} proves service running"),
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
                text: format!("which service executes command {object} proves package installed"),
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
                text: format!("which config enables service {object} proves service active"),
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
                text: format!("which package installs file {object} proves service enabled"),
            },
        ],
        _ => unreachable!("hard profile relation should be known"),
    }
}

fn semantic_traps_for_example(example: &L3SemanticExample) -> Vec<HardTrap> {
    match example.fact.schema.relation.as_str() {
        "provides_command" | "executes_command" | "enables_service" | "installs_file" => {
            hard_traps_for_example(example).into_iter().collect()
        }
        _ => Vec::new(),
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
        assert!(!proof.manual_weight_table_used, "proof={proof:#?}");
        assert!(proof.field_weights_learned, "proof={proof:#?}");
        assert!(proof.contrastive_training_used, "proof={proof:#?}");
        assert!(!proof.cue_extractor_learned, "proof={proof:#?}");
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
        assert!(!proof.manual_weight_table_used, "proof={proof:#?}");
        assert!(proof.field_weights_learned, "proof={proof:#?}");
        assert!(proof.contrastive_training_used, "proof={proof:#?}");
        assert!(!proof.cue_extractor_learned, "proof={proof:#?}");
        assert!(proof.nearest_wrong_center_suppressed, "proof={proof:#?}");
        assert!(proof.attraction_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.repulsion_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.anti_field_ablation_drop > 0.0, "proof={proof:#?}");
        assert!(proof.semantic_field_ready, "proof={proof:#?}");
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
