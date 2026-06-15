use crate::args::{parse_u64, parse_usize};
use nando_core::{
    BytePhaseLut, CacheAwareOrganPlan, CarrierWave, Cell32, CellRank, PHASE_SLOTS, WaveBus,
};

const PROMPT_WAVE_TOP_SLOTS: usize = 8;

const TRAIN_CORPUS: &[u8] = include_bytes!("../../../data/corpus/organ128_train_v1.txt");

const RESPONSE_GATE_REFUSAL_PROMPTS: &[&str] = &[
    "что такое",
    "расскажи про",
    "зачем",
    "what is",
    "why",
    "nando organ128 cell32 wave bus snapshot",
    "carrier wave rust память ответ",
    "what is what is what is",
    "что такое что такое что такое",
    "объясни это",
];

pub(crate) fn run_organ128_train_generate(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let epochs = match args.next() {
        Some(value) => parse_usize(&value, "epochs")?,
        None => 24,
    };
    let prompt = args.next().unwrap_or_else(|| String::from("nando "));
    let generate_len = match args.next() {
        Some(value) => parse_usize(&value, "generate-len")?,
        None => 96,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if epochs == 0 {
        return Err(String::from("epochs must be greater than zero"));
    }
    if prompt.is_empty() {
        return Err(String::from("prompt must not be empty"));
    }

    let plan = CacheAwareOrganPlan::t480_organ128();
    let organ = Organ128Runtime::new(seed);
    let lut = BytePhaseLut::new();
    let mut learner = Organ128ByteLearner::new(0.06);
    let prompt_wave = PromptWave::from_prompt(&lut, &prompt);
    let train_report = train_organ128(&organ, &lut, &mut learner, seed, TRAIN_CORPUS, epochs);
    let generated = generate_organ128(
        &organ,
        &lut,
        &learner,
        seed,
        &prompt,
        prompt_wave,
        generate_len,
    );

    println!("Nando Wave Organ128 train-generate");
    println!("seed: {seed}");
    println!("epochs: {epochs}");
    println!("train_bytes: {}", TRAIN_CORPUS.len());
    println!("organ128_cells: {}", plan.organ128.cell_count());
    println!("organ128_bytes: {}", plan.organ128_bytes);
    println!(
        "l1_active_cells_total: {}",
        plan.hot_window.l1_active_cells_total
    );
    println!("l2_hot_cells_total: {}", plan.hot_window.l2_hot_cells_total);
    println!("train_cases: {}", train_report.cases);
    println!("train_accuracy_before_update: {:.4}", train_report.accuracy);
    println!("state_abs_mean: {:.6}", learner.state_abs_mean());
    println!("prompt: {prompt}");
    println!("prompt_wave_phase: {:.6}", prompt_wave.phase);
    println!("prompt_wave_amplitude: {:.6}", prompt_wave.amplitude);
    println!("prompt_wave_top_slots: {:?}", prompt_wave.top_slots);
    println!("generated: {}", generated.escape_default());
    println!("mode_status: organ128_generated_text_smoke");
    Ok(())
}

pub(crate) fn run_organ128_dialog_generate(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let prompt = args
        .next()
        .unwrap_or_else(|| String::from("что такое nando"));
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if prompt.is_empty() {
        return Err(String::from("prompt must not be empty"));
    }

    let lut = BytePhaseLut::new();
    let organ = Organ128Runtime::new(seed);
    let prompt_wave = PromptWave::from_prompt(&lut, &prompt);
    let (entry, score) = best_dialog_entry(&lut, &prompt, prompt_wave);
    let answer_wave = PromptWave::from_prompt(&lut, entry.answer);
    let active = organ.active_cells(
        &lut,
        seed,
        prompt.as_bytes().last().copied().unwrap_or(b' '),
        prompt_wave,
        0,
    );

    println!("Nando Wave Organ128 dialog-generate");
    println!("seed: {seed}");
    println!("prompt: {prompt}");
    println!("prompt_wave_phase: {:.6}", prompt_wave.phase);
    println!("prompt_wave_amplitude: {:.6}", prompt_wave.amplitude);
    println!("prompt_wave_top_slots: {:?}", prompt_wave.top_slots);
    println!("matched_prompt: {}", entry.prompt);
    println!("match_score: {:.6}", score);
    println!("answer_wave_phase: {:.6}", answer_wave.phase);
    println!("answer_wave_amplitude: {:.6}", answer_wave.amplitude);
    println!("active_cell_ids: {:?}", active.map(|(cell, _)| cell));
    println!("answer: {}", entry.answer);
    println!("mode_status: organ128_dialog_memory_answered");
    Ok(())
}

pub(crate) fn run_organ128_settle_dialog(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let prompt = args
        .next()
        .unwrap_or_else(|| String::from("что такое nando"));
    let ticks = match args.next() {
        Some(value) => parse_usize(&value, "ticks")?,
        None => 5,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if prompt.is_empty() {
        return Err(String::from("prompt must not be empty"));
    }
    if !(1..=16).contains(&ticks) {
        return Err(String::from("ticks must be in 1..=16"));
    }

    let lut = BytePhaseLut::new();
    let organ = Organ128Runtime::new(seed);
    let prompt_wave = PromptWave::from_prompt(&lut, &prompt);
    let settled = organ.settle_dialog(
        &lut,
        seed,
        &prompt,
        prompt_wave,
        ticks,
        CarrierMode::Correct,
    );
    let selected = best_settled_dialog_entry(&lut, &prompt, prompt_wave, &settled);
    let wave_selected = best_wave_dialog_entry(&lut, &settled);
    let no_carrier =
        organ.settle_dialog(&lut, seed, &prompt, prompt_wave, ticks, CarrierMode::None);
    let wrong_carrier =
        organ.settle_dialog(&lut, seed, &prompt, prompt_wave, ticks, CarrierMode::Wrong);
    let corrupted = organ.settle_dialog(
        &lut,
        seed,
        &prompt,
        prompt_wave,
        ticks,
        CarrierMode::CorruptedPrompt,
    );
    let no_selected = best_settled_dialog_entry(&lut, &prompt, prompt_wave, &no_carrier);
    let wrong_selected = best_settled_dialog_entry(&lut, &prompt, prompt_wave, &wrong_carrier);
    let corrupt_selected = best_settled_dialog_entry(&lut, &prompt, prompt_wave, &corrupted);

    println!("Nando Wave Organ128 settle-dialog");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    println!("prompt: {prompt}");
    println!("prompt_wave_phase: {:.6}", prompt_wave.phase);
    println!("prompt_wave_amplitude: {:.6}", prompt_wave.amplitude);
    println!("prompt_wave_top_slots: {:?}", prompt_wave.top_slots);
    println!("cache_layout:");
    println!("l3_warm: cells=128 fast=64 mid=32 guard=16 carrier=8 memory=8 bytes=4194304");
    println!("l2_hot: top32 cells selected by resonance each tick");
    println!("l1_active: quota top4 = 2 Fast + 1 Mid/Guard + 1 Carrier/Memory");
    println!("trace:");
    println!(
        "tick input carrier_phase center_phase coherence entropy phase_velocity l2_roles l1_roles active_cells memory memory_top_slots"
    );
    for tick in &settled.ticks {
        println!(
            "{:>4} {:>5} {:>13.6} {:>12.6} {:>9.6} {:>7.6} {:>14.6} {} {} {:?} {:.6}/{:.6}/{:.6} {:?}",
            tick.tick,
            tick.input_byte,
            tick.carrier_phase,
            tick.center_phase,
            tick.coherence,
            tick.entropy,
            tick.phase_velocity,
            tick.l2_roles.to_compact_text(),
            tick.l1_roles.to_compact_text(),
            tick.active_cell_ids,
            tick.memory.phase,
            tick.memory.strength,
            tick.memory.validation,
            tick.memory.top_slots
        );
    }
    println!("settled_center_phase: {:.6}", settled.center_phase);
    println!("settled_coherence: {:.6}", settled.coherence);
    println!("settled_entropy: {:.6}", settled.entropy);
    println!("settled_stability: {:.6}", settled.stability);
    println!("settle_verdict: {}", settled.verdict().as_str());
    println!("thought_phase: {:.6}", settled.thought.phase);
    println!("thought_strength: {:.6}", settled.thought.strength);
    println!("thought_convergence: {:.6}", settled.thought.convergence);
    println!("thought_drift: {:.6}", settled.thought.drift);
    println!(
        "thought_prompt_specificity: {:.6}",
        settled.thought.prompt_specificity
    );
    println!("thought_specificity: {:.6}", settled.thought.specificity);
    println!("thought_role_balance: {:.6}", settled.thought.role_balance);
    println!("thought_verdict: {}", settled.thought.verdict().as_str());
    println!("memory_phase: {:.6}", settled.memory.phase);
    println!("memory_strength: {:.6}", settled.memory.strength);
    println!("memory_validation: {:.6}", settled.memory.validation);
    println!("memory_top_slots: {:?}", settled.memory.top_slots);
    println!("matched_prompt: {}", selected.entry.prompt);
    println!("match_score: {:.6}", selected.total_score);
    let candidate_margin = candidate_margin(&lut, &prompt, prompt_wave, &settled);
    println!("candidate_margin: {:.6}", candidate_margin);
    println!("score_prompt_component: {:.6}", selected.prompt_score);
    println!("score_lexical_component: {:.6}", selected.lexical_score);
    println!("score_wave_component: {:.6}", selected.wave_score);
    println!("score_stability_component: {:.6}", selected.stability_score);
    println!("wave_matched_prompt: {}", wave_selected.entry.prompt);
    println!("wave_score: {:.6}", wave_selected.wave_score);
    let response_gate = response_gate(&prompt, &settled, &selected, candidate_margin);
    println!("response_gate: {}", response_gate.as_str());
    println!("answer: {}", selected.entry.answer);
    if matches!(response_gate, ResponseGate::Refuse) {
        println!(
            "gated_answer: не отвечаю: внутреннее состояние не дало надежной когерентной опоры."
        );
    } else {
        println!("gated_answer: {}", selected.entry.answer);
    }
    println!("controls:");
    println!(
        "no_carrier: matched_prompt=\"{}\" score={:.6} coherence={:.6} entropy={:.6} memory_validation={:.6} verdict={}",
        no_selected.entry.prompt,
        no_selected.total_score,
        no_carrier.coherence,
        no_carrier.entropy,
        no_carrier.memory.validation,
        no_carrier.verdict().as_str()
    );
    println!(
        "wrong_carrier: matched_prompt=\"{}\" score={:.6} coherence={:.6} entropy={:.6} memory_validation={:.6} verdict={}",
        wrong_selected.entry.prompt,
        wrong_selected.total_score,
        wrong_carrier.coherence,
        wrong_carrier.entropy,
        wrong_carrier.memory.validation,
        wrong_carrier.verdict().as_str()
    );
    println!(
        "corrupted_prompt_wave: matched_prompt=\"{}\" score={:.6} coherence={:.6} entropy={:.6} memory_validation={:.6} verdict={}",
        corrupt_selected.entry.prompt,
        corrupt_selected.total_score,
        corrupted.coherence,
        corrupted.entropy,
        corrupted.memory.validation,
        corrupted.verdict().as_str()
    );
    let no_carrier_score_delta = selected.total_score - no_selected.total_score;
    let wrong_carrier_score_delta = selected.total_score - wrong_selected.total_score;
    let corrupted_score_delta = selected.total_score - corrupt_selected.total_score;
    println!("score_delta_over_no_carrier: {:.6}", no_carrier_score_delta);
    println!(
        "score_delta_over_wrong_carrier: {:.6}",
        wrong_carrier_score_delta
    );
    println!(
        "score_delta_over_corrupted_prompt: {:.6}",
        corrupted_score_delta
    );

    let trace_is_carrier_sensitive = settled.coherence > no_carrier.coherence + 0.05
        || (settled.coherence - wrong_carrier.coherence).abs() > 0.05
        || (settled.coherence - corrupted.coherence).abs() > 0.05;
    let answer_is_wave_sensitive = selected.entry.prompt != no_selected.entry.prompt
        || selected.entry.prompt != wrong_selected.entry.prompt
        || selected.entry.prompt != corrupt_selected.entry.prompt
        || (no_carrier_score_delta > 0.02
            && wrong_carrier_score_delta > 0.02
            && corrupted_score_delta > 0.02);
    let wave_and_final_agree = selected.entry.prompt == wave_selected.entry.prompt;
    let lexical_dominates = selected.lexical_score.abs() + selected.prompt_score.abs()
        > selected.wave_score.abs() * 2.0;
    let mode_status = if answer_is_wave_sensitive && wave_and_final_agree && !lexical_dominates {
        "organ128_settle_dialog_answer_wave_dominant"
    } else if answer_is_wave_sensitive {
        "organ128_settle_dialog_answer_wave_sensitive"
    } else if trace_is_carrier_sensitive {
        "organ128_settle_dialog_trace_carrier_sensitive_matching_dominates"
    } else {
        "organ128_settle_dialog_trace_only"
    };
    println!("mode_status: {mode_status}");
    Ok(())
}

pub(crate) fn run_organ128_wave_scorer_eval(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let epochs = match args.next() {
        Some(value) => parse_usize(&value, "epochs")?,
        None => 12,
    };
    let ticks = match args.next() {
        Some(value) => parse_usize(&value, "ticks")?,
        None => 5,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if epochs == 0 {
        return Err(String::from("epochs must be greater than zero"));
    }
    if !(1..=16).contains(&ticks) {
        return Err(String::from("ticks must be in 1..=16"));
    }

    let lut = BytePhaseLut::new();
    let organ = Organ128Runtime::new(seed);
    let train_indices = dialog_train_indices();
    let holdout_indices = dialog_holdout_indices();
    let trained = train_wave_dialog_scorer(
        &lut,
        &organ,
        seed,
        epochs,
        ticks,
        &train_indices,
        FeatureMask::full(),
    );
    let holdout =
        eval_wave_dialog_scorer(&lut, &organ, &trained.scorer, seed, ticks, &holdout_indices);
    let baseline = eval_untrained_wave_dialog_scorer(&lut, &organ, seed, ticks, &holdout_indices);
    let ablations = [
        ("full", FeatureMask::full()),
        ("no_center", FeatureMask::without_center()),
        ("no_memory", FeatureMask::without_memory()),
        ("no_prompt", FeatureMask::without_prompt()),
        ("no_global", FeatureMask::without_global()),
        ("no_validation", FeatureMask::without_validation()),
    ];

    println!("Nando Wave Organ128 wave-scorer eval");
    println!("seed: {seed}");
    println!("epochs: {epochs}");
    println!("ticks: {ticks}");
    println!("train_items: {}", train_indices.len());
    println!("holdout_items: {}", holdout_indices.len());
    println!("train_cases: {}", trained.train_cases);
    println!(
        "train_accuracy_before_update: {:.6}",
        trained.train_accuracy_before_update
    );
    println!("holdout_accuracy: {:.6}", holdout.accuracy);
    println!("baseline_holdout_accuracy: {:.6}", baseline.accuracy);
    println!("holdout_gain: {:.6}", holdout.accuracy - baseline.accuracy);
    println!("holdout_wave_agree: {:.6}", holdout.wave_agree);
    println!("weight_abs_mean: {:.6}", trained.scorer.weight_abs_mean());
    println!("ablation:");
    for (name, mask) in ablations {
        let ablated =
            train_wave_dialog_scorer(&lut, &organ, seed, epochs, ticks, &train_indices, mask);
        let report =
            eval_wave_dialog_scorer(&lut, &organ, &ablated.scorer, seed, ticks, &holdout_indices);
        println!(
            "{name}: holdout_accuracy={:.6} holdout_gain={:.6} weight_abs_mean={:.6}",
            report.accuracy,
            report.accuracy - baseline.accuracy,
            ablated.scorer.weight_abs_mean()
        );
    }
    println!("holdout_trace:");
    for row in &holdout.rows {
        println!(
            "{} -> predicted=\"{}\" target=\"{}\" score={:.6}",
            row.prompt, row.predicted_prompt, row.target_prompt, row.score
        );
    }
    let mode_status = if holdout.accuracy > baseline.accuracy && holdout.accuracy >= 0.35 {
        "organ128_wave_scorer_candidate"
    } else {
        "not_found_organ128_wave_scorer"
    };
    println!("mode_status: {mode_status}");
    Ok(())
}

pub(crate) fn run_organ128_response_gate_eval(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let ticks = match args.next() {
        Some(value) => parse_usize(&value, "ticks")?,
        None => 12,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if !(1..=16).contains(&ticks) {
        return Err(String::from("ticks must be in 1..=16"));
    }

    let lut = BytePhaseLut::new();
    let organ = Organ128Runtime::new(seed);
    let known = eval_response_gate_known(&lut, &organ, seed, ticks);
    let refusal = eval_response_gate_refusal(&lut, &organ, seed, ticks);

    println!("Nando Wave Organ128 response-gate eval");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    println!("known_cases: {}", known.cases);
    println!("known_answered: {}", known.answered);
    println!("known_refused: {}", known.refused);
    println!("known_answer_rate: {:.6}", known.answer_rate());
    println!("refusal_cases: {}", refusal.cases);
    println!("refusal_answered: {}", refusal.answered);
    println!("refusal_refused: {}", refusal.refused);
    println!("refusal_refuse_rate: {:.6}", refusal.refuse_rate());
    println!("trace:");
    for row in known.rows.iter().chain(refusal.rows.iter()) {
        println!(
            "{} prompt=\"{}\" verdict={} gate={} score={:.6} margin={:.6} lexical={:.6} wave={:.6}",
            row.kind,
            row.prompt,
            row.verdict.as_str(),
            row.gate.as_str(),
            row.score,
            row.margin,
            row.lexical_score,
            row.wave_score
        );
    }

    let mode_status = if known.answer_rate() >= 0.70 && refusal.refuse_rate() >= 0.80 {
        "organ128_response_gate_candidate"
    } else {
        "not_found_organ128_response_gate"
    };
    println!("mode_status: {mode_status}");
    Ok(())
}

pub(crate) fn run_organ128_thought_probe_eval(
    mut args: impl Iterator<Item = String>,
) -> Result<(), String> {
    let seed = match args.next() {
        Some(value) => parse_u64(&value, "seed")?,
        None => 7,
    };
    let ticks = match args.next() {
        Some(value) => parse_usize(&value, "ticks")?,
        None => 12,
    };
    let epochs = match args.next() {
        Some(value) => parse_usize(&value, "epochs")?,
        None => 24,
    };
    if args.next().is_some() {
        return Err(String::from("too many arguments"));
    }
    if !(1..=16).contains(&ticks) {
        return Err(String::from("ticks must be in 1..=16"));
    }
    if epochs == 0 {
        return Err(String::from("epochs must be greater than zero"));
    }

    let lut = BytePhaseLut::new();
    let organ = Organ128Runtime::new(seed);
    let mut probe = ThoughtProbe::new(0.10);
    let mut train_cases = 0usize;
    let mut train_correct_before = 0usize;
    for epoch in 0..epochs {
        for (sample_index, sample) in thought_probe_train_samples().into_iter().enumerate() {
            let train_seed = seed
                .wrapping_add((epoch as u64) << 12)
                .wrapping_add(sample_index as u64);
            let settled = settle_thought_probe_sample(&lut, &organ, train_seed, ticks, sample);
            let predicted = probe.predict(&settled);
            if predicted == sample.should_answer {
                train_correct_before += 1;
            }
            probe.update(&settled, sample.should_answer);
            train_cases += 1;
        }
    }

    let holdout = eval_thought_probe(
        &lut,
        &organ,
        &probe,
        seed,
        ticks,
        thought_probe_holdout_samples(),
    );
    println!("Nando Wave Organ128 thought-probe eval");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    println!("epochs: {epochs}");
    println!("train_cases: {train_cases}");
    println!(
        "train_accuracy_before_update: {:.6}",
        train_correct_before as f32 / train_cases.max(1) as f32
    );
    println!("holdout_cases: {}", holdout.cases);
    println!("holdout_accuracy: {:.6}", holdout.accuracy());
    println!("known_answer_rate: {:.6}", holdout.known_answer_rate());
    println!("refusal_refuse_rate: {:.6}", holdout.refusal_refuse_rate());
    println!("weight_abs_mean: {:.6}", probe.weight_abs_mean());
    println!("trace:");
    for row in &holdout.rows {
        println!(
            "{} prompt=\"{}\" target={} predicted={} score={:.6} thought={} strength={:.6} convergence={:.6} drift={:.6} prompt_specificity={:.6} specificity={:.6} role_balance={:.6} memory_alignment={:.6}",
            row.kind,
            row.prompt,
            if row.should_answer {
                "answer"
            } else {
                "refuse"
            },
            if row.predicted_answer {
                "answer"
            } else {
                "refuse"
            },
            row.score,
            row.thought_verdict.as_str(),
            row.strength,
            row.convergence,
            row.drift,
            row.prompt_specificity,
            row.specificity,
            row.role_balance,
            row.memory_alignment
        );
    }
    let mode_status =
        if holdout.known_answer_rate() >= 0.70 && holdout.refusal_refuse_rate() >= 0.70 {
            "organ128_thought_probe_candidate"
        } else {
            "not_found_organ128_thought_probe"
        };
    println!("mode_status: {mode_status}");
    Ok(())
}

#[derive(Clone)]
struct Organ128Runtime {
    cells: Vec<Cell32>,
}

impl Organ128Runtime {
    fn new(seed: u64) -> Self {
        let plan = CacheAwareOrganPlan::t480_organ128().organ128;
        let mut cells = Vec::with_capacity(plan.cell_count());

        for id in 0..plan.fast_cells {
            cells.push(Cell32::new(id as u32, CellRank::Fast, seed));
        }
        for offset in 0..plan.mid_cells {
            let id = plan.fast_cells + offset;
            cells.push(Cell32::new(id as u32, CellRank::Mid, seed));
        }
        for offset in 0..plan.guard_cells {
            let id = plan.fast_cells + plan.mid_cells + offset;
            cells.push(Cell32::new(id as u32, CellRank::Guard, seed));
        }
        for offset in 0..plan.carrier_cells {
            let id = plan.fast_cells + plan.mid_cells + plan.guard_cells + offset;
            cells.push(Cell32::new(id as u32, CellRank::CarrierAnchor, seed));
        }
        for offset in 0..plan.memory_cells {
            let id =
                plan.fast_cells + plan.mid_cells + plan.guard_cells + plan.carrier_cells + offset;
            cells.push(Cell32::new(id as u32, CellRank::Mid, seed ^ 0xA11C_E128));
        }

        Self { cells }
    }

    fn active_cells(
        &self,
        lut: &BytePhaseLut,
        seed: u64,
        input_byte: u8,
        prompt_wave: PromptWave,
        step: usize,
    ) -> [(usize, f32); 4] {
        let carrier = CarrierWave::from_seed(seed.wrapping_add(step as u64), input_byte);
        self.active_cells_with_carrier(lut, input_byte, prompt_wave, carrier)
    }

    fn active_cells_with_carrier(
        &self,
        lut: &BytePhaseLut,
        input_byte: u8,
        prompt_wave: PromptWave,
        carrier: CarrierWave,
    ) -> [(usize, f32); 4] {
        let mut input_sin = [0.0; PHASE_SLOTS];
        let mut input_cos = [0.0; PHASE_SLOTS];
        for slot in 0..PHASE_SLOTS {
            let prompt_slot_phase = prompt_wave.slot_phase(slot);
            let phase = lut.phases(input_byte)[slot]
                + carrier.phase
                + prompt_wave.phase * 0.18
                + prompt_slot_phase * 0.08;
            input_sin[slot] = phase.sin();
            input_cos[slot] = phase.cos();
        }

        let envelope = carrier.envelope();
        let mut top = [(usize::MAX, f32::NEG_INFINITY); 4];
        for (cell_index, cell) in self.cells.iter().enumerate() {
            let mut score =
                cell.resonance_score_with_carrier_trig(&input_sin, &input_cos, envelope);
            score += prompt_wave.cell_bias(cell_index) * 0.04;
            insert_top_cell(cell_index, score, &mut top);
        }

        top.map(|(index, score)| (index, score.max(0.0)))
    }

    fn settle_dialog(
        &self,
        lut: &BytePhaseLut,
        seed: u64,
        prompt: &str,
        prompt_wave: PromptWave,
        ticks: usize,
        mode: CarrierMode,
    ) -> Organ128SettleState {
        let bytes = prompt.as_bytes();
        let mut wave = match mode {
            CarrierMode::CorruptedPrompt => prompt_wave.corrupted(),
            CarrierMode::Correct | CarrierMode::None | CarrierMode::Wrong => prompt_wave,
        };
        let first_byte = bytes.first().copied().unwrap_or(b' ');
        let mut carrier = carrier_from_prompt_wave(seed, first_byte, wave);
        match mode {
            CarrierMode::Correct | CarrierMode::CorruptedPrompt => {}
            CarrierMode::None => {
                carrier.amplitude = 0.0;
                carrier.boundary = 0.0;
            }
            CarrierMode::Wrong => {
                carrier.phase =
                    (carrier.phase + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU);
                carrier.amplitude = (carrier.amplitude * 0.35).clamp(0.0, 1.0);
            }
        }

        let mut previous_center = None;
        let mut trace = Vec::with_capacity(ticks);
        let mut final_bus = WaveBus::default();
        let mut memory = Organ128MemoryState::from_prompt_wave(prompt_wave);
        for tick in 0..ticks {
            let input_byte = bytes
                .get(tick % bytes.len().max(1))
                .copied()
                .unwrap_or(b' ');
            carrier = carrier.advance(input_byte, 1);
            let memory_validation = memory.validation(wave);
            let memory_pull = memory.pull_phase();
            carrier.phase = (carrier.phase
                + circular_delta_local(carrier.phase, memory_pull.phase)
                    * memory_pull.strength
                    * memory_validation
                    * 0.08)
                .rem_euclid(std::f32::consts::TAU);
            carrier.amplitude = (carrier.amplitude * 0.94
                + memory_pull.strength * memory_validation * 0.06)
                .clamp(0.0, 1.0);
            carrier.phase = (carrier.phase
                + circular_delta_local(carrier.phase, wave.phase) * 0.14)
                .rem_euclid(std::f32::consts::TAU);
            carrier.amplitude = (carrier.amplitude * 0.86 + wave.amplitude * 0.14).clamp(0.0, 1.0);
            if matches!(mode, CarrierMode::None) {
                carrier.amplitude = 0.0;
                carrier.boundary = 0.0;
            }

            let hot = self.hot_cells_with_carrier(lut, input_byte, wave, carrier);
            let active = quota_active_from_hot(&hot);
            let l2_roles = RoleSummary::from_cells(&hot);
            let l1_roles = RoleSummary::from_cells(&active);
            let mut bus = WaveBus::default();
            for (cell_index, _) in active {
                if let Some(cell) = self.cells.get(cell_index) {
                    bus.add_cell(cell, carrier);
                }
            }
            bus.finish_metrics();

            let phase_velocity = previous_center
                .map(|phase| circular_delta_local(phase, bus.center_phase).abs())
                .unwrap_or(0.0);
            previous_center = Some(bus.center_phase);
            carrier.phase = (carrier.phase
                + circular_delta_local(carrier.phase, bus.center_phase) * 0.10)
                .rem_euclid(std::f32::consts::TAU);
            wave = wave.with_phase_pull(bus.center_phase, 0.03);
            memory.update_from_bus(
                tick,
                &active,
                bus.center_phase,
                bus.coherence,
                memory_validation,
            );
            let memory_snapshot = memory.summary();

            trace.push(Organ128SettleTick {
                tick,
                input_byte,
                carrier_phase: carrier.phase,
                center_phase: bus.center_phase,
                coherence: bus.coherence,
                entropy: bus.spectral_entropy,
                phase_velocity,
                l2_roles,
                l1_roles,
                memory: MemorySummary {
                    validation: memory_validation,
                    ..memory_snapshot
                },
                active_cell_ids: active.map(|(cell, _)| cell),
            });
            final_bus = bus;
        }

        let stability = if trace.len() <= 1 {
            final_bus.coherence
        } else {
            let mean_velocity = trace
                .iter()
                .skip(1)
                .map(|tick| tick.phase_velocity)
                .sum::<f32>()
                / (trace.len() - 1) as f32;
            (1.0 - mean_velocity / std::f32::consts::PI).clamp(0.0, 1.0)
        };

        let final_memory = memory.summary();
        let thought = ThoughtState::from_trace(&trace, prompt_wave, &final_memory);
        Organ128SettleState {
            ticks: trace,
            center_phase: final_bus.center_phase,
            coherence: final_bus.coherence,
            entropy: final_bus.spectral_entropy,
            stability,
            thought,
            memory: MemorySummary {
                validation: memory.validation(wave),
                ..final_memory
            },
        }
    }

    fn hot_cells_with_carrier(
        &self,
        lut: &BytePhaseLut,
        input_byte: u8,
        prompt_wave: PromptWave,
        carrier: CarrierWave,
    ) -> [(usize, f32); 32] {
        let mut input_sin = [0.0; PHASE_SLOTS];
        let mut input_cos = [0.0; PHASE_SLOTS];
        for slot in 0..PHASE_SLOTS {
            let prompt_slot_phase = prompt_wave.slot_phase(slot);
            let phase = lut.phases(input_byte)[slot]
                + carrier.phase
                + prompt_wave.phase * 0.18
                + prompt_slot_phase * 0.08;
            input_sin[slot] = phase.sin();
            input_cos[slot] = phase.cos();
        }

        let envelope = carrier.envelope();
        let mut top = [(usize::MAX, f32::NEG_INFINITY); 32];
        for (cell_index, cell) in self.cells.iter().enumerate() {
            let mut score =
                cell.resonance_score_with_carrier_trig(&input_sin, &input_cos, envelope);
            score += prompt_wave.cell_bias(cell_index) * 0.04;
            insert_top_cell(cell_index, score, &mut top);
        }

        top.map(|(index, score)| (index, score.max(0.0)))
    }
}

#[derive(Debug, Clone, Copy)]
enum CarrierMode {
    Correct,
    None,
    Wrong,
    CorruptedPrompt,
}

#[derive(Debug, Clone)]
struct Organ128SettleTick {
    tick: usize,
    input_byte: u8,
    carrier_phase: f32,
    center_phase: f32,
    coherence: f32,
    entropy: f32,
    phase_velocity: f32,
    l2_roles: RoleSummary,
    l1_roles: RoleSummary,
    memory: MemorySummary,
    active_cell_ids: [usize; 4],
}

#[derive(Debug, Clone, Copy)]
struct MemorySummary {
    phase: f32,
    strength: f32,
    validation: f32,
    top_slots: [usize; PROMPT_WAVE_TOP_SLOTS],
}

#[derive(Debug, Clone, Copy)]
struct RoleSummary {
    fast: usize,
    mid: usize,
    guard: usize,
    carrier: usize,
    memory: usize,
}

impl RoleSummary {
    fn from_cells<const N: usize>(cells: &[(usize, f32); N]) -> Self {
        let mut summary = Self {
            fast: 0,
            mid: 0,
            guard: 0,
            carrier: 0,
            memory: 0,
        };
        for (cell_id, _) in cells.iter().copied() {
            match organ128_cell_role(cell_id) {
                Organ128CellRole::Fast => summary.fast += 1,
                Organ128CellRole::Mid => summary.mid += 1,
                Organ128CellRole::Guard => summary.guard += 1,
                Organ128CellRole::Carrier => summary.carrier += 1,
                Organ128CellRole::Memory => summary.memory += 1,
            }
        }
        summary
    }

    fn to_compact_text(self) -> String {
        format!(
            "F{} M{} G{} C{} R{}",
            self.fast, self.mid, self.guard, self.carrier, self.memory
        )
    }
}

#[derive(Debug, Clone, Copy)]
enum Organ128CellRole {
    Fast,
    Mid,
    Guard,
    Carrier,
    Memory,
}

#[derive(Debug, Clone)]
struct Organ128MemoryState {
    anchor: PromptWave,
    phases: [f32; 8],
    strengths: [f32; 8],
    top_slots: [usize; PROMPT_WAVE_TOP_SLOTS],
    slot_energy: [f32; PHASE_SLOTS],
}

impl Organ128MemoryState {
    fn from_prompt_wave(prompt_wave: PromptWave) -> Self {
        let mut phases = [0.0; 8];
        let mut strengths = [0.0; 8];
        for slot in 0..8 {
            let offset = slot as f32 / 8.0 * std::f32::consts::TAU;
            phases[slot] = (prompt_wave.phase + offset * 0.125).rem_euclid(std::f32::consts::TAU);
            strengths[slot] = prompt_wave.amplitude * 0.20;
        }
        let mut slot_energy = [0.0f32; PHASE_SLOTS];
        for (rank, slot) in prompt_wave.top_slots.iter().copied().enumerate() {
            slot_energy[slot] =
                (PROMPT_WAVE_TOP_SLOTS - rank) as f32 / PROMPT_WAVE_TOP_SLOTS as f32;
        }
        Self {
            anchor: prompt_wave,
            phases,
            strengths,
            top_slots: prompt_wave.top_slots,
            slot_energy,
        }
    }

    fn pull_phase(&self) -> MemorySummary {
        self.summary()
    }

    fn validation(&self, wave: PromptWave) -> f32 {
        let phase_match = ((self.anchor.phase - wave.phase).cos() + 1.0) * 0.5;
        let amplitude_match = 1.0 - (self.anchor.amplitude - wave.amplitude).abs();
        let mut positional_slot_match = 0.0f32;
        for (index, slot) in self.anchor.top_slots.iter().copied().enumerate() {
            if wave.top_slots[index] == slot {
                positional_slot_match += 1.0;
            }
        }
        positional_slot_match /= PROMPT_WAVE_TOP_SLOTS as f32;
        (phase_match * 0.45 + amplitude_match * 0.15 + positional_slot_match * 0.40).clamp(0.0, 1.0)
    }

    fn update_from_bus(
        &mut self,
        tick: usize,
        active: &[(usize, f32); 4],
        center_phase: f32,
        coherence: f32,
        validation: f32,
    ) {
        for strength in &mut self.strengths {
            *strength *= 0.92;
        }

        let active_memory_slot = active.iter().find_map(|(cell_id, _)| {
            if matches!(organ128_cell_role(*cell_id), Organ128CellRole::Memory) {
                Some(cell_id.saturating_sub(120).min(7))
            } else {
                None
            }
        });
        let slot = active_memory_slot.unwrap_or(tick % self.phases.len());
        let write_strength = ((0.12 + coherence * 0.28) * validation).clamp(0.0, 1.0);
        self.phases[slot] = (self.phases[slot]
            + circular_delta_local(self.phases[slot], center_phase) * write_strength)
            .rem_euclid(std::f32::consts::TAU);
        self.strengths[slot] = (self.strengths[slot] + write_strength * 0.35).clamp(0.0, 1.0);
        if validation > 0.80 {
            for (cell_id, resonance) in active.iter().copied() {
                let cell_slot = cell_id % PHASE_SLOTS;
                self.slot_energy[cell_slot] += resonance.max(0.0) * write_strength;
            }
            let center_slot = phase_to_slot(center_phase);
            self.slot_energy[center_slot] += coherence * write_strength * 0.25;
            self.refresh_top_slots();
        }
    }

    fn refresh_top_slots(&mut self) {
        let mut top_slots = [usize::MAX; PROMPT_WAVE_TOP_SLOTS];
        let mut top_scores = [f32::NEG_INFINITY; PROMPT_WAVE_TOP_SLOTS];
        for slot in self.anchor.top_slots.iter().copied() {
            insert_unique_top_slot(
                slot,
                self.slot_energy[slot] + 0.20,
                &mut top_slots,
                &mut top_scores,
            );
        }
        for (slot, energy) in self.slot_energy.iter().copied().enumerate() {
            insert_unique_top_slot(slot, energy, &mut top_slots, &mut top_scores);
        }
        self.top_slots = top_slots;
    }

    fn summary(&self) -> MemorySummary {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut total = 0.0f32;
        for (phase, strength) in self.phases.iter().zip(self.strengths.iter()) {
            x += strength * phase.cos();
            y += strength * phase.sin();
            total += strength;
        }
        if total <= f32::EPSILON {
            return MemorySummary {
                phase: 0.0,
                strength: 0.0,
                validation: 0.0,
                top_slots: self.top_slots,
            };
        }
        let magnitude = (x.mul_add(x, y * y)).sqrt();
        MemorySummary {
            phase: y.atan2(x).rem_euclid(std::f32::consts::TAU),
            strength: (magnitude / total).clamp(0.0, 1.0),
            validation: 1.0,
            top_slots: self.top_slots,
        }
    }
}

#[derive(Debug, Clone)]
struct Organ128SettleState {
    ticks: Vec<Organ128SettleTick>,
    center_phase: f32,
    coherence: f32,
    entropy: f32,
    stability: f32,
    thought: ThoughtState,
    memory: MemorySummary,
}

#[derive(Debug, Clone, Copy)]
struct ThoughtState {
    phase: f32,
    strength: f32,
    convergence: f32,
    drift: f32,
    prompt_specificity: f32,
    specificity: f32,
    role_balance: f32,
    memory_alignment: f32,
}

impl ThoughtState {
    fn from_trace(
        trace: &[Organ128SettleTick],
        prompt_wave: PromptWave,
        memory: &MemorySummary,
    ) -> Self {
        if trace.is_empty() {
            return Self {
                phase: prompt_wave.phase,
                strength: 0.0,
                convergence: 0.0,
                drift: 0.0,
                prompt_specificity: 0.0,
                specificity: 0.0,
                role_balance: 0.0,
                memory_alignment: 0.0,
            };
        }

        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut total_weight = 0.0f32;
        let mut velocity_sum = 0.0f32;
        for tick in trace {
            let confidence =
                (tick.coherence * (1.0 - tick.entropy).clamp(0.0, 1.0)).max(tick.coherence * 0.20);
            x += confidence * tick.center_phase.cos();
            y += confidence * tick.center_phase.sin();
            total_weight += confidence;
            velocity_sum += tick.phase_velocity;
        }

        let phase = y.atan2(x).rem_euclid(std::f32::consts::TAU);
        let strength = (x.mul_add(x, y * y)).sqrt() / total_weight.max(f32::EPSILON);
        let mean_velocity = velocity_sum / trace.len().max(1) as f32;
        let convergence = (1.0 - mean_velocity / std::f32::consts::PI).clamp(0.0, 1.0);
        let drift = circular_delta_local(prompt_wave.phase, phase).abs() / std::f32::consts::PI;
        let memory_alignment = ((memory.phase - phase).cos() + 1.0) * 0.5 * memory.validation;
        let prompt_specificity = prompt_wave.spectral_specificity();
        let specificity = thought_cell_specificity(trace);
        let role_balance = thought_role_balance(trace);

        Self {
            phase,
            strength: strength.clamp(0.0, 1.0),
            convergence,
            drift: drift.clamp(0.0, 1.0),
            prompt_specificity,
            specificity,
            role_balance,
            memory_alignment: memory_alignment.clamp(0.0, 1.0),
        }
    }

    fn verdict(self) -> ThoughtVerdict {
        if self.strength < 0.20 || self.convergence < 0.20 {
            return ThoughtVerdict::Diffuse;
        }
        if self.drift > 0.75 && self.memory_alignment < 0.35 {
            return ThoughtVerdict::Detached;
        }
        if self.convergence >= 0.50 && self.memory_alignment >= 0.45 {
            return ThoughtVerdict::Coherent;
        }
        ThoughtVerdict::Unsettled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThoughtVerdict {
    Coherent,
    Unsettled,
    Diffuse,
    Detached,
}

impl ThoughtVerdict {
    fn as_str(self) -> &'static str {
        match self {
            Self::Coherent => "coherent",
            Self::Unsettled => "unsettled",
            Self::Diffuse => "diffuse",
            Self::Detached => "detached",
        }
    }
}

fn thought_cell_specificity(trace: &[Organ128SettleTick]) -> f32 {
    let mut counts = [0u16; 128];
    let mut total = 0u16;
    for tick in trace {
        for cell_id in tick.active_cell_ids {
            if cell_id < counts.len() {
                counts[cell_id] = counts[cell_id].saturating_add(1);
                total = total.saturating_add(1);
            }
        }
    }
    if total == 0 {
        return 0.0;
    }

    let mut energy = 0.0f32;
    for count in counts {
        if count == 0 {
            continue;
        }
        let probability = count as f32 / total as f32;
        energy += probability * probability;
    }

    let concentration = ((energy * total as f32) - 1.0) / (total as f32 - 1.0).max(1.0);
    (1.0 - (concentration - 0.35).abs() / 0.35).clamp(0.0, 1.0)
}

fn thought_role_balance(trace: &[Organ128SettleTick]) -> f32 {
    let mut fast = 0usize;
    let mut mid_guard = 0usize;
    let mut carrier_memory = 0usize;
    for tick in trace {
        fast += tick.l1_roles.fast;
        mid_guard += tick.l1_roles.mid + tick.l1_roles.guard;
        carrier_memory += tick.l1_roles.carrier + tick.l1_roles.memory;
    }
    let total = (fast + mid_guard + carrier_memory).max(1) as f32;
    let fast_p = fast as f32 / total;
    let mid_p = mid_guard as f32 / total;
    let slow_p = carrier_memory as f32 / total;
    let expected_fast = 0.50f32;
    let expected_mid = 0.25f32;
    let expected_slow = 0.25f32;
    let distance = (fast_p - expected_fast).abs()
        + (mid_p - expected_mid).abs()
        + (slow_p - expected_slow).abs();
    (1.0 - distance / 1.5).clamp(0.0, 1.0)
}

impl Organ128SettleState {
    fn verdict(&self) -> SettleVerdict {
        if self.memory.validation < 0.35 {
            return SettleVerdict::RejectedByMemory;
        }
        if self.coherence < 0.08 && self.entropy > 0.995 {
            return SettleVerdict::Incoherent;
        }
        if self.mean_phase_velocity() > 1.20 && self.stability < 0.45 {
            return SettleVerdict::Oscillating;
        }
        if self.coherence >= 0.24 && self.stability >= 0.25 && self.memory.validation >= 0.70 {
            return SettleVerdict::Settled;
        }
        SettleVerdict::Weak
    }

    fn mean_phase_velocity(&self) -> f32 {
        if self.ticks.len() <= 1 {
            return 0.0;
        }
        self.ticks
            .iter()
            .skip(1)
            .map(|tick| tick.phase_velocity)
            .sum::<f32>()
            / (self.ticks.len() - 1) as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettleVerdict {
    Settled,
    Weak,
    Oscillating,
    Incoherent,
    RejectedByMemory,
}

impl SettleVerdict {
    fn as_str(self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::Weak => "weak",
            Self::Oscillating => "oscillating",
            Self::Incoherent => "incoherent",
            Self::RejectedByMemory => "rejected_by_memory",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseGate {
    Answer,
    Refuse,
}

impl ResponseGate {
    fn as_str(self) -> &'static str {
        match self {
            Self::Answer => "answer",
            Self::Refuse => "refuse_unstable_or_low_confidence",
        }
    }
}

fn response_gate(
    prompt: &str,
    settled: &Organ128SettleState,
    selected: &DialogScore,
    candidate_margin: f32,
) -> ResponseGate {
    match settled.verdict() {
        SettleVerdict::RejectedByMemory | SettleVerdict::Incoherent => return ResponseGate::Refuse,
        SettleVerdict::Settled | SettleVerdict::Weak | SettleVerdict::Oscillating => {}
    }

    let prompt_is_exact = normalized_prompt_eq(prompt, selected.entry.prompt);
    let selected_prompt_coverage = selected_prompt_coverage(prompt, selected.entry.prompt);
    let has_specific_prompt = prompt_is_exact || selected_prompt_coverage >= 0.72;
    let has_clear_winner = prompt_is_exact || candidate_margin >= 0.08;
    let has_direct_support =
        has_specific_prompt && (selected.lexical_score >= 0.20 || selected.prompt_score >= 0.35);
    let has_wave_support =
        has_specific_prompt && selected.wave_score >= 0.03 && settled.coherence >= 0.20;
    let strong_total = selected.total_score >= 0.50;
    let oscillating = matches!(settled.verdict(), SettleVerdict::Oscillating);

    if strong_total
        && has_clear_winner
        && (has_direct_support || has_wave_support)
        && (!oscillating || selected.total_score >= 0.70)
    {
        ResponseGate::Answer
    } else {
        ResponseGate::Refuse
    }
}

fn normalized_prompt_eq(left: &str, right: &str) -> bool {
    left.split_whitespace().eq(right.split_whitespace())
}

fn selected_prompt_coverage(prompt: &str, selected_prompt: &str) -> f32 {
    let mut selected_tokens = 0usize;
    let mut covered_tokens = 0usize;
    for selected_token in selected_prompt.split_whitespace() {
        selected_tokens += 1;
        if prompt
            .split_whitespace()
            .any(|token| token == selected_token)
        {
            covered_tokens += 1;
        }
    }
    covered_tokens as f32 / selected_tokens.max(1) as f32
}

#[derive(Debug, Clone)]
struct Organ128ByteLearner {
    learning_rate: f32,
    byte_bias: [f32; 256],
    transition_weights: [[f32; 256]; 256],
    cell_byte_weights: Vec<[f32; 256]>,
}

#[derive(Debug, Clone, Copy)]
struct Organ128TrainReport {
    cases: usize,
    accuracy: f32,
}

#[derive(Debug, Clone, Copy)]
struct DialogEntry {
    prompt: &'static str,
    answer: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct DialogScore {
    entry: DialogEntry,
    total_score: f32,
    prompt_score: f32,
    lexical_score: f32,
    wave_score: f32,
    stability_score: f32,
}

const DIALOG_CORPUS: [DialogEntry; 33] = [
    DialogEntry {
        prompt: "what is nando",
        answer: "nando is a small wave organ made from cell32 packets.",
    },
    DialogEntry {
        prompt: "what is organ128",
        answer: "organ128 is one hundred twenty eight cell32 resonators in a cache aware body.",
    },
    DialogEntry {
        prompt: "write about rust",
        answer: "rust keeps the cells fast and the runtime predictable.",
    },
    DialogEntry {
        prompt: "what is prompt wave",
        answer: "a prompt wave is a compact phase state made from the whole prompt.",
    },
    DialogEntry {
        prompt: "what is carrier wave",
        answer: "the carrier wave is the slow context mode that bounds lower cells.",
    },
    DialogEntry {
        prompt: "what is wave bus",
        answer: "the wave bus is where cell phases interfere and form a center.",
    },
    DialogEntry {
        prompt: "what is snapshot",
        answer: "a snapshot is a small frequency chord of the current state.",
    },
    DialogEntry {
        prompt: "what do you know",
        answer: "i know about nando, organ128, cell32, prompt waves, carrier waves, wave bus, snapshots, rust, and local learning.",
    },
    DialogEntry {
        prompt: "how should nando learn",
        answer: "nando should learn through local feedback, holdout gates, ablation, and snapshots.",
    },
    DialogEntry {
        prompt: "why ablation",
        answer: "ablation proves whether a mode is real or just a lucky match.",
    },
    DialogEntry {
        prompt: "why mono192",
        answer: "mono192 is the fair control for the same memory budget.",
    },
    DialogEntry {
        prompt: "hello nando",
        answer: "hello. the organ is small but alive.",
    },
    DialogEntry {
        prompt: "что такое nando",
        answer: "nando это маленький волновой организм из клеток cell32.",
    },
    DialogEntry {
        prompt: "что такое organ128",
        answer: "organ128 это 128 клеток cell32 в кэшированном теле.",
    },
    DialogEntry {
        prompt: "что такое волна промпта",
        answer: "волна промпта это фазовое состояние всего запроса.",
    },
    DialogEntry {
        prompt: "что такое cell32",
        answer: "cell32 это горячая клетка на 32 килобайта с фазами, амплитудами и резонансом.",
    },
    DialogEntry {
        prompt: "что такое expert64",
        answer: "expert64 это пара cell32: быстрый детектор и стабилизатор.",
    },
    DialogEntry {
        prompt: "что такое mono192",
        answer: "mono192 это монолитный контроль того же размера, с которым надо сравнивать организм.",
    },
    DialogEntry {
        prompt: "что такое несущая волна",
        answer: "несущая волна это медленная мода контекста, которая ограничивает дрейф клеток.",
    },
    DialogEntry {
        prompt: "что такое wave bus",
        answer: "wave bus это место интерференции, где клетки дают общий центр фазы.",
    },
    DialogEntry {
        prompt: "что такое snapshot",
        answer: "snapshot это частотный аккорд состояния, а не полный дамп модели.",
    },
    DialogEntry {
        prompt: "что такое settle loop",
        answer: "settle loop это несколько тиков релаксации перед чтением ответа.",
    },
    DialogEntry {
        prompt: "что ты знаешь",
        answer: "я знаю про nando, organ128, cell32, prompt wave, carrier wave, wave bus, snapshot, rust и локальное обучение.",
    },
    DialogEntry {
        prompt: "расскажи про что ты знаешь",
        answer: "я знаю про клетки, несущие волны, фазовые состояния, снимки памяти и проверку через ablation.",
    },
    DialogEntry {
        prompt: "напиши про rust",
        answer: "rust держит клетки быстрыми и делает ядро предсказуемым.",
    },
    DialogEntry {
        prompt: "привет nando",
        answer: "привет. маленький орган уже слышит волну запроса.",
    },
    DialogEntry {
        prompt: "как клетки пишут",
        answer: "клетки резонируют с промптом, а readout собирает ответ.",
    },
    DialogEntry {
        prompt: "что дальше",
        answer: "дальше нужно учить локальную память клеток и длинный контекст.",
    },
    DialogEntry {
        prompt: "как обучать волнами",
        answer: "нужно переводить промпт и ответ в моды, потом усиливать совпавшие клетки.",
    },
    DialogEntry {
        prompt: "зачем carrier wave",
        answer: "несущая волна удерживает контекст и должна ломаться в контролях no carrier и wrong carrier.",
    },
    DialogEntry {
        prompt: "зачем ablation",
        answer: "ablation показывает, была ли найденная мода настоящей или это просто совпадение.",
    },
    DialogEntry {
        prompt: "зачем mono192",
        answer: "mono192 нужен, чтобы доказать, что клеточная организация лучше простой памяти того же размера.",
    },
    DialogEntry {
        prompt: "nando пишет",
        answer: "nando пока пишет коротко, но уже использует волну промпта.",
    },
];

#[derive(Debug, Clone, Copy)]
struct PromptWave {
    phase: f32,
    amplitude: f32,
    top_slots: [usize; PROMPT_WAVE_TOP_SLOTS],
    byte_bias: [f32; 256],
    slot_energy: [f32; PHASE_SLOTS],
}

impl PromptWave {
    fn from_prompt(lut: &BytePhaseLut, prompt: &str) -> Self {
        Self::from_bytes(lut, prompt.as_bytes())
    }

    fn from_bytes(lut: &BytePhaseLut, bytes: &[u8]) -> Self {
        let mut sin_sum = 0.0f32;
        let mut cos_sum = 0.0f32;
        let mut byte_bias = [0.0f32; 256];
        let mut slot_energy = [0.0f32; PHASE_SLOTS];
        let len = bytes.len().max(1) as f32;

        for (index, byte) in bytes.iter().copied().enumerate() {
            byte_bias[byte as usize] += 1.0 / len;
            let recency = (index + 1) as f32 / len;
            for (slot, phase) in lut.phases(byte).iter().copied().enumerate() {
                let weighted_phase = phase + recency * 0.37;
                sin_sum += weighted_phase.sin() / PHASE_SLOTS as f32;
                cos_sum += weighted_phase.cos() / PHASE_SLOTS as f32;
                slot_energy[slot] += weighted_phase.cos().abs() * (0.5 + recency * 0.5);
            }
        }

        let mut top_slots = [0usize; PROMPT_WAVE_TOP_SLOTS];
        let mut top_scores = [f32::NEG_INFINITY; PROMPT_WAVE_TOP_SLOTS];
        for (slot, energy) in slot_energy.iter().copied().enumerate() {
            insert_top_slot(slot, energy, &mut top_slots, &mut top_scores);
        }

        let phase = sin_sum.atan2(cos_sum).rem_euclid(std::f32::consts::TAU);
        let amplitude = (sin_sum.hypot(cos_sum) * 3.0 / len.sqrt()).clamp(0.05, 1.0);

        Self {
            phase,
            amplitude,
            top_slots,
            byte_bias,
            slot_energy,
        }
    }

    fn byte_bias(&self, byte: u8) -> f32 {
        self.byte_bias[byte as usize]
    }

    fn slot_phase(&self, slot: usize) -> f32 {
        let energy = self.slot_energy[slot] / self.slot_energy_mean().max(f32::EPSILON);
        (energy - 1.0).clamp(-1.0, 1.0) * self.amplitude
    }

    fn cell_bias(&self, cell_index: usize) -> f32 {
        let slot = cell_index % PHASE_SLOTS;
        if self.top_slots.contains(&slot) {
            self.amplitude
        } else {
            0.0
        }
    }

    fn slot_energy_mean(&self) -> f32 {
        self.slot_energy.iter().sum::<f32>() / PHASE_SLOTS as f32
    }

    fn spectral_specificity(&self) -> f32 {
        let total = self.slot_energy.iter().sum::<f32>().max(f32::EPSILON);
        let top_total = self
            .top_slots
            .iter()
            .map(|slot| self.slot_energy[*slot])
            .sum::<f32>();
        let top_share = top_total / total;
        let uniform_share = PROMPT_WAVE_TOP_SLOTS as f32 / PHASE_SLOTS as f32;
        ((top_share - uniform_share) / (1.0 - uniform_share)).clamp(0.0, 1.0)
    }

    fn similarity(self, other: Self) -> f32 {
        let phase_delta = (self.phase - other.phase).cos();
        let amplitude_match = 1.0 - (self.amplitude - other.amplitude).abs();
        let mut slot_overlap = 0.0;
        for slot in self.top_slots {
            if other.top_slots.contains(&slot) {
                slot_overlap += 1.0;
            }
        }
        slot_overlap /= PROMPT_WAVE_TOP_SLOTS as f32;

        phase_delta * 0.35 + amplitude_match * 0.25 + slot_overlap * 0.40
    }

    fn corrupted(self) -> Self {
        let mut corrupted = self;
        corrupted.phase =
            (corrupted.phase + std::f32::consts::PI * 0.73).rem_euclid(std::f32::consts::TAU);
        corrupted.amplitude = (1.0 - corrupted.amplitude * 0.65).clamp(0.05, 1.0);
        corrupted.top_slots.rotate_left(3);
        corrupted
    }

    fn with_phase_pull(mut self, target_phase: f32, strength: f32) -> Self {
        self.phase = (self.phase + circular_delta_local(self.phase, target_phase) * strength)
            .rem_euclid(std::f32::consts::TAU);
        self
    }
}

fn best_dialog_entry(
    lut: &BytePhaseLut,
    prompt: &str,
    prompt_wave: PromptWave,
) -> (DialogEntry, f32) {
    let mut best = DIALOG_CORPUS[0];
    let mut best_score = f32::NEG_INFINITY;
    for entry in DIALOG_CORPUS {
        let entry_wave = PromptWave::from_prompt(lut, entry.prompt);
        let score =
            prompt_wave.similarity(entry_wave) + lexical_overlap_score(prompt, entry.prompt);
        if score > best_score {
            best = entry;
            best_score = score;
        }
    }
    (best, best_score)
}

fn best_settled_dialog_entry(
    lut: &BytePhaseLut,
    prompt: &str,
    prompt_wave: PromptWave,
    settled: &Organ128SettleState,
) -> DialogScore {
    let mut best = DialogScore {
        entry: DIALOG_CORPUS[0],
        total_score: f32::NEG_INFINITY,
        prompt_score: 0.0,
        lexical_score: 0.0,
        wave_score: f32::NEG_INFINITY,
        stability_score: 0.0,
    };
    for entry in DIALOG_CORPUS {
        let score = settled_dialog_score(lut, prompt, prompt_wave, settled, entry);
        if score.total_score > best.total_score {
            best = score;
        }
    }
    best
}

fn candidate_margin(
    lut: &BytePhaseLut,
    prompt: &str,
    prompt_wave: PromptWave,
    settled: &Organ128SettleState,
) -> f32 {
    let mut best = f32::NEG_INFINITY;
    let mut second = f32::NEG_INFINITY;
    for entry in DIALOG_CORPUS {
        let score = settled_dialog_score(lut, prompt, prompt_wave, settled, entry).total_score;
        if score > best {
            second = best;
            best = score;
        } else if score > second {
            second = score;
        }
    }
    (best - second).max(0.0)
}

fn best_wave_dialog_entry(lut: &BytePhaseLut, settled: &Organ128SettleState) -> DialogScore {
    let mut best = DialogScore {
        entry: DIALOG_CORPUS[0],
        total_score: f32::NEG_INFINITY,
        prompt_score: 0.0,
        lexical_score: 0.0,
        wave_score: 0.0,
        stability_score: 0.0,
    };
    for entry in DIALOG_CORPUS {
        let answer_wave = PromptWave::from_prompt(lut, entry.answer);
        let wave_score = wave_answer_score(settled, answer_wave);
        if wave_score > best.wave_score {
            best = DialogScore {
                entry,
                total_score: wave_score,
                prompt_score: 0.0,
                lexical_score: 0.0,
                wave_score,
                stability_score: settled.stability * (1.0 - settled.entropy).clamp(0.0, 1.0),
            };
        }
    }
    best
}

fn settled_dialog_score(
    lut: &BytePhaseLut,
    prompt: &str,
    prompt_wave: PromptWave,
    settled: &Organ128SettleState,
    entry: DialogEntry,
) -> DialogScore {
    let entry_wave = PromptWave::from_prompt(lut, entry.prompt);
    let answer_wave = PromptWave::from_prompt(lut, entry.answer);
    let prompt_score = prompt_wave.similarity(entry_wave) * 0.45;
    let lexical_score = lexical_overlap_score(prompt, entry.prompt) * 0.65;
    let wave_score = wave_answer_score(settled, answer_wave) * 0.35;
    let stability_score = settled.stability * (1.0 - settled.entropy).clamp(0.0, 1.0) * 0.20;
    let total_score = prompt_score + lexical_score + wave_score + stability_score;

    DialogScore {
        entry,
        total_score,
        prompt_score,
        lexical_score,
        wave_score,
        stability_score,
    }
}

fn wave_answer_score(settled: &Organ128SettleState, answer_wave: PromptWave) -> f32 {
    (settled.center_phase - answer_wave.phase).cos() * settled.coherence
}

#[derive(Debug, Clone)]
struct WaveDialogScorer {
    learning_rate: f32,
    weights: [f32; WAVE_SCORER_FEATURES],
    mask: FeatureMask,
}

#[derive(Debug, Clone)]
struct WaveScorerEvalReport {
    accuracy: f32,
    wave_agree: f32,
    rows: Vec<WaveScorerEvalRow>,
}

#[derive(Debug, Clone)]
struct WaveScorerEvalRow {
    prompt: &'static str,
    predicted_prompt: &'static str,
    target_prompt: &'static str,
    score: f32,
}

#[derive(Debug, Clone)]
struct TrainedWaveDialogScorer {
    scorer: WaveDialogScorer,
    train_cases: usize,
    train_accuracy_before_update: f32,
}

#[derive(Debug, Clone)]
struct ResponseGateEvalReport {
    cases: usize,
    answered: usize,
    refused: usize,
    rows: Vec<ResponseGateEvalRow>,
}

impl ResponseGateEvalReport {
    fn answer_rate(&self) -> f32 {
        self.answered as f32 / self.cases.max(1) as f32
    }

    fn refuse_rate(&self) -> f32 {
        self.refused as f32 / self.cases.max(1) as f32
    }
}

#[derive(Debug, Clone)]
struct ResponseGateEvalRow {
    kind: &'static str,
    prompt: &'static str,
    verdict: SettleVerdict,
    gate: ResponseGate,
    score: f32,
    margin: f32,
    lexical_score: f32,
    wave_score: f32,
}

#[derive(Debug, Clone, Copy)]
struct ThoughtProbeSample {
    kind: &'static str,
    prompt: &'static str,
    should_answer: bool,
}

#[derive(Debug, Clone)]
struct ThoughtProbe {
    weights: [f32; THOUGHT_PROBE_FEATURES],
    learning_rate: f32,
}

#[derive(Debug, Clone)]
struct ThoughtProbeEvalReport {
    cases: usize,
    correct: usize,
    known_cases: usize,
    known_answered: usize,
    refusal_cases: usize,
    refusal_refused: usize,
    rows: Vec<ThoughtProbeEvalRow>,
}

impl ThoughtProbeEvalReport {
    fn accuracy(&self) -> f32 {
        self.correct as f32 / self.cases.max(1) as f32
    }

    fn known_answer_rate(&self) -> f32 {
        self.known_answered as f32 / self.known_cases.max(1) as f32
    }

    fn refusal_refuse_rate(&self) -> f32 {
        self.refusal_refused as f32 / self.refusal_cases.max(1) as f32
    }
}

#[derive(Debug, Clone)]
struct ThoughtProbeEvalRow {
    kind: &'static str,
    prompt: &'static str,
    should_answer: bool,
    predicted_answer: bool,
    score: f32,
    thought_verdict: ThoughtVerdict,
    strength: f32,
    convergence: f32,
    drift: f32,
    prompt_specificity: f32,
    specificity: f32,
    role_balance: f32,
    memory_alignment: f32,
}

#[derive(Debug, Clone, Copy)]
struct FeatureMask {
    enabled: [bool; WAVE_SCORER_FEATURES],
}

const WAVE_SCORER_FEATURES: usize = 12;
const THOUGHT_PROBE_FEATURES: usize = 11;

impl WaveDialogScorer {
    fn new(learning_rate: f32, mask: FeatureMask) -> Self {
        Self {
            learning_rate,
            weights: [0.0; WAVE_SCORER_FEATURES],
            mask,
        }
    }

    fn predict(
        &self,
        lut: &BytePhaseLut,
        prompt_wave: PromptWave,
        settled: &Organ128SettleState,
    ) -> usize {
        self.best_index(lut, prompt_wave, settled).0
    }

    fn best_index(
        &self,
        lut: &BytePhaseLut,
        prompt_wave: PromptWave,
        settled: &Organ128SettleState,
    ) -> (usize, f32) {
        let mut best_index = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (index, entry) in DIALOG_CORPUS.iter().copied().enumerate() {
            let features = wave_scorer_features(lut, prompt_wave, settled, entry);
            let score = dot_masked(&self.weights, &features, self.mask);
            if score > best_score {
                best_index = index;
                best_score = score;
            }
        }
        (best_index, best_score)
    }

    fn update(
        &mut self,
        lut: &BytePhaseLut,
        prompt_wave: PromptWave,
        settled: &Organ128SettleState,
        target_index: usize,
        predicted_index: usize,
    ) {
        if target_index == predicted_index {
            return;
        }
        let target = wave_scorer_features(lut, prompt_wave, settled, DIALOG_CORPUS[target_index]);
        let predicted =
            wave_scorer_features(lut, prompt_wave, settled, DIALOG_CORPUS[predicted_index]);
        for (feature_index, ((weight, target_value), predicted_value)) in self
            .weights
            .iter_mut()
            .zip(target.iter())
            .zip(predicted.iter())
            .enumerate()
        {
            if !self.mask.enabled[feature_index] {
                continue;
            }
            *weight += self.learning_rate * (target_value - predicted_value);
        }
    }

    fn weight_abs_mean(&self) -> f32 {
        self.weights.iter().map(|value| value.abs()).sum::<f32>() / self.weights.len() as f32
    }
}

impl ThoughtProbe {
    fn new(learning_rate: f32) -> Self {
        Self {
            weights: [0.0; THOUGHT_PROBE_FEATURES],
            learning_rate,
        }
    }

    fn score(&self, settled: &Organ128SettleState) -> f32 {
        let features = thought_probe_features(settled);
        self.weights
            .iter()
            .zip(features.iter())
            .map(|(weight, feature)| weight * feature)
            .sum()
    }

    fn predict(&self, settled: &Organ128SettleState) -> bool {
        self.score(settled) >= 0.0
    }

    fn update(&mut self, settled: &Organ128SettleState, should_answer: bool) {
        let predicted = self.predict(settled);
        if predicted == should_answer {
            return;
        }
        let target = if should_answer { 1.0 } else { -1.0 };
        let features = thought_probe_features(settled);
        for (weight, feature) in self.weights.iter_mut().zip(features.iter()) {
            *weight += self.learning_rate * target * feature;
        }
    }

    fn weight_abs_mean(&self) -> f32 {
        self.weights.iter().map(|value| value.abs()).sum::<f32>() / self.weights.len() as f32
    }
}

impl FeatureMask {
    fn full() -> Self {
        Self {
            enabled: [true; WAVE_SCORER_FEATURES],
        }
    }

    fn without_center() -> Self {
        let mut mask = Self::full();
        mask.enabled[1] = false;
        mask.enabled[2] = false;
        mask
    }

    fn without_memory() -> Self {
        let mut mask = Self::full();
        mask.enabled[3] = false;
        mask.enabled[4] = false;
        mask.enabled[11] = false;
        mask
    }

    fn without_prompt() -> Self {
        let mut mask = Self::full();
        mask.enabled[5] = false;
        mask.enabled[6] = false;
        mask.enabled[7] = false;
        mask
    }

    fn without_global() -> Self {
        let mut mask = Self::full();
        mask.enabled[8] = false;
        mask.enabled[9] = false;
        mask.enabled[10] = false;
        mask
    }

    fn without_validation() -> Self {
        let mut mask = Self::full();
        mask.enabled[11] = false;
        mask
    }
}

fn train_wave_dialog_scorer(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    epochs: usize,
    ticks: usize,
    train_indices: &[usize],
    mask: FeatureMask,
) -> TrainedWaveDialogScorer {
    let mut scorer = WaveDialogScorer::new(0.08, mask);
    let mut train_cases = 0usize;
    let mut train_correct_before = 0usize;
    for _ in 0..epochs {
        for index in train_indices.iter().copied() {
            let entry = DIALOG_CORPUS[index];
            let prompt_wave = PromptWave::from_prompt(lut, entry.prompt);
            let settled = organ.settle_dialog(
                lut,
                seed.wrapping_add(index as u64),
                entry.prompt,
                prompt_wave,
                ticks,
                CarrierMode::Correct,
            );
            let predicted = scorer.predict(lut, prompt_wave, &settled);
            if predicted == index {
                train_correct_before += 1;
            }
            scorer.update(lut, prompt_wave, &settled, index, predicted);
            train_cases += 1;
        }
    }
    TrainedWaveDialogScorer {
        scorer,
        train_cases,
        train_accuracy_before_update: train_correct_before as f32 / train_cases.max(1) as f32,
    }
}

fn eval_wave_dialog_scorer(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    scorer: &WaveDialogScorer,
    seed: u64,
    ticks: usize,
    indices: &[usize],
) -> WaveScorerEvalReport {
    let mut correct = 0usize;
    let mut wave_agree = 0usize;
    let mut rows = Vec::new();
    for index in indices.iter().copied() {
        let entry = DIALOG_CORPUS[index];
        let prompt_wave = PromptWave::from_prompt(lut, entry.prompt);
        let settled = organ.settle_dialog(
            lut,
            seed.wrapping_add(index as u64),
            entry.prompt,
            prompt_wave,
            ticks,
            CarrierMode::Correct,
        );
        let (predicted, score) = scorer.best_index(lut, prompt_wave, &settled);
        let wave_predicted = best_wave_dialog_entry(lut, &settled);
        if predicted == index {
            correct += 1;
        }
        if DIALOG_CORPUS[predicted].prompt == wave_predicted.entry.prompt {
            wave_agree += 1;
        }
        rows.push(WaveScorerEvalRow {
            prompt: entry.prompt,
            predicted_prompt: DIALOG_CORPUS[predicted].prompt,
            target_prompt: entry.prompt,
            score,
        });
    }
    let cases = indices.len().max(1);
    WaveScorerEvalReport {
        accuracy: correct as f32 / cases as f32,
        wave_agree: wave_agree as f32 / cases as f32,
        rows,
    }
}

fn eval_untrained_wave_dialog_scorer(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    ticks: usize,
    indices: &[usize],
) -> WaveScorerEvalReport {
    let mut correct = 0usize;
    let mut rows = Vec::new();
    for index in indices.iter().copied() {
        let entry = DIALOG_CORPUS[index];
        let prompt_wave = PromptWave::from_prompt(lut, entry.prompt);
        let settled = organ.settle_dialog(
            lut,
            seed.wrapping_add(index as u64),
            entry.prompt,
            prompt_wave,
            ticks,
            CarrierMode::Correct,
        );
        let predicted = best_wave_dialog_entry(lut, &settled);
        if predicted.entry.prompt == entry.prompt {
            correct += 1;
        }
        rows.push(WaveScorerEvalRow {
            prompt: entry.prompt,
            predicted_prompt: predicted.entry.prompt,
            target_prompt: entry.prompt,
            score: predicted.wave_score,
        });
    }
    let cases = indices.len().max(1);
    WaveScorerEvalReport {
        accuracy: correct as f32 / cases as f32,
        wave_agree: 1.0,
        rows,
    }
}

fn eval_response_gate_known(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    ticks: usize,
) -> ResponseGateEvalReport {
    let mut answered = 0usize;
    let mut refused = 0usize;
    let mut rows = Vec::new();
    for (index, entry) in DIALOG_CORPUS.iter().copied().enumerate() {
        let row = response_gate_eval_row(
            lut,
            organ,
            seed.wrapping_add(index as u64),
            ticks,
            "known",
            entry.prompt,
        );
        match row.gate {
            ResponseGate::Answer => answered += 1,
            ResponseGate::Refuse => refused += 1,
        }
        rows.push(row);
    }
    ResponseGateEvalReport {
        cases: DIALOG_CORPUS.len(),
        answered,
        refused,
        rows,
    }
}

fn eval_response_gate_refusal(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    ticks: usize,
) -> ResponseGateEvalReport {
    let mut answered = 0usize;
    let mut refused = 0usize;
    let mut rows = Vec::new();
    for (index, prompt) in RESPONSE_GATE_REFUSAL_PROMPTS.iter().copied().enumerate() {
        let row = response_gate_eval_row(
            lut,
            organ,
            seed.wrapping_add(10_000 + index as u64),
            ticks,
            "refusal",
            prompt,
        );
        match row.gate {
            ResponseGate::Answer => answered += 1,
            ResponseGate::Refuse => refused += 1,
        }
        rows.push(row);
    }
    ResponseGateEvalReport {
        cases: RESPONSE_GATE_REFUSAL_PROMPTS.len(),
        answered,
        refused,
        rows,
    }
}

fn response_gate_eval_row(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    ticks: usize,
    kind: &'static str,
    prompt: &'static str,
) -> ResponseGateEvalRow {
    let prompt_wave = PromptWave::from_prompt(lut, prompt);
    let settled = organ.settle_dialog(lut, seed, prompt, prompt_wave, ticks, CarrierMode::Correct);
    let selected = best_settled_dialog_entry(lut, prompt, prompt_wave, &settled);
    let margin = candidate_margin(lut, prompt, prompt_wave, &settled);
    let gate = response_gate(prompt, &settled, &selected, margin);
    ResponseGateEvalRow {
        kind,
        prompt,
        verdict: settled.verdict(),
        gate,
        score: selected.total_score,
        margin,
        lexical_score: selected.lexical_score,
        wave_score: selected.wave_score,
    }
}

fn thought_probe_train_samples() -> Vec<ThoughtProbeSample> {
    thought_probe_samples(false)
}

fn thought_probe_holdout_samples() -> Vec<ThoughtProbeSample> {
    thought_probe_samples(true)
}

fn thought_probe_samples(holdout: bool) -> Vec<ThoughtProbeSample> {
    let mut samples = Vec::new();
    for (index, entry) in DIALOG_CORPUS.iter().copied().enumerate() {
        if (index % 3 == 1) == holdout {
            samples.push(ThoughtProbeSample {
                kind: "known",
                prompt: entry.prompt,
                should_answer: true,
            });
        }
    }
    for (index, prompt) in RESPONSE_GATE_REFUSAL_PROMPTS.iter().copied().enumerate() {
        if (index % 3 == 1) == holdout {
            samples.push(ThoughtProbeSample {
                kind: "refusal",
                prompt,
                should_answer: false,
            });
        }
    }
    samples
}

fn settle_thought_probe_sample(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    seed: u64,
    ticks: usize,
    sample: ThoughtProbeSample,
) -> Organ128SettleState {
    let prompt_wave = PromptWave::from_prompt(lut, sample.prompt);
    organ.settle_dialog(
        lut,
        seed,
        sample.prompt,
        prompt_wave,
        ticks,
        CarrierMode::Correct,
    )
}

fn eval_thought_probe(
    lut: &BytePhaseLut,
    organ: &Organ128Runtime,
    probe: &ThoughtProbe,
    seed: u64,
    ticks: usize,
    samples: Vec<ThoughtProbeSample>,
) -> ThoughtProbeEvalReport {
    let mut correct = 0usize;
    let mut known_cases = 0usize;
    let mut known_answered = 0usize;
    let mut refusal_cases = 0usize;
    let mut refusal_refused = 0usize;
    let mut rows = Vec::new();
    for (index, sample) in samples.iter().copied().enumerate() {
        let settled = settle_thought_probe_sample(
            lut,
            organ,
            seed.wrapping_add(20_000 + index as u64),
            ticks,
            sample,
        );
        let score = probe.score(&settled);
        let predicted_answer = score >= 0.0;
        if predicted_answer == sample.should_answer {
            correct += 1;
        }
        if sample.should_answer {
            known_cases += 1;
            if predicted_answer {
                known_answered += 1;
            }
        } else {
            refusal_cases += 1;
            if !predicted_answer {
                refusal_refused += 1;
            }
        }
        rows.push(ThoughtProbeEvalRow {
            kind: sample.kind,
            prompt: sample.prompt,
            should_answer: sample.should_answer,
            predicted_answer,
            score,
            thought_verdict: settled.thought.verdict(),
            strength: settled.thought.strength,
            convergence: settled.thought.convergence,
            drift: settled.thought.drift,
            prompt_specificity: settled.thought.prompt_specificity,
            specificity: settled.thought.specificity,
            role_balance: settled.thought.role_balance,
            memory_alignment: settled.thought.memory_alignment,
        });
    }
    ThoughtProbeEvalReport {
        cases: rows.len(),
        correct,
        known_cases,
        known_answered,
        refusal_cases,
        refusal_refused,
        rows,
    }
}

fn dialog_train_indices() -> Vec<usize> {
    (0..DIALOG_CORPUS.len())
        .filter(|index| index % 3 != 1)
        .collect()
}

fn dialog_holdout_indices() -> Vec<usize> {
    (0..DIALOG_CORPUS.len())
        .filter(|index| index % 3 == 1)
        .collect()
}

fn wave_scorer_features(
    lut: &BytePhaseLut,
    prompt_wave: PromptWave,
    settled: &Organ128SettleState,
    entry: DialogEntry,
) -> [f32; WAVE_SCORER_FEATURES] {
    let answer_wave = PromptWave::from_prompt(lut, entry.answer);
    let prompt_entry_wave = PromptWave::from_prompt(lut, entry.prompt);
    let center_answer = circular_delta_local(answer_wave.phase, settled.center_phase);
    let memory_answer = circular_delta_local(answer_wave.phase, settled.memory.phase);
    let prompt_answer = circular_delta_local(answer_wave.phase, prompt_wave.phase);
    let prompt_entry = circular_delta_local(prompt_entry_wave.phase, prompt_wave.phase);
    [
        1.0,
        center_answer.cos() * settled.coherence,
        center_answer.sin() * settled.coherence,
        memory_answer.cos() * settled.memory.strength * settled.memory.validation,
        memory_answer.sin() * settled.memory.strength * settled.memory.validation,
        prompt_answer.cos() * prompt_wave.amplitude,
        prompt_answer.sin() * prompt_wave.amplitude,
        prompt_entry.cos() * prompt_wave.amplitude,
        settled.coherence,
        1.0 - settled.entropy,
        settled.stability,
        settled.memory.validation,
    ]
}

fn thought_probe_features(settled: &Organ128SettleState) -> [f32; THOUGHT_PROBE_FEATURES] {
    [
        1.0,
        settled.thought.strength,
        settled.thought.convergence,
        1.0 - settled.thought.drift,
        settled.thought.prompt_specificity,
        settled.thought.specificity,
        settled.thought.role_balance,
        settled.thought.memory_alignment,
        settled.coherence,
        settled.stability,
        settled.memory.validation,
    ]
}

fn phase_to_slot(phase: f32) -> usize {
    let normalized = phase.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU;
    ((normalized * PHASE_SLOTS as f32).round() as usize) % PHASE_SLOTS
}

fn dot_masked<const N: usize>(weights: &[f32; N], features: &[f32; N], mask: FeatureMask) -> f32 {
    weights
        .iter()
        .zip(features.iter())
        .zip(mask.enabled.iter())
        .map(
            |((weight, feature), enabled)| {
                if *enabled { weight * feature } else { 0.0 }
            },
        )
        .sum()
}

fn lexical_overlap_score(prompt: &str, entry_prompt: &str) -> f32 {
    let mut overlap = 0usize;
    let mut total = 0usize;
    for token in prompt.split_whitespace() {
        total += 1;
        if entry_prompt.split_whitespace().any(|entry| entry == token) {
            overlap += 1;
        }
    }
    overlap as f32 / total.max(1) as f32
}

impl Organ128ByteLearner {
    fn new(learning_rate: f32) -> Self {
        Self {
            learning_rate,
            byte_bias: [0.0; 256],
            transition_weights: [[0.0; 256]; 256],
            cell_byte_weights: vec![[0.0; 256]; 128],
        }
    }

    fn predict(&self, active: &[(usize, f32); 4], input_byte: u8) -> u8 {
        let mut best_byte = b' ';
        let mut best_score = f32::NEG_INFINITY;

        for byte in 0..=u8::MAX {
            let score = self.score(active, input_byte, byte);
            if score > best_score {
                best_score = score;
                best_byte = byte;
            }
        }

        best_byte
    }

    fn predict_decoded(
        &self,
        active: &[(usize, f32); 4],
        input_byte: u8,
        decoder: &DecoderState,
        prompt_wave: PromptWave,
        seed: u64,
        step: usize,
    ) -> u8 {
        let mut top = [(b' ', f32::NEG_INFINITY); 8];
        for byte in b' '..=b'z' {
            if !decoder.allowed(byte) {
                continue;
            }
            let score = self.score(active, input_byte, byte)
                + decoder.context_prior(byte)
                + prompt_wave.byte_bias(byte) * 0.12;
            insert_top_byte(byte, score, &mut top);
        }

        if !top[0].1.is_finite() {
            return b' ';
        }
        if top[1].1.is_finite()
            && top[0].1 - top[1].1 < 0.025
            && deterministic_pick(seed, step, input_byte, 5) == 0
        {
            return top[1].0;
        }
        top[0].0
    }

    fn score(&self, active: &[(usize, f32); 4], input_byte: u8, byte: u8) -> f32 {
        let byte_index = byte as usize;
        let mut score = self.byte_bias[byte_index] * 0.05;
        score += self.transition_weights[input_byte as usize][byte_index] * 0.85;
        score += ascii_prior(byte);

        for (rank, (cell_index, resonance)) in active.iter().copied().enumerate() {
            if cell_index >= self.cell_byte_weights.len() {
                continue;
            }
            let rank_gain = (4 - rank) as f32 / 4.0;
            let resonance_gain = (1.0 + resonance.abs()).clamp(0.25, 2.0);
            score += self.cell_byte_weights[cell_index][byte_index] * rank_gain * resonance_gain;
        }

        score
    }

    fn update(&mut self, active: &[(usize, f32); 4], input_byte: u8, target: u8) -> bool {
        let predicted = self.predict(active, input_byte);
        let correct = predicted == target;
        if correct {
            return true;
        }

        self.byte_bias[target as usize] += self.learning_rate * 0.05;
        self.byte_bias[predicted as usize] -= self.learning_rate * 0.05;
        self.transition_weights[input_byte as usize][target as usize] += self.learning_rate * 0.55;
        self.transition_weights[input_byte as usize][predicted as usize] -=
            self.learning_rate * 0.55;

        for (rank, (cell_index, resonance)) in active.iter().copied().enumerate() {
            if cell_index >= self.cell_byte_weights.len() {
                continue;
            }
            let gain = self.learning_rate * (4 - rank) as f32 / 4.0 * (1.0 + resonance.abs());
            self.cell_byte_weights[cell_index][target as usize] += gain;
            self.cell_byte_weights[cell_index][predicted as usize] -= gain;
        }

        false
    }

    fn state_abs_mean(&self) -> f32 {
        let bias_sum: f32 = self.byte_bias.iter().map(|value| value.abs()).sum();
        let transition_sum: f32 = self
            .transition_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let cell_sum: f32 = self
            .cell_byte_weights
            .iter()
            .flat_map(|row| row.iter())
            .map(|value| value.abs())
            .sum();
        let denom = 256 + 256 * 256 + 128 * 256;

        (bias_sum + transition_sum + cell_sum) / denom as f32
    }
}

fn train_organ128(
    organ: &Organ128Runtime,
    lut: &BytePhaseLut,
    learner: &mut Organ128ByteLearner,
    seed: u64,
    bytes: &[u8],
    epochs: usize,
) -> Organ128TrainReport {
    let mut cases = 0usize;
    let mut correct = 0usize;
    let train_wave = PromptWave::from_bytes(lut, bytes);

    for epoch in 0..epochs {
        for (index, pair) in bytes.windows(2).enumerate() {
            let active =
                organ.active_cells(lut, seed, pair[0], train_wave, epoch * bytes.len() + index);
            if learner.update(&active, pair[0], pair[1]) {
                correct += 1;
            }
            cases += 1;
        }
    }

    Organ128TrainReport {
        cases,
        accuracy: correct as f32 / cases.max(1) as f32,
    }
}

fn generate_organ128(
    organ: &Organ128Runtime,
    lut: &BytePhaseLut,
    learner: &Organ128ByteLearner,
    seed: u64,
    prompt: &str,
    prompt_wave: PromptWave,
    generate_len: usize,
) -> String {
    let mut output = String::from(prompt);
    let mut current = prompt.as_bytes().last().copied().unwrap_or(b' ');
    let mut decoder = DecoderState::from_prompt(prompt);

    for step in 0..generate_len {
        let active = organ.active_cells(lut, seed, current, prompt_wave, step);
        let next = learner.predict_decoded(&active, current, &decoder, prompt_wave, seed, step);
        output.push(next as char);
        decoder.observe(next);
        current = next;
    }

    output
}

#[derive(Debug, Clone, Copy)]
struct DecoderState {
    last: u8,
    repeat_run: usize,
    word_len: usize,
    word: [u8; 16],
    last_word_len: usize,
    last_word: [u8; 16],
    after_sentence_end: bool,
}

impl DecoderState {
    fn from_prompt(prompt: &str) -> Self {
        let mut state = Self {
            last: b' ',
            repeat_run: 0,
            word_len: 0,
            word: [0; 16],
            last_word_len: 0,
            last_word: [0; 16],
            after_sentence_end: false,
        };
        for byte in prompt.bytes() {
            state.observe(byte);
        }
        state
    }

    fn observe(&mut self, byte: u8) {
        if byte == self.last {
            self.repeat_run += 1;
        } else {
            self.last = byte;
            self.repeat_run = 1;
        }
        if byte == b' ' || byte == b'.' {
            if self.word_len > 0 {
                self.last_word_len = self.word_len.min(self.word.len());
                self.last_word = [0; 16];
                self.last_word[..self.last_word_len]
                    .copy_from_slice(&self.word[..self.last_word_len]);
            }
            self.word_len = 0;
            self.word = [0; 16];
        } else {
            if self.word_len < self.word.len() {
                self.word[self.word_len] = byte;
            }
            self.word_len += 1;
        }
        self.after_sentence_end = byte == b'.';
    }

    fn allowed(&self, byte: u8) -> bool {
        if !is_printable_generation_byte(byte) {
            return false;
        }
        if self.after_sentence_end {
            return byte == b' ';
        }
        if self.repeat_run >= 2 && byte == self.last {
            return false;
        }
        if self.word_len >= 10 && byte != b' ' && byte != b'.' {
            return false;
        }
        if self.last == b' ' && matches!(byte, b'.' | b',' | b';' | b':') {
            return false;
        }
        true
    }

    fn context_prior(&self, byte: u8) -> f32 {
        let mut prior = 0.0;
        if self.word_len >= 4 && byte == b' ' {
            prior += 0.18;
        }
        if self.current_word_is_known() && byte == b' ' {
            prior += 0.30;
        }
        if self.current_word_is_known() && byte.is_ascii_lowercase() {
            prior -= 0.18;
        }
        if self.word_len >= 6 && byte == b'.' {
            prior += 0.08;
        }
        if self.word_len == 0 && byte.is_ascii_lowercase() {
            prior += 0.04;
            prior += self.next_word_start_prior(byte);
        }
        prior += self.word_prefix_prior(byte);
        if byte == b' ' && self.last == b' ' {
            prior -= 0.60;
        }
        prior
    }

    fn word_prefix_prior(&self, byte: u8) -> f32 {
        if !byte.is_ascii_lowercase() {
            return 0.0;
        }
        let mut candidate = [0u8; 17];
        let copy_len = self.word_len.min(self.word.len());
        candidate[..copy_len].copy_from_slice(&self.word[..copy_len]);
        candidate[copy_len] = byte;
        let candidate = &candidate[..copy_len + 1];

        let mut has_prefix = false;
        let mut exact_word = false;
        for word in ORGAN128_WORDS {
            if word.starts_with(candidate) {
                has_prefix = true;
                if word.len() == candidate.len() {
                    exact_word = true;
                }
            }
        }

        let repeat_penalty = self.repeat_word_penalty(candidate);
        match (has_prefix, exact_word, self.word_len) {
            (true, true, _) => 0.22 + repeat_penalty,
            (true, false, _) => 0.16 + repeat_penalty,
            (false, _, 0) => -0.02,
            (false, _, _) => -0.18,
        }
    }

    fn repeat_word_penalty(&self, candidate: &[u8]) -> f32 {
        if self.last_word_len == 0 || candidate.is_empty() {
            return 0.0;
        }
        let last = &self.last_word[..self.last_word_len];
        if last == candidate {
            -0.45
        } else if last.starts_with(candidate) || candidate.starts_with(last) {
            -0.20
        } else {
            0.0
        }
    }

    fn current_word_is_known(&self) -> bool {
        if self.word_len == 0 || self.word_len > self.word.len() {
            return false;
        }
        let current = &self.word[..self.word_len];
        ORGAN128_WORDS.contains(&current)
    }

    fn next_word_start_prior(&self, byte: u8) -> f32 {
        if self.last_word_len == 0 {
            return 0.0;
        }
        let last = &self.last_word[..self.last_word_len];
        let preferred = match last {
            b"nando" => matches!(byte, b'w' | b's' | b'o' | b'h'),
            b"wave" => matches!(byte, b'c' | b'l' | b'o' | b't'),
            b"rust" => matches!(byte, b'c' | b'm' | b'a'),
            b"organ" | b"organ128" => matches!(byte, b'l' | b'k' | b'i' | b'w'),
            b"cells" => matches!(byte, b'l' | b'm' | b'a' | b'w'),
            b"text" => matches!(byte, b'g' | b'f' | b'w'),
            _ => false,
        };
        if preferred { 0.22 } else { -0.03 }
    }
}

const ORGAN128_WORDS: [&[u8]; 28] = [
    b"nando",
    b"wave",
    b"organ",
    b"organ128",
    b"learns",
    b"letters",
    b"rust",
    b"cells",
    b"make",
    b"small",
    b"text",
    b"keeps",
    b"fast",
    b"hot",
    b"warm",
    b"short",
    b"generated",
    b"writes",
    b"says",
    b"hello",
    b"from",
    b"answer",
    b"memory",
    b"grows",
    b"simple",
    b"byte",
    b"rhythm",
    b"alive",
];

fn insert_top_cell<const N: usize>(cell_index: usize, score: f32, top: &mut [(usize, f32); N]) {
    for index in 0..top.len() {
        if score > top[index].1 {
            for shift in (index + 1..top.len()).rev() {
                top[shift] = top[shift - 1];
            }
            top[index] = (cell_index, score);
            break;
        }
    }
}

fn insert_top_byte(byte: u8, score: f32, top: &mut [(u8, f32); 8]) {
    for index in 0..top.len() {
        if score > top[index].1 {
            for shift in (index + 1..top.len()).rev() {
                top[shift] = top[shift - 1];
            }
            top[index] = (byte, score);
            break;
        }
    }
}

fn insert_top_slot(slot: usize, score: f32, slots: &mut [usize], scores: &mut [f32]) {
    for index in 0..scores.len() {
        if score > scores[index] {
            for shift in (index + 1..scores.len()).rev() {
                scores[shift] = scores[shift - 1];
                slots[shift] = slots[shift - 1];
            }
            scores[index] = score;
            slots[index] = slot;
            break;
        }
    }
}

fn insert_unique_top_slot(slot: usize, score: f32, slots: &mut [usize], scores: &mut [f32]) {
    if slots.contains(&slot) {
        return;
    }
    for index in 0..scores.len() {
        if score > scores[index] {
            for shift in (index + 1..scores.len()).rev() {
                scores[shift] = scores[shift - 1];
                slots[shift] = slots[shift - 1];
            }
            scores[index] = score;
            slots[index] = slot;
            break;
        }
    }
}

fn organ128_cell_role(cell_id: usize) -> Organ128CellRole {
    match cell_id {
        0..=63 => Organ128CellRole::Fast,
        64..=95 => Organ128CellRole::Mid,
        96..=111 => Organ128CellRole::Guard,
        112..=119 => Organ128CellRole::Carrier,
        _ => Organ128CellRole::Memory,
    }
}

fn quota_active_from_hot(hot: &[(usize, f32); 32]) -> [(usize, f32); 4] {
    let mut active = [(usize::MAX, f32::NEG_INFINITY); 4];
    let mut used = [false; 32];

    fill_active_role_slots(hot, &mut used, &mut active, 0, 2, |role| {
        matches!(role, Organ128CellRole::Fast)
    });
    fill_active_role_slots(hot, &mut used, &mut active, 2, 1, |role| {
        matches!(role, Organ128CellRole::Mid | Organ128CellRole::Guard)
    });
    fill_active_role_slots(hot, &mut used, &mut active, 3, 1, |role| {
        matches!(role, Organ128CellRole::Carrier | Organ128CellRole::Memory)
    });

    for active_slot in &mut active {
        if active_slot.0 != usize::MAX {
            continue;
        }
        for (hot_index, cell) in hot.iter().copied().enumerate() {
            if used[hot_index] {
                continue;
            }
            *active_slot = cell;
            used[hot_index] = true;
            break;
        }
    }

    active
}

fn fill_active_role_slots(
    hot: &[(usize, f32); 32],
    used: &mut [bool; 32],
    active: &mut [(usize, f32); 4],
    start: usize,
    count: usize,
    accepts: impl Fn(Organ128CellRole) -> bool,
) {
    let mut filled = 0usize;
    for (hot_index, cell) in hot.iter().copied().enumerate() {
        if used[hot_index] || !accepts(organ128_cell_role(cell.0)) {
            continue;
        }
        active[start + filled] = cell;
        used[hot_index] = true;
        filled += 1;
        if filled == count {
            break;
        }
    }
}

fn deterministic_pick(seed: u64, step: usize, input_byte: u8, window: usize) -> usize {
    let mixed = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((step as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((input_byte as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    (mixed as usize) % window.max(1)
}

fn carrier_from_prompt_wave(seed: u64, input_byte: u8, prompt_wave: PromptWave) -> CarrierWave {
    let mut carrier = CarrierWave::from_seed(seed, input_byte);
    carrier.phase =
        (carrier.phase * 0.35 + prompt_wave.phase * 0.65).rem_euclid(std::f32::consts::TAU);
    carrier.amplitude = (carrier.amplitude * 0.60 + prompt_wave.amplitude * 0.40).clamp(0.05, 1.0);
    carrier.frequency =
        (carrier.frequency * 0.80 + 1.0 + prompt_wave.amplitude * 0.20).clamp(0.25, 4.0);
    carrier.boundary = (0.62 + prompt_wave.amplitude * 0.20).clamp(0.10, 1.0);
    carrier
}

fn circular_delta_local(from: f32, to: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (to - from + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

fn ascii_prior(byte: u8) -> f32 {
    match byte {
        b'a'..=b'z' | b'A'..=b'Z' => 0.03,
        b' ' => 0.04,
        b'.' | b',' | b';' | b':' => 0.02,
        b'\n' | b'\t' => -0.20,
        0..=31 | 127..=255 => -0.50,
        _ => 0.0,
    }
}

fn is_printable_generation_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'.' | b',' | b';' | b':' | b'0'..=b'9' | b'a'..=b'z')
}
