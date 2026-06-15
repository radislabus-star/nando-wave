use nando_core::{CacheAwareOrganPlan, project_status, run_stage2_tick};

pub(crate) fn print_status() {
    let status = project_status();
    println!("{} status", status.name);
    println!("stage: {}", status.stage);
    println!("scope: {}", status.scope);
    println!("rust_first: {}", status.rust_first);
    println!("cell32_bytes: {}", status.cell32_bytes);
    println!("planned_organ128_bytes: {}", status.planned_organ128_bytes);
    println!("planned_organ192_bytes: {}", status.planned_organ192_bytes);
    println!("core_learning: tiny_live_byte_adapter");
    println!("feedback_learning: primitive_live_cycle_plus_online_adapter");
    println!("text_generation: chat0_once_short_response");
}

pub(crate) fn print_organ128_plan() {
    let plan = CacheAwareOrganPlan::t480_organ128();

    println!("Nando Wave Organ128 cache plan");
    println!("target_cpu: Intel Core i7-8650U / T480");
    println!("cell_atom: Cell32");
    println!("cell_bytes: {}", nando_core::CELL32_BYTES);
    println!("cores: {}", plan.profile.cores);
    println!("l1d_bytes_per_core: {}", plan.profile.l1d_bytes_per_core);
    println!("l2_bytes_per_core: {}", plan.profile.l2_bytes_per_core);
    println!("l3_bytes_shared: {}", plan.profile.l3_bytes_shared);
    println!("organ128_cells: {}", plan.organ128.cell_count());
    println!("organ128_bytes: {}", plan.organ128_bytes);
    println!(
        "organ128_mib: {:.2}",
        plan.organ128_bytes as f64 / 1024.0 / 1024.0
    );
    println!("fast_cells: {}", plan.organ128.fast_cells);
    println!("mid_cells: {}", plan.organ128.mid_cells);
    println!("guard_cells: {}", plan.organ128.guard_cells);
    println!("carrier_cells: {}", plan.organ128.carrier_cells);
    println!("memory_cells: {}", plan.organ128.memory_cells);
    println!(
        "l1_active_cells_total: {}",
        plan.hot_window.l1_active_cells_total
    );
    println!("l2_hot_cells_total: {}", plan.hot_window.l2_hot_cells_total);
    println!(
        "l3_warm_cells_target: {}",
        plan.hot_window.l3_warm_cells_target
    );
    println!("l3_warm_cells_max: {}", plan.hot_window.l3_warm_cells_max);
    println!("ram_cold_cells_min: {}", plan.hot_window.ram_cold_cells_min);
    println!("l3_target_bytes: {}", plan.l3_target_bytes);
    println!("l3_max_bytes: {}", plan.l3_max_bytes);
}

pub(crate) fn print_wave_tick(seed: u64, input_byte: u8) {
    let tick = run_stage2_tick(seed, input_byte);

    println!("Nando Wave stage-2 tick");
    println!("seed: {}", tick.trace.seed);
    println!("input_byte: {}", tick.trace.input_byte);
    println!("carrier_phase: {:.6}", tick.carrier.phase);
    println!("carrier_envelope: {:.6}", tick.carrier.envelope());
    println!("cells_scanned: {}", tick.trace.cells_scanned);
    println!("active_count: {}", tick.trace.active_count);
    println!("active_cell_ids: {:?}", tick.trace.active_cell_ids);
    println!("top_resonance: {:.6}", tick.trace.top_resonance);
    println!("coherence: {:.6}", tick.trace.coherence);
    println!("spectral_entropy: {:.6}", tick.trace.spectral_entropy);
    println!("center_phase: {:.6}", tick.trace.center_phase);
    println!("center_magnitude: {:.6}", tick.trace.center_magnitude);
    println!("snapshot_version: {}", tick.snapshot.version);
    println!("snapshot_top_slots: {:?}", tick.snapshot.top_slots);
}
