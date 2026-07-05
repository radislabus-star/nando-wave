use std::collections::{BTreeMap, HashMap};

pub type WavePredictorCenterId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorHebbianConfig {
    pub eta_pos: i16,
    pub eta_neg: i16,
    pub eta_conflict: i16,
    pub eta_anti: i16,
    pub eta_binding: i16,
    pub state_delta_binding_feature_base: Option<WavePredictorCenterId>,
    pub weight_limit: i16,
}

impl Default for WavePredictorHebbianConfig {
    fn default() -> Self {
        Self {
            eta_pos: 4,
            eta_neg: 3,
            eta_conflict: 2,
            eta_anti: 6,
            eta_binding: 0,
            state_delta_binding_feature_base: None,
            weight_limit: 512,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorActiveCenter {
    pub center_id: WavePredictorCenterId,
    pub strength: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorConvergenceError {
    pub active_fringe: Vec<WavePredictorActiveCenter>,
    pub target_center: WavePredictorCenterId,
    pub nearest_wrong_center: WavePredictorCenterId,
    pub target_gap: i32,
    pub margin_required: i32,
    pub trap_accepted: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorHebbianEdge {
    pub source_center: WavePredictorCenterId,
    pub target_center: WavePredictorCenterId,
    pub compatibility: i16,
    pub conflict: i16,
    pub anti_wave: i16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorHebbianUpdateReport {
    pub touched_edges: usize,
    pub attraction_updates: usize,
    pub repulsion_updates: usize,
    pub conflict_updates: usize,
    pub anti_wave_updates: usize,
    pub target_gap_before: i32,
    pub target_gap_after: i32,
    pub margin_required: i32,
    pub margin_fixed: bool,
    pub base_mass_drift_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorHebbianField {
    base_mass: Vec<i16>,
    edges: BTreeMap<(WavePredictorCenterId, WavePredictorCenterId), WavePredictorHebbianEdge>,
    state_delta_edges: HashMap<(WavePredictorCenterId, u16), i16>,
    state_delta_binding_positive: i16,
    state_delta_binding_negative: i16,
    config: WavePredictorHebbianConfig,
}

impl WavePredictorHebbianField {
    pub fn new(center_count: usize, config: WavePredictorHebbianConfig) -> Self {
        Self {
            base_mass: vec![0; center_count],
            edges: BTreeMap::new(),
            state_delta_edges: HashMap::new(),
            state_delta_binding_positive: 0,
            state_delta_binding_negative: 0,
            config,
        }
    }

    pub fn config(&self) -> WavePredictorHebbianConfig {
        self.config
    }

    pub fn set_base_mass(&mut self, center_id: WavePredictorCenterId, mass: i16) {
        if let Some(slot) = self.base_mass.get_mut(center_id as usize) {
            *slot = mass;
        }
    }

    pub fn base_mass(&self, center_id: WavePredictorCenterId) -> Option<i16> {
        self.base_mass.get(center_id as usize).copied()
    }

    pub fn edge(
        &self,
        source_center: WavePredictorCenterId,
        target_center: WavePredictorCenterId,
    ) -> Option<WavePredictorHebbianEdge> {
        self.edges.get(&(source_center, target_center)).copied()
    }

    pub fn insert_edge(&mut self, edge: WavePredictorHebbianEdge) {
        self.edges
            .insert((edge.source_center, edge.target_center), edge);
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn state_delta_edge(
        &self,
        source_center: WavePredictorCenterId,
        lane_id: u16,
    ) -> Option<i16> {
        self.state_delta_edges
            .get(&(source_center, lane_id))
            .copied()
    }

    pub fn state_delta_edge_count(&self) -> usize {
        self.state_delta_edges.len()
    }

    pub fn state_delta_binding_weights(&self) -> (i16, i16) {
        (
            self.state_delta_binding_positive,
            self.state_delta_binding_negative,
        )
    }

    pub fn score_state_delta_lane(
        &self,
        lane_id: u16,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> i32 {
        active_fringe
            .iter()
            .map(|active| {
                let Some(weight) = self.state_delta_edge(active.center_id, lane_id) else {
                    return 0;
                };
                i32::from(active.strength) * i32::from(weight)
            })
            .sum()
    }

    pub fn score_state_delta_binding_alignment(
        &self,
        lane_id: u16,
        signed_strength: i16,
        active_fringe: &[WavePredictorActiveCenter],
        _binding_output_slot: Option<u8>,
    ) -> i32 {
        if let Some(active_strength) =
            self.state_delta_binding_active_strength(lane_id, active_fringe)
        {
            let weight = if signed_strength < 0 {
                self.state_delta_binding_negative
            } else {
                self.state_delta_binding_positive
            };
            i32::from(active_strength.abs()) * i32::from(weight)
        } else {
            0
        }
    }

    pub fn adjust_state_delta_edge(
        &mut self,
        source_center: WavePredictorCenterId,
        lane_id: u16,
        delta: i32,
    ) -> bool {
        if delta == 0 {
            return false;
        }

        let limit = self.config.weight_limit;
        let edge = self
            .state_delta_edges
            .entry((source_center, lane_id))
            .or_insert(0);
        let before = *edge;
        *edge = clamp_i16(i32::from(*edge) + delta, limit);
        *edge != before
    }

    pub fn adjust_state_delta_binding(&mut self, signed_strength: i16, delta: i32) -> bool {
        if delta == 0 {
            return false;
        }

        let limit = self.config.weight_limit;
        let slot = if signed_strength < 0 {
            &mut self.state_delta_binding_negative
        } else {
            &mut self.state_delta_binding_positive
        };
        let before = *slot;
        *slot = clamp_non_negative_i16(i32::from(*slot) + delta, limit);
        *slot != before
    }

    pub fn state_delta_binding_active_strength(
        &self,
        lane_id: u16,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> Option<i16> {
        let base = self.config.state_delta_binding_feature_base?;
        let center_id = base.checked_add(WavePredictorCenterId::from(lane_id))?;
        active_fringe
            .iter()
            .find(|active| active.center_id == center_id && active.strength != 0)
            .map(|active| active.strength)
    }

    pub fn score_center(
        &self,
        center_id: WavePredictorCenterId,
        active_fringe: &[WavePredictorActiveCenter],
    ) -> i32 {
        active_fringe
            .iter()
            .map(|active| {
                let Some(edge) = self.edge(active.center_id, center_id) else {
                    return 0;
                };
                i32::from(active.strength)
                    * (i32::from(edge.compatibility)
                        - i32::from(edge.conflict)
                        - i32::from(edge.anti_wave))
            })
            .sum()
    }

    pub fn target_gap(&self, error: &WavePredictorConvergenceError) -> i32 {
        self.score_center(error.target_center, &error.active_fringe)
            - self.score_center(error.nearest_wrong_center, &error.active_fringe)
    }

    pub fn apply_sparse_contrastive_error(
        &mut self,
        error: &WavePredictorConvergenceError,
    ) -> WavePredictorHebbianUpdateReport {
        let base_mass_before = self.base_mass.clone();
        let gap_before = error.target_gap;
        let mut report = WavePredictorHebbianUpdateReport {
            target_gap_before: gap_before,
            margin_required: error.margin_required,
            ..WavePredictorHebbianUpdateReport::default()
        };

        if gap_before < error.margin_required {
            for active in &error.active_fringe {
                let strength = active.strength.max(0);
                if strength == 0 {
                    continue;
                }

                let eta_pos = self.config.eta_pos;
                let eta_neg = self.config.eta_neg;
                let eta_conflict = self.config.eta_conflict;
                let limit = self.config.weight_limit;

                {
                    let target_edge = self.edge_mut(active.center_id, error.target_center);
                    target_edge.compatibility = clamp_i16(
                        i32::from(target_edge.compatibility)
                            + i32::from(eta_pos) * i32::from(strength),
                        limit,
                    );
                    report.touched_edges += 1;
                    report.attraction_updates += 1;
                }

                {
                    let wrong_edge = self.edge_mut(active.center_id, error.nearest_wrong_center);
                    wrong_edge.compatibility = clamp_i16(
                        i32::from(wrong_edge.compatibility)
                            - i32::from(eta_neg) * i32::from(strength),
                        limit,
                    );
                    wrong_edge.conflict = clamp_non_negative_i16(
                        i32::from(wrong_edge.conflict)
                            + i32::from(eta_conflict) * i32::from(strength),
                        limit,
                    );
                    report.touched_edges += 1;
                    report.repulsion_updates += 1;
                    report.conflict_updates += 1;
                }
            }
        }

        if error.trap_accepted {
            for active in &error.active_fringe {
                let strength = active.strength.max(0);
                if strength == 0 {
                    continue;
                }
                let eta_anti = self.config.eta_anti;
                let limit = self.config.weight_limit;
                let wrong_edge = self.edge_mut(active.center_id, error.nearest_wrong_center);
                wrong_edge.anti_wave = clamp_non_negative_i16(
                    i32::from(wrong_edge.anti_wave) + i32::from(eta_anti) * i32::from(strength),
                    limit,
                );
                report.touched_edges += 1;
                report.anti_wave_updates += 1;
            }
        }

        report.target_gap_after = self.target_gap(error);
        report.margin_fixed = report.target_gap_after >= error.margin_required;
        report.base_mass_drift_detected = self.base_mass != base_mass_before;
        report
    }

    fn edge_mut(
        &mut self,
        source_center: WavePredictorCenterId,
        target_center: WavePredictorCenterId,
    ) -> &mut WavePredictorHebbianEdge {
        self.edges
            .entry((source_center, target_center))
            .or_insert(WavePredictorHebbianEdge {
                source_center,
                target_center,
                ..WavePredictorHebbianEdge::default()
            })
    }
}

fn clamp_i16(value: i32, limit: i16) -> i16 {
    value.clamp(-i32::from(limit), i32::from(limit)) as i16
}

fn clamp_non_negative_i16(value: i32, limit: i16) -> i16 {
    value.clamp(0, i32::from(limit)) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_contrastive_error_update_repairs_weak_margin_locally() {
        let mut field = WavePredictorHebbianField::new(6, WavePredictorHebbianConfig::default());
        for center in 0..6 {
            field.set_base_mass(center, 100 + center as i16);
        }
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 0,
            target_center: 3,
            compatibility: 24,
            ..WavePredictorHebbianEdge::default()
        });
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 5,
            target_center: 4,
            compatibility: 99,
            ..WavePredictorHebbianEdge::default()
        });

        let error = WavePredictorConvergenceError {
            active_fringe: vec![
                WavePredictorActiveCenter {
                    center_id: 0,
                    strength: 12,
                },
                WavePredictorActiveCenter {
                    center_id: 1,
                    strength: 8,
                },
            ],
            target_center: 2,
            nearest_wrong_center: 3,
            target_gap: -288,
            margin_required: 120,
            trap_accepted: false,
        };

        let untouched_before = field.edge(5, 4);
        let report = field.apply_sparse_contrastive_error(&error);

        assert!(
            report.target_gap_after > report.target_gap_before,
            "report={report:#?}"
        );
        assert!(report.margin_fixed, "report={report:#?}");
        assert_eq!(report.attraction_updates, 2);
        assert_eq!(report.repulsion_updates, 2);
        assert_eq!(report.conflict_updates, 2);
        assert_eq!(report.anti_wave_updates, 0);
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
        assert_eq!(field.base_mass(2), Some(102));
        assert_eq!(field.edge(5, 4), untouched_before);
        assert!(
            field
                .edge(0, 2)
                .expect("target edge should be created")
                .compatibility
                > 0
        );
        let wrong_edge = field
            .edge(0, 3)
            .expect("wrong edge should be updated for repulsion");
        assert!(wrong_edge.compatibility < 24);
        assert!(wrong_edge.conflict > 0);
    }

    #[test]
    fn accepted_trap_reinforces_only_wrong_anti_wave() {
        let mut field = WavePredictorHebbianField::new(4, WavePredictorHebbianConfig::default());
        let error = WavePredictorConvergenceError {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 5,
            }],
            target_center: 1,
            nearest_wrong_center: 2,
            target_gap: 200,
            margin_required: 0,
            trap_accepted: true,
        };

        let report = field.apply_sparse_contrastive_error(&error);

        assert_eq!(report.attraction_updates, 0);
        assert_eq!(report.repulsion_updates, 0);
        assert_eq!(report.conflict_updates, 0);
        assert_eq!(report.anti_wave_updates, 1);
        assert_eq!(field.edge(0, 1), None);
        assert!(
            field
                .edge(0, 2)
                .expect("wrong edge should receive anti-wave")
                .anti_wave
                > 0
        );
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
    }

    #[test]
    fn settled_non_trap_case_does_not_change_weights_or_mass() {
        let mut field = WavePredictorHebbianField::new(3, WavePredictorHebbianConfig::default());
        field.set_base_mass(1, 42);
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 0,
            target_center: 1,
            compatibility: 64,
            ..WavePredictorHebbianEdge::default()
        });
        let before = field.clone();
        let error = WavePredictorConvergenceError {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 4,
            }],
            target_center: 1,
            nearest_wrong_center: 2,
            target_gap: 256,
            margin_required: 120,
            trap_accepted: false,
        };

        let report = field.apply_sparse_contrastive_error(&error);

        assert_eq!(report.touched_edges, 0);
        assert_eq!(field, before);
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
    }
}
