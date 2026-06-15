use std::f32::consts::{PI, TAU};

use super::{STAGE2_ORGAN_CELLS, STAGE2_TOP_K, TickTrace, circular_phase_delta};

const INPUT_CELL_ROWS: usize = STAGE2_ORGAN_CELLS * 256;
const BYTE_CLASSES: usize = 8;
const CLASS_CELL_ROWS: usize = STAGE2_ORGAN_CELLS * BYTE_CLASSES;
const MODE_STATES: usize = 4;
const MODE_CELL_ROWS: usize = STAGE2_ORGAN_CELLS * MODE_STATES;
const PAIR_LINK_ROWS: usize = STAGE2_ORGAN_CELLS * (STAGE2_ORGAN_CELLS - 1) / 2;
const TRIPLE_LINK_ROWS: usize =
    STAGE2_ORGAN_CELLS * (STAGE2_ORGAN_CELLS - 1) * (STAGE2_ORGAN_CELLS - 2) / 6;

const MODE_ASCII: usize = 0;
const MODE_CODE: usize = 1;
const MODE_UTF8: usize = 2;
const MODE_LAYOUT_NOISE: usize = 3;

/// A tiny online next-byte adapter trained from wave traces.
///
/// The fixed `Cell32` packets still do not mutate. This adapter is the first
/// deliberately small trainable layer: active cells vote into byte logits, and
/// local feedback adjusts only the active cell rows plus one byte bias vector.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveByteLearner {
    pub learning_rate: f32,
    pub decay: f32,
    mode_state: usize,
    utf8_cooldown: u8,
    code_cooldown: u8,
    layout_cooldown: u8,
    consonant_run: u8,
    byte_bias: [f32; 256],
    class_byte_weights: [[f32; 256]; BYTE_CLASSES],
    mode_byte_weights: [[f32; 256]; MODE_STATES],
    transition_byte_weights: [[f32; 256]; 256],
    cell_byte_weights: [[f32; 256]; STAGE2_ORGAN_CELLS],
    class_cell_byte_weights: Vec<[f32; 256]>,
    mode_cell_byte_weights: Vec<[f32; 256]>,
    input_cell_byte_weights: Vec<[f32; 256]>,
}

/// One next-byte prediction from the trainable live adapter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveBytePrediction {
    pub predicted_byte: u8,
    pub confidence: f32,
    pub score: f32,
}

/// Local update summary for one trained next-byte step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveByteTrainStep {
    pub prediction: LiveBytePrediction,
    pub target_byte: u8,
    pub correct: bool,
    pub margin: f32,
}

/// Aggregate report for a byte-level online training pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveByteTrainReport {
    pub cases: usize,
    pub correct_before_update: usize,
    pub accuracy_before_update: f32,
    pub mean_confidence: f32,
    pub mean_margin: f32,
    pub bias_abs_mean: f32,
    pub class_weight_abs_mean: f32,
    pub mode_weight_abs_mean: f32,
    pub transition_weight_abs_mean: f32,
    pub weight_abs_mean: f32,
    pub context_weight_abs_mean: f32,
}

/// A smaller cell-local learner used to test whether the organism topology
/// beats a monolithic control before changing fixed `Cell32` packets.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell32Learner {
    pub learning_rate: f32,
    pub cells_enabled: usize,
    byte_bias: [f32; 256],
    cell_byte_weights: [[f32; 256]; STAGE2_ORGAN_CELLS],
    input_cell_byte_weights: Vec<[f32; 256]>,
}

/// Eval-gated promotion report for a candidate learner state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell32PromotionReport {
    pub train_cases: usize,
    pub holdout_cases: usize,
    pub base_accuracy: f32,
    pub candidate_accuracy: f32,
    pub holdout_gap: f32,
    pub oos_target_rate: f32,
    pub accepted: bool,
}

/// A link-tissue layer over Cell32 activity.
///
/// Pair rows model `A x B` interactions. Optional triple rows model
/// `A x B x C` interactions. This is the first explicit "tissue" layer:
/// it learns features that do not belong to any single cell.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkTissue {
    pub learning_rate: f32,
    pub cells_enabled: usize,
    pub triples_enabled: bool,
    pub profile: LinkProfile,
    byte_bias: [f32; 256],
    pair_byte_weights: Vec<[f32; 256]>,
    triple_byte_weights: Vec<[f32; 256]>,
    pair_enabled: [bool; PAIR_LINK_ROWS],
    triple_enabled: [bool; TRIPLE_LINK_ROWS],
}

/// Which cell interactions a [`LinkTissue`] is allowed to learn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkProfile {
    All,
    Typed,
}

impl Default for LiveByteLearner {
    fn default() -> Self {
        Self::new(0.08)
    }
}

impl Cell32Learner {
    /// Create a cell-local learner using the first `cells_enabled` Cell32 rows.
    #[must_use]
    pub fn new(cells_enabled: usize, learning_rate: f32) -> Self {
        Self {
            learning_rate,
            cells_enabled: cells_enabled.clamp(1, STAGE2_ORGAN_CELLS),
            byte_bias: [0.0; 256],
            cell_byte_weights: [[0.0; 256]; STAGE2_ORGAN_CELLS],
            input_cell_byte_weights: vec![[0.0; 256]; INPUT_CELL_ROWS],
        }
    }

