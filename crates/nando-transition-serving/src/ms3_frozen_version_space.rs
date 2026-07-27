//! Durable owner for one immutable MS3 identification-machine checkpoint.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_learning::multi_source::{
    FrozenVersionSpaceContractV1, FrozenVersionSpaceEnvelopeV1, Ms3CompletedFrameCaptureFenceV1,
    Ms3FutureApplicabilityContractV1, Ms3FutureApplicabilityDispositionV1,
    Ms3FutureApplicabilityEventV1, Ms3FutureApplicabilityLedgerV1, Ms3FutureApplicabilityReportV1,
    Ms3FutureApplicabilityV1, Ms3FuturePredictionV1, Ms3GenerationRegistryV1,
    Ms3IndependentFutureEnvelopeV1, Ms3VersionSpaceVersionsV1, PreActionTopologyAuditRowV1,
    PreparedMs3VersionSpaceV1, classify_ms3_unique_law_v1,
};
use serde::{Deserialize, Serialize};

const ENVELOPE_FILE: &str = "frozen-version-space-v1.cbor";
const PREDICTIONS_FILE: &str = "future-predictions-v1.cbor";
const FUTURE_FILE: &str = "independent-future-v1.cbor";
const APPLICABILITY_FILE: &str = "future-applicability-v1.cbor";
#[cfg(test)]
const GENERATION_REGISTRY_FILE: &str = "generation-registry-v1.cbor";
const MAX_ENVELOPE_BYTES: usize = 12 * 1024 * 1024;
const MAX_PREDICTIONS: usize = 256;
const PREDICTION_LEDGER_SCHEMA_V1: &str = "nando.ms3-future-prediction-ledger.v1";

pub(super) struct Ms3FrozenVersionSpaceRuntime {
    generation_sequence: u64,
    envelope: Option<FrozenVersionSpaceEnvelopeV1>,
    envelope_path: PathBuf,
    prediction_ledger: Option<PredictionLedgerV1>,
    prediction_ledger_path: PathBuf,
    applicability_ledger: Option<Ms3FutureApplicabilityLedgerV1>,
    applicability_ledger_path: PathBuf,
    independent_future: Option<Ms3IndependentFutureEnvelopeV1>,
    independent_future_path: PathBuf,
    generation_registry: Ms3GenerationRegistryV1,
    generation_registry_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PredictionLedgerV1 {
    schema: String,
    ledger_root_sha256: String,
    contract_root_sha256: String,
    opened_at_sequence: u64,
    prediction_min_sequence: u64,
    predictions: Vec<Ms3FuturePredictionV1>,
}

impl Ms3FrozenVersionSpaceRuntime {
    pub(super) fn open(
        directory: &Path,
        generation_registry_path: &Path,
        generation_sequence: u64,
        current_capture_sequence: u64,
        opened_at_unix: u64,
    ) -> Result<Self, String> {
        if generation_sequence == 0 {
            return Err("ms3_version_space_generation_invalid".to_owned());
        }
        fs::create_dir_all(directory)
            .map_err(|error| format!("ms3_version_space_directory:{error}"))?;
        let envelope_path = directory.join(ENVELOPE_FILE);
        let envelope = read_bounded(&envelope_path)?
            .map(|bytes| {
                FrozenVersionSpaceEnvelopeV1::from_canonical_bytes(&bytes)
                    .map_err(|error| format!("ms3_version_space_restore:{error}"))
            })
            .transpose()?;
        let prediction_ledger_path = directory.join(PREDICTIONS_FILE);
        let mut prediction_ledger = read_bounded(&prediction_ledger_path)?
            .map(|bytes| {
                serde_cbor::from_slice::<PredictionLedgerV1>(&bytes)
                    .map_err(|error| format!("ms3_prediction_ledger_decode:{error}"))
            })
            .transpose()?;
        let applicability_ledger_path = directory.join(APPLICABILITY_FILE);
        let mut applicability_ledger = read_bounded(&applicability_ledger_path)?
            .map(|bytes| {
                Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&bytes)
                    .map_err(|error| format!("ms3_future_applicability_decode:{error}"))
            })
            .transpose()?;
        let _ = (current_capture_sequence, opened_at_unix);
        validate_committed_state_presence(
            envelope.is_some(),
            prediction_ledger.is_some(),
            applicability_ledger.is_some(),
        )?;
        match &envelope {
            None => {
                // The envelope is the bootstrap commit marker. Earlier files are staging.
                prediction_ledger = None;
                applicability_ledger = None;
            }
            Some(frozen) => {
                let predictions = prediction_ledger
                    .as_ref()
                    .ok_or_else(|| "ms3_future_prediction_state_missing".to_owned())?;
                validate_prediction_ledger(predictions, frozen)?;
                if applicability_ledger.is_none() {
                    return Err("ms3_future_applicability_state_missing".to_owned());
                }
            }
        }
        validate_applicability_link(
            applicability_ledger.as_ref(),
            envelope.as_ref(),
            prediction_ledger.as_ref(),
        )?;
        let independent_future_path = directory.join(FUTURE_FILE);
        let independent_future = match (&envelope, read_bounded(&independent_future_path)?) {
            (Some(frozen), Some(bytes)) => Some(
                Ms3IndependentFutureEnvelopeV1::from_canonical_bytes(&bytes, frozen)
                    .map_err(|error| format!("ms3_independent_future_restore:{error}"))?,
            ),
            (None, Some(_)) => {
                return Err("ms3_independent_future_without_contract".to_owned());
            }
            (_, None) => None,
        };
        validate_runtime_links(
            applicability_ledger.as_ref(),
            envelope.as_ref(),
            prediction_ledger.as_ref(),
            independent_future.as_ref(),
        )?;
        let generation_registry_path = generation_registry_path.to_path_buf();
        let mut generation_registry = read_bounded(&generation_registry_path)?
            .map(|bytes| {
                Ms3GenerationRegistryV1::from_canonical_bytes(&bytes)
                    .map_err(|error| format!("ms3_generation_registry_restore:{error:?}"))
            })
            .transpose()?
            .unwrap_or_default();
        let registry_changed = reconcile_generation_registry(
            &mut generation_registry,
            envelope.as_ref(),
            independent_future.as_ref(),
        )?;
        validate_generation_position(&generation_registry, envelope.as_ref(), generation_sequence)?;
        if registry_changed {
            let bytes = generation_registry
                .canonical_bytes()
                .map_err(|error| format!("ms3_generation_registry_encode:{error:?}"))?;
            write_atomic(&generation_registry_path, &bytes)?;
        }
        Ok(Self {
            generation_sequence,
            envelope,
            envelope_path,
            prediction_ledger,
            prediction_ledger_path,
            applicability_ledger,
            applicability_ledger_path,
            independent_future,
            independent_future_path,
            generation_registry,
            generation_registry_path,
        })
    }

