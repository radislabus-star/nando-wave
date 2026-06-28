use nando_core::{SURFACE_WAVE_BYTES, SurfaceMotifBank, SurfaceMotifSpec};

fn ten_thousand_url_like_pages() -> Vec<String> {
    (0..10_000)
        .map(|index| match index % 4 {
            0 => format!("https://mirror.dxdy.ru/topic{index:04}.html"),
            1 => format!("https://mirror.dxdy.ru/post{index:04}.html#p{index:04}"),
            2 => format!("https://docs.rs/nando-wave/{index:04}/surface_wave/index.html"),
            _ => format!("https://github.com/nando-wave/core/issues/{index:04}"),
        })
        .collect()
}

#[test]
fn ten_thousand_surfaces_store_as_motifs_plus_residuals() {
    let pages = ten_thousand_url_like_pages();
    let bank = SurfaceMotifBank::build(
        pages.iter().map(String::as_str),
        SurfaceMotifSpec {
            min_support: 64,
            max_motifs: 64,
            ..SurfaceMotifSpec::default()
        },
    );

    assert_eq!(bank.records.len(), 10_000);
    assert_eq!(bank.naive_wave_bytes, 10_000 * SURFACE_WAVE_BYTES);
    assert!(!bank.motifs.is_empty());
    assert!(bank.motif_cold_bytes > 0);
    assert!(
        bank.encoded_bytes < bank.naive_wave_bytes / 10,
        "encoded={} naive={} ratio={}",
        bank.encoded_bytes,
        bank.naive_wave_bytes,
        bank.compression_ratio()
    );
    assert!(bank.bytes_saved() > 0);

    let dxdy_topic = bank.record_motif_ids(0);
    let dxdy_topic_next = bank.record_motif_ids(4);
    let github_issue = bank.record_motif_ids(3);

    assert!(!dxdy_topic.is_empty());
    assert_eq!(dxdy_topic, dxdy_topic_next);
    assert_ne!(dxdy_topic, github_issue);
}
