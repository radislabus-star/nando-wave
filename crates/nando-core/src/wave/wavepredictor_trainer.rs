use super::{
    WavePredictorActiveCenter, WavePredictorCenterId, WavePredictorConvergenceError,
    WavePredictorHebbianField, WavePredictorHebbianUpdateReport,
};

pub const WAVEPREDICTOR_TARGET_AXIS_CAP: usize = 16;
pub const WAVEPREDICTOR_STATE_DELTA_CAP: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorMarginSchedule {
    pub start_margin: i32,
    pub target_margin: i32,
    pub warmup_epochs: u16,
    pub ramp_epochs: u16,
}

impl Default for WavePredictorMarginSchedule {
    fn default() -> Self {
        Self {
            start_margin: 24,
            target_margin: 120,
            warmup_epochs: 2,
            ramp_epochs: 8,
        }
    }
}

impl WavePredictorMarginSchedule {
    pub fn margin_for_epoch(&self, epoch_index: u16) -> i32 {
        if epoch_index < self.warmup_epochs {
            return self.start_margin;
        }

        let ramp_epochs = self.ramp_epochs.max(1);
        let elapsed = (epoch_index - self.warmup_epochs + 1).min(ramp_epochs);
        let span = i64::from(self.target_margin) - i64::from(self.start_margin);

        (i64::from(self.start_margin) + span * i64::from(elapsed) / i64::from(ramp_epochs)) as i32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorTrainTask {
    pub active_fringe: Vec<WavePredictorActiveCenter>,
    pub target_center: WavePredictorCenterId,
    pub nearest_wrong_center: WavePredictorCenterId,
    pub trap_accepted: bool,
}

impl WavePredictorTrainTask {
    fn as_error(
        &self,
        target_gap: i32,
        margin_required: i32,
        trap_accepted: bool,
    ) -> WavePredictorConvergenceError {
        WavePredictorConvergenceError {
            active_fringe: self.active_fringe.clone(),
            target_center: self.target_center,
            nearest_wrong_center: self.nearest_wrong_center,
            target_gap,
            margin_required,
            trap_accepted,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorAxisTarget {
    pub axis_id: u16,
    pub target_center: WavePredictorCenterId,
    pub nearest_wrong_center: WavePredictorCenterId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorCompositionalTrainTask {
    pub active_fringe: Vec<WavePredictorActiveCenter>,
    pub axis_targets: [WavePredictorAxisTarget; WAVEPREDICTOR_TARGET_AXIS_CAP],
    pub axis_len: u8,
    pub trap_accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorStateDeltaTrainTask {
    pub active_fringe: Vec<WavePredictorActiveCenter>,
    pub target_delta: WavePredictorStateDeltaTarget,
    pub binding_output_slot: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorStateImpulse {
    pub lane_id: u16,
    pub signed_strength: i16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WavePredictorStateDeltaTarget {
    pub positive: [WavePredictorStateImpulse; WAVEPREDICTOR_STATE_DELTA_CAP],
    pub positive_len: u8,
    pub negative: [WavePredictorStateImpulse; WAVEPREDICTOR_STATE_DELTA_CAP],
    pub negative_len: u8,
}

impl WavePredictorStateDeltaTarget {
    pub fn from_impulses(
        positive: &[WavePredictorStateImpulse],
        negative: &[WavePredictorStateImpulse],
    ) -> Result<Self, &'static str> {
        if positive.is_empty() {
            return Err("state delta target must contain positive impulses");
        }
        if positive.len() > WAVEPREDICTOR_STATE_DELTA_CAP
            || negative.len() > WAVEPREDICTOR_STATE_DELTA_CAP
        {
            return Err("state delta target exceeds impulse capacity");
        }

        let mut positive_packed =
            [WavePredictorStateImpulse::default(); WAVEPREDICTOR_STATE_DELTA_CAP];
        let mut negative_packed =
            [WavePredictorStateImpulse::default(); WAVEPREDICTOR_STATE_DELTA_CAP];
        positive_packed[..positive.len()].copy_from_slice(positive);
        negative_packed[..negative.len()].copy_from_slice(negative);

        Ok(Self {
            positive: positive_packed,
            positive_len: positive.len() as u8,
            negative: negative_packed,
            negative_len: negative.len() as u8,
        })
    }

    pub fn positive_impulses(&self) -> &[WavePredictorStateImpulse] {
        &self.positive[..usize::from(self.positive_len)]
    }

    pub fn negative_impulses(&self) -> &[WavePredictorStateImpulse] {
        &self.negative[..usize::from(self.negative_len)]
    }
}

impl WavePredictorCompositionalTrainTask {
    pub fn from_axis_targets(
        active_fringe: Vec<WavePredictorActiveCenter>,
        axis_targets: &[WavePredictorAxisTarget],
        trap_accepted: bool,
    ) -> Result<Self, &'static str> {
        if axis_targets.is_empty() {
            return Err("compositional target must contain at least one axis");
        }
        if axis_targets.len() > WAVEPREDICTOR_TARGET_AXIS_CAP {
            return Err("compositional target exceeds axis capacity");
        }

        let mut packed = [WavePredictorAxisTarget::default(); WAVEPREDICTOR_TARGET_AXIS_CAP];
        packed[..axis_targets.len()].copy_from_slice(axis_targets);

        Ok(Self {
            active_fringe,
            axis_targets: packed,
            axis_len: axis_targets.len() as u8,
            trap_accepted,
        })
    }

    pub fn active_axis_targets(&self) -> &[WavePredictorAxisTarget] {
        &self.axis_targets[..usize::from(self.axis_len)]
    }

    fn scalar_task_for_axis(&self, axis: WavePredictorAxisTarget) -> WavePredictorTrainTask {
        WavePredictorTrainTask {
            active_fringe: self.active_fringe.clone(),
            target_center: axis.target_center,
            nearest_wrong_center: axis.nearest_wrong_center,
            trap_accepted: self.trap_accepted,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavePredictorTrainerConfig {
    pub epochs: u16,
    pub margin_schedule: WavePredictorMarginSchedule,
    pub anti_wave_trap_updates_per_epoch_cap: Option<usize>,
}

impl Default for WavePredictorTrainerConfig {
    fn default() -> Self {
        Self {
            epochs: 10,
            margin_schedule: WavePredictorMarginSchedule::default(),
            anti_wave_trap_updates_per_epoch_cap: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorEpochReport {
    pub epoch_index: u16,
    pub margin_required: i32,
    pub train_tasks: usize,
    pub update_steps: usize,
    pub touched_edges: usize,
    pub margin_repairs: usize,
    pub margin_fixed: usize,
    pub trap_updates: usize,
    pub anti_wave_cap_skips: usize,
    pub base_mass_drift_detected: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WavePredictorTrainerReport {
    pub epoch_reports: Vec<WavePredictorEpochReport>,
    pub total_update_steps: usize,
    pub total_touched_edges: usize,
    pub total_margin_repairs: usize,
    pub total_trap_updates: usize,
    pub total_anti_wave_cap_skips: usize,
    pub base_mass_drift_detected: bool,
    pub dynamic_margin_used: bool,
    pub eta_ratio_scheduler_used: bool,
    pub l4_opened: bool,
    pub target_center_id_training_used: bool,
    pub axis_target_id_training_used: bool,
    pub state_delta_training_used: bool,
    pub semantic_grokking_claim_allowed: bool,
}

pub struct WavePredictorTrainer;

impl WavePredictorTrainer {
    pub fn train(
        field: &mut WavePredictorHebbianField,
        tasks: &[WavePredictorTrainTask],
        config: WavePredictorTrainerConfig,
    ) -> WavePredictorTrainerReport {
        let mut report = WavePredictorTrainerReport {
            dynamic_margin_used: config.epochs > 1,
            eta_ratio_scheduler_used: false,
            l4_opened: false,
            target_center_id_training_used: true,
            semantic_grokking_claim_allowed: false,
            ..WavePredictorTrainerReport::default()
        };

        for epoch_index in 0..config.epochs {
            let margin_required = config.margin_schedule.margin_for_epoch(epoch_index);
            let mut epoch = WavePredictorEpochReport {
                epoch_index,
                margin_required,
                train_tasks: tasks.len(),
                ..WavePredictorEpochReport::default()
            };
            let mut anti_trap_updates_this_epoch = 0usize;

            for task in tasks {
                let trap_allowed = if task.trap_accepted {
                    match config.anti_wave_trap_updates_per_epoch_cap {
                        Some(cap) if anti_trap_updates_this_epoch >= cap => {
                            epoch.anti_wave_cap_skips += 1;
                            false
                        }
                        _ => true,
                    }
                } else {
                    false
                };

                let probe = task.as_error(0, margin_required, trap_allowed);
                let target_gap = field.target_gap(&probe);
                let error = task.as_error(target_gap, margin_required, trap_allowed);
                let update = field.apply_sparse_contrastive_error(&error);
                Self::record_update(&mut epoch, &update);

                if task.trap_accepted && update.anti_wave_updates > 0 {
                    anti_trap_updates_this_epoch += 1;
                }
            }

            report.total_update_steps += epoch.update_steps;
            report.total_touched_edges += epoch.touched_edges;
            report.total_margin_repairs += epoch.margin_repairs;
            report.total_trap_updates += epoch.trap_updates;
            report.total_anti_wave_cap_skips += epoch.anti_wave_cap_skips;
            report.base_mass_drift_detected |= epoch.base_mass_drift_detected;
            report.epoch_reports.push(epoch);
        }

        report
    }

    pub fn train_compositional(
        field: &mut WavePredictorHebbianField,
        tasks: &[WavePredictorCompositionalTrainTask],
        config: WavePredictorTrainerConfig,
    ) -> WavePredictorTrainerReport {
        let mut report = WavePredictorTrainerReport {
            dynamic_margin_used: config.epochs > 1,
            eta_ratio_scheduler_used: false,
            l4_opened: false,
            axis_target_id_training_used: true,
            semantic_grokking_claim_allowed: false,
            ..WavePredictorTrainerReport::default()
        };

        for epoch_index in 0..config.epochs {
            let margin_required = config.margin_schedule.margin_for_epoch(epoch_index);
            let mut epoch = WavePredictorEpochReport {
                epoch_index,
                margin_required,
                train_tasks: tasks.len(),
                ..WavePredictorEpochReport::default()
            };
            let mut anti_trap_updates_this_epoch = 0usize;

            for task in tasks {
                for axis in task.active_axis_targets() {
                    let trap_allowed = if task.trap_accepted {
                        match config.anti_wave_trap_updates_per_epoch_cap {
                            Some(cap) if anti_trap_updates_this_epoch >= cap => {
                                epoch.anti_wave_cap_skips += 1;
                                false
                            }
                            _ => true,
                        }
                    } else {
                        false
                    };

                    let scalar_task = task.scalar_task_for_axis(*axis);
                    let probe = scalar_task.as_error(0, margin_required, trap_allowed);
                    let target_gap = field.target_gap(&probe);
                    let error = scalar_task.as_error(target_gap, margin_required, trap_allowed);
                    let update = field.apply_sparse_contrastive_error(&error);
                    Self::record_update(&mut epoch, &update);

                    if task.trap_accepted && update.anti_wave_updates > 0 {
                        anti_trap_updates_this_epoch += 1;
                    }
                }
            }

            report.total_update_steps += epoch.update_steps;
            report.total_touched_edges += epoch.touched_edges;
            report.total_margin_repairs += epoch.margin_repairs;
            report.total_trap_updates += epoch.trap_updates;
            report.total_anti_wave_cap_skips += epoch.anti_wave_cap_skips;
            report.base_mass_drift_detected |= epoch.base_mass_drift_detected;
            report.epoch_reports.push(epoch);
        }

        report
    }

    pub fn train_state_delta(
        field: &mut WavePredictorHebbianField,
        tasks: &[WavePredictorStateDeltaTrainTask],
        config: WavePredictorTrainerConfig,
    ) -> WavePredictorTrainerReport {
        let mut report = WavePredictorTrainerReport {
            dynamic_margin_used: config.epochs > 1,
            eta_ratio_scheduler_used: false,
            l4_opened: false,
            target_center_id_training_used: false,
            axis_target_id_training_used: false,
            state_delta_training_used: true,
            semantic_grokking_claim_allowed: false,
            ..WavePredictorTrainerReport::default()
        };

        for epoch_index in 0..config.epochs {
            let margin_required = config.margin_schedule.margin_for_epoch(epoch_index);
            let mut epoch = WavePredictorEpochReport {
                epoch_index,
                margin_required,
                train_tasks: tasks.len(),
                ..WavePredictorEpochReport::default()
            };

            for task in tasks {
                let update = Self::apply_state_delta_task(field, task, margin_required);
                Self::record_update(&mut epoch, &update);
            }

            report.total_update_steps += epoch.update_steps;
            report.total_touched_edges += epoch.touched_edges;
            report.total_margin_repairs += epoch.margin_repairs;
            report.total_trap_updates += epoch.trap_updates;
            report.total_anti_wave_cap_skips += epoch.anti_wave_cap_skips;
            report.base_mass_drift_detected |= epoch.base_mass_drift_detected;
            report.epoch_reports.push(epoch);
        }

        report
    }

    pub fn state_delta_gap(
        field: &WavePredictorHebbianField,
        task: &WavePredictorStateDeltaTrainTask,
    ) -> i32 {
        let positive_min = task
            .target_delta
            .positive_impulses()
            .iter()
            .map(|impulse| {
                signed_lane_alignment(
                    field,
                    &task.active_fringe,
                    *impulse,
                    task.binding_output_slot,
                )
            })
            .min()
            .unwrap_or(0);
        let negative_max = task
            .target_delta
            .negative_impulses()
            .iter()
            .map(|impulse| {
                signed_lane_alignment(
                    field,
                    &task.active_fringe,
                    *impulse,
                    task.binding_output_slot,
                )
            })
            .max()
            .unwrap_or(0);

        positive_min - negative_max
    }

    pub fn train_state_delta_step(
        field: &mut WavePredictorHebbianField,
        task: &WavePredictorStateDeltaTrainTask,
        margin_required: i32,
    ) -> WavePredictorHebbianUpdateReport {
        Self::apply_state_delta_task(field, task, margin_required)
    }

    fn apply_state_delta_task(
        field: &mut WavePredictorHebbianField,
        task: &WavePredictorStateDeltaTrainTask,
        margin_required: i32,
    ) -> WavePredictorHebbianUpdateReport {
        let gap_before = Self::state_delta_gap(field, task);
        let mut report = WavePredictorHebbianUpdateReport {
            target_gap_before: gap_before,
            margin_required,
            ..WavePredictorHebbianUpdateReport::default()
        };

        for impulse in task.target_delta.positive_impulses() {
            let alignment = signed_lane_alignment(
                field,
                &task.active_fringe,
                *impulse,
                task.binding_output_slot,
            );
            if alignment < margin_required {
                let eta_pos = field.config().eta_pos;
                Self::adjust_delta_lane(
                    field,
                    &task.active_fringe,
                    *impulse,
                    task.binding_output_slot,
                    eta_pos,
                    1,
                    &mut report,
                );
                report.attraction_updates += 1;
            }
        }

        for impulse in task.target_delta.negative_impulses() {
            let alignment = signed_lane_alignment(
                field,
                &task.active_fringe,
                *impulse,
                task.binding_output_slot,
            );
            if alignment > -margin_required {
                let eta_neg = field.config().eta_neg;
                Self::adjust_delta_lane(
                    field,
                    &task.active_fringe,
                    *impulse,
                    task.binding_output_slot,
                    eta_neg,
                    -1,
                    &mut report,
                );
                report.repulsion_updates += 1;
            }
        }

        report.target_gap_after = Self::state_delta_gap(field, task);
        report.margin_fixed = report.target_gap_after >= margin_required;
        report
    }

    fn adjust_delta_lane(
        field: &mut WavePredictorHebbianField,
        active_fringe: &[WavePredictorActiveCenter],
        impulse: WavePredictorStateImpulse,
        binding_output_slot: Option<u8>,
        eta: i16,
        direction: i32,
        report: &mut WavePredictorHebbianUpdateReport,
    ) {
        let sign = impulse_sign(impulse.signed_strength);
        let magnitude = i32::from(impulse.signed_strength).abs().max(1);
        if direction > 0
            && let Some(active_strength) =
                field.state_delta_binding_active_strength(impulse.lane_id, active_fringe)
        {
            let eta_binding = field.config().eta_binding;
            let binding_delta =
                i32::from(eta_binding) * i32::from(active_strength.abs()) * magnitude;
            if field.adjust_state_delta_binding(impulse.signed_strength, binding_delta) {
                report.touched_edges += 1;
            }
        }
        let role_binding_delta = direction * i32::from(field.config().eta_binding) * magnitude;
        report.touched_edges += field.adjust_state_delta_role_binding(
            impulse.lane_id,
            impulse.signed_strength,
            active_fringe,
            binding_output_slot,
            role_binding_delta,
        );

        for active in active_fringe {
            let active_strength = active.strength.max(0);
            if active_strength == 0 {
                continue;
            }
            let delta = direction * sign * i32::from(eta) * i32::from(active_strength) * magnitude;
            if field.adjust_state_delta_edge(active.center_id, impulse.lane_id, delta) {
                report.touched_edges += 1;
            }
        }
    }

    fn record_update(
        epoch: &mut WavePredictorEpochReport,
        update: &WavePredictorHebbianUpdateReport,
    ) {
        if update.touched_edges == 0 {
            return;
        }

        epoch.update_steps += 1;
        epoch.touched_edges += update.touched_edges;
        epoch.margin_repairs += usize::from(update.attraction_updates > 0);
        epoch.margin_fixed += usize::from(update.margin_fixed);
        epoch.trap_updates += usize::from(update.anti_wave_updates > 0);
        epoch.base_mass_drift_detected |= update.base_mass_drift_detected;
    }
}

fn signed_lane_alignment(
    field: &WavePredictorHebbianField,
    active_fringe: &[WavePredictorActiveCenter],
    impulse: WavePredictorStateImpulse,
    binding_output_slot: Option<u8>,
) -> i32 {
    impulse_sign(impulse.signed_strength)
        * field.score_state_delta_lane(impulse.lane_id, active_fringe)
        + field.score_state_delta_binding_alignment(
            impulse.lane_id,
            impulse.signed_strength,
            active_fringe,
            binding_output_slot,
        )
}

fn impulse_sign(strength: i16) -> i32 {
    if strength < 0 { -1 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::{
        WavePredictorHebbianConfig, WavePredictorHebbianEdge, WavePredictorHebbianField,
    };

    #[test]
    fn margin_schedule_ramps_after_warmup_and_clamps_at_target() {
        let schedule = WavePredictorMarginSchedule {
            start_margin: 20,
            target_margin: 100,
            warmup_epochs: 2,
            ramp_epochs: 4,
        };

        assert_eq!(schedule.margin_for_epoch(0), 20);
        assert_eq!(schedule.margin_for_epoch(1), 20);
        assert_eq!(schedule.margin_for_epoch(2), 40);
        assert_eq!(schedule.margin_for_epoch(3), 60);
        assert_eq!(schedule.margin_for_epoch(4), 80);
        assert_eq!(schedule.margin_for_epoch(5), 100);
        assert_eq!(schedule.margin_for_epoch(6), 100);
    }

    #[test]
    fn trainer_loop_repairs_margin_without_eta_ratio_or_base_mass_drift() {
        let mut field = WavePredictorHebbianField::new(4, WavePredictorHebbianConfig::default());
        field.set_base_mass(1, 77);
        field.insert_edge(WavePredictorHebbianEdge {
            source_center: 0,
            target_center: 2,
            compatibility: 10,
            ..WavePredictorHebbianEdge::default()
        });

        let task = WavePredictorTrainTask {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 8,
            }],
            target_center: 1,
            nearest_wrong_center: 2,
            trap_accepted: false,
        };
        let config = WavePredictorTrainerConfig {
            epochs: 5,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 16,
                target_margin: 160,
                warmup_epochs: 1,
                ramp_epochs: 4,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        };

        let report = WavePredictorTrainer::train(&mut field, std::slice::from_ref(&task), config);
        let final_gap = field.target_gap(&task.as_error(0, 160, false));

        assert!(final_gap >= 160, "final_gap={final_gap} report={report:#?}");
        assert!(report.total_update_steps > 0, "report={report:#?}");
        assert!(report.total_margin_repairs > 0, "report={report:#?}");
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
        assert!(report.dynamic_margin_used, "report={report:#?}");
        assert!(!report.eta_ratio_scheduler_used, "report={report:#?}");
        assert!(!report.l4_opened, "report={report:#?}");
        assert!(report.target_center_id_training_used, "report={report:#?}");
        assert!(
            !report.semantic_grokking_claim_allowed,
            "report={report:#?}"
        );
        assert_eq!(field.base_mass(1), Some(77));
    }

    #[test]
    fn optional_anti_wave_brake_caps_trap_pressure_per_epoch() {
        let mut field = WavePredictorHebbianField::new(5, WavePredictorHebbianConfig::default());
        let tasks = vec![
            WavePredictorTrainTask {
                active_fringe: vec![WavePredictorActiveCenter {
                    center_id: 0,
                    strength: 1,
                }],
                target_center: 1,
                nearest_wrong_center: 2,
                trap_accepted: true,
            },
            WavePredictorTrainTask {
                active_fringe: vec![WavePredictorActiveCenter {
                    center_id: 3,
                    strength: 1,
                }],
                target_center: 1,
                nearest_wrong_center: 4,
                trap_accepted: true,
            },
        ];
        let config = WavePredictorTrainerConfig {
            epochs: 1,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: -1_000,
                target_margin: -1_000,
                warmup_epochs: 0,
                ramp_epochs: 1,
            },
            anti_wave_trap_updates_per_epoch_cap: Some(1),
        };

        let report = WavePredictorTrainer::train(&mut field, &tasks, config);

        assert_eq!(report.total_trap_updates, 1);
        assert_eq!(report.total_anti_wave_cap_skips, 1);
        assert!(
            field
                .edge(0, 2)
                .expect("first trap edge should receive anti-wave")
                .anti_wave
                > 0
        );
        assert_eq!(field.edge(3, 4), None);
        assert!(!report.eta_ratio_scheduler_used);
        assert!(report.target_center_id_training_used);
        assert!(!report.semantic_grokking_claim_allowed);
    }

    #[test]
    fn compositional_trainer_repairs_each_axis_without_opening_l4() {
        let mut field = WavePredictorHebbianField::new(8, WavePredictorHebbianConfig::default());
        let task = WavePredictorCompositionalTrainTask::from_axis_targets(
            vec![WavePredictorActiveCenter {
                center_id: 0,
                strength: 6,
            }],
            &[
                WavePredictorAxisTarget {
                    axis_id: 0,
                    target_center: 1,
                    nearest_wrong_center: 2,
                },
                WavePredictorAxisTarget {
                    axis_id: 1,
                    target_center: 3,
                    nearest_wrong_center: 4,
                },
            ],
            false,
        )
        .expect("two-axis target should fit");
        let config = WavePredictorTrainerConfig {
            epochs: 4,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 24,
                target_margin: 96,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        };

        let report = WavePredictorTrainer::train_compositional(&mut field, &[task], config);

        assert!(report.total_margin_repairs >= 2, "report={report:#?}");
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
        assert!(!report.eta_ratio_scheduler_used, "report={report:#?}");
        assert!(!report.l4_opened, "report={report:#?}");
        assert!(report.axis_target_id_training_used, "report={report:#?}");
        assert!(
            !report.semantic_grokking_claim_allowed,
            "report={report:#?}"
        );
        assert!(
            field
                .edge(0, 1)
                .expect("axis 0 target edge should be created")
                .compatibility
                > 0
        );
        assert!(
            field
                .edge(0, 3)
                .expect("axis 1 target edge should be created")
                .compatibility
                > 0
        );
    }

    #[test]
    fn state_delta_trainer_updates_lanes_without_target_center_ids() {
        let mut field = WavePredictorHebbianField::new(8, WavePredictorHebbianConfig::default());
        field.set_base_mass(0, 55);
        let target_delta = WavePredictorStateDeltaTarget::from_impulses(
            &[WavePredictorStateImpulse {
                lane_id: 120,
                signed_strength: 2,
            }],
            &[WavePredictorStateImpulse {
                lane_id: 981,
                signed_strength: 2,
            }],
        )
        .expect("state delta target should fit");
        let task = WavePredictorStateDeltaTrainTask {
            active_fringe: vec![WavePredictorActiveCenter {
                center_id: 3,
                strength: 4,
            }],
            target_delta,
            binding_output_slot: None,
        };
        let config = WavePredictorTrainerConfig {
            epochs: 4,
            margin_schedule: WavePredictorMarginSchedule {
                start_margin: 16,
                target_margin: 96,
                warmup_epochs: 1,
                ramp_epochs: 3,
            },
            anti_wave_trap_updates_per_epoch_cap: None,
        };

        let report = WavePredictorTrainer::train_state_delta(
            &mut field,
            std::slice::from_ref(&task),
            config,
        );

        assert!(report.state_delta_training_used, "report={report:#?}");
        assert!(!report.target_center_id_training_used, "report={report:#?}");
        assert!(!report.axis_target_id_training_used, "report={report:#?}");
        assert!(
            !report.semantic_grokking_claim_allowed,
            "report={report:#?}"
        );
        assert!(!report.base_mass_drift_detected, "report={report:#?}");
        assert_eq!(field.base_mass(0), Some(55));
        assert_eq!(field.edge_count(), 0);
        assert!(field.state_delta_edge_count() > 0);
        assert!(
            field.score_state_delta_lane(120, &task.active_fringe) > 0,
            "target lane should be amplified"
        );
        assert!(
            field.score_state_delta_lane(981, &task.active_fringe) < 0,
            "near-negative lane should be suppressed"
        );
        assert!(WavePredictorTrainer::state_delta_gap(&field, &task) >= 96);
    }

    #[test]
    fn state_delta_target_stores_wave_impulses_without_target_centers() {
        let target = WavePredictorStateDeltaTarget::from_impulses(
            &[
                WavePredictorStateImpulse {
                    lane_id: 11,
                    signed_strength: 7,
                },
                WavePredictorStateImpulse {
                    lane_id: 42,
                    signed_strength: -3,
                },
            ],
            &[WavePredictorStateImpulse {
                lane_id: 99,
                signed_strength: -9,
            }],
        )
        .expect("state delta target should fit");

        assert_eq!(target.positive_impulses().len(), 2);
        assert_eq!(target.negative_impulses().len(), 1);
        assert_eq!(target.positive_impulses()[0].lane_id, 11);
        assert_eq!(target.negative_impulses()[0].signed_strength, -9);
    }
}
