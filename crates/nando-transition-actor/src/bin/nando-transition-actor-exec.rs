use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use nando_transition_actor::{
    LiveExecutionResult, TransitionPackage, TransitionRequest, execute_live_request,
};

const DEFAULT_MAX_REQUEST_BYTES: usize = 1_048_576;

fn main() {
    let result = run().unwrap_or_else(LiveExecutionResult::decline);
    match serde_json::to_string(&result) {
        Ok(payload) => println!("{payload}"),
        Err(error) => println!(
            "{{\"local_accept\":false,\"verifier_ok\":false,\"false_accepts\":0,\"reason\":\"serialization:{error}\"}}"
        ),
    }
}

fn run() -> Result<LiveExecutionResult, String> {
    let package_path = package_path()?;
    let package_bytes = fs::read(&package_path)
        .map_err(|error| format!("package_read:{}:{error}", package_path.display()))?;
    let package: TransitionPackage =
        serde_json::from_slice(&package_bytes).map_err(|error| format!("package_json:{error}"))?;

    let max_request_bytes = env::var("NANDO_TYPED_ACTOR_MAX_REQUEST_BYTES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_REQUEST_BYTES);
    let mut input = Vec::new();
    io::stdin()
        .take(max_request_bytes.saturating_add(1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("request_read:{error}"))?;
    if input.len() > max_request_bytes {
        return Err("request_too_large".to_owned());
    }
    let request: TransitionRequest =
        serde_json::from_slice(&input).map_err(|error| format!("request_json:{error}"))?;
    Ok(execute_live_request(&package, &request))
}

fn package_path() -> Result<PathBuf, String> {
    let mut args = env::args_os().skip(1);
    if let Some(flag) = args.next() {
        if flag != "--package" {
            return Err("usage: nando-transition-actor-exec --package PATH".to_owned());
        }
        let Some(path) = args.next() else {
            return Err("package_path_missing".to_owned());
        };
        if args.next().is_some() {
            return Err("unexpected_arguments".to_owned());
        }
        return Ok(PathBuf::from(path));
    }
    env::var_os("NANDO_TYPED_ACTOR_PACKAGE")
        .map(PathBuf::from)
        .ok_or_else(|| "package_path_missing".to_owned())
}
