use crate::{
    ClientFallbackConfig, ConnectorConfig, ConnectorStats, connect_upstream,
    relay_raw_connection_with_prefix,
};
use nando_client_evidence::{ClientRouteIdentityV1, NandoRouteReceiptLedger};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
use std::net::{Shutdown, TcpStream};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_HTTP_HEAD_BYTES: usize = 64 * 1024;
const MAX_CHUNK_LINE_BYTES: usize = 8 * 1024;
static SPOOL_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) trait ReadWrite: Read + Write {}

impl<T: Read + Write> ReadWrite for T {}

pub(crate) type BoxedIo = Box<dyn ReadWrite + Send>;

pub(crate) fn relay_connection(
    client: TcpStream,
    config: &ConnectorConfig,
    fallback: &ClientFallbackConfig,
    stats: &ConnectorStats,
    route_receipts: Option<&Arc<Mutex<NandoRouteReceiptLedger>>>,
) -> io::Result<()> {
    relay_connection_with_dialer(
        client,
        config,
        fallback,
        stats,
        route_receipts,
        crate::tls::connect,
    )
}

fn relay_connection_with_dialer<F>(
    mut client: TcpStream,
    config: &ConnectorConfig,
    fallback: &ClientFallbackConfig,
    stats: &ConnectorStats,
    route_receipts: Option<&Arc<Mutex<NandoRouteReceiptLedger>>>,
    fallback_dialer: F,
) -> io::Result<()>
where
    F: Fn(&ClientFallbackConfig) -> io::Result<BoxedIo>,
{
    client.set_nodelay(true)?;
    let captured = capture_request_head(&mut client)?;
    let (request_bytes, head_end) = match captured {
        CapturedRequest::Raw(prefix) => {
            return relay_raw_connection_with_prefix(&mut client, config, stats, &prefix);
        }
        CapturedRequest::Http { bytes, head_end } => (bytes, head_end),
    };

    let request = match parse_request_head(&request_bytes[..head_end]) {
        Ok(request) if is_nando_api_target(&request.target) => request,
        Ok(_) | Err(_) => {
            return relay_raw_connection_with_prefix(&mut client, config, stats, &request_bytes);
        }
    };

    let mut body = ReplayBody::new(
        fallback.replay_memory_bytes,
        fallback.max_replay_body_bytes,
        &fallback.spool_dir,
    );
    collect_request_body(
        &mut client,
        &request_bytes[head_end..],
        request.framing,
        &mut body,
    )?;
    if body.spilled() {
        stats.replay_spills.fetch_add(1, Ordering::Relaxed);
    }
    let route_identity = route_identity_for_request(&request, &mut body);
    let request_body_sha256 = body.sha256();
    let request_observed_at_unix_nanos = unix_now_nanos();

    stats.http_requests.fetch_add(1, Ordering::Relaxed);
    stats
        .upload_bytes
        .fetch_add(head_end as u64 + body.len(), Ordering::Relaxed);

    let remote_target = format!("{}{}", fallback.remote_local_prefix, request.target);
    let remote_response = attempt_remote_local(config, &request, &remote_target, &mut body);
    if let Ok(response) = &remote_response
        && matches!(response.status, 200 | 418)
        && is_response_target(&request.target)
    {
        match (&route_identity, route_receipts) {
            (Some(identity), Some(route_receipts)) => {
                let result = route_receipts
                    .lock()
                    .map_err(|_| "route_receipt_lock_poisoned".to_owned())
                    .and_then(|mut ledger| {
                        ledger
                            .append(
                                identity,
                                request_body_sha256.clone(),
                                response.status,
                                request_observed_at_unix_nanos,
                                unix_now_nanos(),
                            )
                            .map(|_| ())
                    });
                if let Err(error) = result {
                    stats.route_receipt_failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("nando-connector: route receipt censored ({error})");
                } else {
                    stats.route_receipts.fetch_add(1, Ordering::Relaxed);
                }
            }
            (None, Some(_)) => {
                stats.route_identity_missing.fetch_add(1, Ordering::Relaxed);
            }
            (_, None) => {}
        }
    }

    match remote_response {
        Ok(response) if !requires_client_fallback(response.status) => {
            stats.nando_responses.fetch_add(1, Ordering::Relaxed);
            return relay_response(response.prefix, response.stream, &mut client, stats);
        }
        Ok(response) => {
            if response.status == 418 {
                stats.abstain_fallbacks.fetch_add(1, Ordering::Relaxed);
            } else {
                stats
                    .remote_failure_fallbacks
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Err(error) => {
            stats
                .remote_failure_fallbacks
                .fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "nando-connector: LAN-only Nando route unavailable ({error}); using client fallback"
            );
        }
    }

    stats
        .client_fallback_attempts
        .fetch_add(1, Ordering::Relaxed);
    let fallback_result = attempt_client_fallback(fallback, &request, &mut body, &fallback_dialer);
    match fallback_result {
        Ok(response) => {
            stats
                .client_fallback_successes
                .fetch_add(1, Ordering::Relaxed);
            stats
                .replayed_request_bytes
                .fetch_add(response.request_bytes, Ordering::Relaxed);
            relay_response(response.prefix, response.stream, &mut client, stats)
        }
        Err(error) => {
            stats
                .client_fallback_failures
                .fetch_add(1, Ordering::Relaxed);
            eprintln!("nando-connector: client fallback failed: {error}");
            write_fallback_unavailable(&mut client, stats)
        }
    }
}

fn attempt_remote_local(
    config: &ConnectorConfig,
    request: &RequestHead,
    target: &str,
    body: &mut ReplayBody,
) -> io::Result<PreparedResponse<TcpStream>> {
    let mut stream = connect_upstream(&config.upstream, config.connect_timeout)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(3600)))?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(3600)))?;
    write_request(&mut stream, request, target, "nando-remote", true, body)?;
    let prefix = read_response_head(&mut stream)?;
    let status = parse_response_status(&prefix)?;
    Ok(PreparedResponse {
        status,
        prefix,
        stream,
    })
}

