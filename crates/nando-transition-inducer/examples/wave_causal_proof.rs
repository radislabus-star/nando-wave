use std::path::PathBuf;

use nando_transition_inducer::run_wave_causal_proof;

fn main() -> Result<(), String> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "usage: wave_causal_proof <output.json>".to_owned())?;
    let report = run_wave_causal_proof()?;
    let bytes = serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?;
    std::fs::write(&output, bytes).map_err(|error| error.to_string())?;
    if report.verdicts.core_causal_pass {
        Ok(())
    } else {
        Err("Wave causal proof gate did not pass".to_owned())
    }
}
