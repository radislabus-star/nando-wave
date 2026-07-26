//! Durable owner for one immutable MS3 identification-machine checkpoint.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_learning::multi_source::{
    FrozenVersionSpaceContractV1, FrozenVersionSpaceEnvelopeV1, Ms3FutureApplicabilityContractV1,
    Ms3FutureApplicabilityDispositionV1, Ms3FutureApplicabilityEventV1,
    Ms3FutureApplicabilityLedgerV1, Ms3FutureApplicabilityReportV1, Ms3FutureApplicabilityV1,
    Ms3FuturePredictionV1, Ms3IndependentFutureEnvelopeV1, Ms3VersionSpaceVersionsV1,
    PreActionTopologyAuditRowV1, PreparedMs3VersionSpaceV1, classify_ms3_unique_law_v1,
};
use serde::{Deserialize, Serialize};

const ENVELOPE_FILE: &str = "frozen-version-space-v1.cbor";
const PREDICTIONS_FILE: &str = "future-predictions-v1.cbor";
const FUTURE_FILE: &str = "independent-future-v1.cbor";
const APPLICABILITY_FILE: &str = "future-applicability-v1.cbor";
const MAX_ENVELOPE_BYTES: usize = 12 * 1024 * 1024;
const MAX_PREDICTIONS: usize = 256;
const PREDICTION_LEDGER_SCHEMA_V1: &str = "nando.ms3-future-prediction-ledger.v1";

