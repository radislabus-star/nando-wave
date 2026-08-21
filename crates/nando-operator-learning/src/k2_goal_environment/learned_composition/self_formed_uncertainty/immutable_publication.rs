use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1, valid_composition_path_v1,
};
pub const K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1: usize = 1024 * 1024 - 1;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyImmutablePublicationFaultV1 {
    None,
    BeforePublish(u64),
    AfterPublish(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyImmutableFileV1 {
    pub bytes: Vec<u8>,
    pub unix_mode: u32,
    pub byte_len: u64,
    pub content_sha256: String,
    pub device: u64,
    pub inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct K2UncertaintyImmutableCustodyV1 {
    pub unix_mode: u32,
    pub byte_len: u64,
    pub device: u64,
    pub inode: u64,
    pub link_count: u64,
}

pub fn require_private_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_immutable_directory"))?;
    let canonical =
        fs::canonicalize(path).map_err(|_| K2CompositionErrorV1::Io("canonicalize_self_formed_immutable_directory"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o777 != 0o700
        || canonical != path
    {
        return Err(invalid_v1("self_formed_immutable_directory_invalid"));
    }
    Ok(())
}

pub(super) fn read_bounded_jsonl_line_v1<R: BufRead>(
    reader: &mut R,
    total_bytes: &mut u64,
    max_line_bytes: usize,
    max_total_bytes: u64,
) -> K2CompositionResultV1<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|_| K2CompositionErrorV1::Io("read_self_formed_bounded_jsonl"))?;
        if available.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(invalid_v1("self_formed_bounded_jsonl_line_unterminated"))
            };
        }
        let take = available.iter().position(|byte| *byte == b'\n').map_or(available.len(), |index| index + 1);
        if line.len() + take > max_line_bytes + 1 {
            return Err(invalid_v1("self_formed_bounded_jsonl_line_oversized"));
        }
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        *total_bytes += take as u64;
        if *total_bytes > max_total_bytes {
            return Err(invalid_v1("self_formed_bounded_jsonl_oversized"));
        }
        if line.last() == Some(&b'\n') {
            line.pop();
            return if line.is_empty() {
                Err(invalid_v1("self_formed_bounded_jsonl_empty_line"))
            } else {
                Ok(Some(line))
            };
        }
    }
}

pub(super) fn decode_canonical_json_v1<T>(bytes: &[u8]) -> K2CompositionResultV1<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let value = super::uncertainty_decode_v1(bytes)?;
    if super::uncertainty_bytes_v1(&value)? != bytes {
        return Err(invalid_v1("self_formed_canonical_json_invalid"));
    }
    Ok(value)
}

pub(super) fn canonical_jsonl_line_v1<T: serde::Serialize>(
    value: &T,
    max_line_bytes: usize,
) -> K2CompositionResultV1<Vec<u8>> {
    let mut bytes = super::uncertainty_bytes_v1(value)?;
    if bytes.len() > max_line_bytes {
        return Err(invalid_v1("self_formed_canonical_jsonl_line_oversized"));
    }
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) fn append_canonical_jsonl_v1<T: serde::Serialize>(
    file: &mut File,
    value: &T,
    max_line_bytes: usize,
) -> K2CompositionResultV1<()> {
    file.write_all(&canonical_jsonl_line_v1(value, max_line_bytes)?)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("append_self_formed_canonical_jsonl"))
}

pub(super) fn recover_renamed_file_v1<T, F>(
    destination: &Path,
    parent: &Path,
    maximum_bytes: u64,
    validate: F,
) -> K2CompositionResultV1<T>
where
    F: Fn(File) -> K2CompositionResultV1<T>,
{
    let file = open_nofollow_file_v1(destination, true, false)?;
    let metadata = file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_renamed_file"))?;
    let mode = metadata.permissions().mode() & 0o7777;
    if !metadata.is_file() || metadata.len() > maximum_bytes || metadata.nlink() != 1 || !matches!(mode, 0o400 | 0o600)
    {
        return Err(invalid_v1("self_formed_renamed_file_invalid"));
    }
    validate(file)?;
    if mode == 0o600 {
        fs::set_permissions(destination, fs::Permissions::from_mode(0o400))
            .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_renamed_file"))?;
        sync_directory_v1(parent)?;
    }
    let file = open_nofollow_file_v1(destination, false, false)?;
    validate_regular_file_v1(&file, 0o400, maximum_bytes)?;
    validate(file)
}

