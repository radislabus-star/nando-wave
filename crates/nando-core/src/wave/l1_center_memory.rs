//! L1 center memory over SurfaceWave atoms.
//!
//! This is the first layered-center storage proof. It does not store whole
//! words as waves. It stores reusable local surface-atom/position centers, per-word
//! center-id sequences, and residual counts for rare surface pieces.

use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;

use super::{
    SURFACE_WAVE_BYTES, SURFACE_WAVE_DIM, SURFACE_WAVE_TRITS, SurfaceWave4096, SurfaceWaveTrit,
    surface_atom_projection, surface_atoms,
};

pub const L1_CENTER_RECORD_BYTES: usize = 32;
pub const L1_SEQUENCE_REF_BYTES: usize = 4;
pub const L1_WORD_RECORD_BYTES: usize = 16;
pub const L1_RESIDUAL_NGRAM_BYTES: usize = 8;
pub const L1_FOURIER_BINS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L1CenterMemoryVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct L1CenterMemoryConfig {
    pub min_center_support: usize,
    pub max_centers: usize,
    pub min_heldout_ngram_coverage: f32,
    pub min_average_reconstruction_similarity: f32,
    pub min_average_fourier_similarity: f32,
    pub min_fourier_ablation_drop: f32,
    pub min_real_vs_corrupt_coverage_gap: f32,
    pub max_model_to_naive_ratio: f32,
    pub max_corrupt_eval_words: usize,
    pub max_fourier_eval_words: usize,
    pub ablation_top_bins: usize,
    pub require_no_exact_lookup_overlap: bool,
}

