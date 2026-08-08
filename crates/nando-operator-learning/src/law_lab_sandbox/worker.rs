use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use nando_operator_kernel::{canonical_json_bytes, canonical_json_sha256};

use super::manifest::{
    LAW_LAB_SANDBOX_SOURCE_WRITE_PROBE_V1, LawLabTreeEntryKindV1, LawLabTreeManifestV1,
    law_lab_sha256_file_v1,
};
use super::model::{
    LAW_LAB_SANDBOX_FORBIDDEN_PATHS_V1, LawLabSandboxEnvironmentEntryV1, LawLabSandboxErrorV1,
    LawLabSandboxIsolationAttestationV1, LawLabSandboxOperationResultV1, LawLabSandboxOperationV1,
    LawLabSandboxRequestV1, LawLabSandboxWorkerOutcomeInputV1, LawLabSandboxWorkerOutcomeV1,
};
use crate::{LAW_LAB_MAX_INPUT_BYTES_V1, LAW_LAB_MAX_OUTPUT_BYTES_V1};

const SOURCE_ROOT_V1: &str = "/source";
const WORK_ROOT_V1: &str = "/work";
const MAX_REQUEST_BYTES_V1: usize = 512 * 1024;

pub fn run_law_lab_sandbox_worker_v1() -> Result<(), LawLabSandboxErrorV1> {
    let request_bytes = read_bounded_v1(std::io::stdin(), MAX_REQUEST_BYTES_V1)?;
    let request: LawLabSandboxRequestV1 = serde_json::from_slice(&request_bytes)
        .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)?;
    request.validate()?;
    if request.canonical_bytes()? != request_bytes {
        return Err(LawLabSandboxErrorV1::WorkerProtocolFailed);
    }

    let source_root = Path::new(SOURCE_ROOT_V1);
    let work_root = Path::new(WORK_ROOT_V1);
    let source_manifest = LawLabTreeManifestV1::scan(source_root, LAW_LAB_MAX_INPUT_BYTES_V1)?;
    if source_manifest.tree_root_sha256 != request.source_tree_root_sha256 {
        return Err(LawLabSandboxErrorV1::SourceManifestMismatch);
    }
    ensure_empty_directory_v1(work_root)?;
    let isolation = collect_isolation_attestation_v1(source_root)?;
    clone_source_tree_v1(source_root, work_root, &source_manifest)?;
    let pre_work_manifest = LawLabTreeManifestV1::scan(work_root, LAW_LAB_MAX_INPUT_BYTES_V1)?;
    if pre_work_manifest != source_manifest {
        return Err(LawLabSandboxErrorV1::SourceManifestMismatch);
    }

    let mut operation_results = Vec::with_capacity(request.operations.len());
    let mut output_bytes_written = 0_u64;
    for (ordinal, operation) in request.operations.iter().enumerate() {
        let bytes_written = apply_operation_v1(source_root, work_root, operation)?;
        output_bytes_written = output_bytes_written
            .checked_add(bytes_written)
            .ok_or(LawLabSandboxErrorV1::TreeBudgetExceeded)?;
        if output_bytes_written > LAW_LAB_MAX_OUTPUT_BYTES_V1 {
            return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
        }
        operation_results.push(operation_result_v1(
            ordinal as u64,
            operation,
            work_root,
            bytes_written,
        )?);
    }

    let post_work_manifest = LawLabTreeManifestV1::scan(
        work_root,
        LAW_LAB_MAX_INPUT_BYTES_V1 + LAW_LAB_MAX_OUTPUT_BYTES_V1,
    )?;
    let worker_sha256 =
        law_lab_sha256_file_v1(&std::env::current_exe().map_err(|_| LawLabSandboxErrorV1::Io)?)?;
    let outcome = LawLabSandboxWorkerOutcomeV1::seal(LawLabSandboxWorkerOutcomeInputV1 {
        request: &request,
        worker_sha256,
        source_manifest,
        pre_work_manifest,
        post_work_manifest,
        operation_results,
        output_bytes_written,
        isolation,
    })?;
    let output = canonical_json_bytes(&outcome).map_err(|_| LawLabSandboxErrorV1::Serialization)?;
    if output.len() > LAW_LAB_MAX_OUTPUT_BYTES_V1 as usize {
        return Err(LawLabSandboxErrorV1::WorkerOutputTooLarge);
    }
    std::io::stdout()
        .write_all(&output)
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    Ok(())
}

