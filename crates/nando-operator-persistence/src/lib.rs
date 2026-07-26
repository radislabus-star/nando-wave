//! Crash-safe persistence for immutable operator generations.
//!
//! This crate owns filesystem durability only. It cannot execute an operator,
//! classify evidence, verify an action, or grant admission authority.

mod checkpoint;
mod crystallized_bundle_v4;
mod generation_shadow_store_v3;
mod provider_capture_store_v3;
mod store;

pub use checkpoint::*;
pub use crystallized_bundle_v4::*;
pub use generation_shadow_store_v3::*;
pub use provider_capture_store_v3::*;
pub use store::*;
