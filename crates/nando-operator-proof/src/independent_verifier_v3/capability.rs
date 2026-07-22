use std::collections::BTreeSet;

use nando_operator_kernel::{
    BindingCompletionStateV1, BindingValueTypeV1, RuntimeCapabilityDescriptorV3,
    RuntimeCapabilityKindV3, StructuralContextV3, canonical_json_sha256, sha256_bytes,
};
use serde_json::Value;

use super::IndependentVerifierBudgetV3;

#[derive(Clone, Debug)]
pub(super) struct IndependentCapabilityArgumentV3 {
    pub ordinal: u16,
    pub physical_name: String,
    pub value_type: BindingValueTypeV1,
    pub required: bool,
}

#[derive(Clone, Debug)]
pub(super) struct IndependentCapabilityV3 {
    pub capability_id: u16,
    pub kind: RuntimeCapabilityKindV3,
    pub physical_symbol: String,
    pub arguments: Box<[IndependentCapabilityArgumentV3]>,
    pub argument_topology_ambiguous: bool,
}

pub(super) struct CapabilitySurfaceV3 {
    pub descriptors: Vec<RuntimeCapabilityDescriptorV3>,
    pub capabilities: Vec<IndependentCapabilityV3>,
    pub context: StructuralContextV3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CapabilitySurfaceErrorV3 {
    BudgetExhausted,
    Serialization,
}

#[derive(Clone, Debug)]
struct CapabilitySourceV3 {
    kind: RuntimeCapabilityKindV3,
    physical_symbol: String,
    symbol_sha256: String,
    argument_types: Vec<BindingValueTypeV1>,
    arguments: Vec<CapabilityArgumentSourceV3>,
    required_arity: u16,
    argument_topology_ambiguous: bool,
}

#[derive(Clone, Debug)]
struct CapabilityArgumentSourceV3 {
    physical_name: String,
    value_type: BindingValueTypeV1,
    required: bool,
}

pub(super) fn extract_capability_surface_v3(
    payload: &Value,
    budget: IndependentVerifierBudgetV3,
) -> Result<CapabilitySurfaceV3, CapabilitySurfaceErrorV3> {
    let mut sources = Vec::new();
    let mut declarations_visited = 0_usize;
    if let Some(tools) = payload.get("tools").and_then(Value::as_array) {
        collect_sources_v3(tools, budget, &mut declarations_visited, &mut sources)?;
    }

    let input = payload
        .get("input")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let recent_start = input.len().saturating_sub(32);
    let recent = &input[recent_start..];
    let active_start = recent
        .iter()
        .rposition(|item| {
            item.get("type").and_then(Value::as_str) == Some("message")
                && item.get("role").and_then(Value::as_str) == Some("user")
        })
        .map_or(0, |index| index.saturating_add(1));
    let active = &recent[active_start..];
    let mut call_shape_count = 0_usize;
    let mut output_count = 0_usize;
    for item in active {
        match item.get("type").and_then(Value::as_str) {
            Some("function_call" | "custom_tool_call") => {
                call_shape_count = call_shape_count.saturating_add(1);
            }
            Some("function_call_output" | "custom_tool_call_output") => {
                output_count = output_count.saturating_add(1);
            }
            _ => {}
        }
        if item.get("type").and_then(Value::as_str) == Some("additional_tools") {
            for key in ["tools", "additional_tools", "definitions", "items"] {
                if let Some(tools) = item.get(key).and_then(Value::as_array) {
                    collect_sources_v3(tools, budget, &mut declarations_visited, &mut sources)?;
                }
            }
        }
    }

    sources.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.argument_types.cmp(&right.argument_types))
            .then_with(|| left.required_arity.cmp(&right.required_arity))
            .then_with(|| {
                left.argument_topology_ambiguous
                    .cmp(&right.argument_topology_ambiguous)
            })
            .then_with(|| left.symbol_sha256.cmp(&right.symbol_sha256))
    });
    let context = structural_context_v3(call_shape_count, output_count, active.len(), &sources)?;
    let mut descriptors = Vec::with_capacity(sources.len());
    let mut capabilities = Vec::with_capacity(sources.len());
    for (index, source) in sources.into_iter().enumerate() {
        let capability_id =
            u16::try_from(index).map_err(|_| CapabilitySurfaceErrorV3::BudgetExhausted)?;
        descriptors.push(RuntimeCapabilityDescriptorV3 {
            capability_id,
            kind: source.kind,
            argument_types: source.argument_types,
            required_arity: source.required_arity,
        });
        capabilities.push(IndependentCapabilityV3 {
            capability_id,
            kind: source.kind,
            physical_symbol: source.physical_symbol,
            arguments: source
                .arguments
                .into_iter()
                .enumerate()
                .map(|(ordinal, argument)| IndependentCapabilityArgumentV3 {
                    ordinal: u16::try_from(ordinal).unwrap_or(u16::MAX),
                    physical_name: argument.physical_name,
                    value_type: argument.value_type,
                    required: argument.required,
                })
                .collect(),
            argument_topology_ambiguous: source.argument_topology_ambiguous,
        });
    }
    Ok(CapabilitySurfaceV3 {
        descriptors,
        capabilities,
        context,
    })
}

