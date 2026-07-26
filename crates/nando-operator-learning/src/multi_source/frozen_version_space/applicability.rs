use std::collections::BTreeSet;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};

use super::Ms3FuturePredictionV1;

pub const MS3_FUTURE_APPLICABILITY_CONTRACT_SCHEMA_V1: &str =
    "nando.ms3-future-applicability-contract.v1";
pub const MS3_FUTURE_APPLICABILITY_EVENT_SCHEMA_V1: &str =
    "nando.ms3-future-applicability-event.v1";
pub const MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1: &str =
    "nando.ms3-future-applicability-ledger.v1";
pub const MS3_FUTURE_APPLICABILITY_REPORT_SCHEMA_V1: &str =
    "nando.ms3-future-applicability-report.v1";
pub const MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL: &str =
    "MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL";
pub const MS3_FUTURE_APPLICABILITY_MAX_INDEPENDENT_TOPOLOGIES_V1: u64 = 256;
pub const MS3_FUTURE_APPLICABILITY_WINDOW_SECONDS_V1: u64 = 24 * 60 * 60;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3FutureApplicabilityDispositionV1 {
    StructurallyNotApplicable,
    PredictionCommitted,
    PrecommittedPredictionMissing,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ms3FutureApplicabilityVerdictV1 {
    Collecting,
    ApplicablePredictionPending,
    AcquisitionFail,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FutureApplicabilityContractV1 {
    pub schema: String,
    pub contract_root_sha256: String,
    pub frozen_law_contract_root_sha256: String,
    pub opened_at_sequence: u64,
    pub prediction_min_sequence: u64,
    pub opened_at_unix: u64,
    pub deadline_unix: u64,
    pub max_independent_topologies: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FutureApplicabilityEventV1 {
    pub schema: String,
    pub event_root_sha256: String,
    pub contract_root_sha256: String,
    pub capture_sequence: u64,
    pub topology_root_sha256: String,
    pub session_lineage_sha256: String,
    pub disposition: Ms3FutureApplicabilityDispositionV1,
    pub blocker: String,
    pub prediction_root_sha256: Option<String>,
    pub prediction_durable_at_unix_nanos: Option<u64>,
    pub terminal_receipt_root_sha256: Option<String>,
    pub terminal_completed_at_unix_nanos: Option<u64>,
    pub recorded_at_unix_nanos: u64,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ms3FutureApplicabilityLedgerV1 {
    pub schema: String,
    pub ledger_root_sha256: String,
    pub contract: Ms3FutureApplicabilityContractV1,
    pub events: Vec<Ms3FutureApplicabilityEventV1>,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Ms3FutureApplicabilityReportV1 {
    pub schema: String,
    pub report_root_sha256: String,
    pub ledger_root_sha256: String,
    pub contract: Ms3FutureApplicabilityContractV1,
    pub generated_at_unix: u64,
    pub independent_topologies: u64,
    pub structurally_not_applicable: u64,
    pub predictions_committed: u64,
    pub precommitted_prediction_missing: u64,
    pub active_predictions: u64,
    pub independent_lineages: u64,
    pub verdict: Ms3FutureApplicabilityVerdictV1,
    pub blocker: String,
    pub authority_ready: bool,
    pub phase_mutation_allowed: bool,
}

impl Ms3FutureApplicabilityContractV1 {
    pub fn seal(
        frozen_law_contract_root_sha256: String,
        opened_at_sequence: u64,
        prediction_min_sequence: u64,
        opened_at_unix: u64,
    ) -> Result<Self, &'static str> {
        let deadline_unix = opened_at_unix
            .checked_add(MS3_FUTURE_APPLICABILITY_WINDOW_SECONDS_V1)
            .ok_or("future_applicability_contract_invalid")?;
        let mut contract = Self {
            schema: MS3_FUTURE_APPLICABILITY_CONTRACT_SCHEMA_V1.to_owned(),
            contract_root_sha256: String::new(),
            frozen_law_contract_root_sha256,
            opened_at_sequence,
            prediction_min_sequence,
            opened_at_unix,
            deadline_unix,
            max_independent_topologies: MS3_FUTURE_APPLICABILITY_MAX_INDEPENDENT_TOPOLOGIES_V1,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        contract.contract_root_sha256 = contract.expected_root()?;
        contract
            .validate()
            .then_some(contract)
            .ok_or("future_applicability_contract_invalid")
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS3_FUTURE_APPLICABILITY_CONTRACT_SCHEMA_V1,
            self.frozen_law_contract_root_sha256.as_str(),
            self.opened_at_sequence,
            self.prediction_min_sequence,
            self.opened_at_unix,
            self.deadline_unix,
            self.max_independent_topologies,
            false,
            false,
        ))
        .map_err(|_| "future_applicability_contract_root_failed")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.schema == MS3_FUTURE_APPLICABILITY_CONTRACT_SCHEMA_V1
            && valid_nonzero_sha256(&self.contract_root_sha256)
            && valid_nonzero_sha256(&self.frozen_law_contract_root_sha256)
            && self.opened_at_sequence > 0
            && self.prediction_min_sequence == self.opened_at_sequence.saturating_add(1)
            && self.opened_at_unix > 0
            && self.deadline_unix
                == self
                    .opened_at_unix
                    .saturating_add(MS3_FUTURE_APPLICABILITY_WINDOW_SECONDS_V1)
            && self.max_independent_topologies
                == MS3_FUTURE_APPLICABILITY_MAX_INDEPENDENT_TOPOLOGIES_V1
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.contract_root_sha256)
    }
}

impl Ms3FutureApplicabilityEventV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        contract: &Ms3FutureApplicabilityContractV1,
        capture_sequence: u64,
        topology_root_sha256: String,
        session_lineage_sha256: String,
        disposition: Ms3FutureApplicabilityDispositionV1,
        blocker: String,
        prediction: Option<&Ms3FuturePredictionV1>,
        prediction_durable_at_unix_nanos: Option<u64>,
        terminal: Option<(&str, u64)>,
        recorded_at_unix_nanos: u64,
    ) -> Result<Self, &'static str> {
        let mut event = Self {
            schema: MS3_FUTURE_APPLICABILITY_EVENT_SCHEMA_V1.to_owned(),
            event_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256.clone(),
            capture_sequence,
            topology_root_sha256,
            session_lineage_sha256,
            disposition,
            blocker,
            prediction_root_sha256: prediction.map(|row| row.prediction_root_sha256.clone()),
            prediction_durable_at_unix_nanos,
            terminal_receipt_root_sha256: terminal.map(|(root, _)| root.to_owned()),
            terminal_completed_at_unix_nanos: terminal.map(|(_, completed)| completed),
            recorded_at_unix_nanos,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        event.event_root_sha256 = event.expected_root()?;
        event
            .validate(contract)
            .then_some(event)
            .ok_or("future_applicability_event_invalid")
    }

    fn expected_root(&self) -> Result<String, &'static str> {
        canonical_json_sha256(&(
            MS3_FUTURE_APPLICABILITY_EVENT_SCHEMA_V1,
            self.contract_root_sha256.as_str(),
            self.capture_sequence,
            self.topology_root_sha256.as_str(),
            self.session_lineage_sha256.as_str(),
            self.disposition,
            self.blocker.as_str(),
            self.prediction_root_sha256.as_deref(),
            self.prediction_durable_at_unix_nanos,
            self.terminal_receipt_root_sha256.as_deref(),
            self.terminal_completed_at_unix_nanos,
            self.recorded_at_unix_nanos,
            false,
            false,
        ))
        .map_err(|_| "future_applicability_event_root_failed")
    }

    fn validate(&self, contract: &Ms3FutureApplicabilityContractV1) -> bool {
        let shape_valid = match self.disposition {
            Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable => {
                !self.blocker.is_empty()
                    && self.prediction_root_sha256.is_none()
                    && self.prediction_durable_at_unix_nanos.is_none()
                    && self.terminal_receipt_root_sha256.is_none()
            }
            Ms3FutureApplicabilityDispositionV1::PredictionCommitted => {
                self.blocker.is_empty()
                    && self.prediction_root_sha256.is_some()
                    && self.prediction_durable_at_unix_nanos.is_some()
                    && self.terminal_receipt_root_sha256.is_none()
            }
            Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing => {
                self.blocker == "PRECOMMITTED_PREDICTION_MISSING"
                    && ((self.prediction_root_sha256.is_none()
                        && self.prediction_durable_at_unix_nanos.is_none()
                        && self.terminal_receipt_root_sha256.is_none()
                        && self.terminal_completed_at_unix_nanos.is_none())
                        || (self.prediction_root_sha256.is_some()
                            && self.prediction_durable_at_unix_nanos.is_some()
                            && self.terminal_receipt_root_sha256.is_some()
                            && self.terminal_completed_at_unix_nanos.is_some()
                            && self.terminal_completed_at_unix_nanos
                                <= self.prediction_durable_at_unix_nanos))
            }
        };
        self.schema == MS3_FUTURE_APPLICABILITY_EVENT_SCHEMA_V1
            && self.contract_root_sha256 == contract.contract_root_sha256
            && self.capture_sequence >= contract.prediction_min_sequence
            && valid_nonzero_sha256(&self.topology_root_sha256)
            && valid_nonzero_sha256(&self.session_lineage_sha256)
            && self.recorded_at_unix_nanos > 0
            && self.recorded_at_unix_nanos >= contract.opened_at_unix.saturating_mul(1_000_000_000)
            && shape_valid
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self
                .expected_root()
                .is_ok_and(|root| root == self.event_root_sha256)
    }
}

