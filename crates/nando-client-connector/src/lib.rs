use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_LISTEN: &str = "127.0.0.1:8787";
pub const DEFAULT_METRICS_LISTEN: &str = "127.0.0.1:18786";
pub const DEFAULT_UPSTREAM: &str = "192.168.3.94:8787";
pub const DEFAULT_MAX_CONNECTIONS: usize = 128;
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorConfig {
    pub listen: SocketAddr,
    pub metrics_listen: SocketAddr,
    pub upstream: String,
    pub max_connections: usize,
    pub connect_timeout: Duration,
}

impl ConnectorConfig {
    pub fn new(
        listen: &str,
        metrics_listen: &str,
        upstream: String,
        max_connections: usize,
        connect_timeout: Duration,
    ) -> Result<Self, String> {
        let listen = listen
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid listen address {listen}: {error}"))?;
        if !listen.ip().is_loopback() {
            return Err(format!(
                "connector listen address must be loopback: {listen}"
            ));
        }
        let metrics_listen = metrics_listen
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid metrics listen address {metrics_listen}: {error}"))?;
        if !metrics_listen.ip().is_loopback() {
            return Err(format!(
                "connector metrics address must be loopback: {metrics_listen}"
            ));
        }
        if upstream.trim().is_empty() {
            return Err("upstream address must not be empty".to_owned());
        }
        if max_connections == 0 {
            return Err("max connections must be greater than zero".to_owned());
        }
        if connect_timeout.is_zero() {
            return Err("connect timeout must be greater than zero".to_owned());
        }
        Ok(Self {
            listen,
            metrics_listen,
            upstream,
            max_connections,
            connect_timeout,
        })
    }
}

impl Default for ConnectorConfig {
    fn default() -> Self {
        Self {
            listen: DEFAULT_LISTEN
                .parse()
                .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 8787))),
            metrics_listen: DEFAULT_METRICS_LISTEN
                .parse()
                .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], 18786))),
            upstream: DEFAULT_UPSTREAM.to_owned(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        }
    }
}

pub fn run(config: ConnectorConfig) -> io::Result<()> {
    let listener = TcpListener::bind(config.listen)?;
    let metrics_listener = TcpListener::bind(config.metrics_listen)?;
    serve(listener, metrics_listener, config)
}

pub fn serve(
    listener: TcpListener,
    metrics_listener: TcpListener,
    config: ConnectorConfig,
) -> io::Result<()> {
    let stats = Arc::new(ConnectorStats::new());
    let metrics_stats = Arc::clone(&stats);
    thread::spawn(move || serve_metrics(metrics_listener, metrics_stats));

    for incoming in listener.incoming() {
        let client = match incoming {
            Ok(client) => client,
            Err(error) => {
                stats.accept_failures.fetch_add(1, Ordering::Relaxed);
                eprintln!("nando-connector: accept failed: {error}");
                continue;
            }
        };

        let previous = stats.active_connections.fetch_add(1, Ordering::AcqRel);
        if previous >= config.max_connections {
            stats.active_connections.fetch_sub(1, Ordering::AcqRel);
            stats.rejected_connections.fetch_add(1, Ordering::Relaxed);
            let _ = client.shutdown(Shutdown::Both);
            eprintln!("nando-connector: connection limit reached");
            continue;
        }
        stats.accepted_connections.fetch_add(1, Ordering::Relaxed);

        let connection_stats = Arc::clone(&stats);
        let connection_config = config.clone();
        thread::spawn(move || {
            let _guard = ConnectionGuard(Arc::clone(&connection_stats));
            if let Err(error) = relay_connection(client, &connection_config, &connection_stats) {
                connection_stats
                    .relay_failures
                    .fetch_add(1, Ordering::Relaxed);
                eprintln!("nando-connector: relay failed: {error}");
            }
        });
    }
    Ok(())
}