    /// Predict from active cells without mutating learner state.
    #[must_use]
    pub fn predict(&self, trace: &TickTrace) -> LiveBytePrediction {
        let mut best_byte = 0u8;
        let mut best_score = f32::NEG_INFINITY;
        let mut second_score = f32::NEG_INFINITY;

        for byte in 0..=u8::MAX {
            let score = self.score_byte(trace, byte);
            if score > best_score {
                second_score = best_score;
                best_score = score;
                best_byte = byte;
            } else if score > second_score {
                second_score = score;
            }
        }

        let margin = best_score - second_score;
        LiveBytePrediction {
            predicted_byte: best_byte,
            confidence: sigmoid(margin).clamp(0.0, 1.0),
            score: best_score,
        }
    }

    /// Apply one candidate local update. This is candidate state only: fixed
    /// cell packets and snapshots are untouched until an external gate promotes
    /// the learner.
    pub fn update(&mut self, trace: &TickTrace, target_byte: u8) -> LiveByteTrainStep {
        let prediction = self.predict(trace);
        let correct = prediction.predicted_byte == target_byte;
        let target_score = self.score_byte(trace, target_byte);
        let margin = target_score - prediction.score;

        if !correct {
            self.byte_bias[target_byte as usize] += self.learning_rate * 0.25;
            self.byte_bias[prediction.predicted_byte as usize] -= self.learning_rate * 0.25;

            for (rank, cell_id) in trace
                .active_cell_ids
                .iter()
                .copied()
                .take(trace.active_count)
                .enumerate()
            {
                let cell_index = cell_id as usize;
                if cell_index >= self.cells_enabled {
                    continue;
                }

                let gain = self.learning_rate * active_rank_gain(rank) * trace.coherence.max(0.05);
                self.cell_byte_weights[cell_index][target_byte as usize] += gain;
                self.cell_byte_weights[cell_index][prediction.predicted_byte as usize] -= gain;

                let context_row = input_cell_row(cell_index, trace.input_byte);
                self.input_cell_byte_weights[context_row][target_byte as usize] += gain * 1.40;
                self.input_cell_byte_weights[context_row][prediction.predicted_byte as usize] -=
                    gain * 1.40;
            }
        }

        LiveByteTrainStep {
            prediction,
            target_byte,
            correct,
            margin,
        }
    }

    /// Mean absolute magnitude of candidate cell-local state.
    #[must_use]
    pub fn state_abs_mean(&self) -> f32 {
        let bias_sum: f32 = self.byte_bias.iter().map(|value| value.abs()).sum();
        let cell_sum: f32 = self
            .cell_byte_weights
            .iter()
            .take(self.cells_enabled)
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let context_sum: f32 = self
            .input_cell_byte_weights
            .chunks(256)
            .take(self.cells_enabled)
            .flat_map(|rows| rows.iter().flat_map(|row| row.iter()))
            .map(|value| value.abs())
            .sum();
        let denom = 256 + self.cells_enabled * 256 + self.cells_enabled * 256 * 256;

        (bias_sum + cell_sum + context_sum) / denom as f32
    }

    fn score_byte(&self, trace: &TickTrace, byte: u8) -> f32 {
        self.score(trace, byte)
    }

    /// Score one byte from cell-local candidate state.
    #[must_use]
    pub fn score(&self, trace: &TickTrace, byte: u8) -> f32 {
        let byte_index = byte as usize;
        let mut score = self.byte_bias[byte_index];
        score += phase_affinity(trace.center_phase, byte) * trace.coherence * 0.16;

        for (rank, cell_id) in trace
            .active_cell_ids
            .iter()
            .copied()
            .take(trace.active_count)
            .enumerate()
        {
            let cell_index = cell_id as usize;
            if cell_index >= self.cells_enabled {
                continue;
            }
            let rank_gain = active_rank_gain(rank);
            score += self.cell_byte_weights[cell_index][byte_index] * rank_gain;
            score += self.input_cell_byte_weights[input_cell_row(cell_index, trace.input_byte)]
                [byte_index]
                * rank_gain;
        }

        score
    }
}

impl LinkTissue {
    /// Create a link-tissue over the first `cells_enabled` cells.
    #[must_use]
    pub fn new(cells_enabled: usize, triples_enabled: bool, learning_rate: f32) -> Self {
        Self::with_profile(
            cells_enabled,
            triples_enabled,
            learning_rate,
            LinkProfile::All,
        )
    }

    /// Create a link-tissue with a constrained topology profile.
    #[must_use]
    pub fn with_profile(
        cells_enabled: usize,
        triples_enabled: bool,
        learning_rate: f32,
        profile: LinkProfile,
    ) -> Self {
        let cells_enabled = cells_enabled.clamp(2, STAGE2_ORGAN_CELLS);
        Self {
            learning_rate,
            cells_enabled,
            triples_enabled,
            profile,
            byte_bias: [0.0; 256],
            pair_byte_weights: vec![[0.0; 256]; PAIR_LINK_ROWS],
            triple_byte_weights: vec![[0.0; 256]; TRIPLE_LINK_ROWS],
            pair_enabled: pair_enabled_mask(profile),
            triple_enabled: triple_enabled_mask(profile),
        }
    }

