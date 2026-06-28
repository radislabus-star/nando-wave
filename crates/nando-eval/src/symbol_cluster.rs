use nando_core::{
    PeakOutcome, SYMBOL_L3_DEFAULT_WAVE_CLUSTERS, SYMBOL_WAVE_CLUSTER_CELLS, SymbolCell8Advice,
    SymbolWaveCluster,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolClusterEvalReport {
    pub cluster_cells: usize,
    pub l3_clusters: usize,
    pub sequence_cases: usize,
    pub centered_cases: usize,
    pub accepted_cases: usize,
    pub reflection_cases: usize,
    pub reflection_not_accepted: usize,
    pub mode_status: &'static str,
}

impl SymbolClusterEvalReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            concat!(
                "SymbolWaveCluster eval\n",
                "cluster_cells: {cluster_cells}\n",
                "l3_clusters: {l3_clusters}\n",
                "sequence_cases: {sequence_cases}\n",
                "centered_cases: {centered_cases}\n",
                "accepted_cases: {accepted_cases}\n",
                "reflection_cases: {reflection_cases}\n",
                "reflection_not_accepted: {reflection_not_accepted}\n",
                "mode_status: {mode_status}\n"
            ),
            cluster_cells = self.cluster_cells,
            l3_clusters = self.l3_clusters,
            sequence_cases = self.sequence_cases,
            centered_cases = self.centered_cases,
            accepted_cases = self.accepted_cases,
            reflection_cases = self.reflection_cases,
            reflection_not_accepted = self.reflection_not_accepted,
            mode_status = self.mode_status
        )
    }
}

#[must_use]
pub fn symbol_cluster_eval() -> SymbolClusterEvalReport {
    let cases = ['N', 'A', 'D', 'W'];
    let mut centered_cases = 0usize;
    let mut accepted_cases = 0usize;

    for (index, symbol) in cases.iter().copied().enumerate() {
        let mut cluster = SymbolWaveCluster::new(index as u32, 0xC1A5_7000 + index as u64);
        let _ = cluster.tick_symbol(symbol);
        let _ = cluster.tick_symbol(symbol);
        let third = cluster.tick_symbol(symbol);
        centered_cases += usize::from(third.center.support_count > 0 && third.center.energy > 0);
        accepted_cases += usize::from(third.center.outcome == PeakOutcome::Accepted);
    }

    let reflection_cases = 1usize;
    let mut reflected_cluster = SymbolWaveCluster::new(99, 0xF1EC_7000);
    let incoming = [SymbolCell8Advice {
        peak_slot: 11,
        energy: 240,
        coherence: 8,
        phase: 127,
        role: 99,
    }];
    let reflected = reflected_cluster.tick_symbol_with_incoming('R', &incoming);
    let reflection_not_accepted = usize::from(
        reflected.reflected_messages > 0 && reflected.center.outcome != PeakOutcome::Accepted,
    );

    let mode_status = if centered_cases == cases.len()
        && reflection_not_accepted == reflection_cases
        && SYMBOL_WAVE_CLUSTER_CELLS == 16
        && SYMBOL_L3_DEFAULT_WAVE_CLUSTERS == 32
    {
        "symbol-cluster-eval-pass"
    } else {
        "symbol-cluster-eval-watch"
    };

    SymbolClusterEvalReport {
        cluster_cells: SYMBOL_WAVE_CLUSTER_CELLS,
        l3_clusters: SYMBOL_L3_DEFAULT_WAVE_CLUSTERS,
        sequence_cases: cases.len(),
        centered_cases,
        accepted_cases,
        reflection_cases,
        reflection_not_accepted,
        mode_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_cluster_report_has_required_gates() {
        let report = symbol_cluster_eval();

        assert_eq!(report.cluster_cells, 16);
        assert_eq!(report.l3_clusters, 32);
        assert_eq!(report.centered_cases, report.sequence_cases);
        assert_eq!(report.reflection_not_accepted, report.reflection_cases);
        assert_eq!(report.mode_status, "symbol-cluster-eval-pass");

        let text = report.to_text();
        assert!(text.contains("SymbolWaveCluster eval"));
        assert!(text.contains("l3_clusters: 32"));
        assert!(text.contains("symbol-cluster-eval-pass"));
    }
}
