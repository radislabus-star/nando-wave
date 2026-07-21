use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BindingProofCanonicalError {
    Serialization,
}

pub(super) fn pretty_json_bytes<T: Serialize>(
    value: &T,
) -> Result<Vec<u8>, BindingProofCanonicalError> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|_| BindingProofCanonicalError::Serialization)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn sha256_json<T: Serialize>(value: &T) -> Result<String, BindingProofCanonicalError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| BindingProofCanonicalError::Serialization)
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
