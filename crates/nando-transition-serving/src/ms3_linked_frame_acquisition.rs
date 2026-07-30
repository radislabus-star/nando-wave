//! Durable owner for the bounded MS3 linked-frame acquisition experiment.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::RelationFrame;
use nando_operator_learning::multi_source::{
    MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2, MS3_RECEIPT_LAG_SLO_SECONDS_V1,
    Ms3LinkedFrameAcquisitionContractV1, Ms3LinkedFrameAcquisitionReportV1,
    Ms3ScientificDenominatorEnvelopeV1, Ms3ScientificDenominatorReceiptV1,
    Ms3ScientificDenominatorReconstructionV1, PreActionTopologyAuditRowV1,
    TransportTerminalReceiptV1,
    build_ms3_linked_frame_acquisition_report_with_route_bound_evidence_v1,
    build_ms3_scientific_denominator_receipt_v1, close_ms3_pre_route_receipt_epoch_v1,
};
use std::collections::BTreeSet;

use crate::multi_source_topology_archive::MultiSourceTopologyArchive;

const CONTRACT_FILE: &str = "contract-v1.cbor";
const TERMINAL_REPORT_FILE: &str = "terminal-report-v1.cbor";
const SCIENTIFIC_DENOMINATOR_FILE: &str = "scientific-denominator-v1.cbor";
const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;
const RAW_SCAN_MULTIPLIER_V2: u64 = 16;
const MAX_RAW_SCAN_ROWS_V2: u64 = 4_096;

pub(super) struct Ms3LinkedFrameAcquisitionRuntime {
    generation_sequence: u64,
    contract: Ms3LinkedFrameAcquisitionContractV1,
    terminal_report: Option<Ms3LinkedFrameAcquisitionReportV1>,
    terminal_report_path: PathBuf,
    scientific_denominator: Option<Ms3ScientificDenominatorEnvelopeV1>,
    scientific_denominator_path: PathBuf,
}

impl Ms3LinkedFrameAcquisitionRuntime {
    #[cfg(test)]
    pub(super) fn open(
        directory: &Path,
        topology_archive: &MultiSourceTopologyArchive,
        opened_at_unix: u64,
        max_new_topology_rows: u64,
        max_elapsed_seconds: u64,
    ) -> Result<Self, String> {
        Self::open_generation(
            directory,
            1,
            topology_archive,
            opened_at_unix,
            max_new_topology_rows,
            max_elapsed_seconds,
        )
    }