    /// Predict from interaction terms alone.
    #[must_use]
    pub fn predict(&self, trace: &TickTrace) -> LiveBytePrediction {
        let mut best_byte = 0u8;
        let mut best_score = f32::NEG_INFINITY;
        let mut second_score = f32::NEG_INFINITY;

        for byte in 0..=u8::MAX {
            let score = self.score(trace, byte);
            if score > best_score {
                second_score = best_score;
                best_score = score;
                best_byte = byte;
            } else if score > second_score {
                second_score = score;
            }
        }

        let margin = best_score - second_score;
        LiveBytePrediction {
            predicted_byte: best_byte,
            confidence: sigmoid(margin).clamp(0.0, 1.0),
            score: best_score,
        }
    }

    /// Score one byte from pair/triple tissue interactions.
    #[must_use]
    pub fn score(&self, trace: &TickTrace, byte: u8) -> f32 {
        self.score_masked(trace, byte, None, None)
    }

    /// Score one byte while disabling one pair interaction.
    #[must_use]
    pub fn score_without_pair(&self, trace: &TickTrace, byte: u8, pair: (usize, usize)) -> f32 {
        self.score_masked(trace, byte, Some(pair), None)
    }

    /// Score one byte while disabling one triple interaction.
    #[must_use]
    pub fn score_without_triple(
        &self,
        trace: &TickTrace,
        byte: u8,
        triple: (usize, usize, usize),
    ) -> f32 {
        self.score_masked(trace, byte, None, Some(triple))
    }

    /// Mean absolute state magnitude for one pair link.
    #[must_use]
    pub fn pair_state_abs_mean(&self, pair: (usize, usize)) -> f32 {
        let row = pair_row(pair.0, pair.1);
        self.pair_byte_weights[row]
            .iter()
            .map(|value| value.abs())
            .sum::<f32>()
            / 256.0
    }

    /// Mean absolute state magnitude for one triple link.
    #[must_use]
    pub fn triple_state_abs_mean(&self, triple: (usize, usize, usize)) -> f32 {
        let row = triple_row(triple.0, triple.1, triple.2);
        self.triple_byte_weights[row]
            .iter()
            .map(|value| value.abs())
            .sum::<f32>()
            / 256.0
    }

    fn score_masked(
        &self,
        trace: &TickTrace,
        byte: u8,
        disabled_pair: Option<(usize, usize)>,
        disabled_triple: Option<(usize, usize, usize)>,
    ) -> f32 {
        let byte_index = byte as usize;
        let active = active_cells(trace, self.cells_enabled);
        let mut score = self.byte_bias[byte_index] * 0.05;
        let disabled_pair = disabled_pair.map(|(a, b)| sorted_pair(a, b));
        let disabled_triple = disabled_triple.map(|(a, b, c)| sorted_triple(a, b, c));

        for left in 0..active.len() {
            for right in left + 1..active.len() {
                let (a, gain_a) = active[left];
                let (b, gain_b) = active[right];
                let pair = sorted_pair(a, b);
                let row = pair_row(a, b);
                if disabled_pair == Some(pair) || !self.pair_enabled[row] {
                    continue;
                }
                score += self.pair_byte_weights[row][byte_index]
                    * gain_a
                    * gain_b
                    * trace.coherence.max(0.05);
            }
        }

        if self.triples_enabled && active.len() >= 3 {
            for left in 0..active.len() {
                for mid in left + 1..active.len() {
                    for right in mid + 1..active.len() {
                        let (a, gain_a) = active[left];
                        let (b, gain_b) = active[mid];
                        let (c, gain_c) = active[right];
                        let triple = sorted_triple(a, b, c);
                        let row = triple_row(a, b, c);
                        if disabled_triple == Some(triple) || !self.triple_enabled[row] {
                            continue;
                        }
                        score += self.triple_byte_weights[row][byte_index]
                            * gain_a
                            * gain_b
                            * gain_c
                            * trace.coherence.max(0.05);
                    }
                }
            }
        }

        score
    }

    /// Train tissue against its own current interaction prediction.
    pub fn update(&mut self, trace: &TickTrace, target_byte: u8) -> LiveByteTrainStep {
        let prediction = self.predict(trace);
        let target_score = self.score(trace, target_byte);
        let margin = target_score - prediction.score;
        self.update_from_prediction(trace, target_byte, prediction.predicted_byte);

        LiveByteTrainStep {
            prediction,
            target_byte,
            correct: prediction.predicted_byte == target_byte,
            margin,
        }
    }

