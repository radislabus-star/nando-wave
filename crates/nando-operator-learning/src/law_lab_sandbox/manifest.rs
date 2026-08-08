use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::model::{
    LAW_LAB_SANDBOX_MAX_PATH_BYTES_V1, LawLabSandboxEnvironmentEntryV1, LawLabSandboxErrorV1,
    deterministic_environment_v1, validate_relative_path_v1,
};
use crate::{
    LAW_LAB_MAX_DISK_BYTES_V1, LAW_LAB_MAX_INPUT_BYTES_V1, LAW_LAB_MAX_MEMORY_BYTES_V1,
    LAW_LAB_MAX_OUTPUT_BYTES_V1, LAW_LAB_MAX_PROBE_CPU_MS_V1, LAW_LAB_MAX_PROBE_WALL_MS_V1,
    LAW_LAB_MAX_PROCESSES_V1, LawLabContractV1, LawLabProbeDomainV1,
};

pub const LAW_LAB_TREE_MANIFEST_SCHEMA_V1: &str = "nando.law-lab-tree-manifest.v1";
pub const LAW_LAB_SANDBOX_EXECUTOR_MANIFEST_SCHEMA_V1: &str =
    "nando.law-lab-sandbox-executor-manifest.v1";
pub const LAW_LAB_SANDBOX_ADAPTER_VERSION_V1: u64 = 1;
pub const LAW_LAB_SANDBOX_MAX_TREE_ENTRIES_V1: usize = 1_024;
pub const LAW_LAB_SANDBOX_SOURCE_WRITE_PROBE_V1: &str = ".nando-law-lab-source-write-probe-v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LawLabTreeEntryKindV1 {
    Directory,
    File,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabTreeEntryV1 {
    pub relative_path: String,
    pub kind: LawLabTreeEntryKindV1,
    pub byte_length: u64,
    pub content_sha256: Option<String>,
    pub executable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabTreeManifestV1 {
    pub schema: String,
    pub tree_root_sha256: String,
    pub total_file_bytes: u64,
    pub entries: Vec<LawLabTreeEntryV1>,
}

#[derive(Serialize)]
struct LawLabTreeManifestDigestV1<'a> {
    schema: &'static str,
    total_file_bytes: u64,
    entries: &'a [LawLabTreeEntryV1],
}

