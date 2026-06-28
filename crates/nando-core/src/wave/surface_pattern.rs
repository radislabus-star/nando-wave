//! Proof wrapper for L1 surface pattern learning.
//!
//! This layer does not add semantics. It checks whether a SurfaceWave language
//! model learned a reusable surface operator instead of acting like an exact
//! string lookup.

use std::collections::HashSet;

use super::{SURFACE_WAVE_BYTES, SurfaceWaveGeneration, SurfaceWaveLm, SurfaceWaveLmConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceWavePatternVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceWavePatternProofConfig {
    pub lm_config: SurfaceWaveLmConfig,
    pub min_heldout_accuracy: f32,
    pub max_model_to_naive_ratio: f32,
    pub require_no_exact_lookup_overlap: bool,
}

impl Default for SurfaceWavePatternProofConfig {
    fn default() -> Self {
        Self {
            lm_config: SurfaceWaveLmConfig::default(),
            min_heldout_accuracy: 0.82,
            max_model_to_naive_ratio: 0.10,
            require_no_exact_lookup_overlap: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWaveGenerationCase {
    pub prefix: String,
    pub max_new_bytes: usize,
    pub required_contains: String,
}

impl SurfaceWaveGenerationCase {
    #[must_use]
    pub fn new(
        prefix: impl Into<String>,
        max_new_bytes: usize,
        required_contains: impl Into<String>,
    ) -> Self {
        Self {
            prefix: prefix.into(),
            max_new_bytes,
            required_contains: required_contains.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWaveGenerationProof {
    pub prefix: String,
    pub generated: String,
    pub required_contains: String,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceWavePatternProof {
    pub verdict: SurfaceWavePatternVerdict,
    pub train_sequences: usize,
    pub heldout_sequences: usize,
    pub train_steps: usize,
    pub corrections: usize,
    pub alphabet_len: usize,
    pub heldout_steps: usize,
    pub heldout_correct_predictions: usize,
    pub heldout_accuracy: f32,
    pub naive_train_wave_bytes: usize,
    pub naive_total_wave_bytes: usize,
    pub model_hot_bytes: usize,
    pub model_to_naive_total_ratio: f32,
    pub exact_lookup_heldout_hits: usize,
    pub exact_lookup_heldout_coverage: f32,
    pub compression_pass: bool,
    pub heldout_pass: bool,
    pub anti_lookup_pass: bool,
    pub generation_pass: bool,
    pub generations: Vec<SurfaceWaveGenerationProof>,
}

impl SurfaceWavePatternProof {
    #[must_use]
    pub fn prove<'a, I, J>(
        train: I,
        heldout: J,
        generation_cases: &[SurfaceWaveGenerationCase],
        config: SurfaceWavePatternProofConfig,
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
            .filter(|text| train_set.contains(text.as_str()))
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

        let compression_pass = model_to_naive_total_ratio <= config.max_model_to_naive_ratio;
        let heldout_pass = heldout_report.accuracy >= config.min_heldout_accuracy;
        let anti_lookup_pass =
            !config.require_no_exact_lookup_overlap || exact_lookup_heldout_hits == 0;
        let generation_pass = generations.iter().all(|generation| generation.passed);
        let verdict = if compression_pass && heldout_pass && anti_lookup_pass && generation_pass {
            SurfaceWavePatternVerdict::Proven
        } else {
            SurfaceWavePatternVerdict::Watch
        };

        Self {
            verdict,
            train_sequences: train_report.train_sequences,
            heldout_sequences: heldout_report.eval_sequences,
            train_steps: train_report.train_steps,
            corrections: train_report.corrections,
            alphabet_len: train_report.alphabet_len,
            heldout_steps: heldout_report.eval_steps,
            heldout_correct_predictions: heldout_report.correct_predictions,
            heldout_accuracy: heldout_report.accuracy,
            naive_train_wave_bytes,
            naive_total_wave_bytes,
            model_hot_bytes,
            model_to_naive_total_ratio,
            exact_lookup_heldout_hits,
            exact_lookup_heldout_coverage,
            compression_pass,
            heldout_pass,
            anti_lookup_pass,
            generation_pass,
            generations,
        }
    }
}

fn generation_proof(
    lm: &SurfaceWaveLm,
    case: &SurfaceWaveGenerationCase,
) -> SurfaceWaveGenerationProof {
    let generated = lm.generate(&case.prefix, case.max_new_bytes);
    generation_proof_from_generation(case, generated)
}

fn generation_proof_from_generation(
    case: &SurfaceWaveGenerationCase,
    generated: SurfaceWaveGeneration,
) -> SurfaceWaveGenerationProof {
    let passed = generated.generated.contains(&case.required_contains);
    SurfaceWaveGenerationProof {
        prefix: case.prefix.clone(),
        generated: generated.generated,
        required_contains: case.required_contains.clone(),
        passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::SurfaceWaveLmEvalReport;

    fn topic_url(index: usize) -> String {
        format!("https://mirror.dxdy.ru/topic{index:04}.html")
    }

    fn russian_chatter(index: usize) -> String {
        const TOPICS: &[&str] = &[
            "волна",
            "память",
            "кэш",
            "корпус",
            "фаза",
            "атом",
            "маршрут",
            "гейт",
        ];
        const STATES: &[&str] = &[
            "работает",
            "шумит",
            "держится",
            "проверяется",
            "не спешит",
            "собирается",
        ];

        let topic = TOPICS[index % TOPICS.len()];
        let state = STATES[(index / TOPICS.len()) % STATES.len()];
        format!(
            "болтанка {index:04}: сегодня {topic} {state}; завтра {topic} снова {state}. вопрос {index:04}: что дальше?\n"
        )
    }

    fn noisy_token(index: usize) -> String {
        let mut state = 0xD1B5_4A32_D192_ED03u64 ^ index as u64;
        let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
        let mut text = String::with_capacity(48);
        for _ in 0..48 {
            state = splitmix64(state);
            text.push(alphabet[(state as usize) % alphabet.len()] as char);
        }
        text
    }

    #[test]
    fn surface_pattern_proof_requires_compression_heldout_and_non_lookup() {
        let train: Vec<_> = (0..8_000).map(topic_url).collect();
        let heldout: Vec<_> = (8_000..10_000).map(topic_url).collect();
        let proof = SurfaceWavePatternProof::prove(
            train.iter().map(String::as_str),
            heldout.iter().map(String::as_str),
            &[SurfaceWaveGenerationCase::new(
                "https://mirror.dxdy.ru/topic8",
                12,
                ".html",
            )],
            SurfaceWavePatternProofConfig::default(),
        );

        assert_eq!(
            proof.verdict,
            SurfaceWavePatternVerdict::Proven,
            "proof={proof:?}"
        );
        assert_eq!(proof.train_sequences, 8_000);
        assert_eq!(proof.heldout_sequences, 2_000);
        assert!(proof.compression_pass);
        assert!(proof.heldout_pass);
        assert!(proof.anti_lookup_pass);
        assert!(proof.generation_pass);
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert!(proof.model_to_naive_total_ratio < 0.03, "proof={proof:?}");
        assert!(proof.heldout_accuracy > 0.82, "proof={proof:?}");
    }

    #[test]
    fn russian_chatter_surface_pattern_grokks_form_without_exact_lookup() {
        let train: Vec<_> = (0..800).map(russian_chatter).collect();
        let heldout: Vec<_> = (800..1_000).map(russian_chatter).collect();
        let proof = SurfaceWavePatternProof::prove(
            train.iter().map(String::as_str),
            heldout.iter().map(String::as_str),
            &[SurfaceWaveGenerationCase::new("болтанка 0", 96, "сегодня")],
            SurfaceWavePatternProofConfig {
                lm_config: SurfaceWaveLmConfig {
                    context_ngrams: 16,
                    epochs: 2,
                    learning_rate: 1,
                },
                min_heldout_accuracy: 0.64,
                max_model_to_naive_ratio: 0.30,
                require_no_exact_lookup_overlap: true,
            },
        );

        assert_eq!(
            proof.verdict,
            SurfaceWavePatternVerdict::Proven,
            "proof={proof:?}"
        );
        assert_eq!(proof.train_sequences, 800);
        assert_eq!(proof.heldout_sequences, 200);
        assert!(proof.alphabet_len > 40, "proof={proof:?}");
        assert!(proof.compression_pass, "proof={proof:?}");
        assert!(proof.heldout_pass, "proof={proof:?}");
        assert!(proof.anti_lookup_pass, "proof={proof:?}");
        assert!(proof.generation_pass, "proof={proof:?}");
        assert_eq!(proof.exact_lookup_heldout_hits, 0);
        assert!(proof.heldout_accuracy > 0.64, "proof={proof:?}");
    }

    #[test]
    fn random_surface_noise_does_not_get_false_grokking_score() {
        let train: Vec<_> = (0..384).map(noisy_token).collect();
        let heldout: Vec<_> = (384..512).map(noisy_token).collect();
        let mut lm = SurfaceWaveLm::new(SurfaceWaveLmConfig {
            context_ngrams: 12,
            epochs: 2,
            learning_rate: 1,
        });
        lm.train(train.iter().map(String::as_str));
        let SurfaceWaveLmEvalReport { accuracy, .. } =
            lm.evaluate(heldout.iter().map(String::as_str));

        assert!(
            accuracy < 0.20,
            "noise accuracy should stay low: {accuracy}"
        );
    }

    fn splitmix64(mut value: u64) -> u64 {
        value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}
