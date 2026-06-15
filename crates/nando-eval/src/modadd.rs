use std::time::Instant;

use crate::{BaselineResult, best_baseline, format_baseline, score_prediction, splitmix64};
use nando_core::{
    CarrierWave, OrganState, PHASE_SLOTS, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, Stage2Organ, WaveBus,
    run_stage2_bus_trace_with_organ_carrier, run_stage2_trace_with_organ_carrier,
};

const TAU: f32 = std::f32::consts::TAU;
const MIN_ENSEMBLE_GAIN: f32 = 0.03;
const MIN_KEY_ABLATION_DROP: f32 = 0.05;
const MAX_FALSE_POSITIVE_INCREASE: f32 = 0.02;
const MODADD_SWEEP_SEEDS: [u64; 5] = [7, 13, 29, 97, 131];
const COMPONENT_BUS_FEATURES: usize = PHASE_SLOTS * 4;
const COMPONENT_LINK_FEATURES: usize = PHASE_SLOTS * 2;
const SETTLE_LINK_FEATURES: usize = PHASE_SLOTS * 2 + STAGE2_ORGAN_CELLS * 2 + 4;

/// GOAL v0 modular-addition eval configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Organ128ModAddConfig {
    pub seed: u64,
    pub modulus: u8,
    pub train_cases: usize,
    pub holdout_cases: usize,
}

impl Default for Organ128ModAddConfig {
    fn default() -> Self {
        Self {
            seed: 7,
            modulus: 31,
            train_cases: 256,
            holdout_cases: 256,
        }
    }
}

/// First GOAL-driven Nanda-style modular-addition report.
#[derive(Debug, Clone, PartialEq)]
pub struct Organ128ModAddReport {
    pub config: Organ128ModAddConfig,
    pub elapsed_ms: u128,
    pub random: BaselineResult,
    pub mono192: BaselineResult,
    pub fourier_phase: BaselineResult,
    pub cell32_phase_compose: BaselineResult,
    pub cell32_structured_compose: BaselineResult,
    pub cell32_fourier_census: BaselineResult,
    pub cell32_component_bus_projection: BaselineResult,
    pub cell32_component_link_projection: BaselineResult,
    pub cell32_settle_link_projection: BaselineResult,
    pub cell32_voting: BaselineResult,
    pub cell32_wavebus: BaselineResult,
    pub restricted_key: BaselineResult,
    pub excluded_key: BaselineResult,
    pub label_shuffle: BaselineResult,
    pub component_bus_label_shuffle: BaselineResult,
    pub component_bus_a_only: BaselineResult,
    pub component_bus_b_only: BaselineResult,
    pub component_bus_no_phase: BaselineResult,
    pub component_bus_no_amplitude: BaselineResult,
    pub component_bus_wrong_pair: BaselineResult,
    pub component_link_label_shuffle: BaselineResult,
    pub component_link_no_phase: BaselineResult,
    pub component_link_no_amplitude: BaselineResult,
    pub component_link_wrong_pair: BaselineResult,
    pub settle_link_label_shuffle: BaselineResult,
    pub settle_link_no_coupling: BaselineResult,
    pub settle_link_no_phase: BaselineResult,
    pub settle_link_wrong_pair: BaselineResult,
    pub key_cell: u32,
    pub ensemble_gain: f32,
    pub phase_compose_gain: f32,
    pub structured_compose_gain: f32,
    pub fourier_census_gain: f32,
    pub component_bus_projection_gain: f32,
    pub component_link_projection_gain: f32,
    pub settle_link_projection_gain: f32,
    pub wave_over_fourier_gap: f32,
    pub compose_over_fourier_gap: f32,
    pub structured_over_fourier_gap: f32,
    pub census_over_fourier_gap: f32,
    pub component_bus_over_fourier_gap: f32,
    pub component_link_over_fourier_gap: f32,
    pub settle_link_over_fourier_gap: f32,
    pub component_bus_a_drop: f32,
    pub component_bus_b_drop: f32,
    pub component_bus_phase_drop: f32,
    pub component_bus_amplitude_drop: f32,
    pub component_bus_wrong_pair_drop: f32,
    pub component_link_phase_drop: f32,
    pub component_link_amplitude_drop: f32,
    pub component_link_wrong_pair_drop: f32,
    pub settle_link_coupling_drop: f32,
    pub settle_link_phase_drop: f32,
    pub settle_link_wrong_pair_drop: f32,
    pub key_ablation_drop: f32,
    pub non_key_ablation_drop: f32,
    pub no_shortcut_control: bool,
    pub component_bus_no_shortcut_control: bool,
    pub component_link_no_shortcut_control: bool,
    pub settle_link_no_shortcut_control: bool,
    pub scientific_pass: bool,
    pub engineering_pass: bool,
    pub mode_status: &'static str,
}

/// One row in the modular-addition seed sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct Organ128ModAddSeedSweepRow {
    pub seed: u64,
    pub cell32_wavebus_accuracy: f32,
    pub cell32_phase_compose_accuracy: f32,
    pub cell32_structured_compose_accuracy: f32,
    pub cell32_fourier_census_accuracy: f32,
    pub cell32_component_bus_projection_accuracy: f32,
    pub cell32_component_link_projection_accuracy: f32,
    pub cell32_settle_link_projection_accuracy: f32,
    pub fourier_phase_accuracy: f32,
    pub ensemble_gain: f32,
    pub phase_compose_gain: f32,
    pub structured_compose_gain: f32,
    pub fourier_census_gain: f32,
    pub component_bus_projection_gain: f32,
    pub component_link_projection_gain: f32,
    pub settle_link_projection_gain: f32,
    pub wave_over_fourier_gap: f32,
    pub compose_over_fourier_gap: f32,
    pub structured_over_fourier_gap: f32,
    pub census_over_fourier_gap: f32,
    pub component_bus_over_fourier_gap: f32,
    pub component_link_over_fourier_gap: f32,
    pub settle_link_over_fourier_gap: f32,
    pub key_ablation_drop: f32,
    pub non_key_ablation_drop: f32,
    pub label_shuffle_accuracy: f32,
    pub component_bus_shuffle_accuracy: f32,
    pub component_link_shuffle_accuracy: f32,
    pub settle_link_shuffle_accuracy: f32,
    pub component_bus_a_drop: f32,
    pub component_bus_b_drop: f32,
    pub component_bus_phase_drop: f32,
    pub component_bus_amplitude_drop: f32,
    pub component_bus_wrong_pair_drop: f32,
    pub component_link_phase_drop: f32,
    pub component_link_amplitude_drop: f32,
    pub component_link_wrong_pair_drop: f32,
    pub settle_link_coupling_drop: f32,
    pub settle_link_phase_drop: f32,
    pub settle_link_wrong_pair_drop: f32,
    pub no_shortcut_control: bool,
    pub component_bus_no_shortcut_control: bool,
    pub component_link_no_shortcut_control: bool,
    pub settle_link_no_shortcut_control: bool,
    pub scientific_pass: bool,
    pub engineering_pass: bool,
    pub mode_status: &'static str,
}

/// Seed-robustness report for the GOAL modular-addition probe.
#[derive(Debug, Clone, PartialEq)]
pub struct Organ128ModAddSeedSweepReport {
    pub modulus: u8,
    pub train_cases: usize,
    pub holdout_cases: usize,
    pub rows: [Organ128ModAddSeedSweepRow; 5],
    pub passed_seed_pairs: usize,
    pub candidate_seed_pairs: usize,
    pub min_ensemble_gain: f32,
    pub min_phase_compose_gain: f32,
    pub min_structured_compose_gain: f32,
    pub min_fourier_census_gain: f32,
    pub min_component_bus_projection_gain: f32,
    pub min_component_link_projection_gain: f32,
    pub min_settle_link_projection_gain: f32,
    pub min_wave_over_fourier_gap: f32,
    pub min_compose_over_fourier_gap: f32,
    pub min_structured_over_fourier_gap: f32,
    pub min_census_over_fourier_gap: f32,
    pub min_component_bus_over_fourier_gap: f32,
    pub min_component_link_over_fourier_gap: f32,
    pub min_settle_link_over_fourier_gap: f32,
    pub min_key_ablation_drop: f32,
    pub max_label_shuffle_accuracy: f32,
    pub max_component_bus_shuffle_accuracy: f32,
    pub max_component_link_shuffle_accuracy: f32,
    pub max_settle_link_shuffle_accuracy: f32,
    pub min_component_bus_a_drop: f32,
    pub min_component_bus_b_drop: f32,
    pub min_component_bus_phase_drop: f32,
    pub min_component_bus_amplitude_drop: f32,
    pub min_component_bus_wrong_pair_drop: f32,
    pub min_component_link_phase_drop: f32,
    pub min_component_link_amplitude_drop: f32,
    pub min_component_link_wrong_pair_drop: f32,
    pub min_settle_link_coupling_drop: f32,
    pub min_settle_link_phase_drop: f32,
    pub min_settle_link_wrong_pair_drop: f32,
    pub mode_status: &'static str,
}

