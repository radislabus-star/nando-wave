use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use nando_operator_kernel::Sha256CommitmentV3;

use super::{ProviderCaptureStoreErrorV3, ProviderCaptureStoreSlotV3};

pub(super) fn prepare_root(root: &Path) -> Result<(), ProviderCaptureStoreErrorV3> {
    if root.as_os_str().is_empty() {
        return Err(ProviderCaptureStoreErrorV3::InvalidRoot);
    }
    if let Ok(metadata) = fs::symlink_metadata(root)
        && (!metadata.is_dir() || metadata.file_type().is_symlink())
    {
        return Err(ProviderCaptureStoreErrorV3::InvalidRoot);
    }
    fs::create_dir_all(root).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    let metadata = fs::symlink_metadata(root).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ProviderCaptureStoreErrorV3::InvalidRoot);
    }
    #[cfg(unix)]
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    Ok(())
}

pub(super) fn read_slot(
    root: &Path,
    slot: ProviderCaptureStoreSlotV3,
) -> Result<Option<Vec<u8>>, ProviderCaptureStoreErrorV3> {
    let path = slot.path(root);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ProviderCaptureStoreErrorV3::Io),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ProviderCaptureStoreErrorV3::CommittedSlotCorrupt);
    }
    fs::read(path)
        .map(Some)
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)
}

pub(super) fn write_slot_atomically(
    root: &Path,
    slot: ProviderCaptureStoreSlotV3,
    bytes: &[u8],
) -> Result<(), ProviderCaptureStoreErrorV3> {
    let temporary = slot.temporary_path(root);
    if fs::symlink_metadata(&temporary).is_ok() {
        return Err(ProviderCaptureStoreErrorV3::SlotConflict);
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&temporary)
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    file.write_all(bytes)
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    file.sync_all()
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    fs::rename(&temporary, slot.path(root)).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    sync_directory(root)
}

pub(super) fn quarantine_stale_temporary(
    root: &Path,
    slot: ProviderCaptureStoreSlotV3,
) -> Result<Option<PathBuf>, ProviderCaptureStoreErrorV3> {
    let path = slot.temporary_path(root);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ProviderCaptureStoreErrorV3::Io),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ProviderCaptureStoreErrorV3::SlotConflict);
    }
    let bytes = fs::read(&path).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    let digest = Sha256CommitmentV3::digest_bytes(&bytes).to_hex();
    let quarantine = root.join("quarantine");
    fs::create_dir_all(&quarantine).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    #[cfg(unix)]
    fs::set_permissions(&quarantine, fs::Permissions::from_mode(0o700))
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    let stem = format!("{}.{}", slot.file_name(), digest);
    let mut destination = quarantine.join(format!("{stem}.stale"));
    for suffix in 1..=16 {
        if !destination.exists() {
            break;
        }
        destination = quarantine.join(format!("{stem}.{suffix}.stale"));
    }
    if destination.exists() {
        return Err(ProviderCaptureStoreErrorV3::SlotConflict);
    }
    fs::rename(path, &destination).map_err(|_| ProviderCaptureStoreErrorV3::Io)?;
    sync_directory(&quarantine)?;
    sync_directory(root)?;
    Ok(Some(destination))
}

fn sync_directory(path: &Path) -> Result<(), ProviderCaptureStoreErrorV3> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ProviderCaptureStoreErrorV3::Io)
}
