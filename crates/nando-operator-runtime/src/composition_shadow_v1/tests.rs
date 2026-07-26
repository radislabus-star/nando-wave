use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const VALUE_PORT: CompositionPortIdV1 = CompositionPortIdV1::new(0);
const GRAPH_INPUT: CompositionPortIdV1 = CompositionPortIdV1::new(10);
const GRAPH_OUTPUT: CompositionPortIdV1 = CompositionPortIdV1::new(20);

static COMPLETED_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);

fn bundle(byte: u8) -> CompositionBundleIdV1 {
    CompositionBundleIdV1::from_sha256([byte; 32])
}

fn node(value: u16) -> CompositionNodeIdV1 {
    CompositionNodeIdV1::new(value)
}

fn port(id: CompositionPortIdV1, value_type: CompositionValueTypeV1) -> CompositionPortV1 {
    CompositionPortV1::new(id, value_type)
}

fn unary_capability(
    bundle_id: CompositionBundleIdV1,
    input_type: CompositionValueTypeV1,
    output_type: CompositionValueTypeV1,
    fuel_cost: u64,
    executor: CompositionCapabilityExecutorV1,
) -> CompositionCapabilityV1 {
    let result = CompositionCapabilityV1::new(
        bundle_id,
        vec![port(VALUE_PORT, input_type)],
        vec![port(VALUE_PORT, output_type)],
        fuel_cost,
        executor,
    );
    let Ok(capability) = result else {
        panic!("test capability must be valid");
    };
    capability
}

fn resolver(capabilities: Vec<CompositionCapabilityV1>) -> InMemoryCompositionResolverV1 {
    let result = InMemoryCompositionResolverV1::new(capabilities);
    let Ok(resolver) = result else {
        panic!("test resolver must be valid");
    };
    resolver
}

fn limits(max_fuel: u64) -> CompositionShadowLimitsV1 {
    let result = CompositionShadowLimitsV1::new(8, 8, 8, max_fuel);
    let Ok(limits) = result else {
        panic!("test limits must be valid");
    };
    limits
}

fn append_a(
    inputs: &[CompositionValueV1],
) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1> {
    let [CompositionValueV1::Utf8(value)] = inputs else {
        return Err(CompositionCapabilityFailureV1::Abstain);
    };
    Ok(vec![CompositionValueV1::Utf8(format!("{value}a"))])
}

fn append_b(
    inputs: &[CompositionValueV1],
) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1> {
    let [CompositionValueV1::Utf8(value)] = inputs else {
        return Err(CompositionCapabilityFailureV1::Abstain);
    };
    Ok(vec![CompositionValueV1::Utf8(format!("{value}b"))])
}

fn identity(
    inputs: &[CompositionValueV1],
) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1> {
    let [value] = inputs else {
        return Err(CompositionCapabilityFailureV1::Abstain);
    };
    Ok(vec![value.clone()])
}

fn counted_identity(
    inputs: &[CompositionValueV1],
) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1> {
    COMPLETED_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
    identity(inputs)
}

fn fail(
    _: &[CompositionValueV1],
) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1> {
    Err(CompositionCapabilityFailureV1::Abstain)
}

fn chain_dag(first: CompositionBundleIdV1, second: CompositionBundleIdV1) -> CompositionDagV1 {
    CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(node(2), second),
            CompositionNodeV1::new(node(1), first),
        ],
        vec![CompositionEdgeV1::new(
            node(1),
            VALUE_PORT,
            node(2),
            VALUE_PORT,
        )],
        vec![CompositionInputBindingV1::new(
            port(GRAPH_INPUT, CompositionValueTypeV1::Utf8),
            node(1),
            VALUE_PORT,
        )],
        vec![CompositionOutputBindingV1::new(
            port(GRAPH_OUTPUT, CompositionValueTypeV1::Utf8),
            node(2),
            VALUE_PORT,
        )],
    )
}

