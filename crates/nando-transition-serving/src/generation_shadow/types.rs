use std::path::PathBuf;
use std::time::Duration;

use axum::body::Bytes;
use nando_operator_kernel::{RuntimeProjectionV3, sha256_bytes, valid_nonzero_sha256};
use nando_operator_learning::ProviderRequestCaptureReceiptV3;
use nando_operator_proof::independent_verifier_v3::{
    F6_MAX_RAW_REQUEST_BYTES_V3, F6_MAX_REQUEST_TEXT_BYTES_V3,
};
use serde::Serialize;

// The queue retains bounded provider bytes. Forty-eight worst-case F6 inputs
// stay below the serving memory budget while overload remains non-blocking.
pub const GENERATION_SHADOW_MAX_QUEUE_V3: usize = 48;

#[derive(Clone, Debug)]
pub struct GenerationShadowConfigV3 {
    pub enabled: bool,
    pub store_path: PathBuf,
    pub capture_index_path: PathBuf,
    pub provider_capture_store_path: PathBuf,
    pub receipt_store_path: PathBuf,
    pub queue_capacity: usize,
    pub poll_interval: Duration,
}

pub(crate) struct GenerationShadowIngressV3<'a> {
    pub capture_receipt: ProviderRequestCaptureReceiptV3,
    pub request_text: &'a str,
    pub provider_payload_bytes: Bytes,
}