pub(super) fn closed_tree_paths_v2(root: &Path) -> K2CompositionResultV1<BTreeSet<String>> {
    let mut pending = vec![PathBuf::new()];
    let mut files = BTreeSet::new();
    while let Some(relative) = pending.pop() {
        let directory = root.join(&relative);
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_packet_directory"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() || metadata.permissions().mode() & 0o7777 != 0o500 {
            return Err(invalid_v1("self_formed_r8b_packet_directory_invalid"));
        }
        for entry in
            fs::read_dir(&directory).map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_packet_directory"))?
        {
            let entry = entry.map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_packet_entry"))?;
            let child = relative.join(entry.file_name());
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_packet_entry"))?;
            if metadata.is_dir() {
                pending.push(child);
            } else if metadata.is_file() && !metadata.file_type().is_symlink() {
                files.insert(child.to_string_lossy().into_owned());
            } else {
                return Err(invalid_v1("self_formed_r8b_packet_non_file_entry"));
            }
        }
    }
    Ok(files)
}

pub fn create_private_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    if path.exists() {
        return require_private_directory_v1(path);
    }
    let parent = path.parent().ok_or_else(|| invalid_v1("self_formed_immutable_directory_parent_missing"))?;
    require_private_directory_v1(parent)?;
    fs::create_dir(path).map_err(|_| K2CompositionErrorV1::Io("create_self_formed_immutable_directory"))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_immutable_directory"))?;
    sync_directory_v1(parent)?;
    require_private_directory_v1(path)
}

pub fn publish_immutable_file_v1(
    root: &Path,
    relative_path: &str,
    bytes: &[u8],
    unix_mode: u32,
    publication_id: u64,
    fault: K2UncertaintyImmutablePublicationFaultV1,
) -> K2CompositionResultV1<K2UncertaintyImmutableFileV1> {
    require_publishable_bytes_v1(bytes, unix_mode)?;
    let final_path = checked_final_path_v1(root, relative_path)?;
    let parent = final_path.parent().ok_or_else(|| invalid_v1("self_formed_immutable_parent_missing"))?;
    require_private_directory_v1(parent)?;
    if fs::symlink_metadata(&final_path).is_ok() {
        return Err(invalid_v1("self_formed_immutable_final_exists"));
    }
    let temporary = publication_temp_path_v1(&final_path, publication_id)?;
    if fs::symlink_metadata(&temporary).is_ok() {
        return Err(invalid_v1("self_formed_immutable_temp_exists"));
    }

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(unix_mode)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_immutable_temp"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_immutable_temp"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(unix_mode))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_immutable_temp"))?;

    if fault == K2UncertaintyImmutablePublicationFaultV1::BeforePublish(publication_id) {
        fs::remove_file(&temporary).map_err(|_| K2CompositionErrorV1::Io("remove_self_formed_immutable_temp"))?;
        sync_directory_v1(parent)?;
        return Err(K2CompositionErrorV1::Io("self_formed_immutable_fault_before_publish"));
    }

    fs::hard_link(&temporary, &final_path).map_err(|_| K2CompositionErrorV1::Io("link_self_formed_immutable_final"))?;
    sync_directory_v1(parent)?;
    if fault == K2UncertaintyImmutablePublicationFaultV1::AfterPublish(publication_id) {
        return Err(K2CompositionErrorV1::Io("self_formed_immutable_fault_after_publish"));
    }
    recover_linked_publication_temp_v1(root, relative_path, bytes, unix_mode, publication_id)?;
    read_immutable_file_v1(root, relative_path, unix_mode, bytes.len())
}

pub fn recover_linked_publication_temp_v1(
    root: &Path,
    relative_path: &str,
    expected_bytes: &[u8],
    unix_mode: u32,
    publication_id: u64,
) -> K2CompositionResultV1<()> {
    require_publishable_bytes_v1(expected_bytes, unix_mode)?;
    recover_linked_temp_v1(root, relative_path, unix_mode, expected_bytes.len(), publication_id, Some(expected_bytes))?;
    Ok(())
}

pub fn recover_linked_publication_temp_from_final_v1(
    root: &Path,
    relative_path: &str,
    unix_mode: u32,
    maximum_bytes: usize,
    publication_id: u64,
) -> K2CompositionResultV1<Vec<u8>> {
    recover_linked_temp_v1(root, relative_path, unix_mode, maximum_bytes, publication_id, None)?.map_or_else(
        || read_immutable_file_v1(root, relative_path, unix_mode, maximum_bytes).map(|file| file.bytes),
        Ok,
    )
}

