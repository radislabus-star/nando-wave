use std::path::Path;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() == 2 && arguments[0] == "--attempt-index-only" {
        let measurement =
            nando_transition_serving::k1_exact_opportunity_replay::measure_frozen_attempt_index(
                Path::new(&arguments[1]),
            )?;
        println!(
            "{}",
            serde_json::to_string(&measurement)
                .map_err(|error| format!("k1_exact_attempt_index_measurement_encode:{error}"))?
        );
        return Ok(());
    }
    if arguments.len() != 1 {
        return Err(
            "usage: nando-k1-exact-opportunity-replay [--attempt-index-only] SNAPSHOT_ROOT"
                .to_owned(),
        );
    }
    let replay = nando_transition_serving::k1_exact_opportunity_replay::replay_frozen_snapshot(
        Path::new(&arguments[0]),
    )?;
    println!(
        "{}",
        serde_json::to_string(&replay)
            .map_err(|error| format!("k1_exact_replay_encode:{error}"))?
    );
    Ok(())
}
