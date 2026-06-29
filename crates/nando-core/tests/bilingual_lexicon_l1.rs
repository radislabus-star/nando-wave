use std::path::PathBuf;

use nando_core::{SURFACE_WAVE_BYTES, SurfaceWave4096, surface_ngram_count};

#[derive(Clone, Copy, Debug)]
struct SurfaceCompileReport {
    words: usize,
    compiled: usize,
    skipped_short: usize,
    naive_wave_bytes: usize,
    average_active_lanes: f32,
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
        .unwrap_or_else(|error| panic!("{name} corpus file must be readable: {error}"))
        .lines()
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn bilingual_lexicon_foundation_has_real_ru_en_inputs() {
    let russian_hot = corpus_words("russian_words_300k.txt");
    let russian_cold = corpus_words("russian_words_danakt_full.txt");
    let english_hot = corpus_words("english_words_system_full.txt");
    let manifest = std::fs::read_to_string(corpus_path("lexicon_foundation_v1.json"))
        .expect("lexicon foundation manifest must be readable");

    assert_eq!(russian_hot.len(), 300_000);
    assert!(russian_cold.len() > 1_500_000);
    assert!(english_hot.len() > 70_000);
    assert!(manifest.contains("\"version\": \"lexicon-foundation-v1\""));
    assert!(manifest.contains("\"english_corpus_final\": false"));
    assert!(russian_hot.iter().any(|word| word == "волна"));
    assert!(russian_hot.iter().any(|word| word == "память"));
    assert!(english_hot.iter().any(|word| word == "memory"));
    assert!(english_hot.iter().any(|word| word == "wave"));
}

#[test]
fn bilingual_lexicon_compiles_to_l1_surface_waves_fast() {
    let russian = corpus_words("russian_words_300k.txt");
    let english = corpus_words("english_words_system_full.txt");
    let mut sample = Vec::with_capacity(40_000);
    sample.extend(russian.iter().take(20_000).cloned());
    sample.extend(english.iter().take(20_000).cloned());

    let report = compile_surface_report(&sample);
    eprintln!("fast bilingual L1 lexicon report: {report:#?}");

    assert_eq!(report.words, 40_000);
    assert!(report.compiled > 38_000, "report={report:?}");
    assert!(report.skipped_short < 2_000, "report={report:?}");
    assert_eq!(report.naive_wave_bytes, 40_000 * SURFACE_WAVE_BYTES);
    assert!(report.average_active_lanes > 9.0, "report={report:?}");
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
#[ignore = "heavy bilingual L1 corpus gate; run before large L1/L2 changes"]
fn bilingual_lexicon_full_hot_compiles_to_l1_surface_waves_heavy() {
    let russian = corpus_words("russian_words_300k.txt");
    let english = corpus_words("english_words_system_full.txt");
    let mut sample = Vec::with_capacity(russian.len() + english.len());
    sample.extend(russian);
    sample.extend(english);

    let report = compile_surface_report(&sample);
    eprintln!("heavy bilingual L1 lexicon report: {report:#?}");

    assert_eq!(report.words, 375_119);
    assert!(report.compiled > 373_500, "report={report:?}");
    assert!(report.skipped_short < 1_600, "report={report:?}");
    assert_eq!(report.naive_wave_bytes, 375_119 * SURFACE_WAVE_BYTES);
    assert!(report.average_active_lanes > 10.0, "report={report:?}");
}

fn compile_surface_report(words: &[String]) -> SurfaceCompileReport {
    let mut compiled = 0usize;
    let mut skipped_short = 0usize;
    let mut total_active_lanes = 0usize;
    let mut words_with_positive = 0usize;
    let mut words_with_negative = 0usize;

    for word in words {
        let ngrams = surface_ngram_count(word);
        if ngrams == 0 {
            skipped_short += 1;
            continue;
        }

        let wave = SurfaceWave4096::compile(word);
        compiled += 1;
        total_active_lanes += wave.active_lanes();
        if wave.positive_lanes() > 0 {
            words_with_positive += 1;
        }
        if wave.negative_lanes() > 0 {
            words_with_negative += 1;
        }
    }

    SurfaceCompileReport {
        words: words.len(),
        compiled,
        skipped_short,
        naive_wave_bytes: words.len() * SURFACE_WAVE_BYTES,
        average_active_lanes: total_active_lanes as f32 / compiled as f32,
        words_with_positive,
        words_with_negative,
    }
}
