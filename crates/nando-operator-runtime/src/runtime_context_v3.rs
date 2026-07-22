use std::collections::BTreeSet;

use nando_operator_kernel::{
    BindingCompletionStateV1, BindingValueTypeV1, BoundProtocolValueV3,
    CanonicalRuntimeRequestViewV3, CanonicalStructuralSourceBindingV3, ExtractionReceiptV3,
    RuntimeCapabilityDescriptorV3, RuntimeCapabilityKindV3, RuntimeContextExtractionVerdictV3,
    RuntimeProjectionV3, StructuralCandidateObservationV3, StructuralContextV3,
    StructuralExtractionBudgetV3, StructuralExtractionErrorV3, StructuralExtractionScopeV3,
    build_canonical_runtime_request_view_v3, build_extraction_receipt_v3, canonical_json_sha256,
    canonicalize_runtime_structural_projection_v3, extract_structural_surface_v3, sha256_bytes,
    validate_extraction_receipt_v3,
};
use serde_json::Value;

use crate::request_phase_atom_ids;

pub const RUNTIME_CONTEXT_MAX_JSON_NODES_V3: usize = 4_096;
pub const RUNTIME_CONTEXT_MAX_TEXT_BYTES_V3: usize = 64 * 1_024;
pub const RUNTIME_CONTEXT_MAX_RECENT_EVENTS_V3: usize = 32;
pub const RUNTIME_CONTEXT_MAX_ROLE_CANDIDATES_V3: usize = 64;
pub const RUNTIME_CONTEXT_MAX_RELATIONS_V3: usize = 256;
pub const RUNTIME_CONTEXT_MAX_CAPABILITIES_V3: usize = 64;
pub const RUNTIME_CONTEXT_MAX_REQUEST_TEXT_BYTES_V3: usize = 16 * 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeContextBudgetV3 {
    pub max_json_nodes: usize,
    pub max_text_bytes: usize,
    pub max_recent_events: usize,
    pub max_role_candidates: usize,
    pub max_relations: usize,
    pub max_capabilities: usize,
}

impl Default for RuntimeContextBudgetV3 {
    fn default() -> Self {
        Self {
            max_json_nodes: RUNTIME_CONTEXT_MAX_JSON_NODES_V3,
            max_text_bytes: RUNTIME_CONTEXT_MAX_TEXT_BYTES_V3,
            max_recent_events: RUNTIME_CONTEXT_MAX_RECENT_EVENTS_V3,
            max_role_candidates: RUNTIME_CONTEXT_MAX_ROLE_CANDIDATES_V3,
            max_relations: RUNTIME_CONTEXT_MAX_RELATIONS_V3,
            max_capabilities: RUNTIME_CONTEXT_MAX_CAPABILITIES_V3,
        }
    }
}

