use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, &'static str> {
    let value = serde_json::to_value(value).map_err(|_| "canonical_json_serialize_failed")?;
    let mut output = Vec::new();
    write_canonical_json(&value, &mut output)?;
    Ok(output)
}

pub fn canonical_json_sha256<T: Serialize>(value: &T) -> Result<String, &'static str> {
    Ok(sha256_bytes(&canonical_json_bytes(value)?))
}

#[must_use]
pub fn sha256_bytes(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[must_use]
pub fn valid_nonzero_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

fn write_canonical_json(value: &Value, output: &mut Vec<u8>) -> Result<(), &'static str> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::Number(number) => {
            if number.as_i64().is_none() && number.as_u64().is_none() {
                return Err("canonical_json_float_unsupported");
            }
            output.extend_from_slice(number.to_string().as_bytes());
        }
        Value::String(string) => output.extend_from_slice(
            serde_json::to_string(string)
                .map_err(|_| "canonical_json_string_failed")?
                .as_bytes(),
        ),
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_canonical_json(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .map_err(|_| "canonical_json_key_failed")?
                        .as_bytes(),
                );
                output.push(b':');
                write_canonical_json(&values[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn canonical_json_sorts_objects_and_rejects_floats() {
        assert_eq!(
            canonical_json_bytes(&json!({"z":2,"a":{"y":1,"b":true}})),
            Ok(br#"{"a":{"b":true,"y":1},"z":2}"#.to_vec())
        );
        assert_eq!(
            canonical_json_bytes(&json!({"float": 0.5})),
            Err("canonical_json_float_unsupported")
        );
    }
}
