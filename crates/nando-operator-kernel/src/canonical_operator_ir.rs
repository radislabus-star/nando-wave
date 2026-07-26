use serde::{Deserialize, Serialize};

use crate::{
    CollectionOutputRenderer, ResponseProgram, canonical_json_bytes, canonical_json_sha256,
    valid_nonzero_sha256,
};

pub const CANONICAL_OPERATOR_IR_V1_SCHEMA: &str = "nando.canonical-operator-ir.v1";
pub const CANONICAL_OPERATOR_IR_V1_MAX_ROLES: usize = 32;
pub const CANONICAL_OPERATOR_IR_V1_MAX_RELATIONS: usize = 256;
pub const CANONICAL_OPERATOR_IR_V1_MAX_TRANSFORMS: usize = 16;
pub const CANONICAL_OPERATOR_IR_V1_MAX_COMPOSITION_EDGES: usize = 64;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalOperatorRoleV1 {
    pub type_class: u8,
    pub cardinality_class: u8,
    pub temporal_position: u8,
    pub constraint_mask: u32,
    pub neighboring_relation_planes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalOperatorRelationV1 {
    pub plane: u8,
    pub source_role: u8,
    pub target_role: u8,
    pub state: i8,
    pub phase_re_bits: u64,
    pub phase_im_bits: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalOperatorTransformV1 {
    pub opcode: u8,
    pub output: u8,
    pub source_a: u8,
    pub source_b: u8,
    pub parameter: u16,
    pub flags: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalOperatorCompositionEdgeV1 {
    pub producer_step: u8,
    pub consumer_step: u8,
}

/// Origin-neutral compiler input. Learning provenance, generation counters,
/// proof receipts, and execution authority are deliberately outside this IR.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalOperatorIrV1 {
    schema: String,
    roles: Vec<CanonicalOperatorRoleV1>,
    relations: Vec<CanonicalOperatorRelationV1>,
    transforms: Vec<CanonicalOperatorTransformV1>,
    composition_edges: Vec<CanonicalOperatorCompositionEdgeV1>,
    renderer: CollectionOutputRenderer,
    actor_template: ResponseProgram,
    verifier_contract_sha256: String,
}

#[derive(Serialize)]
struct CanonicalExecutableOperatorIrV1<'a> {
    schema: &'a str,
    roles: &'a [CanonicalOperatorRoleV1],
    relations: Vec<CanonicalExecutableRelationV1>,
    transforms: &'a [CanonicalOperatorTransformV1],
    composition_edges: &'a [CanonicalOperatorCompositionEdgeV1],
    renderer: &'a CollectionOutputRenderer,
    actor_template: &'a ResponseProgram,
    verifier_contract_sha256: &'a str,
}

#[derive(Clone, Copy, Serialize)]
struct CanonicalExecutableRelationV1 {
    plane: u8,
    source_role: u8,
    target_role: u8,
    state: i8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalOperatorIrErrorV1 {
    Decode,
    NonCanonicalEncoding,
    InvalidSchema,
    InvalidRoleCount,
    InvalidRolePlanes,
    InvalidRelationCount,
    InvalidRelation,
    InvalidPhase,
    InvalidTransformCount,
    InvalidTransform,
    InvalidComposition,
    InvalidActor,
    InvalidVerifierContract,
    CanonicalEncoding,
}

impl CanonicalOperatorIrV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        roles: Vec<CanonicalOperatorRoleV1>,
        relations: Vec<CanonicalOperatorRelationV1>,
        transforms: Vec<CanonicalOperatorTransformV1>,
        composition_edges: Vec<CanonicalOperatorCompositionEdgeV1>,
        renderer: CollectionOutputRenderer,
        actor_template: ResponseProgram,
        verifier_contract_sha256: String,
    ) -> Result<Self, CanonicalOperatorIrErrorV1> {
        let mut value = Self {
            schema: CANONICAL_OPERATOR_IR_V1_SCHEMA.to_owned(),
            roles,
            relations,
            transforms,
            composition_edges,
            renderer,
            actor_template,
            verifier_contract_sha256,
        };
        value.canonicalize();
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), CanonicalOperatorIrErrorV1> {
        if self.schema != CANONICAL_OPERATOR_IR_V1_SCHEMA {
            return Err(CanonicalOperatorIrErrorV1::InvalidSchema);
        }
        if self.roles.is_empty() || self.roles.len() > CANONICAL_OPERATOR_IR_V1_MAX_ROLES {
            return Err(CanonicalOperatorIrErrorV1::InvalidRoleCount);
        }
        if self.roles.iter().any(|role| {
            role.neighboring_relation_planes
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        }) {
            return Err(CanonicalOperatorIrErrorV1::InvalidRolePlanes);
        }
        if self.relations.is_empty()
            || self.relations.len() > CANONICAL_OPERATOR_IR_V1_MAX_RELATIONS
        {
            return Err(CanonicalOperatorIrErrorV1::InvalidRelationCount);
        }
        if self.relations.windows(2).any(|pair| pair[0] >= pair[1])
            || self.relations.iter().any(|relation| {
                relation.source_role == relation.target_role
                    || usize::from(relation.source_role) >= self.roles.len()
                    || usize::from(relation.target_role) >= self.roles.len()
                    || !matches!(relation.state, -1 | 1)
            })
        {
            return Err(CanonicalOperatorIrErrorV1::InvalidRelation);
        }
        if self.relations.iter().any(|relation| {
            !f64::from_bits(relation.phase_re_bits).is_finite()
                || !f64::from_bits(relation.phase_im_bits).is_finite()
        }) {
            return Err(CanonicalOperatorIrErrorV1::InvalidPhase);
        }
        if self.transforms.is_empty()
            || self.transforms.len() > CANONICAL_OPERATOR_IR_V1_MAX_TRANSFORMS
        {
            return Err(CanonicalOperatorIrErrorV1::InvalidTransformCount);
        }
        if self.transforms.iter().any(|transform| {
            usize::from(transform.output) >= self.roles.len()
                || usize::from(transform.source_a) >= self.roles.len()
                || (transform.source_b != u8::MAX
                    && usize::from(transform.source_b) >= self.roles.len())
                || transform.output == transform.source_a
                || transform.output == transform.source_b
        }) {
            return Err(CanonicalOperatorIrErrorV1::InvalidTransform);
        }
        if self.composition_edges.len() > CANONICAL_OPERATOR_IR_V1_MAX_COMPOSITION_EDGES
            || self
                .composition_edges
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.composition_edges.iter().any(|edge| {
                edge.producer_step == edge.consumer_step
                    || usize::from(edge.producer_step) >= self.transforms.len()
                    || usize::from(edge.consumer_step) >= self.transforms.len()
            })
        {
            return Err(CanonicalOperatorIrErrorV1::InvalidComposition);
        }
        self.actor_template
            .validate()
            .map_err(|_| CanonicalOperatorIrErrorV1::InvalidActor)?;
        if !valid_nonzero_sha256(&self.verifier_contract_sha256) {
            return Err(CanonicalOperatorIrErrorV1::InvalidVerifierContract);
        }
        canonical_json_bytes(self)
            .map(|_| ())
            .map_err(|_| CanonicalOperatorIrErrorV1::CanonicalEncoding)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CanonicalOperatorIrErrorV1> {
        let value: Self =
            serde_json::from_slice(bytes).map_err(|_| CanonicalOperatorIrErrorV1::Decode)?;
        value.validate()?;
        if value.canonical_bytes()?.as_slice() != bytes {
            return Err(CanonicalOperatorIrErrorV1::NonCanonicalEncoding);
        }
        Ok(value)
    }

    #[must_use]
    pub fn roles(&self) -> &[CanonicalOperatorRoleV1] {
        &self.roles
    }

    #[must_use]
    pub fn relations(&self) -> &[CanonicalOperatorRelationV1] {
        &self.relations
    }

    #[must_use]
    pub fn transforms(&self) -> &[CanonicalOperatorTransformV1] {
        &self.transforms
    }

    #[must_use]
    pub fn composition_edges(&self) -> &[CanonicalOperatorCompositionEdgeV1] {
        &self.composition_edges
    }

    #[must_use]
    pub const fn renderer(&self) -> &CollectionOutputRenderer {
        &self.renderer
    }

    #[must_use]
    pub const fn actor_template(&self) -> &ResponseProgram {
        &self.actor_template
    }

    #[must_use]
    pub fn verifier_contract_sha256(&self) -> &str {
        &self.verifier_contract_sha256
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonicalOperatorIrErrorV1> {
        self.validate()?;
        canonical_json_bytes(self).map_err(|_| CanonicalOperatorIrErrorV1::CanonicalEncoding)
    }

    /// Identifies the complete precompiler artifact, including routing phase.
    pub fn artifact_sha256(&self) -> Result<String, CanonicalOperatorIrErrorV1> {
        self.validate()?;
        canonical_json_sha256(self).map_err(|_| CanonicalOperatorIrErrorV1::CanonicalEncoding)
    }

    /// Identifies executable semantics independently of learned routing phase.
    ///
    /// Phase remains in the full IR and OperatorPage. It is excluded here so
    /// independently induced routes can converge on one VM program without
    /// pretending their routing evidence was byte-identical.
    pub fn executable_sha256(&self) -> Result<String, CanonicalOperatorIrErrorV1> {
        self.validate()?;
        canonical_json_sha256(&self.executable_view())
            .map_err(|_| CanonicalOperatorIrErrorV1::CanonicalEncoding)
    }

    pub fn executable_bytes(&self) -> Result<Vec<u8>, CanonicalOperatorIrErrorV1> {
        self.validate()?;
        canonical_json_bytes(&self.executable_view())
            .map_err(|_| CanonicalOperatorIrErrorV1::CanonicalEncoding)
    }

    fn executable_view(&self) -> CanonicalExecutableOperatorIrV1<'_> {
        CanonicalExecutableOperatorIrV1 {
            schema: &self.schema,
            roles: &self.roles,
            relations: self
                .relations
                .iter()
                .map(|relation| CanonicalExecutableRelationV1 {
                    plane: relation.plane,
                    source_role: relation.source_role,
                    target_role: relation.target_role,
                    state: relation.state,
                })
                .collect(),
            transforms: &self.transforms,
            composition_edges: &self.composition_edges,
            renderer: &self.renderer,
            actor_template: &self.actor_template,
            verifier_contract_sha256: &self.verifier_contract_sha256,
        }
    }

    fn canonicalize(&mut self) {
        for role in &mut self.roles {
            role.neighboring_relation_planes.sort_unstable();
            role.neighboring_relation_planes.dedup();
        }
        self.relations.sort_unstable();
        self.composition_edges.sort_unstable();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ResponseValueSelector, ValueProjectionFormat};

    fn valid_ir() -> CanonicalOperatorIrV1 {
        CanonicalOperatorIrV1::new(
            vec![
                CanonicalOperatorRoleV1 {
                    type_class: 5,
                    cardinality_class: 1,
                    temporal_position: 0,
                    constraint_mask: 1,
                    neighboring_relation_planes: vec![0],
                },
                CanonicalOperatorRoleV1 {
                    type_class: 1,
                    cardinality_class: 1,
                    temporal_position: 1,
                    constraint_mask: 2,
                    neighboring_relation_planes: vec![0],
                },
                CanonicalOperatorRoleV1 {
                    type_class: 1,
                    cardinality_class: 1,
                    temporal_position: 2,
                    constraint_mask: 4,
                    neighboring_relation_planes: vec![],
                },
            ],
            vec![CanonicalOperatorRelationV1 {
                plane: 0,
                source_role: 0,
                target_role: 1,
                state: 1,
                phase_re_bits: 1.0_f64.to_bits(),
                phase_im_bits: 0.0_f64.to_bits(),
            }],
            vec![CanonicalOperatorTransformV1 {
                opcode: 1,
                output: 2,
                source_a: 1,
                source_b: u8::MAX,
                parameter: 0,
                flags: 0,
            }],
            vec![],
            CollectionOutputRenderer::Direct,
            ResponseProgram::project_selected_value(
                ResponseValueSelector::RequestLastToken,
                ValueProjectionFormat::PlainText,
                "completed",
            ),
            "11".repeat(32),
        )
        .expect("valid IR")
    }

    #[test]
    fn canonical_ir_is_order_independent_and_authority_free() {
        let left = valid_ir();
        let mut right = valid_ir();
        right.roles[0].neighboring_relation_planes = vec![0, 0];
        right.canonicalize();
        assert_eq!(left.canonical_bytes(), right.canonical_bytes());
        let bytes = left.canonical_bytes().expect("canonical bytes");
        let text = std::str::from_utf8(&bytes).expect("JSON");
        for forbidden in ["authority", "support", "future", "generation", "teacher"] {
            assert!(!text.contains(forbidden), "{forbidden} leaked into IR");
        }
        assert_eq!(
            CanonicalOperatorIrV1::from_canonical_bytes(&bytes),
            Ok(left)
        );
    }

    #[test]
    fn executable_identity_ignores_only_routing_phase() {
        let left = valid_ir();
        let mut different_phase = valid_ir();
        different_phase.relations[0].phase_re_bits = 0.5_f64.to_bits();
        assert_eq!(
            left.executable_sha256(),
            different_phase.executable_sha256()
        );
        assert_ne!(left.artifact_sha256(), different_phase.artifact_sha256());
    }

    #[test]
    fn canonical_ir_rejects_nonfinite_phase_and_bad_verifier() {
        let mut bad_phase = valid_ir();
        bad_phase.relations[0].phase_re_bits = f64::NAN.to_bits();
        assert_eq!(
            bad_phase.validate(),
            Err(CanonicalOperatorIrErrorV1::InvalidPhase)
        );

        let mut bad_verifier = valid_ir();
        bad_verifier.verifier_contract_sha256 = "00".repeat(32);
        assert_eq!(
            bad_verifier.validate(),
            Err(CanonicalOperatorIrErrorV1::InvalidVerifierContract)
        );
    }
}
