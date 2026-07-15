use std::env;
use std::fs;

use nando_transition_inducer::a1_lab::run_a1_proof;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_a1_proof().map_err(std::io::Error::other)?;
    let json = serde_json::to_string_pretty(&report)?;
    if let Some(path) = env::args().nth(1) {
        fs::write(path, format!("{json}\n"))?;
    } else {
        println!("{json}");
    }
    if report.verdicts.overall_pass {
        Ok(())
    } else {
        Err("A1 proof gate did not pass".into())
    }
}
