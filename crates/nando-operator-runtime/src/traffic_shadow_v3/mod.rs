mod execution;
mod generation;
mod handoff;
mod input;
mod pipeline;
mod receipt;

pub use execution::TrafficShadowExecutionV3;
pub use generation::{TrafficShadowGenerationV3, TrafficShadowRegistryV3};
pub use input::TrafficShadowInputV3;
pub use pipeline::{execute_traffic_shadow_v3, execute_traffic_shadow_with_handoff_v3};

use nando_operator_kernel::RuntimeProjectionV3;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TrafficShadowSourceV3 {
    Ordinary = 1,
    DevelopmentControl = 2,
    Replay = 3,
    SyntheticControl = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TrafficShadowVerdictV3 {
    CompleteShadow = 1,
    CensoredPayloadUnavailable = 2,
    AbstainUnsupportedProjection = 3,
    AbstainContextExtraction = 4,
    AbstainContextBudget = 5,
    AbstainDispatch = 6,
    AbstainBinding = 7,
    AbstainRuntimeBudget = 8,
    RejectInvariantMismatch = 9,
    AbstainMissingCapability = 10,
    AbstainAmbiguousCapability = 11,
    AbstainAmbiguousAction = 12,
    AbstainRoleValue = 13,
    AbstainPhase = 14,
    AbstainActorVm = 15,
    ActorVmParityMismatch = 16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficShadowInputErrorV3 {
    InvalidDigest,
    PartialRuntimePayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficShadowGenerationErrorV3 {
    InvalidSequence,
    InvalidRoot,
    GenerationMismatch,
    NonMonotonicSwap,
    RegistryPoisoned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficShadowHandoffVerdictV3 {
    Enqueued,
    CensoredQueueFull,
    CensoredDisconnected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TrafficShadowReceiptV3 {
    receipt_sha256: String,
    window_row_sha256: String,
    request_sha256: String,
    generation_sequence: u64,
    generation_root_sha256: String,
    index_sha256: String,
    projection: Option<RuntimeProjectionV3>,
    streaming: Option<bool>,
    source: TrafficShadowSourceV3,
    verdict: TrafficShadowVerdictV3,
    extraction_receipt_sha256: Option<String>,
    phase_report_sha256: Option<String>,
    operator_shadow_receipt_sha256: Option<String>,
    elapsed_nanos: u64,
    raw_payloads_persisted: u8,
    local_accepts: u8,
    execution_authority: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct TrafficShadowHandoffCountersV3 {
    attempted: u64,
    enqueued: u64,
    censored_queue_full: u64,
    censored_disconnected: u64,
}

#[cfg(test)]
mod tests;