impl RuntimeContextBudgetV3 {
    fn validate(self) -> Result<Self, RuntimeContextErrorV3> {
        if self.max_capabilities == 0
            || self.max_capabilities > RUNTIME_CONTEXT_MAX_CAPABILITIES_V3
            || self.max_json_nodes > RUNTIME_CONTEXT_MAX_JSON_NODES_V3
            || self.max_text_bytes > RUNTIME_CONTEXT_MAX_TEXT_BYTES_V3
            || self.max_recent_events > RUNTIME_CONTEXT_MAX_RECENT_EVENTS_V3
            || self.max_role_candidates > RUNTIME_CONTEXT_MAX_ROLE_CANDIDATES_V3
            || self.max_relations > RUNTIME_CONTEXT_MAX_RELATIONS_V3
        {
            return Err(RuntimeContextErrorV3::InvalidBudget);
        }
        StructuralExtractionBudgetV3 {
            max_json_nodes: self.max_json_nodes,
            max_text_bytes: self.max_text_bytes,
            max_recent_events: self.max_recent_events,
            max_role_candidates: self.max_role_candidates,
            max_relations: self.max_relations,
        }
        .validate()
        .map_err(|_| RuntimeContextErrorV3::InvalidBudget)?;
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeCapabilityArgumentBindingV3<'a> {
    pub argument_ordinal: u16,
    pub physical_name: &'a str,
    pub value_type: BindingValueTypeV1,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCapabilityBindingV3<'a> {
    pub capability_id: u16,
    pub kind: RuntimeCapabilityKindV3,
    pub physical_symbol: &'a str,
    pub arguments: Box<[RuntimeCapabilityArgumentBindingV3<'a>]>,
    pub argument_topology_ambiguous: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRoleValueBindingV3 {
    role_id: u16,
    value_sha256: String,
    values: Box<[BoundProtocolValueV3]>,
}

impl RuntimeRoleValueBindingV3 {
    #[must_use]
    pub const fn role_id(&self) -> u16 {
        self.role_id
    }

    #[must_use]
    pub fn value_sha256(&self) -> &str {
        &self.value_sha256
    }

    #[must_use]
    pub fn values(&self) -> &[BoundProtocolValueV3] {
        &self.values
    }
}

pub struct CanonicalRuntimeRequestV3<'a> {
    request_sha256: String,
    view: CanonicalRuntimeRequestViewV3,
    provider_payload: &'a Value,
    capability_bindings: Vec<RuntimeCapabilityBindingV3<'a>>,
    role_values: Box<[RuntimeRoleValueBindingV3]>,
}

impl<'a> CanonicalRuntimeRequestV3<'a> {
    #[must_use]
    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    #[must_use]
    pub const fn view(&self) -> &CanonicalRuntimeRequestViewV3 {
        &self.view
    }

    #[must_use]
    pub const fn provider_payload(&self) -> &'a Value {
        self.provider_payload
    }

    #[must_use]
    pub fn capability_bindings(&self) -> &[RuntimeCapabilityBindingV3<'a>] {
        &self.capability_bindings
    }

    #[must_use]
    pub fn role_values(&self) -> &[RuntimeRoleValueBindingV3] {
        &self.role_values
    }

    #[must_use]
    pub fn role_value(&self, role_id: u16) -> Option<&RuntimeRoleValueBindingV3> {
        self.role_values
            .binary_search_by_key(&role_id, RuntimeRoleValueBindingV3::role_id)
            .ok()
            .and_then(|index| self.role_values.get(index))
    }
}

pub struct RuntimeContextExtractionOutcomeV3<'a> {
    context: Option<CanonicalRuntimeRequestV3<'a>>,
    receipt: ExtractionReceiptV3,
}

impl<'a> RuntimeContextExtractionOutcomeV3<'a> {
    #[must_use]
    pub const fn context(&self) -> Option<&CanonicalRuntimeRequestV3<'a>> {
        self.context.as_ref()
    }

    #[must_use]
    pub const fn receipt(&self) -> &ExtractionReceiptV3 {
        &self.receipt
    }

    #[must_use]
    pub fn into_context(self) -> Option<CanonicalRuntimeRequestV3<'a>> {
        self.context
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeContextErrorV3 {
    InvalidBudget,
    InvalidRequestDigest,
    Structural,
    Serialization,
}

#[derive(Clone, Debug)]
struct CapabilitySource<'a> {
    kind: RuntimeCapabilityKindV3,
    physical_symbol: &'a str,
    symbol_sha256: String,
    argument_types: Vec<BindingValueTypeV1>,
    arguments: Vec<CapabilityArgumentSource<'a>>,
    required_arity: u16,
    argument_topology_ambiguous: bool,
}

#[derive(Clone, Debug)]
struct CapabilityArgumentSource<'a> {
    physical_name: &'a str,
    value_type: BindingValueTypeV1,
    required: bool,
}

struct RuntimeInputSynopsis<'a> {
    capability_sources: Vec<CapabilitySource<'a>>,
    capability_exhausted: bool,
    context: StructuralContextV3,
}

