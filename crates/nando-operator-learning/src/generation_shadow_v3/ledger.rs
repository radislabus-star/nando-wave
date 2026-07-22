use std::collections::BTreeSet;

use nando_operator_kernel::{Sha256CommitmentV3, canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_proof::independent_verifier_v3::IndependentVerifierVerdictV3;

use crate::ProviderCaptureIndexV3;

use super::{
    GENERATION_SHADOW_LEDGER_MAX_RECORDS_V3, GENERATION_SHADOW_LEDGER_SCHEMA_V3,
    GenerationShadowLedgerErrorV3, GenerationShadowReceiptInputV3, GenerationShadowReceiptV3,
    GenerationShadowTerminalOutcomeV3,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationShadowReceiptLedgerV3 {
    pub(super) generation_id_sha256: String,
    pub(super) generation_publish_sequence: u64,
    pub(super) generation_checkpoint_sha256: String,
    pub(super) publish_sequence: u64,
    pub(super) receipts: Vec<GenerationShadowReceiptV3>,
    pub(super) ledger_sha256: String,
    pub(super) capture_sequences: BTreeSet<u64>,
    pub(super) event_roots: BTreeSet<Sha256CommitmentV3>,
    pub(super) request_roots: BTreeSet<Sha256CommitmentV3>,
    pub(super) capture_receipt_roots: BTreeSet<Sha256CommitmentV3>,
    pub(super) traffic_receipt_roots: BTreeSet<String>,
    pub(super) verifier_receipt_roots: BTreeSet<String>,
}

impl GenerationShadowReceiptLedgerV3 {
    pub fn new(
        generation_id_sha256: String,
        generation_publish_sequence: u64,
        generation_checkpoint_sha256: String,
    ) -> Result<Self, GenerationShadowLedgerErrorV3> {
        if !valid_nonzero_sha256(&generation_id_sha256)
            || generation_publish_sequence == 0
            || !valid_nonzero_sha256(&generation_checkpoint_sha256)
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidGeneration);
        }
        let ledger_sha256 = ledger_digest(
            &generation_id_sha256,
            generation_publish_sequence,
            &generation_checkpoint_sha256,
            0,
            &[],
        )?;
        Ok(Self {
            generation_id_sha256,
            generation_publish_sequence,
            generation_checkpoint_sha256,
            publish_sequence: 0,
            receipts: Vec::new(),
            ledger_sha256,
            capture_sequences: BTreeSet::new(),
            event_roots: BTreeSet::new(),
            request_roots: BTreeSet::new(),
            capture_receipt_roots: BTreeSet::new(),
            traffic_receipt_roots: BTreeSet::new(),
            verifier_receipt_roots: BTreeSet::new(),
        })
    }

    pub fn append(
        &mut self,
        capture_index: &ProviderCaptureIndexV3,
        input: GenerationShadowReceiptInputV3<'_>,
    ) -> Result<&GenerationShadowReceiptV3, GenerationShadowLedgerErrorV3> {
        if self.receipts.len() >= GENERATION_SHADOW_LEDGER_MAX_RECORDS_V3 {
            return Err(GenerationShadowLedgerErrorV3::BudgetExhausted);
        }
        let capture = input.capture_receipt;
        if !capture_index.contains_exact(
            capture.capture_sequence(),
            capture.event_root_sha256(),
            capture.request_root_sha256(),
            capture.receipt_sha256(),
        ) {
            return Err(GenerationShadowLedgerErrorV3::InvalidCaptureJoin);
        }
        if !valid_nonzero_sha256(input.traffic_receipt_sha256) {
            return Err(GenerationShadowLedgerErrorV3::InvalidTrafficReceipt);
        }
        if input.traffic_generation_sequence == 0
            || input.traffic_generation_id_sha256 != self.generation_id_sha256
            || input.traffic_request_sha256 != capture.request_root_sha256().to_hex()
            || !valid_nonzero_sha256(input.traffic_index_sha256)
            || input.traffic_verdict_code == 0
            || input
                .traffic_phase_report_sha256
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || input
                .traffic_operator_receipt_sha256
                .is_some_and(|root| !valid_nonzero_sha256(root))
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidTrafficReceipt);
        }
        validate_phase_control_input(&input)?;
        if self
            .receipts
            .last()
            .is_some_and(|last| capture.capture_sequence() <= last.capture_sequence())
        {
            return Err(GenerationShadowLedgerErrorV3::NonMonotonicCapture);
        }
        self.ensure_unique(capture, input.traffic_receipt_sha256, input.f6_receipt)?;
        let verifier = validate_outcome(input.outcome, input.parity_mismatch, input.f6_receipt)?;
        let ordinal = u32::try_from(self.receipts.len())
            .map_err(|_| GenerationShadowLedgerErrorV3::BudgetExhausted)?;
        let previous_receipt_sha256 = self
            .receipts
            .last()
            .map(|receipt| receipt.receipt_sha256.clone())
            .unwrap_or(self.ledger_genesis_sha256()?);
        let actor_action_sha256 =
            verifier.map(|receipt| receipt.actor_physical_action_sha256().to_owned());
        let actor_output_sha256 = verifier.map(|receipt| receipt.actor_output_sha256().to_owned());
        let verifier_receipt_sha256 = verifier.map(|receipt| receipt.receipt_sha256().to_owned());
        if verifier.is_some_and(|receipt| {
            receipt.request_sha256() != capture.request_root_sha256().to_hex()
        }) {
            return Err(GenerationShadowLedgerErrorV3::InvalidVerifierReceipt);
        }
        let mut receipt = GenerationShadowReceiptV3 {
            ordinal,
            previous_receipt_sha256,
            generation_id_sha256: self.generation_id_sha256.clone(),
            generation_publish_sequence: self.generation_publish_sequence,
            generation_checkpoint_sha256: self.generation_checkpoint_sha256.clone(),
            capture_index_sha256: capture_index.index_sha256(),
            capture_sequence: capture.capture_sequence(),
            capture_event_sha256: capture.event_root_sha256(),
            request_sha256: capture.request_root_sha256(),
            capture_receipt_sha256: capture.receipt_sha256(),
            traffic_receipt_sha256: input.traffic_receipt_sha256.to_owned(),
            traffic_generation_sequence: input.traffic_generation_sequence,
            traffic_generation_id_sha256: input.traffic_generation_id_sha256.to_owned(),
            traffic_index_sha256: input.traffic_index_sha256.to_owned(),
            traffic_request_sha256: input.traffic_request_sha256.to_owned(),
            traffic_verdict_code: input.traffic_verdict_code,
            traffic_phase_report_sha256: input.traffic_phase_report_sha256.map(str::to_owned),
            traffic_operator_receipt_sha256: input
                .traffic_operator_receipt_sha256
                .map(str::to_owned),
            phase_control_evidence: input.phase_control_evidence.cloned(),
            actor_action_sha256,
            actor_output_sha256,
            verifier_receipt_sha256,
            verifier_receipt: verifier.cloned(),
            outcome: input.outcome,
            parity_mismatch: input.parity_mismatch,
            semantic_updates: u8::from(input.outcome.permits_positive_evidence()),
            raw_payloads_persisted: 0,
            local_accepts: 0,
            execution_authority: false,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt_digest(&receipt)?;
        receipt.validate_fields()?;
        self.capture_sequences.insert(capture.capture_sequence());
        self.event_roots.insert(capture.event_root_sha256());
        self.request_roots.insert(capture.request_root_sha256());
        self.capture_receipt_roots.insert(capture.receipt_sha256());
        self.traffic_receipt_roots
            .insert(input.traffic_receipt_sha256.to_owned());
        if let Some(root) = &receipt.verifier_receipt_sha256 {
            self.verifier_receipt_roots.insert(root.clone());
        }
        self.publish_sequence = self
            .publish_sequence
            .checked_add(1)
            .ok_or(GenerationShadowLedgerErrorV3::BudgetExhausted)?;
        self.receipts.push(receipt);
        self.ledger_sha256 = ledger_digest(
            &self.generation_id_sha256,
            self.generation_publish_sequence,
            &self.generation_checkpoint_sha256,
            self.publish_sequence,
            &self.receipts,
        )?;
        self.receipts
            .last()
            .ok_or(GenerationShadowLedgerErrorV3::InvalidLedger)
    }

    fn ensure_unique(
        &self,
        capture: &crate::ProviderRequestCaptureReceiptV3,
        traffic_receipt_sha256: &str,
        verifier: Option<
            &nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3,
        >,
    ) -> Result<(), GenerationShadowLedgerErrorV3> {
        if self.capture_sequences.contains(&capture.capture_sequence())
            || self.event_roots.contains(&capture.event_root_sha256())
            || self.request_roots.contains(&capture.request_root_sha256())
            || self
                .capture_receipt_roots
                .contains(&capture.receipt_sha256())
            || self.traffic_receipt_roots.contains(traffic_receipt_sha256)
            || verifier.is_some_and(|receipt| {
                self.verifier_receipt_roots
                    .contains(receipt.receipt_sha256())
            })
        {
            return Err(GenerationShadowLedgerErrorV3::DuplicateCommitment);
        }
        Ok(())
    }

    fn ledger_genesis_sha256(&self) -> Result<String, GenerationShadowLedgerErrorV3> {
        canonical_json_sha256(&(
            GENERATION_SHADOW_LEDGER_SCHEMA_V3,
            "genesis",
            self.generation_id_sha256.as_str(),
            self.generation_publish_sequence,
            self.generation_checkpoint_sha256.as_str(),
        ))
        .map_err(|_| GenerationShadowLedgerErrorV3::Serialization)
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
    pub const fn publish_sequence(&self) -> u64 {
        self.publish_sequence
    }

    #[must_use]
    pub fn receipts(&self) -> &[GenerationShadowReceiptV3] {
        &self.receipts
    }

    #[must_use]
    pub fn ledger_sha256(&self) -> &str {
        &self.ledger_sha256
    }

    #[must_use]
    pub const fn raw_payloads_persisted(&self) -> u8 {
        0
    }

    #[must_use]
    pub const fn execution_authority(&self) -> bool {
        false
    }
}