fn recover_linked_temp_v1(
    root: &Path,
    relative_path: &str,
    unix_mode: u32,
    maximum_bytes: usize,
    publication_id: u64,
    expected_bytes: Option<&[u8]>,
) -> K2CompositionResultV1<Option<Vec<u8>>> {
    let final_path = checked_final_path_v1(root, relative_path)?;
    let temporary = publication_temp_path_v1(&final_path, publication_id)?;
    if fs::symlink_metadata(&temporary).is_err() {
        return Ok(None);
    }
    let final_file = open_nofollow_read_v1(&final_path)?;
    let temporary_file = open_nofollow_read_v1(&temporary)?;
    let final_metadata =
        final_file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_immutable_final"))?;
    let temporary_metadata =
        temporary_file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_immutable_temp"))?;
    if !final_metadata.is_file()
        || !temporary_metadata.is_file()
        || final_metadata.dev() != temporary_metadata.dev()
        || final_metadata.ino() != temporary_metadata.ino()
        || final_metadata.nlink() != 2
        || temporary_metadata.nlink() != 2
        || (expected_bytes.is_none() && final_metadata.permissions().mode() & 0o777 != unix_mode)
    {
        return Err(invalid_v1("self_formed_immutable_temp_identity_invalid"));
    }
    let bytes = read_open_file_v1(final_file, maximum_bytes)?;
    if expected_bytes.is_some_and(|expected| {
        bytes != expected
            || final_metadata.permissions().mode() & 0o777 != unix_mode
            || final_metadata.len() != expected.len() as u64
    }) {
        return Err(invalid_v1("self_formed_immutable_temp_bytes_invalid"));
    }
    fs::remove_file(&temporary).map_err(|_| K2CompositionErrorV1::Io("remove_self_formed_immutable_linked_temp"))?;
    sync_directory_v1(final_path.parent().ok_or_else(|| invalid_v1("self_formed_immutable_parent_missing"))?)?;
    if inspect_immutable_file_v1(root, relative_path, unix_mode, maximum_bytes)?.link_count != 1 {
        return Err(invalid_v1("self_formed_immutable_link_count_invalid"));
    }
    Ok(Some(bytes))
}

pub fn inspect_immutable_file_v1(
    root: &Path,
    relative_path: &str,
    unix_mode: u32,
    maximum_bytes: usize,
) -> K2CompositionResultV1<K2UncertaintyImmutableCustodyV1> {
    let path = checked_final_path_v1(root, relative_path)?;
    let file = open_nofollow_read_v1(&path)?;
    let metadata = file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_immutable_file"))?;
    if !metadata.is_file()
        || metadata.permissions().mode() & 0o777 != unix_mode
        || metadata.len() == 0
        || metadata.len() > maximum_bytes as u64
    {
        return Err(invalid_v1("self_formed_immutable_file_custody_invalid"));
    }
    Ok(K2UncertaintyImmutableCustodyV1 {
        unix_mode,
        byte_len: metadata.len(),
        device: metadata.dev(),
        inode: metadata.ino(),
        link_count: metadata.nlink(),
    })
}

pub fn read_immutable_file_v1(
    root: &Path,
    relative_path: &str,
    unix_mode: u32,
    maximum_bytes: usize,
) -> K2CompositionResultV1<K2UncertaintyImmutableFileV1> {
    let custody = inspect_immutable_file_v1(root, relative_path, unix_mode, maximum_bytes)?;
    if custody.link_count != 1 {
        return Err(invalid_v1("self_formed_immutable_link_count_invalid"));
    }
    let file = open_nofollow_read_v1(&checked_final_path_v1(root, relative_path)?)?;
    let bytes = read_open_file_v1(file, maximum_bytes)?;
    Ok(K2UncertaintyImmutableFileV1 {
        byte_len: bytes.len() as u64,
        content_sha256: composition_sha256_bytes_v1(&bytes),
        bytes,
        unix_mode,
        device: custody.device,
        inode: custody.inode,
    })
}

pub fn immutable_publication_temp_relative_path_v1(
    relative_path: &str,
    publication_id: u64,
) -> K2CompositionResultV1<String> {
    if !valid_composition_path_v1(relative_path) {
        return Err(invalid_v1("self_formed_immutable_relative_path_invalid"));
    }
    let path = Path::new(relative_path);
    let parent = path.parent().filter(|value| !value.as_os_str().is_empty());
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_v1("self_formed_immutable_name_invalid"))?;
    let temporary_name = format!(".{name}.{publication_id}.tmp");
    Ok(parent.map_or(temporary_name.clone(), |value| value.join(temporary_name).to_string_lossy().into_owned()))
}

fn require_publishable_bytes_v1(bytes: &[u8], unix_mode: u32) -> K2CompositionResultV1<()> {
    if bytes.is_empty() || bytes.len() > K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1 || !matches!(unix_mode, 0o400 | 0o600) {
        return Err(invalid_v1("self_formed_immutable_payload_invalid"));
    }
    Ok(())
}

fn checked_final_path_v1(root: &Path, relative_path: &str) -> K2CompositionResultV1<PathBuf> {
    require_private_directory_v1(root)?;
    if !valid_composition_path_v1(relative_path)
        || Path::new(relative_path).components().any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(invalid_v1("self_formed_immutable_relative_path_invalid"));
    }
    let path = root.join(relative_path);
    let parent = path.parent().ok_or_else(|| invalid_v1("self_formed_immutable_parent_missing"))?;
    require_directory_chain_v1(root, parent)?;
    Ok(path)
}

