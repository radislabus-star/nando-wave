//! Crash-safe persistence for immutable operator generations.
//!
//! This crate owns filesystem durability only. It cannot execute an operator,
//! classify evidence, verify an action, or grant admission authority.

mod checkpoint;
mod store;

pub use checkpoint::*;
pub use store::*;