fn attempt_client_fallback<F>(
    fallback: &ClientFallbackConfig,
    request: &RequestHead,
    body: &mut ReplayBody,
    fallback_dialer: &F,
) -> io::Result<FallbackResponse>
where
    F: Fn(&ClientFallbackConfig) -> io::Result<BoxedIo>,
{
    let target = codex_backend_target(&request.target)?;
    let mut stream = fallback_dialer(fallback)?;
    let request_bytes = write_request(&mut stream, request, &target, &fallback.host, false, body)?;
    let prefix = read_response_head(&mut stream)?;
    let status = parse_response_status(&prefix)?;
    Ok(FallbackResponse {
        request_bytes,
        prefix,
        stream,
        status,
    })
}

fn requires_client_fallback(status: u16) -> bool {
    matches!(status, 418 | 502 | 503 | 504)
}

fn relay_response<R: Read>(
    prefix: Vec<u8>,
    mut upstream: R,
    client: &mut TcpStream,
    stats: &ConnectorStats,
) -> io::Result<()> {
    client.write_all(&prefix)?;
    let copied = io::copy(&mut upstream, client)?;
    let _ = client.shutdown(Shutdown::Write);
    stats
        .download_bytes
        .fetch_add(prefix.len() as u64 + copied, Ordering::Relaxed);
    Ok(())
}

