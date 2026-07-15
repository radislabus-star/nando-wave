use std::path::PathBuf;

use nando_transition_inducer::a2_lab::build_a2_live_smoke_fixture;

fn main() -> Result<(), String> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "usage: a2_live_smoke_fixture <output.json>".to_owned())?;
    let fixture = build_a2_live_smoke_fixture()?;
    let bytes = serde_json::to_vec_pretty(&fixture).map_err(|error| error.to_string())?;
    std::fs::write(output, bytes).map_err(|error| error.to_string())
}