impl Ms3FutureApplicabilityLedgerV1 {
    pub fn new(contract: Ms3FutureApplicabilityContractV1) -> Result<Self, &'static str> {
        let mut ledger = Self {
            schema: MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1.to_owned(),
            ledger_root_sha256: String::new(),
            contract,
            events: Vec::new(),
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        ledger.reseal()?;
        Ok(ledger)
    }

    pub fn append(&mut self, event: Ms3FutureApplicabilityEventV1) -> Result<bool, &'static str> {
        if self
            .events
            .iter()
            .any(|existing| existing.event_root_sha256 == event.event_root_sha256)
        {
            return Ok(false);
        }
        let classifications = self.events.iter().filter(|existing| {
            existing.topology_root_sha256 == event.topology_root_sha256
                && existing.disposition
                    != Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
        });
        if event.disposition != Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
            && classifications.count() > 0
        {
            return Err("future_applicability_topology_reclassified");
        }
        if event.disposition == Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
            && event.prediction_root_sha256.is_some()
            && !self.events.iter().any(|existing| {
                existing.prediction_root_sha256 == event.prediction_root_sha256
                    && existing.disposition
                        == Ms3FutureApplicabilityDispositionV1::PredictionCommitted
            })
        {
            return Err("future_applicability_prediction_missing");
        }
        self.events.push(event);
        self.events.sort_by(|left, right| {
            left.capture_sequence
                .cmp(&right.capture_sequence)
                .then_with(|| left.event_root_sha256.cmp(&right.event_root_sha256))
        });
        self.reseal()?;
        Ok(true)
    }

    #[must_use]
    pub fn report(&self, generated_at_unix: u64) -> Ms3FutureApplicabilityReportV1 {
        let classifications = self
            .events
            .iter()
            .filter(|event| {
                event.disposition
                    != Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
                    || event.prediction_root_sha256.is_none()
            })
            .collect::<Vec<_>>();
        let disqualified = self
            .events
            .iter()
            .filter_map(|event| {
                (event.disposition
                    == Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing)
                    .then_some(event.prediction_root_sha256.as_deref())
                    .flatten()
            })
            .collect::<BTreeSet<_>>();
        let active_predictions = classifications
            .iter()
            .filter(|event| {
                event.disposition == Ms3FutureApplicabilityDispositionV1::PredictionCommitted
                    && event
                        .prediction_root_sha256
                        .as_deref()
                        .is_some_and(|root| !disqualified.contains(root))
            })
            .count();
        let independent_topologies = classifications.len();
        let exhausted = active_predictions == 0
            && (generated_at_unix >= self.contract.deadline_unix
                || independent_topologies
                    >= usize::try_from(self.contract.max_independent_topologies)
                        .unwrap_or(usize::MAX));
        let (verdict, blocker) = if active_predictions > 0 {
            (
                Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending,
                "independent_future_outcome_pending".to_owned(),
            )
        } else if exhausted {
            (
                Ms3FutureApplicabilityVerdictV1::AcquisitionFail,
                MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL.to_owned(),
            )
        } else {
            (
                Ms3FutureApplicabilityVerdictV1::Collecting,
                "applicable_independent_topology_pending".to_owned(),
            )
        };
        let mut report = Ms3FutureApplicabilityReportV1 {
            schema: MS3_FUTURE_APPLICABILITY_REPORT_SCHEMA_V1.to_owned(),
            report_root_sha256: String::new(),
            ledger_root_sha256: self.ledger_root_sha256.clone(),
            contract: self.contract.clone(),
            generated_at_unix,
            independent_topologies: u64::try_from(independent_topologies).unwrap_or(u64::MAX),
            structurally_not_applicable: count_disposition(
                &self.events,
                Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
            ),
            predictions_committed: count_disposition(
                &self.events,
                Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
            ),
            precommitted_prediction_missing: count_disposition(
                &self.events,
                Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing,
            ),
            active_predictions: u64::try_from(active_predictions).unwrap_or(u64::MAX),
            independent_lineages: u64::try_from(
                classifications
                    .iter()
                    .map(|event| event.session_lineage_sha256.as_str())
                    .collect::<BTreeSet<_>>()
                    .len(),
            )
            .unwrap_or(u64::MAX),
            verdict,
            blocker,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        report.report_root_sha256 = report.expected_root();
        report
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, &'static str> {
        self.validate()
            .then_some(())
            .ok_or("future_applicability_ledger_invalid")?;
        serde_cbor::to_vec(self).map_err(|_| "future_applicability_ledger_encode_failed")
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let ledger: Self = serde_cbor::from_slice(bytes)
            .map_err(|_| "future_applicability_ledger_decode_failed")?;
        if !ledger.validate() || ledger.canonical_bytes()? != bytes {
            return Err("future_applicability_ledger_invalid");
        }
        Ok(ledger)
    }

    fn reseal(&mut self) -> Result<(), &'static str> {
        self.ledger_root_sha256 = canonical_json_sha256(&(
            MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1,
            self.contract.contract_root_sha256.as_str(),
            self.events
                .iter()
                .map(|event| event.event_root_sha256.as_str())
                .collect::<Vec<_>>(),
            false,
            false,
        ))
        .map_err(|_| "future_applicability_ledger_root_failed")?;
        self.validate()
            .then_some(())
            .ok_or("future_applicability_ledger_invalid")
    }

    fn validate(&self) -> bool {
        let event_roots = self
            .events
            .iter()
            .map(|event| event.event_root_sha256.as_str())
            .collect::<BTreeSet<_>>();
        self.schema == MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1
            && self.contract.validate()
            && self.events.len()
                <= 2 * MS3_FUTURE_APPLICABILITY_MAX_INDEPENDENT_TOPOLOGIES_V1 as usize
            && event_roots.len() == self.events.len()
            && self
                .events
                .iter()
                .all(|event| event.validate(&self.contract))
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && canonical_json_sha256(&(
                MS3_FUTURE_APPLICABILITY_LEDGER_SCHEMA_V1,
                self.contract.contract_root_sha256.as_str(),
                self.events
                    .iter()
                    .map(|event| event.event_root_sha256.as_str())
                    .collect::<Vec<_>>(),
                false,
                false,
            ))
            .is_ok_and(|root| root == self.ledger_root_sha256)
    }
}