fn collect_sources_v3(
    declarations: &[Value],
    budget: IndependentVerifierBudgetV3,
    declarations_visited: &mut usize,
    sources: &mut Vec<CapabilitySourceV3>,
) -> Result<(), CapabilitySurfaceErrorV3> {
    if declarations.len()
        > budget
            .max_capabilities
            .saturating_sub(*declarations_visited)
    {
        return Err(CapabilitySurfaceErrorV3::BudgetExhausted);
    }
    for declaration in declarations {
        *declarations_visited = declarations_visited.saturating_add(1);
        if let Some(source) = capability_source_v3(declaration, budget.max_role_candidates)? {
            sources.push(source);
        }
    }
    Ok(())
}

fn capability_source_v3(
    declaration: &Value,
    max_arguments: usize,
) -> Result<Option<CapabilitySourceV3>, CapabilitySurfaceErrorV3> {
    let kind = match declaration
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("function")
    {
        "function" => RuntimeCapabilityKindV3::Function,
        "custom" | "custom_tool" => RuntimeCapabilityKindV3::Custom,
        _ => return Ok(None),
    };
    let Some(physical_symbol) = declaration.get("name").and_then(Value::as_str).or_else(|| {
        declaration
            .pointer("/function/name")
            .and_then(Value::as_str)
    }) else {
        return Ok(None);
    };
    if !valid_physical_name_v3(physical_symbol) {
        return Ok(None);
    }
    let parameters = declaration
        .get("parameters")
        .or_else(|| declaration.pointer("/function/parameters"));
    let properties = parameters
        .and_then(|value| value.get("properties"))
        .and_then(Value::as_object);
    let required = parameters
        .and_then(|value| value.get("required"))
        .and_then(Value::as_array);
    if properties.is_some_and(|value| value.len() > max_arguments)
        || required.is_some_and(|value| value.len() > max_arguments)
    {
        return Err(CapabilitySurfaceErrorV3::BudgetExhausted);
    }
    let required_names = required
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let Some(mut arguments) = properties
        .map(|values| {
            values
                .iter()
                .map(|(name, schema)| {
                    schema_value_type_v3(schema).map(|value_type| CapabilityArgumentSourceV3 {
                        physical_name: name.clone(),
                        value_type,
                        required: required_names.contains(name.as_str()),
                    })
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_else(|| Some(Vec::new()))
    else {
        return Ok(None);
    };
    arguments.sort_by(|left, right| {
        left.value_type
            .cmp(&right.value_type)
            .then_with(|| right.required.cmp(&left.required))
            .then_with(|| left.physical_name.cmp(&right.physical_name))
    });
    let argument_topology_ambiguous = arguments.windows(2).any(|pair| {
        pair[0].value_type == pair[1].value_type && pair[0].required == pair[1].required
    });
    let argument_types = arguments.iter().map(|value| value.value_type).collect();
    Ok(Some(CapabilitySourceV3 {
        kind,
        physical_symbol: physical_symbol.to_owned(),
        symbol_sha256: sha256_bytes(physical_symbol.as_bytes()),
        argument_types,
        arguments,
        required_arity: u16::try_from(required.map_or(0, Vec::len)).unwrap_or(u16::MAX),
        argument_topology_ambiguous,
    }))
}

fn schema_value_type_v3(schema: &Value) -> Option<BindingValueTypeV1> {
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => Some(BindingValueTypeV1::String),
        Some("integer" | "number") => Some(BindingValueTypeV1::Integer),
        Some("boolean") => Some(BindingValueTypeV1::Boolean),
        Some("identifier") => Some(BindingValueTypeV1::Identifier),
        _ => None,
    }
}

fn structural_context_v3(
    call_shape_count: usize,
    output_count: usize,
    active_len: usize,
    capabilities: &[CapabilitySourceV3],
) -> Result<StructuralContextV3, CapabilitySurfaceErrorV3> {
    let completion_state = if call_shape_count > output_count {
        BindingCompletionStateV1::Unresolved
    } else if call_shape_count > 0 || output_count > 0 {
        BindingCompletionStateV1::Completed
    } else {
        BindingCompletionStateV1::Unknown
    };
    let topology_neighborhood_root_sha256 = canonical_json_sha256(&(
        u16::try_from(call_shape_count).unwrap_or(u16::MAX),
        u16::try_from(capabilities.len()).unwrap_or(u16::MAX),
        completion_state,
        count_band_v3(active_len),
        capabilities
            .iter()
            .map(|capability| {
                (
                    capability.kind,
                    capability.argument_types.clone(),
                    capability.required_arity,
                )
            })
            .collect::<BTreeSet<_>>(),
    ))
    .map_err(|_| CapabilitySurfaceErrorV3::Serialization)?;
    Ok(StructuralContextV3 {
        call_shape_count: u16::try_from(call_shape_count).unwrap_or(u16::MAX),
        capability_count: u16::try_from(capabilities.len()).unwrap_or(u16::MAX),
        completion_state,
        temporal_relation_count: u16::try_from(active_len.saturating_sub(1)).unwrap_or(u16::MAX),
        cardinality_relation_count: 5,
        topology_neighborhood_root_sha256,
    })
}

const fn count_band_v3(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        1_usize << (usize::BITS - 1 - value.leading_zeros())
    }
}

fn valid_physical_name_v3(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}