fn write_fallback_unavailable(client: &mut TcpStream, stats: &ConnectorStats) -> io::Result<()> {
    let body = br#"{"error":{"message":"Nando and client fallback are unavailable","type":"nando_transport_unavailable"}}"#;
    let head = format!(
        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    client.write_all(head.as_bytes())?;
    client.write_all(body)?;
    let _ = client.shutdown(Shutdown::Write);
    stats
        .download_bytes
        .fetch_add((head.len() + body.len()) as u64, Ordering::Relaxed);
    Ok(())
}

fn write_request<W: Write + ?Sized>(
    stream: &mut W,
    request: &RequestHead,
    target: &str,
    host: &str,
    mark_remote_local: bool,
    body: &mut ReplayBody,
) -> io::Result<u64> {
    let mut head = Vec::with_capacity(4096);
    write!(
        head,
        "{} {} HTTP/1.1\r\nHost: {}\r\n",
        request.method, target, host
    )?;
    for header in &request.headers {
        if should_drop_request_header(&header.name) {
            continue;
        }
        head.extend_from_slice(header.name.as_bytes());
        head.extend_from_slice(b": ");
        head.extend_from_slice(&header.value);
        head.extend_from_slice(b"\r\n");
    }
    match request.framing {
        BodyFraming::None => {}
        BodyFraming::ContentLength(length) => {
            write!(head, "Content-Length: {length}\r\n")?;
        }
        BodyFraming::Chunked => {
            head.extend_from_slice(b"Transfer-Encoding: chunked\r\n");
        }
    }
    if mark_remote_local {
        head.extend_from_slice(b"X-Nando-Client-Fallback: 1\r\n");
    }
    head.extend_from_slice(b"Connection: close\r\n\r\n");

    stream.write_all(&head)?;
    body.write_to(stream)?;
    stream.flush()?;
    Ok(head.len() as u64 + body.len())
}

fn should_drop_request_header(name: &str) -> bool {
    name.eq_ignore_ascii_case("host")
        || name.eq_ignore_ascii_case("connection")
        || name.eq_ignore_ascii_case("proxy-connection")
        || name.eq_ignore_ascii_case("keep-alive")
        || name.eq_ignore_ascii_case("upgrade")
        || name.eq_ignore_ascii_case("expect")
        || name.eq_ignore_ascii_case("content-length")
        || name.eq_ignore_ascii_case("transfer-encoding")
        || name.eq_ignore_ascii_case("x-nando-client-fallback")
}

fn codex_backend_target(target: &str) -> io::Result<String> {
    for prefix in ["/v1/", "/v2/"] {
        if let Some(rest) = target.strip_prefix(prefix) {
            return Ok(format!("/backend-api/codex/{rest}"));
        }
    }
    if matches!(target, "/v1" | "/v2") {
        return Ok("/backend-api/codex".to_owned());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "request target is not a supported Nando API path",
    ))
}

fn is_nando_api_target(target: &str) -> bool {
    matches!(target, "/v1" | "/v2") || target.starts_with("/v1/") || target.starts_with("/v2/")
}

fn is_response_target(target: &str) -> bool {
    matches!(target, "/v1/responses" | "/v2/responses")
}

fn route_identity_for_request(
    request: &RequestHead,
    body: &mut ReplayBody,
) -> Option<ClientRouteIdentityV1> {
    if request.method != "POST"
        || !is_response_target(&request.target)
        || !matches!(request.framing, BodyFraming::ContentLength(_))
    {
        return None;
    }
    body.parse_json()
        .ok()
        .and_then(|payload| ClientRouteIdentityV1::from_payload(&payload))
}

enum CapturedRequest {
    Http { bytes: Vec<u8>, head_end: usize },
    Raw(Vec<u8>),
}

fn capture_request_head(stream: &mut TcpStream) -> io::Result<CapturedRequest> {
    let mut bytes = Vec::with_capacity(8192);
    let mut chunk = [0_u8; 8192];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(CapturedRequest::Raw(bytes));
        }
        bytes.extend_from_slice(&chunk[..read]);

        if !could_be_http_request(&bytes) {
            return Ok(CapturedRequest::Raw(bytes));
        }
        if let Some(head_end) = find_head_end(&bytes) {
            return Ok(CapturedRequest::Http { bytes, head_end });
        }
        if bytes.len() > MAX_HTTP_HEAD_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "HTTP request head exceeds connector limit",
            ));
        }
    }
}

