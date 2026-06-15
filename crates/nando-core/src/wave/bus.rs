use std::f32::consts::TAU;

use super::{CarrierWave, Cell32, PHASE_SLOTS, normalized_entropy};

/// Wave-bus state for one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveBus {
    pub phase_sum: [f32; PHASE_SLOTS],
    pub amplitude_sum: [f32; PHASE_SLOTS],
    pub coherence: f32,
    pub spectral_entropy: f32,
    pub center_phase: f32,
    pub center_magnitude: f32,
}

impl Default for WaveBus {
    fn default() -> Self {
        Self {
            phase_sum: [0.0; PHASE_SLOTS],
            amplitude_sum: [0.0; PHASE_SLOTS],
            coherence: 0.0,
            spectral_entropy: 1.0,
            center_phase: 0.0,
            center_magnitude: 0.0,
        }
    }
}

impl WaveBus {
    /// Add one active cell to the bus through harmonic superposition.
    pub fn add_cell(&mut self, cell: &Cell32, carrier: CarrierWave) {
        let envelope = carrier.envelope();
        let carrier_sin = carrier.phase.sin();
        let carrier_cos = carrier.phase.cos();

        for ((((phase_acc, amp_acc), cell_sin), cell_cos), amplitude) in self
            .phase_sum
            .iter_mut()
            .zip(self.amplitude_sum.iter_mut())
            .zip(cell.phase_sin.iter())
            .zip(cell.phase_cos.iter())
            .zip(cell.amplitudes.iter())
        {
            let effective_amplitude = amplitude * envelope;
            let shifted_cos = cell_cos.mul_add(carrier_cos, -(cell_sin * carrier_sin));
            *phase_acc += effective_amplitude * shifted_cos;
            *amp_acc += effective_amplitude.abs();
        }
    }

    /// Finalize bus metrics after active cells have been added.
    pub fn finish_metrics(&mut self) {
        let total_amplitude: f32 = self.amplitude_sum.iter().sum();
        if total_amplitude <= f32::EPSILON {
            self.coherence = 0.0;
            self.spectral_entropy = 1.0;
            self.center_phase = 0.0;
            self.center_magnitude = 0.0;
            return;
        }

        let phase_energy: f32 = self.phase_sum.iter().map(|value| value.abs()).sum();
        self.coherence = (phase_energy / total_amplitude).clamp(0.0, 1.0);
        self.spectral_entropy = normalized_entropy(&self.amplitude_sum, total_amplitude);

        let mut x = 0.0;
        let mut y = 0.0;
        for (slot, phase_value) in self.phase_sum.iter().enumerate() {
            let weight = phase_value.abs();
            let angle = TAU * slot as f32 / PHASE_SLOTS as f32;
            x += weight * angle.cos();
            y += weight * angle.sin();
        }

        let magnitude = (x.mul_add(x, y * y)).sqrt();
        self.center_phase = y.atan2(x).rem_euclid(TAU);
        self.center_magnitude = (magnitude / phase_energy.max(f32::EPSILON)).clamp(0.0, 1.0);
    }
}
