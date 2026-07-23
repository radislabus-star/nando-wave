//! Cold evidence, law discovery, and bounded operator learning.
//!
//! This crate may produce immutable candidates, but it never executes hot
//! requests, mutates admission state, or grants authority.

pub mod backward_wave;
pub mod binding_evidence;
pub mod binding_evidence_adjudication;
pub mod binding_evidence_capture_owner;
pub mod binding_evidence_future_capture;
pub mod binding_evidence_preregistration;
pub mod capture_provenance;
pub mod cegis;
pub mod collection_types;
pub mod effect_graph;
pub mod effect_law;
pub mod evidence;
pub mod evidence_graph;
pub mod executable_protocol_mode;
pub mod family_discovery;
pub mod generation_capture_v3;
pub mod generation_evidence_v3;
pub mod generation_shadow_v3;
pub mod grounding;
pub mod learning_evidence_bridge;
pub mod online_checkpoint;
pub mod online_collection_types;
pub mod online_subcenter;
pub mod operator_generation;
pub mod opportunity;
pub mod opportunity_bridge;
pub mod protocol_mode;
pub mod provider_capture_v3;
pub mod rollover;
pub mod runtime_parity;
pub mod self_training_types;
pub mod semantic_alias;
pub mod synthesis;
pub mod teacher_join;
pub mod training_types;
pub mod version_space;
pub mod wave_route_learning;

pub use backward_wave::*;
pub use binding_evidence::*;
pub use binding_evidence_adjudication::*;
pub use binding_evidence_capture_owner::*;
pub use binding_evidence_future_capture::*;
pub use binding_evidence_preregistration::*;
pub use capture_provenance::*;
pub use cegis::*;
pub use collection_types::*;
pub use effect_graph::*;
pub use effect_law::*;
pub use evidence::*;
pub use evidence_graph::*;
pub use executable_protocol_mode::*;
pub use family_discovery::*;
pub use generation_capture_v3::*;
pub use generation_evidence_v3::*;
pub use generation_shadow_v3::*;
pub use grounding::*;
pub use learning_evidence_bridge::*;
pub use online_checkpoint::*;
pub use online_collection_types::*;
pub use online_subcenter::*;
pub use operator_generation::*;
pub use opportunity::*;
pub use opportunity_bridge::*;
pub use protocol_mode::*;
pub use provider_capture_v3::*;
pub use rollover::*;
pub use runtime_parity::*;
pub use self_training_types::*;
pub use semantic_alias::*;
pub use synthesis::{
    SynthesisError, SynthesizedResponseOperator, partition_teacher_training_families,
    synthesize_response_operator, verify_operator_structure,
};
pub use teacher_join::*;
pub use training_types::*;
pub use version_space::*;
pub use wave_route_learning::*;

pub use nando_operator_kernel::contracts;
pub use nando_operator_kernel::{
    AtomValueType, CanonicalEffectLawV3, CustomToolResultProjection, EFFECT_LAW_ACTION_PHASE_V3,
    EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3, GuardCandidate, LearnedWaveRoute, LearnedWaveSubcenter,
    PROGRAM_CANDIDATE_SCHEMA, PROTOCOL_FACET_SCHEMA_V3, ProjectStatusMapping, RelationAtom,
    RelationFrame, ResponseArgument, ResponseOperation, ResponseProgram, ResponseProgramCandidate,
    ResponseValueSelector, RoleHypothesis, SemanticRole, ValueProjectionFormat,
    VerifierConsensusVariant, VerifierProgram, canonical_json_bytes, canonical_json_sha256,
    relation_frame_hidden_wave_atom_ids, relation_frame_online_routing_atom_ids,
    relation_frame_phase_atom_ids, relation_frame_routing_atom_ids,
    response_program_required_routing_atom_ids, sha256_bytes, valid_nonzero_sha256,
};
pub use nando_operator_proof::verified_delta::*;

pub(crate) use nando_operator_kernel::{AtomSource, CollectionOutputRenderer};

#[cfg(test)]
pub(crate) use nando_operator_kernel::RELATION_FRAME_SCHEMA;
