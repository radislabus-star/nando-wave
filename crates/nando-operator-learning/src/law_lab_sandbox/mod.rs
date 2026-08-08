//! Disposable, authority-free execution for bounded Law Lab probes.
//!
//! The adapter binds an externally frozen version space and prediction ledger
//! to an exact executor manifest. It can report an isolated outcome, but it
//! cannot identify operators, issue certificates, mutate K1, or execute hot
//! traffic.

mod adapter;
mod manifest;
mod model;
pub mod worker;

pub use adapter::*;
pub use manifest::*;
pub use model::*;
pub use worker::run_law_lab_sandbox_worker_v1;
