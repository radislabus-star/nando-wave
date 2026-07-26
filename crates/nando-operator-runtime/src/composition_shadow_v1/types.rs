use std::collections::BTreeMap;

pub const COMPOSITION_SHADOW_V1_MAX_NODES: usize = 32;
pub const COMPOSITION_SHADOW_V1_MAX_EDGES: usize = 64;
pub const COMPOSITION_SHADOW_V1_MAX_DEPTH: usize = 16;
pub const COMPOSITION_SHADOW_V1_MAX_FUEL: u64 = 4_096;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionBundleIdV1([u8; 32]);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionNodeIdV1(u16);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompositionPortIdV1(u16);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompositionValueTypeV1 {
    Bytes,
    Utf8,
    Signed64,
    Boolean,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionValueV1 {
    Bytes(Box<[u8]>),
    Utf8(String),
    Signed64(i64),
    Boolean(bool),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CompositionPortV1 {
    id: CompositionPortIdV1,
    value_type: CompositionValueTypeV1,
}

pub type CompositionCapabilityExecutorV1 =
    fn(&[CompositionValueV1]) -> Result<Vec<CompositionValueV1>, CompositionCapabilityFailureV1>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionCapabilityFailureV1 {
    Abstain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionCapabilityDefinitionErrorV1 {
    InvalidBundleId,
    DuplicateInputPort,
    DuplicateOutputPort,
    EmptyOutputPorts,
    ZeroFuelCost,
}

#[derive(Clone, Debug)]
pub struct CompositionCapabilityV1 {
    bundle_id: CompositionBundleIdV1,
    input_ports: Box<[CompositionPortV1]>,
    output_ports: Box<[CompositionPortV1]>,
    fuel_cost: u64,
    pub(super) executor: CompositionCapabilityExecutorV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionResolverErrorV1 {
    DuplicateBundleId,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryCompositionResolverV1 {
    allowed: BTreeMap<CompositionBundleIdV1, CompositionCapabilityV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionNodeV1 {
    pub(super) id: CompositionNodeIdV1,
    pub(super) bundle_id: CompositionBundleIdV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionEdgeV1 {
    pub(super) producer_node: CompositionNodeIdV1,
    pub(super) producer_port: CompositionPortIdV1,
    pub(super) consumer_node: CompositionNodeIdV1,
    pub(super) consumer_port: CompositionPortIdV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionInputBindingV1 {
    pub(super) port: CompositionPortV1,
    pub(super) consumer_node: CompositionNodeIdV1,
    pub(super) consumer_port: CompositionPortIdV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionOutputBindingV1 {
    pub(super) port: CompositionPortV1,
    pub(super) producer_node: CompositionNodeIdV1,
    pub(super) producer_port: CompositionPortIdV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionDagV1 {
    pub(super) nodes: Box<[CompositionNodeV1]>,
    pub(super) edges: Box<[CompositionEdgeV1]>,
    pub(super) inputs: Box<[CompositionInputBindingV1]>,
    pub(super) outputs: Box<[CompositionOutputBindingV1]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionShadowLimitsErrorV1 {
    ZeroLimit,
    ExceedsHardLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionShadowLimitsV1 {
    max_nodes: usize,
    max_edges: usize,
    max_depth: usize,
    max_fuel: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionShadowInputV1 {
    pub(super) port_id: CompositionPortIdV1,
    pub(super) value: CompositionValueV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionShadowOutputV1 {
    pub(super) port_id: CompositionPortIdV1,
    pub(super) value: CompositionValueV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompositionShadowStepV1 {
    pub(super) node_id: CompositionNodeIdV1,
    pub(super) bundle_id: CompositionBundleIdV1,
    pub(super) fuel_cost: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionShadowTraceV1 {
    pub(super) steps: Box<[CompositionShadowStepV1]>,
    pub(super) outputs: Box<[CompositionShadowOutputV1]>,
    pub(super) fuel_used: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionShadowVerdictV1 {
    Complete,
    AbstainInvalidGraph,
    AbstainInvalidInput,
    AbstainCycle,
    AbstainMissingCapability,
    AbstainTypeMismatch,
    AbstainBudgetExhausted,
    AbstainCapability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionShadowExecutionV1 {
    pub(super) verdict: CompositionShadowVerdictV1,
    pub(super) trace: Option<CompositionShadowTraceV1>,
}

impl CompositionBundleIdV1 {
    #[must_use]
    pub const fn from_sha256(sha256: [u8; 32]) -> Self {
        Self(sha256)
    }

    #[must_use]
    pub const fn as_sha256(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn is_valid(self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

impl CompositionNodeIdV1 {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl CompositionPortIdV1 {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl CompositionValueV1 {
    #[must_use]
    pub const fn value_type(&self) -> CompositionValueTypeV1 {
        match self {
            Self::Bytes(_) => CompositionValueTypeV1::Bytes,
            Self::Utf8(_) => CompositionValueTypeV1::Utf8,
            Self::Signed64(_) => CompositionValueTypeV1::Signed64,
            Self::Boolean(_) => CompositionValueTypeV1::Boolean,
        }
    }
}

impl CompositionPortV1 {
    #[must_use]
    pub const fn new(id: CompositionPortIdV1, value_type: CompositionValueTypeV1) -> Self {
        Self { id, value_type }
    }

    #[must_use]
    pub const fn id(self) -> CompositionPortIdV1 {
        self.id
    }

    #[must_use]
    pub const fn value_type(self) -> CompositionValueTypeV1 {
        self.value_type
    }
}

impl CompositionCapabilityV1 {
    pub fn new(
        bundle_id: CompositionBundleIdV1,
        mut input_ports: Vec<CompositionPortV1>,
        mut output_ports: Vec<CompositionPortV1>,
        fuel_cost: u64,
        executor: CompositionCapabilityExecutorV1,
    ) -> Result<Self, CompositionCapabilityDefinitionErrorV1> {
        if !bundle_id.is_valid() {
            return Err(CompositionCapabilityDefinitionErrorV1::InvalidBundleId);
        }
        if output_ports.is_empty() {
            return Err(CompositionCapabilityDefinitionErrorV1::EmptyOutputPorts);
        }
        if fuel_cost == 0 {
            return Err(CompositionCapabilityDefinitionErrorV1::ZeroFuelCost);
        }
        input_ports.sort_unstable();
        output_ports.sort_unstable();
        if duplicate_port_id(&input_ports) {
            return Err(CompositionCapabilityDefinitionErrorV1::DuplicateInputPort);
        }
        if duplicate_port_id(&output_ports) {
            return Err(CompositionCapabilityDefinitionErrorV1::DuplicateOutputPort);
        }
        Ok(Self {
            bundle_id,
            input_ports: input_ports.into_boxed_slice(),
            output_ports: output_ports.into_boxed_slice(),
            fuel_cost,
            executor,
        })
    }

    #[must_use]
    pub const fn bundle_id(&self) -> CompositionBundleIdV1 {
        self.bundle_id
    }

    #[must_use]
    pub fn input_ports(&self) -> &[CompositionPortV1] {
        &self.input_ports
    }

    #[must_use]
    pub fn output_ports(&self) -> &[CompositionPortV1] {
        &self.output_ports
    }

    #[must_use]
    pub const fn fuel_cost(&self) -> u64 {
        self.fuel_cost
    }
}

impl InMemoryCompositionResolverV1 {
    pub fn new(
        capabilities: Vec<CompositionCapabilityV1>,
    ) -> Result<Self, CompositionResolverErrorV1> {
        let mut allowed = BTreeMap::new();
        for capability in capabilities {
            if allowed.insert(capability.bundle_id(), capability).is_some() {
                return Err(CompositionResolverErrorV1::DuplicateBundleId);
            }
        }
        Ok(Self { allowed })
    }

    #[must_use]
    pub fn resolve(&self, bundle_id: CompositionBundleIdV1) -> Option<&CompositionCapabilityV1> {
        self.allowed.get(&bundle_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.allowed.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }
}

impl CompositionNodeV1 {
    #[must_use]
    pub const fn new(id: CompositionNodeIdV1, bundle_id: CompositionBundleIdV1) -> Self {
        Self { id, bundle_id }
    }

    #[must_use]
    pub const fn id(self) -> CompositionNodeIdV1 {
        self.id
    }

    #[must_use]
    pub const fn bundle_id(self) -> CompositionBundleIdV1 {
        self.bundle_id
    }
}

impl CompositionEdgeV1 {
    #[must_use]
    pub const fn new(
        producer_node: CompositionNodeIdV1,
        producer_port: CompositionPortIdV1,
        consumer_node: CompositionNodeIdV1,
        consumer_port: CompositionPortIdV1,
    ) -> Self {
        Self {
            producer_node,
            producer_port,
            consumer_node,
            consumer_port,
        }
    }
}

impl CompositionInputBindingV1 {
    #[must_use]
    pub const fn new(
        port: CompositionPortV1,
        consumer_node: CompositionNodeIdV1,
        consumer_port: CompositionPortIdV1,
    ) -> Self {
        Self {
            port,
            consumer_node,
            consumer_port,
        }
    }
}

impl CompositionOutputBindingV1 {
    #[must_use]
    pub const fn new(
        port: CompositionPortV1,
        producer_node: CompositionNodeIdV1,
        producer_port: CompositionPortIdV1,
    ) -> Self {
        Self {
            port,
            producer_node,
            producer_port,
        }
    }
}

impl CompositionDagV1 {
    #[must_use]
    pub fn new(
        nodes: Vec<CompositionNodeV1>,
        edges: Vec<CompositionEdgeV1>,
        inputs: Vec<CompositionInputBindingV1>,
        outputs: Vec<CompositionOutputBindingV1>,
    ) -> Self {
        Self {
            nodes: nodes.into_boxed_slice(),
            edges: edges.into_boxed_slice(),
            inputs: inputs.into_boxed_slice(),
            outputs: outputs.into_boxed_slice(),
        }
    }

    #[must_use]
    pub fn nodes(&self) -> &[CompositionNodeV1] {
        &self.nodes
    }

    #[must_use]
    pub fn edges(&self) -> &[CompositionEdgeV1] {
        &self.edges
    }

    #[must_use]
    pub fn inputs(&self) -> &[CompositionInputBindingV1] {
        &self.inputs
    }

    #[must_use]
    pub fn outputs(&self) -> &[CompositionOutputBindingV1] {
        &self.outputs
    }
}

impl CompositionShadowLimitsV1 {
    pub fn new(
        max_nodes: usize,
        max_edges: usize,
        max_depth: usize,
        max_fuel: u64,
    ) -> Result<Self, CompositionShadowLimitsErrorV1> {
        if max_nodes == 0 || max_edges == 0 || max_depth == 0 || max_fuel == 0 {
            return Err(CompositionShadowLimitsErrorV1::ZeroLimit);
        }
        if max_nodes > COMPOSITION_SHADOW_V1_MAX_NODES
            || max_edges > COMPOSITION_SHADOW_V1_MAX_EDGES
            || max_depth > COMPOSITION_SHADOW_V1_MAX_DEPTH
            || max_fuel > COMPOSITION_SHADOW_V1_MAX_FUEL
        {
            return Err(CompositionShadowLimitsErrorV1::ExceedsHardLimit);
        }
        Ok(Self {
            max_nodes,
            max_edges,
            max_depth,
            max_fuel,
        })
    }

    #[must_use]
    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }

    #[must_use]
    pub const fn max_edges(self) -> usize {
        self.max_edges
    }

    #[must_use]
    pub const fn max_depth(self) -> usize {
        self.max_depth
    }

    #[must_use]
    pub const fn max_fuel(self) -> u64 {
        self.max_fuel
    }
}

impl Default for CompositionShadowLimitsV1 {
    fn default() -> Self {
        Self {
            max_nodes: COMPOSITION_SHADOW_V1_MAX_NODES,
            max_edges: COMPOSITION_SHADOW_V1_MAX_EDGES,
            max_depth: COMPOSITION_SHADOW_V1_MAX_DEPTH,
            max_fuel: COMPOSITION_SHADOW_V1_MAX_FUEL,
        }
    }
}

impl CompositionShadowInputV1 {
    #[must_use]
    pub const fn new(port_id: CompositionPortIdV1, value: CompositionValueV1) -> Self {
        Self { port_id, value }
    }

    #[must_use]
    pub const fn port_id(&self) -> CompositionPortIdV1 {
        self.port_id
    }

    #[must_use]
    pub const fn value(&self) -> &CompositionValueV1 {
        &self.value
    }
}

impl CompositionShadowOutputV1 {
    #[must_use]
    pub const fn port_id(&self) -> CompositionPortIdV1 {
        self.port_id
    }

    #[must_use]
    pub const fn value(&self) -> &CompositionValueV1 {
        &self.value
    }
}

impl CompositionShadowStepV1 {
    #[must_use]
    pub const fn node_id(self) -> CompositionNodeIdV1 {
        self.node_id
    }

    #[must_use]
    pub const fn bundle_id(self) -> CompositionBundleIdV1 {
        self.bundle_id
    }

    #[must_use]
    pub const fn fuel_cost(self) -> u64 {
        self.fuel_cost
    }
}

impl CompositionShadowTraceV1 {
    #[must_use]
    pub fn steps(&self) -> &[CompositionShadowStepV1] {
        &self.steps
    }

    #[must_use]
    pub fn outputs(&self) -> &[CompositionShadowOutputV1] {
        &self.outputs
    }

    #[must_use]
    pub const fn fuel_used(&self) -> u64 {
        self.fuel_used
    }
}

impl CompositionShadowExecutionV1 {
    #[must_use]
    pub const fn verdict(&self) -> CompositionShadowVerdictV1 {
        self.verdict
    }

    #[must_use]
    pub const fn trace(&self) -> Option<&CompositionShadowTraceV1> {
        self.trace.as_ref()
    }

    #[must_use]
    pub fn outputs(&self) -> Option<&[CompositionShadowOutputV1]> {
        self.trace.as_ref().map(CompositionShadowTraceV1::outputs)
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn duplicate_port_id(ports: &[CompositionPortV1]) -> bool {
    ports.windows(2).any(|pair| pair[0].id() == pair[1].id())
}