fn text_input(value: &str) -> CompositionShadowInputV1 {
    CompositionShadowInputV1::new(GRAPH_INPUT, CompositionValueV1::Utf8(value.to_owned()))
}

#[test]
fn valid_chain_emits_complete_atomic_trace() {
    let first = bundle(1);
    let second = bundle(2);
    let resolver = resolver(vec![
        unary_capability(
            first,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            2,
            append_a,
        ),
        unary_capability(
            second,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            3,
            append_b,
        ),
    ]);

    let execution = execute_composition_shadow_v1(
        &chain_dag(first, second),
        &[text_input("x")],
        &resolver,
        limits(5),
    );

    assert_eq!(execution.verdict(), CompositionShadowVerdictV1::Complete);
    let Some(trace) = execution.trace() else {
        panic!("complete execution must expose its trace");
    };
    assert_eq!(trace.fuel_used(), 5);
    assert_eq!(
        trace
            .steps()
            .iter()
            .map(|step| step.node_id().get())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(
        trace.outputs(),
        &[CompositionShadowOutputV1 {
            port_id: GRAPH_OUTPUT,
            value: CompositionValueV1::Utf8("xab".to_owned()),
        }]
    );
    assert!(!execution.execution_authority());
}

#[test]
fn cycle_abstains_without_trace() {
    let first = bundle(3);
    let second = bundle(4);
    let resolver = resolver(vec![
        unary_capability(
            first,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            identity,
        ),
        unary_capability(
            second,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            identity,
        ),
    ]);
    let dag = CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(node(1), first),
            CompositionNodeV1::new(node(2), second),
        ],
        vec![
            CompositionEdgeV1::new(node(1), VALUE_PORT, node(2), VALUE_PORT),
            CompositionEdgeV1::new(node(2), VALUE_PORT, node(1), VALUE_PORT),
        ],
        vec![],
        vec![CompositionOutputBindingV1::new(
            port(GRAPH_OUTPUT, CompositionValueTypeV1::Utf8),
            node(2),
            VALUE_PORT,
        )],
    );

    let execution =
        execute_composition_shadow_v1(&dag, &[], &resolver, CompositionShadowLimitsV1::default());

    assert_eq!(
        execution.verdict(),
        CompositionShadowVerdictV1::AbstainCycle
    );
    assert!(execution.trace().is_none());
}

#[test]
fn missing_capability_abstains() {
    let missing = bundle(5);
    let dag = CompositionDagV1::new(
        vec![CompositionNodeV1::new(node(1), missing)],
        vec![],
        vec![CompositionInputBindingV1::new(
            port(GRAPH_INPUT, CompositionValueTypeV1::Utf8),
            node(1),
            VALUE_PORT,
        )],
        vec![CompositionOutputBindingV1::new(
            port(GRAPH_OUTPUT, CompositionValueTypeV1::Utf8),
            node(1),
            VALUE_PORT,
        )],
    );

    let execution = execute_composition_shadow_v1(
        &dag,
        &[text_input("x")],
        &InMemoryCompositionResolverV1::default(),
        CompositionShadowLimitsV1::default(),
    );

    assert_eq!(
        execution.verdict(),
        CompositionShadowVerdictV1::AbstainMissingCapability
    );
    assert!(execution.trace().is_none());
}

#[test]
fn zero_bundle_identity_is_rejected_before_resolution() {
    let capability = CompositionCapabilityV1::new(
        CompositionBundleIdV1::from_sha256([0; 32]),
        vec![port(VALUE_PORT, CompositionValueTypeV1::Utf8)],
        vec![port(VALUE_PORT, CompositionValueTypeV1::Utf8)],
        1,
        identity,
    );

    assert_eq!(
        capability.map(|_| ()),
        Err(CompositionCapabilityDefinitionErrorV1::InvalidBundleId)
    );
}

