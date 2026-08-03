use std::collections::BTreeMap;

use serde_json::Value;

use super::selection::runtime_embedded_json_objects;

pub fn canonical_collection_from_provider_output(output: &Value) -> Result<Value, &'static str> {
    match output {
        Value::Object(_) => canonical_collection_root(output.clone()),
        Value::Array(parts) if !is_text_part_array(output) => {
            canonical_collection_root(serde_json::json!({"items": parts}))
        }
        Value::String(text) => canonical_collection_from_texts([text.as_str()]),
        Value::Array(parts) if !parts.is_empty() && parts.len() <= 64 => {
            let texts = parts
                .iter()
                .map(|part| {
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or("collection_output_part_text")
                })
                .collect::<Result<Vec<_>, _>>()?;
            canonical_collection_from_texts(texts)
        }
        _ => Err("collection_output_not_text_or_structured"),
    }
}

fn canonical_collection_from_texts<'a>(
    texts: impl IntoIterator<Item = &'a str>,
) -> Result<Value, &'static str> {
    let mut candidates = BTreeMap::<Vec<u8>, Value>::new();
    let mut total_bytes = 0_usize;
    for text in texts {
        total_bytes = total_bytes
            .checked_add(text.len())
            .ok_or("collection_input_budget")?;
        if text.is_empty() || total_bytes > 65_536 {
            return Err("collection_input_budget");
        }
        collect_collection_candidates(text, &mut candidates)?;
    }
    if candidates.len() != 1 {
        return Err(if candidates.is_empty() {
            "collection_input_not_json"
        } else {
            "collection_input_ambiguous"
        });
    }
    Ok(candidates.into_values().next().expect("one candidate"))
}

fn collect_collection_candidates(
    output: &str,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), &'static str> {
    let mut sources = vec![output.to_owned()];
    let mut fenced = None::<String>;
    for line in output.lines() {
        let trimmed = line.trim();
        if fenced.is_some() && trimmed == "```" {
            sources.push(fenced.take().unwrap_or_default());
        } else if fenced.is_some() {
            let buffer = fenced.as_mut().expect("checked above");
            if !buffer.is_empty() {
                buffer.push('\n');
            }
            buffer.push_str(line);
        } else if trimmed == "```" || trimmed.eq_ignore_ascii_case("```json") {
            fenced = Some(String::new());
        } else if trimmed.starts_with(['{', '[']) {
            sources.push(trimmed.to_owned());
        }
    }
    for source in sources {
        if source.is_empty() || source.len() > 16_384 {
            continue;
        }
        for object in runtime_embedded_json_objects(&source) {
            insert_candidate(Value::Object(object), candidates)?;
        }
        if let Ok(value @ Value::Array(_)) = serde_json::from_str::<Value>(&source)
            && !is_text_part_array(&value)
        {
            insert_candidate(serde_json::json!({"items": value}), candidates)?;
        }
    }
    Ok(())
}

fn insert_candidate(
    value: Value,
    candidates: &mut BTreeMap<Vec<u8>, Value>,
) -> Result<(), &'static str> {
    if bounded_collection_root(&value) {
        let key = serde_json::to_vec(&value).map_err(|_| "collection_serialization")?;
        candidates.insert(key, value);
    }
    Ok(())
}

fn canonical_collection_root(value: Value) -> Result<Value, &'static str> {
    if !bounded_collection_root(&value) {
        return Err("collection_root_invalid");
    }
    Ok(value)
}

fn is_text_part_array(value: &Value) -> bool {
    value.as_array().is_some_and(|parts| {
        !parts.is_empty()
            && parts.iter().all(|part| {
                matches!(
                    part.get("type").and_then(Value::as_str),
                    Some("text" | "input_text" | "output_text")
                ) && part.get("text").is_some_and(Value::is_string)
            })
    })
}

fn bounded_collection_root(value: &Value) -> bool {
    let Some(object) = value.as_object().filter(|object| object.len() <= 16) else {
        return false;
    };
    let mut arrays = object.values().filter_map(Value::as_array);
    let Some(rows) = arrays.next() else {
        return false;
    };
    if arrays.next().is_some() || rows.is_empty() || rows.len() > 1_024 {
        return false;
    }
    rows.iter().all(|row| {
        row.as_object().is_some_and(|fields| {
            !fields.is_empty()
                && fields.len() <= 16
                && fields.iter().all(|(name, value)| {
                    safe_collection_identifier(name) && safe_collection_scalar(value)
                })
        })
    })
}

fn safe_collection_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if value.len() > 64 || !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        && !private_fragments()
            .iter()
            .any(|private| lower.contains(private))
}

fn safe_collection_scalar(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::Number(number) => {
            number
                .as_i64()
                .is_some_and(|value| (-(1_i64 << 53)..=(1_i64 << 53)).contains(&value))
                || number.as_u64().is_some_and(|value| value <= (1_u64 << 53))
        }
        Value::String(text) => {
            text.len() <= 128
                && !private_fragments()
                    .iter()
                    .any(|private| text.to_ascii_lowercase().contains(private))
        }
        Value::Array(_) | Value::Object(_) => false,
    }
}

const fn private_fragments() -> &'static [&'static str] {
    &[
        "auth",
        "cookie",
        "credential",
        "passwd",
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "private_key",
        "privatekey",
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::canonical_collection_from_provider_output;

    #[test]
    fn structured_and_text_forms_share_one_canonical_value() {
        let object = json!({"rows": [{"status": "ready", "value": 7}]});
        let array = json!([{"status": "ready", "value": 7}]);
        let expected_array = json!({"items": [{"status": "ready", "value": 7}]});

        assert_eq!(
            canonical_collection_from_provider_output(&object).expect("object"),
            object
        );
        assert_eq!(
            canonical_collection_from_provider_output(&array).expect("array"),
            expected_array
        );
        assert_eq!(
            canonical_collection_from_provider_output(&Value::String(array.to_string()))
                .expect("json string"),
            expected_array
        );
    }

    #[test]
    fn text_parts_decode_through_the_same_contract() {
        let parts = json!([
            {"type": "output_text", "text": "```json\n"},
            {"type": "output_text", "text": "{\"rows\":[{\"value\":7}]}\n```"}
        ]);
        assert_eq!(
            canonical_collection_from_provider_output(&parts).expect("parts"),
            json!({"rows": [{"value": 7}]})
        );
    }
}
