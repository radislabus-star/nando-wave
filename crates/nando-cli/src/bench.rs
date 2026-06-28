use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nando_core::{
    BytePhaseLut, CarrierWave, Cell32Learner, L1CenterMemoryConfig, L1CenterMemoryProof,
    L2CenterMemoryConfig, L2CenterMemoryProof, L3SemanticGrokkingProof, LinkProfile, LinkTissue,
    Stage2Organ, SymbolCell8, SymbolL3Organism, SymbolWaveCluster, TickTrace,
    run_stage2_tick_with_carrier, run_stage2_tick_with_organ_carrier,
    run_stage2_trace_with_organ_carrier, run_stage2_trace_with_organ_lut_carrier,
};

pub(crate) fn print_stage2_tick_bench(seed: u64, ticks: usize) {
    let ticks = ticks.max(1);
    let seed_tick = bench_seed_tick(seed, ticks);
    let organ_tick = bench_organ_tick(seed, ticks);
    let organ_trace_tick = bench_organ_trace_tick(seed, ticks);
    let organ_trace_lut_tick = bench_organ_trace_lut_tick(seed, ticks);

    println!("Nando Wave stage-2 tick bench");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    print_row("seed_tick", seed_tick, ticks);
    print_row("precomputed_organ_tick", organ_tick, ticks);
    print_row("precomputed_organ_trace_tick", organ_trace_tick, ticks);
    print_row(
        "precomputed_organ_trace_lut_tick",
        organ_trace_lut_tick,
        ticks,
    );
    println!(
        "precomputed_speedup: {:.3}x",
        seed_tick.as_secs_f64() / organ_tick.as_secs_f64().max(f64::EPSILON)
    );
    println!(
        "trace_only_speedup_over_snapshot: {:.3}x",
        organ_tick.as_secs_f64() / organ_trace_tick.as_secs_f64().max(f64::EPSILON)
    );
    println!(
        "lut_speedup_over_direct_trace: {:.3}x",
        organ_trace_tick.as_secs_f64() / organ_trace_lut_tick.as_secs_f64().max(f64::EPSILON)
    );
}

pub(crate) fn print_link_tissue_bench(seed: u64, ticks: usize) {
    let ticks = ticks.max(1);
    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let mut traces = Vec::with_capacity(256);
    for index in 0..256 {
        let input = bench_input(index);
        let carrier = CarrierWave::from_seed(seed, input);
        traces.push(
            run_stage2_trace_with_organ_lut_carrier(&organ, &lut, input, carrier, None).trace,
        );
    }

    let cell3 = Cell32Learner::new(3, 0.08);
    let cell6 = Cell32Learner::new(6, 0.08);
    let pair = LinkTissue::new(6, false, 0.08);
    let triple = LinkTissue::new(6, true, 0.08);
    let typed_pair = LinkTissue::with_profile(6, false, 0.08, LinkProfile::Typed);
    let typed_triple = LinkTissue::with_profile(6, true, 0.08, LinkProfile::Typed);

    let cell3_score = bench_cell_score(&cell3, &traces, ticks);
    let cell6_score = bench_cell_score(&cell6, &traces, ticks);
    let pair_score = bench_tissue_score(&pair, &traces, ticks);
    let triple_score = bench_tissue_score(&triple, &traces, ticks);
    let typed_pair_score = bench_tissue_score(&typed_pair, &traces, ticks);
    let typed_triple_score = bench_tissue_score(&typed_triple, &traces, ticks);

    println!("Nando Wave link tissue bench");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    print_row("cell3_score", cell3_score, ticks);
    print_row("cell6_score", cell6_score, ticks);
    print_row("pair_score", pair_score, ticks);
    print_row("typed_pair_score", typed_pair_score, ticks);
    print_row("triple_score", triple_score, ticks);
    print_row("typed_triple_score", typed_triple_score, ticks);
    println!(
        "typed_pair_vs_pair: {:.3}x",
        pair_score.as_secs_f64() / typed_pair_score.as_secs_f64().max(f64::EPSILON)
    );
    println!(
        "typed_triple_vs_triple: {:.3}x",
        triple_score.as_secs_f64() / typed_triple_score.as_secs_f64().max(f64::EPSILON)
    );
}

