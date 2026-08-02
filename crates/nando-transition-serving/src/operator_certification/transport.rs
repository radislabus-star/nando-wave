use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const AUTHORITY_MAX_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const AUTHORITY_IO_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn read_authority_line(stream: &mut UnixStream) -> Result<String, String> {
    stream
        .set_read_timeout(Some(AUTHORITY_IO_TIMEOUT))
        .map_err(|error| format!("operator_certification_authority_read_timeout:{error}"))?;
    stream
        .set_write_timeout(Some(AUTHORITY_IO_TIMEOUT))
        .map_err(|error| format!("operator_certification_authority_write_timeout:{error}"))?;
    read_bounded_line(&mut BufReader::new(stream), AUTHORITY_MAX_REQUEST_BYTES)
}

fn read_bounded_line<R: BufRead>(reader: &mut R, maximum_bytes: usize) -> Result<String, String> {
    let mut bytes = Vec::with_capacity(maximum_bytes.min(64 * 1024));
    let mut limited = std::io::Read::take(
        &mut *reader,
        u64::try_from(maximum_bytes.saturating_add(2)).unwrap_or(u64::MAX),
    );
    limited
        .read_until(b'\n', &mut bytes)
        .map_err(|error| format!("operator_certification_authority_read:{error}"))?;
    if bytes.is_empty() {
        return Err("operator_certification_authority_request_empty".to_owned());
    }
    if bytes.last() != Some(&b'\n') {
        return Err(if bytes.len() > maximum_bytes {
            "operator_certification_authority_request_too_large".to_owned()
        } else {
            "operator_certification_authority_request_incomplete".to_owned()
        });
    }
    bytes.pop();
    if bytes.len() > maximum_bytes {
        return Err("operator_certification_authority_request_too_large".to_owned());
    }
    String::from_utf8(bytes)
        .map_err(|error| format!("operator_certification_authority_request_utf8:{error}"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn bounded_line_accepts_exact_payload_limit() {
        let mut bytes = vec![b'x'; 8];
        bytes.push(b'\n');
        assert_eq!(
            read_bounded_line(&mut Cursor::new(bytes), 8).expect("bounded line"),
            "xxxxxxxx"
        );
    }

    #[test]
    fn bounded_line_rejects_oversize_and_unterminated_requests() {
        assert_eq!(
            read_bounded_line(&mut Cursor::new(b"123456789\n"), 8),
            Err("operator_certification_authority_request_too_large".to_owned())
        );
        assert_eq!(
            read_bounded_line(&mut Cursor::new(b"{}"), 8),
            Err("operator_certification_authority_request_incomplete".to_owned())
        );
    }
}
