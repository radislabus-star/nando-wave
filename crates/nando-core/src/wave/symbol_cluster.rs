use std::f32::consts::TAU;

use super::{
    PeakOutcome, SYMBOL_CELL8_INTERFERENCE_SLOTS, SYMBOL_WAVE_CLUSTER_CELLS, SymbolCell8,
    SymbolCell8Advice,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolClusterCenter {
    pub peak_slot: u16,
    pub energy: u32,
    pub coherence: u16,
    pub support_count: u16,
    pub accepted_count: u16,
    pub supported_count: u16,
    pub reflected_count: u16,
    pub spurious_count: u16,
    pub limit_cycle_count: u16,
    pub carrier_phase: i8,
    pub outcome: PeakOutcome,
}

impl Default for SymbolClusterCenter {
    fn default() -> Self {
        Self {
            peak_slot: 0,
            energy: 0,
            coherence: 0,
            support_count: 0,
            accepted_count: 0,
            supported_count: 0,
            reflected_count: 0,
            spurious_count: 0,
            limit_cycle_count: 0,
            carrier_phase: 0,
            outcome: PeakOutcome::NoPeak,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolClusterTick {
    pub center: SymbolClusterCenter,
    pub cell_outcomes: [PeakOutcome; SYMBOL_WAVE_CLUSTER_CELLS],
    pub forward_messages: u16,
    pub reflected_messages: u16,
}

#[derive(Clone)]
pub struct SymbolWaveCluster {
    cells: [SymbolCell8; SYMBOL_WAVE_CLUSTER_CELLS],
    last_center: SymbolClusterCenter,
}

impl SymbolWaveCluster {
    #[must_use]
    pub fn new(cluster_id: u32, seed: u64) -> Self {
        let cells = std::array::from_fn(|index| {
            let role = (index % 6) as u8;
            let id = cluster_id
                .wrapping_mul(SYMBOL_WAVE_CLUSTER_CELLS as u32)
                .wrapping_add(index as u32);
            SymbolCell8::new(id, role, seed ^ ((index as u64 + 1) * 0x9E37_79B9))
        });
        Self {
            cells,
            last_center: SymbolClusterCenter::default(),
        }
    }

    #[must_use]
    pub fn cells(&self) -> &[SymbolCell8; SYMBOL_WAVE_CLUSTER_CELLS] {
        &self.cells
    }

    #[must_use]
    pub fn last_center(&self) -> SymbolClusterCenter {
        self.last_center
    }

    pub fn tick_symbol(&mut self, symbol: char) -> SymbolClusterTick {
        let incoming = self.last_center_advice();
        match incoming {
            Some(message) => self.tick_symbol_with_incoming(symbol, &[message]),
            None => self.tick_symbol_with_incoming(symbol, &[]),
        }
    }

    pub fn tick_symbol_with_incoming(
        &mut self,
        symbol: char,
        incoming: &[SymbolCell8Advice],
    ) -> SymbolClusterTick {
        let previous_peak = (self.last_center.energy > 0).then_some(self.last_center.peak_slot);
        let carrier_phase = self.last_center.carrier_phase;
        let mut outcomes = [PeakOutcome::NoPeak; SYMBOL_WAVE_CLUSTER_CELLS];
        let mut advices = [None; SYMBOL_WAVE_CLUSTER_CELLS];
        let mut reflected_messages = 0u16;
        let mut reflected_count = 0u16;
        let mut spurious_count = 0u16;
        let mut limit_cycle_count = 0u16;
        let mut accepted_count = 0u16;
        let mut supported_count = 0u16;

        for (index, cell) in self.cells.iter_mut().enumerate() {
            let tick =
                cell.tick_symbol_with_context(symbol, previous_peak, carrier_phase, incoming);
            outcomes[index] = tick.outcome;
            advices[index] = tick.advice;
            reflected_messages =
                reflected_messages.saturating_add(u16::from(tick.reflection.is_some()));

            match tick.outcome {
                PeakOutcome::Accepted => accepted_count = accepted_count.saturating_add(1),
                PeakOutcome::Supported => supported_count = supported_count.saturating_add(1),
                PeakOutcome::Reflected => reflected_count = reflected_count.saturating_add(1),
                PeakOutcome::Spurious => spurious_count = spurious_count.saturating_add(1),
                PeakOutcome::LimitCycle => limit_cycle_count = limit_cycle_count.saturating_add(1),
                PeakOutcome::Vetoed | PeakOutcome::Unstable | PeakOutcome::NoPeak => {}
            }
        }

        let mut center = center_from_advices(
            &advices,
            accepted_count,
            supported_count,
            reflected_count,
            spurious_count,
            limit_cycle_count,
        );
        center.outcome = classify_cluster_center(center);
        self.last_center = center;

        SymbolClusterTick {
            center,
            cell_outcomes: outcomes,
            forward_messages: center.support_count,
            reflected_messages,
        }
    }

    fn last_center_advice(&self) -> Option<SymbolCell8Advice> {
        (self.last_center.energy > 0).then_some(SymbolCell8Advice {
            peak_slot: self.last_center.peak_slot,
            energy: self.last_center.energy.min(u32::from(u16::MAX)) as u16,
            coherence: self.last_center.coherence,
            phase: self.last_center.carrier_phase,
            role: 0,
        })
    }
}

fn center_from_advices(
    advices: &[Option<SymbolCell8Advice>; SYMBOL_WAVE_CLUSTER_CELLS],
    accepted_count: u16,
    supported_count: u16,
    reflected_count: u16,
    spurious_count: u16,
    limit_cycle_count: u16,
) -> SymbolClusterCenter {
    let mut weighted_x = 0.0f32;
    let mut weighted_y = 0.0f32;
    let mut phase_sum = 0i32;
    let mut weight_sum = 0u32;
    let mut energy_sum = 0u32;

    for advice in advices.iter().flatten() {
        let weight = u32::from(advice.energy.max(1)) * u32::from(advice.coherence.max(1));
        let angle = TAU * f32::from(advice.peak_slot) / SYMBOL_CELL8_INTERFERENCE_SLOTS as f32;
        weighted_x += angle.cos() * weight as f32;
        weighted_y += angle.sin() * weight as f32;
        phase_sum += i32::from(advice.phase) * i32::try_from(weight.min(4096)).unwrap_or(0);
        weight_sum = weight_sum.saturating_add(weight);
        energy_sum = energy_sum.saturating_add(u32::from(advice.energy));
    }

    let support_count = accepted_count.saturating_add(supported_count);
    let (peak_slot, coherence, carrier_phase) = if weight_sum == 0 {
        (0, 0, 0)
    } else {
        let angle = weighted_y.atan2(weighted_x).rem_euclid(TAU);
        let peak_slot = ((angle / TAU) * SYMBOL_CELL8_INTERFERENCE_SLOTS as f32).round() as u16
            % SYMBOL_CELL8_INTERFERENCE_SLOTS as u16;
        let magnitude = (weighted_x.mul_add(weighted_x, weighted_y * weighted_y)).sqrt();
        let coherence = ((magnitude / weight_sum as f32) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u16;
        let carrier_phase = (phase_sum / i32::try_from(weight_sum.min(4096)).unwrap_or(1))
            .clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
        (peak_slot, coherence, carrier_phase)
    };

    SymbolClusterCenter {
        peak_slot,
        energy: energy_sum,
        coherence,
        support_count,
        accepted_count,
        supported_count,
        reflected_count,
        spurious_count,
        limit_cycle_count,
        carrier_phase,
        outcome: PeakOutcome::NoPeak,
    }
}

fn classify_cluster_center(center: SymbolClusterCenter) -> PeakOutcome {
    if center.limit_cycle_count > center.support_count && center.limit_cycle_count > 0 {
        PeakOutcome::LimitCycle
    } else if center.accepted_count >= 4 && center.coherence >= 24 {
        PeakOutcome::Accepted
    } else if center.support_count > 0 && center.energy > 0 {
        PeakOutcome::Supported
    } else if center.spurious_count > 0 {
        PeakOutcome::Spurious
    } else if center.reflected_count > 0 {
        PeakOutcome::Reflected
    } else if center.energy > 0 {
        PeakOutcome::Unstable
    } else {
        PeakOutcome::NoPeak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_has_sixteen_cells_and_empty_initial_center() {
        let cluster = SymbolWaveCluster::new(0, 42);

        assert_eq!(cluster.cells().len(), SYMBOL_WAVE_CLUSTER_CELLS);
        assert_eq!(cluster.last_center().energy, 0);
        assert_eq!(cluster.last_center().outcome, PeakOutcome::NoPeak);
    }

    #[test]
    fn repeated_symbol_builds_cluster_center() {
        let mut cluster = SymbolWaveCluster::new(1, 99);

        let first = cluster.tick_symbol('N');
        let _second = cluster.tick_symbol('N');
        let third = cluster.tick_symbol('N');

        assert!(first.center.energy > 0);
        assert!(third.center.support_count > 0);
        assert_eq!(third.forward_messages, third.center.support_count);
        assert!(matches!(
            third.center.outcome,
            PeakOutcome::Accepted | PeakOutcome::Supported
        ));
    }

    #[test]
    fn cluster_reflection_does_not_become_understanding() {
        let mut cluster = SymbolWaveCluster::new(2, 123);
        let incoming = [SymbolCell8Advice {
            peak_slot: 9,
            energy: 240,
            coherence: 8,
            phase: 127,
            role: 99,
        }];

        let tick = cluster.tick_symbol_with_incoming('R', &incoming);

        assert!(tick.reflected_messages > 0);
        assert_ne!(tick.center.outcome, PeakOutcome::Accepted);
    }
}