    pub(super) fn freeze(
        &mut self,
        prepared: PreparedMs3VersionSpaceV1,
        contract_watermark: u64,
        versions: Ms3VersionSpaceVersionsV1,
        opened_at_unix: u64,
    ) -> Result<FrozenVersionSpaceContractV1, String> {
        if let Some(envelope) = &self.envelope {
            return Ok(envelope.contract.clone());
        }
        let envelope = prepared
            .seal(contract_watermark, versions)
            .map_err(|error| format!("ms3_version_space_seal:{error}"))?;
        let envelope_bytes = envelope
            .canonical_bytes()
            .map_err(|error| format!("ms3_version_space_encode:{error}"))?;
        let prediction_min_sequence = contract_watermark
            .checked_add(1)
            .ok_or_else(|| "ms3_prediction_open_watermark_overflow".to_owned())?;
        let predictions = match read_bounded(&self.prediction_ledger_path)? {
            Some(bytes) => {
                let ledger: PredictionLedgerV1 = serde_cbor::from_slice(&bytes)
                    .map_err(|error| format!("ms3_prediction_ledger_decode:{error}"))?;
                validate_prediction_ledger(&ledger, &envelope)?;
                if !ledger.predictions.is_empty()
                    || ledger.opened_at_sequence != contract_watermark
                    || ledger.prediction_min_sequence != prediction_min_sequence
                {
                    return Err("ms3_prediction_bootstrap_conflict".to_owned());
                }
                ledger
            }
            None => {
                let mut ledger = PredictionLedgerV1 {
                    schema: PREDICTION_LEDGER_SCHEMA_V1.to_owned(),
                    ledger_root_sha256: String::new(),
                    contract_root_sha256: envelope.contract.contract_root_sha256.clone(),
                    opened_at_sequence: contract_watermark,
                    prediction_min_sequence,
                    predictions: Vec::new(),
                };
                ledger.ledger_root_sha256 = prediction_ledger_root(&ledger)?;
                validate_prediction_ledger(&ledger, &envelope)?;
                ledger
            }
        };
        let prediction_bytes = serde_cbor::to_vec(&predictions)
            .map_err(|error| format!("ms3_prediction_ledger_encode:{error}"))?;
        let applicability = match read_bounded(&self.applicability_ledger_path)? {
            Some(bytes) => {
                let ledger = Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&bytes)
                    .map_err(|error| format!("ms3_future_applicability_decode:{error}"))?;
                if !ledger.events.is_empty()
                    || ledger.contract.frozen_law_contract_root_sha256
                        != envelope.contract.contract_root_sha256
                    || ledger.contract.opened_at_sequence != contract_watermark
                    || ledger.contract.prediction_min_sequence != prediction_min_sequence
                {
                    return Err("ms3_applicability_bootstrap_conflict".to_owned());
                }
                ledger
            }
            None => {
                let contract = Ms3FutureApplicabilityContractV1::seal(
                    envelope.contract.contract_root_sha256.clone(),
                    contract_watermark,
                    prediction_min_sequence,
                    opened_at_unix,
                )
                .map_err(str::to_owned)?;
                Ms3FutureApplicabilityLedgerV1::new(contract).map_err(str::to_owned)?
            }
        };
        let applicability_bytes = applicability.canonical_bytes().map_err(str::to_owned)?;
        let mut generation_registry = self.generation_registry.clone();
        let appended_generation = generation_registry
            .append_generation(&envelope)
            .map_err(|error| format!("ms3_generation_registry_append:{error:?}"))?;
        if appended_generation != self.generation_sequence {
            return Err("ms3_generation_registry_sequence_mismatch".to_owned());
        }
        let registry_bytes = generation_registry
            .canonical_bytes()
            .map_err(|error| format!("ms3_generation_registry_encode:{error:?}"))?;

