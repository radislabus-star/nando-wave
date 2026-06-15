use std::f32::consts::TAU;

use super::{
    CELL32_RESERVED_BYTES, CarrierWave, MONO_PHASE_SLOTS, MONO_RESERVED_BYTES, PHASE_SLOTS,
    unit_noise,
};

/// First coarse role split for the six-cell organism.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellRank {
    Fast = 0,
    Mid = 1,
    CarrierAnchor = 2,
    Guard = 3,
}

/// A fixed 32 KB wave cell.
///
/// The large reserved area is intentional. Stage 2 only proves fixed memory,
/// deterministic resonance, and snapshot roundtrip. Later stages can allocate
/// more of the packet without changing the public atom size.
#[repr(C)]
#[derive(Clone)]
pub struct Cell32 {
    pub id: u32,
    pub rank: CellRank,
    pub flags: u8,
    header_reserved: [u8; 2],
    pub last_resonance: f32,
    pub age_ticks: u32,
    pub frequencies: [f32; PHASE_SLOTS],
    pub phases: [f32; PHASE_SLOTS],
    pub phase_sin: [f32; PHASE_SLOTS],
    pub phase_cos: [f32; PHASE_SLOTS],
    pub amplitudes: [f32; PHASE_SLOTS],
    pub decay: [f32; PHASE_SLOTS],
    reserved: [u8; CELL32_RESERVED_BYTES],
}

impl Cell32 {
    /// Create a deterministic cell packet for repeatable experiments.
    #[must_use]
    pub fn new(id: u32, rank: CellRank, seed: u64) -> Self {
        let mut frequencies = [0.0; PHASE_SLOTS];
        let mut phases = [0.0; PHASE_SLOTS];
        let mut phase_sin = [0.0; PHASE_SLOTS];
        let mut phase_cos = [0.0; PHASE_SLOTS];
        let mut amplitudes = [0.0; PHASE_SLOTS];
        let mut decay = [0.0; PHASE_SLOTS];
        let rank_offset = rank as u64 + 1;

        for (slot, (((((frequency, phase), phase_sin), phase_cos), amplitude), decay_value)) in
            frequencies
                .iter_mut()
                .zip(phases.iter_mut())
                .zip(phase_sin.iter_mut())
                .zip(phase_cos.iter_mut())
                .zip(amplitudes.iter_mut())
                .zip(decay.iter_mut())
                .enumerate()
        {
            let slot_u64 = slot as u64;
            let unit_a = unit_noise(seed, id as u64, slot_u64, 0xA11C_E032);
            let unit_b = unit_noise(seed, id as u64, slot_u64, 0xC0DE_5EED);
            let unit_c = unit_noise(seed, id as u64, slot_u64, 0xBADC_AB1E);
            *frequency = 1.0 + ((slot % 32) as f32) + rank_offset as f32 * 0.125;
            *phase = (unit_a * TAU + rank_offset as f32 * 0.07).rem_euclid(TAU);
            *phase_sin = phase.sin();
            *phase_cos = phase.cos();
            *amplitude = 0.25 + unit_b * 0.75;
            *decay_value = 0.990 + unit_c * 0.009;
        }

        Self {
            id,
            rank,
            flags: 0,
            header_reserved: [0; 2],
            last_resonance: 0.0,
            age_ticks: 0,
            frequencies,
            phases,
            phase_sin,
            phase_cos,
            amplitudes,
            decay,
            reserved: [0; CELL32_RESERVED_BYTES],
        }
    }

    /// Compute resonance against an encoded input and a carrier without learning.
    pub fn compute_resonance(
        &mut self,
        input_phases: &[f32; PHASE_SLOTS],
        carrier: CarrierWave,
    ) -> f32 {
        let resonance = self.resonance_score(input_phases, carrier);
        self.last_resonance = resonance;
        self.age_ticks = self.age_ticks.saturating_add(1);
        resonance
    }

    /// Compute resonance without mutating the cell packet.
    ///
    /// This is the hot path for precomputed organisms: one `Stage2Organ` can be
    /// reused across many ticks without rebuilding or cloning all cells.
    #[must_use]
    pub fn resonance_score(&self, input_phases: &[f32; PHASE_SLOTS], carrier: CarrierWave) -> f32 {
        let carrier_phase = carrier.phase;
        let envelope = carrier.envelope();
        let mut input_sin = [0.0; PHASE_SLOTS];
        let mut input_cos = [0.0; PHASE_SLOTS];

        for ((sin_value, cos_value), input_phase) in input_sin
            .iter_mut()
            .zip(input_cos.iter_mut())
            .zip(input_phases.iter())
        {
            let phase = input_phase + carrier_phase;
            *sin_value = phase.sin();
            *cos_value = phase.cos();
        }

        self.resonance_score_with_carrier_trig(&input_sin, &input_cos, envelope)
    }

    /// Compute resonance from precomputed input+carrier trigonometric vectors.
    ///
    /// Hot loops should prefer this path: the input sine/cosine vectors are
    /// built once per tick, while each cell only does multiply-add work.
    #[must_use]
    pub fn resonance_score_with_carrier_trig(
        &self,
        input_carrier_sin: &[f32; PHASE_SLOTS],
        input_carrier_cos: &[f32; PHASE_SLOTS],
        envelope: f32,
    ) -> f32 {
        let mut sum = 0.0;

        for (((input_sin, input_cos), cell_sin), (cell_cos, amplitude)) in input_carrier_sin
            .iter()
            .zip(input_carrier_cos.iter())
            .zip(self.phase_sin.iter())
            .zip(self.phase_cos.iter().zip(self.amplitudes.iter()))
        {
            let aligned = input_cos.mul_add(*cell_cos, input_sin * *cell_sin);
            sum += *amplitude * aligned * envelope;
        }

        sum / PHASE_SLOTS as f32
    }
}

/// A single 192 KB monolith control packet.
#[repr(C)]
#[derive(Clone)]
pub struct Mono192 {
    pub seed: u64,
    pub last_resonance: f32,
    header_reserved: [u8; 4],
    pub frequencies: [f32; MONO_PHASE_SLOTS],
    pub phases: [f32; MONO_PHASE_SLOTS],
    pub amplitudes: [f32; MONO_PHASE_SLOTS],
    reserved: [u8; MONO_RESERVED_BYTES],
}

impl Mono192 {
    /// Create a deterministic monolith packet. Its actual baseline behavior is
    /// introduced in later stages.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            last_resonance: 0.0,
            header_reserved: [0; 4],
            frequencies: [0.0; MONO_PHASE_SLOTS],
            phases: [0.0; MONO_PHASE_SLOTS],
            amplitudes: [0.0; MONO_PHASE_SLOTS],
            reserved: [0; MONO_RESERVED_BYTES],
        }
    }
}