impl Organ128ModAddReport {
    /// Render a stable line-oriented report matching `docs/GOAL.md`.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave Organ128 modadd eval\n");
        output.push_str("task: modular_addition\n");
        output.push_str(&format!("seed: {}\n", self.config.seed));
        output.push_str(&format!("modulus: {}\n", self.config.modulus));
        output.push_str(&format!("train_size: {}\n", self.config.train_cases));
        output.push_str(&format!("holdout_size: {}\n", self.config.holdout_cases));
        output.push_str(&format!("elapsed_ms: {}\n", self.elapsed_ms));
        output.push_str(&format_baseline(self.random));
        output.push_str(&format_baseline(self.mono192));
        output.push_str(&format_baseline(self.fourier_phase));
        output.push_str(&format_baseline(self.cell32_phase_compose));
        output.push_str(&format_baseline(self.cell32_structured_compose));
        output.push_str(&format_baseline(self.cell32_fourier_census));
        output.push_str(&format_baseline(self.cell32_component_bus_projection));
        output.push_str(&format_baseline(self.cell32_component_link_projection));
        output.push_str(&format_baseline(self.cell32_settle_link_projection));
        output.push_str(&format_baseline(self.cell32_voting));
        output.push_str(&format_baseline(self.cell32_wavebus));
        output.push_str(&format_baseline(self.restricted_key));
        output.push_str(&format_baseline(self.excluded_key));
        output.push_str(&format_baseline(self.label_shuffle));
        output.push_str(&format_baseline(self.component_bus_label_shuffle));
        output.push_str(&format_baseline(self.component_bus_a_only));
        output.push_str(&format_baseline(self.component_bus_b_only));
        output.push_str(&format_baseline(self.component_bus_no_phase));
        output.push_str(&format_baseline(self.component_bus_no_amplitude));
        output.push_str(&format_baseline(self.component_bus_wrong_pair));
        output.push_str(&format_baseline(self.component_link_label_shuffle));
        output.push_str(&format_baseline(self.component_link_no_phase));
        output.push_str(&format_baseline(self.component_link_no_amplitude));
        output.push_str(&format_baseline(self.component_link_wrong_pair));
        output.push_str(&format_baseline(self.settle_link_label_shuffle));
        output.push_str(&format_baseline(self.settle_link_no_coupling));
        output.push_str(&format_baseline(self.settle_link_no_phase));
        output.push_str(&format_baseline(self.settle_link_wrong_pair));
        output.push_str(&format!("random_accuracy: {:.6}\n", self.random.accuracy));
        output.push_str(&format!("mono192_accuracy: {:.6}\n", self.mono192.accuracy));
        output.push_str(&format!(
            "fourier_phase_accuracy: {:.6}\n",
            self.fourier_phase.accuracy
        ));
        output.push_str(&format!(
            "cell32_phase_compose_accuracy: {:.6}\n",
            self.cell32_phase_compose.accuracy
        ));
        output.push_str(&format!(
            "cell32_structured_compose_accuracy: {:.6}\n",
            self.cell32_structured_compose.accuracy
        ));
        output.push_str(&format!(
            "cell32_fourier_census_accuracy: {:.6}\n",
            self.cell32_fourier_census.accuracy
        ));
        output.push_str(&format!(
            "cell32_component_bus_projection_accuracy: {:.6}\n",
            self.cell32_component_bus_projection.accuracy
        ));
        output.push_str(&format!(
            "cell32_component_link_projection_accuracy: {:.6}\n",
            self.cell32_component_link_projection.accuracy
        ));
        output.push_str(&format!(
            "cell32_settle_link_projection_accuracy: {:.6}\n",
            self.cell32_settle_link_projection.accuracy
        ));
        output.push_str(&format!(
            "cell32_voting_accuracy: {:.6}\n",
            self.cell32_voting.accuracy
        ));
        output.push_str(&format!(
            "cell32_wavebus_accuracy: {:.6}\n",
            self.cell32_wavebus.accuracy
        ));
        output.push_str(&format!("key_cell: {}\n", self.key_cell));
        output.push_str(&format!("ensemble_gain: {:.6}\n", self.ensemble_gain));
        output.push_str(&format!(
            "phase_compose_gain: {:.6}\n",
            self.phase_compose_gain
        ));
        output.push_str(&format!(
            "structured_compose_gain: {:.6}\n",
            self.structured_compose_gain
        ));
        output.push_str(&format!(
            "fourier_census_gain: {:.6}\n",
            self.fourier_census_gain
        ));
        output.push_str(&format!(
            "component_bus_projection_gain: {:.6}\n",
            self.component_bus_projection_gain
        ));
        output.push_str(&format!(
            "component_link_projection_gain: {:.6}\n",
            self.component_link_projection_gain
        ));
        output.push_str(&format!(
            "settle_link_projection_gain: {:.6}\n",
            self.settle_link_projection_gain
        ));
        output.push_str(&format!(
            "wave_over_fourier_gap: {:.6}\n",
            self.wave_over_fourier_gap
        ));
        output.push_str(&format!(
            "compose_over_fourier_gap: {:.6}\n",
            self.compose_over_fourier_gap
        ));
        output.push_str(&format!(
            "structured_over_fourier_gap: {:.6}\n",
            self.structured_over_fourier_gap
        ));
        output.push_str(&format!(
            "census_over_fourier_gap: {:.6}\n",
            self.census_over_fourier_gap
        ));
        output.push_str(&format!(
            "component_bus_over_fourier_gap: {:.6}\n",
            self.component_bus_over_fourier_gap
        ));
        output.push_str(&format!(
            "component_link_over_fourier_gap: {:.6}\n",
            self.component_link_over_fourier_gap
        ));
        output.push_str(&format!(
            "settle_link_over_fourier_gap: {:.6}\n",
            self.settle_link_over_fourier_gap
        ));
        output.push_str(&format!(
            "component_bus_a_drop: {:.6}\n",
            self.component_bus_a_drop
        ));
        output.push_str(&format!(
            "component_bus_b_drop: {:.6}\n",
            self.component_bus_b_drop
        ));
        output.push_str(&format!(
            "component_bus_phase_drop: {:.6}\n",
            self.component_bus_phase_drop
        ));
        output.push_str(&format!(
            "component_bus_amplitude_drop: {:.6}\n",
            self.component_bus_amplitude_drop
        ));
        output.push_str(&format!(
            "component_bus_wrong_pair_drop: {:.6}\n",
            self.component_bus_wrong_pair_drop
        ));
        output.push_str(&format!(
            "component_link_phase_drop: {:.6}\n",
            self.component_link_phase_drop
        ));
        output.push_str(&format!(
            "component_link_amplitude_drop: {:.6}\n",
            self.component_link_amplitude_drop
        ));
        output.push_str(&format!(
            "component_link_wrong_pair_drop: {:.6}\n",
            self.component_link_wrong_pair_drop
        ));
        output.push_str(&format!(
            "settle_link_coupling_drop: {:.6}\n",
            self.settle_link_coupling_drop
        ));
        output.push_str(&format!(
            "settle_link_phase_drop: {:.6}\n",
            self.settle_link_phase_drop
        ));
        output.push_str(&format!(
            "settle_link_wrong_pair_drop: {:.6}\n",
            self.settle_link_wrong_pair_drop
        ));
        output.push_str(&format!(
            "restricted_key_accuracy: {:.6}\n",
            self.restricted_key.accuracy
        ));
        output.push_str(&format!(
            "excluded_key_accuracy: {:.6}\n",
            self.excluded_key.accuracy
        ));
        output.push_str(&format!(
            "key_ablation_drop: {:.6}\n",
            self.key_ablation_drop
        ));
        output.push_str(&format!(
            "non_key_ablation_drop: {:.6}\n",
            self.non_key_ablation_drop
        ));
        output.push_str(&format!(
            "label_shuffle_accuracy: {:.6}\n",
            self.label_shuffle.accuracy
        ));
        output.push_str(&format!(
            "component_bus_shuffle_accuracy: {:.6}\n",
            self.component_bus_label_shuffle.accuracy
        ));
        output.push_str(&format!(
            "component_link_shuffle_accuracy: {:.6}\n",
            self.component_link_label_shuffle.accuracy
        ));
        output.push_str(&format!(
            "settle_link_shuffle_accuracy: {:.6}\n",
            self.settle_link_label_shuffle.accuracy
        ));
        output.push_str(&format!(
            "no_shortcut_control: {}\n",
            self.no_shortcut_control
        ));
        output.push_str(&format!(
            "component_bus_no_shortcut_control: {}\n",
            self.component_bus_no_shortcut_control
        ));
        output.push_str(&format!(
            "component_link_no_shortcut_control: {}\n",
            self.component_link_no_shortcut_control
        ));
        output.push_str(&format!(
            "settle_link_no_shortcut_control: {}\n",
            self.settle_link_no_shortcut_control
        ));
        output.push_str(&format!("scientific_pass: {}\n", self.scientific_pass));
        output.push_str(&format!("engineering_pass: {}\n", self.engineering_pass));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

impl Organ128ModAddSeedSweepReport {
    /// Render a stable line-oriented seed-robustness report.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut output = String::new();
        output.push_str("Nando Wave Organ128 modadd seed-sweep eval\n");
        output.push_str("task: modular_addition\n");
        output.push_str(&format!("modulus: {}\n", self.modulus));
        output.push_str(&format!("train_size: {}\n", self.train_cases));
        output.push_str(&format!("holdout_size: {}\n", self.holdout_cases));
        output.push_str("seed wavebus_acc component_link_acc component_link_gain settle_link_acc settle_link_gain link_phase_drop settle_coupling_drop settle_phase_drop link_wrong_pair_drop settle_wrong_pair_drop link_shuffle_acc settle_shuffle_acc link_no_shortcut settle_no_shortcut scientific engineering mode_status\n");
        for row in &self.rows {
            output.push_str(&format!(
                "{} {:.6} {:.6} {:+.6} {:.6} {:+.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {:.6} {} {} {} {} {}\n",
                row.seed,
                row.cell32_wavebus_accuracy,
                row.cell32_component_link_projection_accuracy,
                row.component_link_projection_gain,
                row.cell32_settle_link_projection_accuracy,
                row.settle_link_projection_gain,
                row.component_link_phase_drop,
                row.settle_link_coupling_drop,
                row.settle_link_phase_drop,
                row.component_link_wrong_pair_drop,
                row.settle_link_wrong_pair_drop,
                row.component_link_shuffle_accuracy,
                row.settle_link_shuffle_accuracy,
                row.component_link_no_shortcut_control,
                row.settle_link_no_shortcut_control,
                row.scientific_pass,
                row.engineering_pass,
                row.mode_status
            ));
        }
        output.push_str(&format!("passed_seed_pairs: {}\n", self.passed_seed_pairs));
        output.push_str(&format!(
            "candidate_seed_pairs: {}\n",
            self.candidate_seed_pairs
        ));
        output.push_str(&format!(
            "min_ensemble_gain: {:.6}\n",
            self.min_ensemble_gain
        ));
        output.push_str(&format!(
            "min_phase_compose_gain: {:.6}\n",
            self.min_phase_compose_gain
        ));
        output.push_str(&format!(
            "min_structured_compose_gain: {:.6}\n",
            self.min_structured_compose_gain
        ));
        output.push_str(&format!(
            "min_fourier_census_gain: {:.6}\n",
            self.min_fourier_census_gain
        ));
        output.push_str(&format!(
            "min_component_bus_projection_gain: {:.6}\n",
            self.min_component_bus_projection_gain
        ));
        output.push_str(&format!(
            "min_component_link_projection_gain: {:.6}\n",
            self.min_component_link_projection_gain
        ));
        output.push_str(&format!(
            "min_settle_link_projection_gain: {:.6}\n",
            self.min_settle_link_projection_gain
        ));
        output.push_str(&format!(
            "min_wave_over_fourier_gap: {:.6}\n",
            self.min_wave_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_compose_over_fourier_gap: {:.6}\n",
            self.min_compose_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_structured_over_fourier_gap: {:.6}\n",
            self.min_structured_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_census_over_fourier_gap: {:.6}\n",
            self.min_census_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_component_bus_over_fourier_gap: {:.6}\n",
            self.min_component_bus_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_component_link_over_fourier_gap: {:.6}\n",
            self.min_component_link_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_settle_link_over_fourier_gap: {:.6}\n",
            self.min_settle_link_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_key_ablation_drop: {:.6}\n",
            self.min_key_ablation_drop
        ));
        output.push_str(&format!(
            "max_label_shuffle_accuracy: {:.6}\n",
            self.max_label_shuffle_accuracy
        ));
        output.push_str(&format!(
            "max_component_bus_shuffle_accuracy: {:.6}\n",
            self.max_component_bus_shuffle_accuracy
        ));
        output.push_str(&format!(
            "max_component_link_shuffle_accuracy: {:.6}\n",
            self.max_component_link_shuffle_accuracy
        ));
        output.push_str(&format!(
            "max_settle_link_shuffle_accuracy: {:.6}\n",
            self.max_settle_link_shuffle_accuracy
        ));
        output.push_str(&format!(
            "min_component_bus_a_drop: {:.6}\n",
            self.min_component_bus_a_drop
        ));
        output.push_str(&format!(
            "min_component_bus_b_drop: {:.6}\n",
            self.min_component_bus_b_drop
        ));
        output.push_str(&format!(
            "min_component_bus_phase_drop: {:.6}\n",
            self.min_component_bus_phase_drop
        ));
        output.push_str(&format!(
            "min_component_bus_amplitude_drop: {:.6}\n",
            self.min_component_bus_amplitude_drop
        ));
        output.push_str(&format!(
            "min_component_bus_wrong_pair_drop: {:.6}\n",
            self.min_component_bus_wrong_pair_drop
        ));
        output.push_str(&format!(
            "min_component_link_phase_drop: {:.6}\n",
            self.min_component_link_phase_drop
        ));
        output.push_str(&format!(
            "min_component_link_amplitude_drop: {:.6}\n",
            self.min_component_link_amplitude_drop
        ));
        output.push_str(&format!(
            "min_component_link_wrong_pair_drop: {:.6}\n",
            self.min_component_link_wrong_pair_drop
        ));
        output.push_str(&format!(
            "min_settle_link_coupling_drop: {:.6}\n",
            self.min_settle_link_coupling_drop
        ));
        output.push_str(&format!(
            "min_settle_link_phase_drop: {:.6}\n",
            self.min_settle_link_phase_drop
        ));
        output.push_str(&format!(
            "min_settle_link_wrong_pair_drop: {:.6}\n",
            self.min_settle_link_wrong_pair_drop
        ));
        output.push_str(&format!("mode_status: {}\n", self.mode_status));
        output
    }
}

