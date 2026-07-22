use std::path::PathBuf;

use nando_operator_kernel::{RuntimeProjectionV3, Sha256CommitmentV3};
use nando_operator_learning::ProviderRequestCaptureReceiptV3;
use serde::Serialize;

pub const PROVIDER_CAPTURE_MAX_QUEUE_V3: usize = 48;

#[derive(Clone, Debug)]
pub struct ProviderCaptureConfigV3 {
    pub enabled: bool,
    pub store_path: PathBuf,
    pub queue_capacity: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct ProviderCaptureIngressV3 {
    pub lineage_root_sha256: Sha256CommitmentV3,
    pub request_root_sha256: Sha256CommitmentV3,
    pub projection: RuntimeProjectionV3,
    pub streaming: bool,
    pub observed_at_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderCaptureSubmitV3 {
    Enqueued(ProviderRequestCaptureReceiptV3),
    Censored(ProviderCaptureCensoredReasonV3),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCaptureCensoredReasonV3 {
    Disabled,
    NotStarted,
    NoDurableLease,
    QueueFull,
    Disconnected,
    BudgetExhausted,
    InvalidProvenance,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ProviderCaptureStatusV3 {
    pub enabled: bool,
    pub phase: String,
    pub submitted: u64,
    pub enqueued: u64,
    pub captured: u64,
    pub censored: u64,
    pub ingress_censored: u64,
    pub writer_censored: u64,
    pub queue_full: u64,
    pub duplicates: u64,
    pub persistence_failures: u64,
    pub queued_now: u64,
    pub records: u64,
    pub publish_sequence: u64,
    pub reserved_through_sequence: u64,
    pub restart_sequence_reuse: u64,
    pub raw_payloads_persisted: u8,
    pub semantic_updates_from_censored: u8,
    pub local_accepts: u8,
    pub execution_authority: bool,
    pub accounting_identity_holds: bool,
    pub last_error: String,
}

impl ProviderCaptureConfigV3 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.queue_capacity == 0 || self.queue_capacity > PROVIDER_CAPTURE_MAX_QUEUE_V3 {
            return Err("provider_capture_config_invalid");
        }
        Ok(())
    }
}

impl ProviderCaptureSubmitV3 {
    pub fn into_receipt(self) -> Option<ProviderRequestCaptureReceiptV3> {
        match self {
            Self::Enqueued(receipt) => Some(receipt),
            Self::Censored(_) => None,
        }
    }
}