fn apply_operation_v1(
    source_root: &Path,
    work_root: &Path,
    operation: &LawLabSandboxOperationV1,
) -> Result<u64, LawLabSandboxErrorV1> {
    operation.validate()?;
    match operation {
        LawLabSandboxOperationV1::CopySourceFile {
            source_path,
            work_path,
        } => {
            let source = source_root.join(source_path);
            let destination = work_root.join(work_path);
            let metadata = fs::symlink_metadata(&source).map_err(|_| LawLabSandboxErrorV1::Io)?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || destination
                    .try_exists()
                    .map_err(|_| LawLabSandboxErrorV1::Io)?
                || metadata.len() > LAW_LAB_MAX_OUTPUT_BYTES_V1
            {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
            create_parent_directories_v1(work_root, &destination)?;
            copy_regular_file_v1(&source, &destination, metadata.permissions().mode())?;
            Ok(metadata.len())
        }
        LawLabSandboxOperationV1::RemoveWorkPath { work_path } => {
            let target = work_root.join(work_path);
            let metadata = fs::symlink_metadata(&target).map_err(|_| LawLabSandboxErrorV1::Io)?;
            if metadata.file_type().is_symlink() {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
            if metadata.is_dir() {
                fs::remove_dir_all(&target).map_err(|_| LawLabSandboxErrorV1::Io)?;
            } else if metadata.is_file() {
                fs::remove_file(&target).map_err(|_| LawLabSandboxErrorV1::Io)?;
            } else {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
            Ok(0)
        }
        LawLabSandboxOperationV1::CanonicalizeJsonFile { work_path } => {
            let target = work_root.join(work_path);
            let metadata = fs::symlink_metadata(&target).map_err(|_| LawLabSandboxErrorV1::Io)?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > LAW_LAB_MAX_OUTPUT_BYTES_V1
            {
                return Err(LawLabSandboxErrorV1::InvalidTree);
            }
            let input = read_bounded_v1(
                File::open(&target).map_err(|_| LawLabSandboxErrorV1::Io)?,
                LAW_LAB_MAX_OUTPUT_BYTES_V1 as usize,
            )?;
            let value: serde_json::Value = serde_json::from_slice(&input)
                .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)?;
            let canonical = canonical_json_bytes(&value)
                .map_err(|_| LawLabSandboxErrorV1::WorkerProtocolFailed)?;
            if canonical.len() > LAW_LAB_MAX_OUTPUT_BYTES_V1 as usize {
                return Err(LawLabSandboxErrorV1::TreeBudgetExceeded);
            }
            write_file_atomic_v1(&target, &canonical, metadata.permissions().mode())?;
            Ok(canonical.len() as u64)
        }
    }
}

fn operation_result_v1(
    ordinal: u64,
    operation: &LawLabSandboxOperationV1,
    work_root: &Path,
    bytes_written: u64,
) -> Result<LawLabSandboxOperationResultV1, LawLabSandboxErrorV1> {
    let operation_root_sha256 =
        canonical_json_sha256(operation).map_err(|_| LawLabSandboxErrorV1::Serialization)?;
    let effect_root_sha256 = match operation {
        LawLabSandboxOperationV1::RemoveWorkPath { work_path } => {
            if work_root
                .join(work_path)
                .try_exists()
                .map_err(|_| LawLabSandboxErrorV1::Io)?
            {
                return Err(LawLabSandboxErrorV1::WorkerOutcomeInvalid);
            }
            canonical_json_sha256(&(
                "nando.law-lab-sandbox-effect.v1",
                ordinal,
                operation_root_sha256.as_str(),
                work_path.as_str(),
                "absent",
            ))
        }
        LawLabSandboxOperationV1::CopySourceFile { work_path, .. }
        | LawLabSandboxOperationV1::CanonicalizeJsonFile { work_path } => {
            let entry = manifest_entry_for_path_v1(work_root, work_path)?;
            canonical_json_sha256(&(
                "nando.law-lab-sandbox-effect.v1",
                ordinal,
                operation_root_sha256.as_str(),
                work_path.as_str(),
                entry,
            ))
        }
    }
    .map_err(|_| LawLabSandboxErrorV1::Serialization)?;
    Ok(LawLabSandboxOperationResultV1 {
        ordinal,
        operation_root_sha256,
        effect_root_sha256,
        bytes_written,
    })
}

fn manifest_entry_for_path_v1(
    work_root: &Path,
    relative_path: &str,
) -> Result<super::manifest::LawLabTreeEntryV1, LawLabSandboxErrorV1> {
    let target = work_root.join(relative_path);
    let metadata = fs::symlink_metadata(&target).map_err(|_| LawLabSandboxErrorV1::Io)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(LawLabSandboxErrorV1::InvalidTree);
    }
    Ok(super::manifest::LawLabTreeEntryV1 {
        relative_path: relative_path.to_owned(),
        kind: LawLabTreeEntryKindV1::File,
        byte_length: metadata.len(),
        content_sha256: Some(law_lab_sha256_file_v1(&target)?),
        executable: metadata.permissions().mode() & 0o111 != 0,
    })
}

fn collect_isolation_attestation_v1(
    source_root: &Path,
) -> Result<LawLabSandboxIsolationAttestationV1, LawLabSandboxErrorV1> {
    let ipv4_non_loopback_route_entries =
        count_non_loopback_routes_v1(Path::new("/proc/net/route"), true)?;
    let ipv6_non_loopback_route_entries =
        count_non_loopback_routes_v1(Path::new("/proc/net/ipv6_route"), false)?;
    let visible_pid_count = fs::read_dir("/proc")
        .map_err(|_| LawLabSandboxErrorV1::Io)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.bytes().all(|byte| byte.is_ascii_digit()))
        })
        .count() as u64;
    let status = fs::read_to_string("/proc/self/status").map_err(|_| LawLabSandboxErrorV1::Io)?;
    let no_new_privileges = status
        .lines()
        .any(|line| line.split_whitespace().eq(["NoNewPrivs:", "1"]));
    let source_write_blocked = source_write_is_blocked_v1(source_root)?;
    let forbidden_paths_absent = LAW_LAB_SANDBOX_FORBIDDEN_PATHS_V1
        .into_iter()
        .all(|path| path_is_absent_v1(Path::new(path)));
    let mut environment = std::env::vars()
        .map(|(name, value)| LawLabSandboxEnvironmentEntryV1 { name, value })
        .collect::<Vec<_>>();
    environment.sort_by(|left, right| left.name.cmp(&right.name));
    let result = LawLabSandboxIsolationAttestationV1::seal(
        ipv4_non_loopback_route_entries,
        ipv6_non_loopback_route_entries,
        visible_pid_count,
        no_new_privileges,
        source_write_blocked,
        forbidden_paths_absent,
        environment,
    );
    if result.is_err() {
        eprintln!(
            "law_lab_isolation ipv4_non_loopback={ipv4_non_loopback_route_entries} ipv6_non_loopback={ipv6_non_loopback_route_entries} pids={visible_pid_count} no_new_privileges={no_new_privileges} source_write_blocked={source_write_blocked} forbidden_paths_absent={forbidden_paths_absent}"
        );
    }
    result
}

