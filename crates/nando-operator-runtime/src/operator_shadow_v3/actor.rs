use std::collections::BTreeMap;

use nando_operator_kernel::{
    BoundProtocolProgramV3, BoundProtocolValueV3, RuntimeCapabilityKindV3,
};
use serde_json::{Value, json};

use super::ActorExecutionErrorV3;

pub(super) fn execute_actor_program_v3(
    program: &BoundProtocolProgramV3,
) -> Result<String, ActorExecutionErrorV3> {
    if program.capability_kind() != RuntimeCapabilityKindV3::Function {
        return Err(ActorExecutionErrorV3::UnsupportedCapability);
    }
    let mut arguments = BTreeMap::<String, Value>::new();
    for argument in program.arguments() {
        if arguments
            .insert(
                argument.physical_name().to_owned(),
                actor_value(argument.value()),
            )
            .is_some()
        {
            return Err(ActorExecutionErrorV3::DuplicateArgument);
        }
    }
    let output = serde_json::to_string(&json!({
        "name": program.physical_symbol(),
        "arguments": arguments,
    }))
    .map_err(|_| ActorExecutionErrorV3::Serialization)?;
    if output.is_empty() || output.len() > program.max_output_bytes() {
        return Err(ActorExecutionErrorV3::OutputBudget);
    }
    Ok(output)
}

fn actor_value(value: &BoundProtocolValueV3) -> Value {
    match value {
        BoundProtocolValueV3::String(value) | BoundProtocolValueV3::Identifier(value) => {
            Value::String(value.clone())
        }
        BoundProtocolValueV3::Integer(value) => Value::from(*value),
        BoundProtocolValueV3::Boolean(value) => Value::from(*value),
    }
}
