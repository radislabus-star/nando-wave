use std::collections::{BTreeMap, BTreeSet};

use super::types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValueSourceV1 {
    External(CompositionPortIdV1),
    Node {
        node_id: CompositionNodeIdV1,
        port_id: CompositionPortIdV1,
    },
}

struct ValidatedCompositionV1 {
    order: Vec<CompositionNodeIdV1>,
    bundles: BTreeMap<CompositionNodeIdV1, CompositionBundleIdV1>,
    input_sources: BTreeMap<(CompositionNodeIdV1, CompositionPortIdV1), ValueSourceV1>,
    output_sources: BTreeMap<CompositionPortIdV1, (CompositionNodeIdV1, CompositionPortIdV1)>,
    input_types: BTreeMap<CompositionPortIdV1, CompositionValueTypeV1>,
}

#[must_use]
pub fn execute_composition_shadow_v1(
    dag: &CompositionDagV1,
    inputs: &[CompositionShadowInputV1],
    resolver: &InMemoryCompositionResolverV1,
    limits: CompositionShadowLimitsV1,
) -> CompositionShadowExecutionV1 {
    let validated = match validate_composition(dag, resolver, limits) {
        Ok(validated) => validated,
        Err(verdict) => return abstain(verdict),
    };
    let external_values = match validate_inputs(inputs, &validated.input_types) {
        Ok(values) => values,
        Err(verdict) => return abstain(verdict),
    };

    // Intermediate values stay private until every node and final binding succeeds.
    let mut produced = BTreeMap::new();
    let mut steps = Vec::with_capacity(validated.order.len());
    let mut fuel_used = 0_u64;

    for node_id in &validated.order {
        let Some(bundle_id) = validated.bundles.get(node_id).copied() else {
            return abstain(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        let Some(capability) = resolver.resolve(bundle_id) else {
            return abstain(CompositionShadowVerdictV1::AbstainMissingCapability);
        };
        let Some(next_fuel) = fuel_used.checked_add(capability.fuel_cost()) else {
            return abstain(CompositionShadowVerdictV1::AbstainBudgetExhausted);
        };
        if next_fuel > limits.max_fuel() {
            return abstain(CompositionShadowVerdictV1::AbstainBudgetExhausted);
        }

        let mut node_inputs = Vec::with_capacity(capability.input_ports().len());
        for port in capability.input_ports() {
            let endpoint = (*node_id, port.id());
            let Some(source) = validated.input_sources.get(&endpoint) else {
                return abstain(CompositionShadowVerdictV1::AbstainInvalidGraph);
            };
            let value = match source {
                ValueSourceV1::External(port_id) => external_values.get(port_id),
                ValueSourceV1::Node { node_id, port_id } => produced.get(&(*node_id, *port_id)),
            };
            let Some(value) = value else {
                return abstain(CompositionShadowVerdictV1::AbstainInvalidGraph);
            };
            if value.value_type() != port.value_type() {
                return abstain(CompositionShadowVerdictV1::AbstainTypeMismatch);
            }
            node_inputs.push(value.clone());
        }

        let node_outputs = match (capability.executor)(&node_inputs) {
            Ok(outputs) => outputs,
            Err(CompositionCapabilityFailureV1::Abstain) => {
                return abstain(CompositionShadowVerdictV1::AbstainCapability);
            }
        };
        if node_outputs.len() != capability.output_ports().len() {
            return abstain(CompositionShadowVerdictV1::AbstainCapability);
        }
        for (port, value) in capability.output_ports().iter().zip(node_outputs) {
            if value.value_type() != port.value_type() {
                return abstain(CompositionShadowVerdictV1::AbstainTypeMismatch);
            }
            produced.insert((*node_id, port.id()), value);
        }
        fuel_used = next_fuel;
        steps.push(CompositionShadowStepV1 {
            node_id: *node_id,
            bundle_id,
            fuel_cost: capability.fuel_cost(),
        });
    }

    let mut outputs = Vec::with_capacity(validated.output_sources.len());
    for (port_id, source) in validated.output_sources {
        let Some(value) = produced.get(&source).cloned() else {
            return abstain(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        outputs.push(CompositionShadowOutputV1 { port_id, value });
    }

    CompositionShadowExecutionV1 {
        verdict: CompositionShadowVerdictV1::Complete,
        trace: Some(CompositionShadowTraceV1 {
            steps: steps.into_boxed_slice(),
            outputs: outputs.into_boxed_slice(),
            fuel_used,
        }),
    }
}

fn validate_composition(
    dag: &CompositionDagV1,
    resolver: &InMemoryCompositionResolverV1,
    limits: CompositionShadowLimitsV1,
) -> Result<ValidatedCompositionV1, CompositionShadowVerdictV1> {
    if dag.nodes.is_empty() || dag.outputs.is_empty() {
        return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
    }
    if dag.nodes.len() > limits.max_nodes() || dag.edges.len() > limits.max_edges() {
        return Err(CompositionShadowVerdictV1::AbstainBudgetExhausted);
    }

    let mut bundles = BTreeMap::new();
    for node in &dag.nodes {
        if bundles.insert(node.id, node.bundle_id).is_some() {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
    }
    for bundle_id in bundles.values() {
        if resolver.resolve(*bundle_id).is_none() {
            return Err(CompositionShadowVerdictV1::AbstainMissingCapability);
        }
    }

    let mut input_sources = BTreeMap::new();
    let mut seen_edges = BTreeSet::new();
    let mut successors: BTreeMap<CompositionNodeIdV1, BTreeSet<CompositionNodeIdV1>> = bundles
        .keys()
        .map(|node_id| (*node_id, BTreeSet::new()))
        .collect();
    let mut indegrees: BTreeMap<CompositionNodeIdV1, usize> =
        bundles.keys().map(|node_id| (*node_id, 0)).collect();

    for edge in &dag.edges {
        let edge_key = (
            edge.producer_node,
            edge.producer_port,
            edge.consumer_node,
            edge.consumer_port,
        );
        if !seen_edges.insert(edge_key) {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
        let producer = capability_for_node(edge.producer_node, &bundles, resolver)?;
        let consumer = capability_for_node(edge.consumer_node, &bundles, resolver)?;
        let Some(producer_type) = port_type(producer.output_ports(), edge.producer_port) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        let Some(consumer_type) = port_type(consumer.input_ports(), edge.consumer_port) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        if producer_type != consumer_type {
            return Err(CompositionShadowVerdictV1::AbstainTypeMismatch);
        }
        if input_sources
            .insert(
                (edge.consumer_node, edge.consumer_port),
                ValueSourceV1::Node {
                    node_id: edge.producer_node,
                    port_id: edge.producer_port,
                },
            )
            .is_some()
        {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
        let Some(node_successors) = successors.get_mut(&edge.producer_node) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        if node_successors.insert(edge.consumer_node) {
            let Some(indegree) = indegrees.get_mut(&edge.consumer_node) else {
                return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
            };
            *indegree += 1;
        }
    }

    let mut input_types = BTreeMap::new();
    for binding in &dag.inputs {
        let consumer = capability_for_node(binding.consumer_node, &bundles, resolver)?;
        let Some(consumer_type) = port_type(consumer.input_ports(), binding.consumer_port) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        if binding.port.value_type() != consumer_type {
            return Err(CompositionShadowVerdictV1::AbstainTypeMismatch);
        }
        if let Some(existing_type) = input_types.get(&binding.port.id()) {
            if *existing_type != binding.port.value_type() {
                return Err(CompositionShadowVerdictV1::AbstainTypeMismatch);
            }
        } else {
            input_types.insert(binding.port.id(), binding.port.value_type());
        }
        if input_sources
            .insert(
                (binding.consumer_node, binding.consumer_port),
                ValueSourceV1::External(binding.port.id()),
            )
            .is_some()
        {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
    }

    for (node_id, bundle_id) in &bundles {
        let Some(capability) = resolver.resolve(*bundle_id) else {
            return Err(CompositionShadowVerdictV1::AbstainMissingCapability);
        };
        if capability
            .input_ports()
            .iter()
            .any(|port| !input_sources.contains_key(&(*node_id, port.id())))
        {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
    }

    let mut output_sources = BTreeMap::new();
    for binding in &dag.outputs {
        let producer = capability_for_node(binding.producer_node, &bundles, resolver)?;
        let Some(producer_type) = port_type(producer.output_ports(), binding.producer_port) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        if binding.port.value_type() != producer_type {
            return Err(CompositionShadowVerdictV1::AbstainTypeMismatch);
        }
        if output_sources
            .insert(
                binding.port.id(),
                (binding.producer_node, binding.producer_port),
            )
            .is_some()
        {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        }
    }

    let order = deterministic_topological_order(&successors, &indegrees, limits.max_depth())?;
    Ok(ValidatedCompositionV1 {
        order,
        bundles,
        input_sources,
        output_sources,
        input_types,
    })
}

fn deterministic_topological_order(
    successors: &BTreeMap<CompositionNodeIdV1, BTreeSet<CompositionNodeIdV1>>,
    indegrees: &BTreeMap<CompositionNodeIdV1, usize>,
    max_depth: usize,
) -> Result<Vec<CompositionNodeIdV1>, CompositionShadowVerdictV1> {
    let mut remaining_indegrees = indegrees.clone();
    let mut ready: BTreeSet<_> = remaining_indegrees
        .iter()
        .filter_map(|(node_id, indegree)| (*indegree == 0).then_some(*node_id))
        .collect();
    let mut depths: BTreeMap<_, _> = indegrees
        .keys()
        .map(|node_id| (*node_id, 1_usize))
        .collect();
    let mut order = Vec::with_capacity(indegrees.len());

    while let Some(node_id) = ready.iter().next().copied() {
        ready.remove(&node_id);
        order.push(node_id);
        let node_depth = depths.get(&node_id).copied().unwrap_or(1);
        if node_depth > max_depth {
            return Err(CompositionShadowVerdictV1::AbstainBudgetExhausted);
        }
        let Some(node_successors) = successors.get(&node_id) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
        };
        for successor in node_successors {
            let successor_depth = depths.entry(*successor).or_insert(1);
            *successor_depth = (*successor_depth).max(node_depth.saturating_add(1));
            let Some(indegree) = remaining_indegrees.get_mut(successor) else {
                return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
            };
            let Some(next_indegree) = indegree.checked_sub(1) else {
                return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
            };
            *indegree = next_indegree;
            if next_indegree == 0 {
                ready.insert(*successor);
            }
        }
    }

    if order.len() != indegrees.len() {
        return Err(CompositionShadowVerdictV1::AbstainCycle);
    }
    Ok(order)
}

fn validate_inputs(
    inputs: &[CompositionShadowInputV1],
    input_types: &BTreeMap<CompositionPortIdV1, CompositionValueTypeV1>,
) -> Result<BTreeMap<CompositionPortIdV1, CompositionValueV1>, CompositionShadowVerdictV1> {
    if inputs.len() != input_types.len() {
        return Err(CompositionShadowVerdictV1::AbstainInvalidInput);
    }
    let mut values = BTreeMap::new();
    for input in inputs {
        let Some(expected_type) = input_types.get(&input.port_id) else {
            return Err(CompositionShadowVerdictV1::AbstainInvalidInput);
        };
        if input.value.value_type() != *expected_type {
            return Err(CompositionShadowVerdictV1::AbstainTypeMismatch);
        }
        if values.insert(input.port_id, input.value.clone()).is_some() {
            return Err(CompositionShadowVerdictV1::AbstainInvalidInput);
        }
    }
    Ok(values)
}

fn capability_for_node<'a>(
    node_id: CompositionNodeIdV1,
    bundles: &BTreeMap<CompositionNodeIdV1, CompositionBundleIdV1>,
    resolver: &'a InMemoryCompositionResolverV1,
) -> Result<&'a CompositionCapabilityV1, CompositionShadowVerdictV1> {
    let Some(bundle_id) = bundles.get(&node_id) else {
        return Err(CompositionShadowVerdictV1::AbstainInvalidGraph);
    };
    resolver
        .resolve(*bundle_id)
        .ok_or(CompositionShadowVerdictV1::AbstainMissingCapability)
}

fn port_type(
    ports: &[CompositionPortV1],
    port_id: CompositionPortIdV1,
) -> Option<CompositionValueTypeV1> {
    ports
        .iter()
        .find(|port| port.id() == port_id)
        .map(|port| port.value_type())
}

const fn abstain(verdict: CompositionShadowVerdictV1) -> CompositionShadowExecutionV1 {
    CompositionShadowExecutionV1 {
        verdict,
        trace: None,
    }
}
