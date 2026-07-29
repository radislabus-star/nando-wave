use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

pub const DEFAULT_LISTEN: &str = "127.0.0.1:8787";
pub const DEFAULT_UPSTREAM: &str = "192.168.3.94:8787";
pub const DEFAULT_MAX_CONNECTIONS: usize = 128;
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectorConfig {
    pub listen: SocketAddr,
    pub upstream: String,
    pub max_connections: usize,
    pub connect_timeout: Duration,
}

impl ConnectorConfig {
    pub fn new(
        listen: &str,
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
            upstream: DEFAULT_UPSTREAM.to_owned(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
        }
    }
}

pub fn run(config: ConnectorConfig) -> io::Result<()> {
    let listener = TcpListener::bind(config.listen)?;
    serve(listener, config)
}

pub fn serve(listener: TcpListener, config: ConnectorConfig) -> io::Result<()> {
    let active_connections = Arc::new(AtomicUsize::new(0));

    for incoming in listener.incoming() {
        let client = match incoming {
            Ok(client) => client,
            Err(error) => {
                eprintln!("nando-connect: accept failed: {error}");
                continue;
            }
        };

        let previous = active_connections.fetch_add(1, Ordering::AcqRel);
        if previous >= config.max_connections {
            active_connections.fetch_sub(1, Ordering::AcqRel);
            let _ = client.shutdown(Shutdown::Both);
            eprintln!("nando-connect: connection limit reached");
            continue;
        }

        let connection_count = Arc::clone(&active_connections);
        let connection_config = config.clone();
        thread::spawn(move || {
            let _guard = ConnectionGuard(connection_count);
            if let Err(error) = relay_connection(client, &connection_config) {
                eprintln!("nando-connect: relay failed: {error}");
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

fn relay_connection(mut client: TcpStream, config: &ConnectorConfig) -> io::Result<()> {
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

    download_result?;
    upload_result?;
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

struct ConnectionGuard(Arc<AtomicUsize>);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(listen: SocketAddr, upstream: SocketAddr) -> ConnectorConfig {
        ConnectorConfig {
            listen,
            upstream: upstream.to_string(),
            max_connections: 4,
            connect_timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn rejects_non_loopback_listener() {
        let result = ConnectorConfig::new(
            "0.0.0.0:8787",
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
            relay_connection(client, &config)
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
