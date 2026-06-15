use std::f32::consts::TAU;

use super::{
    BytePhaseLut, CarrierWave, Cell32, CellRank, PHASE_SLOTS, STAGE2_ORGAN_CELLS, STAGE2_TOP_K,
    Stage2BusTraceTick, Stage2Tick, TickTrace, WaveBus, circular_phase_delta,
    run_stage2_bus_trace_with_organ_state, run_stage2_tick_with_organ_lut_state,
    run_stage2_tick_with_state,
};

/// Precomputed six-cell Stage 2 organism.
///
/// Constructing all cells is intentionally deterministic but expensive enough
/// to keep out of repeated tick loops. Reuse this packet when many ticks share
/// one seed.
#[derive(Clone)]
pub struct Stage2Organ {
    pub seed: u64,
    cells: [Cell32; STAGE2_ORGAN_CELLS],
}

impl Stage2Organ {
    /// Build the first six-cell organism once for a seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            cells: [
                Cell32::new(0, CellRank::Fast, seed),
                Cell32::new(1, CellRank::Fast, seed),
                Cell32::new(2, CellRank::Mid, seed),
                Cell32::new(3, CellRank::Mid, seed),
                Cell32::new(4, CellRank::CarrierAnchor, seed),
                Cell32::new(5, CellRank::Guard, seed),
            ],
        }
    }

    /// Borrow fixed cells without rebuilding them.
    #[must_use]
    pub fn cells(&self) -> &[Cell32; STAGE2_ORGAN_CELLS] {
        &self.cells
    }
}

/// Runtime coupling state for a six-cell Organ192 settle loop.
///
/// This does not change the fixed Cell32 packet or the snapshot format. It is a
/// small hot runtime wrapper that lets later ticks depend on earlier bus state.
#[derive(Debug, Clone, PartialEq)]
pub struct OrganState {
    pub seed: u64,
    pub carrier: CarrierWave,
    pub tick_index: u32,
    pub previous_center_phase: f32,
    pub previous_coherence: f32,
    pub previous_entropy: f32,
    pub cell_coupling: [f32; STAGE2_ORGAN_CELLS],
    pub previous_phase_sum: [f32; PHASE_SLOTS],
    pub previous_amplitude_sum: [f32; PHASE_SLOTS],
    pub link_phase_sum: [f32; PHASE_SLOTS],
    pub link_amplitude_sum: [f32; PHASE_SLOTS],
}

/// Primitive byte prediction made from the current wave center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LivePrediction {
    pub predicted_byte: u8,
    pub confidence: f32,
}

/// Local update summary for one online feedback step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalUpdateReport {
    pub target_byte: u8,
    pub reward: f32,
    pub correct: bool,
    pub phase_error: f32,
    pub carrier_pull: f32,
    pub coupling_mean: f32,
}

/// One complete live loop: state -> tick -> feedback -> local update -> snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveCycle {
    pub tick: Stage2Tick,
    pub prediction: LivePrediction,
    pub update: LocalUpdateReport,
}

impl OrganState {
    /// Create an Organ192 runtime state seeded by the first byte of a prompt.
    #[must_use]
    pub fn new(seed: u64, first_input_byte: u8) -> Self {
        Self {
            seed,
            carrier: CarrierWave::from_seed(seed, first_input_byte),
            tick_index: 0,
            previous_center_phase: 0.0,
            previous_coherence: 0.0,
            previous_entropy: 1.0,
            cell_coupling: [0.0; STAGE2_ORGAN_CELLS],
            previous_phase_sum: [0.0; PHASE_SLOTS],
            previous_amplitude_sum: [0.0; PHASE_SLOTS],
            link_phase_sum: [0.0; PHASE_SLOTS],
            link_amplitude_sum: [0.0; PHASE_SLOTS],
        }
    }

    /// Advance the slow carrier and phase-lock it to the previous bus center.
    #[must_use]
    pub fn next_locked_carrier(&mut self, input_byte: u8) -> CarrierWave {
        let mut carrier = self.carrier.advance(input_byte, 1);
        if self.tick_index > 0 {
            let pull = circular_phase_delta(carrier.phase, self.previous_center_phase);
            carrier.phase = (carrier.phase + pull * 0.04).rem_euclid(TAU);
            carrier.amplitude = (carrier.amplitude * 0.88
                + (0.55 + self.previous_coherence * 0.35) * 0.12)
                .clamp(0.05, 1.0);
            carrier.boundary = (carrier.boundary * 0.90
                + (0.68 + (1.0 - self.previous_entropy) * 0.22) * 0.10)
                .clamp(0.10, 1.0);
        }
        self.carrier = carrier;
        carrier
    }

