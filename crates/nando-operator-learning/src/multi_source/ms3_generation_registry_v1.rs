use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    FrozenVersionSpaceEnvelopeV1, MS3_CENSORED_INELIGIBLE_PROBE,
    MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH, MS3_CENSORED_UNATTRIBUTED_PROBE,
    MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL, MS3_LINKED_FRAME_ACQUISITION_FAIL,
    Ms3FutureApplicabilityReportV1, Ms3FutureApplicabilityVerdictV1,
    Ms3IndependentFutureEnvelopeV1, Ms3IndependentFutureVerdictV1,
    Ms3LinkedFrameAcquisitionReportV1, Ms3LinkedFrameAcquisitionVerdictV1,
};

pub const MS3_GENERATION_REGISTRY_SCHEMA_V1: &str = "nando.ms3-generation-registry.v1";
pub const MS3_GENERATION_TERMINAL_SCHEMA_V1: &str = "nando.ms3-generation-terminal.v1";
pub const MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1: &str =
    "nando.ms3-generation-acquisition-failure.v1";
pub const MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1: &str =
    "nando.ms3-generation-linked-acquisition-failure.v1";
pub const MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2: &str =
    "nando.ms3-generation-linked-acquisition-failure.v2";
pub const MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3: &str =
    "nando.ms3-generation-linked-acquisition-failure.v3";
pub const MS3_CAPTURE_GAP_REPAIR_REQUIRED: &str = "ms3_capture_gap_repair_required";
pub const MS3_LINKED_EVIDENCE_REUSE: &str = "MS3_LINKED_EVIDENCE_REUSE";
const MAX_MS3_GENERATION_REGISTRY_BYTES: usize = 12 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3GenerationTerminalReceiptV1 {
    pub schema: String,
    pub terminal_root_sha256: String,
    pub generation_sequence: u64,
    pub frozen_contract_root_sha256: String,
    pub future_receipt_root_sha256: String,
    pub future_capture_sequence: u64,
    pub future_topology_root_sha256: String,
    pub future_completed_frame_root_sha256: String,
    pub future_session_lineage_sha256: String,
    pub verdict: Ms3IndependentFutureVerdictV1,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3GenerationAcquisitionFailureReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub generation_sequence: u64,
    pub frozen_contract_root_sha256: String,
    pub applicability_contract_root_sha256: String,
    pub applicability_ledger_root_sha256: String,
    pub applicability_report_root_sha256: String,
    pub terminal_capture_sequence: u64,
    pub independent_topologies: u64,
    pub generated_at_unix: u64,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
    pub schema: String,
    pub receipt_root_sha256: String,
    pub generation_sequence: u64,
    pub acquisition_contract_root_sha256: String,
    pub acquisition_report_root_sha256: String,
    pub topology_prefix_root_sha256: String,
    pub topology_watermark_rows: u64,
    pub evaluated_topology_rows: u64,
    pub terminal_receipt_rows: u64,
    pub closure_capture_sequence: u64,
    pub generated_at_unix: u64,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub raw_scanned_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub eligible_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub censored_unattributed_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub censored_topology_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub censored_pre_route_receipt_rows: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub consumed_topology_cursor_rows: u64,
}

#[derive(Serialize)]
struct Ms3GenerationLinkedAcquisitionClosureDigestV2<'a> {
    schema: &'static str,
    generation_sequence: u64,
    acquisition_contract_root_sha256: &'a str,
    acquisition_report_root_sha256: &'a str,
    topology_prefix_root_sha256: &'a str,
    topology_watermark_rows: u64,
    evaluated_topology_rows: u64,
    terminal_receipt_rows: u64,
    closure_capture_sequence: u64,
    generated_at_unix: u64,
    blocker: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
    raw_scanned_topology_rows: u64,
    eligible_topology_rows: u64,
    censored_unattributed_rows: u64,
    censored_topology_rows: u64,
    consumed_topology_cursor_rows: u64,
}

