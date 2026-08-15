//! Generated-only hidden composition representation experiment.
//!
//! Every route in this module has proposal or evidence authority only. It
//! cannot mutate natural K1, phase memory, packages, certificates, services,
//! or production state.

mod baseline;
mod journal;
mod model;
mod policy;
mod sandbox;
mod trainer;
mod verifier;

pub use baseline::*;
pub use journal::*;
pub use model::*;
pub use policy::*;
pub use sandbox::*;
pub use trainer::*;
pub use verifier::*;
