//! L1 surface-wire compiler.
//!
//! This is deliberately below L3: it compiles surface form into a fixed wave
//! pattern. It does not claim roles, relations, or meaning.

pub const SURFACE_WAVE_DIM: usize = 4_096;
pub const SURFACE_WAVE_NGRAM: usize = 4;
pub const SURFACE_WAVE_TRITS: usize = 3;
pub const SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS: usize = 4;
pub const SURFACE_WAVE_BYTES: usize = SURFACE_WAVE_DIM * std::mem::size_of::<SurfaceWaveLane>();

pub type SurfaceWaveLane = i16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceAtom {
    pub position: u64,
    pub bytes: Vec<u8>,
}

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
        let mut wave = Self::zero();
        for atom in surface_atoms(text) {
            wave.add_atom(atom.position, &atom.bytes);
        }
        wave
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
            self.add_trit(trit);
        }
    }

    fn add_atom(&mut self, position: u64, atom: &[u8]) {
        for trit in surface_atom_projection(position, atom) {
            self.add_trit(trit);
        }
    }

    fn add_trit(&mut self, trit: SurfaceWaveTrit) {
        if trit.value == 0 {
            return;
        }

        self.lanes[usize::from(trit.lane)] =
            self.lanes[usize::from(trit.lane)].saturating_add(i16::from(trit.value));
    }
}

impl Default for SurfaceWave4096 {
    fn default() -> Self {
        Self::zero()
    }
}

#[must_use]
pub fn surface_atoms(text: &str) -> Vec<SurfaceAtom> {
    let mut atoms = raw_byte_atoms(text.as_bytes());
    append_boundary_atoms(text, &mut atoms);
    atoms
}

fn raw_byte_atoms(bytes: &[u8]) -> Vec<SurfaceAtom> {
    if bytes.len() < SURFACE_WAVE_NGRAM {
        return Vec::new();
    }

    bytes
        .windows(SURFACE_WAVE_NGRAM)
        .enumerate()
        .map(|(position, gram)| SurfaceAtom {
            position: position as u64,
            bytes: gram.to_vec(),
        })
        .collect()
}

fn append_boundary_atoms(text: &str, atoms: &mut Vec<SurfaceAtom>) {
    for raw_token in text.split_whitespace() {
        let chars = lower_token_chars(raw_token);
        if chars.is_empty() {
            continue;
        }

        let mut padded = Vec::with_capacity(chars.len() + 2 * (SURFACE_WAVE_NGRAM - 1));
        padded.extend(std::iter::repeat_n(
            BoundarySlot::Begin,
            SURFACE_WAVE_NGRAM - 1,
        ));
        padded.extend(chars.into_iter().map(BoundarySlot::Text));
        padded.extend(std::iter::repeat_n(
            BoundarySlot::End,
            SURFACE_WAVE_NGRAM - 1,
        ));

        for (local_position, window) in padded.windows(SURFACE_WAVE_NGRAM).enumerate() {
            if !window
                .iter()
                .any(|slot| matches!(slot, BoundarySlot::Begin | BoundarySlot::End))
            {
                continue;
            }
            atoms.push(SurfaceAtom {
                position: local_position as u64,
                bytes: encode_boundary_atom(window),
            });
        }

        let service_token = normalize_service_token(raw_token);
        append_short_token_identity_atoms(&service_token, atoms);
        if is_service_word(&service_token) {
            atoms.push(SurfaceAtom {
                position: 0,
                bytes: encode_service_atom(&service_token),
            });
        }
    }
}

fn append_short_token_identity_atoms(token: &str, atoms: &mut Vec<SurfaceAtom>) {
    if token.is_empty() || token.chars().count() >= SURFACE_WAVE_NGRAM {
        return;
    }

    for salt in 0..SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS {
        atoms.push(SurfaceAtom {
            position: 0,
            bytes: encode_short_token_identity_atom(token, salt as u8),
        });
    }
}