pub fn check_upstream(config: &ConnectorConfig) -> io::Result<()> {
    let mut upstream = connect_upstream(&config.upstream, config.connect_timeout)?;
    upstream.set_read_timeout(Some(config.connect_timeout))?;
    upstream.set_write_timeout(Some(config.connect_timeout))?;
    upstream
        .write_all(b"GET /health HTTP/1.1\r\nHost: nando-remote\r\nConnection: close\r\n\r\n")?;

    let mut response = Vec::with_capacity(1024);
    upstream.take(16 * 1024).read_to_end(&mut response)?;
    let response = String::from_utf8_lossy(&response);
    let valid = response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200");
    if valid
        && response.contains("\"ok\":true")
        && response.contains("\"service\":\"nando-nginx-gateway\"")
        && response.contains("\"transport\":\"nginx\"")
    {
        return Ok(());
    }
    Err(io::Error::other(
        "upstream returned an unexpected Nando health contract",
    ))
}

fn relay_connection(
    mut client: TcpStream,
    config: &ConnectorConfig,
    stats: &ConnectorStats,
) -> io::Result<()> {
    client.set_nodelay(true)?;
    let mut upstream = connect_upstream(&config.upstream, config.connect_timeout)?;
    upstream.set_nodelay(true)?;

    let mut client_upload = client.try_clone()?;
    let mut upstream_upload = upstream.try_clone()?;
    let upload = thread::spawn(move || {
        let result = io::copy(&mut client_upload, &mut upstream_upload);
        let _ = upstream_upload.shutdown(Shutdown::Write);
        result
    });

    let download_result = io::copy(&mut upstream, &mut client);
    let _ = client.shutdown(Shutdown::Write);
    let upload_result = upload
        .join()
        .map_err(|_| io::Error::other("upload relay thread panicked"))?;

    let download_bytes = download_result?;
    let upload_bytes = upload_result?;
    stats
        .download_bytes
        .fetch_add(download_bytes, Ordering::Relaxed);
    stats
        .upload_bytes
        .fetch_add(upload_bytes, Ordering::Relaxed);
    Ok(())
}

fn connect_upstream(upstream: &str, timeout: Duration) -> io::Result<TcpStream> {
    let mut last_error = None;
    let addresses = upstream.to_socket_addrs()?;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            format!("upstream resolved to no addresses: {upstream}"),
        )
    }))
}

struct ConnectorStats {
    started_at: Instant,
    active_connections: AtomicUsize,
    accepted_connections: AtomicU64,
    completed_connections: AtomicU64,
    rejected_connections: AtomicU64,
    accept_failures: AtomicU64,
    relay_failures: AtomicU64,
    upload_bytes: AtomicU64,
    download_bytes: AtomicU64,
}

impl ConnectorStats {
    fn new() -> Self {
        Self {
            started_at: Instant::now(),
            active_connections: AtomicUsize::new(0),
            accepted_connections: AtomicU64::new(0),
            completed_connections: AtomicU64::new(0),
            rejected_connections: AtomicU64::new(0),
            accept_failures: AtomicU64::new(0),
            relay_failures: AtomicU64::new(0),
            upload_bytes: AtomicU64::new(0),
            download_bytes: AtomicU64::new(0),
        }
    }

    fn json(&self) -> String {
        format!(
            concat!(
                "{{\"ok\":true,\"service\":\"nando-connector\",",
                "\"uptime_seconds\":{},\"active_connections\":{},",
                "\"accepted_connections\":{},\"completed_connections\":{},",
                "\"rejected_connections\":{},\"accept_failures\":{},",
                "\"relay_failures\":{},\"upload_bytes\":{},\"download_bytes\":{}}}"
            ),
            self.started_at.elapsed().as_secs(),
            self.active_connections.load(Ordering::Relaxed),
            self.accepted_connections.load(Ordering::Relaxed),
            self.completed_connections.load(Ordering::Relaxed),
            self.rejected_connections.load(Ordering::Relaxed),
            self.accept_failures.load(Ordering::Relaxed),
            self.relay_failures.load(Ordering::Relaxed),
            self.upload_bytes.load(Ordering::Relaxed),
            self.download_bytes.load(Ordering::Relaxed),
        )
    }
}

fn serve_metrics(listener: TcpListener, stats: Arc<ConnectorStats>) {
    for incoming in listener.incoming() {
        match incoming {
            Ok(mut stream) => {
                if let Err(error) = write_metrics_response(&mut stream, &stats) {
                    eprintln!("nando-connector: metrics response failed: {error}");
                }
            }
            Err(error) => eprintln!("nando-connector: metrics accept failed: {error}"),
        }
    }
}

fn write_metrics_response(stream: &mut TcpStream, stats: &ConnectorStats) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    let mut request = [0_u8; 1024];
    let _ = stream.read(&mut request)?;
    let body = stats.json();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

