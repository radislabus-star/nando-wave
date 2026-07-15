//! Typed, partial state-transition actors.
//!
//! This crate is intentionally independent from the phase-center router. A
//! router may nominate a program, but only this actor plus its verifier can
//! construct and accept `state_t+1`.

mod adapter;
mod live;
mod program;
mod runtime;
mod verifier;

pub use adapter::{ActionRule, AdaptedState, AdapterError, Layout, SurfaceAdapter};
pub use live::{
    LiveExecutionResult, PACKAGE_SCHEMA, REQUEST_SCHEMA, RESPONSE_SCHEMA, TransitionPackage,
    TransitionRequest, execute_live_request,
};
pub use program::{TransitionOperation, TransitionProgram, ValueKind};
pub use runtime::{
    CanonicalRecord, CanonicalState, ExecutionResult, ExecutionStatus, execute_canonical,
    execute_surface,
};
pub use verifier::{VerificationError, verify_transition};

#[cfg(test)]
mod tests;