    /// Train tissue using an external combined prediction.
    pub fn update_from_prediction(&mut self, trace: &TickTrace, target_byte: u8, predicted: u8) {
        if predicted == target_byte {
            return;
        }

        self.byte_bias[target_byte as usize] += self.learning_rate * 0.10;
        self.byte_bias[predicted as usize] -= self.learning_rate * 0.10;
        let active = active_cells(trace, self.cells_enabled);

        for left in 0..active.len() {
            for right in left + 1..active.len() {
                let (a, gain_a) = active[left];
                let (b, gain_b) = active[right];
                let row = pair_row(a, b);
                if !self.pair_enabled[row] {
                    continue;
                }
                let gain = self.learning_rate * gain_a * gain_b * trace.coherence.max(0.05);
                self.pair_byte_weights[row][target_byte as usize] += gain;
                self.pair_byte_weights[row][predicted as usize] -= gain;
            }
        }

        if self.triples_enabled && active.len() >= 3 {
            for left in 0..active.len() {
                for mid in left + 1..active.len() {
                    for right in mid + 1..active.len() {
                        let (a, gain_a) = active[left];
                        let (b, gain_b) = active[mid];
                        let (c, gain_c) = active[right];
                        let row = triple_row(a, b, c);
                        if !self.triple_enabled[row] {
                            continue;
                        }
                        let gain = self.learning_rate
                            * gain_a
                            * gain_b
                            * gain_c
                            * trace.coherence.max(0.05)
                            * 1.25;
                        self.triple_byte_weights[row][target_byte as usize] += gain;
                        self.triple_byte_weights[row][predicted as usize] -= gain;
                    }
                }
            }
        }
    }

    /// Apply cleanup pressure to interaction state.
    ///
    /// This is the LinkTissue analogue of weight decay: weak memorized traces
    /// fade unless later updates keep reinforcing the same circuit.
    pub fn apply_decay(&mut self, retention: f32) {
        let retention = retention.clamp(0.0, 1.0);
        for value in &mut self.byte_bias {
            *value *= retention;
        }
        for row in &mut self.pair_byte_weights {
            for value in row {
                *value *= retention;
            }
        }
        for row in &mut self.triple_byte_weights {
            for value in row {
                *value *= retention;
            }
        }
    }

    /// Reinforce one explicit pair circuit using an external prediction.
    pub fn update_pair_from_prediction(
        &mut self,
        trace: &TickTrace,
        target_byte: u8,
        predicted: u8,
        pair: (usize, usize),
        scale: f32,
    ) {
        if predicted == target_byte {
            return;
        }
        let pair = sorted_pair(pair.0, pair.1);
        let row = pair_row(pair.0, pair.1);
        if !self.pair_enabled[row] {
            return;
        }

        let active = active_cells(trace, self.cells_enabled);
        let mut left_gain = None;
        let mut right_gain = None;
        for (cell_id, gain) in active {
            if cell_id == pair.0 {
                left_gain = Some(gain);
            } else if cell_id == pair.1 {
                right_gain = Some(gain);
            }
        }

        let (Some(left_gain), Some(right_gain)) = (left_gain, right_gain) else {
            return;
        };
        let gain = self.learning_rate
            * scale.max(0.0)
            * left_gain
            * right_gain
            * trace.coherence.max(0.05);
        self.pair_byte_weights[row][target_byte as usize] += gain;
        self.pair_byte_weights[row][predicted as usize] -= gain;
    }

    /// Mean absolute magnitude of the link-tissue state.
    #[must_use]
    pub fn state_abs_mean(&self) -> f32 {
        let pair_sum: f32 = self
            .pair_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let triple_sum: f32 = self
            .triple_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let bias_sum: f32 = self.byte_bias.iter().map(|value| value.abs()).sum();
        let denom = 256 + (PAIR_LINK_ROWS + TRIPLE_LINK_ROWS) * 256;

        (bias_sum + pair_sum + triple_sum) / denom as f32
    }
}

impl Cell32PromotionReport {
    /// Build a simple promotion decision from a holdout comparison.
    #[must_use]
    pub fn new(
        train_cases: usize,
        holdout_cases: usize,
        base_accuracy: f32,
        candidate_accuracy: f32,
        oos_target_rate: f32,
    ) -> Self {
        let holdout_gap = candidate_accuracy - base_accuracy;
        let accepted = holdout_cases > 0 && holdout_gap > 0.0 && oos_target_rate <= 0.50;

        Self {
            train_cases,
            holdout_cases,
            base_accuracy,
            candidate_accuracy,
            holdout_gap,
            oos_target_rate,
            accepted,
        }
    }
}

