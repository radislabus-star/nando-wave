//! L1 SurfaceWave language model.
//!
//! This is the first generator above surface wire. It does not store meaning:
//! it learns a transition operator from a clamped n-gram wave state to the next
//! byte. Repeated surface form can be generated; one-off literals still need a
//! separate residual layer.

use super::{
    SURFACE_WAVE_DIM, SURFACE_WAVE_NGRAM, SURFACE_WAVE_TRITS, SurfaceWaveTrit,
    surface_ngram_projection,
};

pub const SURFACE_WAVE_LM_OUTPUTS: usize = 256;
pub const SURFACE_WAVE_LM_POSITION_BUCKETS: usize = 128;
pub const SURFACE_WAVE_LM_STATE_MIN: i8 = -3;
pub const SURFACE_WAVE_LM_STATE_MAX: i8 = 3;
pub const SURFACE_WAVE_LM_WEIGHT_BYTES: usize =
    SURFACE_WAVE_DIM * SURFACE_WAVE_LM_OUTPUTS * std::mem::size_of::<i16>();
pub const SURFACE_WAVE_LM_BIAS_BYTES: usize = SURFACE_WAVE_LM_OUTPUTS * std::mem::size_of::<i16>();
pub const SURFACE_WAVE_LM_POSITION_BYTES: usize =
    SURFACE_WAVE_LM_OUTPUTS * SURFACE_WAVE_LM_POSITION_BUCKETS * std::mem::size_of::<i16>();
pub const SURFACE_WAVE_LM_POSITION_SCORE_WEIGHT: i32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceWaveLmConfig {
    pub context_ngrams: usize,
    pub epochs: usize,
    pub learning_rate: i16,
}