fn could_be_http_request(bytes: &[u8]) -> bool {
    let inspected = &bytes[..bytes.len().min(32)];
    if let Some(space) = inspected.iter().position(|byte| *byte == b' ') {
        return space > 0 && inspected[..space].iter().all(u8::is_ascii_uppercase);
    }
    inspected.len() <= 16 && inspected.iter().all(u8::is_ascii_uppercase)
}

fn find_head_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
}

#[derive(Clone, Copy)]
enum BodyFraming {
    None,
    ContentLength(u64),
    Chunked,
}

struct RequestHeader {
    name: String,
    value: Vec<u8>,
}

struct RequestHead {
    method: String,
    target: String,
    headers: Vec<RequestHeader>,
    framing: BodyFraming,
}

fn parse_request_head(bytes: &[u8]) -> io::Result<RequestHead> {
    let mut parsed_headers = [httparse::EMPTY_HEADER; 128];
    let mut parsed = httparse::Request::new(&mut parsed_headers);
    match parsed.parse(bytes).map_err(invalid_http)? {
        httparse::Status::Complete(_) => {}
        httparse::Status::Partial => {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete HTTP request head",
            ));
        }
    }
    let method = parsed
        .method
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP method"))?;
    let target = parsed
        .path
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing HTTP target"))?;
    if parsed.version != Some(1) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "client fallback requires HTTP/1.1",
        ));
    }

    let mut headers = Vec::with_capacity(parsed.headers.len());
    let mut content_length = None;
    let mut chunked = false;
    for header in parsed.headers {
        if header.name.eq_ignore_ascii_case("content-length") {
            let value = std::str::from_utf8(header.value)
                .map_err(invalid_http)?
                .trim()
                .parse::<u64>()
                .map_err(invalid_http)?;
            if content_length
                .replace(value)
                .is_some_and(|old| old != value)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "conflicting Content-Length headers",
                ));
            }
        }
        if header.name.eq_ignore_ascii_case("transfer-encoding") {
            let value = std::str::from_utf8(header.value).map_err(invalid_http)?;
            chunked = value
                .split(',')
                .next_back()
                .is_some_and(|coding| coding.trim().eq_ignore_ascii_case("chunked"));
            if !chunked {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unsupported request Transfer-Encoding",
                ));
            }
        }
        headers.push(RequestHeader {
            name: header.name.to_owned(),
            value: header.value.to_vec(),
        });
    }
    if chunked && content_length.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request has both Content-Length and Transfer-Encoding",
        ));
    }
    let framing = if chunked {
        BodyFraming::Chunked
    } else if let Some(length) = content_length {
        BodyFraming::ContentLength(length)
    } else {
        BodyFraming::None
    };
    Ok(RequestHead {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        framing,
    })
}

fn invalid_http(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

fn collect_request_body(
    stream: &mut TcpStream,
    initial: &[u8],
    framing: BodyFraming,
    body: &mut ReplayBody,
) -> io::Result<()> {
    let mut reader = Cursor::new(initial).chain(stream);
    match framing {
        BodyFraming::None => Ok(()),
        BodyFraming::ContentLength(length) => copy_exact(&mut reader, length, body),
        BodyFraming::Chunked => copy_chunked(&mut reader, body),
    }
}

fn copy_exact<R: Read>(
    reader: &mut R,
    mut remaining: u64,
    body: &mut ReplayBody,
) -> io::Result<()> {
    let mut buffer = [0_u8; 32 * 1024];
    while remaining > 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|error| io::Error::other(error.to_string()))?;
        let read = reader.read(&mut buffer[..wanted])?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "request body ended before Content-Length",
            ));
        }
        body.append(&buffer[..read])?;
        remaining -= read as u64;
    }
    Ok(())
}

