use nando_client_connector::{
    ClientFallbackConfig, ConnectorConfig, DEFAULT_CONNECT_TIMEOUT,
    DEFAULT_FALLBACK_CONNECT_TIMEOUT, DEFAULT_FALLBACK_HOST, DEFAULT_FALLBACK_IO_TIMEOUT,
    DEFAULT_FALLBACK_PORT, DEFAULT_LISTEN, DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_REPLAY_BODY_BYTES,
    DEFAULT_METRICS_LISTEN, DEFAULT_REPLAY_MEMORY_BYTES, DEFAULT_UPSTREAM, check_upstream, run,
};
use std::env;
use std::path::PathBuf;
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
    let mut metrics_listen = env::var("NANDO_CONNECT_METRICS_LISTEN")
        .unwrap_or_else(|_| DEFAULT_METRICS_LISTEN.to_owned());
    let mut upstream =
        env::var("NANDO_CONNECT_UPSTREAM").unwrap_or_else(|_| DEFAULT_UPSTREAM.to_owned());
    let mut max_connections = DEFAULT_MAX_CONNECTIONS;
    let mut connect_timeout = DEFAULT_CONNECT_TIMEOUT;
    let mut client_fallback = env_flag("NANDO_CONNECT_CLIENT_FALLBACK")?;
    let mut fallback_host = env::var("NANDO_CONNECT_FALLBACK_HOST")
        .unwrap_or_else(|_| DEFAULT_FALLBACK_HOST.to_owned());
    let mut fallback_port = env::var("NANDO_CONNECT_FALLBACK_PORT")
        .map_or(Ok(DEFAULT_FALLBACK_PORT), |value| parse_u16(&value))?;
    let mut fallback_connect_timeout = env::var("NANDO_CONNECT_FALLBACK_CONNECT_TIMEOUT_MS")
        .map_or(Ok(DEFAULT_FALLBACK_CONNECT_TIMEOUT), |value| {
            parse_u64(&value).map(Duration::from_millis)
        })?;
    let mut fallback_io_timeout = env::var("NANDO_CONNECT_FALLBACK_IO_TIMEOUT_MS")
        .map_or(Ok(DEFAULT_FALLBACK_IO_TIMEOUT), |value| {
            parse_u64(&value).map(Duration::from_millis)
        })?;
    let mut max_replay_body_bytes = env::var("NANDO_CONNECT_MAX_REPLAY_BODY_MIB")
        .map_or(Ok(DEFAULT_MAX_REPLAY_BODY_BYTES), |value| parse_mib(&value))?;
    let mut replay_memory_bytes = env::var("NANDO_CONNECT_REPLAY_MEMORY_KIB")
        .map_or(Ok(DEFAULT_REPLAY_MEMORY_BYTES), |value| parse_kib(&value))?;
    let mut spool_dir = env::var_os("NANDO_CONNECT_SPOOL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(default_spool_dir);
    let mut allow_degraded_start = env_flag("NANDO_CONNECT_ALLOW_DEGRADED_START")?;
    let mut check_only = false;
    let mut args = args.peekable();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--listen" => listen = next_value(&mut args, "--listen")?,
            "--metrics-listen" => metrics_listen = next_value(&mut args, "--metrics-listen")?,
            "--upstream" => upstream = next_value(&mut args, "--upstream")?,
            "--max-connections" => {
                max_connections = parse_usize(&next_value(&mut args, "--max-connections")?)?;
            }
            "--connect-timeout-ms" => {
                let milliseconds = parse_u64(&next_value(&mut args, "--connect-timeout-ms")?)?;
                connect_timeout = Duration::from_millis(milliseconds);
            }
            "--client-fallback" => client_fallback = true,
            "--no-client-fallback" => client_fallback = false,
            "--allow-degraded-start" => allow_degraded_start = true,
            "--fallback-host" => fallback_host = next_value(&mut args, "--fallback-host")?,
            "--fallback-port" => {
                fallback_port = parse_u16(&next_value(&mut args, "--fallback-port")?)?;
            }
            "--fallback-connect-timeout-ms" => {
                fallback_connect_timeout = Duration::from_millis(parse_u64(&next_value(
                    &mut args,
                    "--fallback-connect-timeout-ms",
                )?)?);
            }
            "--fallback-io-timeout-ms" => {
                fallback_io_timeout = Duration::from_millis(parse_u64(&next_value(
                    &mut args,
                    "--fallback-io-timeout-ms",
                )?)?);
            }
            "--max-replay-body-mib" => {
                max_replay_body_bytes =
                    parse_mib(&next_value(&mut args, "--max-replay-body-mib")?)?;
            }
            "--replay-memory-kib" => {
                replay_memory_bytes = parse_kib(&next_value(&mut args, "--replay-memory-kib")?)?;
            }
            "--spool-dir" => {
                spool_dir = PathBuf::from(next_value(&mut args, "--spool-dir")?);
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

    let mut config = ConnectorConfig::new(
        &listen,
        &metrics_listen,
        upstream,
        max_connections,
        connect_timeout,
    )?;
    if client_fallback {
        let mut fallback = ClientFallbackConfig::new(spool_dir)?;
        fallback.host = fallback_host;
        fallback.port = fallback_port;
        fallback.connect_timeout = fallback_connect_timeout;
        fallback.io_timeout = fallback_io_timeout;
        fallback.max_replay_body_bytes = max_replay_body_bytes;
        fallback.replay_memory_bytes = replay_memory_bytes;
        config = config.with_client_fallback(fallback)?;
    }
    if check_only {
        check_upstream(&config).map_err(|error| format!("health check failed: {error}"))?;
        println!("NANDO_REMOTE_OK {}", config.upstream);
        return Ok(());
    }

    if let Err(error) = check_upstream(&config) {
        if config.client_fallback.is_some() && allow_degraded_start {
            eprintln!(
                "nando-connector: upstream startup check failed ({error}); starting in client-fallback mode"
            );
        } else {
            return Err(format!("startup health check failed: {error}"));
        }
    }
    eprintln!(
        "nando-connect: listening on {} -> {} (metrics={}, max_connections={}, client_fallback={})",
        config.listen,
        config.upstream,
        config.metrics_listen,
        config.max_connections,
        config.client_fallback.is_some()
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

fn parse_u16(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|error| format!("invalid port {value}: {error}"))
}

fn parse_mib(value: &str) -> Result<u64, String> {
    parse_u64(value)?
        .checked_mul(1024 * 1024)
        .ok_or_else(|| format!("MiB value is too large: {value}"))
}

fn parse_kib(value: &str) -> Result<usize, String> {
    parse_usize(value)?
        .checked_mul(1024)
        .ok_or_else(|| format!("KiB value is too large: {value}"))
}

fn env_flag(name: &str) -> Result<bool, String> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" | "" => Ok(false),
            _ => Err(format!("{name} must be a boolean value")),
        },
        Err(env::VarError::NotPresent) => Ok(false),
        Err(error) => Err(format!("cannot read {name}: {error}")),
    }
}

