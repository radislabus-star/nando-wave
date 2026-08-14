use std::error::Error;
use std::io::{self, Read, Write};

use nando_operator_learning::{K2ExactOracleOutcomeV1, K2ExactOracleRequestV1};

const MAX_ORACLE_REQUEST_BYTES_V1: u64 = 64 * 1024;

fn main() -> Result<(), Box<dyn Error>> {
    let mut bytes = Vec::new();
    io::stdin()
        .take(MAX_ORACLE_REQUEST_BYTES_V1 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ORACLE_REQUEST_BYTES_V1 {
        return Err(io::Error::other("k2_exact_oracle_request_too_large").into());
    }
    let request = K2ExactOracleRequestV1::from_canonical_bytes(&bytes)?;
    let outcome = K2ExactOracleOutcomeV1::evaluate(&request)?;
    io::stdout().write_all(&outcome.canonical_bytes()?)?;
    Ok(())
}