#[test]
fn edge_type_mismatch_abstains_before_execution() {
    let first = bundle(6);
    let second = bundle(7);
    let resolver = resolver(vec![
        unary_capability(
            first,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            identity,
        ),
        unary_capability(
            second,
            CompositionValueTypeV1::Signed64,
            CompositionValueTypeV1::Signed64,
            1,
            identity,
        ),
    ]);
    let dag = CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(node(1), first),
            CompositionNodeV1::new(node(2), second),
        ],
        vec![CompositionEdgeV1::new(
            node(1),
            VALUE_PORT,
            node(2),
            VALUE_PORT,
        )],
        vec![CompositionInputBindingV1::new(
            port(GRAPH_INPUT, CompositionValueTypeV1::Utf8),
            node(1),
            VALUE_PORT,
        )],
        vec![CompositionOutputBindingV1::new(
            port(GRAPH_OUTPUT, CompositionValueTypeV1::Signed64),
            node(2),
            VALUE_PORT,
        )],
    );

    let execution = execute_composition_shadow_v1(&dag, &[text_input("x")], &resolver, limits(2));

    assert_eq!(
        execution.verdict(),
        CompositionShadowVerdictV1::AbstainTypeMismatch
    );
    assert!(execution.trace().is_none());
}

#[test]
fn fuel_budget_exhaustion_abstains_atomically() {
    let first = bundle(8);
    let second = bundle(9);
    let resolver = resolver(vec![
        unary_capability(
            first,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            identity,
        ),
        unary_capability(
            second,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            identity,
        ),
    ]);

    let execution = execute_composition_shadow_v1(
        &chain_dag(first, second),
        &[text_input("x")],
        &resolver,
        limits(1),
    );

    assert_eq!(
        execution.verdict(),
        CompositionShadowVerdictV1::AbstainBudgetExhausted
    );
    assert!(execution.trace().is_none());
    assert!(execution.outputs().is_none());
}

#[test]
fn partial_result_is_never_emitted() {
    COMPLETED_EXECUTIONS.store(0, Ordering::SeqCst);
    let first = bundle(10);
    let second = bundle(11);
    let resolver = resolver(vec![
        unary_capability(
            first,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            counted_identity,
        ),
        unary_capability(
            second,
            CompositionValueTypeV1::Utf8,
            CompositionValueTypeV1::Utf8,
            1,
            fail,
        ),
    ]);

    let execution = execute_composition_shadow_v1(
        &chain_dag(first, second),
        &[text_input("private partial")],
        &resolver,
        limits(2),
    );

    assert_eq!(COMPLETED_EXECUTIONS.load(Ordering::SeqCst), 1);
    assert_eq!(
        execution.verdict(),
        CompositionShadowVerdictV1::AbstainCapability
    );
    assert!(execution.trace().is_none());
    assert!(execution.outputs().is_none());
}