fn copy_chunked<R: Read>(reader: &mut R, body: &mut ReplayBody) -> io::Result<()> {
    loop {
        let line = read_crlf_line(reader)?;
        body.append(&line)?;
        let size_text = line
            .strip_suffix(b"\r\n")
            .ok_or_else(|| invalid_http("chunk size line lacks CRLF"))?;
        let size_text = size_text
            .split(|byte| *byte == b';')
            .next()
            .ok_or_else(|| invalid_http("missing chunk size"))?;
        let size = u64::from_str_radix(
            std::str::from_utf8(size_text).map_err(invalid_http)?.trim(),
            16,
        )
        .map_err(invalid_http)?;

        if size == 0 {
            loop {
                let trailer = read_crlf_line(reader)?;
                let done = trailer == b"\r\n";
                body.append(&trailer)?;
                if done {
                    return Ok(());
                }
            }
        }

        copy_exact(reader, size, body)?;
        let mut ending = [0_u8; 2];
        reader.read_exact(&mut ending)?;
        if ending != *b"\r\n" {
            return Err(invalid_http("chunk data lacks CRLF"));
        }
        body.append(&ending)?;
    }
}

fn read_crlf_line<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut line = Vec::with_capacity(32);
    while line.len() < MAX_CHUNK_LINE_BYTES {
        let mut byte = [0_u8; 1];
        reader.read_exact(&mut byte)?;
        line.push(byte[0]);
        if line.ends_with(b"\r\n") {
            return Ok(line);
        }
    }
    Err(invalid_http("chunk line exceeds connector limit"))
}

enum ReplayStorage {
    Memory(Vec<u8>),
    File(File),
}

struct ReplayBody {
    storage: ReplayStorage,
    len: u64,
    memory_limit: usize,
    max_len: u64,
    spool_dir: std::path::PathBuf,
    spilled: bool,
    body_hasher: Sha256,
}

impl ReplayBody {
    fn new(memory_limit: usize, max_len: u64, spool_dir: &Path) -> Self {
        Self {
            storage: ReplayStorage::Memory(Vec::with_capacity(memory_limit.min(64 * 1024))),
            len: 0,
            memory_limit,
            max_len,
            spool_dir: spool_dir.to_path_buf(),
            spilled: false,
            body_hasher: Sha256::new(),
        }
    }

    fn append(&mut self, bytes: &[u8]) -> io::Result<()> {
        let next_len = self
            .len
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| io::Error::other("request body length overflow"))?;
        if next_len > self.max_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request body exceeds replay limit",
            ));
        }
        if matches!(self.storage, ReplayStorage::Memory(_))
            && usize::try_from(next_len).map_or(true, |len| len > self.memory_limit)
        {
            self.spill_to_file()?;
        }
        match &mut self.storage {
            ReplayStorage::Memory(memory) => memory.extend_from_slice(bytes),
            ReplayStorage::File(file) => file.write_all(bytes)?,
        }
        self.body_hasher.update(bytes);
        self.len = next_len;
        Ok(())
    }

    fn spill_to_file(&mut self) -> io::Result<()> {
        std::fs::create_dir_all(&self.spool_dir)?;
        std::fs::set_permissions(&self.spool_dir, std::fs::Permissions::from_mode(0o700))?;
        let sequence = SPOOL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = self
            .spool_dir
            .join(format!(".replay-{}-{sequence}", std::process::id()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        let _ = std::fs::remove_file(&path);
        if let ReplayStorage::Memory(memory) = &self.storage {
            file.write_all(memory)?;
        }
        self.storage = ReplayStorage::File(file);
        self.spilled = true;
        Ok(())
    }

    fn write_to<W: Write + ?Sized>(&mut self, writer: &mut W) -> io::Result<()> {
        match &mut self.storage {
            ReplayStorage::Memory(memory) => writer.write_all(memory),
            ReplayStorage::File(file) => {
                file.seek(SeekFrom::Start(0))?;
                io::copy(file, writer)?;
                Ok(())
            }
        }
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn spilled(&self) -> bool {
        self.spilled
    }

    fn sha256(&self) -> String {
        format!("{:x}", self.body_hasher.clone().finalize())
    }

    fn parse_json(&mut self) -> Result<serde_json::Value, serde_json::Error> {
        match &mut self.storage {
            ReplayStorage::Memory(memory) => serde_json::from_slice(memory),
            ReplayStorage::File(file) => {
                file.seek(SeekFrom::Start(0))
                    .map_err(serde_json::Error::io)?;
                serde_json::from_reader(file)
            }
        }
    }
}

fn unix_now_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
        })
}

