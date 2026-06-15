use crate::{BaselineResult, format_baseline};
use nando_core::STAGE2_ORGAN_CELLS;

use super::indent_report;

/// Fixed synthetic periodic task configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeriodicTaskConfig {
    pub seed: u64,
    pub cases: usize,
    pub start: u8,
    pub step: u8,
}

/// Synthetic phase-composition task configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseCompositionConfig {
    pub seed: u64,
    pub cases: usize,
    pub start: u8,
    pub input_step: u8,
    pub phase_step: u8,
}

impl Default for PhaseCompositionConfig {
    fn default() -> Self {
        Self {
            seed: 13,
            cases: 64,
            start: 19,
            input_step: 23,
            phase_step: 5,
        }
    }
}

impl Default for PeriodicTaskConfig {
    fn default() -> Self {
        Self {
            seed: 7,
            cases: 64,
            start: 11,
            step: 17,
        }
    }
}

/// First Stage 3 report. It is intentionally small and line-oriented.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodicEvalReport {
    pub config: PeriodicTaskConfig,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_bus: BaselineResult,
    pub voting: BaselineResult,
    pub wave_bus: BaselineResult,
    pub ablations: [BaselineResult; STAGE2_ORGAN_CELLS],
    pub ablation_drop: f32,
    pub key_cell: u32,
    pub best_baseline: &'static str,
    pub mode_status: &'static str,
}

/// Stage 3/4 synthetic task that explicitly requires phase composition.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseCompositionReport {
    pub config: PhaseCompositionConfig,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_bus: BaselineResult,
    pub voting: BaselineResult,
    pub wave_bus: BaselineResult,
    pub ablations: [BaselineResult; STAGE2_ORGAN_CELLS],
    pub ablation_drop: f32,
    pub key_cell: u32,
    pub best_baseline: &'static str,
    pub mode_status: &'static str,
}

/// Train/holdout check for a phase-composition candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct PhaseHoldoutReport {
    pub train: PhaseCompositionReport,
    pub holdout: PhaseCompositionReport,
    pub wave_advantage_train: f32,
    pub wave_advantage_holdout: f32,
    pub min_ablation_drop: f32,
    pub mode_status: &'static str,
}

/// CarrierWave control for the phase-composition candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct CarrierControlReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub correct_carrier: BaselineResult,
    pub no_carrier: BaselineResult,
    pub wrong_carrier: BaselineResult,
    pub corrupted_carrier: BaselineResult,
    pub correct_over_no: f32,
    pub correct_over_wrong: f32,
    pub correct_over_corrupted: f32,
    pub mode_status: &'static str,
}

/// Delayed bus-only probe: decoder sees WaveBus center phase, not CarrierWave phase.
#[derive(Debug, Clone, PartialEq)]
pub struct BusTransferReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_bus: BaselineResult,
    pub voting: BaselineResult,
    pub correct_carrier_bus: BaselineResult,
    pub no_carrier_bus: BaselineResult,
    pub wrong_carrier_bus: BaselineResult,
    pub corrupted_carrier_bus: BaselineResult,
    pub ablations: [BaselineResult; STAGE2_ORGAN_CELLS],
    pub ablation_drop: f32,
    pub key_cell: u32,
    pub correct_over_best_baseline: f32,
    pub correct_over_wrong_carrier: f32,
    pub mode_status: &'static str,
}

/// Snapshot-memory probe: replay a serialized wave-state snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotMemoryReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub warm_over_no_snapshot: f32,
    pub warm_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Snapshot-transition probe: use previous snapshot to predict next wave-state.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotTransitionReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub warm_over_no_snapshot: f32,
    pub warm_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Snapshot-dynamics probe over a smooth CarrierWave sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotDynamicsReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub warm_error_gain_over_no: f32,
    pub warm_error_gain_over_wrong: f32,
    pub mode_status: &'static str,
}

/// Multi-tick snapshot probe: keep one snapshot warm while CarrierWave evolves.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotMultiTickReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub warm_error_gain_over_no: f32,
    pub warm_error_gain_over_wrong: f32,
    pub mode_status: &'static str,
}

/// Online local-adaptation probe after snapshot/multitick state.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotAdaptReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub learning_rate: f32,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub adapted_no_snapshot: BaselineResult,
    pub adapted_snapshot: BaselineResult,
    pub adapted_wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub adapted_error_gain_over_warm: f32,
    pub adapted_error_gain_over_no_adapt: f32,
    pub adapted_error_gain_over_wrong_adapt: f32,
    pub mode_status: &'static str,
}

