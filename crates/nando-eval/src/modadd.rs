use std::time::Instant;

use crate::{BaselineResult, best_baseline, format_baseline, score_prediction, splitmix64};
use nando_core::{
    CarrierWave, STAGE2_ORGAN_CELLS, STAGE2_TOP_K, Stage2Organ, run_stage2_trace_with_organ_carrier,
};

const TAU: f32 = std::f32::consts::TAU;
const MIN_ENSEMBLE_GAIN: f32 = 0.03;
const MIN_KEY_ABLATION_DROP: f32 = 0.05;
const MAX_FALSE_POSITIVE_INCREASE: f32 = 0.02;
const MODADD_SWEEP_SEEDS: [u64; 5] = [7, 13, 29, 97, 131];

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
    pub cell32_voting: BaselineResult,
    pub cell32_wavebus: BaselineResult,
    pub restricted_key: BaselineResult,
    pub excluded_key: BaselineResult,
    pub label_shuffle: BaselineResult,
    pub key_cell: u32,
    pub ensemble_gain: f32,
    pub wave_over_fourier_gap: f32,
    pub key_ablation_drop: f32,
    pub non_key_ablation_drop: f32,
    pub no_shortcut_control: bool,
    pub scientific_pass: bool,
    pub engineering_pass: bool,
    pub mode_status: &'static str,
}