impl Default for SurfaceWaveLmConfig {
    fn default() -> Self {
        Self {
            context_ngrams: 12,
            epochs: 2,
            learning_rate: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWaveLmTrainReport {
    pub train_sequences: usize,
    pub train_steps: usize,
    pub corrections: usize,
    pub alphabet_len: usize,
    pub hot_weight_bytes: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWaveLmEvalReport {
    pub eval_sequences: usize,
    pub eval_steps: usize,
    pub correct_predictions: usize,
    pub accuracy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWaveTextScore {
    pub steps: usize,
    pub total_score: i64,
    pub average_score: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWaveGeneration {
    pub prefix: String,
    pub generated: String,
    pub generated_bytes: Vec<u8>,
    pub steps: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWaveContext4096 {
    context_ngrams: usize,
    lanes: [i8; SURFACE_WAVE_DIM],
    known_lanes: Vec<u16>,
    lane_seen: [bool; SURFACE_WAVE_DIM],
    ngrams: Vec<[SurfaceWaveTrit; SURFACE_WAVE_TRITS]>,
}

impl SurfaceWaveContext4096 {
    #[must_use]
    pub fn new(context_ngrams: usize) -> Self {
        Self {
            context_ngrams,
            lanes: [0; SURFACE_WAVE_DIM],
            known_lanes: Vec::new(),
            lane_seen: [false; SURFACE_WAVE_DIM],
            ngrams: Vec::with_capacity(context_ngrams),
        }
    }

    #[must_use]
    pub fn from_prefix(prefix: &[u8], context_ngrams: usize) -> Self {
        let mut context = Self::new(context_ngrams);
        if prefix.len() < SURFACE_WAVE_NGRAM {
            return context;
        }

        for (position, gram) in prefix.windows(SURFACE_WAVE_NGRAM).enumerate() {
            context.push_ngram(position as u64, gram);
        }
        context
    }

    #[must_use]
    pub fn lane_value(&self, lane: usize) -> i8 {
        self.lanes[lane]
    }

    #[must_use]
    pub fn active_lanes(&self) -> Vec<(u16, i8)> {
        self.known_lanes
            .iter()
            .filter_map(|lane| {
                let value = self.lanes[usize::from(*lane)];
                (value != 0).then_some((*lane, value))
            })
            .collect()
    }

    #[must_use]
    pub fn active_lane_count(&self) -> usize {
        self.known_lanes
            .iter()
            .filter(|lane| self.lanes[usize::from(**lane)] != 0)
            .count()
    }

    pub fn push_observed_byte(&mut self, bytes: &[u8], observed_index: usize) {
        if observed_index + 1 < SURFACE_WAVE_NGRAM {
            return;
        }
        let start = observed_index + 1 - SURFACE_WAVE_NGRAM;
        self.push_ngram(start as u64, &bytes[start..start + SURFACE_WAVE_NGRAM]);
    }

    fn push_ngram(&mut self, position: u64, gram: &[u8]) {
        let projection = surface_ngram_projection(position, gram);
        self.apply_projection(&projection, 1);
        self.ngrams.push(projection);

        if self.context_ngrams > 0 && self.ngrams.len() > self.context_ngrams {
            let old = self.ngrams.remove(0);
            self.apply_projection(&old, -1);
        }
    }

    fn apply_projection(&mut self, projection: &[SurfaceWaveTrit; SURFACE_WAVE_TRITS], sign: i8) {
        for trit in projection {
            if trit.value == 0 {
                continue;
            }

            let lane = usize::from(trit.lane);
            if !self.lane_seen[lane] {
                self.lane_seen[lane] = true;
                self.known_lanes.push(trit.lane);
            }

            let delta = sign * trit.value;
            self.lanes[lane] = (self.lanes[lane] + delta)
                .clamp(SURFACE_WAVE_LM_STATE_MIN, SURFACE_WAVE_LM_STATE_MAX);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWaveLm {
    config: SurfaceWaveLmConfig,
    weights: Vec<i16>,
    bias: [i16; SURFACE_WAVE_LM_OUTPUTS],
    position_bias: Vec<i16>,
    alphabet: Vec<u8>,
    alphabet_seen: [bool; SURFACE_WAVE_LM_OUTPUTS],
}

impl SurfaceWaveLm {
    #[must_use]
    pub fn new(config: SurfaceWaveLmConfig) -> Self {
        Self {
            config,
            weights: vec![0; SURFACE_WAVE_DIM * SURFACE_WAVE_LM_OUTPUTS],
            bias: [0; SURFACE_WAVE_LM_OUTPUTS],
            position_bias: vec![0; SURFACE_WAVE_LM_OUTPUTS * SURFACE_WAVE_LM_POSITION_BUCKETS],
            alphabet: Vec::new(),
            alphabet_seen: [false; SURFACE_WAVE_LM_OUTPUTS],
        }
    }

    #[must_use]
    pub fn config(&self) -> SurfaceWaveLmConfig {
        self.config
    }

    #[must_use]
    pub fn alphabet(&self) -> &[u8] {
        &self.alphabet
    }

    #[must_use]
    pub fn hot_weight_bytes(&self) -> usize {
        SURFACE_WAVE_LM_WEIGHT_BYTES + SURFACE_WAVE_LM_BIAS_BYTES + SURFACE_WAVE_LM_POSITION_BYTES
    }

    pub fn train<'a, I>(&mut self, texts: I) -> SurfaceWaveLmTrainReport
    where
        I: IntoIterator<Item = &'a str>,
    {
        let texts: Vec<Vec<u8>> = texts
            .into_iter()
            .map(|text| text.as_bytes().to_vec())
            .collect();
        for bytes in &texts {
            self.observe_alphabet(bytes);
        }
        self.alphabet.sort_unstable();

        let mut report = SurfaceWaveLmTrainReport {
            train_sequences: texts.len(),
            train_steps: 0,
            corrections: 0,
            alphabet_len: self.alphabet.len(),
            hot_weight_bytes: self.hot_weight_bytes(),
        };

        for _ in 0..self.config.epochs {
            for bytes in &texts {
                self.train_bytes(bytes, &mut report);
            }
        }
        report
    }

    #[must_use]
    pub fn evaluate<'a, I>(&self, texts: I) -> SurfaceWaveLmEvalReport
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut report = SurfaceWaveLmEvalReport {
            eval_sequences: 0,
            eval_steps: 0,
            correct_predictions: 0,
            accuracy: 0.0,
        };

        for text in texts {
            report.eval_sequences += 1;
            let bytes = text.as_bytes();
            let mut context = SurfaceWaveContext4096::new(self.config.context_ngrams);
            for (target_index, expected) in bytes.iter().copied().enumerate() {
                if target_index >= SURFACE_WAVE_NGRAM {
                    report.eval_steps += 1;
                    if self.predict_byte(&context, target_index) == Some(expected) {
                        report.correct_predictions += 1;
                    }
                }
                context.push_observed_byte(bytes, target_index);
            }
        }

        if report.eval_steps > 0 {
            report.accuracy = report.correct_predictions as f32 / report.eval_steps as f32;
        }
        report
    }

    #[must_use]
    pub fn score_text(&self, text: &str) -> SurfaceWaveTextScore {
        let bytes = text.as_bytes();
        let mut context = SurfaceWaveContext4096::new(self.config.context_ngrams);
        let mut steps = 0usize;
        let mut total_score = 0i64;

        for (target_index, expected) in bytes.iter().copied().enumerate() {
            if target_index >= SURFACE_WAVE_NGRAM {
                steps += 1;
                total_score += i64::from(self.score_byte(expected, &context, target_index));
            }
            context.push_observed_byte(bytes, target_index);
        }

        let average_score = if steps == 0 {
            0.0
        } else {
            total_score as f32 / steps as f32
        };

        SurfaceWaveTextScore {
            steps,
            total_score,
            average_score,
        }
    }

    #[must_use]
    pub fn predict_next_byte(&self, prefix: &str) -> Option<u8> {
        let context =
            SurfaceWaveContext4096::from_prefix(prefix.as_bytes(), self.config.context_ngrams);
        self.predict_byte(&context, prefix.len())
    }

    #[must_use]
    pub fn generate(&self, prefix: &str, max_new_bytes: usize) -> SurfaceWaveGeneration {
        let mut bytes = prefix.as_bytes().to_vec();
        let mut context = SurfaceWaveContext4096::from_prefix(&bytes, self.config.context_ngrams);
        let mut generated_bytes = Vec::with_capacity(max_new_bytes);

        for _ in 0..max_new_bytes {
            let Some(next) = self.predict_byte(&context, bytes.len()) else {
                break;
            };
            bytes.push(next);
            generated_bytes.push(next);
            context.push_observed_byte(&bytes, bytes.len() - 1);
        }

        SurfaceWaveGeneration {
            prefix: prefix.to_string(),
            generated: String::from_utf8_lossy(&generated_bytes).into_owned(),
            steps: generated_bytes.len(),
            generated_bytes,
        }
    }

    fn observe_alphabet(&mut self, bytes: &[u8]) {
        for byte in bytes {
            let index = usize::from(*byte);
            if !self.alphabet_seen[index] {
                self.alphabet_seen[index] = true;
                self.alphabet.push(*byte);
            }
        }
    }

    fn train_bytes(&mut self, bytes: &[u8], report: &mut SurfaceWaveLmTrainReport) {
        let mut context = SurfaceWaveContext4096::new(self.config.context_ngrams);
        for (target_index, expected) in bytes.iter().copied().enumerate() {
            if target_index >= SURFACE_WAVE_NGRAM {
                report.train_steps += 1;
                let predicted = self
                    .predict_byte(&context, target_index)
                    .unwrap_or(expected);
                if predicted != expected {
                    report.corrections += 1;
                    self.reinforce(&context, target_index, expected, predicted);
                }
            }
            context.push_observed_byte(bytes, target_index);
        }
    }

    fn predict_byte(&self, context: &SurfaceWaveContext4096, target_index: usize) -> Option<u8> {
        let mut best = None;
        let mut best_score = i32::MIN;

        for byte in &self.alphabet {
            let score = self.score_byte(*byte, context, target_index);
            if score > best_score {
                best_score = score;
                best = Some(*byte);
            }
        }
        best
    }

    fn score_byte(&self, byte: u8, context: &SurfaceWaveContext4096, target_index: usize) -> i32 {
        let mut score = i32::from(self.bias[usize::from(byte)]);
        score += SURFACE_WAVE_LM_POSITION_SCORE_WEIGHT
            * i32::from(
                self.position_bias[usize::from(byte) * SURFACE_WAVE_LM_POSITION_BUCKETS
                    + position_bucket(target_index)],
            );
        let row_start = usize::from(byte) * SURFACE_WAVE_DIM;
        for lane in &context.known_lanes {
            let value = context.lanes[usize::from(*lane)];
            if value == 0 {
                continue;
            }
            score += i32::from(value) * i32::from(self.weights[row_start + usize::from(*lane)]);
        }
        score
    }

    fn reinforce(
        &mut self,
        context: &SurfaceWaveContext4096,
        target_index: usize,
        expected: u8,
        predicted: u8,
    ) {
        let expected_index = usize::from(expected);
        let predicted_index = usize::from(predicted);
        self.bias[expected_index] =
            self.bias[expected_index].saturating_add(self.config.learning_rate);
        self.bias[predicted_index] =
            self.bias[predicted_index].saturating_sub(self.config.learning_rate);
        let bucket = position_bucket(target_index);
        self.position_bias[expected_index * SURFACE_WAVE_LM_POSITION_BUCKETS + bucket] = self
            .position_bias[expected_index * SURFACE_WAVE_LM_POSITION_BUCKETS + bucket]
            .saturating_add(self.config.learning_rate);
        self.position_bias[predicted_index * SURFACE_WAVE_LM_POSITION_BUCKETS + bucket] = self
            .position_bias[predicted_index * SURFACE_WAVE_LM_POSITION_BUCKETS + bucket]
            .saturating_sub(self.config.learning_rate);

        let expected_row = expected_index * SURFACE_WAVE_DIM;
        let predicted_row = predicted_index * SURFACE_WAVE_DIM;
        for lane in &context.known_lanes {
            let value = context.lanes[usize::from(*lane)];
            if value == 0 {
                continue;
            }
            let delta = self.config.learning_rate.saturating_mul(i16::from(value));
            self.weights[expected_row + usize::from(*lane)] =
                self.weights[expected_row + usize::from(*lane)].saturating_add(delta);
            self.weights[predicted_row + usize::from(*lane)] =
                self.weights[predicted_row + usize::from(*lane)].saturating_sub(delta);
        }
    }
}

fn position_bucket(target_index: usize) -> usize {
    target_index % SURFACE_WAVE_LM_POSITION_BUCKETS
}

impl Default for SurfaceWaveLm {
    fn default() -> Self {
        Self::new(SurfaceWaveLmConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::SURFACE_WAVE_BYTES;

    fn topic_url(index: usize) -> String {
        format!("https://mirror.dxdy.ru/topic{index:04}.html")
    }

    #[test]
    fn context_state_is_clamped_to_seven_levels() {
        let mut context = SurfaceWaveContext4096::new(128);
        let bytes = b"https://mirror.dxdy.ru/topic3420.html";

        for _ in 0..8 {
            for (index, _) in bytes.iter().enumerate() {
                context.push_observed_byte(bytes, index);
            }
        }

        assert!(context.active_lane_count() > 0);
        for (_, value) in context.active_lanes() {
            assert!((SURFACE_WAVE_LM_STATE_MIN..=SURFACE_WAVE_LM_STATE_MAX).contains(&value));
        }
    }

    #[test]
    fn surface_wave_lm_learns_url_form_on_heldout_indexes() {
        let train: Vec<_> = (0..2_000).map(topic_url).collect();
        let heldout: Vec<_> = (2_000..2_500).map(topic_url).collect();

        let mut lm = SurfaceWaveLm::new(SurfaceWaveLmConfig {
            context_ngrams: 12,
            epochs: 2,
            learning_rate: 1,
        });
        let train_report = lm.train(train.iter().map(String::as_str));
        let eval_report = lm.evaluate(heldout.iter().map(String::as_str));
        let generated = lm.generate("https://mirror.dxdy.ru/topic8", 12);

        assert_eq!(train_report.train_sequences, 2_000);
        assert!(train_report.train_steps > 60_000);
        assert!(train_report.corrections > 0);
        assert!(train_report.alphabet_len > 10);
        assert!(train_report.hot_weight_bytes < 10_000 * SURFACE_WAVE_BYTES);

        assert_eq!(eval_report.eval_sequences, 500);
        assert!(
            eval_report.accuracy > 0.82,
            "heldout accuracy too low: {eval_report:?}"
        );

        assert_eq!(generated.prefix, "https://mirror.dxdy.ru/topic8");
        assert_eq!(generated.steps, 12);
        assert!(
            generated.generated.contains(".html"),
            "generated={}",
            generated.generated
        );
    }

    #[test]
    fn untrained_surface_wave_lm_has_no_magic_memory() {
        let lm = SurfaceWaveLm::default();

        assert_eq!(lm.predict_next_byte("https://mirror.dxdy.ru/topic"), None);
        assert_eq!(lm.generate("https://mirror.dxdy.ru/topic", 16).steps, 0);
    }
}
