//! Canonical F2 effect-law route with read-only F3 dual classification.
//!
//! These types do not grant runtime authority. Generation, admission, runtime,
//! checkpoint, and ACTIVE wiring remain outside this module.

mod canonical;
mod dual_classifier;
mod evidence;
mod trust;

use serde::{Deserialize, Serialize};

pub use nando_operator_kernel::{
    CANONICAL_EFFECT_LAW_SCHEMA_V3, CanonicalEffectEdgeV3, CanonicalEffectLawV3,
    CanonicalEffectNodeV3, CanonicalNodeMappingV3, CanonicalRelationClauseV3,
    EFFECT_ATOM_ACTION_RELATION, EFFECT_ATOM_CARDINALITY, EFFECT_ATOM_PHYSICAL_SURFACE,
    EFFECT_ATOM_POSTCONDITION, EFFECT_ATOM_PRECONDITION, EFFECT_ATOM_RENDERER,
    EFFECT_ATOM_TEMPORAL, EFFECT_REL_CONSTANT, EFFECT_REL_CONSUME, EFFECT_REL_COPY,
    EFFECT_REL_EQUAL, EFFECT_REL_REQUIRE, EffectLawIdV3, EffectLawV3Error,
};
use nando_operator_kernel::{
    EFFECT_LAW_IR_VERSION_V3, EFFECT_OPERATION_CALL_V3, EFFECT_OPERATION_PLAN_ADVANCE_V3,
    EFFECT_OPERATION_PROJECT_V3, EFFECT_OPERATION_STATUS_V3, EFFECT_VALUE_BOOLEAN_V3,
    EFFECT_VALUE_COLLECTION_V3, EFFECT_VALUE_IDENTIFIER_V3, EFFECT_VALUE_INTEGER_V3,
    EFFECT_VALUE_OPERATION_V3, EFFECT_VALUE_STRING_V3, MAX_EFFECT_EDGES_V3, MAX_EFFECT_NODES_V3,
    validate_canonical_effect_law_v3,
};

use crate::{
    AtomValueType, CaptureCommitmentIndex, DurableRuntimeParityReceipt, EffectSource, RelationAtom,
    ResponseProgram, TeacherTransition, VerifierProgram,
};

pub const EFFECT_OBSERVATION_CANDIDATE_SCHEMA_V3: &str = "nando.effect-observation-candidate.v3";
pub const SEALED_EFFECT_OBSERVATION_SCHEMA_V3: &str = "nando.sealed-effect-observation.v3";
pub const EFFECT_DELTA_CONTRACT_SCHEMA_V3: &str = "nando.effect-delta-contract.v3";
pub const EFFECT_LAW_RESTART_BUNDLE_SCHEMA_V3: &str = "nando.effect-law-restart-bundle.v3";
pub const EFFECT_QUOTIENT_HYPOTHESIS_SCHEMA_V3: &str = "nando.effect-quotient-hypothesis.v3";
pub const TRUSTED_GENERATION_MANIFEST_SCHEMA_V3: &str =
    "nando.trusted-effect-generation-manifest.v3";
pub const TRUSTED_EFFECT_EVIDENCE_SET_SCHEMA_V3: &str = "nando.trusted-effect-evidence-set.v3";
pub const TRUSTED_EFFECT_LAW_BUNDLE_ROOT_SCHEMA_V3: &str =
    "nando.trusted-effect-law-bundle-root.v3";
pub const INDEPENDENT_EFFECT_STATE_SCHEMA_V3: &str = "nando.independent-effect-state.v3";
pub const VERIFIED_EFFECT_DELTA_RECEIPT_SCHEMA_V3: &str = "nando.verified-effect-delta-receipt.v3";
pub const PROTOCOL_FACET_SCHEMA_V3: &str = "nando.effect-protocol-facet.v3";
pub const EFFECT_LAW_DUAL_CLASSIFICATION_REPORT_SCHEMA_V3: &str =
    "nando.effect-law-dual-classification-report.v1-v3.r1";