impl LiveByteLearner {
    /// Create a learner with a small stable online learning rate.
    #[must_use]
    pub fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            decay: 0.999,
            mode_state: MODE_ASCII,
            utf8_cooldown: 0,
            code_cooldown: 0,
            layout_cooldown: 0,
            consonant_run: 0,
            byte_bias: [0.0; 256],
            class_byte_weights: [[0.0; 256]; BYTE_CLASSES],
            mode_byte_weights: [[0.0; 256]; MODE_STATES],
            transition_byte_weights: [[0.0; 256]; 256],
            cell_byte_weights: [[0.0; 256]; STAGE2_ORGAN_CELLS],
            class_cell_byte_weights: vec![[0.0; 256]; CLASS_CELL_ROWS],
            mode_cell_byte_weights: vec![[0.0; 256]; MODE_CELL_ROWS],
            input_cell_byte_weights: vec![[0.0; 256]; INPUT_CELL_ROWS],
        }
    }

    /// Advance the lightweight mode memory, then predict without updating weights.
    pub fn predict_observed(&mut self, trace: &TickTrace) -> LiveBytePrediction {
        self.observe_mode(trace.input_byte);
        self.predict(trace)
    }

    /// Predict the next byte from a wave trace before feedback is applied.
    #[must_use]
    pub fn predict(&self, trace: &TickTrace) -> LiveBytePrediction {
        let mut best_byte = 0u8;
        let mut best_score = f32::NEG_INFINITY;
        let mut second_score = f32::NEG_INFINITY;

        for byte in 0..=u8::MAX {
            let score = self.score_byte(trace, byte);
            if score > best_score {
                second_score = best_score;
                best_score = score;
                best_byte = byte;
            } else if score > second_score {
                second_score = score;
            }
        }

        let margin = best_score - second_score;
        LiveBytePrediction {
            predicted_byte: best_byte,
            confidence: sigmoid(margin).clamp(0.0, 1.0),
            score: best_score,
        }
    }

    /// Apply a perceptron-style local update using only active cells.
    pub fn update(&mut self, trace: &TickTrace, target_byte: u8) -> LiveByteTrainStep {
        self.observe_mode(trace.input_byte);
        let prediction = self.predict(trace);
        let correct = prediction.predicted_byte == target_byte;
        let target_score = self.score_byte(trace, target_byte);
        let margin = target_score - prediction.score;

        self.apply_decay();
        if !correct {
            let byte_class = byte_class(trace.input_byte);
            self.byte_bias[target_byte as usize] += self.learning_rate;
            self.byte_bias[prediction.predicted_byte as usize] -= self.learning_rate;
            self.class_byte_weights[byte_class][target_byte as usize] += self.learning_rate * 0.35;
            self.class_byte_weights[byte_class][prediction.predicted_byte as usize] -=
                self.learning_rate * 0.35;
            if self.mode_state == MODE_UTF8 {
                self.mode_byte_weights[self.mode_state][target_byte as usize] +=
                    self.learning_rate * 0.05;
                self.mode_byte_weights[self.mode_state][prediction.predicted_byte as usize] -=
                    self.learning_rate * 0.05;
            }
            self.transition_byte_weights[trace.input_byte as usize][target_byte as usize] +=
                self.learning_rate * 2.25;
            self.transition_byte_weights[trace.input_byte as usize]
                [prediction.predicted_byte as usize] -= self.learning_rate * 2.25;

            for (rank, cell_id) in trace
                .active_cell_ids
                .iter()
                .copied()
                .take(trace.active_count)
                .enumerate()
            {
                let cell_index = cell_id as usize;
                if cell_index >= STAGE2_ORGAN_CELLS {
                    continue;
                }
                let gain = self.learning_rate * active_rank_gain(rank) * trace.coherence.max(0.05);
                self.cell_byte_weights[cell_index][target_byte as usize] += gain;
                self.cell_byte_weights[cell_index][prediction.predicted_byte as usize] -= gain;

                let class_gain = gain * 0.35;
                let class_row = class_cell_row(cell_index, byte_class);
                self.class_cell_byte_weights[class_row][target_byte as usize] += class_gain;
                self.class_cell_byte_weights[class_row][prediction.predicted_byte as usize] -=
                    class_gain;

                if self.mode_state == MODE_UTF8 {
                    let mode_gain = gain * 0.05;
                    let mode_row = mode_cell_row(cell_index, self.mode_state);
                    self.mode_cell_byte_weights[mode_row][target_byte as usize] += mode_gain;
                    self.mode_cell_byte_weights[mode_row][prediction.predicted_byte as usize] -=
                        mode_gain;
                }

                let context_gain = gain * 1.75;
                let context_row = input_cell_row(cell_index, trace.input_byte);
                self.input_cell_byte_weights[context_row][target_byte as usize] += context_gain;
                self.input_cell_byte_weights[context_row][prediction.predicted_byte as usize] -=
                    context_gain;
            }
        }

        LiveByteTrainStep {
            prediction,
            target_byte,
            correct,
            margin,
        }
    }

    /// Return a compact measurement of trainable state magnitude.
    #[must_use]
    pub fn state_energy(&self) -> (f32, f32, f32, f32, f32, f32) {
        let bias_abs_sum: f32 = self.byte_bias.iter().map(|value| value.abs()).sum();
        let class_weight_abs_sum: f32 = self
            .class_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let mode_weight_abs_sum: f32 = self
            .mode_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .chain(
                self.mode_cell_byte_weights
                    .iter()
                    .flat_map(|row| row.iter()),
            )
            .map(|value| value.abs())
            .sum();
        let transition_weight_abs_sum: f32 = self
            .transition_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let weight_abs_sum: f32 = self
            .cell_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let context_weight_abs_sum: f32 = self
            .input_cell_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();

        (
            bias_abs_sum / self.byte_bias.len() as f32,
            class_weight_abs_sum / (BYTE_CLASSES * 256) as f32,
            mode_weight_abs_sum / ((MODE_STATES + MODE_CELL_ROWS) * 256) as f32,
            transition_weight_abs_sum / (256 * 256) as f32,
            weight_abs_sum / (STAGE2_ORGAN_CELLS * 256) as f32,
            context_weight_abs_sum / (INPUT_CELL_ROWS * 256) as f32,
        )
    }

    fn score_byte(&self, trace: &TickTrace, byte: u8) -> f32 {
        let byte_index = byte as usize;
        let mut score = self.byte_bias[byte_index];
        score += phase_affinity(trace.center_phase, byte) * trace.coherence * 0.20;
        score += structural_mode_prior(self.mode_state, trace.input_byte, byte);
        let byte_class = byte_class(trace.input_byte);
        score += self.class_byte_weights[byte_class][byte_index] * 0.20;
        score += self.transition_byte_weights[trace.input_byte as usize][byte_index] * 1.00;
        if self.mode_state == MODE_UTF8 {
            score += self.mode_byte_weights[self.mode_state][byte_index] * 0.05;
        }

        for (rank, cell_id) in trace
            .active_cell_ids
            .iter()
            .copied()
            .take(trace.active_count)
            .enumerate()
        {
            let cell_index = cell_id as usize;
            if cell_index >= STAGE2_ORGAN_CELLS {
                continue;
            }
            score += self.cell_byte_weights[cell_index][byte_index] * active_rank_gain(rank);
            let class_row = class_cell_row(cell_index, byte_class);
            score +=
                self.class_cell_byte_weights[class_row][byte_index] * active_rank_gain(rank) * 0.20;
            if self.mode_state == MODE_UTF8 {
                let mode_row = mode_cell_row(cell_index, self.mode_state);
                score += self.mode_cell_byte_weights[mode_row][byte_index]
                    * active_rank_gain(rank)
                    * 0.05;
            }
            let context_row = input_cell_row(cell_index, trace.input_byte);
            score += self.input_cell_byte_weights[context_row][byte_index] * active_rank_gain(rank);
        }

        score
    }

    fn observe_mode(&mut self, byte: u8) {
        self.utf8_cooldown = self.utf8_cooldown.saturating_sub(1);
        self.code_cooldown = self.code_cooldown.saturating_sub(1);
        self.layout_cooldown = self.layout_cooldown.saturating_sub(1);

        match byte_class(byte) {
            3 | 4 => self.code_cooldown = 4,
            5..=7 => self.utf8_cooldown = 1,
            _ => {}
        }

        if is_ascii_consonant(byte) {
            self.consonant_run = self.consonant_run.saturating_add(1).min(8);
        } else if byte != b'\'' && byte != b'-' {
            self.consonant_run = 0;
        }
        if self.consonant_run >= 4 {
            self.layout_cooldown = 5;
        }

        self.mode_state = if self.utf8_cooldown > 0 {
            MODE_UTF8
        } else if self.code_cooldown > 0 {
            MODE_CODE
        } else if self.layout_cooldown > 0 {
            MODE_LAYOUT_NOISE
        } else {
            MODE_ASCII
        };
    }

    fn apply_decay(&mut self) {
        for value in &mut self.byte_bias {
            *value *= self.decay;
        }
        for row in &mut self.class_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.cell_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.class_cell_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.mode_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.mode_cell_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.transition_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
        for row in &mut self.input_cell_byte_weights {
            for value in row {
                *value *= self.decay;
            }
        }
    }
}

