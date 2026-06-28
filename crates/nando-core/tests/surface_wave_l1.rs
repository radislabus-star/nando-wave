use nando_core::{
    SURFACE_WAVE_BYTES, SURFACE_WAVE_DIM, SURFACE_WAVE_NGRAM, SURFACE_WAVE_TRITS, SurfaceWave4096,
    surface_ngram_count,
};

#[test]
fn url_surface_wire_compiles_to_fixed_l1_wave() {
    let base = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3420.html");
    let nearby = SurfaceWave4096::compile("https://mirror.dxdy.ru/topic3421.html");
    let unrelated = SurfaceWave4096::compile("ssh service externally exposed firewall");

    assert_eq!(SURFACE_WAVE_NGRAM, 4);
    assert_eq!(SURFACE_WAVE_DIM, 4_096);
    assert_eq!(SURFACE_WAVE_TRITS, 3);
    assert_eq!(SURFACE_WAVE_BYTES, 8_192);
    assert!(surface_ngram_count("https://mirror.dxdy.ru/topic3420.html") > 0);
    assert!(base.active_lanes() > 0);
    assert!(base.positive_lanes() > 0);
    assert!(base.negative_lanes() > 0);

    let nearby_score = base.cosine_similarity(&nearby);
    let unrelated_score = base.cosine_similarity(&unrelated);
    assert!(nearby_score > 0.80, "nearby_score={nearby_score}");
    assert!(
        unrelated_score < nearby_score - 0.35,
        "nearby_score={nearby_score} unrelated_score={unrelated_score}"
    );
}
