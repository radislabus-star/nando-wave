use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const NANDO_GATEWAY_PORT: u16 = 8787;
const HTTPS_PORT: u16 = 443;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ClientRoute {
    Nando,
    OutsideNando,
    Mixed,
    Idle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CodexWindowConnection {
    pub(crate) project: String,
    pub(crate) session: String,
    pub(crate) pids: Vec<u32>,
    pub(crate) configured_for_nando: bool,
    pub(crate) route: ClientRoute,
    pub(crate) remote_endpoints: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct ClientConnectionSnapshot {
    pub(crate) generated_at_unix_ms: u64,
    pub(crate) total_windows: u64,
    pub(crate) configured_for_nando: u64,
    pub(crate) active_nando: u64,
    pub(crate) active_outside_nando: u64,
    pub(crate) active_mixed: u64,
    pub(crate) idle: u64,
    pub(crate) misrouted: u64,
    pub(crate) windows: Vec<CodexWindowConnection>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SocketEndpoint {
    display: String,
    remote_port: u16,
    remote_loopback: bool,
}

#[derive(Default)]
struct WindowAccumulator {
    project: String,
    session: String,
    pids: BTreeSet<u32>,
    configured_for_nando: bool,
    endpoints: BTreeSet<SocketEndpoint>,
}

pub(crate) fn snapshot() -> ClientConnectionSnapshot {
    let socket_table = socket_table();
    let mut grouped = BTreeMap::<String, WindowAccumulator>::new();

    let Ok(processes) = fs::read_dir("/proc") else {
        return empty_snapshot();
    };
    for process in processes.flatten() {
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let root = process.path();
        let Some(arguments) = codex_arguments(&root) else {
            continue;
        };
        let cwd = fs::read_link(root.join("cwd")).unwrap_or_default();
        let project = project_label(&cwd);
        let session = arguments
            .iter()
            .find(|argument| uuid_like(argument))
            .cloned()
            .unwrap_or_else(|| format!("pid-{pid}"));
        let key = format!("{session}\u{1f}{project}");
        let accumulator = grouped.entry(key).or_default();
        accumulator.project = project;
        accumulator.session = session;
        accumulator.pids.insert(pid);
        accumulator.configured_for_nando |= arguments.iter().any(|argument| {
            argument.contains("model_provider") && argument.contains("nando_nginx")
        });
        accumulator
            .endpoints
            .extend(process_socket_endpoints(&root, &socket_table));
    }

    let mut windows = grouped
        .into_values()
        .map(|window| {
            let route = classify_route(&window.endpoints);
            CodexWindowConnection {
                project: window.project,
                session: window.session,
                pids: window.pids.into_iter().collect(),
                configured_for_nando: window.configured_for_nando,
                route,
                remote_endpoints: window
                    .endpoints
                    .into_iter()
                    .map(|endpoint| endpoint.display)
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    windows.sort_by_key(|window| {
        (
            route_order(window.route),
            window.project.clone(),
            window.session.clone(),
        )
    });

    let total_windows = windows.len() as u64;
    let configured_for_nando = windows
        .iter()
        .filter(|window| window.configured_for_nando)
        .count() as u64;
    let active_nando = route_count(&windows, ClientRoute::Nando);
    let active_outside_nando = route_count(&windows, ClientRoute::OutsideNando);
    let active_mixed = route_count(&windows, ClientRoute::Mixed);
    let idle = route_count(&windows, ClientRoute::Idle);
    let misrouted = windows
        .iter()
        .filter(|window| window.configured_for_nando && window.route == ClientRoute::OutsideNando)
        .count() as u64;

    ClientConnectionSnapshot {
        generated_at_unix_ms: unix_now_ms(),
        total_windows,
        configured_for_nando,
        active_nando,
        active_outside_nando,
        active_mixed,
        idle,
        misrouted,
        windows,
    }
}

fn empty_snapshot() -> ClientConnectionSnapshot {
    ClientConnectionSnapshot {
        generated_at_unix_ms: unix_now_ms(),
        total_windows: 0,
        configured_for_nando: 0,
        active_nando: 0,
        active_outside_nando: 0,
        active_mixed: 0,
        idle: 0,
        misrouted: 0,
        windows: Vec::new(),
    }
}

fn codex_arguments(process_root: &Path) -> Option<Vec<String>> {
    let executable = fs::read_link(process_root.join("exe")).ok()?;
    if executable.file_name()?.to_str()? != "codex" {
        return None;
    }
    let bytes = fs::read(process_root.join("cmdline")).ok()?;
    let arguments = bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).into_owned())
        .collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument.contains("codex-code-mode-host"))
    {
        return None;
    }
    Some(arguments)
}

fn project_label(cwd: &Path) -> String {
    if cwd == Path::new("/home/ubu") {
        return "~".to_owned();
    }
    cwd.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| cwd.display().to_string())
}

fn uuid_like(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.char_indices().all(|(index, character)| match index {
        8 | 13 | 18 | 23 => character == '-',
        _ => character.is_ascii_hexdigit(),
    })
}

fn socket_table() -> HashMap<u64, SocketEndpoint> {
    let mut sockets = HashMap::new();
    for (path, ipv6) in [("/proc/net/tcp", false), ("/proc/net/tcp6", true)] {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        for line in contents.lines().skip(1) {
            let Some((inode, endpoint)) = parse_socket_row(line, ipv6) else {
                continue;
            };
            sockets.insert(inode, endpoint);
        }
    }
    sockets
}

fn parse_socket_row(line: &str, ipv6: bool) -> Option<(u64, SocketEndpoint)> {
    let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() < 10 || !matches!(fields[3], "01" | "02") {
        return None;
    }
    let inode = fields[9].parse::<u64>().ok()?;
    let (address_raw, port_raw) = fields[2].rsplit_once(':')?;
    let remote_port = u16::from_str_radix(port_raw, 16).ok()?;
    let (address, remote_loopback) = if ipv6 {
        let address = decode_ipv6(address_raw)?;
        (address.to_string(), address.is_loopback())
    } else {
        let address = decode_ipv4(address_raw)?;
        (address.to_string(), address.is_loopback())
    };
    let display = if ipv6 {
        format!("[{address}]:{remote_port}")
    } else {
        format!("{address}:{remote_port}")
    };
    Some((
        inode,
        SocketEndpoint {
            display,
            remote_port,
            remote_loopback,
        },
    ))
}

fn decode_ipv4(raw: &str) -> Option<Ipv4Addr> {
    if raw.len() != 8 {
        return None;
    }
    let encoded = u32::from_str_radix(raw, 16).ok()?;
    Some(Ipv4Addr::from(encoded.to_le_bytes()))
}

fn decode_ipv6(raw: &str) -> Option<Ipv6Addr> {
    if raw.len() != 32 {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (word_index, destination) in bytes.chunks_exact_mut(4).enumerate() {
        let start = word_index * 8;
        let word = u32::from_str_radix(&raw[start..start + 8], 16).ok()?;
        destination.copy_from_slice(&word.to_le_bytes());
    }
    Some(Ipv6Addr::from(bytes))
}

fn process_socket_endpoints(
    process_root: &Path,
    socket_table: &HashMap<u64, SocketEndpoint>,
) -> BTreeSet<SocketEndpoint> {
    let mut endpoints = BTreeSet::new();
    let Ok(descriptors) = fs::read_dir(process_root.join("fd")) else {
        return endpoints;
    };
    for descriptor in descriptors.flatten() {
        let Ok(target) = fs::read_link(descriptor.path()) else {
            continue;
        };
        let target = target.to_string_lossy();
        let Some(inode) = target
            .strip_prefix("socket:[")
            .and_then(|value| value.strip_suffix(']'))
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        if let Some(endpoint) = socket_table.get(&inode) {
            endpoints.insert(endpoint.clone());
        }
    }
    endpoints
}

fn classify_route(endpoints: &BTreeSet<SocketEndpoint>) -> ClientRoute {
    let nando = endpoints
        .iter()
        .any(|endpoint| endpoint.remote_loopback && endpoint.remote_port == NANDO_GATEWAY_PORT);
    let outside_nando = endpoints
        .iter()
        .any(|endpoint| !endpoint.remote_loopback && endpoint.remote_port == HTTPS_PORT);
    match (nando, outside_nando) {
        (true, true) => ClientRoute::Mixed,
        (true, false) => ClientRoute::Nando,
        (false, true) => ClientRoute::OutsideNando,
        (false, false) => ClientRoute::Idle,
    }
}

fn route_count(windows: &[CodexWindowConnection], route: ClientRoute) -> u64 {
    windows
        .iter()
        .filter(|window| window.route == route)
        .count() as u64
}

const fn route_order(route: ClientRoute) -> u8 {
    match route {
        ClientRoute::OutsideNando => 0,
        ClientRoute::Mixed => 1,
        ClientRoute::Nando => 2,
        ClientRoute::Idle => 3,
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linux_ipv4_socket_rows() {
        let row = "1: 0100007F:CACA 0100007F:2253 01 00000000:00000000 00:00000000 00000000 1000 0 39445500 1";
        let Some((inode, endpoint)) = parse_socket_row(row, false) else {
            panic!("fixture must parse");
        };
        assert_eq!(inode, 39_445_500);
        assert_eq!(endpoint.display, "127.0.0.1:8787");
        assert!(endpoint.remote_loopback);
    }

    #[test]
    fn distinguishes_nando_direct_mixed_and_idle_routes() {
        let nando = SocketEndpoint {
            display: "127.0.0.1:8787".to_owned(),
            remote_port: 8787,
            remote_loopback: true,
        };
        let direct = SocketEndpoint {
            display: "172.64.155.209:443".to_owned(),
            remote_port: 443,
            remote_loopback: false,
        };
        assert_eq!(
            classify_route(&BTreeSet::from([nando.clone()])),
            ClientRoute::Nando
        );
        assert_eq!(
            classify_route(&BTreeSet::from([direct.clone()])),
            ClientRoute::OutsideNando
        );
        assert_eq!(
            classify_route(&BTreeSet::from([nando, direct])),
            ClientRoute::Mixed
        );
        assert_eq!(classify_route(&BTreeSet::new()), ClientRoute::Idle);
    }

    #[test]
    fn accepts_only_complete_session_ids() {
        assert!(uuid_like("019dd9bc-b358-7d01-b32d-7e64d0b9509a"));
        assert!(!uuid_like("019dd9bc"));
        assert!(!uuid_like("not-a-session-id"));
    }
}