fn read_response_head<R: Read + ?Sized>(stream: &mut R) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 8192];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "upstream closed before HTTP response head",
            ));
        }
        bytes.extend_from_slice(&chunk[..read]);
        if find_head_end(&bytes).is_some() {
            return Ok(bytes);
        }
        if bytes.len() > MAX_HTTP_HEAD_BYTES {
            return Err(invalid_http("HTTP response head exceeds connector limit"));
        }
    }
}

fn parse_response_status(bytes: &[u8]) -> io::Result<u16> {
    let mut headers = [httparse::EMPTY_HEADER; 128];
    let mut response = httparse::Response::new(&mut headers);
    match response.parse(bytes).map_err(invalid_http)? {
        httparse::Status::Complete(_) => response
            .code
            .ok_or_else(|| invalid_http("HTTP response lacks status code")),
        httparse::Status::Partial => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "incomplete HTTP response head",
        )),
    }
}

struct PreparedResponse<S> {
    status: u16,
    prefix: Vec<u8>,
    stream: S,
}

struct FallbackResponse {
    request_bytes: u64,
    prefix: Vec<u8>,
    stream: BoxedIo,
    #[allow(dead_code)]
    status: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_client_evidence::{
        DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES, NandoRouteReceiptIndex,
        evidence_client_intent_id_sha256, evidence_session_id_sha256,
    };
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::thread;
    use std::time::Duration;