        // Publish the in-memory experiment only after every durable component exists.
        write_atomic(&self.applicability_ledger_path, &applicability_bytes)?;
        write_atomic(&self.prediction_ledger_path, &prediction_bytes)?;
        write_atomic(&self.envelope_path, &envelope_bytes)?;
        write_atomic(&self.generation_registry_path, &registry_bytes)?;
        self.applicability_ledger = Some(applicability);
        self.prediction_ledger = Some(predictions);
        self.envelope = Some(envelope);
        self.generation_registry = generation_registry;
        self.contract()
            .cloned()
            .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())
    }

    pub(super) fn observe_topology(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        predicted_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        self.observe_topology_inner(topology, predicted_at_unix_nanos, true)
    }

    pub(super) fn observe_historical_topology(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        observed_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        self.observe_topology_inner(topology, observed_at_unix_nanos, false)
    }

    fn observe_topology_inner(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        observed_at_unix_nanos: u64,
        allow_concurrent_live_prediction: bool,
    ) -> Result<bool, String> {
        if self.independent_future.is_some() {
            return Ok(false);
        }
        let Some(frozen) = &self.envelope else {
            return Ok(false);
        };
        if self.prediction_ledger.as_ref().is_some_and(|ledger| {
            topology
                .bridge_sequence
                .is_none_or(|sequence| sequence < ledger.prediction_min_sequence)
        }) {
            return Ok(false);
        }
        if self.applicability_ledger.as_ref().is_some_and(|ledger| {
            ledger.events.iter().any(|event| {
                event.topology_root_sha256 == topology.commit.commitment_root_sha256
                    && (!matches!(
                        event.disposition,
                        Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
                            | Ms3FutureApplicabilityDispositionV1::CensoredMissingCompletedFrame
                    ) || event.prediction_root_sha256.is_none())
            })
        }) {
            return Ok(false);
        }
        let gate_contract = self
            .applicability_ledger
            .as_ref()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?
            .contract
            .clone();
        let gate_verdict = self
            .applicability_ledger
            .as_ref()
            .expect("gate checked")
            .report(observed_at_unix_nanos / 1_000_000_000)
            .verdict;
        if !future_topology_observation_allowed(gate_verdict, allow_concurrent_live_prediction) {
            return Ok(false);
        }
        let classification = classify_ms3_unique_law_v1(frozen, topology, observed_at_unix_nanos)
            .map_err(|error| format!("ms3_future_predict:{error}"))?;
        let capture_sequence = topology
            .bridge_sequence
            .ok_or_else(|| "ms3_future_capture_sequence_missing".to_owned())?;
        let lineage = topology
            .session_lineage_sha256
            .clone()
            .ok_or_else(|| "ms3_future_lineage_missing".to_owned())?;
        let prediction = match classification {
            Ms3FutureApplicabilityV1::BeforeFutureWindow
            | Ms3FutureApplicabilityV1::SupportLineageReuse => return Ok(false),
            Ms3FutureApplicabilityV1::StructurallyNotApplicable { blocker } => {
                let event = Ms3FutureApplicabilityEventV1::seal(
                    &gate_contract,
                    capture_sequence,
                    topology.commit.commitment_root_sha256.clone(),
                    lineage,
                    Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
                    blocker.to_owned(),
                    None,
                    None,
                    None,
                    observed_at_unix_nanos,
                )
                .map_err(str::to_owned)?;
                return self.append_applicability_event(event);
            }
            Ms3FutureApplicabilityV1::Applicable { prediction } => *prediction,
        };
        let mut ledger = self
            .prediction_ledger
            .clone()
            .unwrap_or(PredictionLedgerV1 {
                schema: PREDICTION_LEDGER_SCHEMA_V1.to_owned(),
                ledger_root_sha256: String::new(),
                contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
                opened_at_sequence: frozen.contract.contract_watermark,
                prediction_min_sequence: frozen.contract.future_min_sequence,
                predictions: Vec::new(),
            });
        let prediction = if let Some(existing) = ledger
            .predictions
            .iter()
            .find(|existing| existing.topology_root_sha256 == prediction.topology_root_sha256)
            .cloned()
        {
            existing
        } else {
            if ledger.predictions.len() >= MAX_PREDICTIONS {
                return Err("ms3_future_prediction_budget".to_owned());
            }
            ledger.predictions.push(prediction.clone());
            ledger.predictions.sort_by_key(|row| row.capture_sequence);
            ledger.ledger_root_sha256 = prediction_ledger_root(&ledger)?;
            validate_prediction_ledger(&ledger, frozen)?;
            let bytes = serde_cbor::to_vec(&ledger)
                .map_err(|error| format!("ms3_prediction_ledger_encode:{error}"))?;
            write_atomic(&self.prediction_ledger_path, &bytes)?;
            self.prediction_ledger = Some(ledger);
            prediction
        };
        let durable_at_unix_nanos = unix_now_nanos();
        let event = Ms3FutureApplicabilityEventV1::seal(
            &gate_contract,
            capture_sequence,
            topology.commit.commitment_root_sha256.clone(),
            lineage,
            Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
            String::new(),
            Some(&prediction),
            Some(durable_at_unix_nanos),
            None,
            durable_at_unix_nanos,
        )
        .map_err(str::to_owned)?;
        self.append_applicability_event(event)
    }

    pub(super) fn predictions(&self) -> Vec<Ms3FuturePredictionV1> {
        self.prediction_ledger
            .as_ref()
            .map(|ledger| ledger.predictions.clone())
            .unwrap_or_default()
    }

    pub(super) fn prediction_min_sequence(&self) -> Option<u64> {
        self.prediction_ledger
            .as_ref()
            .map(|ledger| ledger.prediction_min_sequence)
    }

    pub(super) fn applicability_report(
        &self,
        generated_at_unix: u64,
    ) -> Result<Option<Ms3FutureApplicabilityReportV1>, String> {
        self.applicability_ledger
            .as_ref()
            .map(|ledger| {
                let report = ledger.report(generated_at_unix);
                report
                    .validate()
                    .then_some(report)
                    .ok_or_else(|| "ms3_future_applicability_report_invalid".to_owned())
            })
            .transpose()
    }

    pub(super) fn prediction_commitment(&self, prediction_root: &str) -> Option<(String, u64)> {
        self.applicability_ledger
            .as_ref()?
            .events
            .iter()
            .find_map(|event| {
                (event.disposition == Ms3FutureApplicabilityDispositionV1::PredictionCommitted
                    && event.prediction_root_sha256.as_deref() == Some(prediction_root))
                .then(|| {
                    event
                        .prediction_durable_at_unix_nanos
                        .map(|durable_at| (event.event_root_sha256.clone(), durable_at))
                })
                .flatten()
            })
    }

    pub(super) fn prediction_is_disqualified(&self, prediction_root: &str) -> bool {
        self.applicability_ledger.as_ref().is_some_and(|ledger| {
            ledger.events.iter().any(|event| {
                matches!(
                    event.disposition,
                    Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
                        | Ms3FutureApplicabilityDispositionV1::CensoredMissingCompletedFrame
                ) && event.prediction_root_sha256.as_deref() == Some(prediction_root)
            })
        })
    }

    pub(super) fn record_precommitted_prediction_missing(
        &mut self,
        prediction: &Ms3FuturePredictionV1,
        terminal_receipt_root_sha256: &str,
        terminal_completed_at_unix_nanos: u64,
        action_observed_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        if self.prediction_is_disqualified(&prediction.prediction_root_sha256) {
            return Ok(false);
        }
        let (_, durable_at) = self
            .prediction_commitment(&prediction.prediction_root_sha256)
            .ok_or_else(|| "ms3_prediction_durable_receipt_missing".to_owned())?;
        let gate = self
            .applicability_ledger
            .as_ref()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?;
        let now = unix_now_nanos();
        let event = Ms3FutureApplicabilityEventV1::seal(
            &gate.contract,
            prediction.capture_sequence,
            prediction.topology_root_sha256.clone(),
            prediction.session_lineage_sha256.clone(),
            Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing,
            "PRECOMMITTED_PREDICTION_MISSING".to_owned(),
            Some(prediction),
            Some(durable_at),
            Some((
                terminal_receipt_root_sha256,
                terminal_completed_at_unix_nanos,
                action_observed_at_unix_nanos,
            )),
            now,
        )
        .map_err(str::to_owned)?;
        self.append_applicability_event(event)
    }

    pub(super) fn record_censored_missing_completed_frame(
        &mut self,
        prediction: &Ms3FuturePredictionV1,
        terminal_receipt_root_sha256: &str,
        terminal_completed_at_unix_nanos: u64,
        capture_fence: Ms3CompletedFrameCaptureFenceV1,
    ) -> Result<bool, String> {
        if self.prediction_is_disqualified(&prediction.prediction_root_sha256) {
            return Ok(false);
        }
        let (_, durable_at) = self
            .prediction_commitment(&prediction.prediction_root_sha256)
            .ok_or_else(|| "ms3_prediction_durable_receipt_missing".to_owned())?;
        let gate = self
            .applicability_ledger
            .as_ref()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?;
        let event = Ms3FutureApplicabilityEventV1::seal_censored_missing_completed_frame(
            &gate.contract,
            prediction,
            durable_at,
            terminal_receipt_root_sha256,
            terminal_completed_at_unix_nanos,
            capture_fence,
            unix_now_nanos(),
        )
        .map_err(str::to_owned)?;
        self.append_applicability_event(event)
    }

    pub(super) fn seal_independent_future(
        &mut self,
        future: Ms3IndependentFutureEnvelopeV1,
    ) -> Result<(), String> {
        if let Some(existing) = &self.independent_future {
            return (existing.envelope_root_sha256 == future.envelope_root_sha256)
                .then_some(())
                .ok_or_else(|| "ms3_independent_future_conflict".to_owned());
        }
        let frozen = self
            .envelope
            .as_ref()
            .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())?;
        let gate = self
            .applicability_ledger
            .as_ref()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?;
        if !future_outcome_resolution_allowed(gate.report(unix_now_nanos() / 1_000_000_000).verdict)
        {
            return Err("ms3_future_applicability_gate_closed".to_owned());
        }
        let bytes = future
            .canonical_bytes(frozen)
            .map_err(|error| format!("ms3_independent_future_encode:{error}"))?;
        validate_runtime_links(
            self.applicability_ledger.as_ref(),
            self.envelope.as_ref(),
            self.prediction_ledger.as_ref(),
            Some(&future),
        )?;
        let mut generation_registry = self.generation_registry.clone();
        generation_registry
            .seal_terminal(frozen, &future)
            .map_err(|error| format!("ms3_generation_registry_terminal:{error:?}"))?;
        let registry_bytes = generation_registry
            .canonical_bytes()
            .map_err(|error| format!("ms3_generation_registry_encode:{error:?}"))?;
        write_atomic(&self.independent_future_path, &bytes)?;
        write_atomic(&self.generation_registry_path, &registry_bytes)?;
        self.independent_future = Some(future);
        self.generation_registry = generation_registry;
        Ok(())
    }

    pub(super) const fn independent_future(&self) -> Option<&Ms3IndependentFutureEnvelopeV1> {
        self.independent_future.as_ref()
    }

    pub(super) const fn envelope(&self) -> Option<&FrozenVersionSpaceEnvelopeV1> {
        self.envelope.as_ref()
    }

    pub(super) fn contract(&self) -> Option<&FrozenVersionSpaceContractV1> {
        self.envelope.as_ref().map(|envelope| &envelope.contract)
    }

    pub(super) const fn generation_registry(&self) -> &Ms3GenerationRegistryV1 {
        &self.generation_registry
    }

    pub(super) const fn generation_sequence(&self) -> u64 {
        self.generation_sequence
    }

    fn append_applicability_event(
        &mut self,
        event: Ms3FutureApplicabilityEventV1,
    ) -> Result<bool, String> {
        let mut next = self
            .applicability_ledger
            .clone()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?;
        if !next.append(event).map_err(str::to_owned)? {
            return Ok(false);
        }
        let bytes = next.canonical_bytes().map_err(str::to_owned)?;
        write_atomic(&self.applicability_ledger_path, &bytes)?;
        self.applicability_ledger = Some(next);
        Ok(true)
    }
}