fn validate_outcome(
    outcome: GenerationShadowTerminalOutcomeV3,
    parity_mismatch: bool,
    verifier: Option<&nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3>,
) -> Result<
    Option<&nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3>,
    GenerationShadowLedgerErrorV3,
> {
    if let Some(receipt) = verifier {
        let bytes = receipt
            .canonical_bytes()
            .map_err(|_| GenerationShadowLedgerErrorV3::InvalidVerifierReceipt)?;
        let restored = nando_operator_proof::independent_verifier_v3::IndependentVerifierReceiptV3::from_canonical_bytes(&bytes)
            .map_err(|_| GenerationShadowLedgerErrorV3::InvalidVerifierReceipt)?;
        if &restored != receipt
            || receipt.execution_authority()
            || receipt.raw_payloads_persisted() != 0
        {
            return Err(GenerationShadowLedgerErrorV3::InvalidVerifierReceipt);
        }
    }
    let verdict = verifier.map(|receipt| receipt.verdict());
    let valid = match outcome {
        GenerationShadowTerminalOutcomeV3::VerifiedPass => {
            !parity_mismatch && verdict == Some(IndependentVerifierVerdictV3::Verified)
        }
        GenerationShadowTerminalOutcomeV3::RuntimeAbstain
        | GenerationShadowTerminalOutcomeV3::RuntimeReject
        | GenerationShadowTerminalOutcomeV3::Censored => verifier.is_none(),
        GenerationShadowTerminalOutcomeV3::VerifierAbstain => {
            verdict.is_some_and(is_verifier_abstain)
        }
        GenerationShadowTerminalOutcomeV3::VerifierReject => verdict.is_some_and(|verdict| {
            verdict != IndependentVerifierVerdictV3::Verified && !is_verifier_abstain(verdict)
        }),
    };
    valid
        .then_some(verifier)
        .ok_or(GenerationShadowLedgerErrorV3::OutcomeMismatch)
}

