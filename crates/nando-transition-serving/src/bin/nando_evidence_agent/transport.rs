#[derive(Clone, Debug)]
struct HttpEndpoint {
    socket_host: String,
    port: u16,
    host_header: String,
    path: String,
}

impl HttpEndpoint {
    fn parse(origin: &str) -> Result<Self, String> {
        let authority = origin
            .strip_prefix("http://")
            .ok_or_else(|| "evidence_agent_server_requires_http_lan_origin".to_owned())?
            .trim_end_matches('/');
        if authority.is_empty() || authority.contains('/') || authority.contains('@') {
            return Err("evidence_agent_server_invalid".to_owned());
        }
        let (socket_host, port) = authority.rsplit_once(':').map_or_else(
            || Ok::<_, String>((authority.to_owned(), 80)),
            |(host, port)| {
                let port = port
                    .parse::<u16>()
                    .map_err(|_| "evidence_agent_server_port_invalid".to_owned())?;
                Ok((host.to_owned(), port))
            },
        )?;
        if socket_host.is_empty() {
            return Err("evidence_agent_server_invalid".to_owned());
        }
        Ok(Self {
            socket_host,
            port,
            host_header: authority.to_owned(),
            path: REMOTE_EVIDENCE_ENDPOINT_V1.to_owned(),
        })
    }

    fn post_batch(
        &self,
        client_id_sha256: &str,
        timestamp_unix: u64,
        signature: &str,
        body: &[u8],
    ) -> Result<RemoteEvidenceAckV1, String> {
        let addresses = (self.socket_host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| format!("evidence_agent_resolve:{error}"))?
            .collect::<Vec<_>>();
        let mut stream = addresses
            .iter()
            .find_map(|address| TcpStream::connect_timeout(address, Duration::from_secs(5)).ok())
            .ok_or_else(|| "evidence_agent_connect_failed".to_owned())?;
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(20))))
            .map_err(|error| format!("evidence_agent_socket_timeout:{error}"))?;
        let headers = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/cbor\r\nContent-Length: {}\r\nX-Nando-Evidence-Client: {}\r\nX-Nando-Evidence-Timestamp: {}\r\nX-Nando-Evidence-Signature: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.host_header,
            body.len(),
            client_id_sha256,
            timestamp_unix,
            signature
        );
        stream
            .write_all(headers.as_bytes())
            .and_then(|()| stream.write_all(body))
            .and_then(|()| stream.flush())
            .map_err(|error| format!("evidence_agent_send:{error}"))?;
        let mut response = Vec::new();
        Read::by_ref(&mut stream)
            .take(MAX_HTTP_RESPONSE_BYTES.saturating_add(1))
            .read_to_end(&mut response)
            .map_err(|error| format!("evidence_agent_receive:{error}"))?;
        if response.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            return Err("evidence_agent_response_budget".to_owned());
        }
        let (status, response_body) = parse_http_response(&response)?;
        if status != 200 {
            let error = serde_json::from_slice::<serde_json::Value>(&response_body)
                .ok()
                .and_then(|value| {
                    value
                        .get("error")
                        .and_then(|error| error.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "remote_rejected".to_owned());
            return Err(format!("evidence_agent_http_{status}:{error}"));
        }
        serde_json::from_slice(&response_body)
            .map_err(|error| format!("evidence_agent_ack_decode:{error}"))
    }
}

fn parse_http_response(response: &[u8]) -> Result<(u16, Vec<u8>), String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "evidence_agent_http_header_incomplete".to_owned())?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| "evidence_agent_http_header_invalid".to_owned())?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_ascii_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "evidence_agent_http_status_invalid".to_owned())?;
    let body = response[header_end.saturating_add(4)..].to_vec();
    if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        return decode_chunked_body(&body).map(|body| (status, body));
    }
    if let Some(length) = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    }) && body.len() != length
    {
        return Err("evidence_agent_http_body_incomplete".to_owned());
    }
    Ok((status, body))
}

fn decode_chunked_body(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = 0_usize;
    let mut output = Vec::new();
    loop {
        let line_end = encoded[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor.saturating_add(offset))
            .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?;
        let length_text = std::str::from_utf8(&encoded[cursor..line_end])
            .map_err(|_| "evidence_agent_http_chunk_invalid".to_owned())?
            .split(';')
            .next()
            .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?;
        let length = usize::from_str_radix(length_text, 16)
            .map_err(|_| "evidence_agent_http_chunk_invalid".to_owned())?;
        cursor = line_end.saturating_add(2);
        if length == 0 {
            return Ok(output);
        }
        let end = cursor.saturating_add(length);
        if encoded.get(end..end.saturating_add(2)) != Some(b"\r\n") {
            return Err("evidence_agent_http_chunk_invalid".to_owned());
        }
        output.extend_from_slice(
            encoded
                .get(cursor..end)
                .ok_or_else(|| "evidence_agent_http_chunk_invalid".to_owned())?,
        );
        if output.len() as u64 > MAX_HTTP_RESPONSE_BYTES {
            return Err("evidence_agent_response_budget".to_owned());
        }
        cursor = end.saturating_add(2);
    }
}

