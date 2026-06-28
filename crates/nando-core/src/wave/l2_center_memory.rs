//! L2 center memory over L1 center-id sequences.
//!
//! L2 does not read raw bytes directly. It consumes the promoted L1 center
//! sequence for each word and learns reusable center-sequence motifs.

use std::collections::{HashMap, HashSet};
use std::f32::consts::TAU;

use super::{L1_SEQUENCE_REF_BYTES, L1CenterMemory, L1CenterMemoryConfig};

pub const L2_CENTER_RECORD_BYTES: usize = 32;
pub const L2_TOKEN_REF_BYTES: usize = 4;
pub const L2_WORD_RECORD_BYTES: usize = 16;
pub const L2_RESIDUAL_REF_BYTES: usize = 4;
pub const L2_FOURIER_BINS: usize = 16;
const L2_MISSING_CENTER_REF: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum L2CenterMemoryVerdict {
    Proven,
    Watch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct L2CenterMemoryConfig {
    pub l1_config: L1CenterMemoryConfig,
    pub motif_len: usize,
    pub min_motif_support: usize,
    pub max_motifs: usize,
    pub min_heldout_ref_coverage: f32,
    pub min_heldout_word_coverage: f32,
    pub min_average_sequence_similarity: f32,
    pub min_average_fourier_similarity: f32,
    pub min_fourier_ablation_drop: f32,
    pub min_real_vs_corrupt_coverage_gap: f32,
    pub max_model_to_naive_ratio: f32,
    pub max_corrupt_eval_words: usize,
    pub max_fourier_eval_words: usize,
    pub ablation_top_bins: usize,
    pub require_no_exact_lookup_overlap: bool,
}

impl Default for L2CenterMemoryConfig {
    fn default() -> Self {
        Self {
            l1_config: L1CenterMemoryConfig::default(),
            motif_len: 4,
            min_motif_support: 4,
            max_motifs: 512_000,
            min_heldout_ref_coverage: 0.65,
            min_heldout_word_coverage: 0.65,
            min_average_sequence_similarity: 0.65,
            min_average_fourier_similarity: 0.60,
            min_fourier_ablation_drop: 0.03,
            min_real_vs_corrupt_coverage_gap: 0.10,
            max_model_to_naive_ratio: 0.85,
            max_corrupt_eval_words: 4_096,
            max_fourier_eval_words: 2_048,
            ablation_top_bins: 4,
            require_no_exact_lookup_overlap: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2SequenceCenter {
    pub id: u32,
    pub sequence_hash: u64,
    pub support: u32,
    pub l1_center_refs: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct L2WordRecord {
    pub source_hash: u64,
    pub l1_ref_count: u16,
    pub token_start: u32,
    pub token_len: u16,
    pub covered_l1_refs: u16,
    pub residual_l1_refs: u16,
}

#[derive(Clone, Debug)]
pub struct L2CenterMemory {
    config: L2CenterMemoryConfig,
    l1: L1CenterMemory,
    centers: Vec<L2SequenceCenter>,
    center_index: HashMap<Vec<u32>, u32>,
    word_records: Vec<L2WordRecord>,
    token_refs: Vec<u32>,
    residual_ref_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct L2TokenSequence {
    pub l1_ref_count: usize,
    pub tokens: Vec<u32>,
    pub motif_refs: usize,
    pub covered_l1_refs: usize,
    pub residual_l1_refs: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct L2WordAssignment {
    pub l1_ref_count: usize,
    pub motif_refs: usize,
    pub covered_l1_refs: usize,
    pub residual_l1_refs: usize,
    pub ref_coverage: f32,
    pub sequence_similarity: f32,
    pub fourier_similarity: f32,
    pub ablated_fourier_similarity: f32,
    pub fourier_ablation_drop: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct L2CenterMemoryProof {
    pub verdict: L2CenterMemoryVerdict,
    pub train_words: usize,
    pub heldout_words: usize,
    pub l1_center_count: usize,
    pub l2_center_count: usize,
    pub train_l1_refs: usize,
    pub train_l2_token_refs: usize,
    pub train_residual_l1_refs: usize,
    pub heldout_l1_refs: usize,
    pub heldout_covered_l1_refs: usize,
    pub heldout_ref_coverage: f32,
    pub heldout_word_coverage: f32,
    pub average_sequence_similarity: f32,
    pub average_fourier_similarity: f32,
    pub average_ablated_fourier_similarity: f32,
    pub fourier_ablation_drop: f32,
    pub corrupt_eval_words: usize,
    pub corrupt_ref_coverage: f32,
    pub real_vs_corrupt_coverage_gap: f32,
    pub exact_lookup_heldout_hits: usize,
    pub exact_lookup_heldout_coverage: f32,
    pub model_hot_bytes: usize,
    pub naive_total_l1_sequence_bytes: usize,
    pub model_to_naive_total_ratio: f32,
    pub coverage_pass: bool,
    pub sequence_pass: bool,
    pub fourier_pass: bool,
    pub ablation_pass: bool,
    pub corrupt_reject_pass: bool,
    pub compression_pass: bool,
    pub anti_lookup_pass: bool,
    pub promotion_ready_for_l3: bool,
}

impl L2CenterMemory {
    #[must_use]
    pub fn build<'a, I>(words: I, config: L2CenterMemoryConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let words = words.into_iter().map(str::to_string).collect::<Vec<_>>();
        let l1 = L1CenterMemory::build(words.iter().map(String::as_str), config.l1_config);
        let train_sequences = words
            .iter()
            .map(|word| l1.center_sequence_for_word(word).center_refs)
            .collect::<Vec<_>>();
        let centers = build_l2_centers(&train_sequences, config);
        let center_index = centers
            .iter()
            .map(|center| (center.l1_center_refs.clone(), center.id))
            .collect::<HashMap<_, _>>();
        let mut memory = Self {
            config,
            l1,
            centers,
            center_index,
            word_records: Vec::with_capacity(words.len()),
            token_refs: Vec::new(),
            residual_ref_count: 0,
        };

        for word in &words {
            memory.encode_train_word(word);
        }

        memory
    }

    #[must_use]
    pub fn l1(&self) -> &L1CenterMemory {
        &self.l1
    }

    #[must_use]
    pub fn center_count(&self) -> usize {
        self.centers.len()
    }

    #[must_use]
    pub fn centers(&self) -> &[L2SequenceCenter] {
        &self.centers
    }

    #[must_use]
    pub fn word_records(&self) -> &[L2WordRecord] {
        &self.word_records
    }

    #[must_use]
    pub fn token_refs(&self) -> &[u32] {
        &self.token_refs
    }

    #[must_use]
    pub fn token_sequence_for_text(&self, text: &str) -> L2TokenSequence {
        let sequence = self.l1.center_sequence_for_word(text).center_refs;
        let encoded = self.encode_sequence(&sequence);
        L2TokenSequence {
            l1_ref_count: sequence.len(),
            tokens: encoded.tokens,
            motif_refs: encoded.motif_refs,
            covered_l1_refs: encoded.covered_l1_refs,
            residual_l1_refs: encoded.residual_l1_refs,
        }
    }

    #[must_use]
    pub fn reconstruct_train_l1_sequence(&self, record_index: usize) -> Option<Vec<u32>> {
        let record = self.word_records.get(record_index)?;
        let start = record.token_start as usize;
        let end = start + record.token_len as usize;
        self.token_refs
            .get(start..end)
            .map(|tokens| self.reconstruct_sequence(tokens))
    }

    #[must_use]
    pub fn hot_bytes(&self) -> usize {
        self.centers.len() * L2_CENTER_RECORD_BYTES
            + self
                .centers
                .iter()
                .map(|center| center.l1_center_refs.len() * L1_SEQUENCE_REF_BYTES)
                .sum::<usize>()
            + self.token_refs.len() * L2_TOKEN_REF_BYTES
            + self.word_records.len() * L2_WORD_RECORD_BYTES
    }

    #[must_use]
    pub fn assign_word(&self, word: &str) -> L2WordAssignment {
        let sequence = self.l1.center_sequence_for_word(word).center_refs;
        self.assign_l1_sequence(&sequence)
    }

    fn assign_l1_sequence(&self, sequence: &[u32]) -> L2WordAssignment {
        let encoded = self.encode_sequence(sequence);
        let center_only = self.reconstruct_center_only_aligned_sequence(&encoded.tokens);
        let sequence_similarity = sequence_cosine_similarity(sequence, &center_only);
        let fourier =
            sequence_fourier_report(sequence, &center_only, self.config.ablation_top_bins);

        L2WordAssignment {
            l1_ref_count: sequence.len(),
            motif_refs: encoded.motif_refs,
            covered_l1_refs: encoded.covered_l1_refs,
            residual_l1_refs: encoded.residual_l1_refs,
            ref_coverage: ratio(encoded.covered_l1_refs, sequence.len()),
            sequence_similarity,
            fourier_similarity: fourier.similarity,
            ablated_fourier_similarity: fourier.ablated_similarity,
            fourier_ablation_drop: fourier.ablation_drop,
        }
    }

    fn encode_train_word(&mut self, word: &str) {
        let sequence = self.l1.center_sequence_for_word(word).center_refs;
        let encoded = self.encode_sequence(&sequence);
        let start = self.token_refs.len();
        self.token_refs.extend(encoded.tokens.iter().copied());
        self.residual_ref_count += encoded.residual_l1_refs;
        self.word_records.push(L2WordRecord {
            source_hash: stable_hash(word.as_bytes()),
            l1_ref_count: sequence.len() as u16,
            token_start: start as u32,
            token_len: encoded.tokens.len() as u16,
            covered_l1_refs: encoded.covered_l1_refs as u16,
            residual_l1_refs: encoded.residual_l1_refs as u16,
        });
    }

    fn encode_sequence(&self, sequence: &[u32]) -> EncodedSequence {
        if sequence.is_empty() {
            return EncodedSequence::default();
        }
        let mut tokens = Vec::new();
        let mut covered_l1_refs = 0usize;
        let mut residual_l1_refs = 0usize;
        let mut motif_refs = 0usize;
        let mut position = 0usize;

        while position < sequence.len() {
            if position + self.config.motif_len <= sequence.len() {
                let window = &sequence[position..position + self.config.motif_len];
                if let Some(center_id) = self.center_index.get(window) {
                    tokens.push(*center_id);
                    covered_l1_refs += self.config.motif_len;
                    motif_refs += 1;
                    position += self.config.motif_len;
                    continue;
                }
            }

            tokens.push(residual_token(sequence[position]));
            residual_l1_refs += 1;
            position += 1;
        }

        EncodedSequence {
            tokens,
            covered_l1_refs,
            residual_l1_refs,
            motif_refs,
        }
    }

    fn reconstruct_center_only_aligned_sequence(&self, tokens: &[u32]) -> Vec<u32> {
        let mut sequence = Vec::new();
        for token in tokens {
            if is_residual_token(*token) {
                sequence.push(L2_MISSING_CENTER_REF);
                continue;
            }
            if let Some(center) = self.centers.get(*token as usize) {
                sequence.extend(center.l1_center_refs.iter().copied());
            }
        }
        sequence
    }

    fn reconstruct_sequence(&self, tokens: &[u32]) -> Vec<u32> {
        let mut sequence = Vec::new();
        for token in tokens {
            if is_residual_token(*token) {
                sequence.push(untag_residual_token(*token));
            } else if let Some(center) = self.centers.get(*token as usize) {
                sequence.extend(center.l1_center_refs.iter().copied());
            }
        }
        sequence
    }
}

impl L2CenterMemoryProof {
    #[must_use]
    pub fn prove<'a, I, J>(train: I, heldout: J, config: L2CenterMemoryConfig) -> Self
    where
        I: IntoIterator<Item = &'a str>,
        J: IntoIterator<Item = &'a str>,
    {
        let train = train.into_iter().map(str::to_string).collect::<Vec<_>>();
        let heldout = heldout.into_iter().map(str::to_string).collect::<Vec<_>>();
        let memory = L2CenterMemory::build(train.iter().map(String::as_str), config);

        let mut heldout_l1_refs = 0usize;
        let mut heldout_covered_l1_refs = 0usize;
        let mut covered_words = 0usize;
        let mut sequence_sum = 0.0;
        let mut fourier_sum = 0.0;
        let mut ablated_fourier_sum = 0.0;
        let mut ablation_drop_sum = 0.0;
        let mut sequence_cases = 0usize;
        let mut fourier_cases = 0usize;

        for (index, word) in heldout.iter().enumerate() {
            let assignment = memory.assign_word(word);
            heldout_l1_refs += assignment.l1_ref_count;
            heldout_covered_l1_refs += assignment.covered_l1_refs;
            if assignment.ref_coverage >= config.min_heldout_ref_coverage {
                covered_words += 1;
            }
            if assignment.l1_ref_count > 0 {
                sequence_sum += assignment.sequence_similarity;
                sequence_cases += 1;
            }
            if index < config.max_fourier_eval_words && assignment.l1_ref_count > 0 {
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
        let mut corrupt_l1_refs = 0usize;
        let mut corrupt_covered_l1_refs = 0usize;
        for word in &corrupt_words {
            let assignment = memory.assign_word(word);
            corrupt_l1_refs += assignment.l1_ref_count;
            corrupt_covered_l1_refs += assignment.covered_l1_refs;
        }

        let train_set = train.iter().map(String::as_str).collect::<HashSet<_>>();
        let exact_lookup_heldout_hits = heldout
            .iter()
            .filter(|word| train_set.contains(word.as_str()))
            .count();
        let exact_lookup_heldout_coverage = ratio(exact_lookup_heldout_hits, heldout.len());

        let heldout_ref_coverage = ratio(heldout_covered_l1_refs, heldout_l1_refs);
        let heldout_word_coverage = ratio(covered_words, heldout.len());
        let average_sequence_similarity = ratio_f32(sequence_sum, sequence_cases);
        let average_fourier_similarity = ratio_f32(fourier_sum, fourier_cases);
        let average_ablated_fourier_similarity = ratio_f32(ablated_fourier_sum, fourier_cases);
        let fourier_ablation_drop = ratio_f32(ablation_drop_sum, fourier_cases);
        let corrupt_ref_coverage = ratio(corrupt_covered_l1_refs, corrupt_l1_refs);
        let real_vs_corrupt_coverage_gap = heldout_ref_coverage - corrupt_ref_coverage;

        let train_l1_refs = memory.l1().sequence_refs().len();
        let model_hot_bytes = memory.hot_bytes();
        let naive_total_l1_sequence_bytes = (train_l1_refs + heldout_l1_refs)
            * L1_SEQUENCE_REF_BYTES
            + (train.len() + heldout.len()) * L2_WORD_RECORD_BYTES;
        let model_to_naive_total_ratio = ratio(model_hot_bytes, naive_total_l1_sequence_bytes);

        let coverage_pass = heldout_ref_coverage >= config.min_heldout_ref_coverage
            && heldout_word_coverage >= config.min_heldout_word_coverage;
        let sequence_pass = average_sequence_similarity >= config.min_average_sequence_similarity;
        let fourier_pass = average_fourier_similarity >= config.min_average_fourier_similarity;
        let ablation_pass = fourier_ablation_drop >= config.min_fourier_ablation_drop;
        let corrupt_reject_pass =
            real_vs_corrupt_coverage_gap >= config.min_real_vs_corrupt_coverage_gap;
        let compression_pass = model_to_naive_total_ratio <= config.max_model_to_naive_ratio;
        let anti_lookup_pass =
            !config.require_no_exact_lookup_overlap || exact_lookup_heldout_hits == 0;
        let promotion_ready_for_l3 = coverage_pass
            && sequence_pass
            && fourier_pass
            && ablation_pass
            && corrupt_reject_pass
            && compression_pass
            && anti_lookup_pass;
        let verdict = if promotion_ready_for_l3 {
            L2CenterMemoryVerdict::Proven
        } else {
            L2CenterMemoryVerdict::Watch
        };

        Self {
            verdict,
            train_words: train.len(),
            heldout_words: heldout.len(),
            l1_center_count: memory.l1().center_count(),
            l2_center_count: memory.center_count(),
            train_l1_refs,
            train_l2_token_refs: memory.token_refs().len(),
            train_residual_l1_refs: memory.residual_ref_count,
            heldout_l1_refs,
            heldout_covered_l1_refs,
            heldout_ref_coverage,
            heldout_word_coverage,
            average_sequence_similarity,
            average_fourier_similarity,
            average_ablated_fourier_similarity,
            fourier_ablation_drop,
            corrupt_eval_words: corrupt_words.len(),
            corrupt_ref_coverage,
            real_vs_corrupt_coverage_gap,
            exact_lookup_heldout_hits,
            exact_lookup_heldout_coverage,
            model_hot_bytes,
            naive_total_l1_sequence_bytes,
            model_to_naive_total_ratio,
            coverage_pass,
            sequence_pass,
            fourier_pass,
            ablation_pass,
            corrupt_reject_pass,
            compression_pass,
            anti_lookup_pass,
            promotion_ready_for_l3,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct EncodedSequence {
    tokens: Vec<u32>,
    covered_l1_refs: usize,
    residual_l1_refs: usize,
    motif_refs: usize,
}

fn build_l2_centers(sequences: &[Vec<u32>], config: L2CenterMemoryConfig) -> Vec<L2SequenceCenter> {
    let mut candidates: HashMap<Vec<u32>, usize> = HashMap::new();
    if config.motif_len == 0 {
        return Vec::new();
    }

    for sequence in sequences {
        if sequence.len() < config.motif_len {
            continue;
        }
        for window in sequence.windows(config.motif_len) {
            *candidates.entry(window.to_vec()).or_default() += 1;
        }
    }

    let mut centers = candidates
        .into_iter()
        .filter(|(_, support)| *support >= config.min_motif_support)
        .collect::<Vec<_>>();
    centers.sort_by(
        |(left_sequence, left_support), (right_sequence, right_support)| {
            right_support
                .cmp(left_support)
                .then_with(|| left_sequence.cmp(right_sequence))
        },
    );
    centers.truncate(config.max_motifs);

    centers
        .into_iter()
        .enumerate()
        .map(|(id, (sequence, support))| L2SequenceCenter {
            id: id as u32,
            sequence_hash: stable_hash_u32s(&sequence),
            support: support as u32,
            l1_center_refs: sequence,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default)]
struct SequenceFourierReport {
    similarity: f32,
    ablated_similarity: f32,
    ablation_drop: f32,
}

fn sequence_fourier_report(
    actual: &[u32],
    reconstructed: &[u32],
    ablation_top_bins: usize,
) -> SequenceFourierReport {
    let actual_signature = L2FourierSignature::from_sequence(actual);
    let reconstructed_signature = L2FourierSignature::from_sequence(reconstructed);
    let similarity = actual_signature.cosine_similarity(&reconstructed_signature);
    let ablated = reconstructed_signature.with_top_bins_zeroed(ablation_top_bins);
    let ablated_similarity = actual_signature.cosine_similarity(&ablated);
    SequenceFourierReport {
        similarity,
        ablated_similarity,
        ablation_drop: similarity - ablated_similarity,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct L2FourierSignature {
    bins: [(f32, f32); L2_FOURIER_BINS],
}

impl L2FourierSignature {
    fn from_sequence(sequence: &[u32]) -> Self {
        if sequence.is_empty() {
            return Self {
                bins: [(0.0, 0.0); L2_FOURIER_BINS],
            };
        }
        let bins = std::array::from_fn(|bin| {
            let frequency = bin + 1;
            let mut re = 0.0;
            let mut im = 0.0;
            for (index, center_id) in sequence.iter().enumerate() {
                let amplitude = center_amplitude(*center_id);
                let theta = TAU * frequency as f32 * index as f32 / sequence.len() as f32;
                re += amplitude * theta.cos();
                im -= amplitude * theta.sin();
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
        for (index, _) in order.into_iter().take(top_bins.min(L2_FOURIER_BINS)) {
            bins[index] = (0.0, 0.0);
        }
        Self { bins }
    }
}

fn sequence_cosine_similarity(left: &[u32], right: &[u32]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let len = left.len().max(right.len());
    let mut dot = 0.0;
    let mut left_energy = 0.0;
    let mut right_energy = 0.0;
    for index in 0..len {
        let left_value = left
            .get(index)
            .map_or(0.0, |value| center_amplitude(*value));
        let right_value = right
            .get(index)
            .map_or(0.0, |value| center_amplitude(*value));
        dot += left_value * right_value;
        left_energy += left_value * left_value;
        right_energy += right_value * right_value;
    }
    if left_energy == 0.0 || right_energy == 0.0 {
        return 0.0;
    }
    dot / (left_energy.sqrt() * right_energy.sqrt())
}

fn center_amplitude(center_id: u32) -> f32 {
    if center_id == L2_MISSING_CENTER_REF {
        return 0.0;
    }
    let mixed = splitmix64(u64::from(center_id) ^ 0x4C32_414D_504C_4954);
    let sign = if (mixed >> 63) == 0 { 1.0 } else { -1.0 };
    let magnitude = 1.0 + ((mixed >> 32) & 0xff) as f32 / 512.0;
    sign * magnitude
}

const RESIDUAL_TOKEN_BIT: u32 = 1 << 31;

fn residual_token(center_id: u32) -> u32 {
    RESIDUAL_TOKEN_BIT | (center_id & !RESIDUAL_TOKEN_BIT)
}

fn is_residual_token(token: u32) -> bool {
    token & RESIDUAL_TOKEN_BIT != 0
}

fn untag_residual_token(token: u32) -> u32 {
    token & !RESIDUAL_TOKEN_BIT
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
    let mut state = 0x4C32_4354_5257_4156u64;
    for byte in bytes {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state ^ bytes.len() as u64)
}

fn stable_hash_u32s(values: &[u32]) -> u64 {
    let mut state = 0x4C32_5345_5148_4153u64;
    for value in values {
        state ^= u64::from(*value).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state ^ values.len() as u64)
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
    fn l2_center_memory_stores_l1_sequence_motifs() {
        let words = [
            "волновая",
            "волнового",
            "волновому",
            "памятная",
            "памятного",
            "памятному",
        ];
        let memory = L2CenterMemory::build(
            words.iter().copied(),
            L2CenterMemoryConfig {
                l1_config: L1CenterMemoryConfig {
                    min_center_support: 1,
                    ..L1CenterMemoryConfig::default()
                },
                min_motif_support: 1,
                motif_len: 3,
                ..L2CenterMemoryConfig::default()
            },
        );

        assert!(memory.l1().center_count() > 0);
        assert!(memory.center_count() > 0);
        assert_eq!(memory.word_records().len(), words.len());
        assert!(memory.token_refs().len() > words.len());
        assert!(memory.hot_bytes() > 0);
        let reconstructed = memory
            .reconstruct_train_l1_sequence(0)
            .expect("train record should reconstruct");
        assert_eq!(
            reconstructed.len(),
            memory.word_records()[0].l1_ref_count as usize
        );

        let assignment = memory.assign_word("волновыми");
        assert!(assignment.l1_ref_count > 0);
        assert!(assignment.covered_l1_refs > 0);
        assert!(assignment.sequence_similarity > 0.0);
        assert!(assignment.fourier_similarity > 0.0);
    }
}
