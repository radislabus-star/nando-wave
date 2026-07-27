//! Durable owner for the bounded MS3 linked-frame acquisition experiment.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::RelationFrame;
use nando_operator_learning::multi_source::{
    Ms3LinkedFrameAcquisitionContractV1, Ms3LinkedFrameAcquisitionReportV1,
    PreActionTopologyAuditRowV1, TransportTerminalReceiptV1,
    build_ms3_linked_frame_acquisition_report_v1,
};

use crate::multi_source_topology_archive::MultiSourceTopologyArchive;

const CONTRACT_FILE: &str = "contract-v1.cbor";
const TERMINAL_REPORT_FILE: &str = "terminal-report-v1.cbor";
const MAX_STATE_BYTES: usize = 4 * 1024 * 1024;

pub(super) struct Ms3LinkedFrameAcquisitionRuntime {
    generation_sequence: u64,
    contract: Ms3LinkedFrameAcquisitionContractV1,
    terminal_report: Option<Ms3LinkedFrameAcquisitionReportV1>,
    terminal_report_path: PathBuf,
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
            let watermark_rows =
                u64::try_from(topology_archive.len()).map_err(|_| "ms3_acquisition_rows")?;
            let prefix_root = topology_archive.prefix_root(topology_archive.len())?;
            let contract = Ms3LinkedFrameAcquisitionContractV1::seal(
                prefix_root,
                watermark_rows,
                opened_at_unix,
                max_new_topology_rows,
                max_elapsed_seconds,
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
        let terminal_report = read_bounded(&terminal_report_path)?
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
        Ok(Self {
            generation_sequence,
            contract,
            terminal_report,
            terminal_report_path,
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

    pub(super) fn evaluate(
        &mut self,
        generated_at_unix: u64,
        new_topologies: Vec<PreActionTopologyAuditRowV1>,
        frames: Vec<RelationFrame>,
        terminals: Vec<TransportTerminalReceiptV1>,
    ) -> Result<Ms3LinkedFrameAcquisitionReportV1, String> {
        if let Some(report) = &self.terminal_report {
            return Ok(report.clone());
        }
        let report = build_ms3_linked_frame_acquisition_report_v1(
            self.contract.clone(),
            generated_at_unix,
            new_topologies,
            frames,
            terminals,
        );
        if !report.validate() {
            return Err("ms3_acquisition_report_invalid".to_owned());
        }
        if report.is_terminal() {
            write_cbor_atomic(&self.terminal_report_path, &report)?;
            self.terminal_report = Some(report.clone());
        }
        Ok(report)
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
    file.write_all(&bytes)
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
