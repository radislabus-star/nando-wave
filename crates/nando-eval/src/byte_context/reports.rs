use crate::{BaselineResult, format_baseline};

use crate::BYTE_CONTEXT_TASKS;

/// First byte-stream bridge toward Chat-0.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub train_cases: usize,
    pub holdout_cases: usize,
    pub snapshot_bytes: usize,
    pub random: BaselineResult,
    pub mono192_prompt_decoder: BaselineResult,
    pub no_snapshot_decoder: BaselineResult,
    pub cell32_voting: BaselineResult,
    pub snapshot_decoder: BaselineResult,
    pub wrong_snapshot_decoder: BaselineResult,
    pub corrupted_snapshot_decoder: BaselineResult,
    pub snapshot_accuracy_over_best_control: f32,
    pub snapshot_error_gain_over_best_control: f32,
    pub snapshot_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Prototype classifier probe for byte-level context modes.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextCentroidReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub train_cases: usize,
    pub holdout_cases: usize,
    pub snapshot_bytes: usize,
    pub random: BaselineResult,
    pub mono192_prompt_centroid: BaselineResult,
    pub no_snapshot_centroid: BaselineResult,
    pub cell32_voting: BaselineResult,
    pub snapshot_centroid: BaselineResult,
    pub wrong_snapshot_centroid: BaselineResult,
    pub corrupted_snapshot_centroid: BaselineResult,
    pub snapshot_accuracy_over_best_control: f32,
    pub snapshot_error_gain_over_best_control: f32,
    pub snapshot_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Per-seed-pair row for byte-context centroid robustness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ByteContextCentroidSeedRow {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub snapshot_accuracy: f32,
    pub best_control_accuracy: f32,
    pub wrong_snapshot_accuracy: f32,
    pub corrupted_snapshot_accuracy: f32,
    pub snapshot_accuracy_over_best_control: f32,
    pub snapshot_error_gain_over_best_control: f32,
    pub snapshot_error_gain_over_wrong_snapshot: f32,
    pub passed: bool,
}

/// Seed sweep for the first byte-context centroid candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextCentroidSeedSweepReport {
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub rows: [ByteContextCentroidSeedRow; 4],
    pub passed_seed_pairs: usize,
    pub min_snapshot_accuracy_over_best_control: f32,
    pub min_error_gain_over_best_control: f32,
    pub min_error_gain_over_wrong_snapshot: f32,
    pub mode_status: &'static str,
}

/// Feature ablation report for the byte-context centroid candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextCentroidAblationReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub full_snapshot: BaselineResult,
    pub ablations: [BaselineResult; 5],
    pub key_feature: &'static str,
    pub max_accuracy_drop: f32,
    pub max_error_increase: f32,
    pub mode_status: &'static str,
}

/// Ablation report for the cellular CarrierWave lock cells.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextCellularCarrierAblationReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub full_snapshot: BaselineResult,
    pub ablations: [BaselineResult; BYTE_CONTEXT_TASKS.len()],
    pub min_accuracy_drop: f32,
    pub max_error_increase: f32,
    pub mode_status: &'static str,
}

/// Ablation report for supervised harmonic CarrierWave lock cells.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextTrainedCarrierAblationReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub full_snapshot: BaselineResult,
    pub ablations: [BaselineResult; BYTE_CONTEXT_TASKS.len()],
    pub min_accuracy_drop: f32,
    pub max_error_increase: f32,
    pub mode_status: &'static str,
}

/// Ablation report for prompt-cloud harmonic CarrierWave lock cells.
#[derive(Debug, Clone, PartialEq)]
pub struct ByteContextPromptCarrierAblationReport {
    pub train_seed: u64,
    pub holdout_seed: u64,
    pub cases_per_split: usize,
    pub snapshot_bytes: usize,
    pub full_snapshot: BaselineResult,
    pub all_disabled: BaselineResult,
    pub ablations: [BaselineResult; BYTE_CONTEXT_TASKS.len()],
    pub min_accuracy_drop: f32,
    pub max_accuracy_drop: f32,
    pub max_error_increase: f32,
    pub accuracy_over_all_disabled: f32,
    pub error_gain_over_all_disabled: f32,
    pub mode_status: &'static str,
}

