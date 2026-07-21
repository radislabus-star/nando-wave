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
pub mod effect_graph;
pub mod effect_law;
pub mod evidence;
pub mod evidence_graph;
pub mod executable_protocol_mode;
pub mod family_discovery;
pub mod grounding;
pub mod online_checkpoint;
pub mod online_subcenter;
pub mod operator_generation;
pub mod opportunity;
pub mod protocol_mode;
pub mod runtime_parity;
pub mod semantic_alias;
pub mod teacher_join;
pub mod training_types;
pub mod version_space;

pub use backward_wave::*;
pub use binding_evidence::*;
pub use binding_evidence_adjudication::*;
pub use binding_evidence_capture_owner::*;
pub use binding_evidence_future_capture::*;
pub use binding_evidence_preregistration::*;
pub use capture_provenance::*;
pub use effect_graph::*;
pub use effect_law::*;
pub use evidence::*;
pub use evidence_graph::*;
pub use executable_protocol_mode::*;
pub use family_discovery::*;
pub use grounding::*;
pub use online_checkpoint::*;
pub use online_subcenter::*;
pub use operator_generation::*;
pub use opportunity::*;
pub use protocol_mode::*;
pub use runtime_parity::*;
pub use semantic_alias::*;
pub use teacher_join::*;
pub use training_types::*;
pub use version_space::*;

pub use nando_operator_kernel::{
    AtomValueType, CanonicalEffectLawV3, EFFECT_LAW_ACTION_PHASE_V3,
    EFFECT_LAW_MAX_PROTOCOL_FACET_ATOMS_V3, PROTOCOL_FACET_SCHEMA_V3, RelationAtom, RelationFrame,
    ResponseOperation, ResponseProgram, ResponseValueSelector, canonical_json_bytes,
    canonical_json_sha256, relation_frame_hidden_wave_atom_ids,
    relation_frame_online_routing_atom_ids, relation_frame_phase_atom_ids,
    relation_frame_routing_atom_ids, response_program_required_routing_atom_ids, sha256_bytes,
    valid_nonzero_sha256,
};
pub use nando_operator_proof::verified_delta::*;

pub(crate) use nando_operator_kernel::{AtomSource, CollectionOutputRenderer};

#[cfg(test)]
pub(crate) use nando_operator_kernel::{ProjectStatusMapping, RELATION_FRAME_SCHEMA};
