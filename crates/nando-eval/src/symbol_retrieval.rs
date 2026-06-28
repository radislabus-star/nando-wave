use nando_core::{SYMBOL_CELL8_BYTES, SYMBOL_WAVE_CLUSTER_CELLS, SymbolL3Center, SymbolL3Organism};

const RETRIEVAL_PATTERNS: [RetrievalPattern; 4] = [
    RetrievalPattern {
        name: "nanda",
        pattern: "NANDA",
        noisy_probe: "NA?DA",
    },
    RetrievalPattern {
        name: "wave",
        pattern: "WAVE",
        noisy_probe: "WA?E",
    },
    RetrievalPattern {
        name: "cache",
        pattern: "CACHE",
        noisy_probe: "CA?HE",
    },
    RetrievalPattern {
        name: "vector",
        pattern: "VECTOR",
        noisy_probe: "VEC?OR",
    },
];

const CONFLICT_PROBES: [&str; 4] = ["WANDA", "NAVE", "CAV?OR", "?????"];
const RETRIEVAL_SCALE_CLUSTERS: [usize; 3] = [1, 16, 32];
const RETRIEVAL_TURBO_CLUSTERS: usize = 16;
const RETRIEVAL_REPEATS: usize = 3;
const RETRIEVAL_CAPACITY_REPEATS: usize = 3;
const RETRIEVAL_STABILITY_COUNTS: [usize; 4] = [4, 8, 16, 32];
const RETRIEVAL_CAPACITY_COUNTS: [usize; 4] = [32, 64, 128, 256];
const RETRIEVAL_CAPACITY_SCALE_CLUSTERS: [usize; 3] = [16, 32, 64];
const RETRIEVAL_CAPACITY_SCALE_PATTERNS: usize = 256;
const RETRIEVAL_CAPACITY_SEEDS: [u64; 2] = [0xCA9A_C17A_0001, 0xCA9A_C17A_0002];
const RETRIEVAL_CAPACITY_CONFLICTS: usize = 16;
const STRONG_MARGIN: u16 = 8;
const SUPERPOSITION_STRONG_MARGIN: u16 = 4;
const TRAINED_RESONANCE_GAIN_MIN: u32 = 24;
const PROJECTION_MISMATCH_PENALTY: u32 = 512;
const TRAJECTORY_STEP_MAX: u16 = 24;