fn reconcile_generation_registry(
    registry: &mut Ms3GenerationRegistryV1,
    frozen: Option<&FrozenVersionSpaceEnvelopeV1>,
    future: Option<&Ms3IndependentFutureEnvelopeV1>,
) -> Result<bool, String> {
    let Some(frozen) = frozen else {
        if registry
            .generations
            .last()
            .is_some_and(|entry| entry.terminal.is_none())
        {
            return Err("ms3_generation_registry_active_artifact_missing".to_owned());
        }
        return future
            .is_none()
            .then_some(false)
            .ok_or_else(|| "ms3_generation_registry_future_without_frozen".to_owned());
    };
    let mut changed = false;
    match registry.generations.last() {
        None => {
            registry
                .append_generation(frozen)
                .map_err(|error| format!("ms3_generation_registry_bootstrap:{error:?}"))?;
            changed = true;
        }
        Some(entry)
            if entry.frozen_envelope_root_sha256 == frozen.envelope_root_sha256
                && entry.frozen_contract_root_sha256 == frozen.contract.contract_root_sha256 => {}
        Some(_) => return Err("ms3_generation_registry_active_generation_mismatch".to_owned()),
    }
    if let Some(future) = future {
        let terminal_missing = registry
            .generations
            .last()
            .is_none_or(|entry| entry.terminal.is_none());
        registry
            .seal_terminal(frozen, future)
            .map_err(|error| format!("ms3_generation_registry_terminal:{error:?}"))?;
        changed |= terminal_missing;
    } else if registry
        .generations
        .last()
        .is_some_and(|entry| entry.terminal.is_some())
    {
        return Err("ms3_generation_registry_terminal_future_missing".to_owned());
    }
    Ok(changed)
}