/// Online transition decoder probe using snapshot-derived phase features.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotDecoderReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub learning_rate: f32,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub warm_snapshot: BaselineResult,
    pub decoder_no_snapshot: BaselineResult,
    pub decoder_snapshot: BaselineResult,
    pub decoder_wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub decoder_error_gain_over_warm: f32,
    pub decoder_error_gain_over_no_decoder: f32,
    pub decoder_error_gain_over_wrong_decoder: f32,
    pub mode_status: &'static str,
}

/// Snapshot-private-state probe: target carries state unavailable to no-snapshot controls.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotKeyedReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub no_snapshot: BaselineResult,
    pub keyed_snapshot: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub keyed_over_no_snapshot: f32,
    pub keyed_over_wrong_snapshot: f32,
    pub keyed_error_gain_over_no: f32,
    pub mode_status: &'static str,
}

/// Keyed transition probe: future wave-state must combine with snapshot-private state.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotKeyedTransitionReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub future_only: BaselineResult,
    pub keyed_transition: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub keyed_over_future_only: f32,
    pub keyed_over_wrong_snapshot: f32,
    pub keyed_error_gain_over_future_only: f32,
    pub mode_status: &'static str,
}

/// Noisy keyed transition probe: snapshot state helps without directly exposing the target.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotNoisyKeyedTransitionReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub horizon: usize,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub future_only: BaselineResult,
    pub keyed_transition: BaselineResult,
    pub wrong_snapshot: BaselineResult,
    pub corrupted_snapshot: BaselineResult,
    pub keyed_accuracy_over_future_only: f32,
    pub keyed_error_gain_over_future_only: f32,
    pub keyed_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Compact per-horizon row for noisy keyed transition sweep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HorizonSweepRow {
    pub horizon: usize,
    pub future_only_accuracy: f32,
    pub keyed_accuracy: f32,
    pub wrong_accuracy: f32,
    pub corrupted_accuracy: f32,
    pub keyed_accuracy_over_future_only: f32,
    pub keyed_error_gain_over_future_only: f32,
    pub keyed_error_gain_over_wrong_snapshot: f32,
    pub passed: bool,
}

/// Sweep noisy keyed transition across several future horizons.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotNoisyKeyedTransitionSweepReport {
    pub train_config: PhaseCompositionConfig,
    pub holdout_config: PhaseCompositionConfig,
    pub snapshot_bytes: usize,
    pub rows: [HorizonSweepRow; 4],
    pub passed_count: usize,
    pub min_keyed_accuracy_over_future_only: f32,
    pub min_error_gain_over_future_only: f32,
    pub min_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Per-seed-pair row for noisy keyed transition robustness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeedSweepRow {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub passed_count: usize,
    pub min_keyed_accuracy_over_future_only: f32,
    pub min_error_gain_over_future_only: f32,
    pub min_error_gain_over_wrong_snapshot: f32,
    pub passed: bool,
}

/// Sweep noisy keyed transition across several seed pairs and horizons.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotNoisyKeyedTransitionSeedSweepReport {
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub rows: [SeedSweepRow; 4],
    pub passed_seed_pairs: usize,
    pub min_keyed_accuracy_over_future_only: f32,
    pub min_error_gain_over_future_only: f32,
    pub min_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

