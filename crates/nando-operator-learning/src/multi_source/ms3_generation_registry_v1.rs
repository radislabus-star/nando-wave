use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::{
    FrozenVersionSpaceEnvelopeV1, MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL,
    Ms3FutureApplicabilityReportV1, Ms3FutureApplicabilityVerdictV1,
    Ms3IndependentFutureEnvelopeV1, Ms3IndependentFutureVerdictV1,
};

pub const MS3_GENERATION_REGISTRY_SCHEMA_V1: &str = "nando.ms3-generation-registry.v1";
pub const MS3_GENERATION_TERMINAL_SCHEMA_V1: &str = "nando.ms3-generation-terminal.v1";
pub const MS3_GENERATION_ACQUISITION_FAILURE_SCHEMA_V1: &str =
    "nando.ms3-generation-acquisition-failure.v1";
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
        if self
            .generations
            .last()
            .is_some_and(|entry| !entry.is_closed())
        {
            return Err(Ms3GenerationRegistryErrorV1::ActiveGenerationExists);
        }
        if self.generations.last().is_some_and(|entry| {
            entry
                .terminal
                .as_ref()
                .is_some_and(|terminal| terminal.verdict == Ms3IndependentFutureVerdictV1::Pass)
        }) {
            return Err(Ms3GenerationRegistryErrorV1::SuccessorAfterPass);
        }
        if self.generations.iter().any(|entry| {
            entry.frozen_envelope_root_sha256 == frozen.envelope_root_sha256
                || entry.frozen_contract_root_sha256 == frozen.contract.contract_root_sha256
                || entry.support_rows_root_sha256 == frozen.contract.support_rows_root_sha256
                || evidence_was_used(entry, frozen)
        }) {
            return Err(Ms3GenerationRegistryErrorV1::EvidenceReuse);
        }
        if let Some(previous) = self.generations.last() {
            let closure_sequence = previous
                .closure_capture_sequence()
                .ok_or(Ms3GenerationRegistryErrorV1::TerminalGenerationMissing)?;
            if frozen.contract.support_watermark <= closure_sequence
                || frozen.contract.future_min_sequence <= closure_sequence
            {
                return Err(Ms3GenerationRegistryErrorV1::EvidenceReuse);
            }
        }
        let generation_sequence = u64::try_from(self.generations.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
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
        self.schema == MS3_GENERATION_REGISTRY_SCHEMA_V1
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && roots_unique
            && evidence_roots_unique
            && self.generations.iter().enumerate().all(|(index, entry)| {
                entry.generation_sequence == u64::try_from(index).unwrap_or(u64::MAX) + 1
                    && valid_nonzero_sha256(&entry.frozen_envelope_root_sha256)
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
            && self.generations.windows(2).all(|pair| {
                pair[0].closure_allows_successor()
                    && pair[0].closure_capture_sequence().is_some_and(|sequence| {
                        pair[1].support_watermark > sequence
                            && pair[1].future_min_sequence > sequence
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
        canonical_json_sha256(&(
            MS3_GENERATION_REGISTRY_SCHEMA_V1,
            &self.generations,
            false,
            false,
        ))
        .map_err(|_| Ms3GenerationRegistryErrorV1::Serialization)
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

    fn closure_allows_successor(&self) -> bool {
        self.terminal.as_ref().is_some_and(|terminal| {
            terminal.verdict == Ms3IndependentFutureVerdictV1::Contradiction
        }) || self.acquisition_failure.is_some()
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