#[test]
fn topological_steps_and_outputs_are_deterministic() {
    let identity_bundle = bundle(12);
    let resolver = resolver(vec![unary_capability(
        identity_bundle,
        CompositionValueTypeV1::Utf8,
        CompositionValueTypeV1::Utf8,
        1,
        identity,
    )]);
    let low_node = node(2);
    let high_node = node(9);
    let low_input = CompositionPortIdV1::new(2);
    let high_input = CompositionPortIdV1::new(9);
    let low_output = CompositionPortIdV1::new(20);
    let high_output = CompositionPortIdV1::new(90);

    let first = CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(high_node, identity_bundle),
            CompositionNodeV1::new(low_node, identity_bundle),
        ],
        vec![],
        vec![
            CompositionInputBindingV1::new(
                port(high_input, CompositionValueTypeV1::Utf8),
                high_node,
                VALUE_PORT,
            ),
            CompositionInputBindingV1::new(
                port(low_input, CompositionValueTypeV1::Utf8),
                low_node,
                VALUE_PORT,
            ),
        ],
        vec![
            CompositionOutputBindingV1::new(
                port(high_output, CompositionValueTypeV1::Utf8),
                high_node,
                VALUE_PORT,
            ),
            CompositionOutputBindingV1::new(
                port(low_output, CompositionValueTypeV1::Utf8),
                low_node,
                VALUE_PORT,
            ),
        ],
    );
    let second = CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(low_node, identity_bundle),
            CompositionNodeV1::new(high_node, identity_bundle),
        ],
        vec![],
        vec![
            CompositionInputBindingV1::new(
                port(low_input, CompositionValueTypeV1::Utf8),
                low_node,
                VALUE_PORT,
            ),
            CompositionInputBindingV1::new(
                port(high_input, CompositionValueTypeV1::Utf8),
                high_node,
                VALUE_PORT,
            ),
        ],
        vec![
            CompositionOutputBindingV1::new(
                port(low_output, CompositionValueTypeV1::Utf8),
                low_node,
                VALUE_PORT,
            ),
            CompositionOutputBindingV1::new(
                port(high_output, CompositionValueTypeV1::Utf8),
                high_node,
                VALUE_PORT,
            ),
        ],
    );
    let first_inputs = [
        CompositionShadowInputV1::new(high_input, CompositionValueV1::Utf8("high".to_owned())),
        CompositionShadowInputV1::new(low_input, CompositionValueV1::Utf8("low".to_owned())),
    ];
    let second_inputs = [first_inputs[1].clone(), first_inputs[0].clone()];

    let first_execution =
        execute_composition_shadow_v1(&first, &first_inputs, &resolver, limits(2));
    let second_execution =
        execute_composition_shadow_v1(&second, &second_inputs, &resolver, limits(2));

    assert_eq!(first_execution, second_execution);
    let Some(trace) = first_execution.trace() else {
        panic!("deterministic graph must complete");
    };
    assert_eq!(
        trace
            .steps()
            .iter()
            .map(|step| step.node_id().get())
            .collect::<Vec<_>>(),
        vec![2, 9]
    );
    assert_eq!(
        trace
            .outputs()
            .iter()
            .map(|output| output.port_id().get())
            .collect::<Vec<_>>(),
        vec![20, 90]
    );
}

#[test]
fn immutable_external_input_can_fan_out_to_multiple_nodes() {
    let identity_bundle = bundle(13);
    let resolver = resolver(vec![unary_capability(
        identity_bundle,
        CompositionValueTypeV1::Utf8,
        CompositionValueTypeV1::Utf8,
        1,
        identity,
    )]);
    let first_node = node(1);
    let second_node = node(2);
    let first_output = CompositionPortIdV1::new(21);
    let second_output = CompositionPortIdV1::new(22);
    let shared_input = port(GRAPH_INPUT, CompositionValueTypeV1::Utf8);
    let dag = CompositionDagV1::new(
        vec![
            CompositionNodeV1::new(first_node, identity_bundle),
            CompositionNodeV1::new(second_node, identity_bundle),
        ],
        vec![],
        vec![
            CompositionInputBindingV1::new(shared_input, first_node, VALUE_PORT),
            CompositionInputBindingV1::new(shared_input, second_node, VALUE_PORT),
        ],
        vec![
            CompositionOutputBindingV1::new(
                port(first_output, CompositionValueTypeV1::Utf8),
                first_node,
                VALUE_PORT,
            ),
            CompositionOutputBindingV1::new(
                port(second_output, CompositionValueTypeV1::Utf8),
                second_node,
                VALUE_PORT,
            ),
        ],
    );

    let execution =
        execute_composition_shadow_v1(&dag, &[text_input("shared")], &resolver, limits(2));

    assert_eq!(execution.verdict(), CompositionShadowVerdictV1::Complete);
    assert_eq!(
        execution.outputs(),
        Some(
            [
                CompositionShadowOutputV1 {
                    port_id: first_output,
                    value: CompositionValueV1::Utf8("shared".to_owned()),
                },
                CompositionShadowOutputV1 {
                    port_id: second_output,
                    value: CompositionValueV1::Utf8("shared".to_owned()),
                },
            ]
            .as_slice()
        )
    );
}