fn validate_generation_position(
    registry: &Ms3GenerationRegistryV1,
    frozen: Option<&FrozenVersionSpaceEnvelopeV1>,
    generation_sequence: u64,
) -> Result<(), String> {
    match (frozen, registry.generations.last()) {
        (Some(frozen), Some(entry))
            if entry.generation_sequence == generation_sequence
                && entry.frozen_envelope_root_sha256 == frozen.envelope_root_sha256
                && entry.frozen_contract_root_sha256
                    == frozen.contract.contract_root_sha256 =>
        {
            Ok(())
        }
        (Some(_), _) => Err("ms3_generation_registry_active_generation_mismatch".to_owned()),
        (None, None) if generation_sequence == 1 => Ok(()),
        (None, Some(entry))
            if entry.generation_sequence.saturating_add(1) == generation_sequence
                && entry.terminal.as_ref().is_some_and(|terminal| {
                    terminal.verdict
                        == nando_operator_learning::multi_source::Ms3IndependentFutureVerdictV1::Contradiction
                }) =>
        {
            Ok(())
        }
        (None, _) => Err("ms3_generation_registry_position_invalid".to_owned()),
    }
}

fn validate_applicability_link(
    applicability: Option<&Ms3FutureApplicabilityLedgerV1>,
    frozen: Option<&FrozenVersionSpaceEnvelopeV1>,
    predictions: Option<&PredictionLedgerV1>,
) -> Result<(), String> {
    match (applicability, frozen, predictions) {
        (Some(applicability), Some(frozen), Some(predictions))
            if applicability.contract.frozen_law_contract_root_sha256
                == frozen.contract.contract_root_sha256
                && applicability.contract.opened_at_sequence == predictions.opened_at_sequence
                && applicability.contract.prediction_min_sequence
                    == predictions.prediction_min_sequence =>
        {
            Ok(())
        }
        (None, None, None) => Ok(()),
        _ => Err("ms3_future_applicability_link_invalid".to_owned()),
    }
}

