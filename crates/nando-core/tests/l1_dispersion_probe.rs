use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use nando_core::{L1CenterMemory, L1CenterMemoryConfig, SURFACE_WAVE_DIM};

const TRAIN_WORDS: usize = 60_000;
const HELDOUT_WORDS: usize = 15_000;
const CORRUPT_WORDS: usize = 4_096;
const MOTIF_LEN: usize = 4;
const MIN_MOTIF_SUPPORT: usize = 4;
const L2_CENTER_RECORD_BYTES: usize = 32;
const L2_TOKEN_REF_BYTES: usize = 4;
const L2_WORD_RECORD_BYTES: usize = 16;
const L1_SEQUENCE_REF_BYTES: usize = 4;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct CenterQuality {
    support: u32,
    lane_dispersion: f32,
    support_dampening: f32,
    quality: f32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct QualityStats {
    support_p50: u32,
    support_p90: u32,
    support_p99: u32,
    quality_p20: f32,
    quality_p40: f32,
    quality_p60: f32,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct ProbeConfig {
    name: &'static str,
    min_center_quality: f32,
}

#[derive(Clone, Debug)]
struct MotifModel {
    motif_count: usize,
    center_index: HashMap<Vec<u32>, u32>,
    build_elapsed: Duration,
}

#[derive(Clone, Copy, Debug)]
struct EncodeMetrics {
    l1_refs: usize,
    covered_l1_refs: usize,
    word_coverage: f32,
    token_refs: usize,
    residual_l1_refs: usize,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct ProbeRow {
    motif_count: usize,
    train_token_refs: usize,
    train_residual_refs: usize,
    heldout_ref_coverage: f32,
    heldout_word_coverage: f32,
    corrupt_ref_coverage: f32,
    real_vs_corrupt_gap: f32,
    model_hot_bytes: usize,
    model_to_naive_ratio: f32,
    build_ms: f64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct DispersionProbeReport {
    train_words: usize,
    heldout_words: usize,
    l1_center_count: usize,
    quality_stats: QualityStats,
    baseline: ProbeRow,
    dispersion_p20: ProbeRow,
    dispersion_p40: ProbeRow,
}

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus")
        .join(name)
}

fn corpus_words(name: &str) -> Vec<String> {
    std::fs::read_to_string(corpus_path(name))
        .unwrap_or_else(|error| panic!("{name} corpus file must be readable: {error}"))
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
#[ignore = "report-only L1 dispersion gate experiment"]
fn l1_dispersion_probe_compares_l2_motif_quality() {
    let words = corpus_words("russian_words_300k.txt");
    let report = run_dispersion_probe(
        &words[..TRAIN_WORDS],
        &words[TRAIN_WORDS..][..HELDOUT_WORDS],
    );
    eprintln!("{report:#?}");

    assert_eq!(report.train_words, TRAIN_WORDS);
    assert_eq!(report.heldout_words, HELDOUT_WORDS);
    assert!(report.l1_center_count > 10_000, "report={report:#?}");
    assert!(report.baseline.motif_count > 1_000, "report={report:#?}");
    assert!(
        report.baseline.heldout_ref_coverage > 0.60,
        "report={report:#?}"
    );
    assert!(
        report.baseline.real_vs_corrupt_gap > 0.20,
        "report={report:#?}"
    );
}

fn run_dispersion_probe(train: &[String], heldout: &[String]) -> DispersionProbeReport {
    let l1 = L1CenterMemory::build(
        train.iter().map(String::as_str),
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
    let qualities = center_qualities(&l1);
    let quality_stats = quality_stats(&qualities);
    let train_sequences = train
        .iter()
        .map(|word| l1.center_sequence_for_word(word).center_refs)
        .collect::<Vec<_>>();
    let heldout_sequences = heldout
        .iter()
        .map(|word| l1.center_sequence_for_word(word).center_refs)
        .collect::<Vec<_>>();
    let corrupt_sequences = heldout
        .iter()
        .take(CORRUPT_WORDS)
        .map(|word| word.chars().rev().collect::<String>())
        .map(|word| l1.center_sequence_for_word(&word).center_refs)
        .collect::<Vec<_>>();
    let configs = [
        ProbeConfig {
            name: "baseline_no_dispersion_gate",
            min_center_quality: 0.0,
        },
        ProbeConfig {
            name: "dispersion_gate_p20",
            min_center_quality: quality_stats.quality_p20,
        },
        ProbeConfig {
            name: "dispersion_gate_p40",
            min_center_quality: quality_stats.quality_p40,
        },
    ];
    let rows = configs
        .iter()
        .map(|config| {
            probe_row(
                config,
                &train_sequences,
                &heldout_sequences,
                &corrupt_sequences,
                &qualities,
            )
        })
        .collect::<Vec<_>>();

    DispersionProbeReport {
        train_words: train.len(),
        heldout_words: heldout.len(),
        l1_center_count: l1.center_count(),
        quality_stats,
        baseline: rows[0],
        dispersion_p20: rows[1],
        dispersion_p40: rows[2],
    }
}

fn probe_row(
    config: &ProbeConfig,
    train_sequences: &[Vec<u32>],
    heldout_sequences: &[Vec<u32>],
    corrupt_sequences: &[Vec<u32>],
    qualities: &[CenterQuality],
) -> ProbeRow {
    let model = build_motif_model(train_sequences, qualities, *config);
    let train_metrics = encode_metrics(&model, train_sequences, 0.0);
    let heldout_metrics = encode_metrics(&model, heldout_sequences, 0.60);
    let corrupt_metrics = encode_metrics(&model, corrupt_sequences, 0.60);
    let model_hot_bytes = model.motif_count * L2_CENTER_RECORD_BYTES
        + model.motif_count * MOTIF_LEN * L1_SEQUENCE_REF_BYTES
        + train_metrics.token_refs * L2_TOKEN_REF_BYTES
        + train_sequences.len() * L2_WORD_RECORD_BYTES;
    let naive_bytes = (train_metrics.l1_refs + heldout_metrics.l1_refs) * L1_SEQUENCE_REF_BYTES
        + (train_sequences.len() + heldout_sequences.len()) * L2_WORD_RECORD_BYTES;

    ProbeRow {
        motif_count: model.motif_count,
        train_token_refs: train_metrics.token_refs,
        train_residual_refs: train_metrics.residual_l1_refs,
        heldout_ref_coverage: ratio(heldout_metrics.covered_l1_refs, heldout_metrics.l1_refs),
        heldout_word_coverage: heldout_metrics.word_coverage,
        corrupt_ref_coverage: ratio(corrupt_metrics.covered_l1_refs, corrupt_metrics.l1_refs),
        real_vs_corrupt_gap: ratio(heldout_metrics.covered_l1_refs, heldout_metrics.l1_refs)
            - ratio(corrupt_metrics.covered_l1_refs, corrupt_metrics.l1_refs),
        model_hot_bytes,
        model_to_naive_ratio: ratio(model_hot_bytes, naive_bytes),
        build_ms: model.build_elapsed.as_secs_f64() * 1_000.0,
    }
}

fn build_motif_model(
    train_sequences: &[Vec<u32>],
    qualities: &[CenterQuality],
    config: ProbeConfig,
) -> MotifModel {
    let start = Instant::now();
    let mut candidates = HashMap::<Vec<u32>, usize>::new();
    for sequence in train_sequences {
        if sequence.len() < MOTIF_LEN {
            continue;
        }
        for window in sequence.windows(MOTIF_LEN) {
            if window_passes_quality(window, qualities, config.min_center_quality) {
                *candidates.entry(window.to_vec()).or_default() += 1;
            }
        }
    }

    let mut centers = candidates
        .into_iter()
        .filter(|(_, support)| *support >= MIN_MOTIF_SUPPORT)
        .collect::<Vec<_>>();
    centers.sort_by(
        |(left_sequence, left_support), (right_sequence, right_support)| {
            right_support
                .cmp(left_support)
                .then_with(|| left_sequence.cmp(right_sequence))
        },
    );
    let center_index = centers
        .into_iter()
        .enumerate()
        .map(|(id, (sequence, _))| (sequence, id as u32))
        .collect::<HashMap<_, _>>();

    MotifModel {
        motif_count: center_index.len(),
        center_index,
        build_elapsed: start.elapsed(),
    }
}

fn window_passes_quality(window: &[u32], qualities: &[CenterQuality], min_quality: f32) -> bool {
    let mut sum = 0.0;
    let mut count = 0usize;
    for center_id in window {
        let Some(quality) = qualities.get(*center_id as usize) else {
            return false;
        };
        sum += quality.quality;
        count += 1;
    }
    ratio_f32(sum, count) >= min_quality
}

fn encode_metrics(
    model: &MotifModel,
    sequences: &[Vec<u32>],
    covered_word_floor: f32,
) -> EncodeMetrics {
    let mut l1_refs = 0usize;
    let mut covered_l1_refs = 0usize;
    let mut covered_words = 0usize;
    let mut token_refs = 0usize;
    let mut residual_l1_refs = 0usize;

    for sequence in sequences {
        l1_refs += sequence.len();
        let mut word_covered_refs = 0usize;
        let mut position = 0usize;
        while position < sequence.len() {
            if position + MOTIF_LEN <= sequence.len() {
                let window = &sequence[position..position + MOTIF_LEN];
                if model.center_index.contains_key(window) {
                    covered_l1_refs += MOTIF_LEN;
                    word_covered_refs += MOTIF_LEN;
                    token_refs += 1;
                    position += MOTIF_LEN;
                    continue;
                }
            }
            residual_l1_refs += 1;
            token_refs += 1;
            position += 1;
        }
        if ratio(word_covered_refs, sequence.len()) >= covered_word_floor {
            covered_words += 1;
        }
    }

    EncodeMetrics {
        l1_refs,
        covered_l1_refs,
        word_coverage: ratio(covered_words, sequences.len()),
        token_refs,
        residual_l1_refs,
    }
}

fn center_qualities(l1: &L1CenterMemory) -> Vec<CenterQuality> {
    let max_support = l1
        .centers()
        .iter()
        .map(|center| center.support)
        .max()
        .unwrap_or(1);
    l1.centers()
        .iter()
        .map(|center| {
            let lane_dispersion = lane_dispersion(center.trits.map(|trit| trit.lane));
            let support_dampening = support_dampening(center.support, max_support);
            let quality = 0.55 * support_dampening + 0.45 * lane_dispersion;
            CenterQuality {
                support: center.support,
                lane_dispersion,
                support_dampening,
                quality,
            }
        })
        .collect()
}

fn lane_dispersion(mut lanes: [u16; 3]) -> f32 {
    lanes.sort();
    let mut distances = Vec::with_capacity(3);
    for pair in lanes.windows(2) {
        distances.push(u32::from(pair[1] - pair[0]));
    }
    distances.push(SURFACE_WAVE_DIM as u32 - u32::from(lanes[2] - lanes[0]));
    let min_distance = distances.into_iter().min().unwrap_or(0) as f32;
    (min_distance / (SURFACE_WAVE_DIM as f32 / 3.0)).clamp(0.0, 1.0)
}

fn support_dampening(support: u32, max_support: u32) -> f32 {
    if max_support <= 1 {
        return 1.0;
    }
    let support = (support.max(1) as f32).ln_1p();
    let max_support = (max_support as f32).ln_1p();
    (1.0 - support / max_support).clamp(0.0, 1.0)
}

fn quality_stats(qualities: &[CenterQuality]) -> QualityStats {
    let mut supports = qualities
        .iter()
        .map(|quality| quality.support)
        .collect::<Vec<_>>();
    supports.sort_unstable();
    let mut quality_values = qualities
        .iter()
        .map(|quality| quality.quality)
        .collect::<Vec<_>>();
    quality_values.sort_by(f32::total_cmp);
    QualityStats {
        support_p50: percentile_u32(&supports, 0.50),
        support_p90: percentile_u32(&supports, 0.90),
        support_p99: percentile_u32(&supports, 0.99),
        quality_p20: percentile_f32(&quality_values, 0.20),
        quality_p40: percentile_f32(&quality_values, 0.40),
        quality_p60: percentile_f32(&quality_values, 0.60),
    }
}

fn percentile_u32(values: &[u32], quantile: f32) -> u32 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) as f32 * quantile).round() as usize;
    values[index.min(values.len() - 1)]
}

fn percentile_f32(values: &[f32], quantile: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f32 * quantile).round() as usize;
    values[index.min(values.len() - 1)]
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn ratio_f32(numerator: f32, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f32
    }
}