fn require_directory_chain_v1(root: &Path, parent: &Path) -> K2CompositionResultV1<()> {
    let relative = parent.strip_prefix(root).map_err(|_| invalid_v1("self_formed_immutable_parent_escape"))?;
    let mut current = root.to_path_buf();
    require_private_directory_v1(&current)?;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(invalid_v1("self_formed_immutable_parent_escape"));
        };
        current.push(name);
        require_private_directory_v1(&current)?;
    }
    Ok(())
}

fn publication_temp_path_v1(final_path: &Path, publication_id: u64) -> K2CompositionResultV1<PathBuf> {
    let name = final_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_v1("self_formed_immutable_name_invalid"))?;
    Ok(final_path.with_file_name(format!(".{name}.{publication_id}.tmp")))
}

pub(super) fn read_closed_file_v2(
    path: &Path,
    maximum_bytes: usize,
    expected_byte_len: Option<u64>,
    expected_content_sha256: Option<&str>,
) -> K2CompositionResultV1<Vec<u8>> {
    let file = open_closed_file_v2(path, maximum_bytes as u64, expected_byte_len)?;
    let bytes = read_open_file_v1(file, maximum_bytes)?;
    if expected_content_sha256.is_some_and(|expected| composition_sha256_bytes_v1(&bytes) != expected) {
        return Err(invalid_v1("self_formed_closed_file_attestation_invalid"));
    }
    Ok(bytes)
}

pub(super) fn read_closed_json_v2<T>(
    path: &Path,
    maximum_bytes: usize,
    expected_byte_len: Option<u64>,
    expected_content_sha256: Option<&str>,
) -> K2CompositionResultV1<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let bytes = read_closed_file_v2(path, maximum_bytes, expected_byte_len, expected_content_sha256)?;
    decode_canonical_json_v1(&bytes)
}

pub(super) fn open_closed_file_v2(
    path: &Path,
    maximum_bytes: u64,
    expected_byte_len: Option<u64>,
) -> K2CompositionResultV1<File> {
    let file = open_nofollow_file_v1(path, false, false)?;
    let metadata = file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_closed_file"))?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o7777 != 0o400
        || metadata.len() == 0
        || metadata.len() > maximum_bytes
        || expected_byte_len.is_some_and(|expected| metadata.len() != expected)
    {
        return Err(invalid_v1("self_formed_closed_file_invalid"));
    }
    Ok(file)
}

fn invalid_v1(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}

fn open_nofollow_read_v1(path: &Path) -> K2CompositionResultV1<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_immutable_file"))
}

fn read_open_file_v1(file: File, maximum_bytes: usize) -> K2CompositionResultV1<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take((maximum_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_immutable_file"))?;
    if bytes.is_empty() || bytes.len() > maximum_bytes {
        return Err(invalid_v1("self_formed_immutable_file_size_invalid"));
    }
    Ok(bytes)
}

pub(super) fn open_nofollow_file_v1(path: &Path, write: bool, append: bool) -> K2CompositionResultV1<File> {
    OpenOptions::new()
        .read(true)
        .write(write)
        .append(append)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_nofollow_file"))
}

pub(super) fn create_exclusive_file_v1(path: &Path, bytes: &[u8], mode: u32) -> K2CompositionResultV1<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_exclusive_file"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("initialize_self_formed_exclusive_file"))
}

pub(super) fn validate_regular_file_v1(file: &File, mode: u32, max_bytes: u64) -> K2CompositionResultV1<()> {
    let metadata = file.metadata().map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_regular_file"))?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o7777 != mode
        || metadata.len() > max_bytes
    {
        return Err(invalid_v1("self_formed_regular_file_invalid"));
    }
    Ok(())
}

pub(super) fn rename_noreplace_same_device_v1(source: &Path, destination: &Path) -> K2CompositionResultV1<()> {
    let destination_parent =
        destination.parent().ok_or_else(|| invalid_v1("self_formed_rename_destination_invalid"))?;
    if fs::metadata(source).map_err(|_| invalid_v1("self_formed_rename_source_stat"))?.dev()
        != fs::metadata(destination_parent).map_err(|_| invalid_v1("self_formed_rename_destination_stat"))?.dev()
    {
        return Err(invalid_v1("self_formed_cross_device_rename"));
    }
    rustix::fs::renameat_with(rustix::fs::CWD, source, rustix::fs::CWD, destination, rustix::fs::RenameFlags::NOREPLACE)
        .map_err(|_| K2CompositionErrorV1::Io("rename_noreplace_self_formed_file"))
}

pub(super) fn sync_directory_v1(path: &Path) -> K2CompositionResultV1<()> {
    require_private_directory_v1(path)?;
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_immutable_directory"))
}