struct ConnectionGuard(Arc<ConnectorStats>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.active_connections.fetch_sub(1, Ordering::AcqRel);
        self.0.completed_connections.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(listen: SocketAddr, upstream: SocketAddr) -> ConnectorConfig {
        ConnectorConfig {
            listen,
            metrics_listen: "127.0.0.1:0".parse().unwrap_or(listen),
            upstream: upstream.to_string(),
            max_connections: 4,
            connect_timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn rejects_non_loopback_listener() {
        let result = ConnectorConfig::new(
            "0.0.0.0:8787",
            "127.0.0.1:18786",
            "127.0.0.1:9000".to_owned(),
            1,
            Duration::from_secs(1),
        );
        assert!(result.is_err());
    }

    #[test]
    fn relay_preserves_unknown_protocol_bytes() -> io::Result<()> {
        let upstream_listener = TcpListener::bind("127.0.0.1:0")?;
        let upstream_address = upstream_listener.local_addr()?;
        let connector_listener = TcpListener::bind("127.0.0.1:0")?;
        let connector_address = connector_listener.local_addr()?;
        let config = test_config(connector_address, upstream_address);

        let upstream = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = upstream_listener.accept()?;
            let mut request = Vec::new();
            stream.read_to_end(&mut request)?;
            assert_eq!(
                request,
                b"FUTURE-CODEX/99\r\nUnknown-Field: preserved\r\n\r\npayload"
            );
            stream.write_all(b"FUTURE-RESPONSE/99\r\n\r\nstreamed")?;
            stream.shutdown(Shutdown::Write)
        });

        let connector = thread::spawn(move || -> io::Result<()> {
            let (client, _) = connector_listener.accept()?;
            let stats = ConnectorStats::new();
            relay_connection(client, &config, &stats)?;
            assert_eq!(
                stats.upload_bytes.load(Ordering::Relaxed),
                b"FUTURE-CODEX/99\r\nUnknown-Field: preserved\r\n\r\npayload".len() as u64
            );
            assert_eq!(
                stats.download_bytes.load(Ordering::Relaxed),
                b"FUTURE-RESPONSE/99\r\n\r\nstreamed".len() as u64
            );
            Ok(())
        });

        let mut client = TcpStream::connect(connector_address)?;
        client.write_all(b"FUTURE-CODEX/99\r\nUnknown-Field: preserved\r\n\r\npayload")?;
        client.shutdown(Shutdown::Write)?;
        let mut response = Vec::new();
        client.read_to_end(&mut response)?;
        assert_eq!(response, b"FUTURE-RESPONSE/99\r\n\r\nstreamed");

        connector
            .join()
            .map_err(|_| io::Error::other("connector test thread panicked"))??;
        upstream
            .join()
            .map_err(|_| io::Error::other("upstream test thread panicked"))??;
        Ok(())
    }

    #[test]
    fn metrics_snapshot_reports_transport_counters() {
        let stats = ConnectorStats::new();
        stats.active_connections.store(2, Ordering::Relaxed);
        stats.accepted_connections.store(7, Ordering::Relaxed);
        stats.upload_bytes.store(123, Ordering::Relaxed);
        stats.download_bytes.store(456, Ordering::Relaxed);

        let json = stats.json();
        assert!(json.contains("\"active_connections\":2"));
        assert!(json.contains("\"accepted_connections\":7"));
        assert!(json.contains("\"upload_bytes\":123"));
        assert!(json.contains("\"download_bytes\":456"));
    }

    #[test]
    fn health_check_requires_expected_contract() -> io::Result<()> {
        let upstream_listener = TcpListener::bind("127.0.0.1:0")?;
        let upstream_address = upstream_listener.local_addr()?;
        let config = test_config(
            "127.0.0.1:0".parse().map_err(io::Error::other)?,
            upstream_address,
        );

        let upstream = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = upstream_listener.accept()?;
            let mut request = [0_u8; 256];
            let _ = stream.read(&mut request)?;
            let body = r#"{"ok":true,"service":"nando-nginx-gateway","transport":"nginx"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
        });

        check_upstream(&config)?;
        upstream
            .join()
            .map_err(|_| io::Error::other("health test thread panicked"))??;
        Ok(())
    }
}
