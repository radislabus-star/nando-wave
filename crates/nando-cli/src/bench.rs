use std::hint::black_box;
use std::time::{Duration, Instant};

use nando_core::{
    BytePhaseLut, CarrierWave, Cell32Learner, LinkProfile, LinkTissue, Stage2Organ, TickTrace,
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

fn bench_input(index: usize) -> u8 {
    (index as u8)
        .wrapping_mul(37)
        .wrapping_add((index >> 8) as u8)
        .wrapping_add(11)
}

fn print_row(name: &str, duration: Duration, ticks: usize) {
    let seconds = duration.as_secs_f64();
    let ns_per_tick = duration.as_nanos() as f64 / ticks as f64;
    let ticks_per_second = ticks as f64 / seconds.max(f64::EPSILON);
    println!("{name}.total_ms: {:.3}", seconds * 1000.0);
    println!("{name}.ns_per_tick: {:.1}", ns_per_tick);
    println!("{name}.ticks_per_second: {:.0}", ticks_per_second);
}