fn default_spool_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("nando-connector")
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
  --metrics-listen ADDRESS     Loopback metrics listener (default: {DEFAULT_METRICS_LISTEN})
  --upstream HOST:PORT         Nando server (default: {DEFAULT_UPSTREAM})
  --max-connections COUNT      Concurrent connection limit (default: {DEFAULT_MAX_CONNECTIONS})
  --connect-timeout-ms MS      Upstream connection timeout (default: 2000)
  --client-fallback            Replay Nando abstains from this client
  --no-client-fallback         Disable client-side fallback
  --allow-degraded-start       Start with client fallback if Nando is offline
  --fallback-host HOST         External fallback host (default: {DEFAULT_FALLBACK_HOST})
  --fallback-port PORT         External fallback TLS port (default: {DEFAULT_FALLBACK_PORT})
  --fallback-connect-timeout-ms MS
                               External TCP/TLS connect timeout (default: 15000)
  --fallback-io-timeout-ms MS  External stream timeout (default: 3600000)
  --max-replay-body-mib MIB    Maximum replayable body (default: 64)
  --replay-memory-kib KIB      Memory before tmpfs spill (default: 1024)
  --spool-dir PATH             Private replay spool directory
  --check                      Verify upstream health and exit
  -V, --version                Print version
  -h, --help                   Print help"
    );
}