    fn test_connector_config(upstream: std::net::SocketAddr) -> ConnectorConfig {
        ConnectorConfig {
            listen: "127.0.0.1:0"
                .parse()
                .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 0))),
            metrics_listen: "127.0.0.1:0"
                .parse()
                .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 0))),
            upstream: upstream.to_string(),
            max_connections: 4,
            connect_timeout: Duration::from_secs(2),
            client_fallback: None,
            route_receipts_path: None,
        }
    }

    fn test_fallback_config(name: &str) -> io::Result<ClientFallbackConfig> {
        let mut config = ClientFallbackConfig::new(std::env::temp_dir().join(format!(
            "nando-connector-test-{name}-{}",
            std::process::id()
        )))
        .map_err(io::Error::other)?;
        config.replay_memory_bytes = 4;
        config.max_replay_body_bytes = 1024 * 1024;
        Ok(config)
    }

    fn read_http_request(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
        let head_and_body = read_response_head(stream)?;
        let head_end = find_head_end(&head_and_body)
            .ok_or_else(|| invalid_http("test request lacks head terminator"))?;
        let parsed = parse_request_head(&head_and_body[..head_end])?;
        let mut complete = head_and_body[..head_end].to_vec();
        let initial = &head_and_body[head_end..];
        match parsed.framing {
            BodyFraming::None => {}
            BodyFraming::ContentLength(length) => {
                complete.extend_from_slice(initial);
                let already = initial.len() as u64;
                if already < length {
                    let mut rest =
                        vec![0_u8; usize::try_from(length - already).map_err(io::Error::other)?];
                    stream.read_exact(&mut rest)?;
                    complete.extend_from_slice(&rest);
                }
            }
            BodyFraming::Chunked => {
                return Err(invalid_http("test helper does not accept chunked requests"));
            }
        }
        Ok(complete)
    }

    #[test]
    fn abstain_replays_opaque_body_through_client_fallback() -> io::Result<()> {
        let remote_listener = TcpListener::bind("127.0.0.1:0")?;
        let remote_address = remote_listener.local_addr()?;
        let fallback_listener = TcpListener::bind("127.0.0.1:0")?;
        let fallback_address = fallback_listener.local_addr()?;
        let connector_listener = TcpListener::bind("127.0.0.1:0")?;
        let connector_address = connector_listener.local_addr()?;
        let config = test_connector_config(remote_address);
        let fallback = test_fallback_config("abstain")?;

        let remote = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = remote_listener.accept()?;
            let request = read_http_request(&mut stream)?;
            let request_text = String::from_utf8_lossy(&request);
            assert!(request_text.starts_with("POST /_nando/local/v1/responses HTTP/1.1\r\n"));
            assert!(request_text.contains("\r\nX-Nando-Client-Fallback: 1\r\n"));
            assert!(request.ends_with(b"opaque-body"));
            stream.write_all(
                b"HTTP/1.1 418 I'm a teapot\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
        });

        let fallback_server = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = fallback_listener.accept()?;
            let request = read_http_request(&mut stream)?;
            let request_text = String::from_utf8_lossy(&request);
            assert!(request_text.starts_with(
                "POST /backend-api/codex/responses HTTP/1.1\r\nHost: chatgpt.com\r\n"
            ));
            assert!(!request_text.contains("X-Nando-Client-Fallback"));
            assert!(request.ends_with(b"opaque-body"));
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 9\r\nConnection: close\r\n\r\ndata: ok\n",
            )
        });

        let connector = thread::spawn(move || -> io::Result<ConnectorStats> {
            let (client, _) = connector_listener.accept()?;
            let stats = ConnectorStats::new();
            relay_connection_with_dialer(client, &config, &fallback, &stats, None, |_| {
                Ok(Box::new(TcpStream::connect(fallback_address)?))
            })?;
            Ok(stats)
        });

        let mut client = TcpStream::connect(connector_address)?;
        client.write_all(
            b"POST /v1/responses HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer hidden\r\nContent-Length: 11\r\n\r\nopaque-body",
        )?;
        let mut response = Vec::new();
        client.read_to_end(&mut response)?;
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(b"data: ok\n"));

        let stats = connector
            .join()
            .map_err(|_| io::Error::other("connector test thread panicked"))??;
        assert_eq!(stats.client_fallback_successes.load(Ordering::Relaxed), 1);
        assert_eq!(stats.abstain_fallbacks.load(Ordering::Relaxed), 1);
        assert_eq!(stats.replay_spills.load(Ordering::Relaxed), 1);
        remote
            .join()
            .map_err(|_| io::Error::other("remote test thread panicked"))??;
        fallback_server
            .join()
            .map_err(|_| io::Error::other("fallback test thread panicked"))??;
        Ok(())
    }

    #[test]
    fn local_success_never_dials_external_fallback() -> io::Result<()> {
        let remote_listener = TcpListener::bind("127.0.0.1:0")?;
        let remote_address = remote_listener.local_addr()?;
        let connector_listener = TcpListener::bind("127.0.0.1:0")?;
        let connector_address = connector_listener.local_addr()?;
        let config = test_connector_config(remote_address);
        let fallback = test_fallback_config("local-success")?;
        let dial_count = Arc::new(AtomicUsize::new(0));

        let remote = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = remote_listener.accept()?;
            let _ = read_http_request(&mut stream)?;
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nlocal",
            )
        });

        let connector_dial_count = Arc::clone(&dial_count);
        let connector = thread::spawn(move || -> io::Result<()> {
            let (client, _) = connector_listener.accept()?;
            let stats = ConnectorStats::new();
            relay_connection_with_dialer(client, &config, &fallback, &stats, None, |_| {
                connector_dial_count.fetch_add(1, AtomicOrdering::Relaxed);
                Err(io::Error::other("external fallback must not be called"))
            })?;
            assert_eq!(stats.nando_responses.load(Ordering::Relaxed), 1);
            Ok(())
        });

        let mut client = TcpStream::connect(connector_address)?;
        client.write_all(b"GET /v1/models HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
        let mut response = Vec::new();
        client.read_to_end(&mut response)?;
        assert!(response.ends_with(b"local"));
        assert_eq!(dial_count.load(AtomicOrdering::Relaxed), 0);
        connector
            .join()
            .map_err(|_| io::Error::other("connector test thread panicked"))??;
        remote
            .join()
            .map_err(|_| io::Error::other("remote test thread panicked"))??;
        Ok(())
    }

    #[test]
    fn confirmed_nando_response_writes_payload_free_route_receipt() -> io::Result<()> {
        let remote_listener = TcpListener::bind("127.0.0.1:0")?;
        let remote_address = remote_listener.local_addr()?;
        let connector_listener = TcpListener::bind("127.0.0.1:0")?;
        let connector_address = connector_listener.local_addr()?;
        let config = test_connector_config(remote_address);
        let fallback = test_fallback_config("route-receipt")?;
        let route_path = std::env::temp_dir().join(format!(
            "nando-connector-route-receipt-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&route_path);
        let route_receipts = Arc::new(Mutex::new(
            NandoRouteReceiptLedger::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .map_err(io::Error::other)?,
        ));

        let remote = thread::spawn(move || -> io::Result<()> {
            let (mut stream, _) = remote_listener.accept()?;
            let _ = read_http_request(&mut stream)?;
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nlocal",
            )
        });

        let connector_route_receipts = Arc::clone(&route_receipts);
        let connector = thread::spawn(move || -> io::Result<ConnectorStats> {
            let (client, _) = connector_listener.accept()?;
            let stats = ConnectorStats::new();
            relay_connection_with_dialer(
                client,
                &config,
                &fallback,
                &stats,
                Some(&connector_route_receipts),
                |_| Err(io::Error::other("external fallback must not be called")),
            )?;
            Ok(stats)
        });

        let body =
            br#"{"client_metadata":{"session_id":"session-a","turn_id":"turn-a"},"input":[]}"#;
        let mut client = TcpStream::connect(connector_address)?;
        write!(
            client,
            "POST /v1/responses HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )?;
        client.write_all(body)?;
        let mut response = Vec::new();
        client.read_to_end(&mut response)?;
        assert!(response.ends_with(b"local"));

        let stats = connector
            .join()
            .map_err(|_| io::Error::other("connector test thread panicked"))??;
        assert_eq!(stats.route_receipts.load(Ordering::Relaxed), 1);
        assert_eq!(stats.route_identity_missing.load(Ordering::Relaxed), 0);
        drop(route_receipts);
        let index =
            NandoRouteReceiptIndex::open(&route_path, DEFAULT_ROUTE_RECEIPT_LEDGER_MAX_BYTES)
                .map_err(io::Error::other)?;
        assert!(
            index
                .receipt_for_frame(
                    &evidence_client_intent_id_sha256("turn-a"),
                    &evidence_session_id_sha256("session-a"),
                    u64::MAX,
                )
                .is_some()
        );
        remote
            .join()
            .map_err(|_| io::Error::other("remote test thread panicked"))??;
        std::fs::remove_file(route_path)?;
        Ok(())
    }

    #[test]
    fn chunked_body_is_replayable_without_payload_parsing() -> io::Result<()> {
        let mut body = ReplayBody::new(1024, 4096, &std::env::temp_dir());
        let encoded = b"4\r\ntest\r\n3\r\n123\r\n0\r\nX-Proof: yes\r\n\r\n";
        copy_chunked(&mut Cursor::new(encoded), &mut body)?;
        let mut replay = Vec::new();
        body.write_to(&mut replay)?;
        assert_eq!(replay, encoded);
        Ok(())
    }
}
