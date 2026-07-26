//! Bounded, non-authoritative composition substrate for MS7 experiments.
//!
//! Bundle resolution is explicit and in-memory. This module neither loads
//! registry state nor grants execution authority.

mod execute;
mod types;

pub use execute::execute_composition_shadow_v1;
pub use types::*;

#[cfg(test)]
mod tests;