impl Ms3FutureApplicabilityReportV1 {
    fn expected_root(&self) -> String {
        canonical_json_sha256(&(
            MS3_FUTURE_APPLICABILITY_REPORT_SCHEMA_V1,
            self.ledger_root_sha256.as_str(),
            &self.contract,
            self.generated_at_unix,
            self.independent_topologies,
            self.structurally_not_applicable,
            self.predictions_committed,
            self.precommitted_prediction_missing,
            self.active_predictions,
            self.independent_lineages,
            self.verdict,
            self.blocker.as_str(),
            false,
            false,
        ))
        .expect("future applicability report serializes")
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        let verdict_valid = match self.verdict {
            Ms3FutureApplicabilityVerdictV1::Collecting => {
                self.active_predictions == 0
                    && self.generated_at_unix < self.contract.deadline_unix
                    && self.independent_topologies < self.contract.max_independent_topologies
                    && self.blocker == "applicable_independent_topology_pending"
            }
            Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending => {
                self.active_predictions > 0 && self.blocker == "independent_future_outcome_pending"
            }
            Ms3FutureApplicabilityVerdictV1::AcquisitionFail => {
                self.active_predictions == 0
                    && (self.generated_at_unix >= self.contract.deadline_unix
                        || self.independent_topologies >= self.contract.max_independent_topologies)
                    && self.blocker == MS3_FUTURE_APPLICABILITY_ACQUISITION_FAIL
            }
        };
        self.schema == MS3_FUTURE_APPLICABILITY_REPORT_SCHEMA_V1
            && valid_nonzero_sha256(&self.report_root_sha256)
            && valid_nonzero_sha256(&self.ledger_root_sha256)
            && self.contract.validate()
            && self.generated_at_unix >= self.contract.opened_at_unix
            && self.independent_lineages <= self.independent_topologies
            && self.structurally_not_applicable <= self.independent_topologies
            && self.predictions_committed <= self.independent_topologies
            && self.active_predictions <= self.predictions_committed
            && verdict_valid
            && !self.authority_ready
            && !self.phase_mutation_allowed
            && self.report_root_sha256 == self.expected_root()
    }
}

fn count_disposition(
    events: &[Ms3FutureApplicabilityEventV1],
    disposition: Ms3FutureApplicabilityDispositionV1,
) -> u64 {
    u64::try_from(
        events
            .iter()
            .filter(|event| event.disposition == disposition)
            .count(),
    )
    .unwrap_or(u64::MAX)
}