pub fn extract_canonical_runtime_request_v3<'a>(
    request_sha256: &str,
    request_text: &str,
    projection: RuntimeProjectionV3,
    provider_payload: &'a Value,
    budget: RuntimeContextBudgetV3,
) -> Result<RuntimeContextExtractionOutcomeV3<'a>, RuntimeContextErrorV3> {
    let budget = budget.validate()?;
    if !is_sha256(request_sha256) {
        return Err(RuntimeContextErrorV3::InvalidRequestDigest);
    }
    if request_text.len() > RUNTIME_CONTEXT_MAX_REQUEST_TEXT_BYTES_V3 {
        let receipt = build_extraction_receipt_v3(
            request_sha256.to_owned(),
            None,
            projection,
            RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted,
            0,
            0,
            0,
            0,
            0,
        )
        .map_err(|_| RuntimeContextErrorV3::Serialization)?;
        return Ok(RuntimeContextExtractionOutcomeV3 {
            context: None,
            receipt,
        });
    }
    let synopsis = runtime_input_synopsis(provider_payload, budget)?;
    let extraction = extract_structural_surface_v3(
        request_text,
        provider_payload,
        synopsis.context.clone(),
        StructuralExtractionBudgetV3 {
            max_json_nodes: budget.max_json_nodes,
            max_text_bytes: budget.max_text_bytes,
            max_recent_events: budget.max_recent_events,
            max_role_candidates: budget.max_role_candidates,
            max_relations: budget.max_relations,
        },
        StructuralExtractionScopeV3::PreActionRuntime,
    )
    .map_err(|_| RuntimeContextErrorV3::Structural)?;

    if synopsis.capability_exhausted
        || extraction.candidate_budget_exhausted
        || extraction.relation_budget_exhausted
    {
        let receipt = build_extraction_receipt_v3(
            request_sha256.to_owned(),
            None,
            projection,
            RuntimeContextExtractionVerdictV3::AbstainBudgetExhausted,
            extraction.json_nodes_visited,
            extraction.text_bytes_visited,
            extraction.candidates.len(),
            extraction.relations.len(),
            synopsis.capability_sources.len(),
        )
        .map_err(|_| RuntimeContextErrorV3::Serialization)?;
        return Ok(RuntimeContextExtractionOutcomeV3 {
            context: None,
            receipt,
        });
    }

    let (structural, source_bindings) =
        canonicalize_runtime_structural_projection_v3(synopsis.context, &extraction)
            .map_err(|_| RuntimeContextErrorV3::Structural)?;
    let role_values = runtime_role_values(&extraction.candidates, &source_bindings)?;
    let (capabilities, capability_bindings) = canonical_capabilities(synopsis.capability_sources)
        .map_err(|_| RuntimeContextErrorV3::Structural)?;
    let view = build_canonical_runtime_request_view_v3(
        projection,
        request_phase_atom_ids(request_text),
        structural,
        capabilities,
    )
    .map_err(|_| RuntimeContextErrorV3::Structural)?;
    let receipt = build_extraction_receipt_v3(
        request_sha256.to_owned(),
        Some(view.request_view_sha256.clone()),
        projection,
        RuntimeContextExtractionVerdictV3::Complete,
        extraction.json_nodes_visited,
        extraction.text_bytes_visited,
        extraction.candidates.len(),
        extraction.relations.len(),
        capability_bindings.len(),
    )
    .map_err(|_| RuntimeContextErrorV3::Serialization)?;
    validate_extraction_receipt_v3(&receipt).map_err(|_| RuntimeContextErrorV3::Serialization)?;
    Ok(RuntimeContextExtractionOutcomeV3 {
        context: Some(CanonicalRuntimeRequestV3 {
            request_sha256: request_sha256.to_owned(),
            view,
            provider_payload,
            capability_bindings,
            role_values,
        }),
        receipt,
    })
}

fn runtime_input_synopsis<'a>(
    payload: &'a Value,
    budget: RuntimeContextBudgetV3,
) -> Result<RuntimeInputSynopsis<'a>, RuntimeContextErrorV3> {
    // This is a bounded shallow synopsis; the kernel owns the only recursive walk.
    let mut sources = Vec::new();
    let mut declarations_visited = 0_usize;
    let mut capability_exhausted = false;
    if let Some(tools) = payload.get("tools").and_then(Value::as_array) {
        collect_capability_sources(
            tools,
            budget,
            &mut declarations_visited,
            &mut sources,
            &mut capability_exhausted,
        );
    }
    let input = payload
        .get("input")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let recent_start = input.len().saturating_sub(budget.max_recent_events);
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
                    collect_capability_sources(
                        tools,
                        budget,
                        &mut declarations_visited,
                        &mut sources,
                        &mut capability_exhausted,
                    );
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
    let context =
        runtime_structural_context(call_shape_count, output_count, active.len(), &sources)
            .map_err(|_| RuntimeContextErrorV3::Serialization)?;
    Ok(RuntimeInputSynopsis {
        capability_sources: sources,
        capability_exhausted,
        context,
    })
}