    /// Run one stateful settle tick with the internally locked carrier.
    pub fn settle_tick(&mut self, input_byte: u8, disabled_cell_id: Option<u32>) -> Stage2Tick {
        let carrier = self.next_locked_carrier(input_byte);
        self.settle_tick_with_carrier(input_byte, carrier, disabled_cell_id)
    }

    /// Run one stateful settle tick with an explicit carrier control.
    pub fn settle_tick_with_carrier(
        &mut self,
        input_byte: u8,
        carrier: CarrierWave,
        disabled_cell_id: Option<u32>,
    ) -> Stage2Tick {
        self.carrier = carrier;
        let tick = run_stage2_tick_with_state(
            self.seed,
            input_byte,
            carrier,
            disabled_cell_id,
            &self.cell_coupling,
        );
        self.update_from_trace(&tick.trace);
        tick
    }

    /// Run one stateful settle tick and retain bus-to-bus link state.
    ///
    /// This is the runtime counterpart of the eval-only component-link probe:
    /// it records how the previous bus and current bus interact without
    /// changing fixed Cell32 packets or using task labels.
    pub fn settle_bus_tick_with_carrier(
        &mut self,
        organ: &Stage2Organ,
        input_byte: u8,
        carrier: CarrierWave,
        disabled_cell_id: Option<u32>,
    ) -> Stage2BusTraceTick {
        self.carrier = carrier;
        let tick = run_stage2_bus_trace_with_organ_state(
            organ,
            input_byte,
            carrier,
            disabled_cell_id,
            &self.cell_coupling,
        );
        self.update_link_from_bus(&tick.bus);
        self.update_from_trace(&tick.trace);
        tick
    }

    /// Run one primitive online-learning cycle against a known next byte.
    ///
    /// This updates only runtime coupling and carrier state. Fixed `Cell32`
    /// packets remain stable, so benchmarks and snapshots stay comparable
    /// while the live loop becomes testable.
    pub fn live_cycle(
        &mut self,
        organ: &Stage2Organ,
        lut: &BytePhaseLut,
        input_byte: u8,
        target_byte: u8,
    ) -> LiveCycle {
        let carrier = self.next_locked_carrier(input_byte);
        let tick = run_stage2_tick_with_organ_lut_state(
            organ,
            lut,
            input_byte,
            carrier,
            None,
            &self.cell_coupling,
        );
        let prediction = prediction_from_trace(&tick.trace);
        let update = self.apply_feedback(&tick, prediction, target_byte);

        LiveCycle {
            tick,
            prediction,
            update,
        }
    }

    /// Mean runtime coupling after local feedback.
    #[must_use]
    pub fn coupling_mean(&self) -> f32 {
        self.cell_coupling.iter().sum::<f32>() / STAGE2_ORGAN_CELLS as f32
    }