const fn is_verifier_abstain(verdict: IndependentVerifierVerdictV3) -> bool {
    matches!(
        verdict,
        IndependentVerifierVerdictV3::AbstainUnsupportedProjection
            | IndependentVerifierVerdictV3::AbstainBudgetExhausted
            | IndependentVerifierVerdictV3::AbstainMissingRole
            | IndependentVerifierVerdictV3::AbstainMissingCapability
            | IndependentVerifierVerdictV3::AbstainAmbiguousCandidate
            | IndependentVerifierVerdictV3::AbstainUnsupportedEffect
    )
}

pub(super) fn receipt_digest(
    receipt: &GenerationShadowReceiptV3,
) -> Result<String, GenerationShadowLedgerErrorV3> {
    canonical_json_sha256(&(
        GENERATION_SHADOW_LEDGER_SCHEMA_V3,
        "receipt",
        (
            receipt.ordinal,
            receipt.previous_receipt_sha256.as_str(),
            receipt.generation_id_sha256.as_str(),
            receipt.generation_publish_sequence,
            receipt.generation_checkpoint_sha256.as_str(),
        ),
        (
            receipt.capture_index_sha256,
            receipt.capture_sequence,
            receipt.capture_event_sha256,
            receipt.request_sha256,
            receipt.capture_receipt_sha256,
        ),
        (
            receipt.traffic_receipt_sha256.as_str(),
            receipt.traffic_generation_sequence,
            receipt.traffic_generation_id_sha256.as_str(),
            receipt.traffic_index_sha256.as_str(),
            receipt.traffic_request_sha256.as_str(),
            receipt.traffic_verdict_code,
            receipt.traffic_phase_report_sha256.as_deref(),
            receipt.traffic_operator_receipt_sha256.as_deref(),
        ),
        receipt.phase_control_evidence.as_ref(),
        (
            receipt.actor_action_sha256.as_deref(),
            receipt.actor_output_sha256.as_deref(),
            receipt.verifier_receipt_sha256.as_deref(),
        ),
        (
            receipt.outcome,
            receipt.parity_mismatch,
            receipt.semantic_updates,
            0_u8,
            0_u8,
            false,
        ),
    ))
    .map_err(|_| GenerationShadowLedgerErrorV3::Serialization)
}