/// Run the first GOAL-driven modular-addition probe.
#[must_use]
pub fn organ128_modadd_eval(config: Organ128ModAddConfig) -> Organ128ModAddReport {
    let started = Instant::now();
    let dataset = ModAddDataset::new(config);
    let organ = Stage2Organ::new(config.seed);

    let readout = ModAddReadout::train(&organ, config, &dataset.train, None);
    let phase_compose_readout =
        PhaseComposeReadout::train(&organ, config, &dataset.train, ComponentEncoding::Random);
    let structured_compose_readout = PhaseComposeReadout::train(
        &organ,
        config,
        &dataset.train,
        ComponentEncoding::Structured,
    );
    let fourier_census_readout = FourierCensusReadout::train(&organ, config, &dataset.train);
    let component_bus_projection_readout =
        ComponentBusProjectionReadout::train(&organ, config, &dataset.train, None);
    let shuffled_component_bus_projection_readout =
        ComponentBusProjectionReadout::train(&organ, config, &dataset.train, Some(config.seed));
    let component_link_projection_readout =
        ComponentLinkProjectionReadout::train(&organ, config, &dataset.train, None);
    let shuffled_component_link_projection_readout =
        ComponentLinkProjectionReadout::train(&organ, config, &dataset.train, Some(config.seed));
    let settle_link_projection_readout =
        SettleLinkProjectionReadout::train(&organ, config, &dataset.train, None);
    let shuffled_settle_link_projection_readout =
        SettleLinkProjectionReadout::train(&organ, config, &dataset.train, Some(config.seed));
    let shuffled_readout = ModAddReadout::train(&organ, config, &dataset.train, Some(config.seed));

    let mut random = BaselineResult::new("random", dataset.holdout.len());
    let mut mono192 = BaselineResult::new("mono192", dataset.holdout.len());
    let mut fourier_phase = BaselineResult::new("fourier_phase_control", dataset.holdout.len());
    let mut cell32_phase_compose =
        BaselineResult::new("cell32_phase_compose", dataset.holdout.len());
    let mut cell32_structured_compose =
        BaselineResult::new("cell32_structured_compose", dataset.holdout.len());
    let mut cell32_fourier_census =
        BaselineResult::new("cell32_fourier_census", dataset.holdout.len());
    let mut cell32_component_bus_projection =
        BaselineResult::new("cell32_component_bus_projection", dataset.holdout.len());
    let mut cell32_component_link_projection =
        BaselineResult::new("cell32_component_link_projection", dataset.holdout.len());
    let mut cell32_settle_link_projection =
        BaselineResult::new("cell32_settle_link_projection", dataset.holdout.len());
    let mut cell32_voting = BaselineResult::new("cell32_voting", dataset.holdout.len());
    let mut cell32_wavebus = BaselineResult::new("cell32_wavebus", dataset.holdout.len());
    let mut label_shuffle = BaselineResult::new("label_shuffle", dataset.holdout.len());
    let mut component_bus_label_shuffle =
        BaselineResult::new("component_bus_label_shuffle", dataset.holdout.len());
    let mut component_bus_a_only =
        BaselineResult::new("component_bus_a_only", dataset.holdout.len());
    let mut component_bus_b_only =
        BaselineResult::new("component_bus_b_only", dataset.holdout.len());
    let mut component_bus_no_phase =
        BaselineResult::new("component_bus_no_phase", dataset.holdout.len());
    let mut component_bus_no_amplitude =
        BaselineResult::new("component_bus_no_amplitude", dataset.holdout.len());
    let mut component_bus_wrong_pair =
        BaselineResult::new("component_bus_wrong_pair", dataset.holdout.len());
    let mut component_link_label_shuffle =
        BaselineResult::new("component_link_label_shuffle", dataset.holdout.len());
    let mut component_link_no_phase =
        BaselineResult::new("component_link_no_phase", dataset.holdout.len());
    let mut component_link_no_amplitude =
        BaselineResult::new("component_link_no_amplitude", dataset.holdout.len());
    let mut component_link_wrong_pair =
        BaselineResult::new("component_link_wrong_pair", dataset.holdout.len());
    let mut settle_link_label_shuffle =
        BaselineResult::new("settle_link_label_shuffle", dataset.holdout.len());
    let mut settle_link_no_coupling =
        BaselineResult::new("settle_link_no_coupling", dataset.holdout.len());
    let mut settle_link_no_phase =
        BaselineResult::new("settle_link_no_phase", dataset.holdout.len());
    let mut settle_link_wrong_pair =
        BaselineResult::new("settle_link_wrong_pair", dataset.holdout.len());
    let mut ablations: [BaselineResult; STAGE2_ORGAN_CELLS] = std::array::from_fn(|cell_id| {
        BaselineResult::new(ablation_name(cell_id), dataset.holdout.len())
    });

    for (case_index, sample) in dataset.holdout.iter().enumerate() {
        let trace = modadd_trace(&organ, config.seed, *sample);
        let target = sample.target(config.modulus);

        let random_prediction =
            random_mod_predict(config.seed, case_index, *sample, config.modulus);
        score_prediction(&mut random, random_prediction, target, 0.0, 1.0);

        let mono_prediction = mono_mod_predict(config.seed, case_index, *sample, config.modulus);
        score_prediction(&mut mono192, mono_prediction, target, 0.0, 1.0);

        let fourier_prediction = fourier_phase_predict(*sample, config.modulus);
        score_prediction(&mut fourier_phase, fourier_prediction, target, 1.0, 0.0);

        let phase_compose_prediction =
            phase_compose_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_phase_compose,
            phase_compose_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let structured_compose_prediction =
            structured_compose_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_structured_compose,
            structured_compose_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let fourier_census_prediction =
            fourier_census_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_fourier_census,
            fourier_census_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let component_bus_prediction =
            component_bus_projection_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_component_bus_projection,
            component_bus_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let component_link_prediction =
            component_link_projection_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_component_link_projection,
            component_link_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let settle_link_prediction =
            settle_link_projection_readout.predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut cell32_settle_link_projection,
            settle_link_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let component_bus_shuffled_prediction = shuffled_component_bus_projection_readout.predict(
            &organ,
            config.seed,
            *sample,
            config.modulus,
        );
        score_prediction(
            &mut component_bus_label_shuffle,
            component_bus_shuffled_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let component_link_shuffled_prediction = shuffled_component_link_projection_readout
            .predict(&organ, config.seed, *sample, config.modulus);
        score_prediction(
            &mut component_link_label_shuffle,
            component_link_shuffled_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let settle_link_shuffled_prediction = shuffled_settle_link_projection_readout.predict(
            &organ,
            config.seed,
            *sample,
            config.modulus,
        );
        score_prediction(
            &mut settle_link_label_shuffle,
            settle_link_shuffled_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        for (mode, result) in [
            (ComponentBusFeatureMode::AOnly, &mut component_bus_a_only),
            (ComponentBusFeatureMode::BOnly, &mut component_bus_b_only),
            (
                ComponentBusFeatureMode::NoPhase,
                &mut component_bus_no_phase,
            ),
            (
                ComponentBusFeatureMode::NoAmplitude,
                &mut component_bus_no_amplitude,
            ),
            (
                ComponentBusFeatureMode::WrongPair,
                &mut component_bus_wrong_pair,
            ),
        ] {
            let prediction = component_bus_projection_readout.predict_with_mode(
                &organ,
                config.seed,
                *sample,
                config.modulus,
                mode,
            );
            score_prediction(
                result,
                prediction,
                target,
                trace.coherence,
                trace.spectral_entropy,
            );
        }

        for (mode, result) in [
            (
                SettleLinkFeatureMode::NoCoupling,
                &mut settle_link_no_coupling,
            ),
            (SettleLinkFeatureMode::NoPhase, &mut settle_link_no_phase),
            (
                SettleLinkFeatureMode::WrongPair,
                &mut settle_link_wrong_pair,
            ),
        ] {
            let prediction = settle_link_projection_readout.predict_with_mode(
                &organ,
                config.seed,
                *sample,
                config.modulus,
                mode,
            );
            score_prediction(
                result,
                prediction,
                target,
                trace.coherence,
                trace.spectral_entropy,
            );
        }

        for (mode, result) in [
            (
                ComponentLinkFeatureMode::NoPhase,
                &mut component_link_no_phase,
            ),
            (
                ComponentLinkFeatureMode::NoAmplitude,
                &mut component_link_no_amplitude,
            ),
            (
                ComponentLinkFeatureMode::WrongPair,
                &mut component_link_wrong_pair,
            ),
        ] {
            let prediction = component_link_projection_readout.predict_with_mode(
                &organ,
                config.seed,
                *sample,
                config.modulus,
                mode,
            );
            score_prediction(
                result,
                prediction,
                target,
                trace.coherence,
                trace.spectral_entropy,
            );
        }

        let voting_prediction = readout.predict_voting(&trace, config.modulus);
        score_prediction(
            &mut cell32_voting,
            voting_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let wave_prediction = readout.predict_wavebus(&trace, config.modulus);
        score_prediction(
            &mut cell32_wavebus,
            wave_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        let shuffled_prediction = shuffled_readout.predict_wavebus(&trace, config.modulus);
        score_prediction(
            &mut label_shuffle,
            shuffled_prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );

        for (cell_id, ablation) in ablations.iter_mut().enumerate() {
            let prediction = readout.predict_wavebus_without_cell(&trace, config.modulus, cell_id);
            score_prediction(
                ablation,
                prediction,
                target,
                trace.coherence,
                trace.spectral_entropy,
            );
        }
    }

    finish_all([
        &mut random,
        &mut mono192,
        &mut fourier_phase,
        &mut cell32_phase_compose,
        &mut cell32_structured_compose,
        &mut cell32_fourier_census,
        &mut cell32_component_bus_projection,
        &mut cell32_component_link_projection,
        &mut cell32_settle_link_projection,
        &mut cell32_voting,
        &mut cell32_wavebus,
        &mut label_shuffle,
        &mut component_bus_label_shuffle,
        &mut component_bus_a_only,
        &mut component_bus_b_only,
        &mut component_bus_no_phase,
        &mut component_bus_no_amplitude,
        &mut component_bus_wrong_pair,
        &mut component_link_label_shuffle,
        &mut component_link_no_phase,
        &mut component_link_no_amplitude,
        &mut component_link_wrong_pair,
        &mut settle_link_label_shuffle,
        &mut settle_link_no_coupling,
        &mut settle_link_no_phase,
        &mut settle_link_wrong_pair,
    ]);
    for ablation in &mut ablations {
        ablation.finish();
    }

    let (key_cell, excluded_key, key_ablation_drop, non_key_ablation_drop) =
        key_ablation(&cell32_wavebus, ablations);

    let mut restricted_key = BaselineResult::new("restricted_key", dataset.holdout.len());
    for sample in &dataset.holdout {
        let trace = modadd_trace(&organ, config.seed, *sample);
        let target = sample.target(config.modulus);
        let prediction = readout.predict_restricted_cell(&trace, config.modulus, key_cell as usize);
        score_prediction(
            &mut restricted_key,
            prediction,
            target,
            trace.coherence,
            trace.spectral_entropy,
        );
    }
    restricted_key.finish();

    let best_control = best_baseline([random, mono192, cell32_voting]);
    let ensemble_gain = cell32_wavebus.accuracy - best_control.accuracy;
    let phase_compose_gain = cell32_phase_compose.accuracy - best_control.accuracy;
    let structured_compose_gain = cell32_structured_compose.accuracy - best_control.accuracy;
    let fourier_census_gain = cell32_fourier_census.accuracy - best_control.accuracy;
    let component_bus_projection_gain =
        cell32_component_bus_projection.accuracy - best_control.accuracy;
    let component_link_projection_gain =
        cell32_component_link_projection.accuracy - best_control.accuracy;
    let settle_link_projection_gain =
        cell32_settle_link_projection.accuracy - best_control.accuracy;
    let wave_over_fourier_gap = cell32_wavebus.accuracy - fourier_phase.accuracy;
    let compose_over_fourier_gap = cell32_phase_compose.accuracy - fourier_phase.accuracy;
    let structured_over_fourier_gap = cell32_structured_compose.accuracy - fourier_phase.accuracy;
    let census_over_fourier_gap = cell32_fourier_census.accuracy - fourier_phase.accuracy;
    let component_bus_over_fourier_gap =
        cell32_component_bus_projection.accuracy - fourier_phase.accuracy;
    let component_link_over_fourier_gap =
        cell32_component_link_projection.accuracy - fourier_phase.accuracy;
    let settle_link_over_fourier_gap =
        cell32_settle_link_projection.accuracy - fourier_phase.accuracy;
    let component_bus_a_drop = ablation_drop(
        cell32_component_bus_projection.accuracy,
        component_bus_a_only.accuracy,
    );
    let component_bus_b_drop = ablation_drop(
        cell32_component_bus_projection.accuracy,
        component_bus_b_only.accuracy,
    );
    let component_bus_phase_drop = ablation_drop(
        cell32_component_bus_projection.accuracy,
        component_bus_no_phase.accuracy,
    );
    let component_bus_amplitude_drop = ablation_drop(
        cell32_component_bus_projection.accuracy,
        component_bus_no_amplitude.accuracy,
    );
    let component_bus_wrong_pair_drop = ablation_drop(
        cell32_component_bus_projection.accuracy,
        component_bus_wrong_pair.accuracy,
    );
    let component_link_phase_drop = ablation_drop(
        cell32_component_link_projection.accuracy,
        component_link_no_phase.accuracy,
    );
    let component_link_amplitude_drop = ablation_drop(
        cell32_component_link_projection.accuracy,
        component_link_no_amplitude.accuracy,
    );
    let component_link_wrong_pair_drop = ablation_drop(
        cell32_component_link_projection.accuracy,
        component_link_wrong_pair.accuracy,
    );
    let settle_link_coupling_drop = ablation_drop(
        cell32_settle_link_projection.accuracy,
        settle_link_no_coupling.accuracy,
    );
    let settle_link_phase_drop = ablation_drop(
        cell32_settle_link_projection.accuracy,
        settle_link_no_phase.accuracy,
    );
    let settle_link_wrong_pair_drop = ablation_drop(
        cell32_settle_link_projection.accuracy,
        settle_link_wrong_pair.accuracy,
    );
    let no_shortcut_control = label_shuffle.accuracy
        <= (random.accuracy + MAX_FALSE_POSITIVE_INCREASE).min(cell32_wavebus.accuracy);
    let component_bus_no_shortcut_control = component_bus_label_shuffle.accuracy
        <= (random.accuracy + MAX_FALSE_POSITIVE_INCREASE)
            .min(cell32_component_bus_projection.accuracy);
    let component_link_no_shortcut_control = component_link_label_shuffle.accuracy
        <= (random.accuracy + MAX_FALSE_POSITIVE_INCREASE)
            .min(cell32_component_link_projection.accuracy);
    let settle_link_no_shortcut_control = settle_link_label_shuffle.accuracy
        <= (random.accuracy + MAX_FALSE_POSITIVE_INCREASE)
            .min(cell32_settle_link_projection.accuracy);
    let scientific_pass = ensemble_gain >= MIN_ENSEMBLE_GAIN
        && cell32_wavebus.accuracy > mono192.accuracy
        && key_ablation_drop >= MIN_KEY_ABLATION_DROP
        && key_ablation_drop >= non_key_ablation_drop * 2.0
        && no_shortcut_control;
    let engineering_pass = cell32_wavebus.accuracy + 0.01 >= mono192.accuracy
        && key_ablation_drop >= MIN_KEY_ABLATION_DROP
        && no_shortcut_control;
    let component_link_candidate = component_link_projection_gain >= MIN_ENSEMBLE_GAIN
        && component_link_phase_drop >= MIN_KEY_ABLATION_DROP
        && component_link_wrong_pair_drop >= MIN_KEY_ABLATION_DROP
        && component_link_no_shortcut_control;
    let settle_link_candidate = settle_link_projection_gain >= MIN_ENSEMBLE_GAIN
        && settle_link_phase_drop >= MIN_KEY_ABLATION_DROP
        && settle_link_wrong_pair_drop >= MIN_KEY_ABLATION_DROP
        && settle_link_no_shortcut_control;
    let mode_status = if scientific_pass {
        "organ128_modadd_key_mode_ablation_passed"
    } else if settle_link_candidate {
        "organ128_modadd_settle_link_candidate"
    } else if component_link_candidate {
        "organ128_modadd_component_link_candidate"
    } else if engineering_pass || (ensemble_gain > 0.0 && no_shortcut_control) {
        "organ128_modadd_candidate"
    } else {
        "not_found_organ128_modadd"
    };

    Organ128ModAddReport {
        config,
        elapsed_ms: started.elapsed().as_millis(),
        random,
        mono192,
        fourier_phase,
        cell32_phase_compose,
        cell32_structured_compose,
        cell32_fourier_census,
        cell32_component_bus_projection,
        cell32_component_link_projection,
        cell32_settle_link_projection,
        cell32_voting,
        cell32_wavebus,
        restricted_key,
        excluded_key,
        label_shuffle,
        component_bus_label_shuffle,
        component_bus_a_only,
        component_bus_b_only,
        component_bus_no_phase,
        component_bus_no_amplitude,
        component_bus_wrong_pair,
        component_link_label_shuffle,
        component_link_no_phase,
        component_link_no_amplitude,
        component_link_wrong_pair,
        settle_link_label_shuffle,
        settle_link_no_coupling,
        settle_link_no_phase,
        settle_link_wrong_pair,
        key_cell,
        ensemble_gain,
        phase_compose_gain,
        structured_compose_gain,
        fourier_census_gain,
        component_bus_projection_gain,
        component_link_projection_gain,
        settle_link_projection_gain,
        wave_over_fourier_gap,
        compose_over_fourier_gap,
        structured_over_fourier_gap,
        census_over_fourier_gap,
        component_bus_over_fourier_gap,
        component_link_over_fourier_gap,
        settle_link_over_fourier_gap,
        component_bus_a_drop,
        component_bus_b_drop,
        component_bus_phase_drop,
        component_bus_amplitude_drop,
        component_bus_wrong_pair_drop,
        component_link_phase_drop,
        component_link_amplitude_drop,
        component_link_wrong_pair_drop,
        settle_link_coupling_drop,
        settle_link_phase_drop,
        settle_link_wrong_pair_drop,
        key_ablation_drop,
        non_key_ablation_drop,
        no_shortcut_control,
        component_bus_no_shortcut_control,
        component_link_no_shortcut_control,
        settle_link_no_shortcut_control,
        scientific_pass,
        engineering_pass,
        mode_status,
    }
}

/// Sweep the GOAL modular-addition probe across fixed seed pairs.
#[must_use]
pub fn organ128_modadd_seed_sweep_eval(
    modulus: u8,
    train_cases: usize,
    holdout_cases: usize,
) -> Organ128ModAddSeedSweepReport {
    let rows = MODADD_SWEEP_SEEDS.map(|seed| {
        let report = organ128_modadd_eval(Organ128ModAddConfig {
            seed,
            modulus,
            train_cases,
            holdout_cases,
        });
        Organ128ModAddSeedSweepRow {
            seed,
            cell32_wavebus_accuracy: report.cell32_wavebus.accuracy,
            cell32_phase_compose_accuracy: report.cell32_phase_compose.accuracy,
            cell32_structured_compose_accuracy: report.cell32_structured_compose.accuracy,
            cell32_fourier_census_accuracy: report.cell32_fourier_census.accuracy,
            cell32_component_bus_projection_accuracy: report
                .cell32_component_bus_projection
                .accuracy,
            cell32_component_link_projection_accuracy: report
                .cell32_component_link_projection
                .accuracy,
            cell32_settle_link_projection_accuracy: report.cell32_settle_link_projection.accuracy,
            fourier_phase_accuracy: report.fourier_phase.accuracy,
            ensemble_gain: report.ensemble_gain,
            phase_compose_gain: report.phase_compose_gain,
            structured_compose_gain: report.structured_compose_gain,
            fourier_census_gain: report.fourier_census_gain,
            component_bus_projection_gain: report.component_bus_projection_gain,
            component_link_projection_gain: report.component_link_projection_gain,
            settle_link_projection_gain: report.settle_link_projection_gain,
            wave_over_fourier_gap: report.wave_over_fourier_gap,
            compose_over_fourier_gap: report.compose_over_fourier_gap,
            structured_over_fourier_gap: report.structured_over_fourier_gap,
            census_over_fourier_gap: report.census_over_fourier_gap,
            component_bus_over_fourier_gap: report.component_bus_over_fourier_gap,
            component_link_over_fourier_gap: report.component_link_over_fourier_gap,
            settle_link_over_fourier_gap: report.settle_link_over_fourier_gap,
            key_ablation_drop: report.key_ablation_drop,
            non_key_ablation_drop: report.non_key_ablation_drop,
            label_shuffle_accuracy: report.label_shuffle.accuracy,
            component_bus_shuffle_accuracy: report.component_bus_label_shuffle.accuracy,
            component_link_shuffle_accuracy: report.component_link_label_shuffle.accuracy,
            settle_link_shuffle_accuracy: report.settle_link_label_shuffle.accuracy,
            component_bus_a_drop: report.component_bus_a_drop,
            component_bus_b_drop: report.component_bus_b_drop,
            component_bus_phase_drop: report.component_bus_phase_drop,
            component_bus_amplitude_drop: report.component_bus_amplitude_drop,
            component_bus_wrong_pair_drop: report.component_bus_wrong_pair_drop,
            component_link_phase_drop: report.component_link_phase_drop,
            component_link_amplitude_drop: report.component_link_amplitude_drop,
            component_link_wrong_pair_drop: report.component_link_wrong_pair_drop,
            settle_link_coupling_drop: report.settle_link_coupling_drop,
            settle_link_phase_drop: report.settle_link_phase_drop,
            settle_link_wrong_pair_drop: report.settle_link_wrong_pair_drop,
            no_shortcut_control: report.no_shortcut_control,
            component_bus_no_shortcut_control: report.component_bus_no_shortcut_control,
            component_link_no_shortcut_control: report.component_link_no_shortcut_control,
            settle_link_no_shortcut_control: report.settle_link_no_shortcut_control,
            scientific_pass: report.scientific_pass,
            engineering_pass: report.engineering_pass,
            mode_status: report.mode_status,
        }
    });

    let passed_seed_pairs = rows.iter().filter(|row| row.scientific_pass).count();
    let candidate_seed_pairs = rows
        .iter()
        .filter(|row| {
            row.scientific_pass
                || row.engineering_pass
                || row.mode_status == "organ128_modadd_candidate"
                || row.mode_status == "organ128_modadd_component_link_candidate"
                || row.mode_status == "organ128_modadd_settle_link_candidate"
        })
        .count();
    let min_ensemble_gain = rows
        .iter()
        .map(|row| row.ensemble_gain)
        .fold(f32::INFINITY, f32::min);
    let min_phase_compose_gain = rows
        .iter()
        .map(|row| row.phase_compose_gain)
        .fold(f32::INFINITY, f32::min);
    let min_structured_compose_gain = rows
        .iter()
        .map(|row| row.structured_compose_gain)
        .fold(f32::INFINITY, f32::min);
    let min_fourier_census_gain = rows
        .iter()
        .map(|row| row.fourier_census_gain)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_projection_gain = rows
        .iter()
        .map(|row| row.component_bus_projection_gain)
        .fold(f32::INFINITY, f32::min);
    let min_component_link_projection_gain = rows
        .iter()
        .map(|row| row.component_link_projection_gain)
        .fold(f32::INFINITY, f32::min);
    let min_settle_link_projection_gain = rows
        .iter()
        .map(|row| row.settle_link_projection_gain)
        .fold(f32::INFINITY, f32::min);
    let min_wave_over_fourier_gap = rows
        .iter()
        .map(|row| row.wave_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_compose_over_fourier_gap = rows
        .iter()
        .map(|row| row.compose_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_structured_over_fourier_gap = rows
        .iter()
        .map(|row| row.structured_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_census_over_fourier_gap = rows
        .iter()
        .map(|row| row.census_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_over_fourier_gap = rows
        .iter()
        .map(|row| row.component_bus_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_component_link_over_fourier_gap = rows
        .iter()
        .map(|row| row.component_link_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_settle_link_over_fourier_gap = rows
        .iter()
        .map(|row| row.settle_link_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_key_ablation_drop = rows
        .iter()
        .map(|row| row.key_ablation_drop)
        .fold(f32::INFINITY, f32::min);
    let max_label_shuffle_accuracy = rows
        .iter()
        .map(|row| row.label_shuffle_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_component_bus_shuffle_accuracy = rows
        .iter()
        .map(|row| row.component_bus_shuffle_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_component_link_shuffle_accuracy = rows
        .iter()
        .map(|row| row.component_link_shuffle_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_settle_link_shuffle_accuracy = rows
        .iter()
        .map(|row| row.settle_link_shuffle_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_component_bus_a_drop = rows
        .iter()
        .map(|row| row.component_bus_a_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_b_drop = rows
        .iter()
        .map(|row| row.component_bus_b_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_phase_drop = rows
        .iter()
        .map(|row| row.component_bus_phase_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_amplitude_drop = rows
        .iter()
        .map(|row| row.component_bus_amplitude_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_bus_wrong_pair_drop = rows
        .iter()
        .map(|row| row.component_bus_wrong_pair_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_link_phase_drop = rows
        .iter()
        .map(|row| row.component_link_phase_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_link_amplitude_drop = rows
        .iter()
        .map(|row| row.component_link_amplitude_drop)
        .fold(f32::INFINITY, f32::min);
    let min_component_link_wrong_pair_drop = rows
        .iter()
        .map(|row| row.component_link_wrong_pair_drop)
        .fold(f32::INFINITY, f32::min);
    let min_settle_link_coupling_drop = rows
        .iter()
        .map(|row| row.settle_link_coupling_drop)
        .fold(f32::INFINITY, f32::min);
    let min_settle_link_phase_drop = rows
        .iter()
        .map(|row| row.settle_link_phase_drop)
        .fold(f32::INFINITY, f32::min);
    let min_settle_link_wrong_pair_drop = rows
        .iter()
        .map(|row| row.settle_link_wrong_pair_drop)
        .fold(f32::INFINITY, f32::min);
    let component_link_seed_pairs = rows
        .iter()
        .filter(|row| row.mode_status == "organ128_modadd_component_link_candidate")
        .count();
    let settle_link_seed_pairs = rows
        .iter()
        .filter(|row| row.mode_status == "organ128_modadd_settle_link_candidate")
        .count();
    let mode_status = if passed_seed_pairs >= 4 {
        "organ128_modadd_seed_robustness_passed"
    } else if settle_link_seed_pairs >= 4 {
        "organ128_modadd_settle_link_seed_sweep_candidate"
    } else if component_link_seed_pairs >= 4 {
        "organ128_modadd_component_link_seed_sweep_candidate"
    } else if candidate_seed_pairs >= 3 {
        "organ128_modadd_seed_sweep_candidate"
    } else {
        "not_found_organ128_modadd_seed_sweep"
    };

    Organ128ModAddSeedSweepReport {
        modulus,
        train_cases,
        holdout_cases,
        rows,
        passed_seed_pairs,
        candidate_seed_pairs,
        min_ensemble_gain,
        min_phase_compose_gain,
        min_structured_compose_gain,
        min_fourier_census_gain,
        min_component_bus_projection_gain,
        min_component_link_projection_gain,
        min_settle_link_projection_gain,
        min_wave_over_fourier_gap,
        min_compose_over_fourier_gap,
        min_structured_over_fourier_gap,
        min_census_over_fourier_gap,
        min_component_bus_over_fourier_gap,
        min_component_link_over_fourier_gap,
        min_settle_link_over_fourier_gap,
        min_key_ablation_drop,
        max_label_shuffle_accuracy,
        max_component_bus_shuffle_accuracy,
        max_component_link_shuffle_accuracy,
        max_settle_link_shuffle_accuracy,
        min_component_bus_a_drop,
        min_component_bus_b_drop,
        min_component_bus_phase_drop,
        min_component_bus_amplitude_drop,
        min_component_bus_wrong_pair_drop,
        min_component_link_phase_drop,
        min_component_link_amplitude_drop,
        min_component_link_wrong_pair_drop,
        min_settle_link_coupling_drop,
        min_settle_link_phase_drop,
        min_settle_link_wrong_pair_drop,
        mode_status,
    }
}

#[derive(Clone, Copy)]
struct ModAddSample {
    a: u8,
    b: u8,
}

impl ModAddSample {
    fn target(self, modulus: u8) -> u8 {
        (self.a + self.b) % modulus
    }

    fn input_byte(self, seed: u64) -> u8 {
        let mix = splitmix64(seed ^ u64::from(self.a) << 8 ^ u64::from(self.b));
        self.a
            .wrapping_mul(31)
            .wrapping_add(self.b.wrapping_mul(17))
            .wrapping_add(mix.to_le_bytes()[0] & 15)
    }

    fn carrier(self, seed: u64) -> CarrierWave {
        let carrier_seed = seed ^ (u64::from(self.a) << 32) ^ (u64::from(self.b) << 16);
        CarrierWave::from_seed(carrier_seed, self.b).advance(self.a, 1)
    }
}

struct ModAddDataset {
    train: Vec<ModAddSample>,
    holdout: Vec<ModAddSample>,
}

impl ModAddDataset {
    fn new(config: Organ128ModAddConfig) -> Self {
        let modulus = usize::from(config.modulus);
        let mut samples = Vec::with_capacity(modulus * modulus);
        for a in 0..config.modulus {
            for b in 0..config.modulus {
                samples.push(ModAddSample { a, b });
            }
        }
        samples.sort_by_key(|sample| {
            splitmix64(
                config.seed ^ (u64::from(sample.a) << 32) ^ (u64::from(sample.b) << 16) ^ 0xADDD,
            )
        });

        let split = config.train_cases.min(samples.len());
        let end = (split + config.holdout_cases).min(samples.len());
        Self {
            train: samples[..split].to_vec(),
            holdout: samples[split..end].to_vec(),
        }
    }
}

struct ModAddTrace {
    active_cell_ids: [u32; STAGE2_TOP_K],
    center_phase: f32,
    coherence: f32,
    spectral_entropy: f32,
}

fn modadd_trace(organ: &Stage2Organ, seed: u64, sample: ModAddSample) -> ModAddTrace {
    modadd_trace_with_carrier(organ, sample.input_byte(seed), sample.carrier(seed))
}

fn modadd_bus_trace(organ: &Stage2Organ, seed: u64, sample: ModAddSample) -> WaveBus {
    run_stage2_bus_trace_with_organ_carrier(
        organ,
        sample.input_byte(seed),
        sample.carrier(seed),
        None,
    )
    .bus
}

fn modadd_trace_with_carrier(
    organ: &Stage2Organ,
    input_byte: u8,
    carrier: CarrierWave,
) -> ModAddTrace {
    let tick = run_stage2_trace_with_organ_carrier(organ, input_byte, carrier, None);
    ModAddTrace {
        active_cell_ids: tick.trace.active_cell_ids,
        center_phase: tick.trace.center_phase,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
    }
}

#[derive(Clone)]
struct ModAddReadout {
    global_offset: u8,
    cell_offsets: [u8; STAGE2_ORGAN_CELLS],
}

impl ModAddReadout {
    fn train(
        organ: &Stage2Organ,
        config: Organ128ModAddConfig,
        samples: &[ModAddSample],
        shuffle_seed: Option<u64>,
    ) -> Self {
        let mut global = OffsetAccumulator::new(config.modulus);
        let mut cells: [OffsetAccumulator; STAGE2_ORGAN_CELLS] =
            std::array::from_fn(|_| OffsetAccumulator::new(config.modulus));

        for (index, sample) in samples.iter().enumerate() {
            let trace = modadd_trace(organ, config.seed, *sample);
            let base = phase_to_mod(trace.center_phase, config.modulus);
            let target = match shuffle_seed {
                Some(seed) => {
                    let shuffled = samples[shuffle_index(seed, index, samples.len())];
                    shuffled.target(config.modulus)
                }
                None => sample.target(config.modulus),
            };
            let offset = modular_offset(base, target, config.modulus);
            global.add(offset);
            for cell_id in trace.active_cell_ids {
                cells[cell_id as usize].add(offset);
            }
        }

        Self {
            global_offset: global.finish(),
            cell_offsets: cells.map(|cell| cell.finish()),
        }
    }

    fn predict_wavebus(&self, trace: &ModAddTrace, modulus: u8) -> u8 {
        self.predict_with_filter(trace, modulus, |_| true)
    }

    fn predict_wavebus_without_cell(
        &self,
        trace: &ModAddTrace,
        modulus: u8,
        disabled_cell: usize,
    ) -> u8 {
        self.predict_with_filter(trace, modulus, |cell_id| cell_id != disabled_cell)
    }

    fn predict_restricted_cell(&self, trace: &ModAddTrace, modulus: u8, key_cell: usize) -> u8 {
        self.predict_with_filter(trace, modulus, |cell_id| cell_id == key_cell)
    }

    fn predict_voting(&self, trace: &ModAddTrace, modulus: u8) -> u8 {
        let base = phase_to_mod(trace.center_phase, modulus);
        let mut votes = [0u8; STAGE2_TOP_K];
        for (index, cell_id) in trace.active_cell_ids.iter().enumerate() {
            votes[index] = add_mod(base, self.cell_offsets[*cell_id as usize], modulus);
        }
        votes.sort_unstable();
        votes[STAGE2_TOP_K / 2]
    }

    fn predict_with_filter(
        &self,
        trace: &ModAddTrace,
        modulus: u8,
        accepts: impl Fn(usize) -> bool,
    ) -> u8 {
        let base = phase_to_mod(trace.center_phase, modulus);
        let mut offset_sum = u16::from(self.global_offset);
        let mut offset_count = 1u16;
        for cell_id in trace.active_cell_ids {
            let cell_id = cell_id as usize;
            if accepts(cell_id) {
                offset_sum += u16::from(self.cell_offsets[cell_id]);
                offset_count += 1;
            }
        }
        let offset = ((offset_sum + offset_count / 2) / offset_count) as u8 % modulus;
        add_mod(base, offset, modulus)
    }
}

#[derive(Clone, Copy)]
enum ComponentEncoding {
    Random,
    Structured,
}

#[derive(Clone, Copy)]
struct PhaseComposeReadout {
    offset: u8,
    encoding: ComponentEncoding,
}

impl PhaseComposeReadout {
    fn train(
        organ: &Stage2Organ,
        config: Organ128ModAddConfig,
        samples: &[ModAddSample],
        encoding: ComponentEncoding,
    ) -> Self {
        let mut offsets = OffsetAccumulator::new(config.modulus);
        for sample in samples {
            let base = phase_to_mod(
                composed_cell_phase(organ, config.seed, *sample, config.modulus, encoding)
                    .rem_euclid(TAU),
                config.modulus,
            );
            offsets.add(modular_offset(
                base,
                sample.target(config.modulus),
                config.modulus,
            ));
        }
        Self {
            offset: offsets.finish(),
            encoding,
        }
    }

    fn predict(self, organ: &Stage2Organ, seed: u64, sample: ModAddSample, modulus: u8) -> u8 {
        let base = phase_to_mod(
            composed_cell_phase(organ, seed, sample, modulus, self.encoding),
            modulus,
        );
        add_mod(base, self.offset, modulus)
    }
}

#[derive(Clone)]
struct FourierCensusReadout {
    slot_offsets: [u8; PHASE_SLOTS],
}

impl FourierCensusReadout {
    fn train(organ: &Stage2Organ, config: Organ128ModAddConfig, samples: &[ModAddSample]) -> Self {
        let mut offsets: [OffsetAccumulator; PHASE_SLOTS] =
            std::array::from_fn(|_| OffsetAccumulator::new(config.modulus));
        for sample in samples {
            let bus = modadd_bus_trace(organ, config.seed, *sample);
            let target = sample.target(config.modulus);
            for (slot, weight) in bus.phase_sum.iter().copied().enumerate() {
                if weight.abs() <= f32::EPSILON {
                    continue;
                }
                let base = bus_slot_to_mod(slot, weight, config.modulus);
                offsets[slot].add(modular_offset(base, target, config.modulus));
            }
        }

        Self {
            slot_offsets: offsets.map(|slot| slot.finish()),
        }
    }

    fn predict(&self, organ: &Stage2Organ, seed: u64, sample: ModAddSample, modulus: u8) -> u8 {
        let bus = modadd_bus_trace(organ, seed, sample);
        weighted_slot_vote(&bus, &self.slot_offsets, modulus)
    }
}

#[derive(Clone)]
struct ComponentBusProjectionReadout {
    centroids: Vec<[f32; COMPONENT_BUS_FEATURES]>,
}

#[derive(Clone)]
struct ComponentLinkProjectionReadout {
    centroids: Vec<[f32; COMPONENT_LINK_FEATURES]>,
}

#[derive(Clone)]
struct SettleLinkProjectionReadout {
    centroids: Vec<[f32; SETTLE_LINK_FEATURES]>,
}

#[derive(Clone, Copy)]
enum ComponentBusFeatureMode {
    Full,
    AOnly,
    BOnly,
    NoPhase,
    NoAmplitude,
    WrongPair,
}

#[derive(Clone, Copy)]
enum ComponentLinkFeatureMode {
    Full,
    NoPhase,
    NoAmplitude,
    WrongPair,
}

#[derive(Clone, Copy)]
enum SettleLinkFeatureMode {
    Full,
    NoCoupling,
    NoPhase,
    WrongPair,
}

impl ComponentBusProjectionReadout {
    fn train(
        organ: &Stage2Organ,
        config: Organ128ModAddConfig,
        samples: &[ModAddSample],
        shuffle_seed: Option<u64>,
    ) -> Self {
        let classes = usize::from(config.modulus);
        let mut centroids = vec![[0.0; COMPONENT_BUS_FEATURES]; classes];
        let mut counts = vec![0usize; classes];
        for (index, sample) in samples.iter().enumerate() {
            let label_sample = match shuffle_seed {
                Some(seed) => samples[shuffle_index(seed, index, samples.len())],
                None => *sample,
            };
            let target = usize::from(label_sample.target(config.modulus));
            let features = component_bus_features(
                organ,
                config.seed,
                *sample,
                config.modulus,
                ComponentBusFeatureMode::Full,
            );
            add_features(&mut centroids[target], &features);
            counts[target] += 1;
        }
        for (centroid, count) in centroids.iter_mut().zip(counts) {
            if count > 0 {
                scale_features(centroid, 1.0 / count as f32);
                normalize_features(centroid);
            }
        }
        Self { centroids }
    }

    fn predict(&self, organ: &Stage2Organ, seed: u64, sample: ModAddSample, modulus: u8) -> u8 {
        self.predict_with_mode(organ, seed, sample, modulus, ComponentBusFeatureMode::Full)
    }

    fn predict_with_mode(
        &self,
        organ: &Stage2Organ,
        seed: u64,
        sample: ModAddSample,
        modulus: u8,
        mode: ComponentBusFeatureMode,
    ) -> u8 {
        let features = component_bus_features(organ, seed, sample, modulus, mode);
        self.centroids
            .iter()
            .enumerate()
            .map(|(class, centroid)| (class, dot_features(&features, centroid)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map_or(0, |(class, _)| class as u8)
    }
}

impl ComponentLinkProjectionReadout {
    fn train(
        organ: &Stage2Organ,
        config: Organ128ModAddConfig,
        samples: &[ModAddSample],
        shuffle_seed: Option<u64>,
    ) -> Self {
        let classes = usize::from(config.modulus);
        let mut centroids = vec![[0.0; COMPONENT_LINK_FEATURES]; classes];
        let mut counts = vec![0usize; classes];
        for (index, sample) in samples.iter().enumerate() {
            let label_sample = match shuffle_seed {
                Some(seed) => samples[shuffle_index(seed, index, samples.len())],
                None => *sample,
            };
            let target = usize::from(label_sample.target(config.modulus));
            let features = component_link_features(
                organ,
                config.seed,
                *sample,
                config.modulus,
                ComponentLinkFeatureMode::Full,
            );
            add_features(&mut centroids[target], &features);
            counts[target] += 1;
        }
        for (centroid, count) in centroids.iter_mut().zip(counts) {
            if count > 0 {
                scale_features(centroid, 1.0 / count as f32);
                normalize_features(centroid);
            }
        }
        Self { centroids }
    }

    fn predict(&self, organ: &Stage2Organ, seed: u64, sample: ModAddSample, modulus: u8) -> u8 {
        self.predict_with_mode(organ, seed, sample, modulus, ComponentLinkFeatureMode::Full)
    }

    fn predict_with_mode(
        &self,
        organ: &Stage2Organ,
        seed: u64,
        sample: ModAddSample,
        modulus: u8,
        mode: ComponentLinkFeatureMode,
    ) -> u8 {
        let features = component_link_features(organ, seed, sample, modulus, mode);
        self.centroids
            .iter()
            .enumerate()
            .map(|(class, centroid)| (class, dot_features(&features, centroid)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map_or(0, |(class, _)| class as u8)
    }
}

impl SettleLinkProjectionReadout {
    fn train(
        organ: &Stage2Organ,
        config: Organ128ModAddConfig,
        samples: &[ModAddSample],
        shuffle_seed: Option<u64>,
    ) -> Self {
        let classes = usize::from(config.modulus);
        let mut centroids = vec![[0.0; SETTLE_LINK_FEATURES]; classes];
        let mut counts = vec![0usize; classes];
        for (index, sample) in samples.iter().enumerate() {
            let label_sample = match shuffle_seed {
                Some(seed) => samples[shuffle_index(seed, index, samples.len())],
                None => *sample,
            };
            let target = usize::from(label_sample.target(config.modulus));
            let features = settle_link_features(
                organ,
                config.seed,
                *sample,
                config.modulus,
                SettleLinkFeatureMode::Full,
            );
            add_features(&mut centroids[target], &features);
            counts[target] += 1;
        }
        for (centroid, count) in centroids.iter_mut().zip(counts) {
            if count > 0 {
                scale_features(centroid, 1.0 / count as f32);
                normalize_features(centroid);
            }
        }
        Self { centroids }
    }

    fn predict(&self, organ: &Stage2Organ, seed: u64, sample: ModAddSample, modulus: u8) -> u8 {
        self.predict_with_mode(organ, seed, sample, modulus, SettleLinkFeatureMode::Full)
    }

    fn predict_with_mode(
        &self,
        organ: &Stage2Organ,
        seed: u64,
        sample: ModAddSample,
        modulus: u8,
        mode: SettleLinkFeatureMode,
    ) -> u8 {
        let features = settle_link_features(organ, seed, sample, modulus, mode);
        self.centroids
            .iter()
            .enumerate()
            .map(|(class, centroid)| (class, dot_features(&features, centroid)))
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map_or(0, |(class, _)| class as u8)
    }
}

#[derive(Clone, Copy)]
struct OffsetAccumulator {
    modulus: u8,
    sin_sum: f32,
    cos_sum: f32,
    count: usize,
}

impl OffsetAccumulator {
    fn new(modulus: u8) -> Self {
        Self {
            modulus,
            sin_sum: 0.0,
            cos_sum: 0.0,
            count: 0,
        }
    }

    fn add(&mut self, offset: u8) {
        let angle = f32::from(offset) / f32::from(self.modulus) * TAU;
        self.sin_sum += angle.sin();
        self.cos_sum += angle.cos();
        self.count += 1;
    }

    fn finish(self) -> u8 {
        if self.count == 0 {
            return 0;
        }
        let angle = self.sin_sum.atan2(self.cos_sum).rem_euclid(TAU);
        ((angle / TAU * f32::from(self.modulus)).round() as u8) % self.modulus
    }
}

fn finish_all<const N: usize>(results: [&mut BaselineResult; N]) {
    for result in results {
        result.finish();
    }
}

fn key_ablation(
    full: &BaselineResult,
    ablations: [BaselineResult; STAGE2_ORGAN_CELLS],
) -> (u32, BaselineResult, f32, f32) {
    let mut key_cell = 0usize;
    let mut key_drop = f32::NEG_INFINITY;
    let mut non_key_drop = 0.0f32;
    for (cell_id, ablation) in ablations.iter().enumerate() {
        let drop = full.accuracy - ablation.accuracy;
        if drop > key_drop {
            if key_drop.is_finite() {
                non_key_drop = non_key_drop.max(key_drop);
            }
            key_cell = cell_id;
            key_drop = drop;
        } else {
            non_key_drop = non_key_drop.max(drop);
        }
    }
    (
        key_cell as u32,
        ablations[key_cell],
        key_drop.max(0.0),
        non_key_drop.max(0.0),
    )
}

fn phase_to_mod(phase: f32, modulus: u8) -> u8 {
    let unit = (phase / TAU).rem_euclid(1.0);
    ((unit * f32::from(modulus)).round() as u8) % modulus
}

fn modular_offset(from: u8, to: u8, modulus: u8) -> u8 {
    (to + modulus - from) % modulus
}

fn add_mod(value: u8, offset: u8, modulus: u8) -> u8 {
    (value + offset) % modulus
}

fn ablation_drop(full: f32, ablated: f32) -> f32 {
    (full - ablated).max(0.0)
}

fn bus_slot_to_mod(slot: usize, weight: f32, modulus: u8) -> u8 {
    let slot_phase = TAU * slot as f32 / PHASE_SLOTS as f32;
    let signed_phase = if weight.is_sign_negative() {
        slot_phase + std::f32::consts::PI
    } else {
        slot_phase
    };
    phase_to_mod(signed_phase, modulus)
}

fn weighted_slot_vote(bus: &WaveBus, slot_offsets: &[u8; PHASE_SLOTS], modulus: u8) -> u8 {
    let mut sin_sum = 0.0;
    let mut cos_sum = 0.0;
    for (slot, weight) in bus.phase_sum.iter().copied().enumerate() {
        let vote_weight = weight.abs();
        if vote_weight <= f32::EPSILON {
            continue;
        }
        let base = bus_slot_to_mod(slot, weight, modulus);
        let prediction = add_mod(base, slot_offsets[slot], modulus);
        let angle = f32::from(prediction) / f32::from(modulus) * TAU;
        sin_sum += vote_weight * angle.sin();
        cos_sum += vote_weight * angle.cos();
    }
    if sin_sum == 0.0 && cos_sum == 0.0 {
        0
    } else {
        phase_to_mod(sin_sum.atan2(cos_sum), modulus)
    }
}

fn random_mod_predict(seed: u64, case_index: usize, sample: ModAddSample, modulus: u8) -> u8 {
    (splitmix64(seed ^ case_index as u64 ^ u64::from(sample.input_byte(seed))) % u64::from(modulus))
        as u8
}

fn mono_mod_predict(seed: u64, case_index: usize, sample: ModAddSample, modulus: u8) -> u8 {
    let mixed = splitmix64(
        seed.rotate_left(7)
            ^ (case_index as u64).wrapping_mul(0x9E37)
            ^ u64::from(sample.input_byte(seed))
            ^ 0x192,
    );
    (mixed % u64::from(modulus)) as u8
}

fn composed_cell_phase(
    organ: &Stage2Organ,
    seed: u64,
    sample: ModAddSample,
    modulus: u8,
    encoding: ComponentEncoding,
) -> f32 {
    let a_trace = component_trace(organ, seed, sample.a, modulus, 0xA17A, encoding);
    let b_trace = component_trace(organ, seed, sample.b, modulus, 0xB17B, encoding);
    (a_trace.center_phase + b_trace.center_phase).rem_euclid(TAU)
}

fn component_trace(
    organ: &Stage2Organ,
    seed: u64,
    value: u8,
    modulus: u8,
    lane: u64,
    encoding: ComponentEncoding,
) -> ModAddTrace {
    let carrier = match encoding {
        ComponentEncoding::Random => CarrierWave::from_seed(seed ^ lane, value),
        ComponentEncoding::Structured => structured_component_carrier(value, modulus, lane),
    };
    let input_byte = match encoding {
        ComponentEncoding::Random => value.wrapping_mul(37).wrapping_add(lane as u8),
        ComponentEncoding::Structured => lane as u8,
    };
    modadd_trace_with_carrier(organ, input_byte, carrier)
}

fn component_bus_features(
    organ: &Stage2Organ,
    seed: u64,
    sample: ModAddSample,
    modulus: u8,
    mode: ComponentBusFeatureMode,
) -> [f32; COMPONENT_BUS_FEATURES] {
    let a_bus = component_bus(organ, seed, sample.a, modulus, 0xA17A);
    let b_value = if matches!(mode, ComponentBusFeatureMode::WrongPair) {
        (sample.b + modulus / 2 + 1) % modulus
    } else {
        sample.b
    };
    let b_bus = component_bus(organ, seed, b_value, modulus, 0xB17B);
    let mut features = [0.0; COMPONENT_BUS_FEATURES];
    if !matches!(mode, ComponentBusFeatureMode::BOnly) {
        write_bus_features(&a_bus, &mut features, 0, mode);
    }
    if !matches!(mode, ComponentBusFeatureMode::AOnly) {
        write_bus_features(&b_bus, &mut features, PHASE_SLOTS * 2, mode);
    }
    normalize_features(&mut features);
    features
}

fn component_link_features(
    organ: &Stage2Organ,
    seed: u64,
    sample: ModAddSample,
    modulus: u8,
    mode: ComponentLinkFeatureMode,
) -> [f32; COMPONENT_LINK_FEATURES] {
    let a_bus = component_bus(organ, seed, sample.a, modulus, 0xA17A);
    let b_value = if matches!(mode, ComponentLinkFeatureMode::WrongPair) {
        (sample.b + modulus / 2 + 1) % modulus
    } else {
        sample.b
    };
    let b_bus = component_bus(organ, seed, b_value, modulus, 0xB17B);
    let mut features = [0.0; COMPONENT_LINK_FEATURES];

    let a_phase_norm = a_bus
        .phase_sum
        .iter()
        .map(|value| value.abs())
        .sum::<f32>()
        .max(f32::EPSILON);
    let b_phase_norm = b_bus
        .phase_sum
        .iter()
        .map(|value| value.abs())
        .sum::<f32>()
        .max(f32::EPSILON);
    let a_amplitude_norm = a_bus.amplitude_sum.iter().sum::<f32>().max(f32::EPSILON);
    let b_amplitude_norm = b_bus.amplitude_sum.iter().sum::<f32>().max(f32::EPSILON);

    for output_slot in 0..PHASE_SLOTS {
        let mut phase_link = 0.0;
        let mut amplitude_link = 0.0;
        for a_slot in 0..PHASE_SLOTS {
            let b_slot = (output_slot + PHASE_SLOTS - a_slot) % PHASE_SLOTS;
            if !matches!(mode, ComponentLinkFeatureMode::NoPhase) {
                phase_link += (a_bus.phase_sum[a_slot] / a_phase_norm)
                    * (b_bus.phase_sum[b_slot] / b_phase_norm);
            }
            if !matches!(mode, ComponentLinkFeatureMode::NoAmplitude) {
                amplitude_link += (a_bus.amplitude_sum[a_slot] / a_amplitude_norm)
                    * (b_bus.amplitude_sum[b_slot] / b_amplitude_norm);
            }
        }
        features[output_slot] = phase_link;
        features[PHASE_SLOTS + output_slot] = amplitude_link;
    }

    normalize_features(&mut features);
    features
}

fn settle_link_features(
    organ: &Stage2Organ,
    seed: u64,
    sample: ModAddSample,
    modulus: u8,
    mode: SettleLinkFeatureMode,
) -> [f32; SETTLE_LINK_FEATURES] {
    let b_value = if matches!(mode, SettleLinkFeatureMode::WrongPair) {
        (sample.b + modulus / 2 + 1) % modulus
    } else {
        sample.b
    };
    let mut state = OrganState::new(seed ^ 0x51E7_11CE, 0x51);

    let a_carrier = structured_component_carrier(sample.a, modulus, 0xA17A ^ seed);
    let a_input = sample.a.wrapping_mul(3).wrapping_add(0xA1);
    let _ = state.settle_bus_tick_with_carrier(organ, a_input, a_carrier, None);
    let b_carrier = structured_component_carrier(b_value, modulus, 0xB17B ^ seed);
    let b_input = b_value.wrapping_mul(5).wrapping_add(0xB1);
    let tick = state.settle_bus_tick_with_carrier(organ, b_input, b_carrier, None);

    let mut features = [0.0; SETTLE_LINK_FEATURES];
    if !matches!(mode, SettleLinkFeatureMode::NoPhase) {
        let phase_norm = state
            .link_phase_sum
            .iter()
            .map(|value| value.abs())
            .sum::<f32>()
            .max(f32::EPSILON);
        for (slot, value) in state.link_phase_sum.iter().copied().enumerate() {
            features[slot] = value / phase_norm;
        }
        let center_slot = phase_to_slot(tick.trace.center_phase);
        features[center_slot] += tick.trace.center_magnitude.max(0.05) * 0.25;
    }

    let amplitude_offset = PHASE_SLOTS;
    let amplitude_norm = state
        .link_amplitude_sum
        .iter()
        .sum::<f32>()
        .max(f32::EPSILON);
    for (slot, value) in state.link_amplitude_sum.iter().copied().enumerate() {
        features[amplitude_offset + slot] = value / amplitude_norm;
    }

    let scalar_offset = PHASE_SLOTS * 2 + STAGE2_ORGAN_CELLS * 2;
    features[scalar_offset] = tick.trace.coherence;
    features[scalar_offset + 1] = 1.0 - tick.trace.spectral_entropy;
    features[scalar_offset + 2] = tick.trace.center_magnitude;
    features[scalar_offset + 3] = if matches!(mode, SettleLinkFeatureMode::NoCoupling) {
        0.0
    } else {
        state.coupling_mean() * 0.05
    };
    normalize_features(&mut features);
    features
}

fn component_bus(organ: &Stage2Organ, seed: u64, value: u8, modulus: u8, lane: u64) -> WaveBus {
    let carrier = structured_component_carrier(value, modulus, lane ^ seed);
    run_stage2_bus_trace_with_organ_carrier(organ, lane as u8, carrier, None).bus
}

fn write_bus_features(
    bus: &WaveBus,
    features: &mut [f32; COMPONENT_BUS_FEATURES],
    offset: usize,
    mode: ComponentBusFeatureMode,
) {
    let phase_norm = bus
        .phase_sum
        .iter()
        .map(|value| value.abs())
        .sum::<f32>()
        .max(f32::EPSILON);
    let amplitude_norm = bus.amplitude_sum.iter().sum::<f32>().max(f32::EPSILON);
    for slot in 0..PHASE_SLOTS {
        if !matches!(mode, ComponentBusFeatureMode::NoPhase) {
            features[offset + slot] = bus.phase_sum[slot] / phase_norm;
        }
        if !matches!(mode, ComponentBusFeatureMode::NoAmplitude) {
            features[offset + PHASE_SLOTS + slot] = bus.amplitude_sum[slot] / amplitude_norm;
        }
    }
}

fn add_features<const N: usize>(target: &mut [f32; N], source: &[f32; N]) {
    for (target_value, source_value) in target.iter_mut().zip(source) {
        *target_value += source_value;
    }
}

fn scale_features<const N: usize>(features: &mut [f32; N], scale: f32) {
    for value in features {
        *value *= scale;
    }
}

fn normalize_features<const N: usize>(features: &mut [f32; N]) {
    let norm = features
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt()
        .max(f32::EPSILON);
    scale_features(features, 1.0 / norm);
}

fn dot_features<const N: usize>(left: &[f32; N], right: &[f32; N]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn structured_component_carrier(value: u8, modulus: u8, lane: u64) -> CarrierWave {
    let lane_shift = if lane & 1 == 0 { 0.0 } else { TAU * 0.25 };
    let phase = (f32::from(value) / f32::from(modulus) * TAU + lane_shift).rem_euclid(TAU);
    let unit = f32::from(value) / f32::from(modulus);
    CarrierWave {
        phase,
        amplitude: 0.85,
        frequency: 1.0 + unit,
        boundary: 0.95,
    }
}

fn phase_to_slot(phase: f32) -> usize {
    let unit = (phase / TAU).rem_euclid(1.0);
    ((unit * PHASE_SLOTS as f32).round() as usize) % PHASE_SLOTS
}

fn fourier_phase_predict(sample: ModAddSample, modulus: u8) -> u8 {
    let phase_a = f32::from(sample.a) / f32::from(modulus) * TAU;
    let phase_b = f32::from(sample.b) / f32::from(modulus) * TAU;
    phase_to_mod((phase_a + phase_b).rem_euclid(TAU), modulus)
}

fn shuffle_index(seed: u64, index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (splitmix64(seed ^ (index as u64).wrapping_mul(0xD1B5)) as usize) % len
    }
}

fn ablation_name(cell_id: usize) -> &'static str {
    match cell_id {
        0 => "modadd_ablate_cell0",
        1 => "modadd_ablate_cell1",
        2 => "modadd_ablate_cell2",
        3 => "modadd_ablate_cell3",
        4 => "modadd_ablate_cell4",
        5 => "modadd_ablate_cell5",
        _ => "modadd_ablate_cell_unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modadd_report_has_goal_v0_fields() {
        let report = organ128_modadd_eval(Organ128ModAddConfig {
            seed: 7,
            modulus: 31,
            train_cases: 64,
            holdout_cases: 64,
        });
        let text = report.to_text();
        assert_eq!(report.config.modulus, 31);
        assert_eq!(report.random.cases, 64);
        assert_eq!(report.fourier_phase.cases, 64);
        assert_eq!(report.cell32_wavebus.cases, 64);
        assert!(text.contains("task: modular_addition"));
        assert!(text.contains("random_accuracy:"));
        assert!(text.contains("mono192_accuracy:"));
        assert!(text.contains("fourier_phase_accuracy:"));
        assert!(text.contains("cell32_phase_compose_accuracy:"));
        assert!(text.contains("cell32_structured_compose_accuracy:"));
        assert!(text.contains("cell32_fourier_census_accuracy:"));
        assert!(text.contains("cell32_component_bus_projection_accuracy:"));
        assert!(text.contains("cell32_voting_accuracy:"));
        assert!(text.contains("cell32_wavebus_accuracy:"));
        assert!(text.contains("ensemble_gain:"));
        assert!(text.contains("phase_compose_gain:"));
        assert!(text.contains("structured_compose_gain:"));
        assert!(text.contains("fourier_census_gain:"));
        assert!(text.contains("component_bus_projection_gain:"));
        assert!(text.contains("component_bus_a_drop:"));
        assert!(text.contains("component_bus_b_drop:"));
        assert!(text.contains("component_bus_phase_drop:"));
        assert!(text.contains("component_bus_amplitude_drop:"));
        assert!(text.contains("component_bus_wrong_pair_drop:"));
        assert!(text.contains("wave_over_fourier_gap:"));
        assert!(text.contains("compose_over_fourier_gap:"));
        assert!(text.contains("structured_over_fourier_gap:"));
        assert!(text.contains("census_over_fourier_gap:"));
        assert!(text.contains("component_bus_over_fourier_gap:"));
        assert!(text.contains("label_shuffle_accuracy:"));
        assert!(text.contains("component_bus_shuffle_accuracy:"));
        assert!(text.contains("no_shortcut_control:"));
        assert!(text.contains("component_bus_no_shortcut_control:"));
        assert!(
            report.mode_status == "organ128_modadd_key_mode_ablation_passed"
                || report.mode_status == "organ128_modadd_settle_link_candidate"
                || report.mode_status == "organ128_modadd_component_link_candidate"
                || report.mode_status == "organ128_modadd_candidate"
                || report.mode_status == "not_found_organ128_modadd"
        );
    }

    #[test]
    fn modadd_seed_sweep_report_has_robustness_fields() {
        let report = organ128_modadd_seed_sweep_eval(31, 32, 32);
        let text = report.to_text();
        assert_eq!(report.rows.len(), 5);
        assert!(text.contains("modadd seed-sweep eval"));
        assert!(text.contains("passed_seed_pairs:"));
        assert!(text.contains("candidate_seed_pairs:"));
        assert!(text.contains("min_phase_compose_gain:"));
        assert!(text.contains("min_structured_compose_gain:"));
        assert!(text.contains("min_fourier_census_gain:"));
        assert!(text.contains("min_component_bus_projection_gain:"));
        assert!(text.contains("min_wave_over_fourier_gap:"));
        assert!(text.contains("min_compose_over_fourier_gap:"));
        assert!(text.contains("min_structured_over_fourier_gap:"));
        assert!(text.contains("min_census_over_fourier_gap:"));
        assert!(text.contains("min_component_bus_over_fourier_gap:"));
        assert!(text.contains("max_label_shuffle_accuracy:"));
        assert!(text.contains("max_component_bus_shuffle_accuracy:"));
        assert!(text.contains("min_component_bus_a_drop:"));
        assert!(text.contains("min_component_bus_b_drop:"));
        assert!(text.contains("min_component_bus_phase_drop:"));
        assert!(text.contains("min_component_bus_amplitude_drop:"));
        assert!(text.contains("min_component_bus_wrong_pair_drop:"));
        assert!(
            report.mode_status == "organ128_modadd_seed_robustness_passed"
                || report.mode_status == "organ128_modadd_settle_link_seed_sweep_candidate"
                || report.mode_status == "organ128_modadd_component_link_seed_sweep_candidate"
                || report.mode_status == "organ128_modadd_seed_sweep_candidate"
                || report.mode_status == "not_found_organ128_modadd_seed_sweep"
        );
    }
}
