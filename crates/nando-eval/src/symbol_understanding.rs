use nando_core::{
    SYMBOL_CELL8_BYTES, SYMBOL_WAVE_CLUSTER_CELLS, SymbolClusterCenter, SymbolL3Center,
    SymbolL3Organism, SymbolWaveCluster,
};

const CONTEXT_PROBES: [ContextProbe; 4] = [
    ContextProbe {
        left_context: "NAX",
        right_context: "WAX",
        probe: '?',
    },
    ContextProbe {
        left_context: "KOL",
        right_context: "SOL",
        probe: '?',
    },
    ContextProbe {
        left_context: "MIR",
        right_context: "DIR",
        probe: '?',
    },
    ContextProbe {
        left_context: "ABQ",
        right_context: "CBQ",
        probe: '?',
    },
];

const UNDERSTANDING_SCALE_CLUSTERS: [usize; 5] = [1, 4, 8, 16, 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContextProbe {
    left_context: &'static str,
    right_context: &'static str,
    probe: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolUnderstandingScaleRow {
    pub profile: &'static str,
    pub clusters: usize,
    pub cells: usize,
    pub active_bytes: usize,
    pub probe_cases: usize,
    pub markov1_collision_cases: usize,
    pub centered_cases: usize,
    pub context_split_cases: usize,
    pub suffix_control_collapses: usize,
    pub replay_stable_cases: usize,
    pub understanding0_candidates: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GateCounts {
    probe_cases: usize,
    markov1_collision_cases: usize,
    centered_cases: usize,
    context_split_cases: usize,
    suffix_control_collapses: usize,
    replay_stable_cases: usize,
    understanding0_candidates: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolUnderstanding0EvalReport {
    pub probe_cases: usize,
    pub markov1_collision_cases: usize,
    pub centered_cases: usize,
    pub context_split_cases: usize,
    pub suffix_control_collapses: usize,
    pub replay_stable_cases: usize,
    pub understanding0_candidates: usize,
    pub min_passing_cells: usize,
    pub scale_rows: [SymbolUnderstandingScaleRow; 5],
    pub mode_status: &'static str,
}

impl SymbolUnderstanding0EvalReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!(
            concat!(
                "Symbol understanding-0 eval\n",
                "probe_cases: {probe_cases}\n",
                "markov1_collision_cases: {markov1_collision_cases}\n",
                "centered_cases: {centered_cases}\n",
                "context_split_cases: {context_split_cases}\n",
                "suffix_control_collapses: {suffix_control_collapses}\n",
                "replay_stable_cases: {replay_stable_cases}\n",
                "understanding0_candidates: {understanding0_candidates}\n",
                "min_passing_cells: {min_passing_cells}\n"
            ),
            probe_cases = self.probe_cases,
            markov1_collision_cases = self.markov1_collision_cases,
            centered_cases = self.centered_cases,
            context_split_cases = self.context_split_cases,
            suffix_control_collapses = self.suffix_control_collapses,
            replay_stable_cases = self.replay_stable_cases,
            understanding0_candidates = self.understanding0_candidates,
            min_passing_cells = self.min_passing_cells
        );

        for row in self.scale_rows {
            text.push_str(&format!(
                concat!(
                    "scale_profile: {profile}\n",
                    "scale_clusters: {clusters}\n",
                    "scale_cells: {cells}\n",
                    "scale_active_bytes: {active_bytes}\n",
                    "scale_centered_cases: {centered_cases}\n",
                    "scale_context_split_cases: {context_split_cases}\n",
                    "scale_suffix_control_collapses: {suffix_control_collapses}\n",
                    "scale_replay_stable_cases: {replay_stable_cases}\n",
                    "scale_understanding0_candidates: {understanding0_candidates}\n",
                    "scale_status: {mode_status}\n"
                ),
                profile = row.profile,
                clusters = row.clusters,
                cells = row.cells,
                active_bytes = row.active_bytes,
                centered_cases = row.centered_cases,
                context_split_cases = row.context_split_cases,
                suffix_control_collapses = row.suffix_control_collapses,
                replay_stable_cases = row.replay_stable_cases,
                understanding0_candidates = row.understanding0_candidates,
                mode_status = row.mode_status
            ));
        }

        text.push_str(&format!("mode_status: {}\n", self.mode_status));
        text
    }
}

#[must_use]
pub fn symbol_understanding0_eval() -> SymbolUnderstanding0EvalReport {
    let cluster_counts = cluster_gate_counts();
    let scale_rows = UNDERSTANDING_SCALE_CLUSTERS.map(scale_gate_row);
    let min_passing_cells = scale_rows
        .iter()
        .find(|row| row.mode_status == "symbol-understanding0-scale-pass")
        .map_or(0, |row| row.cells);

    let mode_status = if gate_passes(cluster_counts)
        && min_passing_cells > 0
        && scale_rows
            .iter()
            .all(|row| row.mode_status == "symbol-understanding0-scale-pass")
    {
        "symbol-understanding0-eval-pass"
    } else {
        "symbol-understanding0-eval-watch"
    };

    SymbolUnderstanding0EvalReport {
        probe_cases: cluster_counts.probe_cases,
        markov1_collision_cases: cluster_counts.markov1_collision_cases,
        centered_cases: cluster_counts.centered_cases,
        context_split_cases: cluster_counts.context_split_cases,
        suffix_control_collapses: cluster_counts.suffix_control_collapses,
        replay_stable_cases: cluster_counts.replay_stable_cases,
        understanding0_candidates: cluster_counts.understanding0_candidates,
        min_passing_cells,
        scale_rows,
        mode_status,
    }
}

fn cluster_gate_counts() -> GateCounts {
    let mut counts = GateCounts::new();

    for (index, probe) in CONTEXT_PROBES.iter().copied().enumerate() {
        let seed = 0xA11D_0000 + index as u64;
        let left = run_cluster_context_probe(seed, probe.left_context, probe.probe);
        let right = run_cluster_context_probe(seed, probe.right_context, probe.probe);
        let replay = run_cluster_context_probe(seed, probe.left_context, probe.probe);
        let suffix = last_char(probe.left_context);
        let suffix_left = run_cluster_context_probe(seed, suffix, probe.probe);
        let suffix_right = run_cluster_context_probe(seed, suffix, probe.probe);

        counts.add(
            markov1_collision(probe),
            is_cluster_centered(left) && is_cluster_centered(right),
            cluster_centers_split(left, right),
            !cluster_centers_split(suffix_left, suffix_right),
            left == replay,
        );
    }

    counts
}

fn scale_gate_row(clusters: usize) -> SymbolUnderstandingScaleRow {
    let mut counts = GateCounts::new();

    for (index, probe) in CONTEXT_PROBES.iter().copied().enumerate() {
        let seed = 0xA11D_0000 + index as u64;
        let left = run_l3_context_probe(seed, clusters, probe.left_context, probe.probe);
        let right = run_l3_context_probe(seed, clusters, probe.right_context, probe.probe);
        let replay = run_l3_context_probe(seed, clusters, probe.left_context, probe.probe);
        let suffix = last_char(probe.left_context);
        let suffix_left = run_l3_context_probe(seed, clusters, suffix, probe.probe);
        let suffix_right = run_l3_context_probe(seed, clusters, suffix, probe.probe);

        counts.add(
            markov1_collision(probe),
            is_l3_centered(left) && is_l3_centered(right),
            l3_centers_split(left, right),
            !l3_centers_split(suffix_left, suffix_right),
            left == replay,
        );
    }

    let cells = clusters * SYMBOL_WAVE_CLUSTER_CELLS;
    SymbolUnderstandingScaleRow {
        profile: scale_profile_name(clusters),
        clusters,
        cells,
        active_bytes: cells * SYMBOL_CELL8_BYTES,
        probe_cases: counts.probe_cases,
        markov1_collision_cases: counts.markov1_collision_cases,
        centered_cases: counts.centered_cases,
        context_split_cases: counts.context_split_cases,
        suffix_control_collapses: counts.suffix_control_collapses,
        replay_stable_cases: counts.replay_stable_cases,
        understanding0_candidates: counts.understanding0_candidates,
        mode_status: if gate_passes(counts) {
            "symbol-understanding0-scale-pass"
        } else {
            "symbol-understanding0-scale-watch"
        },
    }
}

impl GateCounts {
    fn new() -> Self {
        Self {
            probe_cases: 0,
            markov1_collision_cases: 0,
            centered_cases: 0,
            context_split_cases: 0,
            suffix_control_collapses: 0,
            replay_stable_cases: 0,
            understanding0_candidates: 0,
        }
    }

    fn add(
        &mut self,
        markov1_collision: bool,
        centered: bool,
        context_split: bool,
        suffix_control_collapse: bool,
        replay_stable: bool,
    ) {
        let candidate = markov1_collision
            && centered
            && context_split
            && suffix_control_collapse
            && replay_stable;

        self.probe_cases += 1;
        self.markov1_collision_cases += usize::from(markov1_collision);
        self.centered_cases += usize::from(centered);
        self.context_split_cases += usize::from(context_split);
        self.suffix_control_collapses += usize::from(suffix_control_collapse);
        self.replay_stable_cases += usize::from(replay_stable);
        self.understanding0_candidates += usize::from(candidate);
    }
}

fn gate_passes(counts: GateCounts) -> bool {
    counts.probe_cases == CONTEXT_PROBES.len()
        && counts.markov1_collision_cases == counts.probe_cases
        && counts.centered_cases == counts.probe_cases
        && counts.context_split_cases == counts.probe_cases
        && counts.suffix_control_collapses == counts.probe_cases
        && counts.replay_stable_cases == counts.probe_cases
        && counts.understanding0_candidates == counts.probe_cases
}

fn run_cluster_context_probe(seed: u64, context: &str, probe: char) -> SymbolClusterCenter {
    let mut cluster = SymbolWaveCluster::new(0, seed);
    for symbol in context.chars() {
        let _ = cluster.tick_symbol(symbol);
    }
    cluster.tick_symbol(probe).center
}

fn run_l3_context_probe(seed: u64, clusters: usize, context: &str, probe: char) -> SymbolL3Center {
    let mut organism = SymbolL3Organism::with_clusters(seed, clusters);
    for symbol in context.chars() {
        let _ = organism.tick_symbol(symbol);
    }
    organism.tick_symbol(probe).center
}

fn markov1_collision(probe: ContextProbe) -> bool {
    last_char(probe.left_context) == last_char(probe.right_context)
        && probe.left_context != probe.right_context
}

fn last_char(value: &str) -> &str {
    let start = value
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0);
    &value[start..]
}

fn is_cluster_centered(center: SymbolClusterCenter) -> bool {
    center.energy > 0 && center.support_count > 0 && center.coherence > 0
}

fn is_l3_centered(center: SymbolL3Center) -> bool {
    center.energy > 0 && center.support_cells > 0 && center.coherence > 0
}

fn cluster_centers_split(left: SymbolClusterCenter, right: SymbolClusterCenter) -> bool {
    left.peak_slot != right.peak_slot || left.carrier_phase != right.carrier_phase
}

fn l3_centers_split(left: SymbolL3Center, right: SymbolL3Center) -> bool {
    left.peak_slot != right.peak_slot || left.carrier_phase != right.carrier_phase
}

fn scale_profile_name(clusters: usize) -> &'static str {
    match clusters {
        1 => "cluster-16",
        4 => "micro-64",
        8 => "small-128",
        16 => "turbo-256",
        32 => "default-512",
        _ => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_understanding0_report_has_required_gates() {
        let report = symbol_understanding0_eval();

        assert_eq!(report.probe_cases, 4);
        assert_eq!(report.markov1_collision_cases, report.probe_cases);
        assert_eq!(report.centered_cases, report.probe_cases);
        assert_eq!(report.context_split_cases, report.probe_cases);
        assert_eq!(report.suffix_control_collapses, report.probe_cases);
        assert_eq!(report.replay_stable_cases, report.probe_cases);
        assert_eq!(report.understanding0_candidates, report.probe_cases);
        assert_eq!(report.min_passing_cells, 16);

        let expected_cells = [16, 64, 128, 256, 512];
        let expected_context_splits = [4, 4, 3, 4, 4];
        let expected_statuses = [
            "symbol-understanding0-scale-pass",
            "symbol-understanding0-scale-pass",
            "symbol-understanding0-scale-watch",
            "symbol-understanding0-scale-pass",
            "symbol-understanding0-scale-pass",
        ];
        for ((row, cells), (context_splits, status)) in report
            .scale_rows
            .iter()
            .zip(expected_cells)
            .zip(expected_context_splits.into_iter().zip(expected_statuses))
        {
            assert_eq!(row.cells, cells);
            assert_eq!(row.markov1_collision_cases, row.probe_cases);
            assert_eq!(row.centered_cases, row.probe_cases);
            assert_eq!(row.context_split_cases, context_splits);
            assert_eq!(row.suffix_control_collapses, row.probe_cases);
            assert_eq!(row.replay_stable_cases, row.probe_cases);
            assert_eq!(row.understanding0_candidates, context_splits);
            assert_eq!(row.mode_status, status);
        }
        assert_eq!(report.mode_status, "symbol-understanding0-eval-watch");

        let text = report.to_text();
        assert!(text.contains("Symbol understanding-0 eval"));
        assert!(text.contains("scale_profile: turbo-256"));
        assert!(text.contains("scale_profile: default-512"));
        assert!(text.contains("understanding0_candidates"));
        assert!(text.contains("scale_profile: small-128"));
        assert!(text.contains("symbol-understanding0-eval-watch"));
    }
}