pub(crate) fn print_symbol_l3_bench(seed: u64, ticks: usize) {
    let ticks = ticks.max(1);
    let cell_tick = bench_symbol_cell8_tick(seed, ticks);
    let cluster_tick = bench_symbol_cluster_tick(seed, ticks);
    let rows = [
        ("symbol_l3_256_cells", bench_symbol_l3_tick(seed, ticks, 16)),
        (
            "symbol_l3_512_cells_default",
            bench_symbol_l3_tick(seed, ticks, 32),
        ),
        ("symbol_l3_768_cells", bench_symbol_l3_tick(seed, ticks, 48)),
        (
            "symbol_l3_1024_cells_stress",
            bench_symbol_l3_tick(seed, ticks, 64),
        ),
    ];

    println!("Nando Wave SymbolL3 bench");
    println!("seed: {seed}");
    println!("ticks: {ticks}");
    print_row("symbol_cell8_tick", cell_tick, ticks);
    print_row("symbol_cluster16_tick", cluster_tick, ticks);
    for (name, row) in rows {
        println!("{name}.clusters: {}", row.clusters);
        println!("{name}.cells: {}", row.cells);
        println!("{name}.active_bytes: {}", row.active_bytes);
        print_row(name, row.duration, ticks);
    }

    let default_duration = rows[1].1.duration.as_secs_f64();
    let stress_duration = rows[3].1.duration.as_secs_f64();
    println!(
        "default_512_vs_stress_1024_speedup: {:.3}x",
        stress_duration / default_duration.max(f64::EPSILON)
    );
}

pub(crate) fn print_wave_layer_metrics() -> Result<(), String> {
    let words = corpus_words("russian_words_300k.txt")?;

    let start = Instant::now();
    let l1 = L1CenterMemoryProof::prove(
        words[..8_000].iter().map(String::as_str),
        words[8_000..10_000].iter().map(String::as_str),
        L1CenterMemoryConfig {
            min_center_support: 2,
            min_heldout_ngram_coverage: 0.70,
            min_average_reconstruction_similarity: 0.68,
            min_average_fourier_similarity: 0.64,
            min_fourier_ablation_drop: 0.03,
            min_real_vs_corrupt_coverage_gap: 0.12,
            max_model_to_naive_ratio: 0.12,
            max_corrupt_eval_words: 1_024,
            max_fourier_eval_words: 512,
            ..L1CenterMemoryConfig::default()
        },
    );
    let l1_elapsed = start.elapsed();

    let start = Instant::now();
    let l2 = L2CenterMemoryProof::prove(
        words[..20_000].iter().map(String::as_str),
        words[20_000..25_000].iter().map(String::as_str),
        L2CenterMemoryConfig {
            l1_config: L1CenterMemoryConfig {
                min_center_support: 2,
                min_heldout_ngram_coverage: 0.70,
                min_average_reconstruction_similarity: 0.68,
                min_average_fourier_similarity: 0.64,
                min_fourier_ablation_drop: 0.03,
                min_real_vs_corrupt_coverage_gap: 0.12,
                max_model_to_naive_ratio: 0.12,
                max_corrupt_eval_words: 1_024,
                max_fourier_eval_words: 512,
                ..L1CenterMemoryConfig::default()
            },
            motif_len: 4,
            min_motif_support: 4,
            min_heldout_ref_coverage: 0.60,
            min_heldout_word_coverage: 0.50,
            min_average_sequence_similarity: 0.65,
            min_average_fourier_similarity: 0.65,
            min_fourier_ablation_drop: 0.20,
            min_real_vs_corrupt_coverage_gap: 0.30,
            max_model_to_naive_ratio: 0.90,
            max_corrupt_eval_words: 1_024,
            max_fourier_eval_words: 512,
            ..L2CenterMemoryConfig::default()
        },
    );
    let l2_elapsed = start.elapsed();

    let start = Instant::now();
    let l3 = L3SemanticGrokkingProof::prove_hard_semantic_profile();
    let l3_elapsed = start.elapsed();

    println!("Nando Wave layered architecture metrics");
    println!();
    print_l1_metrics(&l1, l1_elapsed);
    print_l2_metrics(&l2, l2_elapsed);
    print_l3_metrics(&l3, l3_elapsed);
    println!();
    println!(
        "meaning_path: L1 surface centers -> L2 motif centers -> learned CueField -> L3 contrastive field -> EquationForm -> operator"
    );
    println!("best_current_use: bounded profile semantic memory with explicit no-answer boundary");
    println!(
        "next_improvement: scale withheld paraphrase families, cross-domain traps, and evidence-specific no-answer states"
    );
    Ok(())
}

