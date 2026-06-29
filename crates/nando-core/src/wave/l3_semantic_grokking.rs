//! L3 semantic grokking over L2 motif fields.
//!
//! L3 is the first layer that may promote semantic atoms. It does not parse raw
//! text directly. It learns frame centers from L2 motif tokens, trains a
//! CueField over L2 motifs plus generic surface residual cues, trains a
//! contrastive interference field over those learned cues, then uses learned
//! answer binding to solve heldout role bindings.

mod answer_binding;
mod cue_field;
mod fixtures;
mod interference;
mod proof;
mod tokens;

use std::collections::{HashMap, HashSet};

use self::answer_binding::L3AnswerBindingMemory;
use self::cue_field::L3LearnedCueField;
use self::interference::L3SemanticInterferenceField;
pub use self::proof::L3SemanticGrokkingProof;
use self::tokens::{motif_tokens, normalized_tokens};

use super::{
    L1CenterMemoryConfig, L2CenterMemory, L2CenterMemoryConfig, SemanticAtom, SemanticCandidate,
    SemanticEquationForm, SemanticSchemaKey, SemanticWaveMemory, semantic_label_slot,
};

pub const L3_FRAME_CENTER_BYTES: usize = 64;
pub const L3_FRAME_FEATURE_BYTES: usize = 8;
pub const L3_CUE_EDGE_BYTES: usize = 12;
pub const L3_INTERFERENCE_EDGE_BYTES: usize = 16;
pub const L3_ANTI_CUE_THRESHOLD: f32 = 0.25;
pub const L3_ANTI_AUTHORITY_THRESHOLD: f32 = 0.5;

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
pub(super) struct L3SemanticFieldSelection {
    pub(super) raw: L3FrameSelection,
    pub(super) settled: L3FrameSelection,
    pub(super) selected_field_score: L3SemanticFieldScore,
    pub(super) cue_margin: f32,
    pub(super) interference_energy: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct L3SemanticFieldScore {
    pub(super) attraction: f32,
    pub(super) repulsion: f32,
    pub(super) anti: f32,
}

impl L3SemanticFieldScore {
    pub(super) fn total(self) -> f32 {
        self.attraction - self.repulsion - self.anti
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct L3FieldAblation {
    pub(super) cues: bool,
    pub(super) attraction: bool,
    pub(super) repulsion: bool,
    pub(super) anti: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum L3CueTokenMode {
    All,
    WithoutSurfaceResidual,
    WithoutMotifPairs,
    SurfaceResidualOnly,
}

#[derive(Clone, Debug)]
pub struct L3SemanticGrokkingMemory {
    config: L3SemanticGrokkingConfig,
    l2: L2CenterMemory,
    frames: Vec<L3FrameCenter>,
    cue_field: L3LearnedCueField,
    field: L3SemanticInterferenceField,
    answer_binding: L3AnswerBindingMemory,
    semantic: SemanticWaveMemory,
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
        let answer_binding = L3AnswerBindingMemory::from_training(examples);

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
        let cue_field = L3LearnedCueField::from_training(&frames, &l2, examples);
        let field = L3SemanticInterferenceField::from_training(&frames, &l2, examples, &cue_field);

        Self {
            config,
            l2,
            frames,
            cue_field,
            field,
            answer_binding,
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
    pub fn answer_binding_operator_count(&self) -> usize {
        self.answer_binding.operator_count()
    }

    #[must_use]
    pub fn answer_binding_learned(&self) -> bool {
        self.answer_binding.learned
    }

    #[must_use]
    pub fn answer_lookup_only(&self) -> bool {
        self.answer_binding.lookup_only
    }

    #[must_use]
    pub fn hot_bytes(&self) -> usize {
        self.l2.hot_bytes()
            + self.semantic.hot_operator_bytes()
            + self.frames.len() * L3_FRAME_CENTER_BYTES
            + self.cue_field.hot_bytes()
            + self.field.hot_bytes()
            + self.answer_binding.hot_bytes()
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

    pub(super) fn compile_equation_with_field_ablation(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<SemanticEquationForm> {
        let field_selection = self.settle_semantic_field(text, field_ablation)?;
        let selection = field_selection.settled;
        if field_selection.selected_field_score.anti >= L3_ANTI_AUTHORITY_THRESHOLD
            && !self.shape_only_anti_is_resolved_by_complete_cues(text, &selection)
        {
            return None;
        }
        if selection.gap < self.config.min_frame_gap {
            return None;
        }
        if !self.structural_cue_supports_selection(text, &selection)
            && !self
                .answer_binding
                .supports_unknown_subject(&selection.schema, &selection.unknown_role)
        {
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

    fn structural_cue_supports_selection(&self, text: &str, selection: &L3FrameSelection) -> bool {
        self.measure_semantic_field_with_cue_mode(text, L3CueTokenMode::WithoutSurfaceResidual)
            .is_some_and(|structural| {
                structural.settled.schema == selection.schema
                    && structural.settled.gap >= self.config.min_frame_gap * 0.5
            })
    }

    fn shape_only_anti_is_resolved_by_complete_cues(
        &self,
        text: &str,
        selection: &L3FrameSelection,
    ) -> bool {
        let Some(frame_index) = frame_index_for_schema(&self.frames, &selection.schema) else {
            return false;
        };
        let cue_inference = self.cue_field.infer(text, &self.l2, &self.frames, false);
        if !cue_inference.cues.complete_for(&self.frames[frame_index]) {
            return false;
        }
        if cue_inference
            .cues
            .anti_signatures
            .iter()
            .any(|signature| signature == "role_swap_surface")
            && starts_as_unknown_object_question(text, &selection.object_anchor)
        {
            return false;
        }
        !cue_inference.cues.anti_signatures.is_empty()
            && cue_inference.cues.anti_signatures.iter().all(|signature| {
                matches!(
                    signature.as_str(),
                    "missing_evidence_shape" | "role_swap_surface"
                )
            })
    }

    #[must_use]
    pub fn solve_query(
        &self,
        text: &str,
        candidates: &[SemanticCandidate],
    ) -> Option<super::SemanticEquationPrediction> {
        let equation = self.compile_equation(text)?;
        self.answer_binding
            .solve(&equation, candidates)
            .or_else(|| self.semantic.solve_equation(&equation, candidates))
    }

    pub(super) fn solve_query_with_role_binding_ablation(
        &self,
        text: &str,
        candidates: &[SemanticCandidate],
    ) -> Option<super::SemanticEquationPrediction> {
        let equation = self.compile_equation(text)?;
        self.answer_binding
            .solve_without_slot_binding(&equation, candidates)
    }

    pub(super) fn select_frame_with_ablation(
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

    pub(super) fn settle_semantic_field(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<L3SemanticFieldSelection> {
        self.settle_semantic_field_inner(text, field_ablation, L3CueTokenMode::All, true)
    }

    pub(super) fn measure_semantic_field(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
    ) -> Option<L3SemanticFieldSelection> {
        self.settle_semantic_field_inner(text, field_ablation, L3CueTokenMode::All, false)
    }

    pub(super) fn measure_semantic_field_with_cue_mode(
        &self,
        text: &str,
        cue_token_mode: L3CueTokenMode,
    ) -> Option<L3SemanticFieldSelection> {
        self.settle_semantic_field_inner(text, L3FieldAblation::default(), cue_token_mode, false)
    }

    fn settle_semantic_field_inner(
        &self,
        text: &str,
        field_ablation: L3FieldAblation,
        cue_token_mode: L3CueTokenMode,
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
        let cue_inference = self.cue_field.infer_with_token_mode(
            text,
            &self.l2,
            &self.frames,
            field_ablation.cues,
            cue_token_mode,
        );
        let cues = &cue_inference.cues;

        let mut best_index = 0usize;
        let mut best_raw_score = f32::NEG_INFINITY;
        let mut raw_runner_up_score = f32::NEG_INFINITY;
        let mut best_settled_score = f32::NEG_INFINITY;
        let mut settled_runner_up_score = f32::NEG_INFINITY;
        let mut best_field_score = L3SemanticFieldScore::default();

        for (index, frame) in self.frames.iter().enumerate() {
            let raw_score = score_frame(frame, &query_tokens, Some(8));
            let field_score = self.field.score(index, cues, field_ablation);
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
            cue_margin: cue_inference.min_margin,
            interference_energy: best_field_score.total().abs(),
        })
    }
}

fn starts_as_unknown_object_question(text: &str, object_anchor: &str) -> bool {
    normalized_tokens(text)
        .windows(2)
        .next()
        .is_some_and(|window| window[0] == "which" && window[1] == object_anchor)
}

#[derive(Clone, Debug)]
struct FrameBuilder {
    unknown_role: String,
    object_anchor: String,
    support: u32,
    weights: HashMap<u32, i32>,
}

pub(super) fn frame_index_for_schema(
    frames: &[L3FrameCenter],
    schema: &SemanticSchemaKey,
) -> Option<usize> {
    frames.iter().position(|frame| &frame.schema == schema)
}

pub(super) fn frame_sort_key(frame: &L3FrameCenter) -> String {
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

fn route_family(route: &str) -> String {
    route.replace('.', "-")
}

pub(super) fn compact_features(weights: HashMap<u32, i32>) -> Vec<(u32, i16)> {
    let mut features = weights
        .into_iter()
        .map(|(token, weight)| (token, weight.min(i32::from(i16::MAX)) as i16))
        .collect::<Vec<_>>();
    features.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    features
}

pub(super) fn score_frame(
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

pub(super) fn copy_object_label_after_anchor(text: &str, anchor: &str) -> Option<String> {
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