pub(super) struct Ms3FrozenVersionSpaceRuntime {
    envelope: Option<FrozenVersionSpaceEnvelopeV1>,
    envelope_path: PathBuf,
    prediction_ledger: Option<PredictionLedgerV1>,
    prediction_ledger_path: PathBuf,
    applicability_ledger: Option<Ms3FutureApplicabilityLedgerV1>,
    applicability_ledger_path: PathBuf,
    independent_future: Option<Ms3IndependentFutureEnvelopeV1>,
    independent_future_path: PathBuf,
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
        current_capture_sequence: u64,
        opened_at_unix: u64,
    ) -> Result<Self, String> {
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
        if prediction_ledger.is_none()
            && let Some(frozen) = &envelope
        {
            let prediction_min_sequence = current_capture_sequence
                .checked_add(1)
                .ok_or_else(|| "ms3_prediction_open_watermark_overflow".to_owned())?
                .max(frozen.contract.future_min_sequence);
            let mut ledger = PredictionLedgerV1 {
                schema: PREDICTION_LEDGER_SCHEMA_V1.to_owned(),
                ledger_root_sha256: String::new(),
                contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
                opened_at_sequence: current_capture_sequence,
                prediction_min_sequence,
                predictions: Vec::new(),
            };
            ledger.ledger_root_sha256 = prediction_ledger_root(&ledger)?;
            validate_prediction_ledger(&ledger, frozen)?;
            let bytes = serde_cbor::to_vec(&ledger)
                .map_err(|error| format!("ms3_prediction_ledger_encode:{error}"))?;
            write_atomic(&prediction_ledger_path, &bytes)?;
            prediction_ledger = Some(ledger);
        }
        if let Some(ledger) = &prediction_ledger {
            let frozen = envelope
                .as_ref()
                .ok_or_else(|| "ms3_prediction_ledger_without_contract".to_owned())?;
            validate_prediction_ledger(ledger, frozen)?;
        }
        let applicability_ledger_path = directory.join(APPLICABILITY_FILE);
        let mut applicability_ledger = read_bounded(&applicability_ledger_path)?
            .map(|bytes| {
                Ms3FutureApplicabilityLedgerV1::from_canonical_bytes(&bytes)
                    .map_err(|error| format!("ms3_future_applicability_decode:{error}"))
            })
            .transpose()?;
        if applicability_ledger.is_none()
            && let (Some(frozen), Some(predictions)) = (&envelope, &prediction_ledger)
        {
            let contract = Ms3FutureApplicabilityContractV1::seal(
                frozen.contract.contract_root_sha256.clone(),
                predictions.opened_at_sequence,
                predictions.prediction_min_sequence,
                opened_at_unix,
            )
            .map_err(str::to_owned)?;
            let ledger = Ms3FutureApplicabilityLedgerV1::new(contract).map_err(str::to_owned)?;
            write_atomic(
                &applicability_ledger_path,
                &ledger.canonical_bytes().map_err(str::to_owned)?,
            )?;
            applicability_ledger = Some(ledger);
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
        Ok(Self {
            envelope,
            envelope_path,
            prediction_ledger,
            prediction_ledger_path,
            applicability_ledger,
            applicability_ledger_path,
            independent_future,
            independent_future_path,
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
        let bytes = envelope
            .canonical_bytes()
            .map_err(|error| format!("ms3_version_space_encode:{error}"))?;
        write_atomic(&self.envelope_path, &bytes)?;
        self.envelope = Some(envelope);
        self.open_future_gate(contract_watermark, opened_at_unix)?;
        self.contract()
            .cloned()
            .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())
    }

    pub(super) fn observe_topology(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        predicted_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        self.observe_topology_inner(topology, predicted_at_unix_nanos)
    }

    pub(super) fn observe_historical_topology(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        observed_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        self.observe_topology_inner(topology, observed_at_unix_nanos)
    }

    fn observe_topology_inner(
        &mut self,
        topology: &PreActionTopologyAuditRowV1,
        observed_at_unix_nanos: u64,
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
                    && (event.disposition
                        != Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
                        || event.prediction_root_sha256.is_none())
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
        if self
            .applicability_ledger
            .as_ref()
            .expect("gate checked")
            .report(observed_at_unix_nanos / 1_000_000_000)
            .verdict
            != nando_operator_learning::multi_source::Ms3FutureApplicabilityVerdictV1::Collecting
        {
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

    pub(super) fn prediction_durable_at(&self, prediction_root: &str) -> Option<u64> {
        self.applicability_ledger
            .as_ref()?
            .events
            .iter()
            .find_map(|event| {
                (event.disposition == Ms3FutureApplicabilityDispositionV1::PredictionCommitted
                    && event.prediction_root_sha256.as_deref() == Some(prediction_root))
                .then_some(event.prediction_durable_at_unix_nanos)
                .flatten()
            })
    }

    pub(super) fn prediction_is_disqualified(&self, prediction_root: &str) -> bool {
        self.applicability_ledger.as_ref().is_some_and(|ledger| {
            ledger.events.iter().any(|event| {
                event.disposition
                    == Ms3FutureApplicabilityDispositionV1::PrecommittedPredictionMissing
                    && event.prediction_root_sha256.as_deref() == Some(prediction_root)
            })
        })
    }

    pub(super) fn record_precommitted_prediction_missing(
        &mut self,
        prediction: &Ms3FuturePredictionV1,
        terminal_receipt_root_sha256: &str,
        terminal_completed_at_unix_nanos: u64,
    ) -> Result<bool, String> {
        if self.prediction_is_disqualified(&prediction.prediction_root_sha256) {
            return Ok(false);
        }
        let durable_at = self
            .prediction_durable_at(&prediction.prediction_root_sha256)
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
            )),
            now,
        )
        .map_err(str::to_owned)?;
        self.append_applicability_event(event)
    }

    pub(super) fn seal_independent_future(
        &mut self,
        future: Ms3IndependentFutureEnvelopeV1,
    ) -> Result<(), String> {
        if self.independent_future.is_some() {
            return Ok(());
        }
        let frozen = self
            .envelope
            .as_ref()
            .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())?;
        let bytes = future
            .canonical_bytes(frozen)
            .map_err(|error| format!("ms3_independent_future_encode:{error}"))?;
        write_atomic(&self.independent_future_path, &bytes)?;
        self.independent_future = Some(future);
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

    fn open_future_gate(
        &mut self,
        contract_watermark: u64,
        opened_at_unix: u64,
    ) -> Result<(), String> {
        if self.prediction_ledger.is_none() {
            let frozen = self
                .envelope
                .as_ref()
                .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())?;
            let prediction_min_sequence = contract_watermark
                .checked_add(1)
                .ok_or_else(|| "ms3_prediction_open_watermark_overflow".to_owned())?;
            let mut ledger = PredictionLedgerV1 {
                schema: PREDICTION_LEDGER_SCHEMA_V1.to_owned(),
                ledger_root_sha256: String::new(),
                contract_root_sha256: frozen.contract.contract_root_sha256.clone(),
                opened_at_sequence: contract_watermark,
                prediction_min_sequence,
                predictions: Vec::new(),
            };
            ledger.ledger_root_sha256 = prediction_ledger_root(&ledger)?;
            write_atomic(
                &self.prediction_ledger_path,
                &serde_cbor::to_vec(&ledger)
                    .map_err(|error| format!("ms3_prediction_ledger_encode:{error}"))?,
            )?;
            self.prediction_ledger = Some(ledger);
        }
        if self.applicability_ledger.is_none() {
            let frozen = self
                .envelope
                .as_ref()
                .ok_or_else(|| "ms3_version_space_contract_missing".to_owned())?;
            let predictions = self
                .prediction_ledger
                .as_ref()
                .ok_or_else(|| "ms3_prediction_ledger_missing".to_owned())?;
            let contract = Ms3FutureApplicabilityContractV1::seal(
                frozen.contract.contract_root_sha256.clone(),
                predictions.opened_at_sequence,
                predictions.prediction_min_sequence,
                opened_at_unix,
            )
            .map_err(str::to_owned)?;
            let ledger = Ms3FutureApplicabilityLedgerV1::new(contract).map_err(str::to_owned)?;
            write_atomic(
                &self.applicability_ledger_path,
                &ledger.canonical_bytes().map_err(str::to_owned)?,
            )?;
            self.applicability_ledger = Some(ledger);
        }
        Ok(())
    }

    fn append_applicability_event(
        &mut self,
        event: Ms3FutureApplicabilityEventV1,
    ) -> Result<bool, String> {
        let ledger = self
            .applicability_ledger
            .as_mut()
            .ok_or_else(|| "ms3_future_applicability_missing".to_owned())?;
        if !ledger.append(event).map_err(str::to_owned)? {
            return Ok(false);
        }
        write_atomic(
            &self.applicability_ledger_path,
            &ledger.canonical_bytes().map_err(str::to_owned)?,
        )?;
        Ok(true)
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