impl LiveByteTrainReport {
    /// Build an aggregate report from online train steps.
    #[must_use]
    pub fn from_steps(steps: &[LiveByteTrainStep], learner: &LiveByteLearner) -> Self {
        if steps.is_empty() {
            return Self {
                cases: 0,
                correct_before_update: 0,
                accuracy_before_update: 0.0,
                mean_confidence: 0.0,
                mean_margin: 0.0,
                bias_abs_mean: 0.0,
                class_weight_abs_mean: 0.0,
                mode_weight_abs_mean: 0.0,
                transition_weight_abs_mean: 0.0,
                weight_abs_mean: 0.0,
                context_weight_abs_mean: 0.0,
            };
        }

        let correct_before_update = steps.iter().filter(|step| step.correct).count();
        let mean_confidence = steps
            .iter()
            .map(|step| step.prediction.confidence)
            .sum::<f32>()
            / steps.len() as f32;
        let mean_margin = steps.iter().map(|step| step.margin).sum::<f32>() / steps.len() as f32;
        let (
            bias_abs_mean,
            class_weight_abs_mean,
            mode_weight_abs_mean,
            transition_weight_abs_mean,
            weight_abs_mean,
            context_weight_abs_mean,
        ) = learner.state_energy();

        Self {
            cases: steps.len(),
            correct_before_update,
            accuracy_before_update: correct_before_update as f32 / steps.len() as f32,
            mean_confidence,
            mean_margin,
            bias_abs_mean,
            class_weight_abs_mean,
            mode_weight_abs_mean,
            transition_weight_abs_mean,
            weight_abs_mean,
            context_weight_abs_mean,
        }
    }
}

#[must_use]
fn input_cell_row(cell_index: usize, input_byte: u8) -> usize {
    cell_index * 256 + input_byte as usize
}

#[must_use]
fn class_cell_row(cell_index: usize, byte_class: usize) -> usize {
    cell_index * BYTE_CLASSES + byte_class
}

#[must_use]
fn mode_cell_row(cell_index: usize, mode_state: usize) -> usize {
    cell_index * MODE_STATES + mode_state
}

#[must_use]
fn active_cells(trace: &TickTrace, cells_enabled: usize) -> Vec<(usize, f32)> {
    trace
        .active_cell_ids
        .iter()
        .copied()
        .take(trace.active_count)
        .enumerate()
        .filter_map(|(rank, cell_id)| {
            let cell_index = cell_id as usize;
            (cell_index < cells_enabled).then_some((cell_index, active_rank_gain(rank)))
        })
        .collect()
}