#[derive(Clone, Copy)]
struct SymbolL3BenchRow {
    duration: Duration,
    clusters: usize,
    cells: usize,
    active_bytes: usize,
}

fn print_l1_metrics(proof: &L1CenterMemoryProof, elapsed: Duration) {
    println!("L1 surface center memory");
    println!("  elapsed_ms: {:.3}", elapsed.as_secs_f64() * 1_000.0);
    println!("  train_words: {}", proof.train_words);
    println!("  heldout_words: {}", proof.heldout_words);
    println!("  centers: {}", proof.center_count);
    println!("  model_hot_bytes: {}", proof.model_hot_bytes);
    println!("  naive_wave_bytes: {}", proof.naive_total_wave_bytes);
    println!(
        "  model_to_naive_ratio: {:.6}",
        proof.model_to_naive_total_ratio
    );
    println!(
        "  heldout_ngram_coverage: {:.6}",
        proof.heldout_ngram_coverage
    );
    println!(
        "  fourier_similarity: {:.6}",
        proof.average_fourier_similarity
    );
    println!(
        "  fourier_ablation_drop: {:.6}",
        proof.fourier_ablation_drop
    );
    println!(
        "  exact_lookup_heldout_hits: {}",
        proof.exact_lookup_heldout_hits
    );
    println!("  ready_for_l2: {}", proof.promotion_ready_for_l2);
    println!();
}

fn print_l2_metrics(proof: &L2CenterMemoryProof, elapsed: Duration) {
    println!("L2 motif center memory");
    println!("  elapsed_ms: {:.3}", elapsed.as_secs_f64() * 1_000.0);
    println!("  train_words: {}", proof.train_words);
    println!("  heldout_words: {}", proof.heldout_words);
    println!("  l1_centers: {}", proof.l1_center_count);
    println!("  l2_centers: {}", proof.l2_center_count);
    println!("  model_hot_bytes: {}", proof.model_hot_bytes);
    println!(
        "  naive_l1_sequence_bytes: {}",
        proof.naive_total_l1_sequence_bytes
    );
    println!(
        "  model_to_naive_ratio: {:.6}",
        proof.model_to_naive_total_ratio
    );
    println!("  heldout_ref_coverage: {:.6}", proof.heldout_ref_coverage);
    println!(
        "  sequence_similarity: {:.6}",
        proof.average_sequence_similarity
    );
    println!(
        "  fourier_similarity: {:.6}",
        proof.average_fourier_similarity
    );
    println!(
        "  fourier_ablation_drop: {:.6}",
        proof.fourier_ablation_drop
    );
    println!(
        "  real_vs_corrupt_coverage_gap: {:.6}",
        proof.real_vs_corrupt_coverage_gap
    );
    println!(
        "  exact_lookup_heldout_hits: {}",
        proof.exact_lookup_heldout_hits
    );
    println!("  ready_for_l3: {}", proof.promotion_ready_for_l3);
    println!();
}

