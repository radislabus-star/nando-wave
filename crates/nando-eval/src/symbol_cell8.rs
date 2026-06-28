use nando_core::{PeakOutcome, SymbolCell8, SymbolCell8Advice};

const SYMBOL_CELL8_EVAL_SEQUENCES: [&str; 4] = ["nanda", "wave", "vector", "cache"];
const MARKOV_AMBIGUITY_CORPUS: [&str; 4] = ["ab", "ac", "ad", "ae"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolCell8EvalReport {
    pub sequence_cases: usize,
    pub first_tick_accepts: usize,
    pub persistent_accepts: usize,
    pub ablation_cases: usize,
    pub ablation_survivals: usize,
    pub spurious_cases: usize,
    pub spurious_rejections: usize,
    pub limit_cycle_cases: usize,
    pub limit_cycle_detections: usize,
    pub reflection_cases: usize,
    pub reflection_detections: usize,
    pub markov1_ambiguous_states: usize,
    pub mode_status: &'static str,
}

impl SymbolCell8EvalReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            concat!(
                "SymbolCell8 eval\n",
                "sequence_cases: {sequence_cases}\n",
                "first_tick_accepts: {first_tick_accepts}\n",
                "persistent_accepts: {persistent_accepts}\n",
                "ablation_cases: {ablation_cases}\n",
                "ablation_survivals: {ablation_survivals}\n",
                "spurious_cases: {spurious_cases}\n",
                "spurious_rejections: {spurious_rejections}\n",
                "limit_cycle_cases: {limit_cycle_cases}\n",
                "limit_cycle_detections: {limit_cycle_detections}\n",
                "reflection_cases: {reflection_cases}\n",
                "reflection_detections: {reflection_detections}\n",
                "markov1_ambiguous_states: {markov1_ambiguous_states}\n",
                "mode_status: {mode_status}\n"
            ),
            sequence_cases = self.sequence_cases,
            first_tick_accepts = self.first_tick_accepts,
            persistent_accepts = self.persistent_accepts,
            ablation_cases = self.ablation_cases,
            ablation_survivals = self.ablation_survivals,
            spurious_cases = self.spurious_cases,
            spurious_rejections = self.spurious_rejections,
            limit_cycle_cases = self.limit_cycle_cases,
            limit_cycle_detections = self.limit_cycle_detections,
            reflection_cases = self.reflection_cases,
            reflection_detections = self.reflection_detections,
            markov1_ambiguous_states = self.markov1_ambiguous_states,
            mode_status = self.mode_status
        )
    }
}

