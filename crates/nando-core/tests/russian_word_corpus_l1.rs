use std::path::PathBuf;

use nando_core::{SURFACE_WAVE_BYTES, SurfaceWave4096, surface_ngram_count};

#[derive(Clone, Copy, Debug)]
struct CorpusSurfaceReport {
    words: usize,
    compiled: usize,
    skipped_short: usize,
    naive_wave_bytes: usize,
    average_active_lanes: f32,
    average_ngrams: f32,
    words_with_positive: usize,
    words_with_negative: usize,
}

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
fn russian_word_corpus_has_real_large_inputs() {
    let words_300k = corpus_words("russian_words_300k.txt");
    let full_words = corpus_words("russian_words_danakt_full.txt");

    assert_eq!(words_300k.len(), 300_000);
    assert!(
        full_words.len() > 1_500_000,
        "full corpus too small: {}",
        full_words.len()
    );
    assert_eq!(words_300k[0], "и");
    assert!(words_300k.iter().any(|word| word == "волна"));
    assert!(words_300k.iter().any(|word| word == "память"));
    assert!(!words_300k.iter().any(|word| word == "quot"));
}

#[test]
fn thirty_thousand_russian_words_compile_to_l1_surface_waves_fast() {
    let words = corpus_words("russian_words_300k.txt");
    let report = compile_surface_report(&words[..30_000]);

    assert_eq!(report.words, 30_000);
    assert!(report.compiled > 29_000, "report={report:?}");
    assert!(report.skipped_short < 1_000, "report={report:?}");
    assert_eq!(report.naive_wave_bytes, 245_760_000);
    assert!(report.average_active_lanes > 10.0, "report={report:?}");
    assert!(report.average_ngrams > 10.0, "report={report:?}");
    assert!(
        report.words_with_positive > report.compiled * 95 / 100,
        "report={report:?}"
    );
    assert!(
        report.words_with_negative > report.compiled * 95 / 100,
        "report={report:?}"
    );
}

#[test]
#[ignore = "heavy 300k Russian corpus L1 gate; run explicitly before release"]
fn three_hundred_thousand_russian_words_compile_to_l1_surface_waves_heavy() {
    let words = corpus_words("russian_words_300k.txt");
    let report = compile_surface_report(&words);

    assert_eq!(report.words, 300_000);
    assert!(report.compiled > 299_000, "report={report:?}");
    assert!(report.skipped_short < 1_000, "report={report:?}");
    assert_eq!(report.naive_wave_bytes, 2_457_600_000);
    assert!(report.average_active_lanes > 10.0, "report={report:?}");
    assert!(report.average_ngrams > 10.0, "report={report:?}");
    assert!(
        report.words_with_positive > report.compiled * 95 / 100,
        "report={report:?}"
    );
    assert!(
        report.words_with_negative > report.compiled * 95 / 100,
        "report={report:?}"
    );
}

fn compile_surface_report(words: &[String]) -> CorpusSurfaceReport {
    let mut compiled = 0usize;
    let mut skipped_short = 0usize;
    let mut total_active_lanes = 0usize;
    let mut words_with_positive = 0usize;
    let mut words_with_negative = 0usize;
    let mut total_ngrams = 0usize;

    for word in words {
        let ngrams = surface_ngram_count(word);
        if ngrams == 0 {
            skipped_short += 1;
            continue;
        }

        let wave = SurfaceWave4096::compile(word);
        compiled += 1;
        total_ngrams += ngrams;
        total_active_lanes += wave.active_lanes();
        if wave.positive_lanes() > 0 {
            words_with_positive += 1;
        }
        if wave.negative_lanes() > 0 {
            words_with_negative += 1;
        }
    }

    let naive_wave_bytes = words.len() * SURFACE_WAVE_BYTES;
    let average_active_lanes = total_active_lanes as f32 / compiled as f32;
    let average_ngrams = total_ngrams as f32 / compiled as f32;

    CorpusSurfaceReport {
        words: words.len(),
        compiled,
        skipped_short,
        naive_wave_bytes,
        average_active_lanes,
        average_ngrams,
        words_with_positive,
        words_with_negative,
    }
}