fn collect_capability_sources<'a>(
    declarations: &'a [Value],
    budget: RuntimeContextBudgetV3,
    declarations_visited: &mut usize,
    sources: &mut Vec<CapabilitySource<'a>>,
    exhausted: &mut bool,
) {
    let remaining = budget
        .max_capabilities
        .saturating_sub(*declarations_visited);
    if declarations.len() > remaining {
        *exhausted = true;
    }
    for declaration in declarations.iter().take(remaining) {
        *declarations_visited = declarations_visited.saturating_add(1);
        let (source, argument_budget_exhausted) =
            capability_source(declaration, budget.max_role_candidates);
        *exhausted |= argument_budget_exhausted;
        if let Some(source) = source {
            sources.push(source);
        }
    }
}

fn capability_source(
    declaration: &Value,
    max_arguments: usize,
) -> (Option<CapabilitySource<'_>>, bool) {
    let raw_kind = declaration
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("function");
    let kind = match raw_kind {
        "function" => RuntimeCapabilityKindV3::Function,
        "custom" | "custom_tool" => RuntimeCapabilityKindV3::Custom,
        _ => return (None, false),
    };
    let physical_symbol = declaration.get("name").and_then(Value::as_str).or_else(|| {
        declaration
            .pointer("/function/name")
            .and_then(Value::as_str)
    });
    let Some(physical_symbol) = physical_symbol else {
        return (None, false);
    };
    if !valid_capability_symbol(physical_symbol) {
        return (None, false);
    }
    let parameters = declaration
        .get("parameters")
        .or_else(|| declaration.pointer("/function/parameters"));
    let properties = parameters
        .and_then(|parameters| parameters.get("properties"))
        .and_then(Value::as_object);
    let required = parameters
        .and_then(|parameters| parameters.get("required"))
        .and_then(Value::as_array);
    let argument_budget_exhausted = properties.is_some_and(|value| value.len() > max_arguments)
        || required.is_some_and(|value| value.len() > max_arguments);
    if argument_budget_exhausted {
        return (None, true);
    }
    let required_names = required
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let mut arguments = properties
        .map(|properties| {
            properties
                .iter()
                .map(|(name, schema)| {
                    schema_value_type(schema).map(|value_type| CapabilityArgumentSource {
                        physical_name: name.as_str(),
                        value_type,
                        required: required_names.contains(name.as_str()),
                    })
                })
                .collect::<Option<Vec<_>>>()
        })
        .unwrap_or_else(|| Some(Vec::new()));
    let Some(mut arguments) = arguments.take() else {
        return (None, false);
    };
    arguments.sort_by(|left, right| {
        left.value_type
            .cmp(&right.value_type)
            .then_with(|| right.required.cmp(&left.required))
            .then_with(|| left.physical_name.cmp(right.physical_name))
    });
    let argument_topology_ambiguous = arguments.windows(2).any(|pair| {
        pair[0].value_type == pair[1].value_type && pair[0].required == pair[1].required
    });
    let argument_types = arguments
        .iter()
        .map(|argument| argument.value_type)
        .collect();
    let required_arity = required.map_or(0, Vec::len);
    (
        Some(CapabilitySource {
            kind,
            physical_symbol,
            symbol_sha256: sha256_bytes(physical_symbol.as_bytes()),
            argument_types,
            arguments,
            required_arity: u16::try_from(required_arity).unwrap_or(u16::MAX),
            argument_topology_ambiguous,
        }),
        false,
    )
}

fn schema_value_type(schema: &Value) -> Option<BindingValueTypeV1> {
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => Some(BindingValueTypeV1::String),
        Some("integer" | "number") => Some(BindingValueTypeV1::Integer),
        Some("boolean") => Some(BindingValueTypeV1::Boolean),
        Some("identifier") => Some(BindingValueTypeV1::Identifier),
        _ => None,
    }
}

