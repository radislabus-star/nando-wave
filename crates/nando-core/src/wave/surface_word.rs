//! Surface-word grokking probe over real word lists.
//!
//! This is still L1 surface learning, not semantics. A pass means the wave LM
//! learned reusable word-form transitions on heldout words without exact lookup.

use std::collections::HashSet;

use super::{
    SURFACE_WAVE_BYTES, SurfaceWaveGenerationCase, SurfaceWaveGenerationProof, SurfaceWaveLm,
    SurfaceWaveLmConfig, SurfaceWaveLmEvalReport,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceWordGrokkingVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceWordGrokkingConfig {
    pub lm_config: SurfaceWaveLmConfig,
    pub min_heldout_accuracy: f32,
    pub min_real_vs_corrupt_margin: f32,
    pub max_model_to_naive_ratio: f32,
    pub max_corrupt_eval_words: usize,
    pub require_no_exact_lookup_overlap: bool,
}

impl Default for SurfaceWordGrokkingConfig {
    fn default() -> Self {
        Self {
            lm_config: SurfaceWaveLmConfig {
                context_ngrams: 10,
                epochs: 2,
                learning_rate: 1,
            },
            min_heldout_accuracy: 0.34,
            min_real_vs_corrupt_margin: 1.0,
            max_model_to_naive_ratio: 0.06,
            max_corrupt_eval_words: 2_048,
            require_no_exact_lookup_overlap: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWordGrokkingProof {
    pub verdict: SurfaceWordGrokkingVerdict,
    pub train_words: usize,
    pub heldout_words: usize,
    pub train_steps: usize,
    pub corrections: usize,
    pub alphabet_len: usize,
    pub heldout_steps: usize,
    pub heldout_correct_predictions: usize,
    pub heldout_accuracy: f32,
    pub random_baseline_accuracy: f32,
    pub lift_over_random: f32,
    pub corrupt_eval_words: usize,
    pub average_heldout_word_score: f32,
    pub average_corrupt_word_score: f32,
    pub average_real_vs_corrupt_margin: f32,
    pub naive_train_wave_bytes: usize,
    pub naive_total_wave_bytes: usize,
    pub model_hot_bytes: usize,
    pub model_to_naive_total_ratio: f32,
    pub exact_lookup_heldout_hits: usize,
    pub exact_lookup_heldout_coverage: f32,
    pub compression_pass: bool,
    pub heldout_pass: bool,
    pub corrupt_reject_pass: bool,
    pub anti_lookup_pass: bool,
    pub generations: Vec<SurfaceWaveGenerationProof>,
}

impl SurfaceWordGrokkingProof {
    #[must_use]
    pub fn prove<'a, I, J>(
        train: I,
        heldout: J,
        generation_cases: &[SurfaceWaveGenerationCase],
        config: SurfaceWordGrokkingConfig,
    ) -> Self
    where
        I: IntoIterator<Item = &'a str>,
        J: IntoIterator<Item = &'a str>,
    {
        let train: Vec<String> = train.into_iter().map(str::to_string).collect();
        let heldout: Vec<String> = heldout.into_iter().map(str::to_string).collect();

        let mut lm = SurfaceWaveLm::new(config.lm_config);
        let train_report = lm.train(train.iter().map(String::as_str));
        let heldout_report = lm.evaluate(heldout.iter().map(String::as_str));
        let generations = generation_cases
            .iter()
            .map(|case| generation_proof(&lm, case))
            .collect::<Vec<_>>();

        let train_set: HashSet<&str> = train.iter().map(String::as_str).collect();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|word| train_set.contains(word.as_str()))
            .count();
        let exact_lookup_heldout_coverage = if heldout.is_empty() {
            0.0
        } else {
            exact_lookup_heldout_hits as f32 / heldout.len() as f32
        };

        let naive_train_wave_bytes = train.len() * SURFACE_WAVE_BYTES;
        let naive_total_wave_bytes = (train.len() + heldout.len()) * SURFACE_WAVE_BYTES;
        let model_hot_bytes = train_report.hot_weight_bytes;
        let model_to_naive_total_ratio = if naive_total_wave_bytes == 0 {
            1.0
        } else {
            model_hot_bytes as f32 / naive_total_wave_bytes as f32
        };

        let random_baseline_accuracy =
            random_byte_baseline(&heldout_report, train_report.alphabet_len);
        let lift_over_random = if random_baseline_accuracy <= f32::EPSILON {
            heldout_report.accuracy
        } else {
            heldout_report.accuracy / random_baseline_accuracy
        };
        let corrupt_report =
            corrupt_word_score_report(&lm, &heldout, config.max_corrupt_eval_words);

        let compression_pass = model_to_naive_total_ratio <= config.max_model_to_naive_ratio;
        let heldout_pass = heldout_report.accuracy >= config.min_heldout_accuracy;
        let corrupt_reject_pass =
            corrupt_report.average_real_vs_corrupt_margin >= config.min_real_vs_corrupt_margin;
        let anti_lookup_pass =
            !config.require_no_exact_lookup_overlap || exact_lookup_heldout_hits == 0;
        let verdict = if compression_pass && heldout_pass && corrupt_reject_pass && anti_lookup_pass
        {
            SurfaceWordGrokkingVerdict::Proven
        } else {
            SurfaceWordGrokkingVerdict::Watch
        };

        Self {
            verdict,
            train_words: train_report.train_sequences,
            heldout_words: heldout_report.eval_sequences,
            train_steps: train_report.train_steps,
            corrections: train_report.corrections,
            alphabet_len: train_report.alphabet_len,
            heldout_steps: heldout_report.eval_steps,
            heldout_correct_predictions: heldout_report.correct_predictions,
            heldout_accuracy: heldout_report.accuracy,
            random_baseline_accuracy,
            lift_over_random,
            corrupt_eval_words: corrupt_report.words,
            average_heldout_word_score: corrupt_report.average_real_score,
            average_corrupt_word_score: corrupt_report.average_corrupt_score,
            average_real_vs_corrupt_margin: corrupt_report.average_real_vs_corrupt_margin,
            naive_train_wave_bytes,
            naive_total_wave_bytes,
            model_hot_bytes,
            model_to_naive_total_ratio,
            exact_lookup_heldout_hits,
            exact_lookup_heldout_coverage,
            compression_pass,
            heldout_pass,
            corrupt_reject_pass,
            anti_lookup_pass,
            generations,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CorruptWordScoreReport {
    words: usize,
    average_real_score: f32,
    average_corrupt_score: f32,
    average_real_vs_corrupt_margin: f32,
}

fn corrupt_word_score_report(
    lm: &SurfaceWaveLm,
    heldout: &[String],
    max_words: usize,
) -> CorruptWordScoreReport {
    let mut words = 0usize;
    let mut real_sum = 0.0f32;
    let mut corrupt_sum = 0.0f32;

    for word in heldout.iter().take(max_words) {
        let corrupt = reversed_chars(word);
        if corrupt == *word {
            continue;
        }
        words += 1;
        real_sum += lm.score_text(word).average_score;
        corrupt_sum += lm.score_text(&corrupt).average_score;
    }

    if words == 0 {
        return CorruptWordScoreReport {
            words,
            average_real_score: 0.0,
            average_corrupt_score: 0.0,
            average_real_vs_corrupt_margin: 0.0,
        };
    }

    let average_real_score = real_sum / words as f32;
    let average_corrupt_score = corrupt_sum / words as f32;
    CorruptWordScoreReport {
        words,
        average_real_score,
        average_corrupt_score,
        average_real_vs_corrupt_margin: average_real_score - average_corrupt_score,
    }
}

fn reversed_chars(word: &str) -> String {
    word.chars().rev().collect()
}

fn random_byte_baseline(report: &SurfaceWaveLmEvalReport, alphabet_len: usize) -> f32 {
    if report.eval_steps == 0 || alphabet_len == 0 {
        0.0
    } else {
        1.0 / alphabet_len as f32
    }
}

fn generation_proof(
    lm: &SurfaceWaveLm,
    case: &SurfaceWaveGenerationCase,
) -> SurfaceWaveGenerationProof {
    let generated = lm.generate(&case.prefix, case.max_new_bytes);
    let passed = generated.generated.contains(&case.required_contains);
    SurfaceWaveGenerationProof {
        prefix: case.prefix.clone(),
        generated: generated.generated,
        required_contains: case.required_contains.clone(),
        passed,
    }
}