/// One row in the modular-addition seed sweep.
#[derive(Debug, Clone, PartialEq)]
pub struct Organ128ModAddSeedSweepRow {
    pub seed: u64,
    pub cell32_wavebus_accuracy: f32,
    pub fourier_phase_accuracy: f32,
    pub ensemble_gain: f32,
    pub wave_over_fourier_gap: f32,
    pub key_ablation_drop: f32,
    pub non_key_ablation_drop: f32,
    pub label_shuffle_accuracy: f32,
    pub no_shortcut_control: bool,
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
    pub min_wave_over_fourier_gap: f32,
    pub min_key_ablation_drop: f32,
    pub max_label_shuffle_accuracy: f32,
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
        output.push_str(&format_baseline(self.cell32_voting));
        output.push_str(&format_baseline(self.cell32_wavebus));
        output.push_str(&format_baseline(self.restricted_key));
        output.push_str(&format_baseline(self.excluded_key));
        output.push_str(&format_baseline(self.label_shuffle));
        output.push_str(&format!("random_accuracy: {:.6}\n", self.random.accuracy));
        output.push_str(&format!("mono192_accuracy: {:.6}\n", self.mono192.accuracy));
        output.push_str(&format!(
            "fourier_phase_accuracy: {:.6}\n",
            self.fourier_phase.accuracy
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
            "wave_over_fourier_gap: {:.6}\n",
            self.wave_over_fourier_gap
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
            "no_shortcut_control: {}\n",
            self.no_shortcut_control
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
        output.push_str("seed wavebus_acc fourier_acc wave_fourier_gap ensemble_gain key_drop non_key_drop shuffle_acc no_shortcut scientific engineering mode_status\n");
        for row in &self.rows {
            output.push_str(&format!(
                "{} {:.6} {:.6} {:+.6} {:+.6} {:.6} {:.6} {:.6} {} {} {} {}\n",
                row.seed,
                row.cell32_wavebus_accuracy,
                row.fourier_phase_accuracy,
                row.wave_over_fourier_gap,
                row.ensemble_gain,
                row.key_ablation_drop,
                row.non_key_ablation_drop,
                row.label_shuffle_accuracy,
                row.no_shortcut_control,
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
            "min_wave_over_fourier_gap: {:.6}\n",
            self.min_wave_over_fourier_gap
        ));
        output.push_str(&format!(
            "min_key_ablation_drop: {:.6}\n",
            self.min_key_ablation_drop
        ));
        output.push_str(&format!(
            "max_label_shuffle_accuracy: {:.6}\n",
            self.max_label_shuffle_accuracy
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
    let shuffled_readout = ModAddReadout::train(&organ, config, &dataset.train, Some(config.seed));

    let mut random = BaselineResult::new("random", dataset.holdout.len());
    let mut mono192 = BaselineResult::new("mono192", dataset.holdout.len());
    let mut fourier_phase = BaselineResult::new("fourier_phase_control", dataset.holdout.len());
    let mut cell32_voting = BaselineResult::new("cell32_voting", dataset.holdout.len());
    let mut cell32_wavebus = BaselineResult::new("cell32_wavebus", dataset.holdout.len());
    let mut label_shuffle = BaselineResult::new("label_shuffle", dataset.holdout.len());
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
        &mut cell32_voting,
        &mut cell32_wavebus,
        &mut label_shuffle,
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
    let wave_over_fourier_gap = cell32_wavebus.accuracy - fourier_phase.accuracy;
    let no_shortcut_control = label_shuffle.accuracy
        <= (random.accuracy + MAX_FALSE_POSITIVE_INCREASE).min(cell32_wavebus.accuracy);
    let scientific_pass = ensemble_gain >= MIN_ENSEMBLE_GAIN
        && cell32_wavebus.accuracy > mono192.accuracy
        && key_ablation_drop >= MIN_KEY_ABLATION_DROP
        && key_ablation_drop >= non_key_ablation_drop * 2.0
        && no_shortcut_control;
    let engineering_pass = cell32_wavebus.accuracy + 0.01 >= mono192.accuracy
        && key_ablation_drop >= MIN_KEY_ABLATION_DROP
        && no_shortcut_control;
    let mode_status = if scientific_pass {
        "organ128_modadd_key_mode_ablation_passed"
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
        cell32_voting,
        cell32_wavebus,
        restricted_key,
        excluded_key,
        label_shuffle,
        key_cell,
        ensemble_gain,
        wave_over_fourier_gap,
        key_ablation_drop,
        non_key_ablation_drop,
        no_shortcut_control,
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
            fourier_phase_accuracy: report.fourier_phase.accuracy,
            ensemble_gain: report.ensemble_gain,
            wave_over_fourier_gap: report.wave_over_fourier_gap,
            key_ablation_drop: report.key_ablation_drop,
            non_key_ablation_drop: report.non_key_ablation_drop,
            label_shuffle_accuracy: report.label_shuffle.accuracy,
            no_shortcut_control: report.no_shortcut_control,
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
        })
        .count();
    let min_ensemble_gain = rows
        .iter()
        .map(|row| row.ensemble_gain)
        .fold(f32::INFINITY, f32::min);
    let min_wave_over_fourier_gap = rows
        .iter()
        .map(|row| row.wave_over_fourier_gap)
        .fold(f32::INFINITY, f32::min);
    let min_key_ablation_drop = rows
        .iter()
        .map(|row| row.key_ablation_drop)
        .fold(f32::INFINITY, f32::min);
    let max_label_shuffle_accuracy = rows
        .iter()
        .map(|row| row.label_shuffle_accuracy)
        .fold(f32::NEG_INFINITY, f32::max);
    let mode_status = if passed_seed_pairs >= 4 {
        "organ128_modadd_seed_robustness_passed"
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
        min_wave_over_fourier_gap,
        min_key_ablation_drop,
        max_label_shuffle_accuracy,
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
    let tick = run_stage2_trace_with_organ_carrier(
        organ,
        sample.input_byte(seed),
        sample.carrier(seed),
        None,
    );
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
        assert!(text.contains("cell32_voting_accuracy:"));
        assert!(text.contains("cell32_wavebus_accuracy:"));
        assert!(text.contains("ensemble_gain:"));
        assert!(text.contains("wave_over_fourier_gap:"));
        assert!(text.contains("label_shuffle_accuracy:"));
        assert!(text.contains("no_shortcut_control:"));
        assert!(
            report.mode_status == "organ128_modadd_key_mode_ablation_passed"
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
        assert!(text.contains("min_wave_over_fourier_gap:"));
        assert!(text.contains("max_label_shuffle_accuracy:"));
        assert!(
            report.mode_status == "organ128_modadd_seed_robustness_passed"
                || report.mode_status == "organ128_modadd_seed_sweep_candidate"
                || report.mode_status == "not_found_organ128_modadd_seed_sweep"
        );
    }
}
