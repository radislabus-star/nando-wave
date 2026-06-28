use std::fmt::Write;

use nando_core::{
    L3_8MB_SYMBOL_ACTIVE_BYTES, L3_8MB_SYMBOL_CELL8_CELLS, L3_8MB_SYMBOL_WAVE_CLUSTERS,
    PeakOutcome, SYMBOL_L3_DEFAULT_ACTIVE_BYTES, SYMBOL_L3_DEFAULT_CELL8_CELLS,
    SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, SYMBOL_L3_TURBO_ACTIVE_BYTES, SYMBOL_L3_TURBO_CELL8_CELLS,
    SYMBOL_L3_TURBO_WAVE_CLUSTERS, SymbolCell8Advice, SymbolL3Organism,
};

const SYMBOL_L3_EVAL_CASES: [char; 2] = ['N', 'A'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolL3ProfileEvalRow {
    pub profile: &'static str,
    pub l3_clusters: usize,
    pub l3_cells: usize,
    pub active_bytes: usize,
    pub sequence_cases: usize,
    pub centered_cases: usize,
    pub accepted_cases: usize,
    pub reflection_cases: usize,
    pub reflection_not_accepted: usize,
    pub max_forward_messages: u32,
    pub max_reflected_messages: u32,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolL3EvalReport {
    pub profiles: [SymbolL3ProfileEvalRow; 3],
    pub mode_status: &'static str,
}

impl SymbolL3EvalReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        writeln!(&mut text, "SymbolL3Organism profile eval").expect("write to string");
        writeln!(&mut text, "profile_count: {}", self.profiles.len()).expect("write to string");
        for row in self.profiles {
            writeln!(&mut text, "profile: {}", row.profile).expect("write to string");
            writeln!(&mut text, "l3_clusters: {}", row.l3_clusters).expect("write to string");
            writeln!(&mut text, "l3_cells: {}", row.l3_cells).expect("write to string");
            writeln!(&mut text, "active_bytes: {}", row.active_bytes).expect("write to string");
            writeln!(&mut text, "sequence_cases: {}", row.sequence_cases).expect("write to string");
            writeln!(&mut text, "centered_cases: {}", row.centered_cases).expect("write to string");
            writeln!(&mut text, "accepted_cases: {}", row.accepted_cases).expect("write to string");
            writeln!(&mut text, "reflection_cases: {}", row.reflection_cases)
                .expect("write to string");
            writeln!(
                &mut text,
                "reflection_not_accepted: {}",
                row.reflection_not_accepted
            )
            .expect("write to string");
            writeln!(
                &mut text,
                "max_forward_messages: {}",
                row.max_forward_messages
            )
            .expect("write to string");
            writeln!(
                &mut text,
                "max_reflected_messages: {}",
                row.max_reflected_messages
            )
            .expect("write to string");
            writeln!(&mut text, "profile_status: {}", row.mode_status).expect("write to string");
        }
        writeln!(&mut text, "mode_status: {}", self.mode_status).expect("write to string");
        text
    }
}

#[must_use]
pub fn symbol_l3_eval() -> SymbolL3EvalReport {
    let profiles = [
        profile_eval(
            "turbo-256",
            SYMBOL_L3_TURBO_WAVE_CLUSTERS,
            SYMBOL_L3_TURBO_CELL8_CELLS,
            SYMBOL_L3_TURBO_ACTIVE_BYTES,
        ),
        profile_eval(
            "default-512",
            SYMBOL_L3_DEFAULT_WAVE_CLUSTERS,
            SYMBOL_L3_DEFAULT_CELL8_CELLS,
            SYMBOL_L3_DEFAULT_ACTIVE_BYTES,
        ),
        profile_eval(
            "stress-1024",
            L3_8MB_SYMBOL_WAVE_CLUSTERS,
            L3_8MB_SYMBOL_CELL8_CELLS,
            L3_8MB_SYMBOL_ACTIVE_BYTES,
        ),
    ];

    let mode_status = if profiles
        .iter()
        .all(|row| row.mode_status == "symbol-l3-profile-pass")
        && profiles[0].l3_cells == 256
        && profiles[1].l3_cells == 512
        && profiles[2].l3_cells == 1024
    {
        "symbol-l3-eval-pass"
    } else {
        "symbol-l3-eval-watch"
    };

    SymbolL3EvalReport {
        profiles,
        mode_status,
    }
}