    pub(super) fn open_generation(
        directory: &Path,
        generation_sequence: u64,
        topology_archive: &MultiSourceTopologyArchive,
        opened_at_unix: u64,
        max_new_topology_rows: u64,
        max_elapsed_seconds: u64,
    ) -> Result<Self, String> {
        Self::open_generation_at_cursor(
            directory,
            generation_sequence,
            topology_archive,
            None,
            opened_at_unix,
            max_new_topology_rows,
            max_elapsed_seconds,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_generation_at_cursor(
        directory: &Path,
        generation_sequence: u64,
        topology_archive: &MultiSourceTopologyArchive,
        topology_cursor_rows: Option<u64>,
        opened_at_unix: u64,
        max_new_topology_rows: u64,
        max_elapsed_seconds: u64,
    ) -> Result<Self, String> {
        if generation_sequence == 0 {
            return Err("ms3_acquisition_generation_invalid".to_owned());
        }
        fs::create_dir_all(directory)
            .map_err(|error| format!("ms3_acquisition_directory:{error}"))?;
        let contract_path = directory.join(CONTRACT_FILE);
        let contract = if let Some(bytes) = read_bounded(&contract_path)? {
            serde_cbor::from_slice::<Ms3LinkedFrameAcquisitionContractV1>(&bytes)
                .map_err(|error| format!("ms3_acquisition_contract_decode:{error}"))?
        } else {
            let watermark_rows = topology_cursor_rows.unwrap_or(
                u64::try_from(topology_archive.len()).map_err(|_| "ms3_acquisition_rows")?,
            );
            let watermark = usize::try_from(watermark_rows).map_err(|_| "ms3_acquisition_rows")?;
            let prefix_root = topology_archive.prefix_root(watermark)?;
            let max_raw_topology_rows = max_new_topology_rows
                .saturating_mul(RAW_SCAN_MULTIPLIER_V2)
                .clamp(max_new_topology_rows, MAX_RAW_SCAN_ROWS_V2);
            let contract = Ms3LinkedFrameAcquisitionContractV1::seal_v3(
                prefix_root,
                watermark_rows,
                opened_at_unix,
                max_new_topology_rows,
                max_raw_topology_rows,
                max_elapsed_seconds,
                MS3_RECEIPT_LAG_SLO_SECONDS_V1.min(max_elapsed_seconds),
            )
            .map_err(str::to_owned)?;
            write_cbor_atomic(&contract_path, &contract)?;
            contract
        };
        if !contract.validate()
            || usize::try_from(contract.topology_watermark_rows)
                .ok()
                .filter(|rows| *rows <= topology_archive.len())
                .is_none()
            || topology_archive.prefix_root(
                usize::try_from(contract.topology_watermark_rows)
                    .map_err(|_| "ms3_acquisition_watermark_range")?,
            )? != contract.topology_prefix_root_sha256
        {
            return Err("ms3_acquisition_contract_invalid".to_owned());
        }
        let terminal_report_path = directory.join(TERMINAL_REPORT_FILE);
        let mut terminal_report = read_bounded(&terminal_report_path)?
            .map(|bytes| {
                serde_cbor::from_slice::<Ms3LinkedFrameAcquisitionReportV1>(&bytes)
                    .map_err(|error| format!("ms3_acquisition_report_decode:{error}"))
            })
            .transpose()?;
        if terminal_report.as_ref().is_some_and(|report| {
            !report.validate()
                || !report.is_terminal()
                || report.acquisition_contract.contract_root_sha256 != contract.contract_root_sha256
        }) {
            return Err("ms3_acquisition_terminal_report_invalid".to_owned());
        }
        let scientific_denominator_path = directory.join(SCIENTIFIC_DENOMINATOR_FILE);
        let scientific_denominator = read_bounded(&scientific_denominator_path)?
            .map(|bytes| {
                Ms3ScientificDenominatorEnvelopeV1::from_canonical_bytes(&bytes)
                    .map_err(str::to_owned)
            })
            .transpose()?;
        if let Some(envelope) = &scientific_denominator {
            if envelope.report.acquisition_contract.contract_root_sha256
                != contract.contract_root_sha256
                || terminal_report
                    .as_ref()
                    .is_some_and(|report| report != &envelope.report)
            {
                return Err("ms3_scientific_denominator_binding_invalid".to_owned());
            }
            if terminal_report.is_none() {
                write_cbor_atomic(&terminal_report_path, &envelope.report)?;
                terminal_report = Some(envelope.report.clone());
            }
        }
        Ok(Self {
            generation_sequence,
            contract,
            terminal_report,
            terminal_report_path,
            scientific_denominator,
            scientific_denominator_path,
        })
    }

    pub(super) const fn generation_sequence(&self) -> u64 {
        self.generation_sequence
    }

    pub(super) const fn contract(&self) -> &Ms3LinkedFrameAcquisitionContractV1 {
        &self.contract
    }

    pub(super) const fn is_terminal(&self) -> bool {
        self.terminal_report.is_some()
    }

    pub(super) fn terminal_report(&self) -> Option<Ms3LinkedFrameAcquisitionReportV1> {
        self.terminal_report.clone()
    }

    pub(super) fn frozen_evaluated_topology_rows(&self) -> Option<u64> {
        self.terminal_report
            .as_ref()
            .map(|report| report.evaluated_topology_rows)
    }

    pub(super) fn consumed_topology_cursor_rows(&self) -> Option<u64> {
        self.terminal_report.as_ref().map(|report| {
            if report.consumed_topology_cursor_rows > 0 {
                report.consumed_topology_cursor_rows
            } else {
                report
                    .acquisition_contract
                    .topology_watermark_rows
                    .saturating_add(report.evaluated_topology_rows)
            }
        })
    }

    pub(super) fn scientific_denominator_receipt(
        &self,
    ) -> Option<Ms3ScientificDenominatorReceiptV1> {
        self.scientific_denominator
            .as_ref()
            .map(|envelope| envelope.receipt.clone())
    }

    #[cfg(test)]
    pub(super) fn evaluate(
        &mut self,
        generated_at_unix: u64,
        new_topologies: Vec<PreActionTopologyAuditRowV1>,
        frames: Vec<RelationFrame>,
        terminals: Vec<TransportTerminalReceiptV1>,
    ) -> Result<Ms3LinkedFrameAcquisitionReportV1, String> {
        let route_bound_frame_roots = frames
            .iter()
            .filter_map(|frame| nando_operator_kernel::canonical_json_sha256(frame).ok())
            .collect();
        self.evaluate_with_route_bound_evidence(
            generated_at_unix,
            new_topologies,
            frames,
            terminals,
            &BTreeSet::new(),
            &route_bound_frame_roots,
        )
    }

    #[cfg(test)]
    pub(super) fn evaluate_excluding_used_evidence(
        &mut self,
        generated_at_unix: u64,
        new_topologies: Vec<PreActionTopologyAuditRowV1>,
        frames: Vec<RelationFrame>,
        terminals: Vec<TransportTerminalReceiptV1>,
        used_evidence_roots: &BTreeSet<String>,
    ) -> Result<Ms3LinkedFrameAcquisitionReportV1, String> {
        self.evaluate_with_route_bound_evidence(
            generated_at_unix,
            new_topologies,
            frames,
            terminals,
            used_evidence_roots,
            &BTreeSet::new(),
        )
    }

    pub(super) fn evaluate_with_route_bound_evidence(
        &mut self,
        generated_at_unix: u64,
        new_topologies: Vec<PreActionTopologyAuditRowV1>,
        frames: Vec<RelationFrame>,
        terminals: Vec<TransportTerminalReceiptV1>,
        used_evidence_roots: &BTreeSet<String>,
        route_bound_frame_roots: &BTreeSet<String>,
    ) -> Result<Ms3LinkedFrameAcquisitionReportV1, String> {
        if let Some(report) = &self.terminal_report {
            return Ok(report.clone());
        }
        if let Some(envelope) = &self.scientific_denominator {
            let report = envelope.report.clone();
            write_cbor_atomic(&self.terminal_report_path, &report)?;
            self.terminal_report = Some(report.clone());
            return Ok(report);
        }
        let report = build_ms3_linked_frame_acquisition_report_with_route_bound_evidence_v1(
            self.contract.clone(),
            generated_at_unix,
            new_topologies.clone(),
            frames.clone(),
            terminals.clone(),
            used_evidence_roots,
            route_bound_frame_roots,
        );
        let report = if self.contract.schema == MS3_LINKED_FRAME_ACQUISITION_CONTRACT_SCHEMA_V2 {
            match close_ms3_pre_route_receipt_epoch_v1(report.clone()) {
                Ok(closed) => closed,
                Err(_) => report,
            }
        } else {
            report
        };
        if !report.validate() {
            return Err("ms3_acquisition_report_invalid".to_owned());
        }
        if report.is_terminal() {
            let scientific_denominator = if self.contract.uses_route_settlement_policy() {
                let receipt = build_ms3_scientific_denominator_receipt_v1(
                    &report,
                    &new_topologies,
                    &frames,
                    &terminals,
                    route_bound_frame_roots,
                    Ms3ScientificDenominatorReconstructionV1::AtomicAtReport,
                )
                .map_err(str::to_owned)?;
                let envelope = Ms3ScientificDenominatorEnvelopeV1::seal(report.clone(), receipt)
                    .map_err(str::to_owned)?;
                let bytes = envelope.canonical_bytes().map_err(str::to_owned)?;
                write_bytes_atomic(&self.scientific_denominator_path, &bytes)?;
                Some(envelope)
            } else {
                None
            };
            if let Some(envelope) = scientific_denominator {
                self.scientific_denominator = Some(envelope);
            }
            write_cbor_atomic(&self.terminal_report_path, &report)?;
            self.terminal_report = Some(report.clone());
        }
        Ok(report)
    }

    pub(super) fn ensure_scientific_denominator(
        &mut self,
        topologies: &[PreActionTopologyAuditRowV1],
        frames: &[RelationFrame],
        terminals: &[TransportTerminalReceiptV1],
        route_bound_frame_roots: &BTreeSet<String>,
    ) -> Result<Ms3ScientificDenominatorReceiptV1, String> {
        if let Some(envelope) = &self.scientific_denominator {
            return Ok(envelope.receipt.clone());
        }
        let report = self
            .terminal_report
            .as_ref()
            .ok_or_else(|| "ms3_scientific_denominator_report_missing".to_owned())?;
        let receipt = build_ms3_scientific_denominator_receipt_v1(
            report,
            topologies,
            frames,
            terminals,
            route_bound_frame_roots,
            Ms3ScientificDenominatorReconstructionV1::AppendOnlyCountEquivalence,
        )
        .or_else(|error| {
            if error != "ms3_scientific_denominator_reconstruction_mismatch" {
                return Err(error);
            }
            let report_route_roots = report
                .receipts
                .iter()
                .map(|receipt| receipt.completed_frame_root_sha256.clone())
                .collect::<BTreeSet<_>>();
            if report_route_roots.is_empty()
                || u64::try_from(report_route_roots.len()).unwrap_or(u64::MAX)
                    != report.relevant_verified_frame_rows
            {
                return Err(error);
            }
            build_ms3_scientific_denominator_receipt_v1(
                report,
                topologies,
                frames,
                terminals,
                &report_route_roots,
                Ms3ScientificDenominatorReconstructionV1::ReportRootClosure,
            )
        })
        .map_err(str::to_owned)?;
        let envelope = Ms3ScientificDenominatorEnvelopeV1::seal(report.clone(), receipt.clone())
            .map_err(str::to_owned)?;
        let bytes = envelope.canonical_bytes().map_err(str::to_owned)?;
        write_bytes_atomic(&self.scientific_denominator_path, &bytes)?;
        self.scientific_denominator = Some(envelope);
        Ok(receipt)
    }
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("ms3_acquisition_open:{}:{error}", path.display())),
    };
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(u64::try_from(MAX_STATE_BYTES + 1).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("ms3_acquisition_read:{}:{error}", path.display()))?;
    if bytes.is_empty() || bytes.len() > MAX_STATE_BYTES {
        return Err("ms3_acquisition_state_budget".to_owned());
    }
    Ok(Some(bytes))
}

fn write_cbor_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes =
        serde_cbor::to_vec(value).map_err(|error| format!("ms3_acquisition_encode:{error}"))?;
    write_bytes_atomic(path, &bytes)
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_STATE_BYTES {
        return Err("ms3_acquisition_state_budget".to_owned());
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("ms3_acquisition_write_open:{error}"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("ms3_acquisition_write:{error}"))?;
    fs::rename(&temporary, path).map_err(|error| format!("ms3_acquisition_rename:{error}"))?;
    File::open(
        path.parent()
            .ok_or_else(|| "ms3_acquisition_parent_missing".to_owned())?,
    )
    .and_then(|directory| directory.sync_all())
    .map_err(|error| format!("ms3_acquisition_directory_sync:{error}"))
}

#[cfg(test)]
#[path = "ms3_linked_frame_acquisition_tests.rs"]
mod tests;
