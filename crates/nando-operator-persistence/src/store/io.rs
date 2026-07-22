use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use nando_operator_kernel::sha256_bytes;

use super::{GenerationStoreErrorV3, GenerationStoreSlotV3};

pub(super) fn prepare_store_root(root: &Path) -> Result<(), GenerationStoreErrorV3> {
    if root.as_os_str().is_empty() {
        return Err(GenerationStoreErrorV3::InvalidRoot);
    }
    fs::create_dir_all(root).map_err(|_| GenerationStoreErrorV3::Io)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| GenerationStoreErrorV3::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GenerationStoreErrorV3::InvalidRoot);
    }
    Ok(())
}

pub(super) fn read_slot(
    root: &Path,
    slot: GenerationStoreSlotV3,
) -> Result<Option<Vec<u8>>, GenerationStoreErrorV3> {
    let path = slot.path(root);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(GenerationStoreErrorV3::Io),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(GenerationStoreErrorV3::InvalidCheckpoint);
    }
    fs::read(path)
        .map(Some)
        .map_err(|_| GenerationStoreErrorV3::Io)
}

pub(super) fn write_slot_atomically(
    root: &Path,
    slot: GenerationStoreSlotV3,
    bytes: &[u8],
) -> Result<(), GenerationStoreErrorV3> {
    let temporary = slot.temporary_path(root);
    if fs::symlink_metadata(&temporary).is_ok() {
        return Err(GenerationStoreErrorV3::SlotConflict);
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|_| GenerationStoreErrorV3::Io)?;
    file.write_all(bytes)
        .map_err(|_| GenerationStoreErrorV3::Io)?;
    file.sync_all().map_err(|_| GenerationStoreErrorV3::Io)?;
    fs::rename(&temporary, slot.path(root)).map_err(|_| GenerationStoreErrorV3::Io)?;
    sync_directory(root)
}

pub(super) fn quarantine_file(root: &Path, path: &Path) -> Result<PathBuf, GenerationStoreErrorV3> {
    let bytes = fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        .and_then(|_| fs::read(path).ok())
        .unwrap_or_default();
    let quarantine = root.join("quarantine");
    fs::create_dir_all(&quarantine).map_err(|_| GenerationStoreErrorV3::Io)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    let digest = sha256_bytes(&bytes);
    let mut destination = quarantine.join(format!("{name}.{digest}.bad"));
    for suffix in 1..=16 {
        if !destination.exists() {
            break;
        }
        destination = quarantine.join(format!("{name}.{digest}.{suffix}.bad"));
    }
    if destination.exists() {
        return Err(GenerationStoreErrorV3::SlotConflict);
    }
    fs::rename(path, &destination).map_err(|_| GenerationStoreErrorV3::Io)?;
    sync_directory(&quarantine)?;
    sync_directory(root)?;
    Ok(destination)
}

pub(super) fn quarantine_stale_temporary(
    root: &Path,
    slot: GenerationStoreSlotV3,
) -> Result<Option<PathBuf>, GenerationStoreErrorV3> {
    let path = slot.temporary_path(root);
    match fs::symlink_metadata(&path) {
        Ok(_) => quarantine_file(root, &path).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(GenerationStoreErrorV3::Io),
    }
}

fn sync_directory(path: &Path) -> Result<(), GenerationStoreErrorV3> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| GenerationStoreErrorV3::Io)
}