fn validate_runtime_links(
    applicability: Option<&Ms3FutureApplicabilityLedgerV1>,
    frozen: Option<&FrozenVersionSpaceEnvelopeV1>,
    predictions: Option<&PredictionLedgerV1>,
    future: Option<&Ms3IndependentFutureEnvelopeV1>,
) -> Result<(), String> {
    validate_applicability_link(applicability, frozen, predictions)?;
    let (Some(applicability), Some(predictions)) = (applicability, predictions) else {
        return future
            .is_none()
            .then_some(())
            .ok_or_else(|| "ms3_independent_future_without_applicability".to_owned());
    };
    validate_prediction_events(applicability, predictions)?;
    if let Some(future) = future {
        let receipt = &future.receipt;
        let event = applicability
            .events
            .iter()
            .find(|event| {
                event.event_root_sha256 == receipt.applicability_event_root_sha256
                    && event.disposition == Ms3FutureApplicabilityDispositionV1::PredictionCommitted
                    && event.prediction_root_sha256.as_deref()
                        == Some(receipt.prediction_root_sha256.as_str())
            })
            .ok_or_else(|| "ms3_future_applicability_event_missing".to_owned())?;
        if event.capture_sequence != receipt.capture_sequence
            || event.topology_root_sha256 != receipt.topology_root_sha256
        {
            return Err("ms3_future_applicability_event_binding_invalid".to_owned());
        }
    }
    Ok(())
}

