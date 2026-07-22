use nando_operator_kernel::{
    RuntimePhaseControlEvidenceV3, Sha256CommitmentV3, valid_nonzero_sha256,
};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3;
use serde::{Deserialize, Serialize};

use crate::ProviderRequestCaptureReceiptV3;

pub const GENERATION_SHADOW_LEDGER_SCHEMA_V3: &str =
    "nando.generation-shadow-receipt-ledger.v3.f8b";
pub const GENERATION_SHADOW_LEDGER_MAX_RECORDS_V3: usize = 4_096;
pub const GENERATION_SHADOW_LEDGER_MAX_BYTES_V3: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationShadowTerminalOutcomeV3 {
    VerifiedPass,
    RuntimeAbstain,
    RuntimeReject,
    VerifierAbstain,
    VerifierReject,
    Censored,
}

impl GenerationShadowTerminalOutcomeV3 {
    #[must_use]
    pub const fn permits_positive_evidence(self) -> bool {
        matches!(self, Self::VerifiedPass)
    }
}

#[derive(Clone, Debug)]
pub struct GenerationShadowReceiptInputV3<'a> {
    pub capture_receipt: &'a ProviderRequestCaptureReceiptV3,
    pub traffic_receipt_sha256: &'a str,
    pub traffic_generation_sequence: u64,
    pub traffic_generation_id_sha256: &'a str,
    pub traffic_index_sha256: &'a str,
    pub traffic_request_sha256: &'a str,
    pub traffic_verdict_code: u8,
    pub traffic_phase_report_sha256: Option<&'a str>,
    pub traffic_operator_receipt_sha256: Option<&'a str>,
    pub phase_control_evidence: Option<&'a RuntimePhaseControlEvidenceV3>,
    pub f6_receipt: Option<&'a IndependentVerifierReceiptV3>,
    pub outcome: GenerationShadowTerminalOutcomeV3,
    pub parity_mismatch: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationShadowReceiptV3 {
    pub(super) ordinal: u32,
    pub(super) previous_receipt_sha256: String,
    pub(super) generation_id_sha256: String,
    pub(super) generation_publish_sequence: u64,
    pub(super) generation_checkpoint_sha256: String,
    pub(super) capture_index_sha256: Sha256CommitmentV3,
    pub(super) capture_sequence: u64,
    pub(super) capture_event_sha256: Sha256CommitmentV3,
    pub(super) request_sha256: Sha256CommitmentV3,
    pub(super) capture_receipt_sha256: Sha256CommitmentV3,
    pub(super) traffic_receipt_sha256: String,
    pub(super) traffic_generation_sequence: u64,
    pub(super) traffic_generation_id_sha256: String,
    pub(super) traffic_index_sha256: String,
    pub(super) traffic_request_sha256: String,
    pub(super) traffic_verdict_code: u8,
    pub(super) traffic_phase_report_sha256: Option<String>,
    pub(super) traffic_operator_receipt_sha256: Option<String>,
    pub(super) phase_control_evidence: Option<RuntimePhaseControlEvidenceV3>,
    pub(super) actor_action_sha256: Option<String>,
    pub(super) actor_output_sha256: Option<String>,
    pub(super) verifier_receipt_sha256: Option<String>,
    pub(super) verifier_receipt: Option<IndependentVerifierReceiptV3>,
    pub(super) outcome: GenerationShadowTerminalOutcomeV3,
    pub(super) parity_mismatch: bool,
    pub(super) semantic_updates: u8,
    pub(super) raw_payloads_persisted: u8,
    pub(super) local_accepts: u8,
    pub(super) execution_authority: bool,
    pub(super) receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationShadowLedgerErrorV3 {
    InvalidGeneration,
    InvalidCheckpoint,
    InvalidCaptureJoin,
    InvalidTrafficReceipt,
    InvalidPhaseControlEvidence,
    InvalidVerifierReceipt,
    OutcomeMismatch,
    NonMonotonicCapture,
    DuplicateCommitment,
    BudgetExhausted,
    EvidenceRollback,
    InvalidLedger,
    Serialization,
}

impl GenerationShadowReceiptV3 {
    pub(super) fn validate_fields(&self) -> Result<(), GenerationShadowLedgerErrorV3> {
        if self.generation_publish_sequence == 0
            || self.capture_sequence == 0
            || !valid_nonzero_sha256(&self.generation_id_sha256)
            || !valid_nonzero_sha256(&self.generation_checkpoint_sha256)
            || !valid_nonzero_sha256(&self.traffic_receipt_sha256)
            || self.traffic_generation_sequence == 0
            || !valid_nonzero_sha256(&self.traffic_generation_id_sha256)
            || self.traffic_generation_id_sha256 != self.generation_id_sha256
            || !valid_nonzero_sha256(&self.traffic_index_sha256)
            || !valid_nonzero_sha256(&self.traffic_request_sha256)
            || self.traffic_request_sha256 != self.request_sha256.to_hex()
            || self.traffic_verdict_code == 0
            || !valid_nonzero_sha256(&self.previous_receipt_sha256)
            || !valid_nonzero_sha256(&self.receipt_sha256)
            || self.semantic_updates != u8::from(self.outcome.permits_positive_evidence())
            || self.raw_payloads_persisted != 0
            || self.local_accepts != 0
            || self.execution_authority
            || !valid_phase_control_evidence(
                self.traffic_index_sha256.as_str(),
                self.traffic_phase_report_sha256.as_deref(),
                self.phase_control_evidence.as_ref(),
            )
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
        }
        for root in [
            self.actor_action_sha256.as_deref(),
            self.actor_output_sha256.as_deref(),
            self.verifier_receipt_sha256.as_deref(),
            self.traffic_phase_report_sha256.as_deref(),
            self.traffic_operator_receipt_sha256.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !valid_nonzero_sha256(root) {
                return Err(GenerationShadowLedgerErrorV3::InvalidLedger);
            }
        }
        Ok(())
    }

    #[must_use]
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    #[must_use]
    pub fn generation_id_sha256(&self) -> &str {
        &self.generation_id_sha256
    }

    #[must_use]
    pub const fn generation_publish_sequence(&self) -> u64 {
        self.generation_publish_sequence
    }

    #[must_use]
    pub fn generation_checkpoint_sha256(&self) -> &str {
        &self.generation_checkpoint_sha256
    }

    #[must_use]
    pub const fn capture_index_sha256(&self) -> Sha256CommitmentV3 {
        self.capture_index_sha256
    }

    #[must_use]
    pub const fn capture_sequence(&self) -> u64 {
        self.capture_sequence
    }

    #[must_use]
    pub const fn capture_event_sha256(&self) -> Sha256CommitmentV3 {
        self.capture_event_sha256
    }

    #[must_use]
    pub const fn request_sha256(&self) -> Sha256CommitmentV3 {
        self.request_sha256
    }

    #[must_use]
    pub const fn capture_receipt_sha256(&self) -> Sha256CommitmentV3 {
        self.capture_receipt_sha256
    }

    #[must_use]
    pub fn traffic_receipt_sha256(&self) -> &str {
        &self.traffic_receipt_sha256
    }

    #[must_use]
    pub const fn traffic_generation_sequence(&self) -> u64 {
        self.traffic_generation_sequence
    }

    #[must_use]
    pub fn traffic_generation_id_sha256(&self) -> &str {
        &self.traffic_generation_id_sha256
    }

    #[must_use]
    pub fn traffic_index_sha256(&self) -> &str {
        &self.traffic_index_sha256
    }

    #[must_use]
    pub fn traffic_request_sha256(&self) -> &str {
        &self.traffic_request_sha256
    }

    #[must_use]
    pub const fn traffic_verdict_code(&self) -> u8 {
        self.traffic_verdict_code
    }

    #[must_use]
    pub fn traffic_phase_report_sha256(&self) -> Option<&str> {
        self.traffic_phase_report_sha256.as_deref()
    }

    #[must_use]
    pub fn traffic_operator_receipt_sha256(&self) -> Option<&str> {
        self.traffic_operator_receipt_sha256.as_deref()
    }

    #[must_use]
    pub const fn phase_control_evidence(&self) -> Option<&RuntimePhaseControlEvidenceV3> {
        self.phase_control_evidence.as_ref()
    }

    #[must_use]
    pub fn actor_action_sha256(&self) -> Option<&str> {
        self.actor_action_sha256.as_deref()
    }

    #[must_use]
    pub fn actor_output_sha256(&self) -> Option<&str> {
        self.actor_output_sha256.as_deref()
    }

    #[must_use]
    pub fn verifier_receipt_sha256(&self) -> Option<&str> {
        self.verifier_receipt_sha256.as_deref()
    }

    #[must_use]
    pub const fn verifier_receipt(&self) -> Option<&IndependentVerifierReceiptV3> {
        self.verifier_receipt.as_ref()
    }

    #[must_use]
    pub const fn outcome(&self) -> GenerationShadowTerminalOutcomeV3 {
        self.outcome
    }

    #[must_use]
    pub const fn parity_mismatch(&self) -> bool {
        self.parity_mismatch
    }

    #[must_use]
    pub const fn semantic_updates(&self) -> u8 {
        self.semantic_updates
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        self.raw_payloads_persisted
    }

    #[must_use]
    pub const fn local_accepts(&self) -> u8 {
        self.local_accepts
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        self.execution_authority
    }

    #[must_use]
    pub fn receipt_sha256(&self) -> &str {
        &self.receipt_sha256
    }
}

fn valid_phase_control_evidence(
    traffic_index_sha256: &str,
    phase_report_sha256: Option<&str>,
    evidence: Option<&RuntimePhaseControlEvidenceV3>,
) -> bool {
    match (phase_report_sha256, evidence) {
        (None, None) => true,
        (Some(report), Some(evidence)) => {
            evidence.report_sha256() == report
                && evidence.index_sha256() == traffic_index_sha256
                && evidence.raw_payloads_persisted() == 0
                && !evidence.execution_authority()
                && evidence.canonical_bytes().is_ok_and(|bytes| {
                    RuntimePhaseControlEvidenceV3::from_canonical_bytes(&bytes)
                        .is_ok_and(|restored| restored == *evidence)
                })
        }
        _ => false,
    }
}