fn lower_token_chars(token: &str) -> Vec<char> {
    token
        .chars()
        .filter(|ch| !ch.is_control())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_service_token(token: &str) -> String {
    token
        .chars()
        .filter(|ch| ch.is_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_service_word(token: &str) -> bool {
    matches!(
        token,
        "и" | "а"
            | "но"
            | "или"
            | "в"
            | "во"
            | "на"
            | "к"
            | "ко"
            | "с"
            | "со"
            | "из"
            | "у"
            | "о"
            | "об"
            | "от"
            | "до"
            | "по"
            | "за"
            | "над"
            | "под"
            | "при"
            | "для"
            | "без"
            | "не"
            | "ни"
            | "да"
            | "же"
            | "ли"
            | "бы"
            | "то"
            | "это"
            | "как"
            | "что"
            | "где"
            | "кто"
            | "мы"
            | "я"
            | "ты"
            | "он"
            | "она"
            | "они"
            | "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "not"
            | "no"
            | "to"
            | "of"
            | "in"
            | "on"
            | "at"
            | "by"
            | "for"
            | "from"
            | "with"
            | "as"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "do"
            | "does"
            | "did"
            | "if"
            | "then"
            | "than"
            | "that"
            | "this"
            | "it"
            | "we"
            | "you"
            | "he"
            | "she"
            | "they"
    )
}

#[derive(Clone, Copy, Debug)]
enum BoundarySlot {
    Begin,
    End,
    Text(char),
}

fn encode_boundary_atom(slots: &[BoundarySlot]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(b"\x1Fbd4\0");
    for slot in slots {
        match slot {
            BoundarySlot::Begin => bytes.push(0x01),
            BoundarySlot::End => bytes.push(0x02),
            BoundarySlot::Text(ch) => {
                let mut buffer = [0u8; 4];
                let encoded = ch.encode_utf8(&mut buffer);
                bytes.push(0x10);
                bytes.push(encoded.len() as u8);
                bytes.extend_from_slice(encoded.as_bytes());
            }
        }
    }
    bytes
}

fn encode_service_atom(token: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(8 + token.len());
    bytes.extend_from_slice(b"\x1Fsvc\0");
    bytes.extend_from_slice(token.as_bytes());
    bytes
}

fn encode_short_token_identity_atom(token: &str, salt: u8) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(10 + token.len());
    bytes.extend_from_slice(b"\x1Fst0\0");
    bytes.push(salt);
    bytes.extend_from_slice(token.as_bytes());
    bytes
}

#[must_use]
pub fn surface_ngram_count(text: &str) -> usize {
    surface_atoms(text).len()
}

#[must_use]
pub fn surface_atom_projection(
    position: u64,
    atom: &[u8],
) -> [SurfaceWaveTrit; SURFACE_WAVE_TRITS] {
    std::array::from_fn(|channel| {
        let position_code = match channel {
            0 => 0,
            1 => position & 0x3f,
            _ => position / 8,
        };
        let mixed = surface_mix(atom, channel as u64, position_code);
        let value = balanced_trit(mixed);
        let lane = (surface_mix(atom, channel as u64 + 17, position_code) % SURFACE_WAVE_DIM as u64)
            as u16;
        SurfaceWaveTrit { lane, value }
    })
}

#[must_use]
pub fn surface_ngram_projection(
    position: u64,
    gram: &[u8],
) -> [SurfaceWaveTrit; SURFACE_WAVE_TRITS] {
    debug_assert_eq!(gram.len(), SURFACE_WAVE_NGRAM);
    surface_atom_projection(position, gram)
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
        assert_eq!(SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS, 4);
        assert_eq!(SURFACE_WAVE_BYTES, 8_192);
        assert_eq!(size_of::<SurfaceWave4096>(), SURFACE_WAVE_BYTES);
    }

    #[test]
    fn identical_surface_compiles_to_identical_wave() {
        let left = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");
        let right = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");

        assert_eq!(left.lanes(), right.lanes());
        assert!(
            (left.cosine_similarity(&right) - 1.0).abs() < f32::EPSILON * 2.0,
            "identical waves should have cosine 1"
        );
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
    fn short_unicode_words_have_boundary_wave_without_service_confusion() {
        let service = SurfaceWave4096::compile("и");
        let owl = SurfaceWave4096::compile("сыч");

        assert!(surface_ngram_count("и") > 0);
        assert!(surface_ngram_count("сыч") > 0);
        assert!(service.energy() > 0);
        assert!(owl.energy() > 0);
        assert!(
            service.cosine_similarity(&owl) < 0.90,
            "service/short-content words should not collapse into one center"
        );
    }

    #[test]
    fn short_symbol_tokens_get_identity_support_without_l3_labels() {
        let token = SurfaceWave4096::compile("Z12");
        let nearby = SurfaceWave4096::compile("Z13");
        let shorter = SurfaceWave4096::compile("Z3");

        assert!(
            surface_ngram_count("Z12")
                >= (SURFACE_WAVE_NGRAM + SURFACE_WAVE_SHORT_TOKEN_IDENTITY_ATOMS),
            "short token should receive boundary atoms plus generic identity atoms"
        );
        assert!(
            token.active_lanes() > shorter.active_lanes() / 2,
            "short-token identity support should not collapse to near-zero"
        );
        assert!(
            token.cosine_similarity(&shorter) < 0.55,
            "different short symbolic fillers should remain separable"
        );
        assert!(
            token.cosine_similarity(&nearby) > token.cosine_similarity(&shorter),
            "nearby numbered fillers should stay closer than shorter unrelated fillers"
        );
    }

    #[test]
    fn real_dollar_text_is_not_end_boundary_marker() {
        let price = SurfaceWave4096::compile("цена$");
        let plain = SurfaceWave4096::compile("цена");

        assert!(price.energy() > 0);
        assert!(plain.energy() > 0);
        assert!(
            price.cosine_similarity(&plain) < 1.0,
            "TEXT('$') must not be encoded as EOS"
        );
    }

    #[test]
    fn negation_function_word_changes_surface_wave() {
        let positive = SurfaceWave4096::compile("работает");
        let negated = SurfaceWave4096::compile("не работает");

        assert!(negated.active_lanes() > positive.active_lanes());
        assert!(
            positive.cosine_similarity(&negated) < 0.95,
            "service negation should be visible to L1"
        );
    }
}