impl SnapshotNoisyKeyedTransitionSeedSweepReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-noisy-keyed-transition-seed-sweep eval\n");
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        for (index, row) in self.rows.iter().enumerate() {
            output.push_str(&format!(
                concat!(
                    "seed_pair_{}.train_seed: {}\n",
                    "seed_pair_{}.holdout_seed: {}\n",
                    "seed_pair_{}.passed_count: {}\n",
                    "seed_pair_{}.min_keyed_accuracy_over_future_only: {:.6}\n",
                    "seed_pair_{}.min_error_gain_over_future_only: {:.6}\n",
                    "seed_pair_{}.min_error_gain_over_wrong_snapshot: {:.6}\n",
                    "seed_pair_{}.passed: {}\n"
                ),
                index,
                row.train_seed,
                index,
                row.holdout_seed,
                index,
                row.passed_count,
                index,
                row.min_keyed_accuracy_over_future_only,
                index,
                row.min_error_gain_over_future_only,
                index,
                row.min_error_gain_over_wrong_snapshot,
                index,
                row.passed
            ));
        }
        output.push_str(&format!("passed_seed_pairs: {}\n", self.passed_seed_pairs));
        output.push_str(&format!(
            "min_keyed_accuracy_over_future_only: {:.6}\n",
            self.min_keyed_accuracy_over_future_only
        ));
        output.push_str(&format!(
            "min_error_gain_over_future_only: {:.6}\n",
            self.min_error_gain_over_future_only
        ));
        output.push_str(&format!(
            "min_error_gain_over_wrong_snapshot: {:.6}\n",
            self.min_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotNoisyKeyedTransitionSweepReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-noisy-keyed-transition-sweep eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        for row in self.rows {
            output.push_str(&format!(
                concat!(
                    "horizon_{}.future_only_accuracy: {:.6}\n",
                    "horizon_{}.keyed_accuracy: {:.6}\n",
                    "horizon_{}.wrong_accuracy: {:.6}\n",
                    "horizon_{}.corrupted_accuracy: {:.6}\n",
                    "horizon_{}.keyed_accuracy_over_future_only: {:.6}\n",
                    "horizon_{}.keyed_error_gain_over_future_only: {:.6}\n",
                    "horizon_{}.keyed_error_gain_over_wrong_snapshot: {:.6}\n",
                    "horizon_{}.passed: {}\n"
                ),
                row.horizon,
                row.future_only_accuracy,
                row.horizon,
                row.keyed_accuracy,
                row.horizon,
                row.wrong_accuracy,
                row.horizon,
                row.corrupted_accuracy,
                row.horizon,
                row.keyed_accuracy_over_future_only,
                row.horizon,
                row.keyed_error_gain_over_future_only,
                row.horizon,
                row.keyed_error_gain_over_wrong_snapshot,
                row.horizon,
                row.passed
            ));
        }
        output.push_str(&format!("passed_count: {}\n", self.passed_count));
        output.push_str(&format!(
            "min_keyed_accuracy_over_future_only: {:.6}\n",
            self.min_keyed_accuracy_over_future_only
        ));
        output.push_str(&format!(
            "min_error_gain_over_future_only: {:.6}\n",
            self.min_error_gain_over_future_only
        ));
        output.push_str(&format!(
            "min_error_gain_over_wrong_snapshot: {:.6}\n",
            self.min_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotNoisyKeyedTransitionReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-noisy-keyed-transition eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.future_only));
        output.push_str(&format_baseline(self.keyed_transition));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "keyed_accuracy_over_future_only: {:.6}\n",
            self.keyed_accuracy_over_future_only
        ));
        output.push_str(&format!(
            "keyed_error_gain_over_future_only: {:.6}\n",
            self.keyed_error_gain_over_future_only
        ));
        output.push_str(&format!(
            "keyed_error_gain_over_wrong_snapshot: {:.6}\n",
            self.keyed_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotKeyedTransitionReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-keyed-transition eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.future_only));
        output.push_str(&format_baseline(self.keyed_transition));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "keyed_over_future_only: {:.6}\n",
            self.keyed_over_future_only
        ));
        output.push_str(&format!(
            "keyed_over_wrong_snapshot: {:.6}\n",
            self.keyed_over_wrong_snapshot
        ));
        output.push_str(&format!(
            "keyed_error_gain_over_future_only: {:.6}\n",
            self.keyed_error_gain_over_future_only
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotKeyedReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-keyed eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.keyed_snapshot));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "keyed_over_no_snapshot: {:.6}\n",
            self.keyed_over_no_snapshot
        ));
        output.push_str(&format!(
            "keyed_over_wrong_snapshot: {:.6}\n",
            self.keyed_over_wrong_snapshot
        ));
        output.push_str(&format!(
            "keyed_error_gain_over_no: {:.6}\n",
            self.keyed_error_gain_over_no
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotDecoderReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-decoder eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format!("learning_rate: {:.6}\n", self.learning_rate));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.decoder_no_snapshot));
        output.push_str(&format_baseline(self.decoder_snapshot));
        output.push_str(&format_baseline(self.decoder_wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "decoder_error_gain_over_warm: {:.6}\n",
            self.decoder_error_gain_over_warm
        ));
        output.push_str(&format!(
            "decoder_error_gain_over_no_decoder: {:.6}\n",
            self.decoder_error_gain_over_no_decoder
        ));
        output.push_str(&format!(
            "decoder_error_gain_over_wrong_decoder: {:.6}\n",
            self.decoder_error_gain_over_wrong_decoder
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotAdaptReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-adapt eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format!("learning_rate: {:.6}\n", self.learning_rate));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.adapted_no_snapshot));
        output.push_str(&format_baseline(self.adapted_snapshot));
        output.push_str(&format_baseline(self.adapted_wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "adapted_error_gain_over_warm: {:.6}\n",
            self.adapted_error_gain_over_warm
        ));
        output.push_str(&format!(
            "adapted_error_gain_over_no_adapt: {:.6}\n",
            self.adapted_error_gain_over_no_adapt
        ));
        output.push_str(&format!(
            "adapted_error_gain_over_wrong_adapt: {:.6}\n",
            self.adapted_error_gain_over_wrong_adapt
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotMultiTickReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-multitick eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format!("horizon: {}\n", self.horizon));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "warm_error_gain_over_no: {:.6}\n",
            self.warm_error_gain_over_no
        ));
        output.push_str(&format!(
            "warm_error_gain_over_wrong: {:.6}\n",
            self.warm_error_gain_over_wrong
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotDynamicsReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-dynamics eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "warm_error_gain_over_no: {:.6}\n",
            self.warm_error_gain_over_no
        ));
        output.push_str(&format!(
            "warm_error_gain_over_wrong: {:.6}\n",
            self.warm_error_gain_over_wrong
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotTransitionReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-transition eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "warm_over_no_snapshot: {:.6}\n",
            self.warm_over_no_snapshot
        ));
        output.push_str(&format!(
            "warm_over_wrong_snapshot: {:.6}\n",
            self.warm_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl SnapshotMemoryReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave snapshot-memory eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_snapshot));
        output.push_str(&format_baseline(self.warm_snapshot));
        output.push_str(&format_baseline(self.wrong_snapshot));
        output.push_str(&format_baseline(self.corrupted_snapshot));
        output.push_str(&format!(
            "warm_over_no_snapshot: {:.6}\n",
            self.warm_over_no_snapshot
        ));
        output.push_str(&format!(
            "warm_over_wrong_snapshot: {:.6}\n",
            self.warm_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl BusTransferReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave delayed bus-transfer eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_bus));
        output.push_str(&format_baseline(self.voting));
        output.push_str(&format_baseline(self.correct_carrier_bus));
        output.push_str(&format_baseline(self.no_carrier_bus));
        output.push_str(&format_baseline(self.wrong_carrier_bus));
        output.push_str(&format_baseline(self.corrupted_carrier_bus));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!("key_cell: {}\n", self.key_cell));
        output.push_str(&format!("ablation_drop: {:.6}\n", self.ablation_drop));
        output.push_str(&format!(
            "correct_over_best_baseline: {:.6}\n",
            self.correct_over_best_baseline
        ));
        output.push_str(&format!(
            "correct_over_wrong_carrier: {:.6}\n",
            self.correct_over_wrong_carrier
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl CarrierControlReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave carrier-control eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_config.seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_config.seed));
        output.push_str(&format!("cases_per_split: {}\n", self.train_config.cases));
        output.push_str(&format_baseline(self.correct_carrier));
        output.push_str(&format_baseline(self.no_carrier));
        output.push_str(&format_baseline(self.wrong_carrier));
        output.push_str(&format_baseline(self.corrupted_carrier));
        output.push_str(&format!("correct_over_no: {:.6}\n", self.correct_over_no));
        output.push_str(&format!(
            "correct_over_wrong: {:.6}\n",
            self.correct_over_wrong
        ));
        output.push_str(&format!(
            "correct_over_corrupted: {:.6}\n",
            self.correct_over_corrupted
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl PhaseHoldoutReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave phase-composition holdout\n");
        output.push_str("train:\n");
        output.push_str(&indent_report(&self.train.to_text()));
        output.push_str("holdout:\n");
        output.push_str(&indent_report(&self.holdout.to_text()));
        output.push_str(&format!(
            "wave_advantage_train: {:.6}\n",
            self.wave_advantage_train
        ));
        output.push_str(&format!(
            "wave_advantage_holdout: {:.6}\n",
            self.wave_advantage_holdout
        ));
        output.push_str(&format!(
            "min_ablation_drop: {:.6}\n",
            self.min_ablation_drop
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl PhaseCompositionReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave phase-composition eval\n");
        output.push_str(&format!("seed: {}\n", self.config.seed));
        output.push_str(&format!("cases: {}\n", self.config.cases));
        output.push_str(&format!("start: {}\n", self.config.start));
        output.push_str(&format!("input_step: {}\n", self.config.input_step));
        output.push_str(&format!("phase_step: {}\n", self.config.phase_step));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_bus));
        output.push_str(&format_baseline(self.voting));
        output.push_str(&format_baseline(self.wave_bus));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!("key_cell: {}\n", self.key_cell));
        output.push_str(&format!("ablation_drop: {:.6}\n", self.ablation_drop));
        output.push_str(&format!("best_baseline: {}\n", self.best_baseline));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl PeriodicEvalReport {
    /// Render a stable report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave periodic eval\n");
        output.push_str(&format!("seed: {}\n", self.config.seed));
        output.push_str(&format!("cases: {}\n", self.config.cases));
        output.push_str(&format!("start: {}\n", self.config.start));
        output.push_str(&format!("step: {}\n", self.config.step));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.no_bus));
        output.push_str(&format_baseline(self.voting));
        output.push_str(&format_baseline(self.wave_bus));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!("key_cell: {}\n", self.key_cell));
        output.push_str(&format!("ablation_drop: {:.6}\n", self.ablation_drop));
        output.push_str(&format!("best_baseline: {}\n", self.best_baseline));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}