fn profile_eval(
    profile: &'static str,
    l3_clusters: usize,
    l3_cells: usize,
    active_bytes: usize,
) -> SymbolL3ProfileEvalRow {
    let mut centered_cases = 0usize;
    let mut accepted_cases = 0usize;
    let mut max_forward_messages = 0u32;
    let mut max_reflected_messages = 0u32;

    for (index, symbol) in SYMBOL_L3_EVAL_CASES.iter().copied().enumerate() {
        let mut organism = SymbolL3Organism::with_clusters(0x13_0000 + index as u64, l3_clusters);
        let first = organism.tick_symbol(symbol);
        let _ = organism.tick_symbol(symbol);
        let third = organism.tick_symbol(symbol);
        centered_cases += usize::from(third.center.support_cells > 0 && third.center.energy > 0);
        accepted_cases += usize::from(third.center.outcome == PeakOutcome::Accepted);
        max_forward_messages = max_forward_messages
            .max(first.forward_messages)
            .max(third.forward_messages);
        max_reflected_messages = max_reflected_messages
            .max(first.reflected_messages)
            .max(third.reflected_messages);
    }

    let reflection_cases = 1usize;
    let mut reflected_organism = SymbolL3Organism::with_clusters(0x13_F1EC, l3_clusters);
    let incoming = [SymbolCell8Advice {
        peak_slot: 5,
        energy: 240,
        coherence: 8,
        phase: 127,
        role: 99,
    }];
    let reflected = reflected_organism.tick_symbol_with_incoming('R', &incoming);
    let reflection_not_accepted = usize::from(
        reflected.reflected_messages > 0 && reflected.center.outcome != PeakOutcome::Accepted,
    );
    max_forward_messages = max_forward_messages.max(reflected.forward_messages);
    max_reflected_messages = max_reflected_messages.max(reflected.reflected_messages);

    let mode_status = if centered_cases == SYMBOL_L3_EVAL_CASES.len()
        && reflection_not_accepted == reflection_cases
        && max_forward_messages <= l3_cells as u32
        && max_reflected_messages <= l3_cells as u32
    {
        "symbol-l3-profile-pass"
    } else {
        "symbol-l3-profile-watch"
    };

    SymbolL3ProfileEvalRow {
        profile,
        l3_clusters,
        l3_cells,
        active_bytes,
        sequence_cases: SYMBOL_L3_EVAL_CASES.len(),
        centered_cases,
        accepted_cases,
        reflection_cases,
        reflection_not_accepted,
        max_forward_messages,
        max_reflected_messages,
        mode_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_l3_report_has_required_gates() {
        let report = symbol_l3_eval();

        assert_eq!(report.profiles.len(), 3);
        assert_eq!(report.profiles[0].profile, "turbo-256");
        assert_eq!(report.profiles[0].l3_clusters, 16);
        assert_eq!(report.profiles[0].l3_cells, 256);
        assert_eq!(report.profiles[0].active_bytes, 2 * 1024 * 1024);
        assert_eq!(report.profiles[1].profile, "default-512");
        assert_eq!(report.profiles[1].l3_clusters, 32);
        assert_eq!(report.profiles[1].l3_cells, 512);
        assert_eq!(report.profiles[1].active_bytes, 4 * 1024 * 1024);
        assert_eq!(report.profiles[2].profile, "stress-1024");
        assert_eq!(report.profiles[2].l3_clusters, 64);
        assert_eq!(report.profiles[2].l3_cells, 1024);
        assert_eq!(report.profiles[2].active_bytes, 8 * 1024 * 1024);

        for row in report.profiles {
            assert_eq!(row.centered_cases, row.sequence_cases);
            assert_eq!(row.reflection_not_accepted, row.reflection_cases);
            assert!(row.max_forward_messages <= row.l3_cells as u32);
            assert!(row.max_reflected_messages <= row.l3_cells as u32);
            assert_eq!(row.mode_status, "symbol-l3-profile-pass");
        }
        assert_eq!(report.mode_status, "symbol-l3-eval-pass");

        let text = report.to_text();
        assert!(text.contains("SymbolL3Organism profile eval"));
        assert!(text.contains("profile: turbo-256"));
        assert!(text.contains("profile: default-512"));
        assert!(text.contains("profile: stress-1024"));
        assert!(text.contains("symbol-l3-eval-pass"));
    }
}
