use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use nando_transition_inducer::{
    LIVE_TRANSITION_MAX_REQUEST_BYTES, LiveTransitionExecutor, LiveTransitionRequest,
    LiveTransitionResponse,
};

fn main() {
    let started = std::time::Instant::now();
    let response = run().unwrap_or_else(|reason| {
        LiveTransitionResponse::decline(
            reason,
            u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        )
    });
    println!(
        "{}",
        serde_json::to_string(&response).unwrap_or_else(|_| {
            "{\"local_accept\":false,\"verifier_ok\":false,\"false_accepts\":0,\"reason\":\"serialization\",\"elapsed_ns\":0}".to_owned()
        })
    );
}

fn run() -> Result<LiveTransitionResponse, String> {
    if env::var("NANDO_CLIENT_KILL_SWITCH").is_ok_and(|value| value == "1") {
        return Err("global_kill_switch".to_owned());
    }
    let registry_path = env::var_os("NANDO_TRANSITION_REGISTRY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/nando-wave/transition/registry.json"));
    let request = read_request()?;
    let executor = LiveTransitionExecutor::load(&registry_path)?;
    Ok(executor.execute(&request))
}

fn read_request() -> Result<LiveTransitionRequest, String> {
    let mut input = Vec::new();
    io::stdin()
        .take(LIVE_TRANSITION_MAX_REQUEST_BYTES.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("request_read:{error}"))?;
    if input.len() > LIVE_TRANSITION_MAX_REQUEST_BYTES {
        return Err("request_too_large".to_owned());
    }
    serde_json::from_slice(&input).map_err(|error| format!("request_json:{error}"))
}