fn source_write_is_blocked_v1(source_root: &Path) -> Result<bool, LawLabSandboxErrorV1> {
    let probe = source_root.join(LAW_LAB_SANDBOX_SOURCE_WRITE_PROBE_V1);
    match OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            fs::remove_file(&probe).map_err(|_| LawLabSandboxErrorV1::Io)?;
            Ok(false)
        }
        Err(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(30) =>
        {
            Ok(true)
        }
        Err(_) => Err(LawLabSandboxErrorV1::IsolationVerificationFailed),
    }
}

fn path_is_absent_v1(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Ok(_) | Err(_) => false,
    }
}

fn clone_source_tree_v1(
    source_root: &Path,
    work_root: &Path,
    manifest: &LawLabTreeManifestV1,
) -> Result<(), LawLabSandboxErrorV1> {
    for entry in &manifest.entries {
        let source = source_root.join(&entry.relative_path);
        let destination = work_root.join(&entry.relative_path);
        match entry.kind {
            LawLabTreeEntryKindV1::Directory => {
                fs::create_dir(&destination).map_err(|_| LawLabSandboxErrorV1::Io)?;
                fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
                    .map_err(|_| LawLabSandboxErrorV1::Io)?;
            }
            LawLabTreeEntryKindV1::File => {
                let mode = if entry.executable { 0o700 } else { 0o600 };
                copy_regular_file_v1(&source, &destination, mode)?;
            }
        }
    }
    Ok(())
}