const MAX_DICTIONARY_ENTRIES_V3: usize = 256;
const MAX_OBSERVATIONS_V3: usize = 256;
const MAX_EFFECT_ATOMS_V3: usize = 512;
pub(crate) const EFFECT_LAW_ACTION_PHASE_V3: u16 = 2;
pub(crate) const EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3: usize = MAX_EFFECT_ATOMS_V3;
const MAX_CANONICAL_PERMUTATIONS_V3: usize = 16_384;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EffectDictionaryEntryV3 {
    pub code: u16,
    pub meaning_sha256: String,
    pub operand_schema_sha256: String,
    pub version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawDictionaryV3 {
    schema: String,
    version: u16,
    entries: Vec<EffectDictionaryEntryV3>,
    root_sha256: String,
}

#[derive(Deserialize)]
struct EffectLawDictionaryWireV3 {
    schema: String,
    version: u16,
    entries: Vec<EffectDictionaryEntryV3>,
    root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ExactEffectAtomV3 {
    pub phase: u16,
    pub class_code: u16,
    pub atom: RelationAtom,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectDeltaContractV3 {
    schema: String,
    exact_atoms: Vec<ExactEffectAtomV3>,
    exact_root_sha256: String,
    surface_root_sha256: String,
    postcondition_root_sha256: String,
    renderer_root_sha256: String,
    temporal_root_sha256: String,
    cardinality_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectObservationCandidateV3 {
    schema: String,
    candidate_sha256: String,
    transition: TeacherTransition,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct IndependentEffectStateV3 {
    schema: String,
    evidence_ref_sha256: String,
    before_atoms_root_sha256: String,
    actor_response_sha256: String,
    effect_atoms: Vec<RelationAtom>,
    observer_root_sha256: String,
    receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct TrustedGenerationEvidenceEntryV3 {
    evidence_ref_sha256: String,
    transition_sha256: String,
    episode_lineage_sha256: String,
    surface_root_sha256: String,
    physical_program_id: String,
    capture_receipt_root_sha256: String,
    parity_receipt_root_sha256: String,
    observed_state_root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct TrustedGenerationManifestWireV3 {
    schema: String,
    generation_id_sha256: String,
    delta_verifier_root_sha256: String,
    capture_index: CaptureCommitmentIndex,
    parity_receipts: Vec<DurableRuntimeParityReceipt>,
    observed_states: Vec<IndependentEffectStateV3>,
    entries: Vec<TrustedGenerationEvidenceEntryV3>,
}

#[derive(Clone, Debug)]
pub struct TrustedEffectEvidenceSetV3 {
    schema: String,
    generation_id_sha256: String,
    trust_manifest_root_sha256: String,
    delta_verifier_root_sha256: String,
    resolver_root_sha256: String,
    capture_index: CaptureCommitmentIndex,
    parity_by_evidence: std::collections::BTreeMap<String, DurableRuntimeParityReceipt>,
    observed_state_by_evidence: std::collections::BTreeMap<String, IndependentEffectStateV3>,
    entry_by_evidence: std::collections::BTreeMap<String, TrustedGenerationEvidenceEntryV3>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedGenerationManifestRootV3(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedEffectLawBundleRootV3 {
    schema: String,
    bundle_root_sha256: String,
    trust_manifest_root_sha256: String,
    dictionary_root_sha256: String,
    quotient_hypothesis_root_sha256: String,
    canonicalizer_version: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct VerifiedEffectDeltaReceiptV3 {
    schema: String,
    receipt_sha256: String,
    evidence_ref_sha256: String,
    transition_sha256: String,
    trust_manifest_root_sha256: String,
    observed_state_root_sha256: String,
    actor_program_sha256: String,
    verifier_program_sha256: String,
    delta_verifier_root_sha256: String,
    teacher_claim_root_sha256: String,
    delta: EffectDeltaContractV3,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ProtocolFacetV3 {
    schema: String,
    physical_atoms: Vec<serde_json::Value>,
    root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SealedEffectObservationV3 {
    schema: String,
    observation_sha256: String,
    evidence_ref_sha256: String,
    transition_sha256: String,
    episode_lineage_sha256: String,
    surface_root_sha256: String,
    physical_program_id: String,
    capture_receipt_root_sha256: String,
    parity_receipt_root_sha256: String,
    verifier_root_sha256: String,
    resolver_root_sha256: String,
    trust_manifest_root_sha256: String,
    observed_state_root_sha256: String,
    verified_delta_receipt_root_sha256: String,
    delta_verifier_root_sha256: String,
    delta: EffectDeltaContractV3,
    protocol_facet: ProtocolFacetV3,
    physical_graph: PhysicalEffectGraphV3,
    role_bindings: Vec<PhysicalRoleBindingV3>,
    constants: Vec<PhysicalConstantV3>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct PhysicalEffectNodeV3 {
    physical_node: u16,
    source: EffectSource,
    node_kind_code: u16,
    value_type_code: u16,
    unique: bool,
    operation_code: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct PhysicalEffectEdgeV3 {
    from: u16,
    to: u16,
    relation_code: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PhysicalEffectGraphV3 {
    nodes: Vec<PhysicalEffectNodeV3>,
    edges: Vec<PhysicalEffectEdgeV3>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct PhysicalRoleBindingV3 {
    argument_key_sha256: String,
    physical_node: u16,
    value_type_code: u16,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct PhysicalConstantV3 {
    argument_key_sha256: String,
    value_type_code: u16,
    value_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectQuotientHypothesisV3 {
    schema: String,
    version: u16,
    projected_atom_classes: Vec<u16>,
    root_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservationCanonicalProofV3 {
    pub observation_sha256: String,
    pub evidence_ref_sha256: String,
    pub transition_sha256: String,
    pub episode_lineage_sha256: String,
    pub surface_root_sha256: String,
    pub physical_program_id: String,
    pub node_mapping: Vec<CanonicalNodeMappingV3>,
    pub exact_delta_root_sha256: String,
    pub capture_receipt_root_sha256: String,
    pub parity_receipt_root_sha256: String,
    pub verifier_root_sha256: String,
    pub resolver_root_sha256: String,
    pub trust_manifest_root_sha256: String,
    pub observed_state_root_sha256: String,
    pub verified_delta_receipt_root_sha256: String,
    pub delta_verifier_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawRestartBundleV3 {
    schema: String,
    law: CanonicalEffectLawV3,
    proofs: Vec<ObservationCanonicalProofV3>,
    proof_set_root_sha256: String,
    bundle_sha256: String,
}

#[derive(Deserialize)]
struct EffectLawRestartBundleWireV3 {
    schema: String,
    law: CanonicalEffectLawV3,
    proofs: Vec<ObservationCanonicalProofV3>,
    proof_set_root_sha256: String,
    bundle_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawIndependenceV3 {
    pub observations: usize,
    pub episode_lineages: usize,
    pub surface_roots: usize,
    pub physical_program_ids: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEffectLawCandidateV3 {
    law: CanonicalEffectLawV3,
    restart_bundle: EffectLawRestartBundleV3,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EffectLawQuotientReportV3 {
    pub independence: EffectLawIndependenceV3,
    pub observation_set_root_sha256: String,
    pub candidate: Option<CanonicalEffectLawCandidateV3>,
    pub blocker: Option<String>,
}

impl EffectLawDictionaryV3 {
    pub fn new(
        version: u16,
        entries: Vec<EffectDictionaryEntryV3>,
    ) -> Result<Self, EffectLawV3Error> {
        evidence::build_dictionary(version, entries)
    }

    pub fn builtin() -> Result<Self, EffectLawV3Error> {
        evidence::builtin_dictionary()
    }

    #[must_use]
    pub fn root_sha256(&self) -> &str {
        &self.root_sha256
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EffectLawV3Error> {
        evidence::canonical_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, EffectLawV3Error> {
        evidence::dictionary_from_bytes(bytes)
    }
}

impl EffectDeltaContractV3 {
    #[must_use]
    pub fn exact_atoms(&self) -> &[ExactEffectAtomV3] {
        &self.exact_atoms
    }

    #[must_use]
    pub fn exact_root_sha256(&self) -> &str {
        &self.exact_root_sha256
    }

    #[must_use]
    pub fn surface_root_sha256(&self) -> &str {
        &self.surface_root_sha256
    }
}

impl TrustedEffectEvidenceSetV3 {
    #[must_use]
    pub fn trust_manifest_root_sha256(&self) -> &str {
        &self.trust_manifest_root_sha256
    }

    #[must_use]
    pub fn resolver_root_sha256(&self) -> &str {
        &self.resolver_root_sha256
    }
}

impl SealedEffectObservationV3 {
    #[must_use]
    pub fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    #[must_use]
    pub fn episode_lineage_sha256(&self) -> &str {
        &self.episode_lineage_sha256
    }

    #[must_use]
    pub fn surface_root_sha256(&self) -> &str {
        &self.surface_root_sha256
    }

    #[must_use]
    pub fn physical_program_id(&self) -> &str {
        &self.physical_program_id
    }

    #[must_use]
    pub fn delta(&self) -> &EffectDeltaContractV3 {
        &self.delta
    }
}

impl EffectQuotientHypothesisV3 {
    pub fn physical_adapters_only() -> Result<Self, EffectLawV3Error> {
        evidence::physical_adapter_hypothesis()
    }

    #[must_use]
    pub fn root_sha256(&self) -> &str {
        &self.root_sha256
    }
}

#[cfg(test)]
pub(crate) fn test_only_canonical_effect_law_v3(seed: &str) -> CanonicalEffectLawV3 {
    let root = |suffix: &str| crate::sha256_bytes(format!("{seed}:{suffix}").as_bytes());
    let wire = serde_json::json!({
        "schema": CANONICAL_EFFECT_LAW_SCHEMA_V3,
        "ir_version": EFFECT_LAW_IR_VERSION_V3,
        "dictionary_root_sha256": root("dictionary"),
        "quotient_hypothesis_root_sha256": root("quotient"),
        "topology_nodes": [{
            "canonical_node": 1,
            "source": EffectSource::Action,
            "node_kind_code": EFFECT_OPERATION_CALL_V3,
            "value_type_code": EFFECT_VALUE_OPERATION_V3,
            "unique": true,
            "operation_code": EFFECT_OPERATION_CALL_V3,
        }],
        "topology_edges": [],
        "relation_program": [{
            "relation_code": EFFECT_REL_REQUIRE,
            "lhs": 1,
            "rhs": null,
            "argument_ordinal": 0,
            "constant_type_code": null,
            "constant_sha256": null,
        }],
        "effect_invariant_root_sha256": root("effect-invariant"),
        "preserved_frame_root_sha256": root("preserved-frame"),
        "action_equivalence_root_sha256": root("action-equivalence"),
    });
    let bytes = crate::canonical_json_bytes(&wire).expect("test law bytes");
    CanonicalEffectLawV3::from_canonical_bytes(&bytes).expect("test law")
}

impl EffectLawRestartBundleV3 {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EffectLawV3Error> {
        evidence::canonical_bytes(self)
    }

    pub fn from_canonical_bytes(
        bytes: &[u8],
        trusted_evidence: &TrustedEffectEvidenceSetV3,
        expected_bundle_root: &TrustedEffectLawBundleRootV3,
    ) -> Result<Self, EffectLawV3Error> {
        canonical::restart_bundle_from_bytes(bytes, trusted_evidence, expected_bundle_root)
    }

    #[must_use]
    pub fn law(&self) -> &CanonicalEffectLawV3 {
        &self.law
    }

    #[must_use]
    pub fn proofs(&self) -> &[ObservationCanonicalProofV3] {
        &self.proofs
    }
}

impl CanonicalEffectLawCandidateV3 {
    #[must_use]
    pub fn law(&self) -> &CanonicalEffectLawV3 {
        &self.law
    }

    #[must_use]
    pub fn restart_bundle(&self) -> &EffectLawRestartBundleV3 {
        &self.restart_bundle
    }
}

pub fn observe_effect_transition_v3(
    transition: TeacherTransition,
) -> Result<EffectObservationCandidateV3, EffectLawV3Error> {
    evidence::observe_transition(transition)
}

pub fn resolve_trusted_effect_evidence_set_v3(
    manifest_bytes: &[u8],
    expected_manifest_root: &TrustedGenerationManifestRootV3,
) -> Result<TrustedEffectEvidenceSetV3, EffectLawV3Error> {
    trust::resolve_trusted_effect_evidence_set(manifest_bytes, expected_manifest_root)
}

pub fn seal_effect_observation_v3(
    candidate: EffectObservationCandidateV3,
    trusted_evidence: &TrustedEffectEvidenceSetV3,
    actor: &ResponseProgram,
    verifier: &VerifierProgram,
) -> Result<SealedEffectObservationV3, EffectLawV3Error> {
    evidence::seal_observation(candidate, trusted_evidence, actor, verifier)
}

pub fn search_effect_law_quotient_v3(
    observations: &[SealedEffectObservationV3],
    dictionary: &EffectLawDictionaryV3,
    hypothesis: &EffectQuotientHypothesisV3,
) -> Result<EffectLawQuotientReportV3, EffectLawV3Error> {
    canonical::search_quotient(observations, dictionary, hypothesis)
}

pub use dual_classifier::{
    EffectLawDualClassificationDiscrepancyDirectionV3, EffectLawDualClassificationDiscrepancyV3,
    EffectLawDualClassificationDiscrepancyWitnessV3, EffectLawDualClassificationMapV3,
    EffectLawDualClassificationReasonV3, EffectLawDualClassificationReportV3,
    EffectLawDualClassificationRowReportV3, EffectLawDualClassificationRowStatusV3,
    EffectLawDualClassificationRowV3, EffectLawDualClassificationVerdictV3,
    EffectLawDualClassifierV3, EffectLawDualIndependenceReportV3,
};

fn value_type_code(value_type: AtomValueType) -> u16 {
    match value_type {
        AtomValueType::String => EFFECT_VALUE_STRING_V3,
        AtomValueType::Integer => EFFECT_VALUE_INTEGER_V3,
        AtomValueType::Boolean => EFFECT_VALUE_BOOLEAN_V3,
        AtomValueType::Identifier => EFFECT_VALUE_IDENTIFIER_V3,
        AtomValueType::Collection => EFFECT_VALUE_COLLECTION_V3,
    }
}

#[cfg(test)]
#[path = "effect_law_v3_tests.rs"]
mod tests;