#[must_use]
pub fn symbol_cell8_eval() -> SymbolCell8EvalReport {
    let sequence_cases = SYMBOL_CELL8_EVAL_SEQUENCES.len();
    let mut first_tick_accepts = 0;
    let mut persistent_accepts = 0;

    for (index, sequence) in SYMBOL_CELL8_EVAL_SEQUENCES.iter().enumerate() {
        let mut cell = SymbolCell8::new(index as u32, 1, 0xC311_8000 + index as u64);
        let symbol = sequence.chars().next().unwrap_or('x');
        let first = cell.tick_symbol(symbol);
        let _ = cell.tick_symbol(symbol);
        let third = cell.tick_symbol(symbol);
        first_tick_accepts += usize::from(first.outcome == PeakOutcome::Accepted);
        persistent_accepts += usize::from(third.outcome == PeakOutcome::Accepted);
    }

    let ablation_cases = SYMBOL_CELL8_EVAL_SEQUENCES.len();
    let mut ablation_survivals = 0;
    for (index, sequence) in SYMBOL_CELL8_EVAL_SEQUENCES.iter().enumerate() {
        let mut cell = SymbolCell8::new(100 + index as u32, 2, 0xAB1A_7000 + index as u64);
        let symbol = sequence.chars().next().unwrap_or('x');
        let lane = cell.project_symbol(symbol).lane as usize;
        cell.projection[lane] = Default::default();
        let _ = cell.tick_symbol(symbol);
        let _ = cell.tick_symbol(symbol);
        let third = cell.tick_symbol(symbol);
        ablation_survivals +=
            usize::from(third.score.energy > 0 && !matches!(third.outcome, PeakOutcome::NoPeak));
    }

    let spurious_cases = 1;
    let mut spurious_cell = SymbolCell8::new(200, 3, 0x5A11_EE00);
    spurious_cell.calibration.coherence_min = 255;
    spurious_cell.calibration.accept = 1;
    let spurious = spurious_cell.tick_symbol('s');
    let spurious_rejections = usize::from(spurious.outcome == PeakOutcome::Spurious);

    let limit_cycle_cases = 1;
    let mut probe = SymbolCell8::new(300, 4, 0x1A11_CE00);
    let expected = probe.tick_symbol_with_context('l', Some(0), 0, &[]);
    let mut cycle_cell = SymbolCell8::new(300, 4, 0x1A11_CE00);
    cycle_cell.calibration.accept = 1;
    cycle_cell.calibration.previous_peak = expected.peak_slot;
    cycle_cell.calibration.last_peak = (expected.peak_slot + 1) % 128;
    let limit_cycle = cycle_cell.tick_symbol_with_context('l', Some(0), 0, &[]);
    let limit_cycle_detections = usize::from(limit_cycle.outcome == PeakOutcome::LimitCycle);

    let reflection_cases = 1;
    let mut reflection_cell = SymbolCell8::new(400, 5, 0x0E1E_C700);
    let incoming = [SymbolCell8Advice {
        peak_slot: 3,
        energy: 240,
        coherence: 12,
        phase: 127,
        role: 9,
    }];
    let reflected = reflection_cell.tick_symbol_with_context('r', None, -120, &incoming);
    let reflection_detections = usize::from(reflected.outcome == PeakOutcome::Reflected);

    let markov1_ambiguous_states = markov1_ambiguous_states(&MARKOV_AMBIGUITY_CORPUS);
    let mode_status = if first_tick_accepts == 0
        && persistent_accepts > first_tick_accepts
        && ablation_survivals == ablation_cases
        && spurious_rejections == spurious_cases
        && limit_cycle_detections == limit_cycle_cases
        && reflection_detections == reflection_cases
        && markov1_ambiguous_states > 0
    {
        "symbol-cell8-eval-pass"
    } else {
        "symbol-cell8-eval-watch"
    };

    SymbolCell8EvalReport {
        sequence_cases,
        first_tick_accepts,
        persistent_accepts,
        ablation_cases,
        ablation_survivals,
        spurious_cases,
        spurious_rejections,
        limit_cycle_cases,
        limit_cycle_detections,
        reflection_cases,
        reflection_detections,
        markov1_ambiguous_states,
        mode_status,
    }
}

fn markov1_ambiguous_states(corpus: &[&str]) -> usize {
    let mut nexts = [[false; 256]; 256];
    for text in corpus {
        let bytes = text.as_bytes();
        for pair in bytes.windows(2) {
            nexts[pair[0] as usize][pair[1] as usize] = true;
        }
    }

    nexts
        .iter()
        .filter(|row| row.iter().filter(|seen| **seen).count() > 1)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_cell8_report_has_required_gates() {
        let report = symbol_cell8_eval();

        assert_eq!(report.first_tick_accepts, 0);
        assert!(report.persistent_accepts > report.first_tick_accepts);
        assert_eq!(report.ablation_survivals, report.ablation_cases);
        assert_eq!(report.spurious_rejections, report.spurious_cases);
        assert_eq!(report.limit_cycle_detections, report.limit_cycle_cases);
        assert_eq!(report.reflection_detections, report.reflection_cases);
        assert!(report.markov1_ambiguous_states > 0);
        assert_eq!(report.mode_status, "symbol-cell8-eval-pass");

        let text = report.to_text();
        assert!(text.contains("first_tick_accepts"));
        assert!(text.contains("markov1_ambiguous_states"));
        assert!(text.contains("symbol-cell8-eval-pass"));
    }
}