#[derive(Clone)]
pub struct GenerationShadowRequestV3 {
    capture_receipt: Option<ProviderRequestCaptureReceiptV3>,
    window_row_sha256: String,
    request_sha256: String,
    projection: RuntimeProjectionV3,
    streaming: bool,
    request_text: String,
    provider_payload_bytes: Bytes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowRequestErrorV3 {
    InvalidCommitment,
    RequestDigestMismatch,
    BudgetExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationShadowSubmitVerdictV3 {
    Enqueued,
    CensoredDisabled,
    CensoredNotStarted,
    CensoredNoGeneration,
    CensoredQueueFull,
    CensoredDisconnected,
    CensoredBudget,
    CensoredInvalidRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationShadowEvaluationVerdictV3 {
    Verified,
    RuntimeAbstain,
    RuntimeReject,
    VerifierAbstain,
    VerifierReject,
    InvalidRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GenerationShadowEvaluationReceiptV3 {
    pub generation_id_sha256: String,
    pub publish_sequence: u64,
    pub request_sha256: String,
    pub capture_sequence: Option<u64>,
    pub capture_event_sha256: Option<String>,
    pub capture_receipt_sha256: Option<String>,
    pub traffic_receipt_sha256: String,
    pub verifier_receipt_sha256: Option<String>,
    pub verdict: GenerationShadowEvaluationVerdictV3,
    pub parity_mismatch: bool,
    pub raw_payloads_persisted: u8,
    pub local_accepts: u8,
    pub execution_authority: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct GenerationShadowStatusV3 {
    pub enabled: bool,
    pub phase: String,
    pub generation_sequence: u64,
    pub generation_id_sha256: String,
    pub publish_sequence: u64,
    pub checkpoint_sha256: String,
    pub capture_index_sha256: String,
    pub load_attempts: u64,
    pub load_successes: u64,
    pub load_failures: u64,
    pub submitted: u64,
    pub enqueued: u64,
    pub censored: u64,
    pub evaluated: u64,
    pub verified: u64,
    pub runtime_abstains: u64,
    pub runtime_rejects: u64,
    pub verifier_abstains: u64,
    pub verifier_rejects: u64,
    pub durable_appends: u64,
    pub durable_censored: u64,
    pub shadow_ledger_sha256: String,
    pub false_accepts: u64,
    pub parity_mismatches: u64,
    pub local_accepts: u64,
    pub last_error: String,
    pub execution_authority: bool,
}

impl GenerationShadowConfigV3 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.queue_capacity == 0
            || self.queue_capacity > GENERATION_SHADOW_MAX_QUEUE_V3
            || self.poll_interval < Duration::from_millis(100)
        {
            return Err("generation_shadow_config_invalid");
        }
        Ok(())
    }
}

impl GenerationShadowRequestV3 {
    pub fn new(
        window_row_sha256: String,
        request_sha256: String,
        projection: RuntimeProjectionV3,
        streaming: bool,
        request_text: String,
        provider_payload_bytes: Bytes,
    ) -> Result<Self, GenerationShadowRequestErrorV3> {
        if !valid_nonzero_sha256(&window_row_sha256) || !valid_nonzero_sha256(&request_sha256) {
            return Err(GenerationShadowRequestErrorV3::InvalidCommitment);
        }
        if sha256_bytes(&provider_payload_bytes) != request_sha256 {
            return Err(GenerationShadowRequestErrorV3::RequestDigestMismatch);
        }
        Self::from_capture_owner(
            None,
            window_row_sha256,
            request_sha256,
            projection,
            streaming,
            request_text,
            provider_payload_bytes,
        )
    }

    pub(super) fn from_capture_owner(
        capture_receipt: Option<ProviderRequestCaptureReceiptV3>,
        window_row_sha256: String,
        request_sha256: String,
        projection: RuntimeProjectionV3,
        streaming: bool,
        request_text: String,
        provider_payload_bytes: Bytes,
    ) -> Result<Self, GenerationShadowRequestErrorV3> {
        if provider_payload_bytes.is_empty()
            || provider_payload_bytes.len() > F6_MAX_RAW_REQUEST_BYTES_V3
            || request_text.len() > F6_MAX_REQUEST_TEXT_BYTES_V3
        {
            return Err(GenerationShadowRequestErrorV3::BudgetExhausted);
        }
        Ok(Self {
            capture_receipt,
            window_row_sha256,
            request_sha256,
            projection,
            streaming,
            request_text,
            provider_payload_bytes,
        })
    }

    pub fn from_provider_capture(
        capture_receipt: ProviderRequestCaptureReceiptV3,
        request_text: String,
        provider_payload_bytes: Bytes,
    ) -> Result<Self, GenerationShadowRequestErrorV3> {
        let window_row_sha256 = capture_receipt.event_root_sha256().to_hex();
        let request_sha256 = capture_receipt.request_root_sha256().to_hex();
        let projection = capture_receipt.projection();
        let streaming = capture_receipt.streaming();
        Self::from_capture_owner(
            Some(capture_receipt),
            window_row_sha256,
            request_sha256,
            projection,
            streaming,
            request_text,
            provider_payload_bytes,
        )
    }

    pub(super) fn window_row_sha256(&self) -> &str {
        &self.window_row_sha256
    }

    pub(super) fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub(super) const fn capture_receipt(&self) -> Option<&ProviderRequestCaptureReceiptV3> {
        self.capture_receipt.as_ref()
    }

    pub(super) const fn projection(&self) -> RuntimeProjectionV3 {
        self.projection
    }

    pub(super) const fn streaming(&self) -> bool {
        self.streaming
    }

    pub(crate) fn request_text(&self) -> &str {
        &self.request_text
    }

    pub(super) fn provider_payload_bytes(&self) -> &[u8] {
        &self.provider_payload_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nando_operator_kernel::Sha256CommitmentV3;
    use nando_operator_learning::{
        ProviderRequestCaptureInputV3, seal_provider_request_capture_v3,
    };

    #[test]
    fn live_shadow_preserves_the_capture_owner_receipt_without_a_second_body_hash() {
        let request_root = Sha256CommitmentV3::digest_bytes(b"owner-hashed-provider-body");
        let receipt = seal_provider_request_capture_v3(ProviderRequestCaptureInputV3 {
            capture_sequence: 17,
            capture_epoch_root: root("epoch"),
            lineage_root_sha256: root("lineage"),
            request_root_sha256: request_root,
            projection: RuntimeProjectionV3::Responses,
            streaming: true,
            observed_at_unix_ms: 1_750_000_000_000,
        })
        .expect("capture receipt");

        // F6 owns the independent byte-parity check off the request path.
        let request = GenerationShadowRequestV3::from_provider_capture(
            receipt.clone(),
            "continue".to_owned(),
            Bytes::from_static(b"not-rehashed-on-ingress"),
        )
        .expect("shadow request");
        assert_eq!(request.request_sha256(), request_root.to_hex());
        assert_eq!(
            request.window_row_sha256(),
            receipt.event_root_sha256().to_hex()
        );
        assert_eq!(request.capture_receipt(), Some(&receipt));
    }

    fn root(label: &str) -> Sha256CommitmentV3 {
        Sha256CommitmentV3::digest_bytes(label.as_bytes())
    }
}
