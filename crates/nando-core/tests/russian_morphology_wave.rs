use std::path::PathBuf;

use nando_core::{
    MorphologyGrokkingProof, MorphologyGrokkingVerdict, MorphologyScalingReport,
    MorphologyWaveBank, MorphologyWaveConfig,
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
fn morphology_bank_extracts_productive_atoms_not_exact_words() {
    let words = corpus_words("russian_words_300k.txt");
    let bank = MorphologyWaveBank::build(words[..20_000].iter().map(String::as_str), fast_config());

    assert!(bank.atom_count() > 128, "atoms={}", bank.atom_count());

    let extraction = words[20_000..25_000]
        .iter()
        .find_map(|word| bank.extract(word))
        .expect("heldout word should extract through a productive morphology atom");
    assert!(extraction.stem.len() < "газотурбинными".len());
    assert!(extraction.ending.chars().count() >= fast_config().min_ending_chars);
    assert!(extraction.support >= fast_config().min_support);
    assert!(extraction.stem_diversity >= fast_config().min_stem_diversity);
}

#[test]
fn russian_morphology_wave_grokks_heldout_endings_fast() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = MorphologyGrokkingProof::prove(
        words[..20_000].iter().map(String::as_str),
        words[20_000..25_000].iter().map(String::as_str),
        fast_config(),
    );
    eprintln!("fast russian morphology proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        MorphologyGrokkingVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 20_000);
    assert_eq!(proof.heldout_words, 5_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.atom_count > 128, "proof={proof:#?}");
    assert!(proof.extraction_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.anti_lookup_pass, "proof={proof:#?}");
}

#[test]
#[ignore = "scaling exploration; sorted full corpus is non-stationary, not a release gate"]
fn russian_morphology_scaling_curve_shows_saturation_region() {
    let words = corpus_words("russian_words_danakt_full.txt");
    let report = MorphologyScalingReport::from_words(
        &words,
        &[
            5_000, 10_000, 20_000, 40_000, 80_000, 160_000, 240_000, 300_000, 600_000,
        ],
        20_000,
    );
    eprintln!("russian morphology scaling report: {report:#?}");

    assert_eq!(report.rows.len(), 9);
    assert!(report.rows.iter().any(|row| row.heldout_coverage > 0.85));
}

#[test]
#[ignore = "heavy 300k Russian morphology-wave gate; run explicitly before release"]
fn three_hundred_thousand_russian_words_have_morphology_wave_signal_heavy() {
    let words = corpus_words("russian_words_300k.txt");
    let proof = MorphologyGrokkingProof::prove(
        words[..240_000].iter().map(String::as_str),
        words[240_000..300_000].iter().map(String::as_str),
        MorphologyWaveConfig {
            min_support: 128,
            min_stem_diversity: 128,
            min_heldout_coverage: 0.72,
            min_real_vs_corrupt_coverage_gap: 0.20,
            max_model_to_naive_ratio: 0.0002,
            max_corrupt_eval_words: 16_384,
            ..fast_config()
        },
    );
    eprintln!("heavy russian morphology proof: {proof:#?}");

    assert_eq!(
        proof.verdict,
        MorphologyGrokkingVerdict::Proven,
        "proof={proof:#?}"
    );
    assert_eq!(proof.train_words, 240_000);
    assert_eq!(proof.heldout_words, 60_000);
    assert_eq!(proof.exact_lookup_heldout_hits, 0);
    assert!(proof.extraction_pass, "proof={proof:#?}");
    assert!(proof.corrupt_reject_pass, "proof={proof:#?}");
    assert!(proof.compression_pass, "proof={proof:#?}");
    assert!(proof.anti_lookup_pass, "proof={proof:#?}");
}

fn fast_config() -> MorphologyWaveConfig {
    MorphologyWaveConfig {
        min_support: 24,
        min_stem_diversity: 24,
        min_ending_chars: 2,
        min_heldout_coverage: 0.60,
        min_real_vs_corrupt_coverage_gap: 0.18,
        max_model_to_naive_ratio: 0.005,
        max_corrupt_eval_words: 4_096,
        ..MorphologyWaveConfig::default()
    }
}
