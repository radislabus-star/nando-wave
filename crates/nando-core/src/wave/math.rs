use std::f32::consts::TAU;

use super::{PHASE_SLOTS, SNAPSHOT_TOP_SLOTS, STAGE2_TOP_K};

pub(crate) fn insert_top_index(
    index: usize,
    score: f32,
    top_indices: &mut [usize; STAGE2_TOP_K],
    top_scores: &mut [f32; STAGE2_TOP_K],
) {
    for position in 0..STAGE2_TOP_K {
        if score <= top_scores[position] {
            continue;
        }

        for shift in (position + 1..STAGE2_TOP_K).rev() {
            top_scores[shift] = top_scores[shift - 1];
            top_indices[shift] = top_indices[shift - 1];
        }

        top_scores[position] = score;
        top_indices[position] = index;
        break;
    }
}

pub(crate) fn insert_top_slot(
    slot: u16,
    score: f32,
    top_slots: &mut [u16; SNAPSHOT_TOP_SLOTS],
    top_scores: &mut [f32; SNAPSHOT_TOP_SLOTS],
) {
    for position in 0..SNAPSHOT_TOP_SLOTS {
        if score <= top_scores[position] {
            continue;
        }

        for shift in (position + 1..SNAPSHOT_TOP_SLOTS).rev() {
            top_scores[shift] = top_scores[shift - 1];
            top_slots[shift] = top_slots[shift - 1];
        }

        top_scores[position] = score;
        top_slots[position] = slot;
        break;
    }
}

pub(crate) fn normalized_entropy(values: &[f32; PHASE_SLOTS], total: f32) -> f32 {
    let mut entropy = 0.0;

    for value in values {
        if *value <= f32::EPSILON {
            continue;
        }
        let probability = value / total;
        entropy -= probability * probability.ln();
    }

    (entropy / (PHASE_SLOTS as f32).ln()).clamp(0.0, 1.0)
}

pub(crate) fn circular_phase_delta(from: f32, to: f32) -> f32 {
    let mut delta = (to - from).rem_euclid(TAU);
    if delta > std::f32::consts::PI {
        delta -= TAU;
    }
    delta
}

pub(crate) fn unit_noise(seed: u64, cell_id: u64, slot: u64, salt: u64) -> f32 {
    let mixed = splitmix64(
        seed ^ cell_id.rotate_left(17) ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt,
    );
    let bits = (mixed >> 40) as u32;
    bits as f32 / 16_777_216.0
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
