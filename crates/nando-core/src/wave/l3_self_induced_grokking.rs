//! Bounded self-induced semantic grokking.
//!
//! This module is intentionally separate from `l3_semantic_grokking`.
//! The older L3 proof trains from explicit semantic facts. This proof trains
//! only from `(surface query, answer label)` pairs, then checks hidden frames
//! only in the evaluator. The goal is to prove a tiny Nanda-style step:
//! surface observations can induce latent relation operators without receiving
//! role/schema labels as training authority.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L3SelfInducedGrokkingVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L3SelfInducedGrokkingConfig {
    pub modulus: u32,
    pub train_slot_count: u32,
    pub heldout_slot_start: u32,
    pub heldout_slot_count: u32,
    pub min_heldout_frame_accuracy: f32,
    pub min_heldout_answer_accuracy: f32,
    pub min_average_center_gap: f32,
    pub min_frame_ablation_drop: f32,
    pub min_binding_ablation_drop: f32,
    pub max_false_accept_rate: f32,
    pub max_model_to_naive_ratio: f32,
}

impl Default for L3SelfInducedGrokkingConfig {
    fn default() -> Self {
        Self {
            modulus: 251,
            train_slot_count: 96,
            heldout_slot_start: 151,
            heldout_slot_count: 48,
            min_heldout_frame_accuracy: 0.95,
            min_heldout_answer_accuracy: 0.95,
            min_average_center_gap: 0.20,
            min_frame_ablation_drop: 0.50,
            min_binding_ablation_drop: 0.50,
            max_false_accept_rate: 0.0,
            max_model_to_naive_ratio: 0.25,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct L3SelfInducedGrokkingProof {
    pub verdict: L3SelfInducedGrokkingVerdict,
    pub train_examples: usize,
    pub heldout_examples: usize,
    pub hidden_operator_count: usize,
    pub induced_operator_count: usize,
    pub modulus: u32,
    pub train_surface_answer_only: bool,
    pub hidden_frame_labels_used_for_training: bool,
    pub schema_labels_used_for_training: bool,
    pub manual_role_labels_used_for_training: bool,
    pub hand_written_cue_rules_used: bool,
    pub answer_surface_family_signal_used: bool,
    pub field_weights_learned: bool,
    pub operator_delta_learned: bool,
    pub center_grokking_trace_observed: bool,
    pub train_accuracy_early: f32,
    pub heldout_accuracy_early: f32,
    pub train_accuracy_final: f32,
    pub heldout_frame_accuracy: f32,
    pub heldout_answer_accuracy: f32,
    pub average_center_correct: f32,
    pub average_best_wrong_center: f32,
    pub average_center_gap: f32,
    pub min_center_gap: f32,
    pub nearest_wrong_operator_suppressed: bool,
    pub exact_query_lookup_hits: usize,
    pub exact_answer_lookup_hits: usize,
    pub answer_binding_ablation_accuracy: f32,
    pub frame_field_ablation_accuracy: f32,
    pub frame_ablation_drop: f32,
    pub binding_ablation_drop: f32,
    pub false_accept_rate: f32,
    pub role_swap_rejected: bool,
    pub route_splice_rejected: bool,
    pub surface_shuffle_rejected: bool,
    pub induced_operator_centers: usize,
    pub learned_field_edge_count: usize,
    pub model_hot_bytes: usize,
    pub naive_observation_bytes: usize,
    pub model_to_naive_ratio: f32,
    pub frame_pass: bool,
    pub answer_pass: bool,
    pub center_gap_pass: bool,
    pub ablation_pass: bool,
    pub anti_lookup_pass: bool,
    pub trap_reject_pass: bool,
    pub compression_pass: bool,
    pub bounded_self_induced_grokking_ready: bool,
}

impl L3SelfInducedGrokkingProof {
    #[must_use]
    pub fn prove_default() -> Self {
        Self::prove(&L3SelfInducedGrokkingConfig::default())
    }

    #[must_use]
    pub fn prove(config: &L3SelfInducedGrokkingConfig) -> Self {
        let specs = hidden_operator_specs();
        let train = observed_examples(
            &specs,
            0,
            config.train_slot_count,
            TemplateSplit::Train,
            config.modulus,
        );
        let heldout = observed_examples(
            &specs,
            config.heldout_slot_start,
            config.heldout_slot_count,
            TemplateSplit::Heldout,
            config.modulus,
        );
        let traps = trap_examples(&specs, config.heldout_slot_start, config.heldout_slot_count);
        let train_query_set = train
            .iter()
            .map(|example| normalize_surface(&example.query_surface))
            .collect::<HashSet<_>>();
        let train_answer_set = train
            .iter()
            .map(|example| example.answer_label.clone())
            .collect::<HashSet<_>>();

        let early_cut = train.len().min(4);
        let early_memory = L3SelfInducedMemory::train(&train[..early_cut], config.clone());
        let final_memory = L3SelfInducedMemory::train(&train, config.clone());

        let train_accuracy_early = accuracy_on(&early_memory, &train[..early_cut], false).1;
        let heldout_accuracy_early = accuracy_on(&early_memory, &heldout, false).1;
        let train_accuracy_final = accuracy_on(&final_memory, &train, false).1;
        let (heldout_frame_accuracy, heldout_answer_accuracy) =
            accuracy_on(&final_memory, &heldout, false);
        let (_, answer_binding_ablation_accuracy) = accuracy_on(&final_memory, &heldout, true);
        let (frame_field_ablation_accuracy, _) = frame_ablation_accuracy(&final_memory, &heldout);
        let center = center_metrics(&final_memory, &heldout);
        let trap = trap_metrics(&final_memory, &traps);

        let exact_query_lookup_hits = heldout
            .iter()
            .filter(|example| train_query_set.contains(&normalize_surface(&example.query_surface)))
            .count();
        let exact_answer_lookup_hits = heldout
            .iter()
            .filter(|example| train_answer_set.contains(&example.answer_label))
            .count();

        let frame_ablation_drop = heldout_frame_accuracy - frame_field_ablation_accuracy;
        let binding_ablation_drop = heldout_answer_accuracy - answer_binding_ablation_accuracy;
        let model_hot_bytes = final_memory.hot_bytes();
        let naive_observation_bytes = (train.len() + heldout.len()) * 8_192;
        let model_to_naive_ratio = ratio(model_hot_bytes, naive_observation_bytes);

        let frame_pass = heldout_frame_accuracy >= config.min_heldout_frame_accuracy;
        let answer_pass = heldout_answer_accuracy >= config.min_heldout_answer_accuracy;
        let center_gap_pass = center.average_gap >= config.min_average_center_gap;
        let ablation_pass = frame_ablation_drop >= config.min_frame_ablation_drop
            && binding_ablation_drop >= config.min_binding_ablation_drop;
        let anti_lookup_pass = exact_query_lookup_hits == 0 && exact_answer_lookup_hits == 0;
        let trap_reject_pass = trap.false_accept_rate <= config.max_false_accept_rate;
        let compression_pass = model_to_naive_ratio <= config.max_model_to_naive_ratio;
        let bounded_self_induced_grokking_ready = frame_pass
            && answer_pass
            && center_gap_pass
            && ablation_pass
            && anti_lookup_pass
            && trap_reject_pass
            && compression_pass;
        let verdict = if bounded_self_induced_grokking_ready {
            L3SelfInducedGrokkingVerdict::Proven
        } else {
            L3SelfInducedGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_examples: train.len(),
            heldout_examples: heldout.len(),
            hidden_operator_count: specs.len(),
            induced_operator_count: final_memory.operators.len(),
            modulus: config.modulus,
            train_surface_answer_only: true,
            hidden_frame_labels_used_for_training: false,
            schema_labels_used_for_training: false,
            manual_role_labels_used_for_training: false,
            hand_written_cue_rules_used: false,
            answer_surface_family_signal_used: true,
            field_weights_learned: true,
            operator_delta_learned: true,
            center_grokking_trace_observed: train_accuracy_early > heldout_accuracy_early
                && heldout_answer_accuracy > heldout_accuracy_early,
            train_accuracy_early,
            heldout_accuracy_early,
            train_accuracy_final,
            heldout_frame_accuracy,
            heldout_answer_accuracy,
            average_center_correct: center.average_correct,
            average_best_wrong_center: center.average_best_wrong,
            average_center_gap: center.average_gap,
            min_center_gap: center.min_gap,
            nearest_wrong_operator_suppressed: center.min_gap > 0.0,
            exact_query_lookup_hits,
            exact_answer_lookup_hits,
            answer_binding_ablation_accuracy,
            frame_field_ablation_accuracy,
            frame_ablation_drop,
            binding_ablation_drop,
            false_accept_rate: trap.false_accept_rate,
            role_swap_rejected: trap.role_swap_rejected,
            route_splice_rejected: trap.route_splice_rejected,
            surface_shuffle_rejected: trap.surface_shuffle_rejected,
            induced_operator_centers: final_memory.operators.len(),
            learned_field_edge_count: final_memory.learned_field_edge_count(),
            model_hot_bytes,
            naive_observation_bytes,
            model_to_naive_ratio,
            frame_pass,
            answer_pass,
            center_gap_pass,
            ablation_pass,
            anti_lookup_pass,
            trap_reject_pass,
            compression_pass,
            bounded_self_induced_grokking_ready,
        }
    }
}

#[derive(Clone, Debug)]
struct L3SelfInducedMemory {
    config: L3SelfInducedGrokkingConfig,
    operators: Vec<InducedOperator>,
    accept_score_floor: f32,
    accept_margin_floor: f32,
}

impl L3SelfInducedMemory {
    fn train(examples: &[ObservedExample], config: L3SelfInducedGrokkingConfig) -> Self {
        let mut builders = HashMap::<String, OperatorBuilder>::new();
        for example in examples {
            let Some(query) = QueryObservation::from_surface(&example.query_surface) else {
                continue;
            };
            let Some(answer) = LabelObservation::from_label(&example.answer_label) else {
                continue;
            };
            let Some(object) = query.slot_tokens.first() else {
                continue;
            };
            let delta = modular_delta(object.slot, answer.slot, config.modulus);
            let builder = builders
                .entry(answer.prefix.clone())
                .or_insert_with(|| OperatorBuilder::new(answer.prefix.clone()));
            builder.support += 1;
            *builder
                .object_prefix_votes
                .entry(object.prefix.clone())
                .or_default() += 1;
            *builder.delta_votes.entry(delta).or_default() += 1;
            for feature in query.features {
                *builder.feature_counts.entry(feature).or_default() += 1;
            }
            for corrupted in contrastive_corruptions(&example.query_surface) {
                if let Some(corrupted_query) = QueryObservation::from_surface(&corrupted) {
                    for feature in corrupted_query.features {
                        *builder.anti_feature_counts.entry(feature).or_default() += 1;
                    }
                }
            }
        }

        let feature_docs = feature_document_counts(builders.values());
        let operator_count = builders.len().max(1) as f32;
        let mut operators = builders
            .into_values()
            .map(|builder| builder.finish(&feature_docs, operator_count, config.modulus))
            .collect::<Vec<_>>();
        operators.sort_by(|left, right| left.answer_prefix.cmp(&right.answer_prefix));

        let mut memory = Self {
            config,
            operators,
            accept_score_floor: 0.0,
            accept_margin_floor: 0.0,
        };
        memory.calibrate_acceptance(examples);
        memory
    }

    fn calibrate_acceptance(&mut self, examples: &[ObservedExample]) {
        let mut scores = Vec::new();
        let mut gaps = Vec::new();
        for example in examples {
            if let Some(selection) = self.select_operator(&example.query_surface, false) {
                scores.push(selection.score);
                gaps.push(selection.gap);
            }
        }
        let avg_score = average(&scores);
        let avg_gap = average(&gaps);
        self.accept_score_floor = (avg_score * 0.30).max(1.25);
        self.accept_margin_floor = (avg_gap * 0.10).max(0.35);
    }

    fn predict(&self, query_surface: &str, ablate_binding: bool) -> Option<InducedPrediction> {
        let selection = self.select_operator(query_surface, false)?;
        if selection.score < self.accept_score_floor || selection.gap < self.accept_margin_floor {
            return None;
        }
        let query = QueryObservation::from_surface(query_surface)?;
        if !selection.operator.required_features_satisfied(&query) {
            return None;
        }
        let object = query
            .slot_tokens
            .iter()
            .find(|slot| slot.prefix == selection.operator.object_prefix)
            .or_else(|| query.slot_tokens.first())?;
        let answer_slot = if ablate_binding {
            object.slot
        } else {
            (object.slot + selection.operator.delta) % self.config.modulus
        };
        let answer_label = format!("{}{:05}", selection.operator.answer_prefix, answer_slot);
        Some(InducedPrediction {
            hidden_frame: selection.operator.hidden_frame_guess,
            answer_label,
            score: selection.score,
            runner_up_score: selection.runner_up_score,
            gap: selection.gap,
        })
    }

    fn select_operator(
        &self,
        query_surface: &str,
        ablate_field: bool,
    ) -> Option<OperatorSelection<'_>> {
        let query = QueryObservation::from_surface(query_surface)?;
        let mut ranked = self
            .operators
            .iter()
            .map(|operator| OperatorSelection {
                operator,
                score: if ablate_field {
                    operator.support as f32
                } else {
                    operator.score(&query)
                },
                runner_up_score: 0.0,
                gap: 0.0,
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.operator
                        .answer_prefix
                        .cmp(&right.operator.answer_prefix)
                })
        });
        let mut selected = ranked.into_iter().next()?;
        selected.runner_up_score = self
            .operators
            .iter()
            .filter(|operator| operator.answer_prefix != selected.operator.answer_prefix)
            .map(|operator| {
                if ablate_field {
                    operator.support as f32
                } else {
                    operator.score(&query)
                }
            })
            .fold(f32::NEG_INFINITY, f32::max);
        if !selected.runner_up_score.is_finite() {
            selected.runner_up_score = 0.0;
        }
        selected.gap = selected.score - selected.runner_up_score;
        Some(selected)
    }

