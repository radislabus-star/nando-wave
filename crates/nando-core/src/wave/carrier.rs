use std::f32::consts::TAU;

use super::unit_noise;

/// Explicit slow carrier wave. It is part of state, not a hidden bias.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarrierWave {
    pub phase: f32,
    pub amplitude: f32,
    pub frequency: f32,
    pub boundary: f32,
}

impl CarrierWave {
    /// Deterministic carrier for the first wave tick.
    #[must_use]
    pub fn from_seed(seed: u64, input_byte: u8) -> Self {
        let base = input_byte as f32 / 255.0;
        let phase = (unit_noise(seed, input_byte as u64, 0, 0xCA22_1E22) * TAU).rem_euclid(TAU);
        Self {
            phase,
            amplitude: 0.60 + base * 0.25,
            frequency: 1.0 + base,
            boundary: 0.75,
        }
    }

    /// Current amplitude envelope applied to lower cells.
    #[must_use]
    pub fn envelope(self) -> f32 {
        (self.amplitude * self.boundary).clamp(0.0, 1.0)
    }

    /// Evolve this carrier as a slow state variable for transition tests.
    #[must_use]
    pub fn advance(self, input_byte: u8, step: u32) -> Self {
        let input_unit = input_byte as f32 / 255.0;
        let step_unit = step as f32;
        let phase_delta = TAU * (0.021 + self.frequency * 0.003 + input_unit * 0.002);
        let amplitude_target = 0.60 + input_unit * 0.25;
        let frequency_target = 1.0 + input_unit;

        Self {
            phase: (self.phase + phase_delta * step_unit).rem_euclid(TAU),
            amplitude: self.amplitude * 0.92 + amplitude_target * 0.08,
            frequency: self.frequency * 0.90 + frequency_target * 0.10,
            boundary: self.boundary,
        }
    }
}
