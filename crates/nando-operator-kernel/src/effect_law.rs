use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{AtomSource, canonical_json_bytes, canonical_json_sha256, valid_nonzero_sha256};

pub const CANONICAL_EFFECT_LAW_SCHEMA_V3: &str = "nando.canonical-effect-law.v3";
pub const EFFECT_LAW_IR_VERSION_V3: u16 = 3;

pub const EFFECT_REL_EQUAL: u16 = 0x1001;
pub const EFFECT_REL_COPY: u16 = 0x1002;
pub const EFFECT_REL_CONSUME: u16 = 0x1003;
pub const EFFECT_REL_REQUIRE: u16 = 0x1004;
pub const EFFECT_REL_CONSTANT: u16 = 0x1005;

pub const EFFECT_OPERATION_CALL_V3: u16 = 0x2001;
pub const EFFECT_OPERATION_PROJECT_V3: u16 = 0x2002;
pub const EFFECT_OPERATION_STATUS_V3: u16 = 0x2003;
pub const EFFECT_OPERATION_PLAN_ADVANCE_V3: u16 = 0x2004;

pub const EFFECT_ATOM_PRECONDITION: u16 = 0x3001;
pub const EFFECT_ATOM_ACTION_RELATION: u16 = 0x3002;
pub const EFFECT_ATOM_POSTCONDITION: u16 = 0x3003;
pub const EFFECT_ATOM_RENDERER: u16 = 0x3004;
pub const EFFECT_ATOM_TEMPORAL: u16 = 0x3005;
pub const EFFECT_ATOM_CARDINALITY: u16 = 0x3006;
pub const EFFECT_ATOM_PHYSICAL_SURFACE: u16 = 0x3007;

pub const EFFECT_VALUE_STRING_V3: u16 = 0x4001;
pub const EFFECT_VALUE_INTEGER_V3: u16 = 0x4002;
pub const EFFECT_VALUE_BOOLEAN_V3: u16 = 0x4003;
pub const EFFECT_VALUE_IDENTIFIER_V3: u16 = 0x4004;
pub const EFFECT_VALUE_COLLECTION_V3: u16 = 0x4005;
pub const EFFECT_VALUE_OPERATION_V3: u16 = 0x4006;

pub const MAX_EFFECT_NODES_V3: usize = 32;
pub const MAX_EFFECT_EDGES_V3: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectSource {
    Request,
    Observation,
    Action,
    Outcome,
    Derived,
}

