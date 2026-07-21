//! Cold evidence, law discovery, and bounded operator learning.
//!
//! This crate may produce immutable candidates, but it never executes hot
//! requests, mutates admission state, or grants authority.

pub mod binding_evidence;
pub mod binding_evidence_capture_owner;
pub mod binding_evidence_future_capture;
pub mod binding_evidence_preregistration;
pub mod capture_provenance;
pub mod effect_graph;
pub mod effect_law;
pub mod evidence;
pub mod evidence_graph;
pub mod runtime_parity;
pub mod teacher_join;
pub mod training_types;

pub use binding_evidence::*;
pub use binding_evidence_capture_owner::*;
pub use binding_evidence_future_capture::*;
pub use binding_evidence_preregistration::*;
pub use capture_provenance::*;
pub use effect_graph::*;
pub use effect_law::*;
pub use evidence::*;
pub use evidence_graph::*;
pub use runtime_parity::*;
pub use teacher_join::*;
pub use training_types::*;

pub(crate) use nando_operator_kernel::{AtomSource, AtomValueType, CollectionOutputRenderer};

#[cfg(test)]
pub(crate) use nando_operator_kernel::{RELATION_FRAME_SCHEMA, ResponseValueSelector};

#[cfg(test)]
pub(crate) const SOURCE_NEUTRAL_EXTRACTOR_VERSION: &str = "response-relation-extractor.v16";
