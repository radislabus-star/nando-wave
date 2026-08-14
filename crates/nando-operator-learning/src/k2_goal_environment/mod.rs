//! Authority-free goal-conditioned decision episodes for the isolated Law Lab.
//!
//! The module records capability evidence only. It cannot mutate natural
//! evidence, K1 membership, admission, economics, or phase memory.

mod journal;
pub mod learned_capability;
pub mod learned_composition;
pub mod learned_journal;
mod model;

pub use journal::*;
pub use learned_capability::*;
pub use learned_composition::*;
pub use learned_journal::*;
pub use model::*;

#[cfg(test)]
mod tests;
