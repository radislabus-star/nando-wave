use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::super::super::{MultiSourceT1IdentificationStateV1, MultiSourceT1IdentificationV3};
use super::super::model::{canonical_root_slice, canonical_roots, version_space_root};

const K1_GENERATION_TERMINAL_VERDICT_SCHEMA_V1: &str = "nando.k1-generation-terminal-verdict.v1";
const K1_TRANSFER_SETTLEMENT_SCHEMA_V1: &str = "nando.k1-transfer-settlement.v1";
pub const K1_DUPLICATE_PROTOCOL_BLOCKER_V1: &str = "all_supported_t1_protocol_modes_already_active";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K1GenerationVerdictClassV1 {
    Pass,
    Abstain,
    AcquisitionFail,
    IndependentFutureNotObserved,
    ProbeExhausted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1GenerationTerminalVerdictV1 {
    pub schema: String,
    pub verdict_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub identification_freeze_root_sha256: Option<String>,
    pub final_version_space_root_sha256: Option<String>,
    pub surviving_semantic_class_roots_sha256: Vec<String>,
    pub evidence_roots_sha256: Vec<String>,
    pub verdict: K1GenerationVerdictClassV1,
    pub blocker: String,
    pub terminal_at_unix: u64,
    pub transfer_identification: Option<MultiSourceT1IdentificationV3>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K1TransferSettlementV1 {
    pub schema: String,
    pub settlement_root_sha256: String,
    pub terminal_verdict_root_sha256: String,
    pub candidate_freeze_root_sha256: String,
    pub identification_report_root_sha256: String,
    pub package_id: String,
    pub package_candidate_root_sha256: String,
    pub certification_entry_root_sha256: String,
    pub certification_ledger_root_sha256: String,
    pub law_certificate_root_sha256: String,
    pub settled_at_unix: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl K1GenerationTerminalVerdictV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        candidate_freeze_root_sha256: String,
        identification_freeze_root_sha256: Option<String>,
        mut surviving_semantic_class_roots_sha256: Vec<String>,
        mut evidence_roots_sha256: Vec<String>,
        verdict: K1GenerationVerdictClassV1,
        blocker: String,
        terminal_at_unix: u64,
        transfer_identification: Option<MultiSourceT1IdentificationV3>,
    ) -> Result<Self, &'static str> {
        canonical_roots(&mut surviving_semantic_class_roots_sha256)?;
        canonical_roots(&mut evidence_roots_sha256)?;
        let final_version_space_root_sha256 = (!surviving_semantic_class_roots_sha256.is_empty())
            .then(|| version_space_root(&surviving_semantic_class_roots_sha256))
            .transpose()?;
        let mut receipt = Self {
            schema: K1_GENERATION_TERMINAL_VERDICT_SCHEMA_V1.to_owned(),
            verdict_root_sha256: String::new(),
            candidate_freeze_root_sha256,
            identification_freeze_root_sha256,
            final_version_space_root_sha256,
            surviving_semantic_class_roots_sha256,
            evidence_roots_sha256,
            verdict,
            blocker,
            terminal_at_unix,
            transfer_identification,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        receipt.verdict_root_sha256 = receipt.expected_root()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        let expected_final_root = (!self.surviving_semantic_class_roots_sha256.is_empty())
            .then(|| version_space_root(&self.surviving_semantic_class_roots_sha256))
            .transpose()?;
        if self.schema != K1_GENERATION_TERMINAL_VERDICT_SCHEMA_V1
            || !valid_nonzero_sha256(&self.verdict_root_sha256)
            || !valid_nonzero_sha256(&self.candidate_freeze_root_sha256)
            || self
                .identification_freeze_root_sha256
                .as_deref()
                .is_some_and(|root| !valid_nonzero_sha256(root))
            || self.evidence_roots_sha256.is_empty()
            || !canonical_root_slice(&self.evidence_roots_sha256)
            || (!self.surviving_semantic_class_roots_sha256.is_empty()
                && !canonical_root_slice(&self.surviving_semantic_class_roots_sha256))
            || self.final_version_space_root_sha256.as_deref() != expected_final_root.as_deref()
            || (self.verdict == K1GenerationVerdictClassV1::Pass
                && (self.surviving_semantic_class_roots_sha256.len() != 1
                    || !self.blocker.is_empty()
                    || self
                        .transfer_identification
                        .as_ref()
                        .is_none_or(|identification| {
                            !identification.validate()
                                || identification.state
                                    != MultiSourceT1IdentificationStateV1::TransferReady
                                || !identification.exact_transfer_parity
                                || identification.remaining_semantic_class_roots_sha256
                                    != self.surviving_semantic_class_roots_sha256
                                || !self
                                    .evidence_roots_sha256
                                    .contains(&identification.report_root_sha256)
                        })))
            || (self.verdict != K1GenerationVerdictClassV1::Pass
                && (self.blocker.is_empty() || self.transfer_identification.is_some()))
            || self.terminal_at_unix == 0
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.verdict_root_sha256 != self.expected_root()?
        {
            return Err("k1_generation_terminal_verdict_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_GENERATION_TERMINAL_VERDICT_SCHEMA_V1,
            self.candidate_freeze_root_sha256.as_str(),
            self.identification_freeze_root_sha256.as_deref(),
            self.final_version_space_root_sha256.as_deref(),
            &self.surviving_semantic_class_roots_sha256,
            &self.evidence_roots_sha256,
            self.verdict,
            self.blocker.as_str(),
            self.terminal_at_unix,
            self.transfer_identification
                .as_ref()
                .map(|identification| identification.report_root_sha256.as_str()),
            false,
            false,
        ))
    }
}

impl K1TransferSettlementV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        terminal: &K1GenerationTerminalVerdictV1,
        package_id: String,
        package_candidate_root_sha256: String,
        certification_entry_root_sha256: String,
        certification_ledger_root_sha256: String,
        law_certificate_root_sha256: String,
        settled_at_unix: u64,
    ) -> Result<Self, &'static str> {
        terminal.validate()?;
        let identification = terminal
            .transfer_identification
            .as_ref()
            .ok_or("k1_transfer_settlement_identification_missing")?;
        let mut settlement = Self {
            schema: K1_TRANSFER_SETTLEMENT_SCHEMA_V1.to_owned(),
            settlement_root_sha256: String::new(),
            terminal_verdict_root_sha256: terminal.verdict_root_sha256.clone(),
            candidate_freeze_root_sha256: terminal.candidate_freeze_root_sha256.clone(),
            identification_report_root_sha256: identification.report_root_sha256.clone(),
            package_id,
            package_candidate_root_sha256,
            certification_entry_root_sha256,
            certification_ledger_root_sha256,
            law_certificate_root_sha256,
            settled_at_unix,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        settlement.settlement_root_sha256 = settlement.expected_root()?;
        settlement.validate()?;
        Ok(settlement)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != K1_TRANSFER_SETTLEMENT_SCHEMA_V1
            || self.package_id.is_empty()
            || self.settled_at_unix == 0
            || ![
                self.settlement_root_sha256.as_str(),
                self.terminal_verdict_root_sha256.as_str(),
                self.candidate_freeze_root_sha256.as_str(),
                self.identification_report_root_sha256.as_str(),
                self.package_candidate_root_sha256.as_str(),
                self.certification_entry_root_sha256.as_str(),
                self.certification_ledger_root_sha256.as_str(),
                self.law_certificate_root_sha256.as_str(),
            ]
            .into_iter()
            .all(valid_nonzero_sha256)
            || self.authority_ready
            || self.phase_mutation_allowed
            || self.settlement_root_sha256 != self.expected_root()?
        {
            return Err("k1_transfer_settlement_invalid");
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            K1_TRANSFER_SETTLEMENT_SCHEMA_V1,
            self.terminal_verdict_root_sha256.as_str(),
            self.candidate_freeze_root_sha256.as_str(),
            self.identification_report_root_sha256.as_str(),
            self.package_id.as_str(),
            self.package_candidate_root_sha256.as_str(),
            self.certification_entry_root_sha256.as_str(),
            self.certification_ledger_root_sha256.as_str(),
            self.law_certificate_root_sha256.as_str(),
            self.settled_at_unix,
            false,
            false,
        ))
    }
}