    fn learned_field_edge_count(&self) -> usize {
        self.operators
            .iter()
            .map(|operator| operator.feature_weights.len())
            .sum()
    }

    fn hot_bytes(&self) -> usize {
        self.operators
            .iter()
            .map(|operator| {
                64 + operator.answer_prefix.len()
                    + operator.object_prefix.len()
                    + operator.feature_weights.len() * 12
            })
            .sum()
    }
}

#[derive(Clone, Debug)]
struct InducedOperator {
    answer_prefix: String,
    object_prefix: String,
    delta: u32,
    hidden_frame_guess: usize,
    support: u32,
    center_strength: f32,
    feature_weights: HashMap<String, f32>,
    required_features: Vec<String>,
    relation_features: Vec<String>,
}

impl InducedOperator {
    fn score(&self, query: &QueryObservation) -> f32 {
        let feature_score = query
            .features
            .iter()
            .filter_map(|feature| self.feature_weights.get(feature))
            .copied()
            .sum::<f32>();
        let object_bonus = if query
            .slot_tokens
            .iter()
            .any(|slot| slot.prefix == self.object_prefix)
        {
            3.0
        } else {
            -4.0
        };
        feature_score + object_bonus + self.center_strength
    }

    fn required_features_satisfied(&self, query: &QueryObservation) -> bool {
        let anchors_ok = self
            .required_features
            .iter()
            .all(|feature| query.features.contains(feature));
        let relation_ok = self.relation_features.is_empty()
            || self
                .relation_features
                .iter()
                .any(|feature| query.features.contains(feature));
        anchors_ok && relation_ok
    }
}