fn future_topology_observation_allowed(
    verdict: nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1,
    allow_concurrent_live_prediction: bool,
) -> bool {
    use nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1;

    match verdict {
        Ms3FutureApplicabilityVerdictV1::Collecting => true,
        Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending => {
            allow_concurrent_live_prediction
        }
        Ms3FutureApplicabilityVerdictV1::AcquisitionFail => false,
    }
}

fn future_outcome_resolution_allowed(
    verdict: nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1,
) -> bool {
    use nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1;

    matches!(
        verdict,
        Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending
            | Ms3FutureApplicabilityVerdictV1::AcquisitionFail
    )
}

fn validate_committed_state_presence(
    envelope: bool,
    predictions: bool,
    applicability: bool,
) -> Result<(), String> {
    if envelope && !predictions {
        return Err("ms3_future_prediction_state_missing".to_owned());
    }
    if envelope && !applicability {
        return Err("ms3_future_applicability_state_missing".to_owned());
    }
    Ok(())
}

fn validate_prediction_events(
    applicability: &Ms3FutureApplicabilityLedgerV1,
    predictions: &PredictionLedgerV1,
) -> Result<(), String> {
    for event in &applicability.events {
        if event.disposition != Ms3FutureApplicabilityDispositionV1::PredictionCommitted {
            continue;
        }
        let prediction = predictions
            .predictions
            .iter()
            .find(|prediction| {
                event.prediction_root_sha256.as_deref()
                    == Some(prediction.prediction_root_sha256.as_str())
            })
            .ok_or_else(|| "ms3_applicability_prediction_missing".to_owned())?;
        if event.capture_sequence != prediction.capture_sequence
            || event.topology_root_sha256 != prediction.topology_root_sha256
            || event.session_lineage_sha256 != prediction.session_lineage_sha256
        {
            return Err("ms3_applicability_prediction_binding_invalid".to_owned());
        }
    }
    Ok(())
}

fn unix_now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn prediction_ledger_root(ledger: &PredictionLedgerV1) -> Result<String, String> {
    nando_operator_kernel::canonical_json_sha256(&(
        PREDICTION_LEDGER_SCHEMA_V1,
        ledger.contract_root_sha256.as_str(),
        ledger.opened_at_sequence,
        ledger.prediction_min_sequence,
        ledger
            .predictions
            .iter()
            .map(|prediction| prediction.prediction_root_sha256.as_str())
            .collect::<Vec<_>>(),
    ))
    .map_err(|error| format!("ms3_prediction_ledger_root:{error}"))
}