#[derive(Serialize)]
struct Ms3GenerationLinkedAcquisitionClosureDigestV3<'a> {
    schema: &'static str,
    generation_sequence: u64,
    acquisition_contract_root_sha256: &'a str,
    acquisition_report_root_sha256: &'a str,
    topology_prefix_root_sha256: &'a str,
    topology_watermark_rows: u64,
    evaluated_topology_rows: u64,
    terminal_receipt_rows: u64,
    closure_capture_sequence: u64,
    generated_at_unix: u64,
    blocker: &'a str,
    authority_ready: bool,
    phase_mutation_allowed: bool,
    raw_scanned_topology_rows: u64,
    eligible_topology_rows: u64,
    censored_unattributed_rows: u64,
    censored_topology_rows: u64,
    censored_pre_route_receipt_rows: u64,
    consumed_topology_cursor_rows: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3GenerationEntryV1 {
    pub generation_sequence: u64,
    pub frozen_envelope_root_sha256: String,
    pub frozen_contract_root_sha256: String,
    pub support_rows_root_sha256: String,
    pub topology_root_sha256: String,
    pub frame_root_sha256: String,
    pub session_lineage_sha256: String,
    pub support_watermark: u64,
    pub future_min_sequence: u64,
    pub terminal: Option<Ms3GenerationTerminalReceiptV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acquisition_failure: Option<Ms3GenerationAcquisitionFailureReceiptV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3GenerationRegistryV1 {
    pub schema: String,
    pub registry_root_sha256: String,
    pub generations: Vec<Ms3GenerationEntryV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_acquisition_failures: Vec<Ms3GenerationLinkedAcquisitionFailureReceiptV1>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ms3GenerationRegistryErrorV1 {
    InvalidRegistry,
    InvalidFrozenEnvelope,
    ActiveGenerationExists,
    TerminalGenerationMissing,
    SuccessorAfterPass,
    EvidenceReuse,
    InvalidFuture,
    Serialization,
}

impl Ms3GenerationRegistryV1 {
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            schema: MS3_GENERATION_REGISTRY_SCHEMA_V1.to_owned(),
            registry_root_sha256: String::new(),
            generations: Vec::new(),
            linked_acquisition_failures: Vec::new(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        registry.registry_root_sha256 =
            registry.expected_root().expect("empty registry serializes");
        registry
    }

    pub fn append_generation(
        &mut self,
        frozen: &FrozenVersionSpaceEnvelopeV1,
    ) -> Result<u64, Ms3GenerationRegistryErrorV1> {
        frozen
            .validate()
            .map_err(|_| Ms3GenerationRegistryErrorV1::InvalidFrozenEnvelope)?;
        let generation_sequence = self.next_generation_sequence();
        self.require_successor_allowed(generation_sequence)?;
        if self.generations.iter().any(|entry| {
            entry.frozen_envelope_root_sha256 == frozen.envelope_root_sha256
                || entry.frozen_contract_root_sha256 == frozen.contract.contract_root_sha256
                || entry.support_rows_root_sha256 == frozen.contract.support_rows_root_sha256
                || evidence_was_used(entry, frozen)
        }) {
            return Err(Ms3GenerationRegistryErrorV1::EvidenceReuse);
        }
        if generation_sequence > 1 {
            let closure_sequence = self
                .generation_closure_capture_sequence(generation_sequence - 1)
                .ok_or(Ms3GenerationRegistryErrorV1::TerminalGenerationMissing)?;
            if frozen.contract.support_watermark <= closure_sequence
                || frozen.contract.future_min_sequence <= closure_sequence
            {
                return Err(Ms3GenerationRegistryErrorV1::EvidenceReuse);
            }
        }
        self.generations.push(Ms3GenerationEntryV1 {
            generation_sequence,
            frozen_envelope_root_sha256: frozen.envelope_root_sha256.clone(),
            frozen_contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
            support_rows_root_sha256: frozen.contract.support_rows_root_sha256.clone(),
            topology_root_sha256: frozen.contract.topology_root_sha256.clone(),
            frame_root_sha256: frozen.contract.frame_root_sha256.clone(),
            session_lineage_sha256: frozen.contract.session_lineage_sha256.clone(),
            support_watermark: frozen.contract.support_watermark,
            future_min_sequence: frozen.contract.future_min_sequence,
            terminal: None,
            acquisition_failure: None,
        });
        self.reseal()?;
        Ok(generation_sequence)
    }

    pub fn seal_linked_acquisition_failure(
        &mut self,
        generation_sequence: u64,
        report: &Ms3LinkedFrameAcquisitionReportV1,
        closure_capture_sequence: u64,
    ) -> Result<Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        if !report.validate()
            || report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::AcquisitionFail
            || report.blocker != MS3_LINKED_FRAME_ACQUISITION_FAIL
            || generation_sequence == 0
            || closure_capture_sequence == 0
            || report.consumed_capture_sequence > 0
                && closure_capture_sequence != report.consumed_capture_sequence
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(existing) = self
            .linked_acquisition_failures
            .iter()
            .find(|receipt| receipt.generation_sequence == generation_sequence)
        {
            return (existing.acquisition_contract_root_sha256
                == report.acquisition_contract.contract_root_sha256
                && existing.acquisition_report_root_sha256 == report.report_root_sha256)
                .then_some(existing.clone())
                .ok_or(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if generation_sequence != self.next_generation_sequence()
            || self
                .generations
                .iter()
                .any(|entry| entry.generation_sequence == generation_sequence)
        {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        self.require_successor_allowed(generation_sequence)?;
        let use_cursor_receipt = report.consumed_topology_cursor_rows > 0;
        let mut receipt = Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
            schema: if use_cursor_receipt {
                MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2
            } else {
                MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1
            }
            .to_owned(),
            receipt_root_sha256: String::new(),
            generation_sequence,
            acquisition_contract_root_sha256: report
                .acquisition_contract
                .contract_root_sha256
                .clone(),
            acquisition_report_root_sha256: report.report_root_sha256.clone(),
            topology_prefix_root_sha256: report
                .acquisition_contract
                .topology_prefix_root_sha256
                .clone(),
            topology_watermark_rows: report.acquisition_contract.topology_watermark_rows,
            evaluated_topology_rows: report.evaluated_topology_rows,
            terminal_receipt_rows: report.terminal_receipt_rows,
            closure_capture_sequence,
            generated_at_unix: report.generated_at_unix,
            blocker: report.blocker.clone(),
            authority_ready: false,
            phase_mutation_allowed: false,
            raw_scanned_topology_rows: if use_cursor_receipt {
                report.raw_scanned_topology_rows
            } else {
                0
            },
            eligible_topology_rows: if use_cursor_receipt {
                report.eligible_topology_rows
            } else {
                0
            },
            censored_unattributed_rows: if use_cursor_receipt {
                report.censored_unattributed_rows
            } else {
                0
            },
            censored_topology_rows: if use_cursor_receipt {
                report.censored_topology_rows
            } else {
                0
            },
            censored_pre_route_receipt_rows: 0,
            consumed_topology_cursor_rows: if use_cursor_receipt {
                report.consumed_topology_cursor_rows
            } else {
                0
            },
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        self.linked_acquisition_failures.push(receipt.clone());
        self.reseal()?;
        Ok(receipt)
    }

    pub fn seal_unattributed_probe_censor(
        &mut self,
        generation_sequence: u64,
        report: &Ms3LinkedFrameAcquisitionReportV1,
        closure_capture_sequence: u64,
    ) -> Result<Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        if report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        self.seal_ineligible_probe_censor(generation_sequence, report, closure_capture_sequence)
    }

    pub fn seal_ineligible_probe_censor(
        &mut self,
        generation_sequence: u64,
        report: &Ms3LinkedFrameAcquisitionReportV1,
        closure_capture_sequence: u64,
    ) -> Result<Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        let censor_matches = matches!(
            (
                report.verdict,
                report.blocker.as_str(),
                report.censored_topology_rows
            ),
            (
                Ms3LinkedFrameAcquisitionVerdictV1::CensoredUnattributedProbe,
                MS3_CENSORED_UNATTRIBUTED_PROBE,
                0
            ) | (
                Ms3LinkedFrameAcquisitionVerdictV1::CensoredIneligibleProbe,
                MS3_CENSORED_INELIGIBLE_PROBE,
                1..
            ) | (
                Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch,
                MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH,
                1..
            )
        );
        if !report.validate()
            || !censor_matches
            || generation_sequence == 0
            || closure_capture_sequence == 0
            || closure_capture_sequence != report.consumed_capture_sequence
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(existing) = self
            .linked_acquisition_failures
            .iter()
            .find(|receipt| receipt.generation_sequence == generation_sequence)
        {
            return (existing.acquisition_contract_root_sha256
                == report.acquisition_contract.contract_root_sha256
                && existing.acquisition_report_root_sha256 == report.report_root_sha256
                && existing.blocker == report.blocker)
                .then_some(existing.clone())
                .ok_or(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if generation_sequence != self.next_generation_sequence()
            || self
                .generations
                .iter()
                .any(|entry| entry.generation_sequence == generation_sequence)
        {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        self.require_successor_allowed(generation_sequence)?;
        let censored_pre_route_receipt_rows = report
            .ineligible_reason_counts
            .get(&super::MultiSourceJoinCensoredReasonV1::TerminalReceiptUnavailable)
            .copied()
            .unwrap_or(0);
        let mut receipt = Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
            schema: if report.verdict
                == Ms3LinkedFrameAcquisitionVerdictV1::CensoredPreRouteReceiptEpoch
            {
                MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3
            } else {
                MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2
            }
            .to_owned(),
            receipt_root_sha256: String::new(),
            generation_sequence,
            acquisition_contract_root_sha256: report
                .acquisition_contract
                .contract_root_sha256
                .clone(),
            acquisition_report_root_sha256: report.report_root_sha256.clone(),
            topology_prefix_root_sha256: report
                .acquisition_contract
                .topology_prefix_root_sha256
                .clone(),
            topology_watermark_rows: report.acquisition_contract.topology_watermark_rows,
            evaluated_topology_rows: report.evaluated_topology_rows,
            terminal_receipt_rows: report.terminal_receipt_rows,
            closure_capture_sequence,
            generated_at_unix: report.generated_at_unix,
            blocker: report.blocker.clone(),
            authority_ready: false,
            phase_mutation_allowed: false,
            raw_scanned_topology_rows: report.raw_scanned_topology_rows,
            eligible_topology_rows: report.eligible_topology_rows,
            censored_unattributed_rows: report.censored_unattributed_rows,
            censored_topology_rows: report.censored_topology_rows,
            censored_pre_route_receipt_rows,
            consumed_topology_cursor_rows: report.consumed_topology_cursor_rows,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        self.linked_acquisition_failures.push(receipt.clone());
        self.reseal()?;
        Ok(receipt)
    }

    pub fn seal_linked_capture_gap_repair(
        &mut self,
        generation_sequence: u64,
        report: &Ms3LinkedFrameAcquisitionReportV1,
        closure_capture_sequence: u64,
    ) -> Result<Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        if !report.validate()
            || report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved
            || report.receipts.is_empty()
            || report.receipts.iter().any(|receipt| {
                receipt.gap_class != Some(super::RepresentationGapClassV1::CaptureGapA)
            })
            || generation_sequence == 0
            || closure_capture_sequence == 0
            || report.consumed_capture_sequence > 0
                && closure_capture_sequence != report.consumed_capture_sequence
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(existing) = self
            .linked_acquisition_failures
            .iter()
            .find(|receipt| receipt.generation_sequence == generation_sequence)
        {
            return (existing.acquisition_contract_root_sha256
                == report.acquisition_contract.contract_root_sha256
                && existing.acquisition_report_root_sha256 == report.report_root_sha256
                && existing.blocker == MS3_CAPTURE_GAP_REPAIR_REQUIRED)
                .then_some(existing.clone())
                .ok_or(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if generation_sequence != self.next_generation_sequence()
            || self
                .generations
                .iter()
                .any(|entry| entry.generation_sequence == generation_sequence)
        {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        self.require_successor_allowed(generation_sequence)?;
        let mut receipt = Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
            schema: MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            generation_sequence,
            acquisition_contract_root_sha256: report
                .acquisition_contract
                .contract_root_sha256
                .clone(),
            acquisition_report_root_sha256: report.report_root_sha256.clone(),
            topology_prefix_root_sha256: report
                .acquisition_contract
                .topology_prefix_root_sha256
                .clone(),
            topology_watermark_rows: report.acquisition_contract.topology_watermark_rows,
            evaluated_topology_rows: report.evaluated_topology_rows,
            terminal_receipt_rows: report.terminal_receipt_rows,
            closure_capture_sequence,
            generated_at_unix: report.generated_at_unix,
            blocker: MS3_CAPTURE_GAP_REPAIR_REQUIRED.to_owned(),
            authority_ready: false,
            phase_mutation_allowed: false,
            raw_scanned_topology_rows: 0,
            eligible_topology_rows: 0,
            censored_unattributed_rows: 0,
            censored_topology_rows: 0,
            censored_pre_route_receipt_rows: 0,
            consumed_topology_cursor_rows: 0,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        self.linked_acquisition_failures.push(receipt.clone());
        self.reseal()?;
        Ok(receipt)
    }

    pub fn seal_linked_evidence_reuse(
        &mut self,
        generation_sequence: u64,
        report: &Ms3LinkedFrameAcquisitionReportV1,
        closure_capture_sequence: u64,
    ) -> Result<Ms3GenerationLinkedAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        if !report.validate()
            || report.verdict != Ms3LinkedFrameAcquisitionVerdictV1::LinkedFrameObserved
            || report.receipts.is_empty()
            || !report
                .receipts
                .iter()
                .all(|receipt| self.linked_evidence_was_used(receipt))
            || generation_sequence == 0
            || closure_capture_sequence == 0
            || report.consumed_capture_sequence > 0
                && closure_capture_sequence != report.consumed_capture_sequence
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(existing) = self
            .linked_acquisition_failures
            .iter()
            .find(|receipt| receipt.generation_sequence == generation_sequence)
        {
            return (existing.acquisition_contract_root_sha256
                == report.acquisition_contract.contract_root_sha256
                && existing.acquisition_report_root_sha256 == report.report_root_sha256
                && existing.blocker == MS3_LINKED_EVIDENCE_REUSE)
                .then_some(existing.clone())
                .ok_or(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if generation_sequence != self.next_generation_sequence()
            || self
                .generations
                .iter()
                .any(|entry| entry.generation_sequence == generation_sequence)
        {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        self.require_successor_allowed(generation_sequence)?;
        let mut receipt = Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
            schema: MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            generation_sequence,
            acquisition_contract_root_sha256: report
                .acquisition_contract
                .contract_root_sha256
                .clone(),
            acquisition_report_root_sha256: report.report_root_sha256.clone(),
            topology_prefix_root_sha256: report
                .acquisition_contract
                .topology_prefix_root_sha256
                .clone(),
            topology_watermark_rows: report.acquisition_contract.topology_watermark_rows,
            evaluated_topology_rows: report.evaluated_topology_rows,
            terminal_receipt_rows: report.terminal_receipt_rows,
            closure_capture_sequence,
            generated_at_unix: report.generated_at_unix,
            blocker: MS3_LINKED_EVIDENCE_REUSE.to_owned(),
            authority_ready: false,
            phase_mutation_allowed: false,
            raw_scanned_topology_rows: 0,
            eligible_topology_rows: 0,
            censored_unattributed_rows: 0,
            censored_topology_rows: 0,
            censored_pre_route_receipt_rows: 0,
            consumed_topology_cursor_rows: 0,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        self.linked_acquisition_failures.push(receipt.clone());
        self.reseal()?;
        Ok(receipt)
    }

    pub fn seal_terminal(
        &mut self,
        frozen: &FrozenVersionSpaceEnvelopeV1,
        future: &Ms3IndependentFutureEnvelopeV1,
    ) -> Result<Ms3GenerationTerminalReceiptV1, Ms3GenerationRegistryErrorV1> {
        future
            .validate(frozen)
            .map_err(|_| Ms3GenerationRegistryErrorV1::InvalidFuture)?;
        let entry_index = self
            .generations
            .len()
            .checked_sub(1)
            .ok_or(Ms3GenerationRegistryErrorV1::TerminalGenerationMissing)?;
        let entry = &self.generations[entry_index];
        if entry.frozen_envelope_root_sha256 != frozen.envelope_root_sha256
            || entry.frozen_contract_root_sha256 != frozen.contract.contract_root_sha256
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(terminal) = &entry.terminal {
            if terminal.future_receipt_root_sha256 == future.receipt.receipt_root_sha256 {
                return Ok(terminal.clone());
            }
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if entry.acquisition_failure.is_some() {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if self.future_evidence_was_used(future) {
            return Err(Ms3GenerationRegistryErrorV1::EvidenceReuse);
        }
        let mut terminal = Ms3GenerationTerminalReceiptV1 {
            schema: MS3_GENERATION_TERMINAL_SCHEMA_V1.to_owned(),
            terminal_root_sha256: String::new(),
            generation_sequence: entry.generation_sequence,
            frozen_contract_root_sha256: entry.frozen_contract_root_sha256.clone(),
            future_receipt_root_sha256: future.receipt.receipt_root_sha256.clone(),
            future_capture_sequence: future.receipt.capture_sequence,
            future_topology_root_sha256: future.receipt.topology_root_sha256.clone(),
            future_completed_frame_root_sha256: future.receipt.completed_frame_root_sha256.clone(),
            future_session_lineage_sha256: future.receipt.session_lineage_sha256.clone(),
            verdict: future.receipt.verdict,
            blocker: future.receipt.blocker.clone(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        terminal.terminal_root_sha256 = terminal.expected_root()?;
        self.generations[entry_index].terminal = Some(terminal.clone());
        self.reseal()?;
        Ok(terminal)
    }

    pub fn seal_acquisition_failure(
        &mut self,
        frozen: &FrozenVersionSpaceEnvelopeV1,
        report: &Ms3FutureApplicabilityReportV1,
        terminal_capture_sequence: u64,
    ) -> Result<Ms3GenerationAcquisitionFailureReceiptV1, Ms3GenerationRegistryErrorV1> {
        if !report.validate()
            || report.verdict != Ms3FutureApplicabilityVerdictV1::AcquisitionFail
            || report.blocker != MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        let entry_index = self
            .generations
            .len()
            .checked_sub(1)
            .ok_or(Ms3GenerationRegistryErrorV1::TerminalGenerationMissing)?;
        let entry = &self.generations[entry_index];
        if entry.frozen_envelope_root_sha256 != frozen.envelope_root_sha256
            || entry.frozen_contract_root_sha256 != frozen.contract.contract_root_sha256
            || report.contract.frozen_law_contract_root_sha256
                != frozen.contract.contract_root_sha256
            || terminal_capture_sequence < report.contract.opened_at_sequence
        {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if let Some(existing) = &entry.acquisition_failure {
            return (existing.applicability_ledger_root_sha256 == report.ledger_root_sha256)
                .then_some(existing.clone())
                .ok_or(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        if entry.terminal.is_some() {
            return Err(Ms3GenerationRegistryErrorV1::InvalidFuture);
        }
        let mut receipt = Ms3GenerationAcquisitionFailureReceiptV1 {
            schema: MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1.to_owned(),
            receipt_root_sha256: String::new(),
            generation_sequence: entry.generation_sequence,
            frozen_contract_root_sha256: entry.frozen_contract_root_sha256.clone(),
            applicability_contract_root_sha256: report.contract.contract_root_sha256.clone(),
            applicability_ledger_root_sha256: report.ledger_root_sha256.clone(),
            applicability_report_root_sha256: report.report_root_sha256.clone(),
            terminal_capture_sequence,
            independent_topologies: report.independent_topologies,
            generated_at_unix: report.generated_at_unix,
            blocker: report.blocker.clone(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.receipt_root_sha256 = receipt.expected_root()?;
        self.generations[entry_index].acquisition_failure = Some(receipt.clone());
        self.reseal()?;
        Ok(receipt)
    }

    #[must_use]
    pub fn future_evidence_was_used(&self, future: &Ms3IndependentFutureEnvelopeV1) -> bool {
        let future_roots = [
            future.receipt.topology_root_sha256.as_str(),
            future.receipt.completed_frame_root_sha256.as_str(),
            future.receipt.session_lineage_sha256.as_str(),
        ];
        self.evidence_roots_were_used(future_roots)
    }

    fn evidence_roots_were_used(&self, future_roots: [&str; 3]) -> bool {
        self.generations.iter().any(|entry| {
            let support_roots = [
                entry.topology_root_sha256.as_str(),
                entry.frame_root_sha256.as_str(),
                entry.session_lineage_sha256.as_str(),
            ];
            future_roots.iter().any(|root| support_roots.contains(root))
                || entry.terminal.as_ref().is_some_and(|terminal| {
                    let terminal_roots = [
                        terminal.future_topology_root_sha256.as_str(),
                        terminal.future_completed_frame_root_sha256.as_str(),
                        terminal.future_session_lineage_sha256.as_str(),
                    ];
                    future_roots
                        .iter()
                        .any(|root| terminal_roots.contains(root))
                })
        })
    }

    #[must_use]
    pub fn linked_evidence_was_used(&self, receipt: &super::Ms3LinkedFrameReceiptV1) -> bool {
        self.evidence_roots_were_used([
            receipt.topology_commitment_root_sha256.as_str(),
            receipt.completed_frame_root_sha256.as_str(),
            receipt.session_lineage_sha256.as_str(),
        ])
    }

    #[must_use]
    pub fn used_evidence_roots(&self) -> BTreeSet<String> {
        self.generations
            .iter()
            .flat_map(|entry| {
                let mut roots = vec![
                    entry.topology_root_sha256.clone(),
                    entry.frame_root_sha256.clone(),
                    entry.session_lineage_sha256.clone(),
                ];
                if let Some(terminal) = &entry.terminal {
                    roots.extend([
                        terminal.future_topology_root_sha256.clone(),
                        terminal.future_completed_frame_root_sha256.clone(),
                        terminal.future_session_lineage_sha256.clone(),
                    ]);
                }
                roots
            })
            .collect()
    }

    #[must_use]
    pub fn next_generation_sequence(&self) -> u64 {
        self.generations
            .iter()
            .map(|entry| entry.generation_sequence)
            .chain(
                self.linked_acquisition_failures
                    .iter()
                    .map(|receipt| receipt.generation_sequence),
            )
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    #[must_use]
    pub fn linked_acquisition_failure(
        &self,
        generation_sequence: u64,
    ) -> Option<&Ms3GenerationLinkedAcquisitionFailureReceiptV1> {
        self.linked_acquisition_failures
            .iter()
            .find(|receipt| receipt.generation_sequence == generation_sequence)
    }

    #[must_use]
    pub fn closure_root(&self, generation_sequence: u64) -> Option<&str> {
        self.generations
            .iter()
            .find(|entry| entry.generation_sequence == generation_sequence)
            .and_then(Ms3GenerationEntryV1::closure_root)
            .or_else(|| {
                self.linked_acquisition_failure(generation_sequence)
                    .map(|receipt| receipt.receipt_root_sha256.as_str())
            })
    }

    #[must_use]
    pub fn generation_is_open(&self, generation_sequence: u64) -> bool {
        self.generations
            .iter()
            .find(|entry| entry.generation_sequence == generation_sequence)
            .is_some_and(|entry| !entry.is_closed())
    }

    fn require_successor_allowed(
        &self,
        generation_sequence: u64,
    ) -> Result<(), Ms3GenerationRegistryErrorV1> {
        if generation_sequence == 1 {
            return Ok(());
        }
        let previous_sequence = generation_sequence - 1;
        if self
            .generations
            .iter()
            .find(|entry| entry.generation_sequence == previous_sequence)
            .is_some_and(|entry| {
                entry
                    .terminal
                    .as_ref()
                    .is_some_and(|terminal| terminal.verdict == Ms3IndependentFutureVerdictV1::Pass)
            })
        {
            return Err(Ms3GenerationRegistryErrorV1::SuccessorAfterPass);
        }
        if self.generation_is_open(previous_sequence) {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        if self.closure_root(previous_sequence).is_none() {
            return Err(Ms3GenerationRegistryErrorV1::TerminalGenerationMissing);
        }
        Ok(())
    }

    pub fn generation_closure_capture_sequence(&self, generation_sequence: u64) -> Option<u64> {
        self.generations
            .iter()
            .find(|entry| entry.generation_sequence == generation_sequence)
            .and_then(Ms3GenerationEntryV1::closure_capture_sequence)
            .or_else(|| {
                self.linked_acquisition_failure(generation_sequence)
                    .map(|receipt| receipt.closure_capture_sequence)
            })
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let roots_unique = {
            let envelope_roots = self
                .generations
                .iter()
                .map(|entry| entry.frozen_envelope_root_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let contract_roots = self
                .generations
                .iter()
                .map(|entry| entry.frozen_contract_root_sha256.as_str())
                .collect::<BTreeSet<_>>();
            let support_roots = self
                .generations
                .iter()
                .map(|entry| entry.support_rows_root_sha256.as_str())
                .collect::<BTreeSet<_>>();
            envelope_roots.len() == self.generations.len()
                && contract_roots.len() == self.generations.len()
                && support_roots.len() == self.generations.len()
        };
        let evidence_roots_unique = {
            let roots = self
                .generations
                .iter()
                .flat_map(|entry| {
                    let mut roots = vec![
                        entry.topology_root_sha256.as_str(),
                        entry.frame_root_sha256.as_str(),
                        entry.session_lineage_sha256.as_str(),
                    ];
                    if let Some(terminal) = &entry.terminal {
                        roots.extend([
                            terminal.future_topology_root_sha256.as_str(),
                            terminal.future_completed_frame_root_sha256.as_str(),
                            terminal.future_session_lineage_sha256.as_str(),
                        ]);
                    }
                    roots
                })
                .collect::<Vec<_>>();
            roots.iter().copied().collect::<BTreeSet<_>>().len() == roots.len()
        };
        let sequence_count = self
            .generations
            .len()
            .saturating_add(self.linked_acquisition_failures.len());
        let generation_sequences = self
            .generations
            .iter()
            .map(|entry| entry.generation_sequence)
            .chain(
                self.linked_acquisition_failures
                    .iter()
                    .map(|receipt| receipt.generation_sequence),
            )
            .collect::<BTreeSet<_>>();
        let sequences_are_contiguous = generation_sequences.len() == sequence_count
            && generation_sequences
                .iter()
                .copied()
                .eq(1..=u64::try_from(sequence_count).unwrap_or(u64::MAX));
        self.schema == MS3_GENERATION_REGISTRY_SCHEMA_V1
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && roots_unique
            && evidence_roots_unique
            && sequences_are_contiguous
            && self.generations.iter().all(|entry| {
                valid_nonzero_sha256(&entry.frozen_envelope_root_sha256)
                    && valid_nonzero_sha256(&entry.frozen_contract_root_sha256)
                    && valid_nonzero_sha256(&entry.support_rows_root_sha256)
                    && valid_nonzero_sha256(&entry.topology_root_sha256)
                    && valid_nonzero_sha256(&entry.frame_root_sha256)
                    && valid_nonzero_sha256(&entry.session_lineage_sha256)
                    && entry.support_watermark > 0
                    && entry.future_min_sequence > entry.support_watermark
                    && entry.terminal.as_ref().is_none_or(|terminal| {
                        terminal.validate()
                            && terminal.generation_sequence == entry.generation_sequence
                            && terminal.frozen_contract_root_sha256
                                == entry.frozen_contract_root_sha256
                            && terminal.future_capture_sequence >= entry.future_min_sequence
                    })
                    && entry.acquisition_failure.as_ref().is_none_or(|failure| {
                        failure.validate()
                            && failure.generation_sequence == entry.generation_sequence
                            && failure.frozen_contract_root_sha256
                                == entry.frozen_contract_root_sha256
                            && failure.terminal_capture_sequence >= entry.support_watermark
                    })
                    && !(entry.terminal.is_some() && entry.acquisition_failure.is_some())
            })
            && self
                .linked_acquisition_failures
                .iter()
                .all(Ms3GenerationLinkedAcquisitionFailureReceiptV1::validate)
            && (1..u64::try_from(sequence_count).unwrap_or(u64::MAX)).all(|sequence| {
                self.closure_root(sequence).is_some()
                    && self
                        .generations
                        .iter()
                        .find(|entry| entry.generation_sequence == sequence + 1)
                        .is_none_or(|next| {
                            self.generation_closure_capture_sequence(sequence)
                                .is_some_and(|closure| {
                                    next.support_watermark > closure
                                        && next.future_min_sequence > closure
                                })
                        })
            })
            && self
                .expected_root()
                .is_ok_and(|root| root == self.registry_root_sha256)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, Ms3GenerationRegistryErrorV1> {
        if !self.validate() {
            return Err(Ms3GenerationRegistryErrorV1::InvalidRegistry);
        }
        let bytes =
            serde_cbor::to_vec(self).map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)?;
        if bytes.is_empty() || bytes.len() > MAX_MS3_GENERATION_REGISTRY_BYTES {
            return Err(Ms3GenerationRegistryErrorV1::Serialization);
        }
        Ok(bytes)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, Ms3GenerationRegistryErrorV1> {
        if bytes.is_empty() || bytes.len() > MAX_MS3_GENERATION_REGISTRY_BYTES {
            return Err(Ms3GenerationRegistryErrorV1::InvalidRegistry);
        }
        let registry: Self = serde_cbor::from_slice(bytes)
            .map_err(|_| Ms3GenerationRegistryErrorV1::InvalidRegistry)?;
        if !registry.validate() || registry.canonical_bytes()? != bytes {
            return Err(Ms3GenerationRegistryErrorV1::InvalidRegistry);
        }
        Ok(registry)
    }

    fn reseal(&mut self) -> Result<(), Ms3GenerationRegistryErrorV1> {
        self.registry_root_sha256 = self.expected_root()?;
        self.validate()
            .then_some(())
            .ok_or(Ms3GenerationRegistryErrorV1::InvalidRegistry)
    }

    fn expected_root(&self) -> Result<String, Ms3GenerationRegistryErrorV1> {
        let root = if self.linked_acquisition_failures.is_empty() {
            canonical_json_sha256(&(
                MS3_GENERATION_REGISTRY_SCHEMA_V1,
                &self.generations,
                false,
                false,
            ))
        } else {
            canonical_json_sha256(&(
                MS3_GENERATION_REGISTRY_SCHEMA_V1,
                &self.generations,
                &self.linked_acquisition_failures,
                false,
                false,
            ))
        };
        root.map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)
    }
}

impl Default for Ms3GenerationRegistryV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Ms3GenerationTerminalReceiptV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_GENERATION_TERMINAL_SCHEMA_V1
            && self.generation_sequence > 0
            && valid_nonzero_sha256(&self.terminal_root_sha256)
            && valid_nonzero_sha256(&self.frozen_contract_root_sha256)
            && valid_nonzero_sha256(&self.future_receipt_root_sha256)
            && self.future_capture_sequence > 0
            && valid_nonzero_sha256(&self.future_topology_root_sha256)
            && valid_nonzero_sha256(&self.future_completed_frame_root_sha256)
            && valid_nonzero_sha256(&self.future_session_lineage_sha256)
            && match self.verdict {
                Ms3IndependentFutureVerdictV1::Pass => self.blocker.is_empty(),
                Ms3IndependentFutureVerdictV1::Contradiction => !self.blocker.is_empty(),
            }
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.terminal_root_sha256)
    }

    fn expected_root(&self) -> Result<String, Ms3GenerationRegistryErrorV1> {
        canonical_json_sha256(&(
            MS3_GENERATION_TERMINAL_SCHEMA_V1,
            self.generation_sequence,
            self.frozen_contract_root_sha256.as_str(),
            self.future_receipt_root_sha256.as_str(),
            self.future_capture_sequence,
            self.future_topology_root_sha256.as_str(),
            self.future_completed_frame_root_sha256.as_str(),
            self.future_session_lineage_sha256.as_str(),
            self.verdict,
            self.blocker.as_str(),
            false,
            false,
        ))
        .map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)
    }
}

impl Ms3GenerationAcquisitionFailureReceiptV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1
            && self.generation_sequence > 0
            && valid_nonzero_sha256(&self.receipt_root_sha256)
            && valid_nonzero_sha256(&self.frozen_contract_root_sha256)
            && valid_nonzero_sha256(&self.applicability_contract_root_sha256)
            && valid_nonzero_sha256(&self.applicability_ledger_root_sha256)
            && valid_nonzero_sha256(&self.applicability_report_root_sha256)
            && self.terminal_capture_sequence > 0
            && self.generated_at_unix > 0
            && self.blocker == MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.receipt_root_sha256)
    }

    fn expected_root(&self) -> Result<String, Ms3GenerationRegistryErrorV1> {
        canonical_json_sha256(&(
            MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1,
            self.generation_sequence,
            self.frozen_contract_root_sha256.as_str(),
            self.applicability_contract_root_sha256.as_str(),
            self.applicability_ledger_root_sha256.as_str(),
            self.applicability_report_root_sha256.as_str(),
            self.terminal_capture_sequence,
            self.independent_topologies,
            self.generated_at_unix,
            self.blocker.as_str(),
            false,
            false,
        ))
        .map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)
    }
}

impl Ms3GenerationLinkedAcquisitionFailureReceiptV1 {
    #[must_use]
    pub fn validate(&self) -> bool {
        let schema_v1 = self.schema == MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1;
        let schema_v2 = self.schema == MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2;
        let schema_v3 = self.schema == MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3;
        let cursor_receipt_valid = self.raw_scanned_topology_rows > 0
            && self.raw_scanned_topology_rows >= self.eligible_topology_rows
            && self.eligible_topology_rows == self.evaluated_topology_rows
            && self
                .censored_unattributed_rows
                .saturating_add(self.censored_topology_rows)
                == self
                    .raw_scanned_topology_rows
                    .saturating_sub(self.eligible_topology_rows)
            && self.consumed_topology_cursor_rows
                == self
                    .topology_watermark_rows
                    .saturating_add(self.raw_scanned_topology_rows);
        let blocker_valid = match self.blocker.as_str() {
            MS3_LINKED_FRAME_ACQUISITION_FAIL => {
                self.terminal_receipt_rows >= self.evaluated_topology_rows
                    && (schema_v1 || cursor_receipt_valid)
            }
            MS3_CAPTURE_GAP_REPAIR_REQUIRED | MS3_LINKED_EVIDENCE_REUSE => {
                self.terminal_receipt_rows > 0
            }
            MS3_CENSORED_UNATTRIBUTED_PROBE => {
                schema_v2
                    && self.raw_scanned_topology_rows > self.eligible_topology_rows
                    && self.censored_unattributed_rows > 0
                    && self.censored_topology_rows == 0
                    && self.terminal_receipt_rows >= self.eligible_topology_rows
                    && cursor_receipt_valid
            }
            MS3_CENSORED_INELIGIBLE_PROBE => {
                schema_v2
                    && self.raw_scanned_topology_rows > self.eligible_topology_rows
                    && self.censored_topology_rows > 0
                    && self.terminal_receipt_rows >= self.eligible_topology_rows
                    && cursor_receipt_valid
            }
            MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH => {
                schema_v3
                    && self.censored_topology_rows > 0
                    && self.censored_pre_route_receipt_rows > 0
                    && self.censored_pre_route_receipt_rows <= self.censored_topology_rows
                    && self.terminal_receipt_rows <= self.eligible_topology_rows
                    && cursor_receipt_valid
            }
            _ => false,
        };
        let schema_payload_valid = schema_v1
            && self.raw_scanned_topology_rows == 0
            && self.eligible_topology_rows == 0
            && self.censored_unattributed_rows == 0
            && self.censored_topology_rows == 0
            && self.censored_pre_route_receipt_rows == 0
            && self.consumed_topology_cursor_rows == 0
            || schema_v2
                && self.censored_pre_route_receipt_rows == 0
                && matches!(
                    self.blocker.as_str(),
                    MS3_LINKED_FRAME_ACQUISITION_FAIL
                        | MS3_CENSORED_UNATTRIBUTED_PROBE
                        | MS3_CENSORED_INELIGIBLE_PROBE
                )
            || schema_v3
                && self.blocker == MS3_CENSORED_PRE_ROUTE_RECEIPT_EPOCH
                && self.censored_pre_route_receipt_rows > 0;
        (schema_v1 || schema_v2 || schema_v3)
            && self.generation_sequence > 0
            && valid_nonzero_sha256(&self.receipt_root_sha256)
            && valid_nonzero_sha256(&self.acquisition_contract_root_sha256)
            && valid_nonzero_sha256(&self.acquisition_report_root_sha256)
            && valid_nonzero_sha256(&self.topology_prefix_root_sha256)
            && self.evaluated_topology_rows > 0
            && blocker_valid
            && schema_payload_valid
            && self.closure_capture_sequence > 0
            && self.generated_at_unix > 0
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.receipt_root_sha256)
    }

    fn expected_root(&self) -> Result<String, Ms3GenerationRegistryErrorV1> {
        let root = if self.schema == MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3 {
            canonical_json_sha256(&Ms3GenerationLinkedAcquisitionClosureDigestV3 {
                schema: MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V3,
                generation_sequence: self.generation_sequence,
                acquisition_contract_root_sha256: &self.acquisition_contract_root_sha256,
                acquisition_report_root_sha256: &self.acquisition_report_root_sha256,
                topology_prefix_root_sha256: &self.topology_prefix_root_sha256,
                topology_watermark_rows: self.topology_watermark_rows,
                evaluated_topology_rows: self.evaluated_topology_rows,
                terminal_receipt_rows: self.terminal_receipt_rows,
                closure_capture_sequence: self.closure_capture_sequence,
                generated_at_unix: self.generated_at_unix,
                blocker: &self.blocker,
                authority_ready: false,
                phase_mutation_allowed: false,
                raw_scanned_topology_rows: self.raw_scanned_topology_rows,
                eligible_topology_rows: self.eligible_topology_rows,
                censored_unattributed_rows: self.censored_unattributed_rows,
                censored_topology_rows: self.censored_topology_rows,
                censored_pre_route_receipt_rows: self.censored_pre_route_receipt_rows,
                consumed_topology_cursor_rows: self.consumed_topology_cursor_rows,
            })
        } else if self.schema == MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2 {
            canonical_json_sha256(&Ms3GenerationLinkedAcquisitionClosureDigestV2 {
                schema: MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V2,
                generation_sequence: self.generation_sequence,
                acquisition_contract_root_sha256: &self.acquisition_contract_root_sha256,
                acquisition_report_root_sha256: &self.acquisition_report_root_sha256,
                topology_prefix_root_sha256: &self.topology_prefix_root_sha256,
                topology_watermark_rows: self.topology_watermark_rows,
                evaluated_topology_rows: self.evaluated_topology_rows,
                terminal_receipt_rows: self.terminal_receipt_rows,
                closure_capture_sequence: self.closure_capture_sequence,
                generated_at_unix: self.generated_at_unix,
                blocker: &self.blocker,
                authority_ready: false,
                phase_mutation_allowed: false,
                raw_scanned_topology_rows: self.raw_scanned_topology_rows,
                eligible_topology_rows: self.eligible_topology_rows,
                censored_unattributed_rows: self.censored_unattributed_rows,
                censored_topology_rows: self.censored_topology_rows,
                consumed_topology_cursor_rows: self.consumed_topology_cursor_rows,
            })
        } else {
            canonical_json_sha256(&(
                MS3_GENERATION_LINKED_ACQUISITION_FAILURE_SCHEMA_V1,
                self.generation_sequence,
                self.acquisition_contract_root_sha256.as_str(),
                self.acquisition_report_root_sha256.as_str(),
                self.topology_prefix_root_sha256.as_str(),
                self.topology_watermark_rows,
                self.evaluated_topology_rows,
                self.terminal_receipt_rows,
                self.closure_capture_sequence,
                self.generated_at_unix,
                self.blocker.as_str(),
                false,
                false,
            ))
        };
        root.map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)
    }
}

const fn is_zero(value: &u64) -> bool {
    *value == 0
}

impl Ms3GenerationEntryV1 {
    fn is_closed(&self) -> bool {
        self.terminal.is_some() || self.acquisition_failure.is_some()
    }

    fn closure_capture_sequence(&self) -> Option<u64> {
        self.terminal
            .as_ref()
            .map(|terminal| terminal.future_capture_sequence)
            .or_else(|| {
                self.acquisition_failure
                    .as_ref()
                    .map(|failure| failure.terminal_capture_sequence)
            })
    }

    fn closure_root(&self) -> Option<&str> {
        self.terminal
            .as_ref()
            .filter(|terminal| terminal.verdict == Ms3IndependentFutureVerdictV1::Contradiction)
            .map(|terminal| terminal.terminal_root_sha256.as_str())
            .or_else(|| {
                self.acquisition_failure
                    .as_ref()
                    .map(|failure| failure.receipt_root_sha256.as_str())
            })
    }
}

fn evidence_was_used(entry: &Ms3GenerationEntryV1, frozen: &FrozenVersionSpaceEnvelopeV1) -> bool {
    let new_roots = [
        frozen.contract.topology_root_sha256.as_str(),
        frozen.contract.frame_root_sha256.as_str(),
        frozen.contract.session_lineage_sha256.as_str(),
    ];
    let support_roots = [
        entry.topology_root_sha256.as_str(),
        entry.frame_root_sha256.as_str(),
        entry.session_lineage_sha256.as_str(),
    ];
    new_roots
        .into_iter()
        .any(|root| support_roots.contains(&root))
        || entry.terminal.as_ref().is_some_and(|terminal| {
            let future_roots = [
                terminal.future_topology_root_sha256.as_str(),
                terminal.future_completed_frame_root_sha256.as_str(),
                terminal.future_session_lineage_sha256.as_str(),
            ];
            new_roots
                .into_iter()
                .any(|root| future_roots.contains(&root))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_is_canonical_and_non_authoritative() {
        let registry = Ms3GenerationRegistryV1::new();
        assert!(registry.validate());
        assert!(!registry.authority_ready);
        assert!(!registry.phase_mutation_allowed);
    }

    #[test]
    fn terminal_verdict_and_blocker_must_agree() {
        let mut terminal = Ms3GenerationTerminalReceiptV1 {
            schema: MS3_GENERATION_TERMINAL_SCHEMA_V1.to_owned(),
            terminal_root_sha256: String::new(),
            generation_sequence: 1,
            frozen_contract_root_sha256: "a".repeat(64),
            future_receipt_root_sha256: "b".repeat(64),
            future_capture_sequence: 8,
            future_topology_root_sha256: "c".repeat(64),
            future_completed_frame_root_sha256: "d".repeat(64),
            future_session_lineage_sha256: "e".repeat(64),
            verdict: Ms3IndependentFutureVerdictV1::Pass,
            blocker: "contradiction".to_owned(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        terminal.terminal_root_sha256 = terminal.expected_root().expect("terminal root");
        assert!(!terminal.validate());
    }

    #[test]
    fn terminal_cannot_precede_the_generation_future_boundary() {
        let mut terminal = Ms3GenerationTerminalReceiptV1 {
            schema: MS3_GENERATION_TERMINAL_SCHEMA_V1.to_owned(),
            terminal_root_sha256: String::new(),
            generation_sequence: 1,
            frozen_contract_root_sha256: "b".repeat(64),
            future_receipt_root_sha256: "2".repeat(64),
            future_capture_sequence: 8,
            future_topology_root_sha256: "3".repeat(64),
            future_completed_frame_root_sha256: "4".repeat(64),
            future_session_lineage_sha256: "5".repeat(64),
            verdict: Ms3IndependentFutureVerdictV1::Contradiction,
            blocker: "physical_transition_mismatch".to_owned(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        terminal.terminal_root_sha256 = terminal.expected_root().expect("terminal root");
        assert!(terminal.validate());

        let mut registry = Ms3GenerationRegistryV1 {
            schema: MS3_GENERATION_REGISTRY_SCHEMA_V1.to_owned(),
            registry_root_sha256: String::new(),
            generations: vec![Ms3GenerationEntryV1 {
                generation_sequence: 1,
                frozen_envelope_root_sha256: "6".repeat(64),
                frozen_contract_root_sha256: "b".repeat(64),
                support_rows_root_sha256: "7".repeat(64),
                topology_root_sha256: "8".repeat(64),
                frame_root_sha256: "9".repeat(64),
                session_lineage_sha256: "1".repeat(64),
                support_watermark: 7,
                future_min_sequence: 9,
                terminal: Some(terminal),
                acquisition_failure: None,
            }],
            linked_acquisition_failures: Vec::new(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        registry.registry_root_sha256 = registry.expected_root().expect("registry root");

        assert!(!registry.validate());
    }

    #[test]
    fn future_evidence_cannot_reuse_a_prior_generation_lineage() {
        let registry = Ms3GenerationRegistryV1 {
            schema: MS3_GENERATION_REGISTRY_SCHEMA_V1.to_owned(),
            registry_root_sha256: String::new(),
            generations: vec![Ms3GenerationEntryV1 {
                generation_sequence: 1,
                frozen_envelope_root_sha256: "1".repeat(64),
                frozen_contract_root_sha256: "2".repeat(64),
                support_rows_root_sha256: "3".repeat(64),
                topology_root_sha256: "4".repeat(64),
                frame_root_sha256: "5".repeat(64),
                session_lineage_sha256: "6".repeat(64),
                support_watermark: 7,
                future_min_sequence: 8,
                terminal: None,
                acquisition_failure: None,
            }],
            linked_acquisition_failures: Vec::new(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        let unused_topology = "7".repeat(64);
        let unused_frame = "8".repeat(64);
        let reused_lineage = "6".repeat(64);
        let fresh_lineage = "9".repeat(64);

        assert!(registry.evidence_roots_were_used([
            &unused_topology,
            &unused_frame,
            &reused_lineage,
        ]));
        assert!(!registry.evidence_roots_were_used([
            &unused_topology,
            &unused_frame,
            &fresh_lineage,
        ]));
    }
}
