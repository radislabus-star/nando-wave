use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1};
use super::{
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2,
    K2UncertaintyR8BEvidenceKindV2, K2UncertaintyR8BExecutableManifestV2,
    K2UncertaintyR8BManifestClassV2, K2UncertaintyR8BPacketEntryV2,
    K2UncertaintyR8BPacketManifestV2, K2UncertaintyR8BProducedReceiptV2, uncertainty_bytes_v1,
    uncertainty_decode_v1,
};

pub(super) fn load_identity_manifest_v2(
    root: &Path,
    packet: &K2UncertaintyR8BPacketManifestV2,
    kind: K2UncertaintyR8BEvidenceKindV2,
    class: K2UncertaintyR8BManifestClassV2,
) -> K2CompositionResultV1<K2UncertaintyR8BExecutableManifestV2> {
    let entry = packet
        .entries
        .iter()
        .find(|entry| entry.kind == kind)
        .ok_or_else(|| invalid("self_formed_r8b_identity_manifest_missing"))?;
    let value: K2UncertaintyR8BExecutableManifestV2 = uncertainty_decode_v1(&read_closed_file_v2(
        &root.join(&entry.relative_path),
        Some(entry),
    )?)?;
    value.validate()?;
    if value.class != class
        || value.identities.len() as u64 != entry.observed
        || value.manifest_root_sha256 != entry.semantic_root_sha256
    {
        return Err(invalid("self_formed_r8b_identity_manifest_entry_invalid"));
    }
    Ok(value)
}

pub(super) fn descriptor_matches_entry_v2(
    descriptor: &K2UncertaintyR8BProducedReceiptV2,
    entry: &K2UncertaintyR8BPacketEntryV2,
) -> bool {
    descriptor.byte_len == entry.byte_len
        && descriptor.content_sha256 == entry.content_sha256
        && descriptor.receipt_schema == entry.receipt_schema
        && descriptor.semantic_root_sha256 == entry.semantic_root_sha256
        && ((descriptor.relative_path == K2_UNCERTAINTY_R8B_STDOUT_RECEIPT_PATH_V2
            && descriptor.unix_mode == 0)
            || (descriptor.relative_path == entry.relative_path
                && descriptor.unix_mode == entry.unix_mode))
}

pub(super) fn read_closed_file_v2(
    path: &Path,
    entry: Option<&K2UncertaintyR8BPacketEntryV2>,
) -> K2CompositionResultV1<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_packet_file"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o7777 != 0o400
        || metadata.len() == 0
        || metadata.len() as usize > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
    {
        return Err(invalid("self_formed_r8b_packet_file_invalid"));
    }
    let bytes =
        fs::read(path).map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_packet_file"))?;
    if let Some(entry) = entry
        && (metadata.len() != entry.byte_len
            || composition_sha256_bytes_v1(&bytes) != entry.content_sha256)
    {
        return Err(invalid("self_formed_r8b_packet_file_content_invalid"));
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("self_formed_r8b_packet_json_invalid"))?;
    if uncertainty_bytes_v1(&value)? != bytes {
        return Err(invalid("self_formed_r8b_packet_json_not_canonical"));
    }
    if let Some(entry) = entry {
        let schema = value.get("schema").and_then(serde_json::Value::as_str);
        let root = value
            .get(entry.kind.expected_root_field())
            .and_then(serde_json::Value::as_str);
        if schema != Some(entry.receipt_schema.as_str())
            || root != Some(entry.semantic_root_sha256.as_str())
        {
            return Err(invalid("self_formed_r8b_packet_typed_root_invalid"));
        }
    }
    Ok(bytes)
}

pub(super) fn closed_tree_paths_v2(root: &Path) -> K2CompositionResultV1<BTreeSet<String>> {
    let mut pending = vec![PathBuf::new()];
    let mut files = BTreeSet::new();
    while let Some(relative) = pending.pop() {
        let directory = root.join(&relative);
        let metadata = fs::symlink_metadata(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_packet_directory"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.permissions().mode() & 0o222 != 0
        {
            return Err(invalid("self_formed_r8b_packet_directory_invalid"));
        }
        for entry in fs::read_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_packet_directory"))?
        {
            let entry =
                entry.map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r8b_packet_entry"))?;
            let child = relative.join(entry.file_name());
            let child_metadata = fs::symlink_metadata(entry.path())
                .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r8b_packet_entry"))?;
            if child_metadata.is_dir() {
                pending.push(child);
            } else if child_metadata.is_file() && !child_metadata.file_type().is_symlink() {
                files.insert(child.to_string_lossy().into_owned());
            } else {
                return Err(invalid("self_formed_r8b_packet_non_file_entry"));
            }
        }
    }
    Ok(files)
}

fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