    fn apply_feedback(
        &mut self,
        tick: &Stage2Tick,
        prediction: LivePrediction,
        target_byte: u8,
    ) -> LocalUpdateReport {
        let target_phase = byte_phase(target_byte);
        let predicted_phase = byte_phase(prediction.predicted_byte);
        let phase_error = circular_phase_delta(predicted_phase, target_phase);
        let phase_alignment = (1.0 - phase_error.abs() / std::f32::consts::PI).clamp(0.0, 1.0);
        let correct = prediction.predicted_byte == target_byte;
        let reward = if correct {
            1.0
        } else {
            phase_alignment.mul_add(2.0, -1.0)
        };
        let carrier_pull = circular_phase_delta(self.carrier.phase, target_phase) * 0.06 * reward;

        for coupling in &mut self.cell_coupling {
            *coupling *= 0.88;
        }

        for (rank, cell_id) in tick.trace.active_cell_ids.iter().copied().enumerate() {
            let index = cell_id as usize;
            if index >= STAGE2_ORGAN_CELLS {
                continue;
            }
            let rank_gain = (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
            let delta = reward * prediction.confidence * rank_gain * 0.16;
            self.cell_coupling[index] = (self.cell_coupling[index] + delta).clamp(-1.0, 1.0);
        }

        self.carrier.phase = (self.carrier.phase + carrier_pull).rem_euclid(TAU);
        self.previous_center_phase = tick.trace.center_phase;
        self.previous_coherence = tick.trace.coherence;
        self.previous_entropy = tick.trace.spectral_entropy;
        self.tick_index = self.tick_index.saturating_add(1);

        LocalUpdateReport {
            target_byte,
            reward,
            correct,
            phase_error,
            carrier_pull,
            coupling_mean: self.coupling_mean(),
        }
    }

    fn update_from_trace(&mut self, trace: &TickTrace) {
        for coupling in &mut self.cell_coupling {
            *coupling *= 0.82;
        }

        for (rank, cell_id) in trace.active_cell_ids.iter().copied().enumerate() {
            let index = cell_id as usize;
            if index >= STAGE2_ORGAN_CELLS {
                continue;
            }
            let rank_gain = (STAGE2_TOP_K - rank) as f32 / STAGE2_TOP_K as f32;
            self.cell_coupling[index] =
                (self.cell_coupling[index] + rank_gain * trace.coherence * 0.12).clamp(0.0, 1.0);
        }

        self.previous_center_phase = trace.center_phase;
        self.previous_coherence = trace.coherence;
        self.previous_entropy = trace.spectral_entropy;
        self.tick_index = self.tick_index.saturating_add(1);
    }

    fn update_link_from_bus(&mut self, bus: &WaveBus) {
        if self.tick_index > 0 {
            let previous_phase_norm = self
                .previous_phase_sum
                .iter()
                .map(|value| value.abs())
                .sum::<f32>()
                .max(f32::EPSILON);
            let current_phase_norm = bus
                .phase_sum
                .iter()
                .map(|value| value.abs())
                .sum::<f32>()
                .max(f32::EPSILON);
            let previous_amplitude_norm = self
                .previous_amplitude_sum
                .iter()
                .sum::<f32>()
                .max(f32::EPSILON);
            let current_amplitude_norm = bus.amplitude_sum.iter().sum::<f32>().max(f32::EPSILON);

            for value in &mut self.link_phase_sum {
                *value *= 0.72;
            }
            for value in &mut self.link_amplitude_sum {
                *value *= 0.72;
            }

            for output_slot in 0..PHASE_SLOTS {
                let mut phase_link = 0.0;
                let mut amplitude_link = 0.0;
                for previous_slot in 0..PHASE_SLOTS {
                    let current_slot = (output_slot + PHASE_SLOTS - previous_slot) % PHASE_SLOTS;
                    phase_link += (self.previous_phase_sum[previous_slot] / previous_phase_norm)
                        * (bus.phase_sum[current_slot] / current_phase_norm);
                    amplitude_link += (self.previous_amplitude_sum[previous_slot]
                        / previous_amplitude_norm)
                        * (bus.amplitude_sum[current_slot] / current_amplitude_norm);
                }
                self.link_phase_sum[output_slot] += phase_link;
                self.link_amplitude_sum[output_slot] += amplitude_link;
            }
        }

        self.previous_phase_sum = bus.phase_sum;
        self.previous_amplitude_sum = bus.amplitude_sum;
    }
}

#[must_use]
fn prediction_from_trace(trace: &TickTrace) -> LivePrediction {
    let phase_unit = (trace.center_phase / TAU).rem_euclid(1.0);
    let predicted_byte = (phase_unit * 256.0).round() as u8;
    let confidence = (trace.coherence * trace.center_magnitude).clamp(0.0, 1.0);

    LivePrediction {
        predicted_byte,
        confidence,
    }
}

#[must_use]
fn byte_phase(byte: u8) -> f32 {
    TAU * byte as f32 / PHASE_SLOTS as f32
}

/// Create the first six-cell organism.
#[must_use]
pub fn stage2_organ(seed: u64) -> [Cell32; STAGE2_ORGAN_CELLS] {
    Stage2Organ::new(seed).cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_settle_ticks_record_link_state() {
        let seed = 13;
        let organ = Stage2Organ::new(seed);
        let mut state = OrganState::new(seed, b'l');
        let first_carrier = CarrierWave::from_seed(seed, b'e');
        let second_carrier = CarrierWave::from_seed(seed, b't');

        state.settle_bus_tick_with_carrier(&organ, b'e', first_carrier, None);
        let first_phase_energy: f32 = state.link_phase_sum.iter().map(|value| value.abs()).sum();
        assert_eq!(first_phase_energy, 0.0);

        state.settle_bus_tick_with_carrier(&organ, b't', second_carrier, None);
        let phase_energy: f32 = state.link_phase_sum.iter().map(|value| value.abs()).sum();
        let amplitude_energy: f32 = state.link_amplitude_sum.iter().sum();

        assert!(phase_energy > 0.0);
        assert!(amplitude_energy > 0.0);
    }
}
