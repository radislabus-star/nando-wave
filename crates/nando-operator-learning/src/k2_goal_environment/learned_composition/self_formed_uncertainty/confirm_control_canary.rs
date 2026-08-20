use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1};

const REPRESENTATION_COUNT_V1: usize = 6;
const COMPLETE_HIT_MASK_V1: u8 = (1 << REPRESENTATION_COUNT_V1) - 1;
const MAX_NEEDLE_BYTES_V1: usize = 64;
const STREAM_BYTES_V1: usize = 64 * 1024;

const CANARY_REPRESENTATIONS_V1: [&[u8]; REPRESENTATION_COUNT_V1] = [
    b"z~z~z~z~z~z~z~z~z~z~z~z~z~z~z~z~",
    b"7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e7a7e",
    b"7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E7A7E",
    b"en56fnp+en56fnp+en56fnp+en56fnp+en56fnp+en4",
    b"en56fnp+en56fnp+en56fnp+en56fnp+en56fnp+en4=",
    b"856a844c677c7623f8004621d1dcd5b584f03de2909f4686eb57594227851502",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum K2UncertaintyCanarySurfaceV1 {
    Argv = 0,
    Environment = 1,
    PathComponent = 2,
    PersistedRequest = 3,
    PublicFile = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct K2UncertaintyCanaryCensusV1 {
    hit_masks: [u8; 5],
    scanned_items: [u64; 5],
    public_bytes: u64,
}

impl K2UncertaintyCanaryCensusV1 {
    pub(super) fn require_complete_hits(
        &self,
        surfaces: &[K2UncertaintyCanarySurfaceV1],
    ) -> K2CompositionResultV1<()> {
        if surfaces.iter().any(|surface| {
            self.hit_masks[*surface as usize] != COMPLETE_HIT_MASK_V1
                || self.scanned_items[*surface as usize] == 0
        }) {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_canary_fixture_denominator_incomplete",
            ));
        }
        Ok(())
    }

    pub(super) fn require_absent(&self, hit_error: &'static str) -> K2CompositionResultV1<()> {
        if self.hit_masks.iter().any(|mask| *mask != 0) {
            return Err(K2CompositionErrorV1::Invalid(hit_error));
        }
        Ok(())
    }
}

pub(super) fn scan_self_formed_r7k_canary_v1(
    argv: &[Vec<u8>],
    environment: &[Vec<u8>],
    normalized_paths: &[&Path],
    persisted_request_bytes: &[u8],
    public_root: &Path,
) -> K2CompositionResultV1<K2UncertaintyCanaryCensusV1> {
    let mut census = K2UncertaintyCanaryCensusV1 {
        hit_masks: [0; 5],
        scanned_items: [0; 5],
        public_bytes: 0,
    };
    scan_items_v1(argv, K2UncertaintyCanarySurfaceV1::Argv, &mut census);
    scan_items_v1(
        environment,
        K2UncertaintyCanarySurfaceV1::Environment,
        &mut census,
    );
    for path in normalized_paths {
        for component in path.components() {
            match component {
                Component::Normal(value) => scan_item_v1(
                    value.as_bytes(),
                    K2UncertaintyCanarySurfaceV1::PathComponent,
                    &mut census,
                ),
                Component::Prefix(_) | Component::RootDir => {}
                Component::CurDir | Component::ParentDir => {
                    return Err(K2CompositionErrorV1::Invalid(
                        "self_formed_r7k_canary_path_not_normalized",
                    ));
                }
            }
        }
    }
    scan_item_v1(
        persisted_request_bytes,
        K2UncertaintyCanarySurfaceV1::PersistedRequest,
        &mut census,
    );
    scan_public_tree_v1(public_root, &mut census)?;
    Ok(census)
}

fn scan_items_v1(
    items: &[Vec<u8>],
    surface: K2UncertaintyCanarySurfaceV1,
    census: &mut K2UncertaintyCanaryCensusV1,
) {
    for item in items {
        scan_item_v1(item, surface, census);
    }
}

fn scan_item_v1(
    bytes: &[u8],
    surface: K2UncertaintyCanarySurfaceV1,
    census: &mut K2UncertaintyCanaryCensusV1,
) {
    census.scanned_items[surface as usize] += 1;
    for (index, representation) in CANARY_REPRESENTATIONS_V1.iter().enumerate() {
        if contains_bytes_v1(bytes, representation) {
            census.hit_masks[surface as usize] |= 1 << index;
        }
    }
}

fn scan_public_tree_v1(
    root: &Path,
    census: &mut K2UncertaintyCanaryCensusV1,
) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r7k_public_canary_root"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_r7k_public_canary_root_invalid",
        ));
    }
    let mut entries = fs::read_dir(root)
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_public_canary_root"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| K2CompositionErrorV1::Io("collect_self_formed_r7k_public_canary_root"))?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_r7k_public_canary_entry"))?;
        if metadata.file_type().is_symlink() {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_public_canary_symlink_forbidden",
            ));
        }
        if metadata.is_dir() {
            scan_public_tree_v1(&path, census)?;
        } else if metadata.is_file() {
            scan_public_file_v1(&path, metadata.len(), census)?;
        } else {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_public_canary_special_file_forbidden",
            ));
        }
    }
    Ok(())
}

fn scan_public_file_v1(
    path: &Path,
    size: u64,
    census: &mut K2UncertaintyCanaryCensusV1,
) -> K2CompositionResultV1<()> {
    census.scanned_items[K2UncertaintyCanarySurfaceV1::PublicFile as usize] += 1;
    census.public_bytes =
        census
            .public_bytes
            .checked_add(size)
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_r7k_public_canary_bytes_overflow",
            ))?;
    let mut file = fs::File::open(path)
        .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_r7k_public_canary_file"))?;
    let mut tail = Vec::new();
    let mut buffer = vec![0_u8; STREAM_BYTES_V1];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_r7k_public_canary_file"))?;
        if read == 0 {
            break;
        }
        let mut window = tail;
        window.extend_from_slice(&buffer[..read]);
        for (index, representation) in CANARY_REPRESENTATIONS_V1.iter().enumerate() {
            if contains_bytes_v1(&window, representation) {
                census.hit_masks[K2UncertaintyCanarySurfaceV1::PublicFile as usize] |= 1 << index;
            }
        }
        let retain = window.len().min(MAX_NEEDLE_BYTES_V1 - 1);
        tail = window[window.len() - retain..].to_vec();
    }
    Ok(())
}

fn contains_bytes_v1(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
