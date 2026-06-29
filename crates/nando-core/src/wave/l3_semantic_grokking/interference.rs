use std::collections::{HashMap, HashSet};

use super::super::L2CenterMemory;
use super::contrastive::L3ContrastiveTrainingSet;
use super::cue_field::{L3LearnedCueField, L3SemanticFieldCues};
use super::tokens::{motif_tokens, normalized_surface_key};
use super::{
    L3_INTERFERENCE_EDGE_BYTES, L3FieldAblation, L3FrameCenter, L3SemanticExample,
    L3SemanticFieldScore, frame_index_for_schema, score_frame,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum L3SemanticFieldLane {
    Attraction,
    Repulsion,
    AntiTrap,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct L3SemanticInterferenceEdge {
    pub(super) lane: L3SemanticFieldLane,
    pub(super) source_kind: String,
    pub(super) source_value: String,
    pub(super) target_frame: usize,
    pub(super) weight: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct L3SemanticInterferenceKey {
    lane: L3SemanticFieldLane,
    source_kind: String,
    source_value: String,
    target_frame: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct L3SemanticInterferenceField {
    pub(super) edges: Vec<L3SemanticInterferenceEdge>,
    pub(super) learned: bool,
    pub(super) contrastive: bool,
    pub(super) manual_weight_table_used: bool,
}

impl L3SemanticInterferenceField {
    pub(super) fn from_training(
        frames: &[L3FrameCenter],
        l2: &L2CenterMemory,
        examples: &[L3SemanticExample],
        cue_field: &L3LearnedCueField,
        contrastive_set: &L3ContrastiveTrainingSet,
    ) -> Self {
        let mut weights: HashMap<L3SemanticInterferenceKey, f32> = HashMap::new();
        let mut trap_cue_cache: HashMap<String, L3SemanticFieldCues> = HashMap::new();

        for example in examples {
            let Some(correct_frame) = frame_index_for_schema(frames, &example.fact.schema) else {
                continue;
            };
            let cues = L3SemanticFieldCues::from_schema(&example.fact.schema);
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
        }

        for negative in &contrastive_set.negative_cases {
            let trap_cues = trap_cue_cache
                .entry(normalized_surface_key(&negative.text))
                .or_insert_with(|| cue_field.infer(&negative.text, l2, frames, false).cues)
                .clone();
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

    pub(super) fn hot_bytes(&self) -> usize {
        self.edges.len() * L3_INTERFERENCE_EDGE_BYTES
    }

    pub(super) fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub(super) fn score(
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

pub(super) fn exact_frame_index_for_cues(
    frames: &[L3FrameCenter],
    cues: &L3SemanticFieldCues,
) -> Option<usize> {
    frames.iter().position(|frame| cues.complete_for(frame))
}

pub(super) fn nearest_wrong_frame_index(
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

pub(super) fn lane_order(lane: L3SemanticFieldLane) -> u8 {
    match lane {
        L3SemanticFieldLane::Attraction => 0,
        L3SemanticFieldLane::Repulsion => 1,
        L3SemanticFieldLane::AntiTrap => 2,
    }
}