impl LawLabTreeManifestV1 {
    pub fn scan(root: &Path, maximum_bytes: u64) -> Result<Self, LawLabSandboxErrorV1> {
        let metadata = fs::symlink_metadata(root).map_err(|_| LawLabSandboxErrorV1::Io)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
        let mut entries = Vec::new();
        let mut total_file_bytes = 0_u64;
        scan_directory_v1(root, "", maximum_bytes, &mut total_file_bytes, &mut entries)?;
        entries.sort();
        let mut manifest = Self {
            schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1.to_owned(),
            tree_root_sha256: String::new(),
            total_file_bytes,
            entries,
        };
        manifest.tree_root_sha256 = manifest.expected_root()?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        if self.schema != LAW_LAB_TREE_MANIFEST_SCHEMA_V1
            || self.entries.len() > LAW_LAB_SANDBOX_MAX_TREE_ENTRIES_V1
            || self.entries.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
        let mut total = 0_u64;
        for entry in &self.entries {
            validate_relative_path_v1(&entry.relative_path)?;
            if entry.relative_path.len() > LAW_LAB_SANDBOX_MAX_PATH_BYTES_V1
                || entry.relative_path == LAW_LAB_SANDBOX_SOURCE_WRITE_PROBE_V1
            {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
            match entry.kind {
                LawLabTreeEntryKindV1::Directory => {
                    if entry.byte_length != 0 || entry.content_sha256.is_some() || entry.executable
                    {
                        return Err(LawLabSandboxErrorV1::InvalidTree);
                    }
                }
                LawLabTreeEntryKindV1::File => {
                    if entry
                        .content_sha256
                        .as_deref()
                        .is_none_or(|root| !valid_nonzero_sha256(root))
                    {
                        return Err(LawLabSandboxErrorV1::InvalidTree);
                    }
                    total = total
                        .checked_add(entry.byte_length)
                        .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
                }
            }
            if let Some((parent, _)) = entry.relative_path.rsplit_once('/')
                && !self.entries.iter().any(|candidate| {
                    candidate.relative_path == parent
                        && candidate.kind == LawLabTreeEntryKindV1::Directory
                })
            {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
        }
        if total != self.total_file_bytes || self.tree_root_sha256 != self.expected_root()? {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
        Ok(())
    }

    #[must_use]
    pub fn entry(&self, relative_path: &str) -> Option<&LawLabTreeEntryV1> {
        self.entries
            .iter()
            .find(|entry| entry.relative_path == relative_path)
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabTreeManifestDigestV1 {
            schema: LAW_LAB_TREE_MANIFEST_SCHEMA_V1,
            total_file_bytes: self.total_file_bytes,
            entries: &self.entries,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxLimitsV1 {
    pub wall_ms: u64,
    pub cpu_ms: u64,
    pub address_space_bytes: u64,
    pub process_count: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    pub disk_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LawLabSandboxExecutorManifestV1 {
    pub schema: String,
    pub manifest_root_sha256: String,
    pub contract_root_sha256: String,
    pub adapter_version: u64,
    pub bwrap_host_path: String,
    pub bwrap_sha256: String,
    pub prlimit_host_path: String,
    pub prlimit_sha256: String,
    pub worker_host_path: String,
    pub worker_sha256: String,
    pub source_store_host_path: String,
    pub workspace_store_host_path: String,
    pub root_owned_worker_required: bool,
    pub root_owned_source_snapshot_required: bool,
    pub content_addressed_worker_path_required: bool,
    pub generated_capability_fixture_only: bool,
    pub supported_domains: Vec<LawLabProbeDomainV1>,
    pub runtime_read_only_binds: Vec<String>,
    pub deterministic_environment: Vec<LawLabSandboxEnvironmentEntryV1>,
    pub limits: LawLabSandboxLimitsV1,
    pub network_enabled: bool,
    pub shell_interpretation_allowed: bool,
    pub production_state_mount_allowed: bool,
    pub source_snapshot_read_only: bool,
    pub disposable_workspace_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LawLabSandboxExecutorManifestInputV1 {
    pub bwrap_host_path: String,
    pub bwrap_sha256: String,
    pub prlimit_host_path: String,
    pub prlimit_sha256: String,
    pub worker_host_path: String,
    pub worker_sha256: String,
    pub source_store_host_path: String,
    pub workspace_store_host_path: String,
    pub root_owned_worker_required: bool,
    pub root_owned_source_snapshot_required: bool,
    pub content_addressed_worker_path_required: bool,
    pub generated_capability_fixture_only: bool,
    pub runtime_read_only_binds: Vec<String>,
    pub wall_ms: u64,
}

#[derive(Serialize)]
struct LawLabSandboxExecutorManifestDigestV1<'a> {
    schema: &'static str,
    contract_root_sha256: &'a str,
    adapter_version: u64,
    bwrap_host_path: &'a str,
    bwrap_sha256: &'a str,
    prlimit_host_path: &'a str,
    prlimit_sha256: &'a str,
    worker_host_path: &'a str,
    worker_sha256: &'a str,
    source_store_host_path: &'a str,
    workspace_store_host_path: &'a str,
    root_owned_worker_required: bool,
    root_owned_source_snapshot_required: bool,
    content_addressed_worker_path_required: bool,
    generated_capability_fixture_only: bool,
    supported_domains: &'a [LawLabProbeDomainV1],
    runtime_read_only_binds: &'a [String],
    deterministic_environment: &'a [LawLabSandboxEnvironmentEntryV1],
    limits: LawLabSandboxLimitsV1,
    network_enabled: bool,
    shell_interpretation_allowed: bool,
    production_state_mount_allowed: bool,
    source_snapshot_read_only: bool,
    disposable_workspace_required: bool,
}

impl LawLabSandboxExecutorManifestV1 {
    pub(crate) fn seal(
        input: LawLabSandboxExecutorManifestInputV1,
    ) -> Result<Self, LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        let mut manifest = Self {
            schema: LAW_LAB_SANDBOX_EXECUTOR_MANIFEST_SCHEMA_V1.to_owned(),
            manifest_root_sha256: String::new(),
            contract_root_sha256: contract.contract_root_sha256,
            adapter_version: LAW_LAB_SANDBOX_ADAPTER_VERSION_V1,
            bwrap_host_path: input.bwrap_host_path,
            bwrap_sha256: input.bwrap_sha256,
            prlimit_host_path: input.prlimit_host_path,
            prlimit_sha256: input.prlimit_sha256,
            worker_host_path: input.worker_host_path,
            worker_sha256: input.worker_sha256,
            source_store_host_path: input.source_store_host_path,
            workspace_store_host_path: input.workspace_store_host_path,
            root_owned_worker_required: input.root_owned_worker_required,
            root_owned_source_snapshot_required: input.root_owned_source_snapshot_required,
            content_addressed_worker_path_required: input.content_addressed_worker_path_required,
            generated_capability_fixture_only: input.generated_capability_fixture_only,
            supported_domains: vec![
                LawLabProbeDomainV1::Filesystem,
                LawLabProbeDomainV1::StructuredData,
            ],
            runtime_read_only_binds: input.runtime_read_only_binds,
            deterministic_environment: deterministic_environment_v1(),
            limits: LawLabSandboxLimitsV1 {
                wall_ms: input.wall_ms,
                cpu_ms: LAW_LAB_MAX_PROBE_CPU_MS_V1,
                address_space_bytes: LAW_LAB_MAX_MEMORY_BYTES_V1,
                process_count: LAW_LAB_MAX_PROCESSES_V1,
                input_bytes: LAW_LAB_MAX_INPUT_BYTES_V1,
                output_bytes: LAW_LAB_MAX_OUTPUT_BYTES_V1,
                disk_bytes: LAW_LAB_MAX_DISK_BYTES_V1,
            },
            network_enabled: false,
            shell_interpretation_allowed: false,
            production_state_mount_allowed: false,
            source_snapshot_read_only: true,
            disposable_workspace_required: true,
        };
        manifest.manifest_root_sha256 = manifest.expected_root()?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), LawLabSandboxErrorV1> {
        let contract = LawLabContractV1::preregistered_v1()
            .map_err(|_| LawLabSandboxErrorV1::ContractInvalid)?;
        if self.schema != LAW_LAB_SANDBOX_EXECUTOR_MANIFEST_SCHEMA_V1
            || self.contract_root_sha256 != contract.contract_root_sha256
            || self.adapter_version != LAW_LAB_SANDBOX_ADAPTER_VERSION_V1
            || [
                self.bwrap_sha256.as_str(),
                self.prlimit_sha256.as_str(),
                self.worker_sha256.as_str(),
                self.manifest_root_sha256.as_str(),
            ]
            .into_iter()
            .any(|root| !valid_nonzero_sha256(root))
            || self.bwrap_host_path != "/usr/bin/bwrap"
            || self.prlimit_host_path != "/usr/bin/prlimit"
            || !Path::new(&self.worker_host_path).is_absolute()
            || !Path::new(&self.source_store_host_path).is_absolute()
            || !Path::new(&self.workspace_store_host_path).is_absolute()
            || Path::new(&self.source_store_host_path) == Path::new(&self.workspace_store_host_path)
            || Path::new(&self.source_store_host_path)
                .starts_with(Path::new(&self.workspace_store_host_path))
            || Path::new(&self.workspace_store_host_path)
                .starts_with(Path::new(&self.source_store_host_path))
            || self.supported_domains
                != [
                    LawLabProbeDomainV1::Filesystem,
                    LawLabProbeDomainV1::StructuredData,
                ]
            || self.runtime_read_only_binds.is_empty()
            || !self
                .runtime_read_only_binds
                .iter()
                .any(|path| path == "/usr")
            || self
                .runtime_read_only_binds
                .iter()
                .any(|path| !matches!(path.as_str(), "/usr" | "/lib" | "/lib64"))
            || self
                .runtime_read_only_binds
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || self.deterministic_environment != deterministic_environment_v1()
            || self.limits.wall_ms == 0
            || self.limits.wall_ms > LAW_LAB_MAX_PROBE_WALL_MS_V1
            || self.limits.cpu_ms != LAW_LAB_MAX_PROBE_CPU_MS_V1
            || self.limits.address_space_bytes != LAW_LAB_MAX_MEMORY_BYTES_V1
            || self.limits.process_count != LAW_LAB_MAX_PROCESSES_V1
            || self.limits.input_bytes != LAW_LAB_MAX_INPUT_BYTES_V1
            || self.limits.output_bytes != LAW_LAB_MAX_OUTPUT_BYTES_V1
            || self.limits.disk_bytes != LAW_LAB_MAX_DISK_BYTES_V1
            || self.network_enabled
            || self.shell_interpretation_allowed
            || self.production_state_mount_allowed
            || !self.source_snapshot_read_only
            || !self.disposable_workspace_required
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(LawLabSandboxErrorV1::ExecutorManifestInvalid);
        }
        Ok(())
    }

    fn expected_root(&self) -> Result<String, LawLabSandboxErrorV1> {
        canonical_json_sha256(&LawLabSandboxExecutorManifestDigestV1 {
            schema: LAW_LAB_SANDBOX_EXECUTOR_MANIFEST_SCHEMA_V1,
            contract_root_sha256: &self.contract_root_sha256,
            adapter_version: self.adapter_version,
            bwrap_host_path: &self.bwrap_host_path,
            bwrap_sha256: &self.bwrap_sha256,
            prlimit_host_path: &self.prlimit_host_path,
            prlimit_sha256: &self.prlimit_sha256,
            worker_host_path: &self.worker_host_path,
            worker_sha256: &self.worker_sha256,
            source_store_host_path: &self.source_store_host_path,
            workspace_store_host_path: &self.workspace_store_host_path,
            root_owned_worker_required: self.root_owned_worker_required,
            root_owned_source_snapshot_required: self.root_owned_source_snapshot_required,
            content_addressed_worker_path_required: self.content_addressed_worker_path_required,
            generated_capability_fixture_only: self.generated_capability_fixture_only,
            supported_domains: &self.supported_domains,
            runtime_read_only_binds: &self.runtime_read_only_binds,
            deterministic_environment: &self.deterministic_environment,
            limits: self.limits,
            network_enabled: self.network_enabled,
            shell_interpretation_allowed: self.shell_interpretation_allowed,
            production_state_mount_allowed: self.production_state_mount_allowed,
            source_snapshot_read_only: self.source_snapshot_read_only,
            disposable_workspace_required: self.disposable_workspace_required,
        })
        .map_err(|_| LawLabSandboxErrorV1::Serialization)
    }
}

pub fn law_lab_sha256_file_v1(path: &Path) -> Result<String, LawLabSandboxErrorV1> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LawLabSandboxErrorV1::Io)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(LawLabSandboxErrorV1::ToolUntrusted);
    }
    let mut file = File::open(path).map_err(|_| LawLabSandboxErrorV1::Io)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn scan_directory_v1(
    root: &Path,
    relative_parent: &str,
    maximum_bytes: u64,
    total_file_bytes: &mut u64,
    entries: &mut Vec<LawLabTreeEntryV1>,
) -> Result<(), LawLabSandboxErrorV1> {
    let mut children = fs::read_dir(root)
        .map_err(|_| LawLabSandboxErrorV1::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        if entries.len() >= LAW_LAB_SANDBOX_MAX_TREE_ENTRIES_V1 {
            return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
        }
        let name = child
            .file_name()
            .into_string()
            .map_err(|_| LawLabSandboxErrorV1::InvalidTree)?;
        let relative_path = if relative_parent.is_empty() {
            name
        } else {
            format!("{relative_parent}/{name}")
        };
        validate_relative_path_v1(&relative_path)?;
        if relative_path == LAW_LAB_SANDBOX_SOURCE_WRITE_PROBE_V1 {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
        let path = child.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| LawLabSandboxErrorV1::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
        if metadata.is_dir() {
            entries.push(LawLabTreeEntryV1 {
                relative_path: relative_path.clone(),
                kind: LawLabTreeEntryKindV1::Directory,
                byte_length: 0,
                content_sha256: None,
                executable: false,
            });
            scan_directory_v1(
                &path,
                &relative_path,
                maximum_bytes,
                total_file_bytes,
                entries,
            )?;
        } else if metadata.is_file() {
            *total_file_bytes = total_file_bytes
                .checked_add(metadata.len())
                .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
            if *total_file_bytes > maximum_bytes {
                return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
            }
            entries.push(LawLabTreeEntryV1 {
                relative_path,
                kind: LawLabTreeEntryKindV1::File,
                byte_length: metadata.len(),
                content_sha256: Some(law_lab_sha256_file_v1(&path)?),
                executable: metadata.permissions().mode() & 0o111 != 0,
            });
        } else {
            return Err(LawLabSandboxErrorV1::InvalidTree);
        }
    }
    Ok(())
}
