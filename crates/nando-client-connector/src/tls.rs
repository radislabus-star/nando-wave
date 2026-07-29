use crate::ClientFallbackConfig;
use crate::http_fallback::BoxedIo;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::io;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};

static TLS_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

pub(crate) fn connect(config: &ClientFallbackConfig) -> io::Result<BoxedIo> {
    let server_name = ServerName::try_from(config.host.to_owned())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let addresses = resolved_addresses(&config.host, config.port)?;
    let mut last_error = None;

    for address in addresses {
        let tcp = match TcpStream::connect_timeout(&address, config.connect_timeout) {
            Ok(tcp) => tcp,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };
        tcp.set_nodelay(true)?;
        tcp.set_read_timeout(Some(config.io_timeout))?;
        tcp.set_write_timeout(Some(config.io_timeout))?;

        let connection = ClientConnection::new(tls_config(), server_name.clone())
            .map_err(|error| io::Error::other(error.to_string()))?;
        let mut stream = StreamOwned::new(connection, tcp);
        match stream.conn.complete_io(&mut stream.sock) {
            Ok(_) => return Ok(Box::new(stream)),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            format!(
                "fallback host resolved to no reachable addresses: {}:{}",
                config.host, config.port
            ),
        )
    }))
}

fn resolved_addresses(host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
    let mut addresses: Vec<_> = (host, port).to_socket_addrs()?.collect();
    addresses.sort_by_key(SocketAddr::is_ipv6);
    addresses.dedup();
    Ok(addresses)
}

fn tls_config() -> Arc<ClientConfig> {
    Arc::clone(TLS_CONFIG.get_or_init(|| {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_prefers_ipv4_without_discarding_ipv6() -> io::Result<()> {
        let addresses = resolved_addresses("localhost", 443)?;
        assert!(!addresses.is_empty());
        let first_v6 = addresses.iter().position(SocketAddr::is_ipv6);
        if let Some(first_v6) = first_v6 {
            assert!(addresses[..first_v6].iter().all(SocketAddr::is_ipv4));
        }
        Ok(())
    }
}
