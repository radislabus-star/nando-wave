//! Evaluation harness placeholder for Nando Wave.
//!
//! Stage 2 has fixed packets, a deterministic tick, and a minimal one-tick
//! report. Actual baselines, ablation, and mode detection start later.

use nando_core::{STAGE2_TOP_K, project_status, run_stage2_tick};

mod byte_context;
mod chat0;
mod math;
mod modadd;
mod phase;
mod result;
mod settle_word;
mod symbol_cell8;
mod symbol_cluster;
mod symbol_l3;
mod symbol_retrieval;
mod symbol_understanding;
pub use byte_context::{
    ByteContextCellularCarrierAblationReport, ByteContextCentroidAblationReport,
    ByteContextCentroidReport, ByteContextCentroidSeedRow, ByteContextCentroidSeedSweepReport,
    ByteContextPromptCarrierAblationReport, ByteContextReport,
    ByteContextTrainedCarrierAblationReport, byte_context_cellular_carrier_ablation_eval,
    byte_context_cellular_carrier_centroid_eval,
    byte_context_cellular_carrier_centroid_seed_sweep_eval, byte_context_centroid_ablation_eval,
    byte_context_centroid_eval, byte_context_centroid_seed_sweep_eval,
    byte_context_denoised_centroid_eval, byte_context_denoised_centroid_seed_sweep_eval,
    byte_context_eval, byte_context_lexical_carrier_centroid_eval,
    byte_context_lexical_carrier_centroid_seed_sweep_eval, byte_context_offset_centroid_eval,
    byte_context_offset_centroid_seed_sweep_eval, byte_context_prompt_carrier_ablation_eval,
    byte_context_prompt_carrier_centroid_eval,
    byte_context_prompt_carrier_centroid_seed_sweep_eval,
    byte_context_prompt_carrier_diverse_ablation_eval,
    byte_context_prompt_carrier_diverse_centroid_eval,
    byte_context_prompt_carrier_diverse_centroid_seed_sweep_eval,
    byte_context_relative_centroid_eval, byte_context_relative_centroid_seed_sweep_eval,
    byte_context_trained_carrier_ablation_eval, byte_context_trained_carrier_centroid_eval,
    byte_context_trained_carrier_centroid_seed_sweep_eval, chat0_eval, chat0_once,
    chat0_once_with_promoted_state, chat0_promote_eval, chat0_promoted_holdout_eval,
    chat0_route_eval,
};
pub use chat0::{
    Chat0EvalReport, Chat0FeedbackEntry, Chat0PromoteEvalReport, Chat0PromotedEntry,
    Chat0PromotedHoldoutEvalReport, Chat0PromotedState, Chat0RouteEvalReport, Chat0Trace,
};
pub(crate) use chat0::{
    chat0_response_for_target, chat0_target_for_response, chat0_task_for_target,
};
pub(crate) use math::{byte_to_phase, circular_delta, splitmix64};
pub use modadd::{
    Organ128ModAddConfig, Organ128ModAddReport, Organ128ModAddSeedSweepReport,
    Organ128ModAddSeedSweepRow, organ128_modadd_eval, organ128_modadd_seed_sweep_eval,
};
pub use phase::{
    BusTransferReport, CarrierControlReport, HorizonSweepRow, PeriodicEvalReport,
    PeriodicTaskConfig, PhaseCompositionConfig, PhaseCompositionReport, PhaseHoldoutReport,
    SeedSweepRow, SnapshotAdaptReport, SnapshotDecoderReport, SnapshotDynamicsReport,
    SnapshotKeyedReport, SnapshotKeyedTransitionReport, SnapshotMemoryReport,
    SnapshotMultiTickReport, SnapshotNoisyKeyedTransitionReport,
    SnapshotNoisyKeyedTransitionSeedSweepReport, SnapshotNoisyKeyedTransitionSweepReport,
    SnapshotTransitionReport, bus_transfer_eval, carrier_control_eval, periodic_eval,
    phase_composition_eval, phase_composition_holdout_eval, snapshot_adapt_eval,
    snapshot_decoder_eval, snapshot_dynamics_eval, snapshot_keyed_eval,
    snapshot_keyed_transition_eval, snapshot_memory_eval, snapshot_multitick_eval,
    snapshot_noisy_keyed_transition_eval, snapshot_noisy_keyed_transition_seed_sweep_eval,
    snapshot_noisy_keyed_transition_sweep_eval, snapshot_transition_eval,
};
pub(crate) use phase::{
    best_baseline, build_corrupted_carrier_snapshot, build_corrupted_snapshot,
    corrupted_carrier_wave, no_carrier_wave, random_predict, score_prediction, snapshot_roundtrip,
    voting_predict, wave_bus_predict, wrong_carrier_wave,
};
pub use result::{BaselineResult, Chat0Result};
pub(crate) use result::{best_chat0_control, format_baseline, format_chat0_result};
pub use settle_word::{
    SettleWordEvalReport, SettleWordSeedSweepReport, SettleWordSeedSweepRow, settle_word_eval,
    settle_word_seed_sweep_eval,
};
pub use symbol_cell8::{SymbolCell8EvalReport, symbol_cell8_eval};
pub use symbol_cluster::{SymbolClusterEvalReport, symbol_cluster_eval};
pub use symbol_l3::{SymbolL3EvalReport, SymbolL3ProfileEvalRow, symbol_l3_eval};
pub use symbol_retrieval::{
    SymbolRetrieval0EvalReport, SymbolRetrievalCapacityReport, SymbolRetrievalCapacityRow,
    SymbolRetrievalCapacityScaleReport, SymbolRetrievalScaleRow, SymbolRetrievalStabilityReport,
    SymbolRetrievalStabilityRow, symbol_retrieval_capacity_eval,
    symbol_retrieval_capacity_scale_eval, symbol_retrieval_stability_sweep, symbol_retrieval0_eval,
};
pub use symbol_understanding::{SymbolUnderstanding0EvalReport, symbol_understanding0_eval};

