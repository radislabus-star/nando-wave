use std::path::PathBuf;

use nando_core::{
    L1_SEQUENCE_REF_BYTES, L1CenterMemoryConfig, L2CenterMemoryConfig, L2CenterMemoryProof,
    L2CenterMemoryVerdict,
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
fn russian_l2_center_memory_proves_sequence_motifs_fast() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = L2CenterMemoryProof::prove(
        words[..20_000].iter().map(String::as_str),
        words[20_000..25_000].iter().map(String::as_str),
        fast_config(),
    );
    eprintln!("fast russian L2 center-memory proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        L2CenterMemoryVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 20_000);
    assert_eq!(proof.heldout_words, 5_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.l1_center_count > 1_000, "proof={proof:#?}");
    assert!(proof.l2_center_count > 500, "proof={proof:#?}");
    assert!(proof.coverage_pass, "proof={proof:#?}");
    assert!(proof.sequence_pass, "proof={proof:#?}");
    assert!(proof.fourier_pass, "proof={proof:#?}");
    assert!(proof.ablation_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.promotion_ready_for_l3, "proof={proof:#?}");
    assert!(proof.naive_total_l1_sequence_bytes > 10_000 * L1_SEQUENCE_REF_BYTES);
}

#[test]
#[ignore = "heavy 240k/60k Russian L2 center-memory gate; run explicitly before release"]
fn russian_l2_center_memory_proves_240k_60k_sequence_motifs_heavy() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = L2CenterMemoryProof::prove(
        words[..240_000].iter().map(String::as_str),
        words[240_000..300_000].iter().map(String::as_str),
        L2CenterMemoryConfig {
            l1_config: L1CenterMemoryConfig {
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
            motif_len: 4,
            min_motif_support: 4,
            min_heldout_ref_coverage: 0.80,
            min_heldout_word_coverage: 0.75,
            min_average_sequence_similarity: 0.88,
            min_average_fourier_similarity: 0.85,
            min_fourier_ablation_drop: 0.25,
            min_real_vs_corrupt_coverage_gap: 0.35,
            max_model_to_naive_ratio: 0.60,
            max_corrupt_eval_words: 8_192,
            max_fourier_eval_words: 4_096,
            ..L2CenterMemoryConfig::default()
        },
    );
    eprintln!("heavy russian L2 center-memory proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        L2CenterMemoryVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 240_000);
    assert_eq!(proof.heldout_words, 60_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.coverage_pass, "proof={proof:#?}");
    assert!(proof.sequence_pass, "proof={proof:#?}");
    assert!(proof.fourier_pass, "proof={proof:#?}");
    assert!(proof.ablation_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.promotion_ready_for_l3, "proof={proof:#?}");
}

fn fast_config() -> L2CenterMemoryConfig {
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
        motif_len: 3,
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
    }
}
