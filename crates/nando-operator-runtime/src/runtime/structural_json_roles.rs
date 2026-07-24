use nando_operator_kernel::AtomValueType;
use serde_json::Value;

use super::selection::{
    continuation_handle_scalar, identifier_tokens, observed_json_path_digest,
    request_identifier_positions,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub struct ObservedJsonScalarRole {
    pub request_position: Option<u16>,
    pub json_path_sha256: [u8; 32],
    pub value_sha256: String,
    pub value_type: AtomValueType,
    pub depth_bucket: u8,
}

/// Captures bounded scalar role witnesses with runtime-equivalent request
/// semantics while retaining neither field names nor values.
#[doc(hidden)]
pub fn observed_json_scalar_roles(
    request: &str,
    output: &Value,
) -> Result<Vec<ObservedJsonScalarRole>, &'static str> {
    if request.len() > 16_384 {
        return Err("observed_json_role_request_budget");
    }
    let request_tokens = identifier_tokens(request);
    let mut path = Vec::new();
    let mut roles = Vec::new();
    collect_json_scalar_roles(output, &request_tokens, &mut path, 0, None, &mut roles)?;
    Ok(roles)
}

/// Exposes the semantic continuation role to pre-action topology capture. The
/// protocol parser remains runtime-owned; learning sees only type and hashes.
#[doc(hidden)]
pub fn observed_continuation_handle_role(
    payload: &Value,
) -> Result<ObservedJsonScalarRole, &'static str> {
    let scalar = continuation_handle_scalar(payload, AtomValueType::Identifier)?;
    Ok(ObservedJsonScalarRole {
        request_position: None,
        json_path_sha256: observed_json_path_digest(&["semantic:continuation_handle".to_owned()]),
        value_sha256: nando_operator_kernel::canonical_json_sha256(&scalar.value)?,
        value_type: AtomValueType::Identifier,
        depth_bucket: 1,
    })
}

fn collect_json_scalar_roles(
    value: &Value,
    request_tokens: &[String],
    path: &mut Vec<String>,
    depth: usize,
    request_position: Option<usize>,
    output: &mut Vec<ObservedJsonScalarRole>,
) -> Result<(), &'static str> {
    if depth > 8 || output.len() >= 64 {
        return Err("observed_json_role_structure_budget");
    }
    match value {
        Value::Object(object) => {
            for (field, value) in object {
                path.push(format!("k:{field}"));
                let positions = request_identifier_positions(request_tokens, field);
                if positions.len() > 1 {
                    return Err("observed_json_role_mention_ambiguous");
                }
                collect_json_scalar_roles(
                    value,
                    request_tokens,
                    path,
                    depth.saturating_add(1),
                    positions.first().copied(),
                    output,
                )?;
                path.pop();
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                path.push(format!("i:{index}"));
                collect_json_scalar_roles(
                    value,
                    request_tokens,
                    path,
                    depth.saturating_add(1),
                    None,
                    output,
                )?;
                path.pop();
            }
        }
        scalar => {
            let Some(value_type) = scalar_type(scalar) else {
                return Ok(());
            };
            output.push(ObservedJsonScalarRole {
                request_position: request_position
                    .map(u16::try_from)
                    .transpose()
                    .map_err(|_| "observed_json_role_request_position")?,
                json_path_sha256: observed_json_path_digest(path),
                value_sha256: nando_operator_kernel::canonical_json_sha256(scalar)?,
                value_type,
                depth_bucket: u8::try_from(depth.min(7)).map_err(|_| "observed_json_role_depth")?,
            });
        }
    }
    Ok(())
}

fn scalar_type(value: &Value) -> Option<AtomValueType> {
    match value {
        Value::String(_) => Some(AtomValueType::String),
        Value::Number(number) if number.is_i64() || number.is_u64() => Some(AtomValueType::Integer),
        Value::Bool(_) => Some(AtomValueType::Boolean),
        _ => None,
    }
}