/// Describe the current eval harness state.
#[must_use]
pub fn eval_harness_status() -> &'static str {
    if project_status().rust_first {
        "eval-harness-stage-2-one-tick"
    } else {
        "invalid-non-rust-runtime"
    }
}

/// Minimal Stage 2 report for one deterministic tick.
#[derive(Debug, Clone, PartialEq)]
pub struct OneTickReport {
    pub seed: u64,
    pub input_byte: u8,
    pub cells_scanned: usize,
    pub active_count: usize,
    pub active_cell_ids: [u32; STAGE2_TOP_K],
    pub coherence: f32,
    pub spectral_entropy: f32,
    pub center_phase: f32,
    pub center_magnitude: f32,
    pub snapshot_bytes: usize,
    pub mode_status: &'static str,
}

impl OneTickReport {
    /// Render a stable, line-oriented report for CLI and files.
    #[must_use]
    pub fn to_text(&self) -> String {
        format!(
            concat!(
                "Nando Wave one-tick eval\n",
                "seed: {seed}\n",
                "input_byte: {input_byte}\n",
                "cells_scanned: {cells_scanned}\n",
                "active_count: {active_count}\n",
                "active_cell_ids: {active_cell_ids:?}\n",
                "coherence: {coherence:.6}\n",
                "spectral_entropy: {spectral_entropy:.6}\n",
                "center_phase: {center_phase:.6}\n",
                "center_magnitude: {center_magnitude:.6}\n",
                "snapshot_bytes: {snapshot_bytes}\n",
                "mode_status: {mode_status}\n"
            ),
            seed = self.seed,
            input_byte = self.input_byte,
            cells_scanned = self.cells_scanned,
            active_count = self.active_count,
            active_cell_ids = self.active_cell_ids,
            coherence = self.coherence,
            spectral_entropy = self.spectral_entropy,
            center_phase = self.center_phase,
            center_magnitude = self.center_magnitude,
            snapshot_bytes = self.snapshot_bytes,
            mode_status = self.mode_status
        )
    }
}

/// Build the Stage 2 one-tick report.
#[must_use]
pub fn one_tick_report(seed: u64, input_byte: u8) -> OneTickReport {
    let tick = run_stage2_tick(seed, input_byte);

    OneTickReport {
        seed,
        input_byte,
        cells_scanned: tick.trace.cells_scanned,
        active_count: tick.trace.active_count,
        active_cell_ids: tick.trace.active_cell_ids,
        coherence: tick.trace.coherence,
        spectral_entropy: tick.trace.spectral_entropy,
        center_phase: tick.trace.center_phase,
        center_magnitude: tick.trace.center_magnitude,
        snapshot_bytes: tick.snapshot.to_bytes().len(),
        mode_status: "not_tested_stage_2_no_baseline",
    }
}

const BYTE_CONTEXT_TASKS: [(&str, u8); 8] = [
    ("ping", b'p'),
    ("name", b'n'),
    ("time", b't'),
    ("help", b'h'),
    ("echo", b'e'),
    ("save", b's'),
    ("open", b'o'),
    ("close", b'c'),
];

#[cfg(test)]
mod tests {
    use super::*;
    use nando_core::{SNAPSHOT_V1_BYTES, STAGE2_ORGAN_CELLS};

    #[test]
    fn eval_harness_is_placeholder_in_stage_two() {
        assert_eq!(eval_harness_status(), "eval-harness-stage-2-one-tick");
    }

    #[test]
    fn one_tick_report_has_stage_two_shape() {
        let report = one_tick_report(7, 42);
        assert_eq!(report.cells_scanned, STAGE2_ORGAN_CELLS);
        assert_eq!(report.active_count, STAGE2_TOP_K);
        assert_eq!(report.snapshot_bytes, SNAPSHOT_V1_BYTES);
        assert_eq!(report.mode_status, "not_tested_stage_2_no_baseline");
        assert!(report.to_text().contains("mode_status: not_tested"));
    }
}
