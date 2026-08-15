use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use super::super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_file_v1,
};
use super::super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, uncertainty_bytes_v1, uncertainty_decode_v1,
};
use super::{
    DEVELOPMENT_FREEZE_FILE_V1, K2_UNCERTAINTY_DEVELOPMENT_FREEZE_ROOT_ENV_V1,
    K2UncertaintyDevelopmentFreezeInputV1, K2UncertaintyDevelopmentFreezeV1,
    seal_self_formed_development_freeze_v1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum K2UncertaintyDevelopmentFreezeFaultV1 {
    None,
    BeforeRename,
    AfterRename,
}

pub fn publish_self_formed_development_freeze_v1(
    root: &Path,
    receipt: &K2UncertaintyDevelopmentFreezeV1,
) -> K2CompositionResultV1<()> {
    publish_self_formed_development_freeze_with_fault_v1(
        root,
        receipt,
        K2UncertaintyDevelopmentFreezeFaultV1::None,
    )
}

pub(super) fn publish_self_formed_development_freeze_with_fault_v1(
    root: &Path,
    receipt: &K2UncertaintyDevelopmentFreezeV1,
    fault: K2UncertaintyDevelopmentFreezeFaultV1,
) -> K2CompositionResultV1<()> {
    receipt.validate()?;
    let bytes = uncertainty_bytes_v1(receipt)?;
    fs::create_dir_all(root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_development_freeze_root"))?;
    let path = root.join(DEVELOPMENT_FREEZE_FILE_V1);
    let temporary = root.join(format!(".{DEVELOPMENT_FREEZE_FILE_V1}.tmp"));
    if path.exists() {
        let existing = fs::read(&path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_development_freeze"))?;
        if existing != bytes {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_freeze_collision",
            ));
        }
    } else {
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|_| {
                K2CompositionErrorV1::Io("remove_stale_self_formed_development_freeze_temp")
            })?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_development_freeze_temp"))?;
        if file
            .write_all(&bytes)
            .and_then(|()| file.sync_all())
            .is_err()
        {
            let _ = fs::remove_file(&temporary);
            return Err(K2CompositionErrorV1::Io(
                "sync_self_formed_development_freeze_temp",
            ));
        }
        if fault == K2UncertaintyDevelopmentFreezeFaultV1::BeforeRename {
            fs::remove_file(&temporary).map_err(|_| {
                K2CompositionErrorV1::Io("remove_self_formed_development_freeze_fault_temp")
            })?;
            File::open(root)
                .and_then(|handle| handle.sync_all())
                .map_err(|_| {
                    K2CompositionErrorV1::Io("sync_self_formed_development_freeze_fault_root")
                })?;
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_freeze_fault_before_rename",
            ));
        }
        fs::rename(&temporary, &path)
            .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_development_freeze"))?;
        if fault == K2UncertaintyDevelopmentFreezeFaultV1::AfterRename {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_development_freeze_fault_after_rename",
            ));
        }
    }
    File::open(root)
        .and_then(|handle| handle.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_development_freeze_root"))
}

pub fn read_self_formed_development_freeze_v1(
    root: &Path,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentFreezeV1> {
    let bytes = fs::read(root.join(DEVELOPMENT_FREEZE_FILE_V1))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_development_freeze"))?;
    let receipt: K2UncertaintyDevelopmentFreezeV1 = uncertainty_decode_v1(&bytes)?;
    receipt.validate()?;
    Ok(receipt)
}

pub fn run_self_formed_development_freeze_process_v1() -> K2CompositionResultV1<()> {
    let mut input_bytes = Vec::new();
    std::io::stdin()
        .take((K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 + 1) as u64)
        .read_to_end(&mut input_bytes)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_development_freeze_stdin"))?;
    let input: K2UncertaintyDevelopmentFreezeInputV1 = uncertainty_decode_v1(&input_bytes)?;
    let executable = std::env::current_exe()
        .map_err(|_| K2CompositionErrorV1::Io("resolve_self_formed_development_freeze_owner"))?;
    if composition_sha256_file_v1(&executable)? != input.freeze_owner_executable_sha256 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_freeze_owner_mismatch",
        ));
    }
    let root = std::env::var_os(K2_UNCERTAINTY_DEVELOPMENT_FREEZE_ROOT_ENV_V1).ok_or(
        K2CompositionErrorV1::Invalid("self_formed_development_freeze_root_missing"),
    )?;
    let receipt = seal_self_formed_development_freeze_v1(&input)?;
    publish_self_formed_development_freeze_v1(Path::new(&root), &receipt)?;
    std::io::stdout()
        .write_all(&uncertainty_bytes_v1(&receipt)?)
        .map_err(|_| K2CompositionErrorV1::Io("write_self_formed_development_freeze_stdout"))
}
