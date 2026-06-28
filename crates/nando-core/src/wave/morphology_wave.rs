//! MorphologyWave: productive surface-morphology atoms between words and meaning.
//!
//! This layer is not semantics. It discovers reusable ending/suffix atoms from a
//! training word list and checks whether they transfer to heldout words without
//! exact word lookup.

use std::collections::{HashMap, HashSet};

use super::SURFACE_WAVE_BYTES;

pub const MORPHOLOGY_ATOM_BYTES: usize = 48;
pub const MORPHOLOGY_STEM_SHAPE_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorphologyGrokkingVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MorphologyWaveConfig {
    pub min_word_chars: usize,
    pub min_stem_chars: usize,
    pub stem_shape_chars: usize,
    pub min_ending_chars: usize,
    pub max_ending_chars: usize,
    pub min_support: usize,
    pub min_stem_diversity: usize,
    pub max_atom_corrupt_rate: f32,
    pub require_stem_shape_match: bool,
    pub max_atoms: usize,
    pub max_corrupt_eval_words: usize,
    pub min_heldout_coverage: f32,
    pub min_real_vs_corrupt_coverage_gap: f32,
    pub max_model_to_naive_ratio: f32,
    pub require_no_exact_lookup_overlap: bool,
}

impl Default for MorphologyWaveConfig {
    fn default() -> Self {
        Self {
            min_word_chars: 5,
            min_stem_chars: 3,
            stem_shape_chars: 1,
            min_ending_chars: 3,
            max_ending_chars: 6,
            min_support: 32,
            min_stem_diversity: 32,
            max_atom_corrupt_rate: 0.15,
            require_stem_shape_match: true,
            max_atoms: 4_096,
            max_corrupt_eval_words: 4_096,
            min_heldout_coverage: 0.70,
            min_real_vs_corrupt_coverage_gap: 0.20,
            max_model_to_naive_ratio: 0.01,
            require_no_exact_lookup_overlap: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyAtom {
    pub ending: String,
    pub ending_chars: usize,
    pub support: usize,
    pub stem_diversity: usize,
    pub stem_shape_diversity: usize,
    pub corrupt_support: usize,
    pub corrupt_rate: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MorphologyExtraction {
    pub word: String,
    pub stem: String,
    pub atom_index: usize,
    pub ending: String,
    pub support: usize,
    pub stem_diversity: usize,
    pub stem_shape_diversity: usize,
}

#[derive(Clone, Debug)]
pub struct MorphologyWaveBank {
    config: MorphologyWaveConfig,
    atoms: Vec<MorphologyAtom>,
    atom_stem_shapes: Vec<HashSet<u64>>,
    index: HashMap<String, usize>,
}

impl MorphologyWaveBank {
    #[must_use]
    pub fn build<'a, I>(words: I, config: MorphologyWaveConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut candidates: HashMap<String, CandidateEndingStats> = HashMap::new();

        for word in words {
            let char_count = word.chars().count();
            if char_count < config.min_word_chars {
                continue;
            }

            for ending_chars in config.min_ending_chars..=config.max_ending_chars {
                let Some((stem, ending)) = split_ending(word, ending_chars) else {
                    continue;
                };
                if stem.chars().count() < config.min_stem_chars {
                    continue;
                }

                let entry = candidates.entry(ending.to_string()).or_default();
                entry.support += 1;
                entry.stems.insert(stable_hash(stem.as_bytes()));
                entry
                    .stem_shapes
                    .insert(stem_shape_hash(stem, config.stem_shape_chars));
            }

            let corrupt_word = word.chars().rev().collect::<String>();
            if corrupt_word == word {
                continue;
            }
            for ending_chars in config.min_ending_chars..=config.max_ending_chars {
                let Some((stem, ending)) = split_ending(&corrupt_word, ending_chars) else {
                    continue;
                };
                if stem.chars().count() < config.min_stem_chars {
                    continue;
                }
                candidates
                    .entry(ending.to_string())
                    .or_default()
                    .corrupt_support += 1;
            }
        }

        let mut built_atoms = candidates
            .into_iter()
            .filter_map(|(ending, stats)| {
                let stem_diversity = stats.stems.len();
                let stem_shape_diversity = stats.stem_shapes.len();
                let corrupt_rate = ratio(stats.corrupt_support, stats.support);
                (stats.support >= config.min_support
                    && stem_diversity >= config.min_stem_diversity
                    && corrupt_rate <= config.max_atom_corrupt_rate)
                    .then(|| BuiltMorphologyAtom {
                        atom: MorphologyAtom {
                            ending_chars: ending.chars().count(),
                            ending,
                            support: stats.support,
                            stem_diversity,
                            stem_shape_diversity,
                            corrupt_support: stats.corrupt_support,
                            corrupt_rate,
                        },
                        stem_shapes: stats.stem_shapes,
                    })
            })
            .collect::<Vec<_>>();
        built_atoms.sort_by(|left, right| {
            right
                .atom
                .ending_chars
                .cmp(&left.atom.ending_chars)
                .then_with(|| right.atom.support.cmp(&left.atom.support))
                .then_with(|| left.atom.ending.cmp(&right.atom.ending))
        });
        built_atoms.truncate(config.max_atoms);

        let atoms = built_atoms
            .iter()
            .map(|built| built.atom.clone())
            .collect::<Vec<_>>();
        let atom_stem_shapes = built_atoms
            .into_iter()
            .map(|built| built.stem_shapes)
            .collect::<Vec<_>>();

        let index = atoms
            .iter()
            .enumerate()
            .map(|(index, atom)| (atom.ending.clone(), index))
            .collect();

        Self {
            config,
            atoms,
            atom_stem_shapes,
            index,
        }
    }

    #[must_use]
    pub fn atoms(&self) -> &[MorphologyAtom] {
        &self.atoms
    }

    #[must_use]
    pub fn atom_count(&self) -> usize {
        self.atoms.len()
    }

    #[must_use]
    pub fn hot_bytes(&self) -> usize {
        self.atoms.len() * MORPHOLOGY_ATOM_BYTES
            + self
                .atom_stem_shapes
                .iter()
                .map(|shapes| shapes.len() * MORPHOLOGY_STEM_SHAPE_BYTES)
                .sum::<usize>()
    }

    #[must_use]
    pub fn extract(&self, word: &str) -> Option<MorphologyExtraction> {
        if word.chars().count() < self.config.min_word_chars {
            return None;
        }

        for atom in &self.atoms {
            if !word.ends_with(&atom.ending) {
                continue;
            }
            let Some((stem, _)) = split_ending(word, atom.ending_chars) else {
                continue;
            };
            if stem.chars().count() < self.config.min_stem_chars {
                continue;
            }
            let atom_index = self.index[&atom.ending];
            if self.config.require_stem_shape_match {
                let stem_shape = stem_shape_hash(stem, self.config.stem_shape_chars);
                if !self.atom_stem_shapes[atom_index].contains(&stem_shape) {
                    continue;
                }
            }
            return Some(MorphologyExtraction {
                word: word.to_string(),
                stem: stem.to_string(),
                atom_index,
                ending: atom.ending.clone(),
                support: atom.support,
                stem_diversity: atom.stem_diversity,
                stem_shape_diversity: atom.stem_shape_diversity,
            });
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyGrokkingProof {
    pub verdict: MorphologyGrokkingVerdict,
    pub train_words: usize,
    pub heldout_words: usize,
    pub atom_count: usize,
    pub train_extracted_words: usize,
    pub heldout_extracted_words: usize,
    pub heldout_coverage: f32,
    pub corrupt_eval_words: usize,
    pub corrupt_extracted_words: usize,
    pub corrupt_coverage: f32,
    pub real_vs_corrupt_coverage_gap: f32,
    pub exact_lookup_heldout_hits: usize,
    pub exact_lookup_heldout_coverage: f32,
    pub model_hot_bytes: usize,
    pub naive_total_wave_bytes: usize,
    pub model_to_naive_total_ratio: f32,
    pub extraction_pass: bool,
    pub corrupt_reject_pass: bool,
    pub anti_lookup_pass: bool,
    pub compression_pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyScalingRow {
    pub train_words: usize,
    pub heldout_words: usize,
    pub atom_count: usize,
    pub heldout_coverage: f32,
    pub corrupt_coverage: f32,
    pub real_vs_corrupt_coverage_gap: f32,
    pub model_hot_bytes: usize,
    pub model_to_naive_total_ratio: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MorphologyScalingReport {
    pub rows: Vec<MorphologyScalingRow>,
    pub plateau_train_words: Option<usize>,
    pub plateau_delta_threshold: f32,
}

impl MorphologyGrokkingProof {
    #[must_use]
    pub fn prove<'a, I, J>(train: I, heldout: J, config: MorphologyWaveConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
        J: IntoIterator<Item = &'a str>,
    {
        let train = train.into_iter().map(str::to_string).collect::<Vec<_>>();
        let heldout = heldout.into_iter().map(str::to_string).collect::<Vec<_>>();
        let bank = MorphologyWaveBank::build(train.iter().map(String::as_str), config);

        let train_extracted_words = train
            .iter()
            .filter(|word| bank.extract(word).is_some())
            .count();
        let heldout_extracted_words = heldout
            .iter()
            .filter(|word| bank.extract(word).is_some())
            .count();
        let heldout_coverage = ratio(heldout_extracted_words, heldout.len());

        let corrupt_words = heldout
            .iter()
            .take(config.max_corrupt_eval_words)
            .map(|word| word.chars().rev().collect::<String>())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        let corrupt_extracted_words = corrupt_words
            .iter()
            .filter(|word| bank.extract(word).is_some())
            .count();
        let corrupt_coverage = ratio(corrupt_extracted_words, corrupt_words.len());
        let real_vs_corrupt_coverage_gap = heldout_coverage - corrupt_coverage;

        let train_set = train.iter().map(String::as_str).collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|word| train_set.contains(word.as_str()))
            .count();
        let exact_lookup_heldout_coverage = ratio(exact_lookup_heldout_hits, heldout.len());

        let model_hot_bytes = bank.hot_bytes();
        let naive_total_wave_bytes = (train.len() + heldout.len()) * SURFACE_WAVE_BYTES;
        let model_to_naive_total_ratio = ratio(model_hot_bytes, naive_total_wave_bytes);

        let extraction_pass = heldout_coverage >= config.min_heldout_coverage;
        let corrupt_reject_pass =
            real_vs_corrupt_coverage_gap >= config.min_real_vs_corrupt_coverage_gap;
        let anti_lookup_pass =
            !config.require_no_exact_lookup_overlap || exact_lookup_heldout_hits == 0;
        let compression_pass = model_to_naive_total_ratio <= config.max_model_to_naive_ratio;
        let verdict =
            if extraction_pass && corrupt_reject_pass && anti_lookup_pass && compression_pass {
                MorphologyGrokkingVerdict::Proven
            } else {
                MorphologyGrokkingVerdict::Watch
            };

        Self {
            verdict,
            train_words: train.len(),
            heldout_words: heldout.len(),
            atom_count: bank.atom_count(),
            train_extracted_words,
            heldout_extracted_words,
            heldout_coverage,
            corrupt_eval_words: corrupt_words.len(),
            corrupt_extracted_words,
            corrupt_coverage,
            real_vs_corrupt_coverage_gap,
            exact_lookup_heldout_hits,
            exact_lookup_heldout_coverage,
            model_hot_bytes,
            naive_total_wave_bytes,
            model_to_naive_total_ratio,
            extraction_pass,
            corrupt_reject_pass,
            anti_lookup_pass,
            compression_pass,
        }
    }
}

impl MorphologyScalingReport {
    #[must_use]
    pub fn from_words(words: &[String], train_sizes: &[usize], heldout_words: usize) -> Self {
        let plateau_delta_threshold = 0.01;
        let mut rows = Vec::new();
        for train_words in train_sizes {
            if *train_words + heldout_words > words.len() {
                continue;
            }
            let proof = MorphologyGrokkingProof::prove(
                words[..*train_words].iter().map(String::as_str),
                words[*train_words..*train_words + heldout_words]
                    .iter()
                    .map(String::as_str),
                scaling_config(*train_words),
            );
            rows.push(MorphologyScalingRow {
                train_words: proof.train_words,
                heldout_words: proof.heldout_words,
                atom_count: proof.atom_count,
                heldout_coverage: proof.heldout_coverage,
                corrupt_coverage: proof.corrupt_coverage,
                real_vs_corrupt_coverage_gap: proof.real_vs_corrupt_coverage_gap,
                model_hot_bytes: proof.model_hot_bytes,
                model_to_naive_total_ratio: proof.model_to_naive_total_ratio,
            });
        }

        let plateau_train_words = rows
            .windows(2)
            .find(|window| {
                (window[1].heldout_coverage - window[0].heldout_coverage).abs()
                    <= plateau_delta_threshold
            })
            .map(|window| window[1].train_words);

        Self {
            rows,
            plateau_train_words,
            plateau_delta_threshold,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct CandidateEndingStats {
    support: usize,
    corrupt_support: usize,
    stems: HashSet<u64>,
    stem_shapes: HashSet<u64>,
}

#[derive(Clone, Debug)]
struct BuiltMorphologyAtom {
    atom: MorphologyAtom,
    stem_shapes: HashSet<u64>,
}

fn scaling_config(train_words: usize) -> MorphologyWaveConfig {
    let min_support = (train_words / 1_875).clamp(24, 128);
    MorphologyWaveConfig {
        min_ending_chars: 2,
        min_support,
        min_stem_diversity: min_support,
        max_atom_corrupt_rate: 0.15,
        min_heldout_coverage: 0.0,
        min_real_vs_corrupt_coverage_gap: 0.0,
        max_model_to_naive_ratio: 1.0,
        max_corrupt_eval_words: 8_192,
        ..MorphologyWaveConfig::default()
    }
}

fn split_ending(word: &str, ending_chars: usize) -> Option<(&str, &str)> {
    let char_count = word.chars().count();
    if ending_chars >= char_count {
        return None;
    }

    let split_char = char_count - ending_chars;
    let split_byte = word
        .char_indices()
        .nth(split_char)
        .map(|(index, _)| index)?;
    Some((&word[..split_byte], &word[split_byte..]))
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut state = 0x4D4F_5250_4857_4156u64;
    for byte in bytes {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state ^ bytes.len() as u64)
}

fn stem_shape_hash(stem: &str, shape_chars: usize) -> u64 {
    if shape_chars == 0 {
        return stable_hash(stem.as_bytes());
    }

    let chars = stem.chars().collect::<Vec<_>>();
    if chars.len() <= shape_chars * 2 {
        return stable_hash(stem.as_bytes());
    }

    let mut shape = String::with_capacity(shape_chars * 4);
    for ch in chars.iter().take(shape_chars) {
        shape.push(*ch);
    }
    shape.push('|');
    for ch in chars.iter().skip(chars.len() - shape_chars) {
        shape.push(*ch);
    }
    stable_hash(shape.as_bytes())
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
