use nando_client_connector::{
    ConnectorConfig, DEFAULT_CONNECT_TIMEOUT, DEFAULT_LISTEN, DEFAULT_MAX_CONNECTIONS,
    DEFAULT_UPSTREAM, check_upstream, run,
};
use std::env;
use std::process::ExitCode;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match execute(env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("nando-connector: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut listen = env::var("NANDO_CONNECT_LISTEN").unwrap_or_else(|_| DEFAULT_LISTEN.to_owned());
    let mut upstream =
        env::var("NANDO_CONNECT_UPSTREAM").unwrap_or_else(|_| DEFAULT_UPSTREAM.to_owned());
    let mut max_connections = DEFAULT_MAX_CONNECTIONS;
    let mut connect_timeout = DEFAULT_CONNECT_TIMEOUT;
    let mut check_only = false;
    let mut args = args.peekable();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--listen" => listen = next_value(&mut args, "--listen")?,
            "--upstream" => upstream = next_value(&mut args, "--upstream")?,
            "--max-connections" => {
                max_connections = parse_usize(&next_value(&mut args, "--max-connections")?)?;
            }
            "--connect-timeout-ms" => {
                let milliseconds = parse_u64(&next_value(&mut args, "--connect-timeout-ms")?)?;
                connect_timeout = Duration::from_millis(milliseconds);
            }
            "--check" => check_only = true,
            "--version" | "-V" => {
                println!("nando-connector {VERSION}");
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    let config = ConnectorConfig::new(&listen, upstream, max_connections, connect_timeout)?;
    if check_only {
        check_upstream(&config).map_err(|error| format!("health check failed: {error}"))?;
        println!("NANDO_REMOTE_OK {}", config.upstream);
        return Ok(());
    }

    check_upstream(&config).map_err(|error| format!("startup health check failed: {error}"))?;
    eprintln!(
        "nando-connect: listening on {} -> {} (max_connections={})",
        config.listen, config.upstream, config.max_connections
    );
    run(config).map_err(|error| format!("connector stopped: {error}"))
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {option}"))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid positive integer {value}: {error}"))
}

fn parse_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid positive integer {value}: {error}"))
}

fn print_help() {
    println!(
        "nando-connector {VERSION}

Protocol-transparent client transport for Nando.

Usage:
  nando-connector [OPTIONS]
  nando-connector --check [OPTIONS]

Options:
  --listen ADDRESS             Loopback listener (default: {DEFAULT_LISTEN})
  --upstream HOST:PORT         Nando server (default: {DEFAULT_UPSTREAM})
  --max-connections COUNT      Concurrent connection limit (default: {DEFAULT_MAX_CONNECTIONS})
  --connect-timeout-ms MS      Upstream connection timeout (default: 2000)
  --check                      Verify upstream health and exit
  -V, --version                Print version
  -h, --help                   Print help"
    );
}
