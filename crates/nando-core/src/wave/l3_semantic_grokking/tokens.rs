use std::collections::HashSet;

use super::super::L2CenterMemory;
use super::{L3CueTokenMode, L3SemanticExample};

const L3_CUE_PAIR_TOKEN_FLAG: u32 = 1 << 31;
const L3_SURFACE_RESIDUAL_TOKEN_FLAG: u32 = 1 << 30;
const L3_CUE_TOKEN_VALUE_MASK: u32 = !(L3_CUE_PAIR_TOKEN_FLAG | L3_SURFACE_RESIDUAL_TOKEN_FLAG);

pub(super) fn motif_tokens(l2: &L2CenterMemory, text: &str) -> Vec<u32> {
    l2.token_sequence_for_text(text)
        .tokens
        .into_iter()
        .filter(|token| token & (1 << 31) == 0)
        .collect()
}

pub(super) fn cue_tokens(l2: &L2CenterMemory, text: &str) -> Vec<u32> {
    cue_tokens_with_mode(l2, text, L3CueTokenMode::All)
}

pub(super) fn cue_tokens_with_mode(
    l2: &L2CenterMemory,
    text: &str,
    mode: L3CueTokenMode,
) -> Vec<u32> {
    let base = motif_tokens(l2, text);
    let mut tokens = Vec::new();

    if mode != L3CueTokenMode::SurfaceResidualOnly {
        tokens.extend(base.iter().copied());
    }

    if !matches!(
        mode,
        L3CueTokenMode::WithoutMotifPairs | L3CueTokenMode::SurfaceResidualOnly
    ) {
        for (left_index, left) in base.iter().enumerate() {
            for right in base.iter().skip(left_index + 1).take(4) {
                tokens.push(cue_pair_token(*left, *right));
            }
        }
    }

    if mode != L3CueTokenMode::WithoutSurfaceResidual {
        let surface_tokens = normalized_tokens(text)
            .into_iter()
            .map(|token| normalize_digits(&token))
            .collect::<Vec<_>>();
        for token in &surface_tokens {
            tokens.push(cue_surface_token("word", token));
        }
        for window in surface_tokens.windows(2) {
            tokens.push(cue_surface_token(
                "bigram",
                &format!("{}|{}", window[0], window[1]),
            ));
        }
    }

    tokens.sort_unstable();
    tokens.dedup();
    tokens
}

fn cue_pair_token(left: u32, right: u32) -> u32 {
    let mut value = u64::from(left) ^ u64::from(right).rotate_left(21);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    L3_CUE_PAIR_TOKEN_FLAG | ((value ^ (value >> 31)) as u32 & L3_CUE_TOKEN_VALUE_MASK)
}

fn cue_surface_token(kind: &str, value: &str) -> u32 {
    let mut hash = 0xC6A4_A793_5BD1_E995u64;
    for byte in kind.bytes().chain([b':']).chain(value.bytes()) {
        hash ^= u64::from(byte).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        hash = hash.rotate_left(27);
    }
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    L3_CUE_PAIR_TOKEN_FLAG
        | L3_SURFACE_RESIDUAL_TOKEN_FLAG
        | ((hash ^ (hash >> 31)) as u32 & L3_CUE_TOKEN_VALUE_MASK)
}

fn normalize_digits(token: &str) -> String {
    token
        .chars()
        .map(|ch| if ch.is_ascii_digit() { '0' } else { ch })
        .collect()
}

pub(super) fn normalized_surface_key(text: &str) -> String {
    normalized_tokens(text)
        .into_iter()
        .map(|token| normalize_digits(&token))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn normalized_bigrams(text: &str) -> HashSet<String> {
    normalized_tokens(text)
        .into_iter()
        .map(|token| normalize_digits(&token))
        .collect::<Vec<_>>()
        .windows(2)
        .map(|window| format!("{}|{}", window[0], window[1]))
        .collect()
}

pub(super) fn normalized_bigram_index(examples: &[L3SemanticExample]) -> HashSet<String> {
    examples
        .iter()
        .flat_map(|example| normalized_bigrams(&example.query_surface))
        .collect()
}

pub(super) fn normalized_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| matches!(ch, '?' | '.' | ',' | ';' | ':'))
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect()
}