const RETRIEVAL_STABILITY_PATTERNS: [RetrievalPattern; 32] = [
    RetrievalPattern {
        name: "nanda",
        pattern: "NANDA",
        noisy_probe: "NA?DA",
    },
    RetrievalPattern {
        name: "wave",
        pattern: "WAVE",
        noisy_probe: "WA?E",
    },
    RetrievalPattern {
        name: "cache",
        pattern: "CACHE",
        noisy_probe: "CA?HE",
    },
    RetrievalPattern {
        name: "vector",
        pattern: "VECTOR",
        noisy_probe: "VEC?OR",
    },
    RetrievalPattern {
        name: "logic",
        pattern: "LOGIC",
        noisy_probe: "LO?IC",
    },
    RetrievalPattern {
        name: "phase",
        pattern: "PHASE",
        noisy_probe: "PH?SE",
    },
    RetrievalPattern {
        name: "carrier",
        pattern: "CARRIER",
        noisy_probe: "CAR?IER",
    },
    RetrievalPattern {
        name: "memory",
        pattern: "MEMORY",
        noisy_probe: "MEM?RY",
    },
    RetrievalPattern {
        name: "signal",
        pattern: "SIGNAL",
        noisy_probe: "SIG?AL",
    },
    RetrievalPattern {
        name: "filter",
        pattern: "FILTER",
        noisy_probe: "FIL?ER",
    },
    RetrievalPattern {
        name: "route",
        pattern: "ROUTE",
        noisy_probe: "RO?TE",
    },
    RetrievalPattern {
        name: "search",
        pattern: "SEARCH",
        noisy_probe: "SEA?CH",
    },
    RetrievalPattern {
        name: "token",
        pattern: "TOKEN",
        noisy_probe: "TO?EN",
    },
    RetrievalPattern {
        name: "symbol",
        pattern: "SYMBOL",
        noisy_probe: "SYM?OL",
    },
    RetrievalPattern {
        name: "stable",
        pattern: "STABLE",
        noisy_probe: "STA?LE",
    },
    RetrievalPattern {
        name: "peak",
        pattern: "PEAKS",
        noisy_probe: "PE?KS",
    },
    RetrievalPattern {
        name: "noise",
        pattern: "NOISE",
        noisy_probe: "NO?SE",
    },
    RetrievalPattern {
        name: "delta",
        pattern: "DELTA",
        noisy_probe: "DE?TA",
    },
    RetrievalPattern {
        name: "omega",
        pattern: "OMEGA",
        noisy_probe: "OM?GA",
    },
    RetrievalPattern {
        name: "sigma",
        pattern: "SIGMA",
        noisy_probe: "SI?MA",
    },
    RetrievalPattern {
        name: "alpha",
        pattern: "ALPHA",
        noisy_probe: "AL?HA",
    },
    RetrievalPattern {
        name: "beta",
        pattern: "BETAS",
        noisy_probe: "BE?AS",
    },
    RetrievalPattern {
        name: "gamma",
        pattern: "GAMMA",
        noisy_probe: "GA?MA",
    },
    RetrievalPattern {
        name: "theta",
        pattern: "THETA",
        noisy_probe: "TH?TA",
    },
    RetrievalPattern {
        name: "kappa",
        pattern: "KAPPA",
        noisy_probe: "KA?PA",
    },
    RetrievalPattern {
        name: "lambda",
        pattern: "LAMBDA",
        noisy_probe: "LAM?DA",
    },
    RetrievalPattern {
        name: "matrix",
        pattern: "MATRIX",
        noisy_probe: "MAT?IX",
    },
    RetrievalPattern {
        name: "orbit",
        pattern: "ORBIT",
        noisy_probe: "OR?IT",
    },
    RetrievalPattern {
        name: "thread",
        pattern: "THREAD",
        noisy_probe: "THR?AD",
    },
    RetrievalPattern {
        name: "buffer",
        pattern: "BUFFER",
        noisy_probe: "BUF?ER",
    },
    RetrievalPattern {
        name: "window",
        pattern: "WINDOW",
        noisy_probe: "WIN?OW",
    },
    RetrievalPattern {
        name: "center",
        pattern: "CENTER",
        noisy_probe: "CEN?ER",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetrievalPattern {
    name: &'static str,
    pattern: &'static str,
    noisy_probe: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapacityPattern {
    pattern: String,
    noisy_probe: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Prototype {
    name: &'static str,
    pattern: &'static str,
    center: SymbolL3Center,
    path: Vec<SymbolL3Center>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapacityPrototype {
    pattern: String,
    center: SymbolL3Center,
    path: Vec<SymbolL3Center>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueryTrace {
    center: SymbolL3Center,
    path: Vec<SymbolL3Center>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolRetrievalScaleRow {
    pub profile: &'static str,
    pub clusters: usize,
    pub cells: usize,
    pub active_bytes: usize,
    pub stored_patterns: usize,
    pub noisy_probe_cases: usize,
    pub noisy_hits: usize,
    pub noisy_strong_hits: usize,
    pub veto_noisy_accepts: usize,
    pub veto_noisy_close_steps: usize,
    pub veto_noisy_compared_steps: usize,
    pub conflict_cases: usize,
    pub conflict_rejections: usize,
    pub veto_conflict_rejections: usize,
    pub veto_conflict_close_steps: usize,
    pub veto_conflict_compared_steps: usize,
    pub cold_ablation_cases: usize,
    pub cold_ablation_failures: usize,
    pub veto_cold_rejections: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRetrieval0EvalReport {
    pub scale_rows: [SymbolRetrievalScaleRow; 3],
    pub passing_profiles: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolRetrievalStabilityRow {
    pub stored_patterns: usize,
    pub clusters: usize,
    pub cells: usize,
    pub active_bytes: usize,
    pub noisy_probe_cases: usize,
    pub noisy_hits: usize,
    pub veto_noisy_accepts: usize,
    pub conflict_cases: usize,
    pub veto_conflict_rejections: usize,
    pub cold_ablation_cases: usize,
    pub veto_cold_rejections: usize,
    pub min_noisy_margin: u16,
    pub max_noisy_best_distance: u32,
    pub min_noisy_close_steps: usize,
    pub min_noisy_compared_steps: usize,
    pub first_noisy_rejected: &'static str,
    pub max_cold_margin: u16,
    pub min_cold_best_distance: u32,
    pub max_cold_close_steps: usize,
    pub max_cold_compared_steps: usize,
    pub first_cold_accepted: &'static str,
    pub first_cold_accepted_margin: u16,
    pub first_cold_accepted_close_steps: usize,
    pub first_cold_accepted_compared_steps: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRetrievalStabilityReport {
    pub rows: [SymbolRetrievalStabilityRow; 4],
    pub max_passing_patterns: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolRetrievalCapacityRow {
    pub stored_patterns: usize,
    pub seed_cases: usize,
    pub passing_seeds: usize,
    pub clusters: usize,
    pub cells: usize,
    pub active_bytes: usize,
    pub noisy_probe_cases: usize,
    pub noisy_hits: usize,
    pub veto_noisy_accepts: usize,
    pub conflict_cases: usize,
    pub veto_conflict_rejections: usize,
    pub cold_ablation_cases: usize,
    pub veto_cold_rejections: usize,
    pub min_noisy_margin: u16,
    pub min_trained_gain: u32,
    pub max_noisy_best_distance: u32,
    pub first_noisy_rejected_seed: usize,
    pub first_noisy_rejected_index: usize,
    pub first_noisy_rejected_readout_index: usize,
    pub first_noisy_rejected_margin: u16,
    pub first_noisy_rejected_trained_gain: u32,
    pub first_noisy_rejected_cold_index: usize,
    pub first_noisy_rejected_cold_margin: u16,
    pub first_cold_accepted_seed: usize,
    pub first_cold_accepted_index: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRetrievalCapacityReport {
    pub rows: [SymbolRetrievalCapacityRow; 4],
    pub max_passing_patterns: usize,
    pub mode_status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRetrievalCapacityScaleReport {
    pub rows: [SymbolRetrievalCapacityRow; 3],
    pub stored_patterns: usize,
    pub max_passing_cells: usize,
    pub mode_status: &'static str,
}

impl SymbolRetrieval0EvalReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!(
            concat!(
                "Symbol retrieval-0 eval\n",
                "profile_count: {profile_count}\n",
                "passing_profiles: {passing_profiles}\n"
            ),
            profile_count = self.scale_rows.len(),
            passing_profiles = self.passing_profiles
        );

        for row in self.scale_rows {
            text.push_str(&format!(
                concat!(
                    "profile: {profile}\n",
                    "clusters: {clusters}\n",
                    "cells: {cells}\n",
                    "active_bytes: {active_bytes}\n",
                    "stored_patterns: {stored_patterns}\n",
                    "noisy_probe_cases: {noisy_probe_cases}\n",
                    "noisy_hits: {noisy_hits}\n",
                    "noisy_strong_hits: {noisy_strong_hits}\n",
                    "veto_noisy_accepts: {veto_noisy_accepts}\n",
                    "veto_noisy_close_steps: {veto_noisy_close_steps}\n",
                    "veto_noisy_compared_steps: {veto_noisy_compared_steps}\n",
                    "conflict_cases: {conflict_cases}\n",
                    "conflict_rejections: {conflict_rejections}\n",
                    "veto_conflict_rejections: {veto_conflict_rejections}\n",
                    "veto_conflict_close_steps: {veto_conflict_close_steps}\n",
                    "veto_conflict_compared_steps: {veto_conflict_compared_steps}\n",
                    "cold_ablation_cases: {cold_ablation_cases}\n",
                    "cold_ablation_failures: {cold_ablation_failures}\n",
                    "veto_cold_rejections: {veto_cold_rejections}\n",
                    "profile_status: {mode_status}\n"
                ),
                profile = row.profile,
                clusters = row.clusters,
                cells = row.cells,
                active_bytes = row.active_bytes,
                stored_patterns = row.stored_patterns,
                noisy_probe_cases = row.noisy_probe_cases,
                noisy_hits = row.noisy_hits,
                noisy_strong_hits = row.noisy_strong_hits,
                veto_noisy_accepts = row.veto_noisy_accepts,
                veto_noisy_close_steps = row.veto_noisy_close_steps,
                veto_noisy_compared_steps = row.veto_noisy_compared_steps,
                conflict_cases = row.conflict_cases,
                conflict_rejections = row.conflict_rejections,
                veto_conflict_rejections = row.veto_conflict_rejections,
                veto_conflict_close_steps = row.veto_conflict_close_steps,
                veto_conflict_compared_steps = row.veto_conflict_compared_steps,
                cold_ablation_cases = row.cold_ablation_cases,
                cold_ablation_failures = row.cold_ablation_failures,
                veto_cold_rejections = row.veto_cold_rejections,
                mode_status = row.mode_status
            ));
        }

        text.push_str(&format!("mode_status: {}\n", self.mode_status));
        text
    }
}

impl SymbolRetrievalStabilityReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!(
            concat!(
                "Symbol retrieval stability sweep\n",
                "profile: turbo-256\n",
                "readout: superposition-wave\n",
                "max_passing_patterns: {max_passing_patterns}\n"
            ),
            max_passing_patterns = self.max_passing_patterns
        );

        for row in self.rows {
            text.push_str(&format!(
                concat!(
                    "stored_patterns: {stored_patterns}\n",
                    "clusters: {clusters}\n",
                    "cells: {cells}\n",
                    "active_bytes: {active_bytes}\n",
                    "noisy_probe_cases: {noisy_probe_cases}\n",
                    "noisy_hits: {noisy_hits}\n",
                    "veto_noisy_accepts: {veto_noisy_accepts}\n",
                    "conflict_cases: {conflict_cases}\n",
                    "veto_conflict_rejections: {veto_conflict_rejections}\n",
                    "cold_ablation_cases: {cold_ablation_cases}\n",
                    "veto_cold_rejections: {veto_cold_rejections}\n",
                    "min_noisy_margin: {min_noisy_margin}\n",
                    "max_noisy_best_distance: {max_noisy_best_distance}\n",
                    "min_noisy_close_steps: {min_noisy_close_steps}\n",
                    "min_noisy_compared_steps: {min_noisy_compared_steps}\n",
                    "first_noisy_rejected: {first_noisy_rejected}\n",
                    "max_cold_margin: {max_cold_margin}\n",
                    "min_cold_best_distance: {min_cold_best_distance}\n",
                    "max_cold_close_steps: {max_cold_close_steps}\n",
                    "max_cold_compared_steps: {max_cold_compared_steps}\n",
                    "first_cold_accepted: {first_cold_accepted}\n",
                    "first_cold_accepted_margin: {first_cold_accepted_margin}\n",
                    "first_cold_accepted_close_steps: {first_cold_accepted_close_steps}\n",
                    "first_cold_accepted_compared_steps: {first_cold_accepted_compared_steps}\n",
                    "row_status: {mode_status}\n"
                ),
                stored_patterns = row.stored_patterns,
                clusters = row.clusters,
                cells = row.cells,
                active_bytes = row.active_bytes,
                noisy_probe_cases = row.noisy_probe_cases,
                noisy_hits = row.noisy_hits,
                veto_noisy_accepts = row.veto_noisy_accepts,
                conflict_cases = row.conflict_cases,
                veto_conflict_rejections = row.veto_conflict_rejections,
                cold_ablation_cases = row.cold_ablation_cases,
                veto_cold_rejections = row.veto_cold_rejections,
                min_noisy_margin = row.min_noisy_margin,
                max_noisy_best_distance = row.max_noisy_best_distance,
                min_noisy_close_steps = row.min_noisy_close_steps,
                min_noisy_compared_steps = row.min_noisy_compared_steps,
                first_noisy_rejected = row.first_noisy_rejected,
                max_cold_margin = row.max_cold_margin,
                min_cold_best_distance = row.min_cold_best_distance,
                max_cold_close_steps = row.max_cold_close_steps,
                max_cold_compared_steps = row.max_cold_compared_steps,
                first_cold_accepted = row.first_cold_accepted,
                first_cold_accepted_margin = row.first_cold_accepted_margin,
                first_cold_accepted_close_steps = row.first_cold_accepted_close_steps,
                first_cold_accepted_compared_steps = row.first_cold_accepted_compared_steps,
                mode_status = row.mode_status
            ));
        }

        text.push_str(&format!("mode_status: {}\n", self.mode_status));
        text
    }
}

impl SymbolRetrievalCapacityReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!(
            concat!(
                "Symbol retrieval capacity-1 eval\n",
                "profile: turbo-256\n",
                "readout: superposition-wave\n",
                "seed_cases: {seed_cases}\n",
                "max_passing_patterns: {max_passing_patterns}\n"
            ),
            seed_cases = RETRIEVAL_CAPACITY_SEEDS.len(),
            max_passing_patterns = self.max_passing_patterns
        );

        for row in self.rows {
            text.push_str(&format!(
                concat!(
                    "stored_patterns: {stored_patterns}\n",
                    "passing_seeds: {passing_seeds}/{seed_cases}\n",
                    "clusters: {clusters}\n",
                    "cells: {cells}\n",
                    "active_bytes: {active_bytes}\n",
                    "noisy_probe_cases: {noisy_probe_cases}\n",
                    "noisy_hits: {noisy_hits}\n",
                    "veto_noisy_accepts: {veto_noisy_accepts}\n",
                    "conflict_cases: {conflict_cases}\n",
                    "veto_conflict_rejections: {veto_conflict_rejections}\n",
                    "cold_ablation_cases: {cold_ablation_cases}\n",
                    "veto_cold_rejections: {veto_cold_rejections}\n",
                    "min_noisy_margin: {min_noisy_margin}\n",
                    "min_trained_gain: {min_trained_gain}\n",
                    "max_noisy_best_distance: {max_noisy_best_distance}\n",
                    "first_noisy_rejected_seed: {first_noisy_rejected_seed}\n",
                    "first_noisy_rejected_index: {first_noisy_rejected_index}\n",
                    "first_noisy_rejected_readout_index: {first_noisy_rejected_readout_index}\n",
                    "first_noisy_rejected_margin: {first_noisy_rejected_margin}\n",
                    "first_noisy_rejected_trained_gain: {first_noisy_rejected_trained_gain}\n",
                    "first_noisy_rejected_cold_index: {first_noisy_rejected_cold_index}\n",
                    "first_noisy_rejected_cold_margin: {first_noisy_rejected_cold_margin}\n",
                    "first_cold_accepted_seed: {first_cold_accepted_seed}\n",
                    "first_cold_accepted_index: {first_cold_accepted_index}\n",
                    "row_status: {mode_status}\n"
                ),
                stored_patterns = row.stored_patterns,
                passing_seeds = row.passing_seeds,
                seed_cases = row.seed_cases,
                clusters = row.clusters,
                cells = row.cells,
                active_bytes = row.active_bytes,
                noisy_probe_cases = row.noisy_probe_cases,
                noisy_hits = row.noisy_hits,
                veto_noisy_accepts = row.veto_noisy_accepts,
                conflict_cases = row.conflict_cases,
                veto_conflict_rejections = row.veto_conflict_rejections,
                cold_ablation_cases = row.cold_ablation_cases,
                veto_cold_rejections = row.veto_cold_rejections,
                min_noisy_margin = row.min_noisy_margin,
                min_trained_gain = row.min_trained_gain,
                max_noisy_best_distance = row.max_noisy_best_distance,
                first_noisy_rejected_seed = format_optional_index(row.first_noisy_rejected_seed),
                first_noisy_rejected_index = format_optional_index(row.first_noisy_rejected_index),
                first_noisy_rejected_readout_index =
                    format_optional_index(row.first_noisy_rejected_readout_index),
                first_noisy_rejected_margin = row.first_noisy_rejected_margin,
                first_noisy_rejected_trained_gain = row.first_noisy_rejected_trained_gain,
                first_noisy_rejected_cold_index =
                    format_optional_index(row.first_noisy_rejected_cold_index),
                first_noisy_rejected_cold_margin = row.first_noisy_rejected_cold_margin,
                first_cold_accepted_seed = format_optional_index(row.first_cold_accepted_seed),
                first_cold_accepted_index = format_optional_index(row.first_cold_accepted_index),
                mode_status = row.mode_status
            ));
        }

        text.push_str(&format!("mode_status: {}\n", self.mode_status));
        text
    }
}

impl SymbolRetrievalCapacityScaleReport {
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut text = format!(
            concat!(
                "Symbol retrieval capacity-scale eval\n",
                "readout: superposition-wave\n",
                "stored_patterns: {stored_patterns}\n",
                "seed_cases: {seed_cases}\n",
                "max_passing_cells: {max_passing_cells}\n"
            ),
            stored_patterns = self.stored_patterns,
            seed_cases = RETRIEVAL_CAPACITY_SEEDS.len(),
            max_passing_cells = self.max_passing_cells
        );

        for row in self.rows {
            text.push_str(&format!(
                concat!(
                    "clusters: {clusters}\n",
                    "cells: {cells}\n",
                    "active_bytes: {active_bytes}\n",
                    "passing_seeds: {passing_seeds}/{seed_cases}\n",
                    "noisy_probe_cases: {noisy_probe_cases}\n",
                    "noisy_hits: {noisy_hits}\n",
                    "veto_noisy_accepts: {veto_noisy_accepts}\n",
                    "conflict_cases: {conflict_cases}\n",
                    "veto_conflict_rejections: {veto_conflict_rejections}\n",
                    "cold_ablation_cases: {cold_ablation_cases}\n",
                    "veto_cold_rejections: {veto_cold_rejections}\n",
                    "min_noisy_margin: {min_noisy_margin}\n",
                    "min_trained_gain: {min_trained_gain}\n",
                    "max_noisy_best_distance: {max_noisy_best_distance}\n",
                    "first_noisy_rejected_seed: {first_noisy_rejected_seed}\n",
                    "first_noisy_rejected_index: {first_noisy_rejected_index}\n",
                    "first_noisy_rejected_readout_index: {first_noisy_rejected_readout_index}\n",
                    "first_noisy_rejected_margin: {first_noisy_rejected_margin}\n",
                    "first_noisy_rejected_trained_gain: {first_noisy_rejected_trained_gain}\n",
                    "first_noisy_rejected_cold_index: {first_noisy_rejected_cold_index}\n",
                    "first_noisy_rejected_cold_margin: {first_noisy_rejected_cold_margin}\n",
                    "first_cold_accepted_seed: {first_cold_accepted_seed}\n",
                    "first_cold_accepted_index: {first_cold_accepted_index}\n",
                    "row_status: {mode_status}\n"
                ),
                clusters = row.clusters,
                cells = row.cells,
                active_bytes = row.active_bytes,
                passing_seeds = row.passing_seeds,
                seed_cases = row.seed_cases,
                noisy_probe_cases = row.noisy_probe_cases,
                noisy_hits = row.noisy_hits,
                veto_noisy_accepts = row.veto_noisy_accepts,
                conflict_cases = row.conflict_cases,
                veto_conflict_rejections = row.veto_conflict_rejections,
                cold_ablation_cases = row.cold_ablation_cases,
                veto_cold_rejections = row.veto_cold_rejections,
                min_noisy_margin = row.min_noisy_margin,
                min_trained_gain = row.min_trained_gain,
                max_noisy_best_distance = row.max_noisy_best_distance,
                first_noisy_rejected_seed = format_optional_index(row.first_noisy_rejected_seed),
                first_noisy_rejected_index = format_optional_index(row.first_noisy_rejected_index),
                first_noisy_rejected_readout_index =
                    format_optional_index(row.first_noisy_rejected_readout_index),
                first_noisy_rejected_margin = row.first_noisy_rejected_margin,
                first_noisy_rejected_trained_gain = row.first_noisy_rejected_trained_gain,
                first_noisy_rejected_cold_index =
                    format_optional_index(row.first_noisy_rejected_cold_index),
                first_noisy_rejected_cold_margin = row.first_noisy_rejected_cold_margin,
                first_cold_accepted_seed = format_optional_index(row.first_cold_accepted_seed),
                first_cold_accepted_index = format_optional_index(row.first_cold_accepted_index),
                mode_status = row.mode_status
            ));
        }

        text.push_str(&format!("mode_status: {}\n", self.mode_status));
        text
    }
}

fn format_optional_index(index: usize) -> String {
    if index == usize::MAX {
        "-".to_string()
    } else {
        index.to_string()
    }
}

#[must_use]
pub fn symbol_retrieval0_eval() -> SymbolRetrieval0EvalReport {
    let scale_rows = RETRIEVAL_SCALE_CLUSTERS.map(retrieval_scale_row);
    let passing_profiles = scale_rows
        .iter()
        .filter(|row| row.mode_status == "symbol-retrieval0-profile-pass")
        .count();
    let turbo_passes = scale_rows.iter().any(|row| {
        row.profile == "turbo-256" && row.mode_status == "symbol-retrieval0-profile-pass"
    });
    let mode_status = if turbo_passes {
        "symbol-retrieval0-eval-pass"
    } else {
        "symbol-retrieval0-eval-watch"
    };

    SymbolRetrieval0EvalReport {
        scale_rows,
        passing_profiles,
        mode_status,
    }
}

#[must_use]
pub fn symbol_retrieval_stability_sweep() -> SymbolRetrievalStabilityReport {
    let rows = RETRIEVAL_STABILITY_COUNTS.map(retrieval_stability_row);
    let max_passing_patterns = rows
        .iter()
        .filter(|row| row.mode_status == "symbol-retrieval-stability-pass")
        .map(|row| row.stored_patterns)
        .max()
        .unwrap_or(0);
    let mode_status = if max_passing_patterns >= RETRIEVAL_PATTERNS.len() {
        "symbol-retrieval-stability-eval-pass"
    } else {
        "symbol-retrieval-stability-eval-watch"
    };

    SymbolRetrievalStabilityReport {
        rows,
        max_passing_patterns,
        mode_status,
    }
}

#[must_use]
pub fn symbol_retrieval_capacity_eval() -> SymbolRetrievalCapacityReport {
    let rows = RETRIEVAL_CAPACITY_COUNTS.map(retrieval_capacity_row);
    let max_passing_patterns = rows
        .iter()
        .filter(|row| row.mode_status == "symbol-retrieval-capacity-pass")
        .map(|row| row.stored_patterns)
        .max()
        .unwrap_or(0);
    let mode_status = if max_passing_patterns >= RETRIEVAL_CAPACITY_COUNTS[0] {
        "symbol-retrieval-capacity-eval-pass"
    } else {
        "symbol-retrieval-capacity-eval-watch"
    };

    SymbolRetrievalCapacityReport {
        rows,
        max_passing_patterns,
        mode_status,
    }
}

#[must_use]
pub fn symbol_retrieval_capacity_scale_eval() -> SymbolRetrievalCapacityScaleReport {
    let rows = RETRIEVAL_CAPACITY_SCALE_CLUSTERS.map(|clusters| {
        retrieval_capacity_row_with_clusters(RETRIEVAL_CAPACITY_SCALE_PATTERNS, clusters)
    });
    let max_passing_cells = rows
        .iter()
        .filter(|row| row.mode_status == "symbol-retrieval-capacity-pass")
        .map(|row| row.cells)
        .max()
        .unwrap_or(0);
    let mode_status = if max_passing_cells > 0 {
        "symbol-retrieval-capacity-scale-eval-pass"
    } else {
        "symbol-retrieval-capacity-scale-eval-watch"
    };

    SymbolRetrievalCapacityScaleReport {
        rows,
        stored_patterns: RETRIEVAL_CAPACITY_SCALE_PATTERNS,
        max_passing_cells,
        mode_status,
    }
}

fn retrieval_scale_row(clusters: usize) -> SymbolRetrievalScaleRow {
    let seed = 0x5EED_5000 + clusters as u64;
    let trained = train_pattern_set(seed, clusters, &RETRIEVAL_PATTERNS);
    let prototypes = build_prototypes(&trained, &RETRIEVAL_PATTERNS);

    let mut noisy_hits = 0usize;
    let mut noisy_strong_hits = 0usize;
    let mut veto_noisy_accepts = 0usize;
    let mut veto_noisy_close_steps = 0usize;
    let mut veto_noisy_compared_steps = 0usize;
    let mut cold_ablation_failures = 0usize;
    let mut veto_cold_rejections = 0usize;

    for (target_index, pattern) in RETRIEVAL_PATTERNS.iter().enumerate() {
        let query = run_query_trace(trained.clone(), pattern.noisy_probe);
        let match_result = nearest_prototype(query.center, &prototypes);
        let veto = retrieval_veto(pattern.noisy_probe, &query, &prototypes);
        noisy_hits += usize::from(match_result.index == target_index);
        noisy_strong_hits +=
            usize::from(match_result.index == target_index && match_result.margin >= STRONG_MARGIN);
        veto_noisy_accepts += usize::from(veto.accepted && veto.index == target_index);
        veto_noisy_close_steps += veto.close_steps;
        veto_noisy_compared_steps += veto.compared_steps;

        let cold_query = run_query_trace(
            SymbolL3Organism::with_clusters(seed, clusters),
            pattern.noisy_probe,
        );
        let cold_result = nearest_prototype(cold_query.center, &prototypes);
        let cold_veto = retrieval_veto(pattern.noisy_probe, &cold_query, &prototypes);
        cold_ablation_failures +=
            usize::from(cold_result.index != target_index || cold_result.margin < STRONG_MARGIN);
        veto_cold_rejections += usize::from(!cold_veto.accepted);
    }

    let mut conflict_rejections = 0usize;
    let mut veto_conflict_rejections = 0usize;
    let mut veto_conflict_close_steps = 0usize;
    let mut veto_conflict_compared_steps = 0usize;
    for probe in CONFLICT_PROBES {
        let query = run_query_trace(trained.clone(), probe);
        let match_result = nearest_prototype(query.center, &prototypes);
        let veto = retrieval_veto(probe, &query, &prototypes);
        conflict_rejections += usize::from(match_result.margin < STRONG_MARGIN);
        veto_conflict_rejections += usize::from(!veto.accepted);
        veto_conflict_close_steps += veto.close_steps;
        veto_conflict_compared_steps += veto.compared_steps;
    }

    let cells = clusters * SYMBOL_WAVE_CLUSTER_CELLS;
    let mode_status = if veto_noisy_accepts == RETRIEVAL_PATTERNS.len()
        && veto_conflict_rejections == CONFLICT_PROBES.len()
        && veto_cold_rejections == RETRIEVAL_PATTERNS.len()
    {
        "symbol-retrieval0-profile-pass"
    } else {
        "symbol-retrieval0-profile-watch"
    };

    SymbolRetrievalScaleRow {
        profile: profile_name(clusters),
        clusters,
        cells,
        active_bytes: cells * SYMBOL_CELL8_BYTES,
        stored_patterns: RETRIEVAL_PATTERNS.len(),
        noisy_probe_cases: RETRIEVAL_PATTERNS.len(),
        noisy_hits,
        noisy_strong_hits,
        veto_noisy_accepts,
        veto_noisy_close_steps,
        veto_noisy_compared_steps,
        conflict_cases: CONFLICT_PROBES.len(),
        conflict_rejections,
        veto_conflict_rejections,
        veto_conflict_close_steps,
        veto_conflict_compared_steps,
        cold_ablation_cases: RETRIEVAL_PATTERNS.len(),
        cold_ablation_failures,
        veto_cold_rejections,
        mode_status,
    }
}

fn retrieval_stability_row(stored_patterns: usize) -> SymbolRetrievalStabilityRow {
    let patterns = &RETRIEVAL_STABILITY_PATTERNS[..stored_patterns];
    let seed = 0x5EED_5000 + RETRIEVAL_TURBO_CLUSTERS as u64;
    let trained = train_pattern_set(seed, RETRIEVAL_TURBO_CLUSTERS, patterns);
    let prototypes = build_prototypes(&trained, patterns);

    let mut noisy_hits = 0usize;
    let mut veto_noisy_accepts = 0usize;
    let mut veto_cold_rejections = 0usize;
    let mut min_noisy_margin = u16::MAX;
    let mut max_noisy_best_distance = 0u32;
    let mut min_noisy_close_steps = usize::MAX;
    let mut min_noisy_compared_steps = 0usize;
    let mut first_noisy_rejected = "-";
    let mut max_cold_margin = 0u16;
    let mut min_cold_best_distance = u32::MAX;
    let mut max_cold_close_steps = 0usize;
    let mut max_cold_compared_steps = 0usize;
    let mut first_cold_accepted = "-";
    let mut first_cold_accepted_margin = 0u16;
    let mut first_cold_accepted_close_steps = 0usize;
    let mut first_cold_accepted_compared_steps = 0usize;
    for (target_index, pattern) in patterns.iter().enumerate() {
        let query = run_query_trace(trained.clone(), pattern.noisy_probe);
        let readout = superposition_retrieval_veto(pattern.noisy_probe, &query, &prototypes);
        noisy_hits += usize::from(readout.index == target_index);
        veto_noisy_accepts += usize::from(readout.accepted && readout.index == target_index);
        min_noisy_margin = min_noisy_margin.min(readout.margin);
        max_noisy_best_distance = max_noisy_best_distance.max(readout.best_distance);
        if readout.close_steps < min_noisy_close_steps {
            min_noisy_close_steps = readout.close_steps;
            min_noisy_compared_steps = readout.compared_steps;
        }
        if !readout.accepted && first_noisy_rejected == "-" {
            first_noisy_rejected = pattern.name;
        }

        let cold_organism = SymbolL3Organism::with_clusters(seed, RETRIEVAL_TURBO_CLUSTERS);
        let cold_query = run_query_trace(cold_organism, pattern.noisy_probe);
        let cold_readout =
            superposition_retrieval_veto(pattern.noisy_probe, &cold_query, &prototypes);
        if cold_readout.margin > max_cold_margin {
            max_cold_margin = cold_readout.margin;
            max_cold_close_steps = cold_readout.close_steps;
            max_cold_compared_steps = cold_readout.compared_steps;
        }
        min_cold_best_distance = min_cold_best_distance.min(cold_readout.best_distance);
        if cold_readout.accepted && first_cold_accepted == "-" {
            first_cold_accepted = pattern.name;
            first_cold_accepted_margin = cold_readout.margin;
            first_cold_accepted_close_steps = cold_readout.close_steps;
            first_cold_accepted_compared_steps = cold_readout.compared_steps;
        }
        veto_cold_rejections += usize::from(!cold_readout.accepted);
    }

    let mut veto_conflict_rejections = 0usize;
    for probe in CONFLICT_PROBES {
        let query = run_query_trace(trained.clone(), probe);
        let readout = superposition_retrieval_veto(probe, &query, &prototypes);
        veto_conflict_rejections += usize::from(!readout.accepted);
    }

    let cells = RETRIEVAL_TURBO_CLUSTERS * SYMBOL_WAVE_CLUSTER_CELLS;
    let mode_status = if veto_noisy_accepts == patterns.len()
        && veto_conflict_rejections == CONFLICT_PROBES.len()
        && veto_cold_rejections == patterns.len()
    {
        "symbol-retrieval-stability-pass"
    } else {
        "symbol-retrieval-stability-watch"
    };

    SymbolRetrievalStabilityRow {
        stored_patterns,
        clusters: RETRIEVAL_TURBO_CLUSTERS,
        cells,
        active_bytes: cells * SYMBOL_CELL8_BYTES,
        noisy_probe_cases: patterns.len(),
        noisy_hits,
        veto_noisy_accepts,
        conflict_cases: CONFLICT_PROBES.len(),
        veto_conflict_rejections,
        cold_ablation_cases: patterns.len(),
        veto_cold_rejections,
        min_noisy_margin,
        max_noisy_best_distance,
        min_noisy_close_steps,
        min_noisy_compared_steps,
        first_noisy_rejected,
        max_cold_margin,
        min_cold_best_distance,
        max_cold_close_steps,
        max_cold_compared_steps,
        first_cold_accepted,
        first_cold_accepted_margin,
        first_cold_accepted_close_steps,
        first_cold_accepted_compared_steps,
        mode_status,
    }
}

fn retrieval_capacity_row(stored_patterns: usize) -> SymbolRetrievalCapacityRow {
    retrieval_capacity_row_with_clusters(stored_patterns, RETRIEVAL_TURBO_CLUSTERS)
}

fn retrieval_capacity_row_with_clusters(
    stored_patterns: usize,
    clusters: usize,
) -> SymbolRetrievalCapacityRow {
    let patterns = build_capacity_patterns(stored_patterns);
    let conflict_probes = build_capacity_conflict_probes(&patterns);
    let mut passing_seeds = 0usize;
    let mut noisy_hits = 0usize;
    let mut veto_noisy_accepts = 0usize;
    let mut veto_conflict_rejections = 0usize;
    let mut veto_cold_rejections = 0usize;
    let mut min_noisy_margin = u16::MAX;
    let mut min_trained_gain = u32::MAX;
    let mut max_noisy_best_distance = 0u32;
    let mut first_noisy_rejected_seed = usize::MAX;
    let mut first_noisy_rejected_index = usize::MAX;
    let mut first_noisy_rejected_readout_index = usize::MAX;
    let mut first_noisy_rejected_margin = 0u16;
    let mut first_noisy_rejected_trained_gain = 0u32;
    let mut first_noisy_rejected_cold_index = usize::MAX;
    let mut first_noisy_rejected_cold_margin = 0u16;
    let mut first_cold_accepted_seed = usize::MAX;
    let mut first_cold_accepted_index = usize::MAX;

    for (seed_index, seed) in RETRIEVAL_CAPACITY_SEEDS.iter().copied().enumerate() {
        let trained = train_capacity_pattern_set(seed, clusters, &patterns);
        let prototypes = build_capacity_prototypes(&trained, &patterns);
        let cold_organism = SymbolL3Organism::with_clusters(seed, clusters);
        let mut seed_noisy_accepts = 0usize;
        let mut seed_conflict_rejections = 0usize;
        let mut seed_cold_rejections = 0usize;

        for (target_index, pattern) in patterns.iter().enumerate() {
            let query = run_query_trace(trained.clone(), &pattern.noisy_probe);
            let readout =
                superposition_capacity_retrieval_veto(&pattern.noisy_probe, &query, &prototypes);
            let cold_query = run_query_trace(cold_organism.clone(), &pattern.noisy_probe);
            let cold_readout = superposition_capacity_retrieval_veto(
                &pattern.noisy_probe,
                &cold_query,
                &prototypes,
            );
            let trained_gain = cold_readout
                .best_distance
                .saturating_sub(readout.best_distance);
            let trained_resonance =
                !cold_readout.accepted || trained_gain >= TRAINED_RESONANCE_GAIN_MIN;
            let accepted_target = readout.index == target_index
                && readout.margin >= SUPERPOSITION_STRONG_MARGIN
                && trained_resonance;
            noisy_hits += usize::from(readout.index == target_index);
            veto_noisy_accepts += usize::from(accepted_target);
            seed_noisy_accepts += usize::from(accepted_target);
            min_noisy_margin = min_noisy_margin.min(readout.margin);
            min_trained_gain = min_trained_gain.min(trained_gain);
            max_noisy_best_distance = max_noisy_best_distance.max(readout.best_distance);
            if !accepted_target && first_noisy_rejected_seed == usize::MAX {
                first_noisy_rejected_seed = seed_index;
                first_noisy_rejected_index = target_index;
                first_noisy_rejected_readout_index = readout.index;
                first_noisy_rejected_margin = readout.margin;
                first_noisy_rejected_trained_gain = trained_gain;
                first_noisy_rejected_cold_index = cold_readout.index;
                first_noisy_rejected_cold_margin = cold_readout.margin;
            }

            let cold_rejected = trained_resonance;
            veto_cold_rejections += usize::from(cold_rejected);
            seed_cold_rejections += usize::from(cold_rejected);
            if !cold_rejected && first_cold_accepted_seed == usize::MAX {
                first_cold_accepted_seed = seed_index;
                first_cold_accepted_index = target_index;
            }
        }

        for probe in &conflict_probes {
            let query = run_query_trace(trained.clone(), probe);
            let readout = superposition_capacity_retrieval_veto(probe, &query, &prototypes);
            let rejected = !readout.accepted;
            veto_conflict_rejections += usize::from(rejected);
            seed_conflict_rejections += usize::from(rejected);
        }

        passing_seeds += usize::from(
            seed_noisy_accepts == patterns.len()
                && seed_conflict_rejections == conflict_probes.len()
                && seed_cold_rejections == patterns.len(),
        );
    }

    let seed_cases = RETRIEVAL_CAPACITY_SEEDS.len();
    let cells = clusters * SYMBOL_WAVE_CLUSTER_CELLS;
    let mode_status = if passing_seeds == seed_cases {
        "symbol-retrieval-capacity-pass"
    } else {
        "symbol-retrieval-capacity-watch"
    };

    SymbolRetrievalCapacityRow {
        stored_patterns,
        seed_cases,
        passing_seeds,
        clusters,
        cells,
        active_bytes: cells * SYMBOL_CELL8_BYTES,
        noisy_probe_cases: patterns.len() * seed_cases,
        noisy_hits,
        veto_noisy_accepts,
        conflict_cases: conflict_probes.len() * seed_cases,
        veto_conflict_rejections,
        cold_ablation_cases: patterns.len() * seed_cases,
        veto_cold_rejections,
        min_noisy_margin,
        min_trained_gain,
        max_noisy_best_distance,
        first_noisy_rejected_seed,
        first_noisy_rejected_index,
        first_noisy_rejected_readout_index,
        first_noisy_rejected_margin,
        first_noisy_rejected_trained_gain,
        first_noisy_rejected_cold_index,
        first_noisy_rejected_cold_margin,
        first_cold_accepted_seed,
        first_cold_accepted_index,
        mode_status,
    }
}

fn train_pattern_set(
    seed: u64,
    clusters: usize,
    patterns: &[RetrievalPattern],
) -> SymbolL3Organism {
    let mut organism = SymbolL3Organism::with_clusters(seed, clusters);
    for _ in 0..RETRIEVAL_REPEATS {
        for pattern in patterns {
            for symbol in pattern.pattern.chars() {
                let _ = organism.tick_symbol(symbol);
            }
            let _ = organism.tick_symbol('|');
        }
    }
    organism
}

fn train_capacity_pattern_set(
    seed: u64,
    clusters: usize,
    patterns: &[CapacityPattern],
) -> SymbolL3Organism {
    let mut organism = SymbolL3Organism::with_clusters(seed, clusters);
    for _ in 0..RETRIEVAL_CAPACITY_REPEATS {
        for pattern in patterns {
            for symbol in pattern.pattern.chars() {
                let _ = organism.tick_symbol(symbol);
            }
            let _ = organism.tick_symbol('|');
        }
    }
    organism
}

fn build_prototypes(organism: &SymbolL3Organism, patterns: &[RetrievalPattern]) -> Vec<Prototype> {
    patterns
        .iter()
        .map(|pattern| {
            let trace = run_query_trace(organism.clone(), pattern.pattern);
            Prototype {
                name: pattern.name,
                pattern: pattern.pattern,
                center: trace.center,
                path: trace.path,
            }
        })
        .collect()
}

fn build_capacity_prototypes(
    organism: &SymbolL3Organism,
    patterns: &[CapacityPattern],
) -> Vec<CapacityPrototype> {
    patterns
        .iter()
        .map(|pattern| {
            let trace = run_query_trace(organism.clone(), &pattern.pattern);
            CapacityPrototype {
                pattern: pattern.pattern.clone(),
                center: trace.center,
                path: trace.path,
            }
        })
        .collect()
}

fn run_query_trace(mut organism: SymbolL3Organism, query: &str) -> QueryTrace {
    let mut center = organism.last_center();
    let mut path = Vec::with_capacity(query.chars().count());
    for symbol in query.chars() {
        center = organism.tick_symbol(symbol).center;
        path.push(center);
    }
    QueryTrace { center, path }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatchResult {
    index: usize,
    distance: u16,
    margin: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VetoReadout {
    index: usize,
    accepted: bool,
    close_steps: usize,
    compared_steps: usize,
    margin: u16,
    best_distance: u32,
}

fn nearest_prototype(center: SymbolL3Center, prototypes: &[Prototype]) -> MatchResult {
    let mut best_index = 0usize;
    let mut best_distance = u16::MAX;
    let mut second_distance = u16::MAX;

    for (index, prototype) in prototypes.iter().enumerate() {
        let distance = center_distance(center, prototype.center);
        if distance < best_distance {
            second_distance = best_distance;
            best_distance = distance;
            best_index = index;
        } else if distance < second_distance {
            second_distance = distance;
        }
    }

    MatchResult {
        index: best_index,
        distance: best_distance,
        margin: second_distance.saturating_sub(best_distance),
    }
}

fn center_distance(left: SymbolL3Center, right: SymbolL3Center) -> u16 {
    circular_slot_delta(left.peak_slot, right.peak_slot)
        .saturating_mul(4)
        .saturating_add(phase_delta(left.carrier_phase, right.carrier_phase))
        .saturating_add(left.coherence.abs_diff(right.coherence) / 16)
}

fn retrieval_veto(query: &str, trace: &QueryTrace, prototypes: &[Prototype]) -> VetoReadout {
    let nearest = nearest_prototype(trace.center, prototypes);
    let prototype = &prototypes[nearest.index];
    let mut compared_steps = 0usize;
    let mut close_steps = 0usize;

    for (index, symbol) in query.chars().enumerate() {
        if symbol == '?' {
            continue;
        }
        let Some(query_center) = trace.path.get(index) else {
            continue;
        };
        let Some(prototype_center) = prototype.path.get(index) else {
            continue;
        };

        compared_steps += 1;
        close_steps +=
            usize::from(center_distance(*query_center, *prototype_center) <= TRAJECTORY_STEP_MAX);
    }

    let accepted = nearest.margin >= STRONG_MARGIN
        && compared_steps >= 2
        && close_steps.saturating_mul(2) >= compared_steps;

    VetoReadout {
        index: nearest.index,
        accepted,
        close_steps,
        compared_steps,
        margin: nearest.margin,
        best_distance: u32::from(nearest.distance),
    }
}

fn superposition_retrieval_veto(
    query: &str,
    trace: &QueryTrace,
    prototypes: &[Prototype],
) -> VetoReadout {
    let mut best_index = 0usize;
    let mut best_distance = u32::MAX;
    let mut second_distance = u32::MAX;

    for (index, prototype) in prototypes.iter().enumerate() {
        let distance = superposition_distance(query, trace, prototype);
        if distance < best_distance {
            second_distance = best_distance;
            best_distance = distance;
            best_index = index;
        } else if distance < second_distance {
            second_distance = distance;
        }
    }

    let prototype = &prototypes[best_index];
    let (close_steps, compared_steps) = trajectory_closeness(query, trace, prototype);
    let projection_mismatches = projection_mismatch_count(query, prototype.pattern);
    let margin = second_distance
        .saturating_sub(best_distance)
        .min(u32::from(u16::MAX)) as u16;
    let accepted = margin >= SUPERPOSITION_STRONG_MARGIN
        && projection_mismatches == 0
        && compared_steps >= 2
        && close_steps.saturating_mul(2) >= compared_steps;

    VetoReadout {
        index: best_index,
        accepted,
        close_steps,
        compared_steps,
        margin,
        best_distance,
    }
}

fn superposition_capacity_retrieval_veto(
    query: &str,
    trace: &QueryTrace,
    prototypes: &[CapacityPrototype],
) -> VetoReadout {
    let mut best_index = 0usize;
    let mut best_distance = u32::MAX;
    let mut second_distance = u32::MAX;

    for (index, prototype) in prototypes.iter().enumerate() {
        let distance = superposition_capacity_distance(query, trace, prototype);
        if distance < best_distance {
            second_distance = best_distance;
            best_distance = distance;
            best_index = index;
        } else if distance < second_distance {
            second_distance = distance;
        }
    }

    let prototype = &prototypes[best_index];
    let (close_steps, compared_steps) = capacity_trajectory_closeness(query, trace, prototype);
    let projection_mismatches = projection_mismatch_count(query, &prototype.pattern);
    let margin = second_distance
        .saturating_sub(best_distance)
        .min(u32::from(u16::MAX)) as u16;
    let accepted = margin >= SUPERPOSITION_STRONG_MARGIN
        && projection_mismatches == 0
        && compared_steps >= 2
        && close_steps.saturating_mul(2) >= compared_steps;

    VetoReadout {
        index: best_index,
        accepted,
        close_steps,
        compared_steps,
        margin,
        best_distance,
    }
}

fn superposition_distance(query: &str, trace: &QueryTrace, prototype: &Prototype) -> u32 {
    let mut distance = center_shape_distance(trace.center, prototype.center).saturating_mul(2);
    distance = distance.saturating_add(
        u32::try_from(projection_mismatch_count(query, prototype.pattern))
            .unwrap_or(u32::MAX)
            .saturating_mul(PROJECTION_MISMATCH_PENALTY),
    );

    for (index, symbol) in query.chars().enumerate() {
        if symbol == '?' {
            continue;
        }
        let Some(query_center) = trace.path.get(index) else {
            continue;
        };
        let Some(prototype_center) = prototype.path.get(index) else {
            continue;
        };

        distance = distance.saturating_add(center_shape_distance(*query_center, *prototype_center));
    }

    distance
}

fn superposition_capacity_distance(
    query: &str,
    trace: &QueryTrace,
    prototype: &CapacityPrototype,
) -> u32 {
    let mut distance = center_shape_distance(trace.center, prototype.center).saturating_mul(2);
    distance = distance.saturating_add(
        u32::try_from(projection_mismatch_count(query, &prototype.pattern))
            .unwrap_or(u32::MAX)
            .saturating_mul(PROJECTION_MISMATCH_PENALTY),
    );

    for (index, symbol) in query.chars().enumerate() {
        if symbol == '?' {
            continue;
        }
        let Some(query_center) = trace.path.get(index) else {
            continue;
        };
        let Some(prototype_center) = prototype.path.get(index) else {
            continue;
        };

        distance = distance.saturating_add(center_shape_distance(*query_center, *prototype_center));
    }

    distance
}

fn projection_mismatch_count(query: &str, pattern: &str) -> usize {
    let mut mismatches = query.chars().count().abs_diff(pattern.chars().count());
    for (query_symbol, pattern_symbol) in query.chars().zip(pattern.chars()) {
        if query_symbol != '?' && query_symbol != pattern_symbol {
            mismatches = mismatches.saturating_add(1);
        }
    }
    mismatches
}

fn center_shape_distance(left: SymbolL3Center, right: SymbolL3Center) -> u32 {
    u32::from(center_distance(left, right))
        .saturating_add((left.energy.abs_diff(right.energy) / 512).min(255) as u32)
        .saturating_add(u32::from(left.support_cells.abs_diff(right.support_cells)))
        .saturating_add(u32::from(
            left.supported_clusters
                .abs_diff(right.supported_clusters)
                .saturating_mul(4),
        ))
        .saturating_add(u32::from(
            left.accepted_clusters
                .abs_diff(right.accepted_clusters)
                .saturating_mul(6),
        ))
        .saturating_add(u32::from(
            left.reflected_clusters
                .abs_diff(right.reflected_clusters)
                .saturating_mul(3),
        ))
        .saturating_add(u32::from(
            left.spurious_clusters
                .abs_diff(right.spurious_clusters)
                .saturating_mul(3),
        ))
}

fn trajectory_closeness(query: &str, trace: &QueryTrace, prototype: &Prototype) -> (usize, usize) {
    let mut compared_steps = 0usize;
    let mut close_steps = 0usize;

    for (index, symbol) in query.chars().enumerate() {
        if symbol == '?' {
            continue;
        }
        let Some(query_center) = trace.path.get(index) else {
            continue;
        };
        let Some(prototype_center) = prototype.path.get(index) else {
            continue;
        };

        compared_steps += 1;
        close_steps +=
            usize::from(center_distance(*query_center, *prototype_center) <= TRAJECTORY_STEP_MAX);
    }

    (close_steps, compared_steps)
}

fn capacity_trajectory_closeness(
    query: &str,
    trace: &QueryTrace,
    prototype: &CapacityPrototype,
) -> (usize, usize) {
    let mut compared_steps = 0usize;
    let mut close_steps = 0usize;

    for (index, symbol) in query.chars().enumerate() {
        if symbol == '?' {
            continue;
        }
        let Some(query_center) = trace.path.get(index) else {
            continue;
        };
        let Some(prototype_center) = prototype.path.get(index) else {
            continue;
        };

        compared_steps += 1;
        close_steps +=
            usize::from(center_distance(*query_center, *prototype_center) <= TRAJECTORY_STEP_MAX);
    }

    (close_steps, compared_steps)
}

fn build_capacity_patterns(count: usize) -> Vec<CapacityPattern> {
    let mut patterns: Vec<CapacityPattern> = Vec::with_capacity(count);
    let mut candidate = 0u64;
    while patterns.len() < count {
        let pattern = capacity_pattern(candidate);
        candidate = candidate.saturating_add(1);
        if patterns.iter().all(|stored| {
            stored.pattern != pattern.pattern
                && visible_probe_distance(&stored.noisy_probe, &pattern.noisy_probe) >= 3
        }) {
            patterns.push(pattern);
        }
    }
    patterns
}

fn capacity_pattern(index: u64) -> CapacityPattern {
    const SYMBOLS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut state = mix_capacity_index(index);
    let mut pattern = String::with_capacity(6);
    for position in 0..6 {
        state =
            mix_capacity_index(state ^ (position as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let symbol = SYMBOLS[(state as usize) % SYMBOLS.len()] as char;
        pattern.push(symbol);
    }
    let mut probe: Vec<char> = pattern.chars().collect();
    let hole_index = 1 + ((index as usize * 5) % 4);
    probe[hole_index] = '?';
    let noisy_probe = probe.into_iter().collect();

    CapacityPattern {
        pattern,
        noisy_probe,
    }
}

fn mix_capacity_index(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn visible_probe_distance(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .filter(|(left_symbol, right_symbol)| {
            *left_symbol != '?' && *right_symbol != '?' && left_symbol != right_symbol
        })
        .count()
}

fn build_capacity_conflict_probes(patterns: &[CapacityPattern]) -> Vec<String> {
    let conflict_cases = RETRIEVAL_CAPACITY_CONFLICTS.min(patterns.len());
    (0..conflict_cases)
        .map(|case| {
            let pattern = &patterns[(case * 7) % patterns.len()].pattern;
            let mut chars: Vec<char> = pattern.chars().collect();
            let index = (case * 5 + 1) % chars.len();
            chars[index] = next_conflict_symbol(chars[index]);
            chars.into_iter().collect()
        })
        .collect()
}

fn next_conflict_symbol(symbol: char) -> char {
    match symbol {
        'A'..='Y' => char::from_u32(symbol as u32 + 1).unwrap_or('Z'),
        'Z' => 'A',
        _ => 'X',
    }
}

fn circular_slot_delta(left: u16, right: u16) -> u16 {
    let direct = left.abs_diff(right);
    direct.min(128 - direct.min(128))
}

fn phase_delta(left: i8, right: i8) -> u16 {
    i16::from(left).abs_diff(i16::from(right))
}

fn profile_name(clusters: usize) -> &'static str {
    match clusters {
        1 => "cluster-16",
        16 => "turbo-256",
        32 => "default-512",
        _ => "custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_retrieval0_report_has_required_gates() {
        let report = symbol_retrieval0_eval();

        assert_eq!(report.scale_rows.len(), 3);
        assert_eq!(report.scale_rows[0].cells, 16);
        assert_eq!(report.scale_rows[1].cells, 256);
        assert_eq!(report.scale_rows[2].cells, 512);
        for row in report.scale_rows {
            assert_eq!(row.stored_patterns, 4);
            assert_eq!(row.noisy_probe_cases, 4);
            assert_eq!(row.conflict_cases, 4);
            assert_eq!(row.cold_ablation_cases, 4);
            assert!(row.noisy_hits <= row.noisy_probe_cases);
            assert!(row.noisy_strong_hits <= row.noisy_hits);
            assert!(row.veto_noisy_accepts <= row.noisy_probe_cases);
            assert!(row.veto_noisy_close_steps <= row.veto_noisy_compared_steps);
            assert!(row.conflict_rejections <= row.conflict_cases);
            assert!(row.veto_conflict_rejections <= row.conflict_cases);
            assert!(row.veto_conflict_close_steps <= row.veto_conflict_compared_steps);
            assert!(row.cold_ablation_failures <= row.cold_ablation_cases);
            assert!(row.veto_cold_rejections <= row.cold_ablation_cases);
        }

        let text = report.to_text();
        assert!(text.contains("Symbol retrieval-0 eval"));
        assert!(text.contains("profile: turbo-256"));
        assert!(text.contains("profile: default-512"));
        assert!(text.contains("mode_status: symbol-retrieval0"));
    }

    #[test]
    fn symbol_retrieval_stability_sweep_has_required_rows() {
        let report = symbol_retrieval_stability_sweep();

        assert_eq!(report.rows.len(), 4);
        assert!(report.max_passing_patterns >= 32);
        assert_eq!(
            report.rows[0].mode_status,
            "symbol-retrieval-stability-pass"
        );
        assert_eq!(
            report.rows[1].mode_status,
            "symbol-retrieval-stability-pass"
        );
        assert_eq!(
            report.rows[2].mode_status,
            "symbol-retrieval-stability-pass"
        );
        assert_eq!(
            report.rows[3].mode_status,
            "symbol-retrieval-stability-pass"
        );
        assert_eq!(
            report.rows.map(|row| row.stored_patterns),
            RETRIEVAL_STABILITY_COUNTS
        );
        for row in report.rows {
            assert_eq!(row.clusters, RETRIEVAL_TURBO_CLUSTERS);
            assert_eq!(row.cells, 256);
            assert_eq!(row.noisy_probe_cases, row.stored_patterns);
            assert_eq!(row.cold_ablation_cases, row.stored_patterns);
            assert_eq!(row.conflict_cases, CONFLICT_PROBES.len());
            assert!(row.noisy_hits <= row.noisy_probe_cases);
            assert!(row.veto_noisy_accepts <= row.noisy_probe_cases);
            assert!(row.veto_conflict_rejections <= row.conflict_cases);
            assert!(row.veto_cold_rejections <= row.cold_ablation_cases);
        }

        let text = report.to_text();
        assert!(text.contains("Symbol retrieval stability sweep"));
        assert!(text.contains("stored_patterns: 4"));
        assert!(text.contains("stored_patterns: 32"));
        assert!(text.contains("mode_status: symbol-retrieval-stability"));
    }

    #[test]
    fn symbol_retrieval_capacity_eval_has_required_rows() {
        let report = symbol_retrieval_capacity_eval();

        assert_eq!(report.rows.len(), 4);
        assert!(report.max_passing_patterns >= 128);
        assert_eq!(report.rows[2].mode_status, "symbol-retrieval-capacity-pass");
        assert_eq!(
            report.rows.map(|row| row.stored_patterns),
            RETRIEVAL_CAPACITY_COUNTS
        );
        for row in report.rows {
            assert_eq!(row.seed_cases, RETRIEVAL_CAPACITY_SEEDS.len());
            assert_eq!(row.clusters, RETRIEVAL_TURBO_CLUSTERS);
            assert_eq!(row.cells, 256);
            assert_eq!(row.noisy_probe_cases, row.stored_patterns * row.seed_cases);
            assert_eq!(
                row.cold_ablation_cases,
                row.stored_patterns * row.seed_cases
            );
            assert_eq!(
                row.conflict_cases,
                RETRIEVAL_CAPACITY_CONFLICTS * row.seed_cases
            );
            assert!(row.passing_seeds <= row.seed_cases);
            assert!(row.noisy_hits <= row.noisy_probe_cases);
            assert!(row.veto_noisy_accepts <= row.noisy_probe_cases);
            assert!(row.veto_conflict_rejections <= row.conflict_cases);
            assert!(row.veto_cold_rejections <= row.cold_ablation_cases);
        }

        let text = report.to_text();
        assert!(text.contains("Symbol retrieval capacity-1 eval"));
        assert!(text.contains("stored_patterns: 32"));
        assert!(text.contains("stored_patterns: 256"));
        assert!(text.contains("mode_status: symbol-retrieval-capacity"));
    }

    #[test]
    fn symbol_retrieval_capacity_scale_eval_has_required_rows() {
        let report = symbol_retrieval_capacity_scale_eval();

        assert_eq!(report.rows.len(), 3);
        assert_eq!(report.stored_patterns, RETRIEVAL_CAPACITY_SCALE_PATTERNS);
        assert_eq!(
            report.rows.map(|row| row.clusters),
            RETRIEVAL_CAPACITY_SCALE_CLUSTERS
        );
        assert_eq!(report.rows.map(|row| row.cells), [256, 512, 1024]);
        for row in report.rows {
            assert_eq!(row.stored_patterns, RETRIEVAL_CAPACITY_SCALE_PATTERNS);
            assert_eq!(row.seed_cases, RETRIEVAL_CAPACITY_SEEDS.len());
            assert_eq!(row.noisy_probe_cases, row.stored_patterns * row.seed_cases);
            assert_eq!(
                row.cold_ablation_cases,
                row.stored_patterns * row.seed_cases
            );
            assert_eq!(
                row.conflict_cases,
                RETRIEVAL_CAPACITY_CONFLICTS * row.seed_cases
            );
            assert!(row.passing_seeds <= row.seed_cases);
            assert!(row.noisy_hits <= row.noisy_probe_cases);
            assert!(row.veto_noisy_accepts <= row.noisy_probe_cases);
            assert!(row.veto_conflict_rejections <= row.conflict_cases);
            assert!(row.veto_cold_rejections <= row.cold_ablation_cases);
        }

        let text = report.to_text();
        assert!(text.contains("Symbol retrieval capacity-scale eval"));
        assert!(text.contains("stored_patterns: 256"));
        assert!(text.contains("cells: 1024"));
        assert!(text.contains("mode_status: symbol-retrieval-capacity-scale"));
    }
}
