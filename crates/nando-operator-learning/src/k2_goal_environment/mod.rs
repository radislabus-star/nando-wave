//! Authority-free goal-conditioned decision episodes for the isolated Law Lab.
//!
//! The module records capability evidence only. It cannot mutate natural
//! evidence, K1 membership, admission, economics, or phase memory.

mod journal;
mod model;

pub use journal::*;
pub use model::*;

#[cfg(test)]
mod tests;