fn validate_phase_control_input(
    input: &GenerationShadowReceiptInputV3<'_>,
) -> Result<(), GenerationShadowLedgerErrorV3> {
    match (
        input.traffic_phase_report_sha256,
        input.phase_control_evidence,
    ) {
        (None, None) => Ok(()),
        (Some(report), Some(evidence))
            if evidence.report_sha256() == report
                && evidence.index_sha256() == input.traffic_index_sha256
                && evidence.raw_payloads_persisted() == 0
                && !evidence.execution_authority() =>
        {
            let bytes = evidence
                .canonical_bytes()
                .map_err(|_| GenerationShadowLedgerErrorV3::InvalidPhaseControlEvidence)?;
            let restored =
                nando_operator_kernel::RuntimePhaseControlEvidenceV3::from_canonical_bytes(&bytes)
                    .map_err(|_| GenerationShadowLedgerErrorV3::InvalidPhaseControlEvidence)?;
            (restored == *evidence)
                .then_some(())
                .ok_or(GenerationShadowLedgerErrorV3::InvalidPhaseControlEvidence)
        }
        _ => Err(GenerationShadowLedgerErrorV3::InvalidPhaseControlEvidence),
    }
}

pub(super) fn ledger_digest(
    generation_id_sha256: &str,
    generation_publish_sequence: u64,
    generation_checkpoint_sha256: &str,
    publish_sequence: u64,
    receipts: &[GenerationShadowReceiptV3],
) -> Result<String, GenerationShadowLedgerErrorV3> {
    let receipt_roots = receipts
        .iter()
        .map(|receipt| receipt.receipt_sha256.as_str())
        .collect::<Vec<_>>();
    canonical_json_sha256(&(
        GENERATION_SHADOW_LEDGER_SCHEMA_V3,
        "ledger",
        generation_id_sha256,
        generation_publish_sequence,
        generation_checkpoint_sha256,
        publish_sequence,
        receipt_roots,
        0_u8,
        false,
    ))
    .map_err(|_| GenerationShadowLedgerErrorV3::Serialization)
}