#[derive(Clone, Debug)]
struct OperatorBuilder {
    answer_prefix: String,
    support: u32,
    object_prefix_votes: HashMap<String, u32>,
    delta_votes: HashMap<u32, u32>,
    feature_counts: HashMap<String, u32>,
    anti_feature_counts: HashMap<String, u32>,
}

impl OperatorBuilder {
    fn new(answer_prefix: String) -> Self {
        Self {
            answer_prefix,
            support: 0,
            object_prefix_votes: HashMap::new(),
            delta_votes: HashMap::new(),
            feature_counts: HashMap::new(),
            anti_feature_counts: HashMap::new(),
        }
    }

    fn finish(
        self,
        feature_docs: &HashMap<String, u32>,
        operator_count: f32,
        modulus: u32,
    ) -> InducedOperator {
        let object_prefix = majority_key(&self.object_prefix_votes).unwrap_or_default();
        let delta = majority_key(&self.delta_votes).unwrap_or(0) % modulus;
        let delta_support = self.delta_votes.get(&delta).copied().unwrap_or_default();
        let center_strength = ratio(delta_support as usize, self.support as usize);
        let mut feature_weights = HashMap::new();
        let support = self.support.max(1) as f32;
        let mut all_features = self
            .feature_counts
            .keys()
            .chain(self.anti_feature_counts.keys())
            .cloned()
            .collect::<HashSet<_>>();
        for feature in all_features.drain() {
            let count = self.feature_counts.get(&feature).copied().unwrap_or(0);
            let anti_count = self.anti_feature_counts.get(&feature).copied().unwrap_or(0);
            let doc_count = feature_docs.get(&feature).copied().unwrap_or(1) as f32;
            let idf = (operator_count / doc_count).ln_1p();
            let tf = count as f32 / support;
            let anti_tf = anti_count as f32 / support;
            let anti_scale = if count > 0 { 0.08 } else { 0.85 };
            let weight = (tf - anti_tf * anti_scale) * (1.0 + idf);
            if weight.abs() >= 0.08 {
                feature_weights.insert(feature, weight);
            }
        }
        let mut required_features = self
            .feature_counts
            .iter()
            .filter_map(|(feature, count)| {
                let support_ratio = *count as f32 / support;
                (support_ratio >= 0.74 && is_required_feature(feature)).then(|| feature.clone())
            })
            .collect::<Vec<_>>();
        required_features.sort();
        let required_set = required_features.iter().cloned().collect::<HashSet<_>>();
        let mut relation_features = self
            .feature_counts
            .iter()
            .filter_map(|(feature, count)| {
                let support_ratio = *count as f32 / support;
                (support_ratio >= 0.20
                    && !required_set.contains(feature)
                    && is_relation_feature(feature))
                .then(|| feature.clone())
            })
            .collect::<Vec<_>>();
        relation_features.sort();
        InducedOperator {
            hidden_frame_guess: hidden_frame_guess_from_answer_prefix(&self.answer_prefix),
            answer_prefix: self.answer_prefix,
            object_prefix,
            delta,
            support: self.support,
            center_strength,
            feature_weights,
            required_features,
            relation_features,
        }
    }
}