impl ByteContextPromptCarrierAblationReport {
    /// Render a stable report for prompt-cloud CarrierWave lock ablation.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-prompt-carrier-ablation eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.full_snapshot));
        output.push_str(&format_baseline(self.all_disabled));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!(
            "min_accuracy_drop: {:.6}\n",
            self.min_accuracy_drop
        ));
        output.push_str(&format!(
            "max_accuracy_drop: {:.6}\n",
            self.max_accuracy_drop
        ));
        output.push_str(&format!(
            "max_error_increase: {:.6}\n",
            self.max_error_increase
        ));
        output.push_str(&format!(
            "accuracy_over_all_disabled: {:.6}\n",
            self.accuracy_over_all_disabled
        ));
        output.push_str(&format!(
            "error_gain_over_all_disabled: {:.6}\n",
            self.error_gain_over_all_disabled
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextTrainedCarrierAblationReport {
    /// Render a stable report for trained CarrierWave lock ablation.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-trained-carrier-ablation eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.full_snapshot));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!(
            "min_accuracy_drop: {:.6}\n",
            self.min_accuracy_drop
        ));
        output.push_str(&format!(
            "max_error_increase: {:.6}\n",
            self.max_error_increase
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextCellularCarrierAblationReport {
    /// Render a stable report for cellular CarrierWave lock ablation.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-cellular-carrier-ablation eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.full_snapshot));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!(
            "min_accuracy_drop: {:.6}\n",
            self.min_accuracy_drop
        ));
        output.push_str(&format!(
            "max_error_increase: {:.6}\n",
            self.max_error_increase
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextCentroidAblationReport {
    /// Render a stable report for byte-context centroid feature ablation.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-centroid-ablation eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.full_snapshot));
        for ablation in self.ablations {
            output.push_str(&format_baseline(ablation));
        }
        output.push_str(&format!("key_feature: {}\n", self.key_feature));
        output.push_str(&format!(
            "max_accuracy_drop: {:.6}\n",
            self.max_accuracy_drop
        ));
        output.push_str(&format!(
            "max_error_increase: {:.6}\n",
            self.max_error_increase
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextCentroidSeedSweepReport {
    /// Render a stable report for the byte-context centroid seed sweep.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-centroid-seed-sweep eval\n");
        output.push_str(&format!("cases_per_split: {}\n", self.cases_per_split));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        for (index, row) in self.rows.iter().enumerate() {
            output.push_str(&format!(
                concat!(
                    "seed_pair_{}.train_seed: {}\n",
                    "seed_pair_{}.holdout_seed: {}\n",
                    "seed_pair_{}.snapshot_accuracy: {:.6}\n",
                    "seed_pair_{}.best_control_accuracy: {:.6}\n",
                    "seed_pair_{}.wrong_snapshot_accuracy: {:.6}\n",
                    "seed_pair_{}.corrupted_snapshot_accuracy: {:.6}\n",
                    "seed_pair_{}.snapshot_accuracy_over_best_control: {:.6}\n",
                    "seed_pair_{}.snapshot_error_gain_over_best_control: {:.6}\n",
                    "seed_pair_{}.snapshot_error_gain_over_wrong_snapshot: {:.6}\n",
                    "seed_pair_{}.passed: {}\n"
                ),
                index,
                row.train_seed,
                index,
                row.holdout_seed,
                index,
                row.snapshot_accuracy,
                index,
                row.best_control_accuracy,
                index,
                row.wrong_snapshot_accuracy,
                index,
                row.corrupted_snapshot_accuracy,
                index,
                row.snapshot_accuracy_over_best_control,
                index,
                row.snapshot_error_gain_over_best_control,
                index,
                row.snapshot_error_gain_over_wrong_snapshot,
                index,
                row.passed
            ));
        }
        output.push_str(&format!("passed_seed_pairs: {}\n", self.passed_seed_pairs));
        output.push_str(&format!(
            "min_snapshot_accuracy_over_best_control: {:.6}\n",
            self.min_snapshot_accuracy_over_best_control
        ));
        output.push_str(&format!(
            "min_error_gain_over_best_control: {:.6}\n",
            self.min_error_gain_over_best_control
        ));
        output.push_str(&format!(
            "min_error_gain_over_wrong_snapshot: {:.6}\n",
            self.min_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextCentroidReport {
    /// Render a stable report for the byte-context prototype probe.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context-centroid eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("train_cases: {}\n", self.train_cases));
        output.push_str(&format!("holdout_cases: {}\n", self.holdout_cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192_prompt_centroid));
        output.push_str(&format_baseline(self.no_snapshot_centroid));
        output.push_str(&format_baseline(self.cell32_voting));
        output.push_str(&format_baseline(self.snapshot_centroid));
        output.push_str(&format_baseline(self.wrong_snapshot_centroid));
        output.push_str(&format_baseline(self.corrupted_snapshot_centroid));
        output.push_str(&format!(
            "snapshot_accuracy_over_best_control: {:.6}\n",
            self.snapshot_accuracy_over_best_control
        ));
        output.push_str(&format!(
            "snapshot_error_gain_over_best_control: {:.6}\n",
            self.snapshot_error_gain_over_best_control
        ));
        output.push_str(&format!(
            "snapshot_error_gain_over_wrong_snapshot: {:.6}\n",
            self.snapshot_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl ByteContextReport {
    /// Render a stable report for the first byte-level context probe.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave byte-context eval\n");
        output.push_str(&format!("train_seed: {}\n", self.train_seed));
        output.push_str(&format!("holdout_seed: {}\n", self.holdout_seed));
        output.push_str(&format!("train_cases: {}\n", self.train_cases));
        output.push_str(&format!("holdout_cases: {}\n", self.holdout_cases));
        output.push_str(&format!("snapshot_bytes: {}\n", self.snapshot_bytes));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192_prompt_decoder));
        output.push_str(&format_baseline(self.no_snapshot_decoder));
        output.push_str(&format_baseline(self.cell32_voting));
        output.push_str(&format_baseline(self.snapshot_decoder));
        output.push_str(&format_baseline(self.wrong_snapshot_decoder));
        output.push_str(&format_baseline(self.corrupted_snapshot_decoder));
        output.push_str(&format!(
            "snapshot_accuracy_over_best_control: {:.6}\n",
            self.snapshot_accuracy_over_best_control
        ));
        output.push_str(&format!(
            "snapshot_error_gain_over_best_control: {:.6}\n",
            self.snapshot_error_gain_over_best_control
        ));
        output.push_str(&format!(
            "snapshot_error_gain_over_wrong_snapshot: {:.6}\n",
            self.snapshot_error_gain_over_wrong_snapshot
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}