fn canonical_capabilities<'a>(
    sources: Vec<CapabilitySource<'a>>,
) -> Result<
    (
        Vec<RuntimeCapabilityDescriptorV3>,
        Vec<RuntimeCapabilityBindingV3<'a>>,
    ),
    StructuralExtractionErrorV3,
> {
    let mut descriptors = Vec::with_capacity(sources.len());
    let mut bindings = Vec::with_capacity(sources.len());
    for (index, source) in sources.into_iter().enumerate() {
        let capability_id =
            u16::try_from(index).map_err(|_| StructuralExtractionErrorV3::BudgetExhausted)?;
        descriptors.push(RuntimeCapabilityDescriptorV3 {
            capability_id,
            kind: source.kind,
            argument_types: source.argument_types,
            required_arity: source.required_arity,
        });
        bindings.push(RuntimeCapabilityBindingV3 {
            capability_id,
            kind: source.kind,
            physical_symbol: source.physical_symbol,
            arguments: source
                .arguments
                .into_iter()
                .enumerate()
                .map(|(index, argument)| RuntimeCapabilityArgumentBindingV3 {
                    argument_ordinal: u16::try_from(index).unwrap_or(u16::MAX),
                    physical_name: argument.physical_name,
                    value_type: argument.value_type,
                    required: argument.required,
                })
                .collect(),
            argument_topology_ambiguous: source.argument_topology_ambiguous,
        });
    }
    Ok((descriptors, bindings))
}

fn runtime_role_values(
    candidates: &[StructuralCandidateObservationV3],
    source_bindings: &[CanonicalStructuralSourceBindingV3],
) -> Result<Box<[RuntimeRoleValueBindingV3]>, RuntimeContextErrorV3> {
    let mut values = Vec::with_capacity(source_bindings.len());
    for binding in source_bindings {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.source_role_id == binding.source_role_id)
            .ok_or(RuntimeContextErrorV3::Structural)?;
        let mut typed = candidate
            .normalized_values
            .iter()
            .map(|value| typed_runtime_value(value, candidate.features.value_type))
            .collect::<Option<Vec<_>>>()
            .ok_or(RuntimeContextErrorV3::Structural)?;
        typed.sort();
        typed.dedup();
        values.push(RuntimeRoleValueBindingV3 {
            role_id: binding.canonical_role_id,
            value_sha256: candidate.value_sha256.clone(),
            values: typed.into_boxed_slice(),
        });
    }
    values.sort_by_key(RuntimeRoleValueBindingV3::role_id);
    Ok(values.into_boxed_slice())
}

fn typed_runtime_value(
    value: &str,
    value_type: BindingValueTypeV1,
) -> Option<BoundProtocolValueV3> {
    match value_type {
        BindingValueTypeV1::String => Some(BoundProtocolValueV3::String(value.to_owned())),
        BindingValueTypeV1::Identifier => Some(BoundProtocolValueV3::Identifier(value.to_owned())),
        BindingValueTypeV1::Integer => value.parse::<u64>().ok().map(BoundProtocolValueV3::Integer),
        BindingValueTypeV1::Boolean => value
            .parse::<bool>()
            .ok()
            .map(BoundProtocolValueV3::Boolean),
    }
}

fn runtime_structural_context(
    call_shape_count: usize,
    output_count: usize,
    active_len: usize,
    capabilities: &[CapabilitySource<'_>],
) -> Result<StructuralContextV3, StructuralExtractionErrorV3> {
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
        count_band(active_len),
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
    .map_err(|_| StructuralExtractionErrorV3::Serialization)?;
    Ok(StructuralContextV3 {
        call_shape_count: u16::try_from(call_shape_count).unwrap_or(u16::MAX),
        capability_count: u16::try_from(capabilities.len()).unwrap_or(u16::MAX),
        completion_state,
        temporal_relation_count: u16::try_from(active_len.saturating_sub(1)).unwrap_or(u16::MAX),
        cardinality_relation_count: 5,
        topology_neighborhood_root_sha256,
    })
}

const fn count_band(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        1_usize << (usize::BITS - 1 - value.leading_zeros())
    }
}

fn valid_capability_symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':')
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
