use std::path::PathBuf;

use nando_core::{
    SurfaceWaveLmConfig, SurfaceWordGrokkingConfig, SurfaceWordGrokkingProof,
    SurfaceWordGrokkingVerdict,
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
fn russian_words_have_surface_grokking_signal_without_exact_lookup_fast() {
    let words = corpus_words("russian_words_300k.txt");
    let train = &words[..20_000];
    let heldout = &words[20_000..25_000];

    let proof = SurfaceWordGrokkingProof::prove(
        train.iter().map(String::as_str),
        heldout.iter().map(String::as_str),
        &[],
        SurfaceWordGrokkingConfig {
            lm_config: SurfaceWaveLmConfig {
                context_ngrams: 10,
                epochs: 2,
                learning_rate: 1,
            },
            min_heldout_accuracy: 0.30,
            min_real_vs_corrupt_margin: 1.0,
            max_model_to_naive_ratio: 0.02,
            max_corrupt_eval_words: 2_048,
            require_no_exact_lookup_overlap: true,
        },
    );
    eprintln!("fast russian word grokking proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        SurfaceWordGrokkingVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 20_000);
    assert_eq!(proof.heldout_words, 5_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.heldout_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.anti_lookup_pass, "proof={proof:#?}");
    assert!(proof.lift_over_random > 10.0, "proof={proof:#?}");
}

#[test]
#[ignore = "heavy 300k Russian word grokking gate; run explicitly before release"]
fn three_hundred_thousand_russian_words_have_surface_grokking_signal_heavy() {
    let words = corpus_words("russian_words_300k.txt");
    let train = &words[..240_000];
    let heldout = &words[240_000..300_000];

    let proof = SurfaceWordGrokkingProof::prove(
        train.iter().map(String::as_str),
        heldout.iter().map(String::as_str),
        &[],
        SurfaceWordGrokkingConfig {
            lm_config: SurfaceWaveLmConfig {
                context_ngrams: 10,
                epochs: 2,
                learning_rate: 1,
            },
            min_heldout_accuracy: 0.30,
            min_real_vs_corrupt_margin: 1.0,
            max_model_to_naive_ratio: 0.002,
            max_corrupt_eval_words: 8_192,
            require_no_exact_lookup_overlap: true,
        },
    );
    eprintln!("heavy russian word grokking proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        SurfaceWordGrokkingVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 240_000);
    assert_eq!(proof.heldout_words, 60_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.heldout_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.anti_lookup_pass, "proof={proof:#?}");
}
