use std::f32::consts::TAU;

use super::{
    L3_8MB_SYMBOL_WAVE_CLUSTERS, PeakOutcome, SYMBOL_CELL8_INTERFERENCE_SLOTS,
    SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, SYMBOL_L3_TURBO_WAVE_CLUSTERS, SYMBOL_WAVE_CLUSTER_CELLS,
    SymbolCell8Advice, SymbolWaveCluster,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymbolL3Center {
    pub peak_slot: u16,
    pub energy: u64,
    pub coherence: u16,
    pub support_cells: u16,
    pub supported_clusters: u16,
    pub accepted_clusters: u16,
    pub reflected_clusters: u16,
    pub spurious_clusters: u16,
    pub limit_cycle_clusters: u16,
    pub carrier_phase: i8,
    pub outcome: PeakOutcome,
}

impl Default for SymbolL3Center {
    fn default() -> Self {
        Self {
            peak_slot: 0,
            energy: 0,
            coherence: 0,
            support_cells: 0,
            supported_clusters: 0,
            accepted_clusters: 0,
            reflected_clusters: 0,
            spurious_clusters: 0,
            limit_cycle_clusters: 0,
            carrier_phase: 0,
            outcome: PeakOutcome::NoPeak,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolL3Tick {
    pub center: SymbolL3Center,
    pub cluster_outcomes: [PeakOutcome; L3_8MB_SYMBOL_WAVE_CLUSTERS],
    pub active_clusters: usize,
    pub active_cells: usize,
    pub active_bytes: usize,
    pub forward_messages: u32,
    pub reflected_messages: u32,
}

#[derive(Clone)]
pub struct SymbolL3Organism {
    clusters: Vec<SymbolWaveCluster>,
    last_center: SymbolL3Center,
}

impl SymbolL3Organism {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::with_clusters(seed, SYMBOL_L3_DEFAULT_WAVE_CLUSTERS)
    }

    #[must_use]
    pub fn new_turbo_2mb(seed: u64) -> Self {
        Self::with_clusters(seed, SYMBOL_L3_TURBO_WAVE_CLUSTERS)
    }

    #[must_use]
    pub fn new_max_8mb(seed: u64) -> Self {
        Self::with_clusters(seed, L3_8MB_SYMBOL_WAVE_CLUSTERS)
    }

    #[must_use]
    pub fn with_clusters(seed: u64, cluster_count: usize) -> Self {
        let cluster_count = cluster_count.clamp(1, L3_8MB_SYMBOL_WAVE_CLUSTERS);
        let mut clusters = Vec::with_capacity(cluster_count);
        for index in 0..cluster_count {
            clusters.push(SymbolWaveCluster::new(
                index as u32,
                seed ^ ((index as u64 + 1) * 0xA53A_9E37),
            ));
        }

        Self {
            clusters,
            last_center: SymbolL3Center::default(),
        }
    }

    #[must_use]
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cluster_count() * SYMBOL_WAVE_CLUSTER_CELLS
    }

    #[must_use]
    pub fn active_bytes(&self) -> usize {
        self.cell_count() * super::SYMBOL_CELL8_BYTES
    }

    #[must_use]
    pub fn last_center(&self) -> SymbolL3Center {
        self.last_center
    }

    pub fn tick_symbol(&mut self, symbol: char) -> SymbolL3Tick {
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
    ) -> SymbolL3Tick {
        let active_clusters = self.cluster_count();
        let mut cluster_outcomes = [PeakOutcome::NoPeak; L3_8MB_SYMBOL_WAVE_CLUSTERS];
        let mut cluster_centers = [SymbolL3Center::default(); L3_8MB_SYMBOL_WAVE_CLUSTERS];
        let mut forward_messages = 0u32;
        let mut reflected_messages = 0u32;

        for (index, cluster) in self.clusters.iter_mut().enumerate() {
            let tick = cluster.tick_symbol_with_incoming(symbol, incoming);
            cluster_outcomes[index] = tick.center.outcome;
            cluster_centers[index] = SymbolL3Center {
                peak_slot: tick.center.peak_slot,
                energy: u64::from(tick.center.energy),
                coherence: tick.center.coherence,
                support_cells: tick.center.support_count,
                supported_clusters: u16::from(tick.center.outcome == PeakOutcome::Supported),
                accepted_clusters: u16::from(tick.center.outcome == PeakOutcome::Accepted),
                reflected_clusters: u16::from(tick.center.outcome == PeakOutcome::Reflected),
                spurious_clusters: u16::from(tick.center.outcome == PeakOutcome::Spurious),
                limit_cycle_clusters: u16::from(tick.center.outcome == PeakOutcome::LimitCycle),
                carrier_phase: tick.center.carrier_phase,
                outcome: tick.center.outcome,
            };
            forward_messages = forward_messages.saturating_add(u32::from(tick.forward_messages));
            reflected_messages =
                reflected_messages.saturating_add(u32::from(tick.reflected_messages));
        }

        let mut center = center_from_clusters(&cluster_centers[..active_clusters]);
        center.outcome = classify_l3_center(center);
        self.last_center = center;

        SymbolL3Tick {
            center,
            cluster_outcomes,
            active_clusters,
            active_cells: self.cell_count(),
            active_bytes: self.active_bytes(),
            forward_messages,
            reflected_messages,
        }
    }

    fn last_center_advice(&self) -> Option<SymbolCell8Advice> {
        (self.last_center.energy > 0).then_some(SymbolCell8Advice {
            peak_slot: self.last_center.peak_slot,
            energy: self.last_center.energy.min(u64::from(u16::MAX)) as u16,
            coherence: self.last_center.coherence,
            phase: self.last_center.carrier_phase,
            role: 0,
        })
    }
}

fn center_from_clusters(clusters: &[SymbolL3Center]) -> SymbolL3Center {
    let mut weighted_x = 0.0f32;
    let mut weighted_y = 0.0f32;
    let mut phase_sum = 0i64;
    let mut weight_sum = 0u64;
    let mut energy_sum = 0u64;
    let mut support_cells = 0u16;
    let mut supported_clusters = 0u16;
    let mut accepted_clusters = 0u16;
    let mut reflected_clusters = 0u16;
    let mut spurious_clusters = 0u16;
    let mut limit_cycle_clusters = 0u16;

    for center in clusters {
        if center.energy == 0 {
            continue;
        }

        let support_weight = u64::from(center.support_cells.max(1));
        let weight = center
            .energy
            .saturating_mul(u64::from(center.coherence.max(1)))
            .saturating_mul(support_weight);
        let angle = TAU * f32::from(center.peak_slot) / SYMBOL_CELL8_INTERFERENCE_SLOTS as f32;
        weighted_x += angle.cos() * weight as f32;
        weighted_y += angle.sin() * weight as f32;
        phase_sum +=
            i64::from(center.carrier_phase) * i64::try_from(weight.min(65_536)).unwrap_or(0);
        weight_sum = weight_sum.saturating_add(weight);
        energy_sum = energy_sum.saturating_add(center.energy);
        support_cells = support_cells.saturating_add(center.support_cells);
        supported_clusters = supported_clusters.saturating_add(center.supported_clusters);
        accepted_clusters = accepted_clusters.saturating_add(center.accepted_clusters);
        reflected_clusters = reflected_clusters.saturating_add(center.reflected_clusters);
        spurious_clusters = spurious_clusters.saturating_add(center.spurious_clusters);
        limit_cycle_clusters = limit_cycle_clusters.saturating_add(center.limit_cycle_clusters);
    }

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
        let carrier_phase = (phase_sum / i64::try_from(weight_sum.min(65_536)).unwrap_or(1))
            .clamp(i64::from(i8::MIN), i64::from(i8::MAX)) as i8;
        (peak_slot, coherence, carrier_phase)
    };

    SymbolL3Center {
        peak_slot,
        energy: energy_sum,
        coherence,
        support_cells,
        supported_clusters,
        accepted_clusters,
        reflected_clusters,
        spurious_clusters,
        limit_cycle_clusters,
        carrier_phase,
        outcome: PeakOutcome::NoPeak,
    }
}

fn classify_l3_center(center: SymbolL3Center) -> PeakOutcome {
    if center.limit_cycle_clusters > center.supported_clusters + center.accepted_clusters
        && center.limit_cycle_clusters > 0
    {
        PeakOutcome::LimitCycle
    } else if center.accepted_clusters >= 4 && center.coherence >= 16 {
        PeakOutcome::Accepted
    } else if center.supported_clusters + center.accepted_clusters >= 4 && center.energy > 0 {
        PeakOutcome::Supported
    } else if center.spurious_clusters > 0 {
        PeakOutcome::Spurious
    } else if center.reflected_clusters > 0 {
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
    fn l3_organism_can_build_256_cell_turbo_profile() {
        let organism = SymbolL3Organism::new_turbo_2mb(42);

        assert_eq!(organism.cluster_count(), SYMBOL_L3_TURBO_WAVE_CLUSTERS);
        assert_eq!(
            organism.cell_count(),
            super::super::SYMBOL_L3_TURBO_CELL8_CELLS
        );
        assert_eq!(
            organism.active_bytes(),
            super::super::SYMBOL_L3_TURBO_ACTIVE_BYTES
        );
        assert_eq!(organism.active_bytes(), 2 * 1024 * 1024);
    }

    #[test]
    fn l3_organism_defaults_to_512_cells_inside_4mib_budget() {
        let organism = SymbolL3Organism::new(42);

        assert_eq!(
            organism.cluster_count(),
            super::super::SYMBOL_L3_DEFAULT_WAVE_CLUSTERS
        );
        assert_eq!(
            organism.cell_count(),
            super::super::SYMBOL_L3_DEFAULT_CELL8_CELLS
        );
        assert_eq!(
            organism.active_bytes(),
            super::super::SYMBOL_L3_DEFAULT_ACTIVE_BYTES
        );
        assert_eq!(organism.active_bytes(), 4 * 1024 * 1024);
    }

    #[test]
    fn l3_organism_can_build_1024_cell_stress_profile() {
        let organism = SymbolL3Organism::new_max_8mb(42);

        assert_eq!(organism.cluster_count(), L3_8MB_SYMBOL_WAVE_CLUSTERS);
        assert_eq!(
            organism.cell_count(),
            super::super::L3_8MB_SYMBOL_CELL8_CELLS
        );
        assert_eq!(
            organism.active_bytes(),
            super::super::L3_8MB_SYMBOL_ACTIVE_BYTES
        );
    }

    #[test]
    fn repeated_symbol_builds_l3_center() {
        let mut organism = SymbolL3Organism::new(99);

        let first = organism.tick_symbol('N');
        let _second = organism.tick_symbol('N');
        let third = organism.tick_symbol('N');

        assert_eq!(
            first.active_clusters,
            super::super::SYMBOL_L3_DEFAULT_WAVE_CLUSTERS
        );
        assert_eq!(
            first.active_cells,
            super::super::SYMBOL_L3_DEFAULT_CELL8_CELLS
        );
        assert_eq!(first.active_bytes, 4 * 1024 * 1024);
        assert!(third.center.support_cells > 0);
        assert!(third.forward_messages <= super::super::SYMBOL_L3_DEFAULT_CELL8_CELLS as u32);
        assert!(matches!(
            third.center.outcome,
            PeakOutcome::Accepted | PeakOutcome::Supported
        ));
    }

    #[test]
    fn l3_reflection_is_bounded_and_not_accepted() {
        let mut organism = SymbolL3Organism::new(123);
        let incoming = [SymbolCell8Advice {
            peak_slot: 3,
            energy: 240,
            coherence: 8,
            phase: 127,
            role: 99,
        }];

        let tick = organism.tick_symbol_with_incoming('R', &incoming);

        assert!(tick.reflected_messages > 0);
        assert!(tick.reflected_messages <= super::super::SYMBOL_L3_DEFAULT_CELL8_CELLS as u32);
        assert_ne!(tick.center.outcome, PeakOutcome::Accepted);
    }
}