fn validate_prediction_ledger(
    ledger: &PredictionLedgerV1,
    frozen: &FrozenVersionSpaceEnvelopeV1,
) -> Result<(), String> {
    if ledger.schema != PREDICTION_LEDGER_SCHEMA_V1
        || ledger.contract_root_sha256 != frozen.contract.contract_root_sha256
        || ledger.prediction_min_sequence
            != ledger
                .opened_at_sequence
                .saturating_add(1)
                .max(frozen.contract.future_min_sequence)
        || ledger.predictions.len() > MAX_PREDICTIONS
        || !ledger
            .predictions
            .windows(2)
            .all(|pair| pair[0].capture_sequence < pair[1].capture_sequence)
        || ledger.predictions.iter().any(|prediction| {
            prediction.capture_sequence < ledger.prediction_min_sequence
                || prediction.validate(frozen).is_err()
        })
        || ledger.ledger_root_sha256 != prediction_ledger_root(ledger)?
    {
        return Err("ms3_prediction_ledger_invalid".to_owned());
    }
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("ms3_version_space_open:{}:{error}", path.display())),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(MAX_ENVELOPE_BYTES + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("ms3_version_space_read:{}:{error}", path.display()))?;
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err("ms3_version_space_state_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_ENVELOPE_BYTES {
        return Err("ms3_version_space_state_budget".to_owned());
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("ms3_version_space_write_open:{error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("ms3_version_space_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("ms3_version_space_rename:{error}"))?;
    File::open(
        path.parent()
            .ok_or_else(|| "ms3_version_space_parent_missing".to_owned())?,
    )
    .and_then(|directory| directory.sync_all())
    .map_err(|error| format!("ms3_version_space_directory_sync:{error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use nando_operator_kernel::sha256_bytes;

    use super::*;

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn test_root(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nando-ms3-{label}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn committed_state_loss_is_fail_closed_but_precommit_staging_is_retryable() {
        assert!(validate_committed_state_presence(false, true, true).is_ok());
        assert_eq!(
            validate_committed_state_presence(true, false, true),
            Err("ms3_future_prediction_state_missing".to_owned())
        );
        assert_eq!(
            validate_committed_state_presence(true, true, false),
            Err("ms3_future_applicability_state_missing".to_owned())
        );
    }

    #[test]
    fn live_topology_can_precommit_concurrently_but_historical_replay_cannot() {
        use nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1;

        assert!(future_topology_observation_allowed(
            Ms3FutureApplicabilityVerdictV1::Collecting,
            false
        ));
        assert!(future_topology_observation_allowed(
            Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending,
            true
        ));
        assert!(!future_topology_observation_allowed(
            Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending,
            false
        ));
        assert!(!future_topology_observation_allowed(
            Ms3FutureApplicabilityVerdictV1::AcquisitionFail,
            true
        ));
        assert!(future_outcome_resolution_allowed(
            Ms3FutureApplicabilityVerdictV1::ApplicablePredictionPending
        ));
        assert!(future_outcome_resolution_allowed(
            Ms3FutureApplicabilityVerdictV1::AcquisitionFail
        ));
        assert!(!future_outcome_resolution_allowed(
            Ms3FutureApplicabilityVerdictV1::Collecting
        ));
    }

    #[test]
    fn failed_applicability_write_does_not_mutate_ram_ledger() {
        let root = test_directory("append-rollback");
        fs::create_dir_all(&root).expect("test root");
        let contract =
            Ms3FutureApplicabilityContractV1::seal(test_root("law"), 7, 8, 100).expect("contract");
        let ledger = Ms3FutureApplicabilityLedgerV1::new(contract.clone()).expect("ledger");
        let before = ledger.clone();
        let event = Ms3FutureApplicabilityEventV1::seal(
            &contract,
            8,
            test_root("topology"),
            test_root("lineage"),
            Ms3FutureApplicabilityDispositionV1::StructurallyNotApplicable,
            "missing-role".to_owned(),
            None,
            None,
            None,
            101_000_000_000,
        )
        .expect("event");
        let mut runtime = Ms3FrozenVersionSpaceRuntime {
            generation_sequence: 1,
            envelope: None,
            envelope_path: root.join(ENVELOPE_FILE),
            prediction_ledger: None,
            prediction_ledger_path: root.join(PREDICTIONS_FILE),
            applicability_ledger: Some(ledger),
            applicability_ledger_path: root.join("missing-parent").join(APPLICABILITY_FILE),
            independent_future: None,
            independent_future_path: root.join(FUTURE_FILE),
            generation_registry: Ms3GenerationRegistryV1::new(),
            generation_registry_path: root.join(GENERATION_REGISTRY_FILE),
        };

        assert!(runtime.append_applicability_event(event).is_err());
        assert_eq!(runtime.applicability_ledger.as_ref(), Some(&before));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn committed_applicability_event_without_prediction_row_is_rejected() {
        let contract =
            Ms3FutureApplicabilityContractV1::seal(test_root("law"), 7, 8, 100).expect("contract");
        let prediction = Ms3FuturePredictionV1 {
            schema: nando_operator_learning::multi_source::MS3_FUTURE_PREDICTION_SCHEMA_V1
                .to_owned(),
            prediction_root_sha256: test_root("prediction"),
            contract_root_sha256: test_root("contract"),
            candidate_freeze_root_sha256: test_root("freeze"),
            canonical_program_root_sha256: test_root("program"),
            capture_sequence: 8,
            topology_root_sha256: test_root("topology"),
            request_event_id_sha256: test_root("request"),
            turn_intent_id_sha256: test_root("intent"),
            session_lineage_sha256: test_root("lineage"),
            pre_action_binding_root_sha256: test_root("binding"),
            predicted_at_unix_nanos: 100_000_000_001,
            authority_ready: false,
            phase_mutation_allowed: false,
        };
        let event = Ms3FutureApplicabilityEventV1::seal(
            &contract,
            8,
            prediction.topology_root_sha256.clone(),
            prediction.session_lineage_sha256.clone(),
            Ms3FutureApplicabilityDispositionV1::PredictionCommitted,
            String::new(),
            Some(&prediction),
            Some(101_000_000_000),
            None,
            101_000_000_000,
        )
        .expect("event");
        let mut applicability =
            Ms3FutureApplicabilityLedgerV1::new(contract).expect("applicability");
        applicability.append(event).expect("append");
        let predictions = PredictionLedgerV1 {
            schema: PREDICTION_LEDGER_SCHEMA_V1.to_owned(),
            ledger_root_sha256: test_root("ledger"),
            contract_root_sha256: test_root("contract"),
            opened_at_sequence: 7,
            prediction_min_sequence: 8,
            predictions: Vec::new(),
        };

        assert_eq!(
            validate_prediction_events(&applicability, &predictions),
            Err("ms3_applicability_prediction_missing".to_owned())
        );
    }
}