fn print_l3_metrics(proof: &L3SemanticGrokkingProof, elapsed: Duration) {
    println!("L3 semantic grokking");
    println!("  elapsed_ms: {:.3}", elapsed.as_secs_f64() * 1_000.0);
    println!("  train_examples: {}", proof.train_examples);
    println!("  heldout_examples: {}", proof.heldout_examples);
    println!("  relation_family_count: {}", proof.relation_family_count);
    println!(
        "  paraphrase_template_count: {}",
        proof.paraphrase_template_count
    );
    println!("  frame_count: {}", proof.frame_count);
    println!("  l2_center_count: {}", proof.l2_center_count);
    println!("  operator_count: {}", proof.operator_count);
    println!("  model_hot_bytes: {}", proof.model_hot_bytes);
    println!(
        "  naive_semantic_fact_bytes: {}",
        proof.naive_semantic_fact_bytes
    );
    println!("  model_to_naive_ratio: {:.6}", proof.model_to_naive_ratio);
    println!("  frame_accuracy: {:.6}", proof.frame_accuracy);
    println!("  answer_accuracy: {:.6}", proof.answer_accuracy);
    println!("  average_frame_gap: {:.6}", proof.average_frame_gap);
    println!(
        "  average_raw_field_gap: {:.6}",
        proof.average_raw_field_gap
    );
    println!(
        "  average_settled_field_gap: {:.6}",
        proof.average_settled_field_gap
    );
    println!(
        "  interference_gap_lift: {:.6}",
        proof.interference_gap_lift
    );
    println!(
        "  average_interference_energy: {:.6}",
        proof.average_interference_energy
    );
    println!("  cue_edge_count: {}", proof.cue_edge_count);
    println!("  manual_cue_rules_used: {}", proof.manual_cue_rules_used);
    println!("  cue_field_learned: {}", proof.cue_field_learned);
    println!(
        "  cue_contrastive_training_used: {}",
        proof.cue_contrastive_training_used
    );
    println!("  cue_extractor_learned: {}", proof.cue_extractor_learned);
    println!("  cue_accuracy: {:.6}", proof.cue_accuracy);
    println!("  cue_margin_min: {:.6}", proof.cue_margin_min);
    println!("  cue_ablation_drop: {:.6}", proof.cue_ablation_drop);
    println!("  wrong_cue_suppressed: {}", proof.wrong_cue_suppressed);
    println!(
        "  semantic_compiler_ready: {}",
        proof.semantic_compiler_ready
    );
    println!(
        "  interference_edge_count: {}",
        proof.interference_edge_count
    );
    println!(
        "  manual_weight_table_used: {}",
        proof.manual_weight_table_used
    );
    println!("  field_weights_learned: {}", proof.field_weights_learned);
    println!(
        "  contrastive_training_used: {}",
        proof.contrastive_training_used
    );
    println!("  heldout_margin_min: {:.6}", proof.heldout_margin_min);
    println!(
        "  nearest_wrong_center_suppressed: {}",
        proof.nearest_wrong_center_suppressed
    );
    println!(
        "  attraction_ablation_drop: {:.6}",
        proof.attraction_ablation_drop
    );
    println!(
        "  repulsion_ablation_drop: {:.6}",
        proof.repulsion_ablation_drop
    );
    println!(
        "  anti_field_ablation_drop: {:.6}",
        proof.anti_field_ablation_drop
    );
    println!("  frame_ablation_drop: {:.6}", proof.frame_ablation_drop);
    println!("  object_anchor_pass: {}", proof.object_anchor_pass);
    println!(
        "  evidence_requirement_pass: {}",
        proof.evidence_requirement_pass
    );
    println!(
        "  missing_evidence_blocked: {}",
        proof.missing_evidence_blocked
    );
    println!("  role_swap_rejected: {}", proof.role_swap_rejected);
    println!("  route_splice_rejected: {}", proof.route_splice_rejected);
    println!(
        "  negative_route_rejected: {}",
        proof.negative_route_rejected
    );
    println!("  false_promotion_rate: {:.6}", proof.false_promotion_rate);
    println!(
        "  interference_ablation_pass: {}",
        proof.interference_ablation_pass
    );
    println!(
        "  exact_lookup_heldout_hits: {}",
        proof.exact_lookup_heldout_hits
    );
    println!("  semantic_field_ready: {}", proof.semantic_field_ready);
    println!(
        "  semantic_grokking_ready: {}",
        proof.semantic_grokking_ready
    );
    println!("  hard_profile_ready: {}", proof.hard_profile_ready);
}

fn corpus_words(name: &str) -> Result<Vec<String>, String> {
    let path = corpus_path(name);
    std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|word| !word.is_empty())
                .map(str::to_string)
                .collect()
        })
}

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus")
        .join(name)
}

