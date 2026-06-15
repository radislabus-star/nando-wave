use nando_core::{SnapshotParseError, SpectrumSnapshot, run_stage2_tick};

pub(crate) fn save_snapshot(seed: u64, input_byte: u8, path: &str) -> Result<(), String> {
    let tick = run_stage2_tick(seed, input_byte);
    let bytes = tick.snapshot.to_bytes();
    let path = std::path::Path::new(path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }

    std::fs::write(path, bytes)
        .map_err(|error| format!("failed to write '{}': {error}", path.display()))?;

    println!("snapshot_saved: {}", path.display());
    println!("bytes: {}", bytes.len());
    println!("seed: {}", tick.snapshot.seed);
    println!("input_byte: {}", tick.snapshot.input_byte);
    println!("coherence: {:.6}", tick.snapshot.coherence);
    println!("spectral_entropy: {:.6}", tick.snapshot.spectral_entropy);
    Ok(())
}

pub(crate) fn read_snapshot(path: &str) -> Result<(), String> {
    let path = std::path::Path::new(path);
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let snapshot = SpectrumSnapshot::from_bytes(&bytes)
        .map_err(|error| format_snapshot_parse_error(path, error))?;

    println!("snapshot_read: {}", path.display());
    println!("version: {}", snapshot.version);
    println!("seed: {}", snapshot.seed);
    println!("input_byte: {}", snapshot.input_byte);
    println!("carrier_phase: {:.6}", snapshot.carrier.phase);
    println!("carrier_envelope: {:.6}", snapshot.carrier.envelope());
    println!("coherence: {:.6}", snapshot.coherence);
    println!("spectral_entropy: {:.6}", snapshot.spectral_entropy);
    println!("center_phase: {:.6}", snapshot.center_phase);
    println!("center_magnitude: {:.6}", snapshot.center_magnitude);
    println!("active_cell_ids: {:?}", snapshot.active_cell_ids);
    println!("top_slots: {:?}", snapshot.top_slots);
    Ok(())
}

fn format_snapshot_parse_error(path: &std::path::Path, error: SnapshotParseError) -> String {
    format!("failed to parse '{}': {error}", path.display())
}