impl From<AtomSource> for EffectSource {
    fn from(value: AtomSource) -> Self {
        match value {
            AtomSource::Request => Self::Request,
            AtomSource::Observation => Self::Observation,
            AtomSource::Action => Self::Action,
            AtomSource::Outcome => Self::Outcome,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalEffectNodeV3 {
    pub canonical_node: u16,
    pub source: EffectSource,
    pub node_kind_code: u16,
    pub value_type_code: u16,
    pub unique: bool,
    pub operation_code: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalEffectEdgeV3 {
    pub from: u16,
    pub to: u16,
    pub relation_code: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalRelationClauseV3 {
    pub relation_code: u16,
    pub lhs: u16,
    pub rhs: Option<u16>,
    pub argument_ordinal: Option<u16>,
    pub constant_type_code: Option<u16>,
    pub constant_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CanonicalNodeMappingV3 {
    pub physical_node: u16,
    pub canonical_node: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectLawV3 {
    schema: String,
    ir_version: u16,
    dictionary_root_sha256: String,
    quotient_hypothesis_root_sha256: String,
    topology_nodes: Vec<CanonicalEffectNodeV3>,
    topology_edges: Vec<CanonicalEffectEdgeV3>,
    relation_program: Vec<CanonicalRelationClauseV3>,
    effect_invariant_root_sha256: String,
    preserved_frame_root_sha256: String,
    action_equivalence_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EffectLawIdV3(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectLawV3Error {
    InvalidCandidate,
    InvalidCaptureReceipt,
    InvalidParityReceipt,
    InvalidVerifierReceipt,
    InvalidTrustRoot,
    EffectDeltaDisagreement,
    IncompleteEffectDelta,
    InvalidDictionary,
    InsufficientIndependentEvidence,
    NoInvariantQuotient,
    AmbiguousActionEquivalence,
    OverBudget,
    InvalidRestartBundle,
    Serialization,
}

impl fmt::Display for EffectLawV3Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidCandidate => "effect observation candidate is invalid",
            Self::InvalidCaptureReceipt => "capture receipt is missing or unresolved",
            Self::InvalidParityReceipt => "runtime parity receipt is missing or invalid",
            Self::InvalidVerifierReceipt => "independent verifier receipt is invalid",
            Self::InvalidTrustRoot => "effect evidence is not bound to the external trust root",
            Self::EffectDeltaDisagreement => {
                "independently observed effect delta disagrees with the teacher claim"
            }
            Self::IncompleteEffectDelta => "effect delta lacks required proof-bearing relations",
            Self::InvalidDictionary => "effect dictionary is invalid",
            Self::InsufficientIndependentEvidence => {
                "effect quotient lacks multidimensional independent evidence"
            }
            Self::NoInvariantQuotient => "observations do not share one invariant effect law",
            Self::AmbiguousActionEquivalence => {
                "symmetric role mappings produce multiple action classes"
            }
            Self::OverBudget => "effect law v3 exceeds a bounded search limit",
            Self::InvalidRestartBundle => "effect law restart bundle is invalid",
            Self::Serialization => "effect law v3 serialization failed",
        })
    }
}

impl std::error::Error for EffectLawV3Error {}

impl CanonicalEffectLawV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EffectLawV3Error> {
        canonical_json_bytes(self).map_err(|_| EffectLawV3Error::Serialization)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawV3Error> {
        let law: Self =
            serde_json::from_slice(bytes).map_err(|_| EffectLawV3Error::InvalidCandidate)?;
        if law.canonical_bytes()? != bytes
            || law.schema != CANONICAL_EFFECT_LAW_SCHEMA_V3
            || law.ir_version != EFFECT_LAW_IR_VERSION_V3
            || !valid_nonzero_sha256(&law.dictionary_root_sha256)
            || !valid_nonzero_sha256(&law.quotient_hypothesis_root_sha256)
            || !valid_nonzero_sha256(&law.effect_invariant_root_sha256)
            || !valid_nonzero_sha256(&law.preserved_frame_root_sha256)
            || !valid_nonzero_sha256(&law.action_equivalence_root_sha256)
        {
            return Err(EffectLawV3Error::InvalidCandidate);
        }
        Ok(law)
    }

    pub fn effect_law_id(&self) -> Result<EffectLawIdV3, EffectLawV3Error> {
        Ok(EffectLawIdV3(
            canonical_json_sha256(self).map_err(|_| EffectLawV3Error::Serialization)?,
        ))
    }

    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    #[must_use]
    pub fn ir_version(&self) -> u16 {
        self.ir_version
    }

    #[must_use]
    pub fn dictionary_root_sha256(&self) -> &str {
        &self.dictionary_root_sha256
    }

    #[must_use]
    pub fn quotient_hypothesis_root_sha256(&self) -> &str {
        &self.quotient_hypothesis_root_sha256
    }

    #[must_use]
    pub fn topology_nodes(&self) -> &[CanonicalEffectNodeV3] {
        &self.topology_nodes
    }

    #[must_use]
    pub fn topology_edges(&self) -> &[CanonicalEffectEdgeV3] {
        &self.topology_edges
    }

    #[must_use]
    pub fn relation_program(&self) -> &[CanonicalRelationClauseV3] {
        &self.relation_program
    }

    #[must_use]
    pub fn effect_invariant_root_sha256(&self) -> &str {
        &self.effect_invariant_root_sha256
    }

    #[must_use]
    pub fn preserved_frame_root_sha256(&self) -> &str {
        &self.preserved_frame_root_sha256
    }

    #[must_use]
    pub fn action_equivalence_root_sha256(&self) -> &str {
        &self.action_equivalence_root_sha256
    }
}

impl EffectLawIdV3 {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn validate_canonical_effect_law_v3(
    law: &CanonicalEffectLawV3,
) -> Result<(), EffectLawV3Error> {
    if law.topology_nodes.is_empty()
        || law.topology_nodes.len() > MAX_EFFECT_NODES_V3
        || law.topology_edges.len() > MAX_EFFECT_EDGES_V3
        || law.relation_program.len() > MAX_EFFECT_EDGES_V3 + MAX_EFFECT_NODES_V3 * 2
        || law
            .topology_nodes
            .iter()
            .enumerate()
            .any(|(index, node)| usize::from(node.canonical_node) != index)
        || law.topology_edges.windows(2).any(|pair| pair[0] >= pair[1])
        || law
            .relation_program
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    let node_count = law.topology_nodes.len();
    if law.topology_edges.iter().any(|edge| {
        usize::from(edge.from) >= node_count
            || usize::from(edge.to) >= node_count
            || edge.relation_code == 0
    }) || law.relation_program.iter().any(|clause| {
        usize::from(clause.lhs) >= node_count
            || clause.rhs.is_some_and(|rhs| usize::from(rhs) >= node_count)
            || clause.relation_code == 0
            || clause
                .constant_sha256
                .as_ref()
                .is_some_and(|digest| !valid_nonzero_sha256(digest))
            || clause.constant_sha256.is_some() && clause.constant_type_code.is_none()
    }) {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    let expected_action_root = canonical_json_sha256(&(
        &law.relation_program,
        &law.effect_invariant_root_sha256,
        &law.preserved_frame_root_sha256,
    ))
    .map_err(|_| EffectLawV3Error::Serialization)?;
    if expected_action_root != law.action_equivalence_root_sha256 {
        return Err(EffectLawV3Error::InvalidRestartBundle);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_law_roundtrips_without_public_fields() {
        let root = |byte: char| byte.to_string().repeat(64);
        let relation_program = vec![CanonicalRelationClauseV3 {
            relation_code: EFFECT_REL_REQUIRE,
            lhs: 0,
            rhs: None,
            argument_ordinal: Some(0),
            constant_type_code: None,
            constant_sha256: None,
        }];
        let effect_invariant_root_sha256 = root('a');
        let preserved_frame_root_sha256 = root('b');
        let action_equivalence_root_sha256 = canonical_json_sha256(&(
            &relation_program,
            &effect_invariant_root_sha256,
            &preserved_frame_root_sha256,
        ))
        .expect("action root");
        let bytes = canonical_json_bytes(&serde_json::json!({
            "schema": CANONICAL_EFFECT_LAW_SCHEMA_V3,
            "ir_version": EFFECT_LAW_IR_VERSION_V3,
            "dictionary_root_sha256": root('c'),
            "quotient_hypothesis_root_sha256": root('d'),
            "topology_nodes": [{
                "canonical_node": 0,
                "source": "action",
                "node_kind_code": 1,
                "value_type_code": EFFECT_VALUE_OPERATION_V3,
                "unique": true,
                "operation_code": EFFECT_OPERATION_CALL_V3
            }],
            "topology_edges": [],
            "relation_program": relation_program,
            "effect_invariant_root_sha256": effect_invariant_root_sha256,
            "preserved_frame_root_sha256": preserved_frame_root_sha256,
            "action_equivalence_root_sha256": action_equivalence_root_sha256
        }))
        .expect("canonical bytes");

        let law = CanonicalEffectLawV3::from_canonical_bytes(&bytes).expect("law");
        assert_eq!(law.canonical_bytes(), Ok(bytes));
        assert_eq!(validate_canonical_effect_law_v3(&law), Ok(()));
    }
}
