pub(crate) fn prompt_hash(seed: u64, prompt: &[u8]) -> f32 {
    let mut hash = seed ^ 0xCBF2_9CE4_8422_2325;
    for byte in prompt {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01B3);
    }
    let unit = splitmix64(hash) as f32 / u64::MAX as f32;
    (unit * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU)
}

pub(crate) fn byte_to_phase(byte: u8) -> f32 {
    ((f32::from(byte) + 0.5) / 256.0 * std::f32::consts::TAU).rem_euclid(std::f32::consts::TAU)
}

pub(crate) fn circular_delta(from: f32, to: f32) -> f32 {
    let delta = to - from;
    delta.sin().atan2(delta.cos())
}

pub(crate) fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
