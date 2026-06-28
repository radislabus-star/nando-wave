//! L1 surface-wire compiler.
//!
//! This is deliberately below L3: it compiles raw bytes into a fixed wave
//! pattern. It does not claim roles, relations, or meaning.

pub const SURFACE_WAVE_DIM: usize = 4_096;
pub const SURFACE_WAVE_NGRAM: usize = 4;
pub const SURFACE_WAVE_TRITS: usize = 3;
pub const SURFACE_WAVE_BYTES: usize = SURFACE_WAVE_DIM * std::mem::size_of::<SurfaceWaveLane>();

pub type SurfaceWaveLane = i16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceWaveTrit {
    pub lane: u16,
    pub value: i8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceWave4096 {
    lanes: [SurfaceWaveLane; SURFACE_WAVE_DIM],
}

impl SurfaceWave4096 {
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            lanes: [0; SURFACE_WAVE_DIM],
        }
    }

    #[must_use]
    pub fn compile(text: &str) -> Self {
        Self::compile_bytes(text.as_bytes())
    }

    #[must_use]
    pub fn compile_bytes(bytes: &[u8]) -> Self {
        let mut wave = Self::zero();
        if bytes.len() < SURFACE_WAVE_NGRAM {
            return wave;
        }

        for (position, gram) in bytes.windows(SURFACE_WAVE_NGRAM).enumerate() {
            wave.add_ngram(position as u64, gram);
        }
        wave
    }

    #[must_use]
    pub fn lanes(&self) -> &[SurfaceWaveLane; SURFACE_WAVE_DIM] {
        &self.lanes
    }

    #[must_use]
    pub fn active_lanes(&self) -> usize {
        self.lanes.iter().filter(|value| **value != 0).count()
    }

    #[must_use]
    pub fn positive_lanes(&self) -> usize {
        self.lanes.iter().filter(|value| **value > 0).count()
    }

    #[must_use]
    pub fn negative_lanes(&self) -> usize {
        self.lanes.iter().filter(|value| **value < 0).count()
    }

    #[must_use]
    pub fn dot(&self, other: &Self) -> i64 {
        self.lanes
            .iter()
            .zip(other.lanes.iter())
            .map(|(left, right)| i64::from(*left) * i64::from(*right))
            .sum()
    }

    #[must_use]
    pub fn energy(&self) -> u64 {
        self.lanes
            .iter()
            .map(|value| {
                let value = i64::from(*value);
                (value * value) as u64
            })
            .sum()
    }

    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let left = self.energy();
        let right = other.energy();
        if left == 0 || right == 0 {
            return 0.0;
        }

        self.dot(other) as f32 / ((left as f32).sqrt() * (right as f32).sqrt())
    }

    fn add_ngram(&mut self, position: u64, gram: &[u8]) {
        debug_assert_eq!(gram.len(), SURFACE_WAVE_NGRAM);

        for trit in surface_ngram_projection(position, gram) {
            if trit.value == 0 {
                continue;
            }

            self.lanes[usize::from(trit.lane)] =
                self.lanes[usize::from(trit.lane)].saturating_add(i16::from(trit.value));
        }
    }
}

impl Default for SurfaceWave4096 {
    fn default() -> Self {
        Self::zero()
    }
}

#[must_use]
pub fn surface_ngram_count(text: &str) -> usize {
    text.len().saturating_sub(SURFACE_WAVE_NGRAM - 1)
}

#[must_use]
pub fn surface_ngram_projection(
    position: u64,
    gram: &[u8],
) -> [SurfaceWaveTrit; SURFACE_WAVE_TRITS] {
    debug_assert_eq!(gram.len(), SURFACE_WAVE_NGRAM);

    std::array::from_fn(|channel| {
        let position_code = match channel {
            0 => 0,
            1 => position & 0x3f,
            _ => position / 8,
        };
        let mixed = surface_mix(gram, channel as u64, position_code);
        let value = balanced_trit(mixed);
        let lane = (surface_mix(gram, channel as u64 + 17, position_code) % SURFACE_WAVE_DIM as u64)
            as u16;
        SurfaceWaveTrit { lane, value }
    })
}

fn balanced_trit(value: u64) -> i8 {
    match value % 3 {
        0 => -1,
        1 => 0,
        _ => 1,
    }
}

fn surface_mix(gram: &[u8], channel: u64, position_code: u64) -> u64 {
    let mut state =
        0x5346_5741_5645_4C31u64 ^ channel.rotate_left(19) ^ position_code.rotate_left(37);
    for byte in gram {
        state ^= u64::from(*byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        state = splitmix64(state);
    }
    splitmix64(state ^ (gram.len() as u64).rotate_left(11))
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
    use std::mem::size_of;

    #[test]
    fn surface_wave_is_fixed_l1_wire_not_l3_memory() {
        assert_eq!(SURFACE_WAVE_NGRAM, 4);
        assert_eq!(SURFACE_WAVE_DIM, 4_096);
        assert_eq!(SURFACE_WAVE_TRITS, 3);
        assert_eq!(SURFACE_WAVE_BYTES, 8_192);
        assert_eq!(size_of::<SurfaceWave4096>(), SURFACE_WAVE_BYTES);
    }

    #[test]
    fn identical_surface_compiles_to_identical_wave() {
        let left = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");
        let right = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");

        assert_eq!(left.lanes(), right.lanes());
        assert_eq!(left.cosine_similarity(&right), 1.0);
        assert!(left.active_lanes() > 0);
    }

    #[test]
    fn balanced_ternary_contributions_have_positive_negative_and_neutral_lanes() {
        let text = "https://mirror.dxdy.ru/topic3420.html";
        let wave = SurfaceWave4096::compile(text);
        let max_possible_active_writes = surface_ngram_count(text) * SURFACE_WAVE_TRITS;

        assert!(wave.positive_lanes() > 0);
        assert!(wave.negative_lanes() > 0);
        assert!(wave.active_lanes() < max_possible_active_writes);
    }

    #[test]
    fn small_surface_mutation_is_closer_than_unrelated_surface() {
        let base = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");
        let nearby = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3421.html");
        let unrelated = SurfaceWave4096::compile("ssh service externally exposed firewall");

        let nearby_score = base.cosine_similarity(&nearby);
        let unrelated_score = base.cosine_similarity(&unrelated);

        assert!(nearby_score > 0.80, "nearby_score={nearby_score}");
        assert!(
            unrelated_score < nearby_score - 0.35,
            "nearby_score={nearby_score} unrelated_score={unrelated_score}"
        );
    }

    #[test]
    fn position_channels_penalize_reordered_surface() {
        let base = SurfaceWave4096::compile("abcdefg12345");
        let reordered = SurfaceWave4096::compile("12345abcdefg");
        let mutation = SurfaceWave4096::compile("abcdefg12346");

        let reorder_score = base.cosine_similarity(&reordered);
        let mutation_score = base.cosine_similarity(&mutation);

        assert!(
            reorder_score < mutation_score,
            "reorder_score={reorder_score} mutation_score={mutation_score}"
        );
    }

    #[test]
    fn shorter_than_one_ngram_is_empty() {
        let wave = SurfaceWave4096::compile("abc");

        assert_eq!(surface_ngram_count("abc"), 0);
        assert_eq!(wave.energy(), 0);
        assert_eq!(wave.active_lanes(), 0);
    }
}