#[must_use]
fn pair_row(a: usize, b: usize) -> usize {
    let (a, b) = sorted_pair(a, b);
    let mut row = 0usize;
    for left in 0..a {
        row += STAGE2_ORGAN_CELLS - left - 1;
    }
    row + (b - a - 1)
}

#[must_use]
fn triple_row(a: usize, b: usize, c: usize) -> usize {
    let (a, b, c) = sorted_triple(a, b, c);
    let mut row = 0usize;
    for left in 0..a {
        let remaining = STAGE2_ORGAN_CELLS - left - 1;
        row += remaining * (remaining - 1) / 2;
    }
    for mid in a + 1..b {
        row += STAGE2_ORGAN_CELLS - mid - 1;
    }
    row + (c - b - 1)
}

#[must_use]
fn pair_enabled_mask(profile: LinkProfile) -> [bool; PAIR_LINK_ROWS] {
    let mut enabled = [false; PAIR_LINK_ROWS];
    for left in 0..STAGE2_ORGAN_CELLS {
        for right in left + 1..STAGE2_ORGAN_CELLS {
            enabled[pair_row(left, right)] = match profile {
                LinkProfile::All => true,
                LinkProfile::Typed => typed_pair_allowed(left, right),
            };
        }
    }
    enabled
}

#[must_use]
fn triple_enabled_mask(profile: LinkProfile) -> [bool; TRIPLE_LINK_ROWS] {
    let mut enabled = [false; TRIPLE_LINK_ROWS];
    for left in 0..STAGE2_ORGAN_CELLS {
        for mid in left + 1..STAGE2_ORGAN_CELLS {
            for right in mid + 1..STAGE2_ORGAN_CELLS {
                enabled[triple_row(left, mid, right)] = match profile {
                    LinkProfile::All => true,
                    LinkProfile::Typed => typed_triple_allowed(left, mid, right),
                };
            }
        }
    }
    enabled
}

#[must_use]
fn sorted_pair(a: usize, b: usize) -> (usize, usize) {
    if a < b { (a, b) } else { (b, a) }
}

#[must_use]
fn sorted_triple(a: usize, b: usize, c: usize) -> (usize, usize, usize) {
    let mut values = [a, b, c];
    values.sort_unstable();
    (values[0], values[1], values[2])
}

#[must_use]
fn typed_pair_allowed(a: usize, b: usize) -> bool {
    matches!(
        (cell_role(a), cell_role(b)),
        (CellRole::Fast, CellRole::Fast)
            | (CellRole::Mid, CellRole::Mid)
            | (CellRole::Fast, CellRole::Mid)
            | (CellRole::Mid, CellRole::Fast)
            | (CellRole::Fast, CellRole::Carrier)
            | (CellRole::Carrier, CellRole::Fast)
            | (CellRole::Fast, CellRole::Guard)
            | (CellRole::Guard, CellRole::Fast)
            | (CellRole::Mid, CellRole::Carrier)
            | (CellRole::Carrier, CellRole::Mid)
            | (CellRole::Mid, CellRole::Guard)
            | (CellRole::Guard, CellRole::Mid)
            | (CellRole::Carrier, CellRole::Guard)
            | (CellRole::Guard, CellRole::Carrier)
    )
}