fn bench_seed_tick(seed: u64, ticks: usize) -> Duration {
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let input = bench_input(index);
        let carrier = CarrierWave::from_seed(seed, input);
        let tick = run_stage2_tick_with_carrier(seed, input, carrier, None);
        checksum += tick.trace.center_phase + tick.trace.coherence;
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_organ_tick(seed: u64, ticks: usize) -> Duration {
    let organ = Stage2Organ::new(seed);
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let input = bench_input(index);
        let carrier = CarrierWave::from_seed(seed, input);
        let tick = run_stage2_tick_with_organ_carrier(&organ, input, carrier, None);
        checksum += tick.trace.center_phase + tick.trace.coherence;
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_organ_trace_tick(seed: u64, ticks: usize) -> Duration {
    let organ = Stage2Organ::new(seed);
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let input = bench_input(index);
        let carrier = CarrierWave::from_seed(seed, input);
        let tick = run_stage2_trace_with_organ_carrier(&organ, input, carrier, None);
        checksum += tick.trace.center_phase + tick.trace.coherence;
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_organ_trace_lut_tick(seed: u64, ticks: usize) -> Duration {
    let organ = Stage2Organ::new(seed);
    let lut = BytePhaseLut::new();
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let input = bench_input(index);
        let carrier = CarrierWave::from_seed(seed, input);
        let tick = run_stage2_trace_with_organ_lut_carrier(&organ, &lut, input, carrier, None);
        checksum += tick.trace.center_phase + tick.trace.coherence;
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_cell_score(cell: &Cell32Learner, traces: &[TickTrace], ticks: usize) -> Duration {
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let trace = traces[index & 255];
        let byte = bench_input(index);
        checksum += cell.score(&trace, byte);
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_tissue_score(tissue: &LinkTissue, traces: &[TickTrace], ticks: usize) -> Duration {
    let start = Instant::now();
    let mut checksum = 0.0f32;
    for index in 0..ticks {
        let trace = traces[index & 255];
        let byte = bench_input(index);
        checksum += tissue.score(&trace, byte);
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_symbol_l3_tick(seed: u64, ticks: usize, clusters: usize) -> SymbolL3BenchRow {
    let mut organism = SymbolL3Organism::with_clusters(seed, clusters);
    for index in 0..ticks.min(16) {
        let _ = organism.tick_symbol(bench_symbol(index));
    }

    let start = Instant::now();
    let mut checksum = 0u64;
    for index in 0..ticks {
        let tick = organism.tick_symbol(bench_symbol(index));
        checksum = checksum
            .wrapping_add(tick.center.energy)
            .wrapping_add(u64::from(tick.center.coherence))
            .wrapping_add(u64::from(tick.center.support_cells))
            .wrapping_add(u64::from(tick.forward_messages));
    }
    let duration = start.elapsed();
    black_box(checksum);

    SymbolL3BenchRow {
        duration,
        clusters: organism.cluster_count(),
        cells: organism.cell_count(),
        active_bytes: organism.active_bytes(),
    }
}

fn bench_symbol_cell8_tick(seed: u64, ticks: usize) -> Duration {
    let mut cell = SymbolCell8::new(0, 1, seed);
    for index in 0..ticks.min(16) {
        let _ = cell.tick_symbol(bench_symbol(index));
    }

    let start = Instant::now();
    let mut checksum = 0u64;
    for index in 0..ticks {
        let tick = cell.tick_symbol(bench_symbol(index));
        checksum = checksum
            .wrapping_add(u64::from(tick.score.energy))
            .wrapping_add(u64::from(tick.score.coherence))
            .wrapping_add(u64::from(tick.score.stable_score))
            .wrapping_add(u64::from(cell.calibration.active_slot_count));
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_symbol_cluster_tick(seed: u64, ticks: usize) -> Duration {
    let mut cluster = SymbolWaveCluster::new(0, seed);
    for index in 0..ticks.min(16) {
        let _ = cluster.tick_symbol(bench_symbol(index));
    }

    let start = Instant::now();
    let mut checksum = 0u64;
    for index in 0..ticks {
        let tick = cluster.tick_symbol(bench_symbol(index));
        checksum = checksum
            .wrapping_add(u64::from(tick.center.energy))
            .wrapping_add(u64::from(tick.center.coherence))
            .wrapping_add(u64::from(tick.center.support_count));
    }
    black_box(checksum);
    start.elapsed()
}

fn bench_input(index: usize) -> u8 {
    (index as u8)
        .wrapping_mul(37)
        .wrapping_add((index >> 8) as u8)
        .wrapping_add(11)
}

fn bench_symbol(index: usize) -> char {
    const SYMBOLS: [char; 8] = ['N', 'A', 'D', 'W', '0', '1', 'x', ' '];
    SYMBOLS[index & 7]
}

fn print_row(name: &str, duration: Duration, ticks: usize) {
    let seconds = duration.as_secs_f64();
    let ns_per_tick = duration.as_nanos() as f64 / ticks as f64;
    let ticks_per_second = ticks as f64 / seconds.max(f64::EPSILON);
    println!("{name}.total_ms: {:.3}", seconds * 1000.0);
    println!("{name}.ns_per_tick: {:.1}", ns_per_tick);
    println!("{name}.ticks_per_second: {:.0}", ticks_per_second);
}