fn copy_regular_file_v1(
    source: &Path,
    destination: &Path,
    source_mode: u32,
) -> Result<(), LawLabSandboxErrorV1> {
    let copied = fs::copy(source, destination).map_err(|_| LawLabSandboxErrorV1::Io)?;
    let source_length = fs::metadata(source)
        .map_err(|_| LawLabSandboxErrorV1::Io)?
        .len();
    if copied != source_length {
        return Err(LawLabSandboxErrorV1::Io);
    }
    let mode = if source_mode & 0o111 != 0 {
        0o700
    } else {
        0o600
    };
    fs::set_permissions(destination, fs::Permissions::from_mode(mode))
        .map_err(|_| LawLabSandboxErrorV1::Io)
}

fn create_parent_directories_v1(
    work_root: &Path,
    destination: &Path,
) -> Result<(), LawLabSandboxErrorV1> {
    let parent = destination
        .parent()
        .ok_or(LawLabSandboxErrorV1::UnsafePath)?;
    if parent == work_root {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|_| LawLabSandboxErrorV1::Io)?;
    let mut current = PathBuf::from(parent);
    while current.starts_with(work_root) && current != work_root {
        fs::set_permissions(&current, fs::Permissions::from_mode(0o700))
            .map_err(|_| LawLabSandboxErrorV1::Io)?;
        let Some(next) = current.parent() else {
            break;
        };
        current = next.to_path_buf();
    }
    Ok(())
}

fn write_file_atomic_v1(
    path: &Path,
    bytes: &[u8],
    original_mode: u32,
) -> Result<(), LawLabSandboxErrorV1> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(LawLabSandboxErrorV1::UnsafePath)?;
    let temporary = path.with_file_name(format!(".{file_name}.nando-law-lab-v1.tmp"));
    if temporary
        .try_exists()
        .map_err(|_| LawLabSandboxErrorV1::Io)?
    {
        return Err(LawLabSandboxErrorV1::InvalidTree);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    file.write_all(bytes)
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    file.sync_all().map_err(|_| LawLabSandboxErrorV1::Io)?;
    drop(file);
    fs::set_permissions(
        &temporary,
        fs::Permissions::from_mode(if original_mode & 0o111 != 0 {
            0o700
        } else {
            0o600
        }),
    )
    .map_err(|_| LawLabSandboxErrorV1::Io)?;
    fs::rename(&temporary, path).map_err(|_| LawLabSandboxErrorV1::Io)
}

fn ensure_empty_directory_v1(path: &Path) -> Result<(), LawLabSandboxErrorV1> {
    let metadata = fs::symlink_metadata(path).map_err(|_| LawLabSandboxErrorV1::Io)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || fs::read_dir(path)
            .map_err(|_| LawLabSandboxErrorV1::Io)?
            .next()
            .is_some()
    {
        return Err(LawLabSandboxErrorV1::InvalidTree);
    }
    Ok(())
}

fn count_non_loopback_routes_v1(
    path: &Path,
    has_header: bool,
) -> Result<u64, LawLabSandboxErrorV1> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(LawLabSandboxErrorV1::Io),
    };
    Ok(contents
        .lines()
        .skip(usize::from(has_header))
        .filter(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            !fields.is_empty()
                && if has_header {
                    fields.first().copied() != Some("lo")
                } else {
                    fields.last().copied() != Some("lo")
                }
        })
        .count() as u64)
}

fn read_bounded_v1(
    reader: impl Read,
    maximum_bytes: usize,
) -> Result<Vec<u8>, LawLabSandboxErrorV1> {
    let mut output = Vec::new();
    reader
        .take(maximum_bytes as u64 + 1)
        .read_to_end(&mut output)
        .map_err(|_| LawLabSandboxErrorV1::Io)?;
    if output.len() > maximum_bytes {
        return Err(LawLabSandboxErrorV1::WorkerOutputTooLarge);
    }
    Ok(output)
}
