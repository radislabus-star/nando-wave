use std::path::PathBuf;

use nando_core::{
    L1CenterMemoryConfig, L1CenterMemoryProof, L1CenterMemoryVerdict, SURFACE_WAVE_BYTES,
};

fn corpus_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/corpus")
        .join(name)
}

fn corpus_words(name: &str) -> Vec<String> {
    std::fs::read_to_string(corpus_path(name))
        .expect("russian corpus file must be readable")
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn russian_l1_center_memory_proves_surface_centers_fast() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = L1CenterMemoryProof::prove(
        words[..8_000].iter().map(String::as_str),
        words[8_000..10_000].iter().map(String::as_str),
        fast_config(),
    );
    eprintln!("fast russian L1 center-memory proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        L1CenterMemoryVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 8_000);
    assert_eq!(proof.heldout_words, 2_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.center_count > 1_000, "proof={proof:#?}");
    assert!(proof.coverage_pass, "proof={proof:#?}");
    assert!(proof.reconstruction_pass, "proof={proof:#?}");
    assert!(proof.fourier_pass, "proof={proof:#?}");
    assert!(proof.ablation_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.promotion_ready_for_l2, "proof={proof:#?}");
    assert_eq!(proof.naive_total_wave_bytes, 10_000 * SURFACE_WAVE_BYTES);
}

#[test]
#[ignore = "heavy 240k/60k Russian L1 center-memory gate; run explicitly before release"]
fn russian_l1_center_memory_proves_240k_60k_surface_centers_heavy() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = L1CenterMemoryProof::prove(
        words[..240_000].iter().map(String::as_str),
        words[240_000..300_000].iter().map(String::as_str),
        L1CenterMemoryConfig {
            min_center_support: 2,
            min_heldout_ngram_coverage: 0.82,
            min_average_reconstruction_similarity: 0.80,
            min_average_fourier_similarity: 0.75,
            min_fourier_ablation_drop: 0.03,
            min_real_vs_corrupt_coverage_gap: 0.15,
            max_model_to_naive_ratio: 0.08,
            max_corrupt_eval_words: 8_192,
            max_fourier_eval_words: 4_096,
            ..L1CenterMemoryConfig::default()
        },
    );
    eprintln!("heavy russian L1 center-memory proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        L1CenterMemoryVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 240_000);
    assert_eq!(proof.heldout_words, 60_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.coverage_pass, "proof={proof:#?}");
    assert!(proof.reconstruction_pass, "proof={proof:#?}");
    assert!(proof.fourier_pass, "proof={proof:#?}");
    assert!(proof.ablation_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.promotion_ready_for_l2, "proof={proof:#?}");
    assert_eq!(proof.naive_total_wave_bytes, 300_000 * SURFACE_WAVE_BYTES);
}

fn fast_config() -> L1CenterMemoryConfig {
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
    }
}