#[must_use]
fn typed_triple_allowed(a: usize, b: usize, c: usize) -> bool {
    let roles = [cell_role(a), cell_role(b), cell_role(c)];
    let has_fast = roles.contains(&CellRole::Fast);
    let has_mid = roles.contains(&CellRole::Mid);
    let has_context = roles.contains(&CellRole::Carrier) || roles.contains(&CellRole::Guard);
    let local_sheet = roles.iter().filter(|role| **role == CellRole::Fast).count() >= 2
        || roles.iter().filter(|role| **role == CellRole::Mid).count() >= 2;

    has_fast && has_mid && (has_context || local_sheet)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellRole {
    Fast,
    Mid,
    Carrier,
    Guard,
}

#[must_use]
fn cell_role(cell: usize) -> CellRole {
    match cell {
        0 | 1 => CellRole::Fast,
        2 | 3 => CellRole::Mid,
        4 => CellRole::Carrier,
        _ => CellRole::Guard,
    }
}

#[must_use]
fn byte_class(byte: u8) -> usize {
    match byte {
        b'a'..=b'z' | b'A'..=b'Z' | b'_' => 0,
        b'0'..=b'9' => 1,
        b' ' | b'\n' | b'\t' => 2,
        b'(' | b')' | b'{' | b'}' | b'[' | b']' => 3,
        b';' | b',' | b'.' | b':' | b'+' | b'-' | b'=' | b'*' | b'/' => 4,
        0xC0..=0xDF => 5,
        0xE0..=0xEF => 6,
        0x80..=0xBF => 7,
        _ => 0,
    }
}

#[must_use]
fn structural_mode_prior(mode_state: usize, input_byte: u8, candidate: u8) -> f32 {
    let input_class = byte_class(input_byte);
    let candidate_class = byte_class(candidate);

    match mode_state {
        MODE_UTF8 if matches!(input_class, 5 | 6) && candidate_class == 7 => 0.65,
        _ => 0.0,
    }
}

#[must_use]
fn is_ascii_consonant(byte: u8) -> bool {
    matches!(
        byte,
        b'b' | b'c'
            | b'd'
            | b'f'
            | b'g'
            | b'h'
            | b'j'
            | b'k'
            | b'l'
            | b'm'
            | b'n'
            | b'p'
            | b'q'
            | b'r'
            | b's'
            | b't'
            | b'v'
            | b'w'
            | b'x'
            | b'z'
            | b'B'
            | b'C'
            | b'D'
            | b'F'
            | b'G'
            | b'H'
            | b'J'
            | b'K'
            | b'L'
            | b'M'
            | b'N'
            | b'P'
            | b'Q'
            | b'R'
            | b'S'
            | b'T'
            | b'V'
            | b'W'
            | b'X'
            | b'Z'
    )
}

#[must_use]
fn active_rank_gain(rank: usize) -> f32 {
    (STAGE2_TOP_K.saturating_sub(rank) as f32 / STAGE2_TOP_K as f32).max(0.0)
}

#[must_use]
fn phase_affinity(center_phase: f32, byte: u8) -> f32 {
    let byte_phase = TAU * byte as f32 / 256.0;
    let delta = circular_phase_delta(center_phase, byte_phase);
    (1.0 - delta.abs() / PI).clamp(-1.0, 1.0)
}

#[must_use]
fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_tissue_updates_pair_and_triple_state() {
        let trace = TickTrace {
            seed: 7,
            input_byte: b'a',
            cells_scanned: STAGE2_ORGAN_CELLS,
            active_count: STAGE2_TOP_K,
            active_cell_ids: [0, 1, 2],
            top_resonance: 0.5,
            coherence: 0.75,
            spectral_entropy: 0.25,
            center_phase: 1.0,
            center_magnitude: 0.5,
        };
        let mut tissue = LinkTissue::new(6, true, 0.08);

        tissue.update_from_prediction(&trace, b'b', b'a');

        assert!(tissue.state_abs_mean() > 0.0);
        assert_ne!(tissue.score(&trace, b'a'), tissue.score(&trace, b'b'));
    }

    #[test]
    fn link_rows_are_unique_for_stage2_cells() {
        let mut pair_seen = [false; PAIR_LINK_ROWS];
        for a in 0..STAGE2_ORGAN_CELLS {
            for b in a + 1..STAGE2_ORGAN_CELLS {
                let row = pair_row(a, b);
                assert!(!pair_seen[row]);
                pair_seen[row] = true;
            }
        }
        assert!(pair_seen.into_iter().all(|seen| seen));

        let mut triple_seen = [false; TRIPLE_LINK_ROWS];
        for a in 0..STAGE2_ORGAN_CELLS {
            for b in a + 1..STAGE2_ORGAN_CELLS {
                for c in b + 1..STAGE2_ORGAN_CELLS {
                    let row = triple_row(a, b, c);
                    assert!(!triple_seen[row]);
                    triple_seen[row] = true;
                }
            }
        }
        assert!(triple_seen.into_iter().all(|seen| seen));
    }

    #[test]
    fn typed_link_tissue_filters_untyped_links() {
        let trace = TickTrace {
            seed: 7,
            input_byte: b'a',
            cells_scanned: STAGE2_ORGAN_CELLS,
            active_count: STAGE2_TOP_K,
            active_cell_ids: [0, 1, 2],
            top_resonance: 0.5,
            coherence: 0.75,
            spectral_entropy: 0.25,
            center_phase: 1.0,
            center_magnitude: 0.5,
        };
        let mut typed = LinkTissue::with_profile(6, true, 0.08, LinkProfile::Typed);

        typed.update_from_prediction(&trace, b'b', b'a');

        assert!(typed.pair_state_abs_mean((0, 1)) > 0.0);
        assert!(typed.pair_state_abs_mean((0, 2)) > 0.0);
        assert!(typed.triple_state_abs_mean((0, 1, 2)) > 0.0);
        assert_eq!(typed.pair_state_abs_mean((4, 5)), 0.0);
    }

    #[test]
    fn link_tissue_decay_and_pair_boost_are_local() {
        let trace = TickTrace {
            seed: 7,
            input_byte: b'a',
            cells_scanned: STAGE2_ORGAN_CELLS,
            active_count: STAGE2_TOP_K,
            active_cell_ids: [0, 1, 2],
            top_resonance: 0.5,
            coherence: 0.75,
            spectral_entropy: 0.25,
            center_phase: 1.0,
            center_magnitude: 0.5,
        };
        let mut tissue = LinkTissue::with_profile(6, false, 0.08, LinkProfile::Typed);

        tissue.update_pair_from_prediction(&trace, b'b', b'a', (0, 2), 1.0);
        let before_decay = tissue.pair_state_abs_mean((0, 2));
        assert!(before_decay > 0.0);
        assert_eq!(tissue.pair_state_abs_mean((3, 5)), 0.0);

        tissue.apply_decay(0.5);
        let after_decay = tissue.pair_state_abs_mean((0, 2));
        assert!(after_decay > 0.0);
        assert!(after_decay < before_decay);
    }
}
