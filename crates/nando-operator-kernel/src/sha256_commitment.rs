use std::fmt;

use serde::de::{Error as _, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Sha256CommitmentV3([u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sha256CommitmentErrorV3 {
    InvalidHex,
    InvalidLength,
    Zero,
}

impl Sha256CommitmentV3 {
    pub fn new(bytes: [u8; 32]) -> Result<Self, Sha256CommitmentErrorV3> {
        if bytes.iter().all(|byte| *byte == 0) {
            return Err(Sha256CommitmentErrorV3::Zero);
        }
        Ok(Self(bytes))
    }

    pub fn from_hex(value: &str) -> Result<Self, Sha256CommitmentErrorV3> {
        if value.len() != 64 {
            return Err(Sha256CommitmentErrorV3::InvalidLength);
        }
        let mut bytes = [0_u8; 32];
        for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = decode_nibble(chunk[0]).ok_or(Sha256CommitmentErrorV3::InvalidHex)?;
            let low = decode_nibble(chunk[1]).ok_or(Sha256CommitmentErrorV3::InvalidHex)?;
            bytes[index] = (high << 4) | low;
        }
        Self::new(bytes)
    }

    #[must_use]
    pub fn digest_bytes(value: &[u8]) -> Self {
        Self(Sha256::digest(value).into())
    }

    #[must_use]
    pub fn digest_parts(domain: &[u8], parts: &[&[u8]]) -> Self {
        let mut digest = Sha256::new();
        digest.update((domain.len() as u64).to_be_bytes());
        digest.update(domain);
        for part in parts {
            digest.update((part.len() as u64).to_be_bytes());
            digest.update(part);
        }
        Self(digest.finalize().into())
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
        }
        output
    }
}

impl Serialize for Sha256CommitmentV3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Sha256CommitmentV3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(Sha256CommitmentVisitorV3)
    }
}

struct Sha256CommitmentVisitorV3;

impl<'de> Visitor<'de> for Sha256CommitmentVisitorV3 {
    type Value = Sha256CommitmentV3;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a non-zero 32-byte SHA-256 commitment")
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let bytes = <[u8; 32]>::try_from(value).map_err(|_| E::custom("invalid SHA-256 length"))?;
        Sha256CommitmentV3::new(bytes).map_err(|_| E::custom("zero SHA-256 commitment"))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_bytes(&value)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut bytes = [0_u8; 32];
        for byte in &mut bytes {
            *byte = sequence
                .next_element()?
                .ok_or_else(|| A::Error::custom("invalid SHA-256 length"))?;
        }
        if sequence.next_element::<u8>()?.is_some() {
            return Err(A::Error::custom("invalid SHA-256 length"));
        }
        Sha256CommitmentV3::new(bytes).map_err(|_| A::Error::custom("zero SHA-256 commitment"))
    }
}

const fn decode_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commitment_roundtrips_through_serde() {
        let commitment = Sha256CommitmentV3::digest_bytes(b"provider request");
        let encoded = serde_json::to_vec(&commitment).expect("json");
        let decoded: Sha256CommitmentV3 = serde_json::from_slice(&encoded).expect("decode");

        assert_eq!(decoded, commitment);
        assert_eq!(
            Sha256CommitmentV3::from_hex(&commitment.to_hex()),
            Ok(commitment)
        );
    }

    #[test]
    fn invalid_or_zero_commitments_are_rejected() {
        assert_eq!(
            Sha256CommitmentV3::from_hex(&"0".repeat(64)),
            Err(Sha256CommitmentErrorV3::Zero)
        );
        assert_eq!(
            Sha256CommitmentV3::from_hex(&"A".repeat(64)),
            Err(Sha256CommitmentErrorV3::InvalidHex)
        );
    }
}