impl Default for L1CenterMemoryConfig {
    fn default() -> Self {
        Self {
            min_center_support: 2,
            max_centers: 1_000_000,
            min_heldout_ngram_coverage: 0.82,
            min_average_reconstruction_similarity: 0.80,
            min_average_fourier_similarity: 0.75,
            min_fourier_ablation_drop: 0.05,
            min_real_vs_corrupt_coverage_gap: 0.15,
            max_model_to_naive_ratio: 0.08,
            max_corrupt_eval_words: 4_096,
            max_fourier_eval_words: 2_048,
            ablation_top_bins: 4,
            require_no_exact_lookup_overlap: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct L1CenterKey {
    ngram_hash: u64,
    position_code: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L1SurfaceCenter {
    pub id: u32,
    pub ngram_hash: u64,
    pub position_code: u16,
    pub support: u32,
    pub trits: [SurfaceWaveTrit; SURFACE_WAVE_TRITS],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L1WordCenterRecord {
    pub source_hash: u64,
    pub ngram_count: u16,
    pub center_ref_start: u32,
    pub center_ref_len: u16,
    pub residual_ngram_count: u16,
}

#[derive(Clone, Debug)]
pub struct L1CenterMemory {
    config: L1CenterMemoryConfig,
    centers: Vec<L1SurfaceCenter>,
    center_index: HashMap<L1CenterKey, u32>,
    word_records: Vec<L1WordCenterRecord>,
    sequence_refs: Vec<u32>,
    residual_ngram_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct L1WordAssignment {
    pub ngram_count: usize,
    pub center_hits: usize,
    pub residual_ngrams: usize,
    pub ngram_coverage: f32,
    pub reconstruction_similarity: f32,
    pub fourier_similarity: f32,
    pub ablated_fourier_similarity: f32,
    pub fourier_ablation_drop: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct L1CenterSequence {
    pub ngram_count: usize,
    pub center_refs: Vec<u32>,
    pub residual_ngrams: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L1CenterMemoryProof {
    pub verdict: L1CenterMemoryVerdict,
    pub train_words: usize,
    pub heldout_words: usize,
    pub center_count: usize,
    pub train_sequence_refs: usize,
    pub train_residual_ngrams: usize,
    pub heldout_ngrams: usize,
    pub heldout_center_hits: usize,
    pub heldout_ngram_coverage: f32,
    pub heldout_word_coverage: f32,
    pub average_reconstruction_similarity: f32,
    pub average_fourier_similarity: f32,
    pub average_ablated_fourier_similarity: f32,
    pub fourier_ablation_drop: f32,
    pub corrupt_eval_words: usize,
    pub corrupt_ngram_coverage: f32,
    pub real_vs_corrupt_coverage_gap: f32,
    pub exact_lookup_heldout_hits: usize,
    pub exact_lookup_heldout_coverage: f32,
    pub model_hot_bytes: usize,
    pub naive_total_wave_bytes: usize,
    pub model_to_naive_total_ratio: f32,
    pub coverage_pass: bool,
    pub reconstruction_pass: bool,
    pub fourier_pass: bool,
    pub ablation_pass: bool,
    pub corrupt_reject_pass: bool,
    pub compression_pass: bool,
    pub anti_lookup_pass: bool,
    pub promotion_ready_for_l2: bool,
}

impl L1CenterMemory {
    #[must_use]
    pub fn build<'a, I>(words: I, config: L1CenterMemoryConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let words = words.into_iter().map(str::to_string).collect::<Vec<_>>();
        let centers = build_centers(words.iter().map(String::as_str), config);
        let center_index = centers
            .iter()
            .map(|center| {
                (
                    L1CenterKey {
                        ngram_hash: center.ngram_hash,
                        position_code: center.position_code,
                    },
                    center.id,
                )
            })
            .collect::<HashMap<_, _>>();

        let mut memory = Self {
            config,
            centers,
            center_index,
            word_records: Vec::with_capacity(words.len()),
            sequence_refs: Vec::new(),
            residual_ngram_count: 0,
        };
        for word in &words {
            memory.encode_train_word(word);
        }
        memory
    }

    #[must_use]
    pub fn center_count(&self) -> usize {
        self.centers.len()
    }

    #[must_use]
    pub fn centers(&self) -> &[L1SurfaceCenter] {
        &self.centers
    }

    #[must_use]
    pub fn word_records(&self) -> &[L1WordCenterRecord] {
        &self.word_records
    }

    #[must_use]
    pub fn sequence_refs(&self) -> &[u32] {
        &self.sequence_refs
    }

    #[must_use]
    pub fn hot_bytes(&self) -> usize {
        self.centers.len() * L1_CENTER_RECORD_BYTES
            + self.sequence_refs.len() * L1_SEQUENCE_REF_BYTES
            + self.word_records.len() * L1_WORD_RECORD_BYTES
            + self.residual_ngram_count * L1_RESIDUAL_NGRAM_BYTES
    }

    #[must_use]
    pub fn assign_word(&self, word: &str) -> L1WordAssignment {
        let refs = self.center_refs_for_word(word);
        let actual_wave = SurfaceWave4096::compile(word);
        let reconstructed = self.reconstruct_lanes(&refs.center_refs);
        let reconstruction_similarity = cosine_similarity_i16(actual_wave.lanes(), &reconstructed);
        let fourier = fourier_report(
            actual_wave.lanes(),
            &reconstructed,
            self.config.ablation_top_bins,
        );

        L1WordAssignment {
            ngram_count: refs.ngram_count,
            center_hits: refs.center_refs.len(),
            residual_ngrams: refs.residual_ngrams,
            ngram_coverage: ratio(refs.center_refs.len(), refs.ngram_count),
            reconstruction_similarity,
            fourier_similarity: fourier.similarity,
            ablated_fourier_similarity: fourier.ablated_similarity,
            fourier_ablation_drop: fourier.ablation_drop,
        }
    }

    #[must_use]
    pub fn center_sequence_for_word(&self, word: &str) -> L1CenterSequence {
        let refs = self.center_refs_for_word(word);
        L1CenterSequence {
            ngram_count: refs.ngram_count,
            center_refs: refs.center_refs,
            residual_ngrams: refs.residual_ngrams,
        }
    }

    fn encode_train_word(&mut self, word: &str) {
        let refs = self.center_refs_for_word(word);
        let start = self.sequence_refs.len();
        self.sequence_refs.extend(refs.center_refs.iter().copied());
        self.residual_ngram_count += refs.residual_ngrams;
        self.word_records.push(L1WordCenterRecord {
            source_hash: stable_hash(word.as_bytes()),
            ngram_count: refs.ngram_count as u16,
            center_ref_start: start as u32,
            center_ref_len: refs.center_refs.len() as u16,
            residual_ngram_count: refs.residual_ngrams as u16,
        });
    }

    fn center_refs_for_word(&self, word: &str) -> WordRefs {
        let atoms = surface_atoms(word);

        let mut center_refs = Vec::with_capacity(atoms.len());
        let mut residual_ngrams = 0usize;
        for atom in &atoms {
            let key = L1CenterKey {
                ngram_hash: stable_hash(&atom.bytes),
                position_code: position_code(atom.position),
            };
            if let Some(center_id) = self.center_index.get(&key) {
                center_refs.push(*center_id);
            } else {
                residual_ngrams += 1;
            }
        }

        WordRefs {
            ngram_count: atoms.len(),
            center_refs,
            residual_ngrams,
        }
    }

    fn reconstruct_lanes(&self, center_refs: &[u32]) -> [i16; SURFACE_WAVE_DIM] {
        let mut lanes = [0i16; SURFACE_WAVE_DIM];
        for center_id in center_refs {
            let center = &self.centers[*center_id as usize];
            for trit in center.trits {
                if trit.value == 0 {
                    continue;
                }
                let lane = usize::from(trit.lane);
                lanes[lane] = lanes[lane].saturating_add(i16::from(trit.value));
            }
        }
        lanes
    }
}

impl L1CenterMemoryProof {
    #[must_use]
    pub fn prove<'a, I, J>(train: I, heldout: J, config: L1CenterMemoryConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
        J: IntoIterator<Item = &'a str>,
    {
        let train = train.into_iter().map(str::to_string).collect::<Vec<_>>();
        let heldout = heldout.into_iter().map(str::to_string).collect::<Vec<_>>();
        let memory = L1CenterMemory::build(train.iter().map(String::as_str), config);

        let mut heldout_ngrams = 0usize;
        let mut heldout_center_hits = 0usize;
        let mut covered_words = 0usize;
        let mut reconstruction_sum = 0.0;
        let mut reconstruction_cases = 0usize;
        let mut fourier_sum = 0.0;
        let mut ablated_fourier_sum = 0.0;
        let mut ablation_drop_sum = 0.0;
        let mut fourier_cases = 0usize;

        for (index, word) in heldout.iter().enumerate() {
            let assignment = memory.assign_word(word);
            heldout_ngrams += assignment.ngram_count;
            heldout_center_hits += assignment.center_hits;
            if assignment.ngram_coverage >= config.min_heldout_ngram_coverage {
                covered_words += 1;
            }
            if assignment.ngram_count > 0 {
                reconstruction_sum += assignment.reconstruction_similarity;
                reconstruction_cases += 1;
            }
            if index < config.max_fourier_eval_words && assignment.ngram_count > 0 {
                fourier_sum += assignment.fourier_similarity;
                ablated_fourier_sum += assignment.ablated_fourier_similarity;
                ablation_drop_sum += assignment.fourier_ablation_drop;
                fourier_cases += 1;
            }
        }

        let corrupt_words = heldout
            .iter()
            .take(config.max_corrupt_eval_words)
            .map(|word| word.chars().rev().collect::<String>())
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        let mut corrupt_ngrams = 0usize;
        let mut corrupt_center_hits = 0usize;
        for word in &corrupt_words {
            let assignment = memory.assign_word(word);
            corrupt_ngrams += assignment.ngram_count;
            corrupt_center_hits += assignment.center_hits;
        }

        let train_set = train.iter().map(String::as_str).collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|word| train_set.contains(word.as_str()))
            .count();
        let exact_lookup_heldout_coverage = ratio(exact_lookup_heldout_hits, heldout.len());

        let heldout_ngram_coverage = ratio(heldout_center_hits, heldout_ngrams);
        let heldout_word_coverage = ratio(covered_words, heldout.len());
        let average_reconstruction_similarity = ratio_f32(reconstruction_sum, reconstruction_cases);
        let average_fourier_similarity = ratio_f32(fourier_sum, fourier_cases);
        let average_ablated_fourier_similarity = ratio_f32(ablated_fourier_sum, fourier_cases);
        let fourier_ablation_drop = ratio_f32(ablation_drop_sum, fourier_cases);
        let corrupt_ngram_coverage = ratio(corrupt_center_hits, corrupt_ngrams);
        let real_vs_corrupt_coverage_gap = heldout_ngram_coverage - corrupt_ngram_coverage;

        let model_hot_bytes = memory.hot_bytes();
        let naive_total_wave_bytes = (train.len() + heldout.len()) * SURFACE_WAVE_BYTES;
        let model_to_naive_total_ratio = ratio(model_hot_bytes, naive_total_wave_bytes);

        let coverage_pass = heldout_ngram_coverage >= config.min_heldout_ngram_coverage;
        let reconstruction_pass =
            average_reconstruction_similarity >= config.min_average_reconstruction_similarity;
        let fourier_pass = average_fourier_similarity >= config.min_average_fourier_similarity;
        let ablation_pass = fourier_ablation_drop >= config.min_fourier_ablation_drop;
        let corrupt_reject_pass =
            real_vs_corrupt_coverage_gap >= config.min_real_vs_corrupt_coverage_gap;
        let compression_pass = model_to_naive_total_ratio <= config.max_model_to_naive_ratio;
        let anti_lookup_pass =
            !config.require_no_exact_lookup_overlap || exact_lookup_heldout_hits == 0;
        let promotion_ready_for_l2 = coverage_pass
            && reconstruction_pass
            && fourier_pass
            && ablation_pass
            && corrupt_reject_pass
            && compression_pass
            && anti_lookup_pass;
        let verdict = if promotion_ready_for_l2 {
            L1CenterMemoryVerdict::Proven
        } else {
            L1CenterMemoryVerdict::Watch
        };

        Self {
            verdict,
            train_words: train.len(),
            heldout_words: heldout.len(),
            center_count: memory.center_count(),
            train_sequence_refs: memory.sequence_refs().len(),
            train_residual_ngrams: memory.residual_ngram_count,
            heldout_ngrams,
            heldout_center_hits,
            heldout_ngram_coverage,
            heldout_word_coverage,
            average_reconstruction_similarity,
            average_fourier_similarity,
            average_ablated_fourier_similarity,
            fourier_ablation_drop,
            corrupt_eval_words: corrupt_words.len(),
            corrupt_ngram_coverage,
            real_vs_corrupt_coverage_gap,
            exact_lookup_heldout_hits,
            exact_lookup_heldout_coverage,
            model_hot_bytes,
            naive_total_wave_bytes,
            model_to_naive_total_ratio,
            coverage_pass,
            reconstruction_pass,
            fourier_pass,
            ablation_pass,
            corrupt_reject_pass,
            compression_pass,
            anti_lookup_pass,
            promotion_ready_for_l2,
        }
    }
}

#[derive(Default)]
struct WordRefs {
    ngram_count: usize,
    center_refs: Vec<u32>,
    residual_ngrams: usize,
}

#[derive(Clone, Copy, Debug)]
struct CenterStats {
    support: usize,
    trits: [SurfaceWaveTrit; SURFACE_WAVE_TRITS],
}

fn build_centers<'a, I>(words: I, config: L1CenterMemoryConfig) -> Vec<L1SurfaceCenter>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut candidates: HashMap<L1CenterKey, CenterStats> = HashMap::new();
    for word in words {
        for atom in surface_atoms(word) {
            let trits = surface_atom_projection(atom.position, &atom.bytes);
            candidates
                .entry(L1CenterKey {
                    ngram_hash: stable_hash(&atom.bytes),
                    position_code: position_code(atom.position),
                })
                .and_modify(|stats| stats.support += 1)
                .or_insert(CenterStats { support: 1, trits });
        }
    }

    let mut centers = candidates
        .into_iter()
        .filter(|(_, stats)| stats.support >= config.min_center_support)
        .collect::<Vec<_>>();
    centers.sort_by(|(left_key, left), (right_key, right)| {
        right
            .support
            .cmp(&left.support)
            .then_with(|| left_key.ngram_hash.cmp(&right_key.ngram_hash))
            .then_with(|| left_key.position_code.cmp(&right_key.position_code))
    });
    centers.truncate(config.max_centers);

    centers
        .into_iter()
        .enumerate()
        .map(|(id, (key, stats))| L1SurfaceCenter {
            id: id as u32,
            ngram_hash: key.ngram_hash,
            position_code: key.position_code,
            support: stats.support as u32,
            trits: stats.trits,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default)]
struct FourierReport {
    similarity: f32,
    ablated_similarity: f32,
    ablation_drop: f32,
}

fn fourier_report(
    actual: &[i16; SURFACE_WAVE_DIM],
    reconstructed: &[i16; SURFACE_WAVE_DIM],
    ablation_top_bins: usize,
) -> FourierReport {
    let actual_signature = L1FourierSignature::from_lanes(actual);
    let reconstructed_signature = L1FourierSignature::from_lanes(reconstructed);
    let similarity = actual_signature.cosine_similarity(&reconstructed_signature);
    let ablated = reconstructed_signature.with_top_bins_zeroed(ablation_top_bins);
    let ablated_similarity = actual_signature.cosine_similarity(&ablated);
    FourierReport {
        similarity,
        ablated_similarity,
        ablation_drop: similarity - ablated_similarity,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct L1FourierSignature {
    bins: [(f32, f32); L1_FOURIER_BINS],
}

impl L1FourierSignature {
    fn from_lanes(lanes: &[i16; SURFACE_WAVE_DIM]) -> Self {
        let bins = std::array::from_fn(|bin| {
            let frequency = bin + 1;
            let mut re = 0.0;
            let mut im = 0.0;
            for (index, value) in lanes.iter().enumerate() {
                if *value == 0 {
                    continue;
                }
                let theta = TAU * frequency as f32 * index as f32 / SURFACE_WAVE_DIM as f32;
                re += f32::from(*value) * theta.cos();
                im -= f32::from(*value) * theta.sin();
            }
            (re, im)
        });
        Self { bins }
    }

    fn cosine_similarity(&self, other: &Self) -> f32 {
        let mut dot = 0.0;
        let mut left_norm = 0.0;
        let mut right_norm = 0.0;
        for ((left_re, left_im), (right_re, right_im)) in self.bins.iter().zip(other.bins.iter()) {
            dot += left_re * right_re + left_im * right_im;
            left_norm += left_re * left_re + left_im * left_im;
            right_norm += right_re * right_re + right_im * right_im;
        }
        if left_norm == 0.0 || right_norm == 0.0 {
            return 0.0;
        }
        dot / (left_norm.sqrt() * right_norm.sqrt())
    }

    fn with_top_bins_zeroed(&self, top_bins: usize) -> Self {
        let mut bins = self.bins;
        let mut order = bins
            .iter()
            .enumerate()
            .map(|(index, (re, im))| (index, re * re + im * im))
            .collect::<Vec<_>>();
        order.sort_by(|left, right| right.1.total_cmp(&left.1));
        for (index, _) in order.into_iter().take(top_bins.min(L1_FOURIER_BINS)) {
            bins[index] = (0.0, 0.0);
        }
        Self { bins }
    }
}

fn cosine_similarity_i16(left: &[i16; SURFACE_WAVE_DIM], right: &[i16; SURFACE_WAVE_DIM]) -> f32 {
    let mut dot = 0i64;
    let mut left_energy = 0i64;
    let mut right_energy = 0i64;
    for (left, right) in left.iter().zip(right.iter()) {
        dot += i64::from(*left) * i64::from(*right);
        left_energy += i64::from(*left) * i64::from(*left);
        right_energy += i64::from(*right) * i64::from(*right);
    }
    if left_energy == 0 || right_energy == 0 {
        return 0.0;
    }
    dot as f32 / ((left_energy as f32).sqrt() * (right_energy as f32).sqrt())
}

fn position_code(position: u64) -> u16 {
    let low = (position as u16) & 0x3f;
    let block = ((position / 8) as u16) & 0x03ff;
    low | (block << 6)
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

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut state = 0x4C31_4345_4E54_4552u64;
    for byte in bytes {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state ^ bytes.len() as u64)
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_center_memory_stores_sequences_not_whole_word_waves() {
        let words = [
            "волновая",
            "волнового",
            "волновому",
            "памятная",
            "памятного",
            "памятному",
        ];
        let memory = L1CenterMemory::build(
            words.iter().copied(),
            L1CenterMemoryConfig {
                min_center_support: 1,
                ..L1CenterMemoryConfig::default()
            },
        );

        assert!(memory.center_count() > 0);
        assert_eq!(memory.word_records().len(), words.len());
        assert!(memory.sequence_refs().len() > words.len());
        assert!(memory.hot_bytes() < words.len() * SURFACE_WAVE_BYTES);

        let assignment = memory.assign_word("волновыми");
        assert!(assignment.ngram_count > 0);
        assert!(assignment.center_hits > 0);
        assert!(assignment.reconstruction_similarity > 0.0);
    }

    #[test]
    fn l1_center_memory_keeps_short_words_and_function_words_as_surface_atoms() {
        let words = ["и", "в", "не", "сыч", "и", "в", "не", "сыч", "не работает"];
        let memory = L1CenterMemory::build(
            words.iter().copied(),
            L1CenterMemoryConfig {
                min_center_support: 1,
                ..L1CenterMemoryConfig::default()
            },
        );

        for word in ["и", "в", "не", "сыч"] {
            let sequence = memory.center_sequence_for_word(word);
            assert!(
                sequence.ngram_count > 0,
                "word={word} sequence={sequence:?}"
            );
            assert!(!sequence.center_refs.is_empty(), "word={word}");
        }

        let service = memory.center_sequence_for_word("и");
        let content = memory.center_sequence_for_word("сыч");
        assert_ne!(service.center_refs, content.center_refs);
    }
}