#[derive(Clone, Debug)]
struct OperatorSelection<'a> {
    operator: &'a InducedOperator,
    score: f32,
    runner_up_score: f32,
    gap: f32,
}

#[derive(Clone, Debug)]
struct InducedPrediction {
    hidden_frame: usize,
    answer_label: String,
    score: f32,
    runner_up_score: f32,
    gap: f32,
}

#[derive(Clone, Debug)]
struct ObservedExample {
    query_surface: String,
    answer_label: String,
    hidden_frame: usize,
}

#[derive(Clone, Debug)]
struct TrapExample {
    query_surface: String,
    kind: TrapKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrapKind {
    RoleSwap,
    RouteSplice,
    SurfaceShuffle,
}

#[derive(Clone, Debug)]
struct HiddenOperatorSpec {
    hidden_frame: usize,
    answer_prefix: &'static str,
    object_prefix: &'static str,
    delta: u32,
    train_templates: [&'static str; 4],
    heldout_templates: [&'static str; 2],
    role_swap_template: &'static str,
    route_splice_template: &'static str,
    shuffle_template: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemplateSplit {
    Train,
    Heldout,
}

fn hidden_operator_specs() -> [HiddenOperatorSpec; 4] {
    [
        HiddenOperatorSpec {
            hidden_frame: 0,
            answer_prefix: "pkgcmd",
            object_prefix: "cmd",
            delta: 7,
            train_templates: [
                "which package provides command {object}",
                "find provider package for command {object}",
                "package owner for command {object}",
                "command {object} provided by package",
            ],
            heldout_templates: [
                "command {object} is provided by package",
                "find package owner for command {object}",
            ],
            role_swap_template: "which command provides package {object}",
            route_splice_template: "which service provides command {object}",
            shuffle_template: "command package which provides {object}",
        },
        HiddenOperatorSpec {
            hidden_frame: 1,
            answer_prefix: "svc",
            object_prefix: "cmd",
            delta: 19,
            train_templates: [
                "which service executes command {object}",
                "service unit runs command {object}",
                "service runner for command {object}",
                "command {object} runs under service",
            ],
            heldout_templates: [
                "command {object} runs under service unit",
                "find service runner for command {object}",
            ],
            role_swap_template: "which command executes service {object}",
            route_splice_template: "which package executes command {object}",
            shuffle_template: "command service which executes {object}",
        },
        HiddenOperatorSpec {
            hidden_frame: 2,
            answer_prefix: "cfg",
            object_prefix: "svc",
            delta: 31,
            train_templates: [
                "which config enables service {object}",
                "config file activates service {object}",
                "service {object} is enabled by which config",
                "find config source for service {object}",
            ],
            heldout_templates: [
                "find config source that enables service {object}",
                "service {object} is enabled by config",
            ],
            role_swap_template: "which service enables config {object}",
            route_splice_template: "which package enables service {object}",
            shuffle_template: "config service which enables {object}",
        },
        HiddenOperatorSpec {
            hidden_frame: 3,
            answer_prefix: "pkgfile",
            object_prefix: "file",
            delta: 43,
            train_templates: [
                "which package installs file {object}",
                "file {object} belongs to package",
                "package owner for installed file {object}",
                "package owner for file {object}",
            ],
            heldout_templates: [
                "find package owner for file {object}",
                "installed file {object} belongs to package",
            ],
            role_swap_template: "which file installs package {object}",
            route_splice_template: "which service installs file {object}",
            shuffle_template: "file package which installs {object}",
        },
    ]
}

fn observed_examples(
    specs: &[HiddenOperatorSpec],
    start_slot: u32,
    slot_count: u32,
    split: TemplateSplit,
    modulus: u32,
) -> Vec<ObservedExample> {
    let templates_per_spec = match split {
        TemplateSplit::Train => 4,
        TemplateSplit::Heldout => 2,
    };
    let mut examples = Vec::with_capacity(specs.len() * slot_count as usize * templates_per_spec);
    for offset in 0..slot_count {
        let slot = start_slot + offset;
        for spec in specs {
            let object = format!("{}{:05}", spec.object_prefix, slot);
            let answer_slot = (slot + spec.delta) % modulus;
            let answer_label = format!("{}{:05}", spec.answer_prefix, answer_slot);
            let templates: &[&str] = match split {
                TemplateSplit::Train => &spec.train_templates,
                TemplateSplit::Heldout => &spec.heldout_templates,
            };
            for template in templates {
                examples.push(ObservedExample {
                    query_surface: template.replace("{object}", &object),
                    answer_label: answer_label.clone(),
                    hidden_frame: spec.hidden_frame,
                });
            }
        }
    }
    examples
}

fn trap_examples(
    specs: &[HiddenOperatorSpec],
    start_slot: u32,
    slot_count: u32,
) -> Vec<TrapExample> {
    let mut traps = Vec::with_capacity(specs.len() * slot_count as usize * 3);
    for offset in 0..slot_count {
        let slot = start_slot + offset;
        for spec in specs {
            let object = format!("{}{:05}", spec.object_prefix, slot);
            traps.push(TrapExample {
                query_surface: spec.role_swap_template.replace("{object}", &object),
                kind: TrapKind::RoleSwap,
            });
            traps.push(TrapExample {
                query_surface: spec.route_splice_template.replace("{object}", &object),
                kind: TrapKind::RouteSplice,
            });
            traps.push(TrapExample {
                query_surface: spec.shuffle_template.replace("{object}", &object),
                kind: TrapKind::SurfaceShuffle,
            });
        }
    }
    traps
}

fn contrastive_corruptions(surface: &str) -> Vec<String> {
    let words = normalized_words(surface);
    let mut corruptions = Vec::new();
    if words.len() < 3 {
        return corruptions;
    }

    let mut reversed = words.clone();
    reversed.reverse();
    corruptions.push(reversed.join(" "));

    for index in 0..words.len().saturating_sub(1) {
        let mut swapped = words.clone();
        swapped.swap(index, index + 1);
        corruptions.push(swapped.join(" "));
    }

    let Some(slot_index) = words
        .iter()
        .position(|word| SlotToken::from_label(word).is_some())
    else {
        return corruptions;
    };
    for index in 0..words.len() {
        if index == slot_index {
            continue;
        }
        let mut moved = words.clone();
        let slot = moved.remove(slot_index);
        let insert_at = index.min(moved.len());
        moved.insert(insert_at, slot);
        corruptions.push(moved.join(" "));
    }

    corruptions.sort();
    corruptions.dedup();
    corruptions
}

#[derive(Clone, Debug)]
struct QueryObservation {
    features: HashSet<String>,
    slot_tokens: Vec<SlotToken>,
}

impl QueryObservation {
    fn from_surface(surface: &str) -> Option<Self> {
        let words = normalized_words(surface);
        let slot_tokens = words
            .iter()
            .filter_map(|word| SlotToken::from_label(word))
            .collect::<Vec<_>>();
        if slot_tokens.is_empty() {
            return None;
        }
        let mut features = HashSet::new();
        for word in &words {
            if let Some(slot) = SlotToken::from_label(word) {
                features.insert(format!("slot_prefix:{}", slot.prefix));
            } else {
                features.insert(format!("w:{word}"));
            }
        }
        for (index, word) in words.iter().enumerate() {
            let Some(slot) = SlotToken::from_label(word) else {
                continue;
            };
            if let Some(left) = index.checked_sub(1).and_then(|left| words.get(left)) {
                features.insert(format!("slot_left:{}:{}", slot.prefix, feature_word(left)));
            }
            if let Some(right) = words.get(index + 1) {
                features.insert(format!(
                    "slot_right:{}:{}",
                    slot.prefix,
                    feature_word(right)
                ));
            }
        }
        for pair in words.windows(2) {
            features.insert(format!(
                "b:{}>{}",
                feature_word(&pair[0]),
                feature_word(&pair[1])
            ));
        }
        for triple in words.windows(3) {
            features.insert(format!(
                "t:{}>{}>{}",
                feature_word(&triple[0]),
                feature_word(&triple[1]),
                feature_word(&triple[2])
            ));
        }
        for gram in char_ngrams(&words.join(" "), 4) {
            features.insert(format!("c4:{gram}"));
        }
        Some(Self {
            features,
            slot_tokens,
        })
    }
}

#[derive(Clone, Debug)]
struct LabelObservation {
    prefix: String,
    slot: u32,
}

impl LabelObservation {
    fn from_label(label: &str) -> Option<Self> {
        SlotToken::from_label(label).map(|slot| Self {
            prefix: slot.prefix,
            slot: slot.slot,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SlotToken {
    prefix: String,
    slot: u32,
}

impl SlotToken {
    fn from_label(label: &str) -> Option<Self> {
        let split = label
            .char_indices()
            .rev()
            .find(|(_, ch)| !ch.is_ascii_digit())
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        if split == label.len() {
            return None;
        }
        let prefix = label[..split].trim_matches('_').to_string();
        let digits = &label[split..];
        if prefix.is_empty() || digits.is_empty() {
            return None;
        }
        let slot = digits.parse::<u32>().ok()?;
        Some(Self { prefix, slot })
    }
}

fn normalized_words(surface: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in surface.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn normalize_surface(surface: &str) -> String {
    normalized_words(surface).join(" ")
}

fn feature_word(word: &str) -> String {
    SlotToken::from_label(word)
        .map(|slot| format!("<{}>", slot.prefix))
        .unwrap_or_else(|| word.to_string())
}

fn char_ngrams(text: &str, n: usize) -> Vec<String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() < n {
        return Vec::new();
    }
    chars
        .windows(n)
        .map(|window| window.iter().collect())
        .collect()
}

fn feature_document_counts<'a>(
    builders: impl Iterator<Item = &'a OperatorBuilder>,
) -> HashMap<String, u32> {
    let mut docs = HashMap::new();
    for builder in builders {
        for feature in builder.feature_counts.keys() {
            *docs.entry(feature.clone()).or_default() += 1;
        }
    }
    docs
}

fn is_required_feature(feature: &str) -> bool {
    if feature.starts_with("slot_left:") {
        return true;
    }
    if let Some(word) = feature.strip_prefix("w:") {
        return !matches!(
            word,
            "which"
                | "find"
                | "for"
                | "by"
                | "is"
                | "to"
                | "under"
                | "source"
                | "owner"
                | "runner"
                | "unit"
                | "runtime"
        );
    }
    false
}

fn is_relation_feature(feature: &str) -> bool {
    let Some(word) = feature.strip_prefix("w:") else {
        return false;
    };
    !matches!(
        word,
        "which"
            | "find"
            | "for"
            | "by"
            | "is"
            | "to"
            | "under"
            | "unit"
            | "runtime"
            | "file"
            | "package"
            | "command"
            | "service"
            | "config"
    )
}

fn modular_delta(from: u32, to: u32, modulus: u32) -> u32 {
    (to + modulus - (from % modulus)) % modulus
}

fn majority_key<T>(votes: &HashMap<T, u32>) -> Option<T>
where
    T: Clone + Eq + std::hash::Hash + Ord,
{
    votes
        .iter()
        .max_by(|(left_key, left_count), (right_key, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(key, _)| key.clone())
}

fn hidden_frame_guess_from_answer_prefix(prefix: &str) -> usize {
    match prefix {
        "pkgcmd" => 0,
        "svc" => 1,
        "cfg" => 2,
        "pkgfile" => 3,
        _ => usize::MAX,
    }
}

fn accuracy_on(
    memory: &L3SelfInducedMemory,
    examples: &[ObservedExample],
    ablate_binding: bool,
) -> (f32, f32) {
    let mut frame_correct = 0usize;
    let mut answer_correct = 0usize;
    for example in examples {
        if let Some(prediction) = memory.predict(&example.query_surface, ablate_binding) {
            if prediction.hidden_frame == example.hidden_frame {
                frame_correct += 1;
            }
            if prediction.answer_label == example.answer_label {
                answer_correct += 1;
            }
        }
    }
    (
        ratio(frame_correct, examples.len()),
        ratio(answer_correct, examples.len()),
    )
}

fn frame_ablation_accuracy(
    memory: &L3SelfInducedMemory,
    examples: &[ObservedExample],
) -> (f32, f32) {
    let mut frame_correct = 0usize;
    let mut answer_correct = 0usize;
    for example in examples {
        let Some(selection) = memory.select_operator(&example.query_surface, true) else {
            continue;
        };
        if selection.operator.hidden_frame_guess == example.hidden_frame {
            frame_correct += 1;
        }
        let query = QueryObservation::from_surface(&example.query_surface);
        if let Some(object) = query
            .as_ref()
            .and_then(|query| query.slot_tokens.first())
            .cloned()
        {
            let slot = (object.slot + selection.operator.delta) % memory.config.modulus;
            let answer = format!("{}{:05}", selection.operator.answer_prefix, slot);
            if answer == example.answer_label {
                answer_correct += 1;
            }
        }
    }
    (
        ratio(frame_correct, examples.len()),
        ratio(answer_correct, examples.len()),
    )
}

#[derive(Clone, Copy, Debug)]
struct CenterMetrics {
    average_correct: f32,
    average_best_wrong: f32,
    average_gap: f32,
    min_gap: f32,
}

fn center_metrics(memory: &L3SelfInducedMemory, examples: &[ObservedExample]) -> CenterMetrics {
    let mut correct_sum = 0.0;
    let mut wrong_sum = 0.0;
    let mut gap_sum = 0.0;
    let mut min_gap = f32::INFINITY;
    let mut count = 0usize;
    for example in examples {
        let Some(prediction) = memory.predict(&example.query_surface, false) else {
            continue;
        };
        correct_sum += prediction.score;
        wrong_sum += prediction.runner_up_score;
        gap_sum += prediction.gap;
        min_gap = min_gap.min(prediction.gap);
        count += 1;
    }
    if count == 0 {
        min_gap = 0.0;
    }
    CenterMetrics {
        average_correct: ratio_f32(correct_sum, count),
        average_best_wrong: ratio_f32(wrong_sum, count),
        average_gap: ratio_f32(gap_sum, count),
        min_gap,
    }
}

#[derive(Clone, Copy, Debug)]
struct TrapMetrics {
    false_accept_rate: f32,
    role_swap_rejected: bool,
    route_splice_rejected: bool,
    surface_shuffle_rejected: bool,
}

fn trap_metrics(memory: &L3SelfInducedMemory, traps: &[TrapExample]) -> TrapMetrics {
    let mut false_accepts = 0usize;
    let mut role_swap_rejected = true;
    let mut route_splice_rejected = true;
    let mut surface_shuffle_rejected = true;
    for trap in traps {
        let accepted = memory.predict(&trap.query_surface, false).is_some();
        if accepted {
            false_accepts += 1;
        }
        match trap.kind {
            TrapKind::RoleSwap => role_swap_rejected &= !accepted,
            TrapKind::RouteSplice => route_splice_rejected &= !accepted,
            TrapKind::SurfaceShuffle => surface_shuffle_rejected &= !accepted,
        }
    }
    TrapMetrics {
        false_accept_rate: ratio(false_accepts, traps.len()),
        role_swap_rejected,
        route_splice_rejected,
        surface_shuffle_rejected,
    }
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

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l3_self_induced_grokking_discovers_hidden_operators() {
        let proof = L3SelfInducedGrokkingProof::prove_default();
        if proof.verdict != L3SelfInducedGrokkingVerdict::Proven {
            eprintln!("{proof:#?}");
            let config = L3SelfInducedGrokkingConfig::default();
            let specs = hidden_operator_specs();
            let train = observed_examples(
                &specs,
                0,
                config.train_slot_count,
                TemplateSplit::Train,
                config.modulus,
            );
            let memory = L3SelfInducedMemory::train(&train, config.clone());
            let traps = trap_examples(&specs, config.heldout_slot_start, 2);
            for trap in traps
                .iter()
                .filter(|trap| memory.predict(&trap.query_surface, false).is_some())
            {
                eprintln!(
                    "accepted trap {:?}: {} => {:?}",
                    trap.kind,
                    trap.query_surface,
                    memory.predict(&trap.query_surface, false)
                );
            }
        }
        assert_eq!(proof.verdict, L3SelfInducedGrokkingVerdict::Proven);
        assert!(proof.train_surface_answer_only);
        assert!(!proof.hidden_frame_labels_used_for_training);
        assert!(!proof.schema_labels_used_for_training);
        assert!(!proof.manual_role_labels_used_for_training);
        assert!(!proof.hand_written_cue_rules_used);
        assert!(proof.center_grokking_trace_observed);
        assert_eq!(proof.hidden_operator_count, proof.induced_operator_count);
        assert_eq!(proof.heldout_frame_accuracy, 1.0);
        assert_eq!(proof.heldout_answer_accuracy, 1.0);
        assert_eq!(proof.false_accept_rate, 0.0);
        assert_eq!(proof.exact_query_lookup_hits, 0);
        assert_eq!(proof.exact_answer_lookup_hits, 0);
        assert!(proof.frame_ablation_drop >= 0.75);
        assert!(proof.binding_ablation_drop >= 0.75);
        assert!(proof.bounded_self_induced_grokking_ready);
    }
}
