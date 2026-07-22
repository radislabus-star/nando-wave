//! Crash-safe persistence for immutable operator generations.
//!
//! This crate owns filesystem durability only. It cannot execute an operator,
//! classify evidence, verify an action, or grant admission authority.

mod checkpoint;
mod generation_shadow_store_v3;
mod provider_capture_store_v3;
mod store;

pub use checkpoint::*;
pub use generation_shadow_store_v3::*;
pub use provider_capture_store_v3::*;
pub use store::*;
