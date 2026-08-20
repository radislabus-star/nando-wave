use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_CLEANUP_MANIFEST_SCHEMA_V1, K2_UNCERTAINTY_CLEANUP_MAX_ENTRIES_V1,
    K2_UNCERTAINTY_CLEANUP_PAGE_ENTRIES_V1, K2_UNCERTAINTY_CLEANUP_PAGE_SCHEMA_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2UncertaintyCleanupClassifiedPathV1,
    K2UncertaintyCleanupFileKindV1, K2UncertaintyCleanupRegistryEntryV1, cleanup_policy_root_v1,
    cleanup_registry_root_v1, denied_authority_v1, require_denied_authority_v1,
    uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
    validate_cleanup_relative_path_v1,
};

const CLEANUP_MANIFEST_FILE_V1: &str = "cleanup-manifest.json";
const CLEANUP_PAGE_DIRECTORY_V1: &str = "cleanup-manifest-pages";
pub(crate) const CLEANUP_AFTER_CENSUS_PAGE_DIRECTORY_V1: &str = "cleanup-after-census-pages";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K2UncertaintyCleanupVerifierFaultV1 {
    None,
    AfterCensusPage { page: usize },
    BeforeReceipt,
    AfterReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum K2UncertaintyCleanupManifestFaultV1 {
    None,
    AfterPage { page: usize },
    BeforeDescriptor,
    AfterDescriptor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupAfterCensusEntryV1 {
    pub relative_path: String,
    pub file_kind: K2UncertaintyCleanupFileKindV1,
    pub content_sha256: Option<String>,
    pub mode: u32,
    pub size_bytes: u64,
    pub entry_root_sha256: String,
}

impl K2UncertaintyCleanupAfterCensusEntryV1 {
    pub fn seal(
        relative_path: String,
        file_kind: K2UncertaintyCleanupFileKindV1,
        content_sha256: Option<String>,
        mode: u32,
        size_bytes: u64,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            relative_path,
            file_kind,
            content_sha256,
            mode,
            size_bytes,
            entry_root_sha256: String::new(),
        };
        value.entry_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        validate_cleanup_relative_path_v1(&self.relative_path)?;
        if let Some(root) = &self.content_sha256 {
            require_composition_root_v1(root)?;
        }
        if matches!(self.file_kind, K2UncertaintyCleanupFileKindV1::Regular)
            != self.content_sha256.is_some()
            || (self.file_kind == K2UncertaintyCleanupFileKindV1::Directory && self.size_bytes != 0)
            || self.entry_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_after_census_entry_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            "nando.k2-self-formed-cleanup-after-census-entry.v1",
            &self.relative_path,
            self.file_kind,
            &self.content_sha256,
            self.mode,
            self.size_bytes,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupAfterCensusPageV1 {
    pub sequence: u64,
    pub entries: Vec<K2UncertaintyCleanupAfterCensusEntryV1>,
    pub page_root_sha256: String,
}

impl K2UncertaintyCleanupAfterCensusPageV1 {
    fn seal(
        sequence: u64,
        entries: Vec<K2UncertaintyCleanupAfterCensusEntryV1>,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            sequence,
            entries,
            page_root_sha256: String::new(),
        };
        value.page_root_sha256 = uncertainty_root_v1(&(
            "nando.k2-self-formed-cleanup-after-census-page.v1",
            value.sequence,
            &value.entries,
        ))?;
        if value.entries.is_empty()
            || value.entries.len() > K2_UNCERTAINTY_CLEANUP_PAGE_ENTRIES_V1
            || uncertainty_bytes_v1(&value)?.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_after_census_page_invalid",
            ));
        }
        Ok(value)
    }
}

pub(crate) fn paginate_cleanup_after_census_v1(
    entries: Vec<K2UncertaintyCleanupAfterCensusEntryV1>,
) -> K2CompositionResultV1<Vec<K2UncertaintyCleanupAfterCensusPageV1>> {
    entries
        .chunks(K2_UNCERTAINTY_CLEANUP_PAGE_ENTRIES_V1)
        .enumerate()
        .map(|(sequence, entries)| {
            K2UncertaintyCleanupAfterCensusPageV1::seal(sequence as u64, entries.to_vec())
        })
        .collect()
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupManifestPageV1 {
    pub schema: String,
    pub sequence: u64,
    pub entries: Vec<K2UncertaintyCleanupClassifiedPathV1>,
    pub page_root_sha256: String,
}

impl K2UncertaintyCleanupManifestPageV1 {
    pub fn seal(
        sequence: u64,
        entries: Vec<K2UncertaintyCleanupClassifiedPathV1>,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_PAGE_SCHEMA_V1.to_owned(),
            sequence,
            entries,
            page_root_sha256: String::new(),
        };
        value.page_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            entry.validate()?;
            if !paths.insert(&entry.relative_path) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_page_duplicate_path",
                ));
            }
        }
        if self.schema != K2_UNCERTAINTY_CLEANUP_PAGE_SCHEMA_V1
            || self.entries.is_empty()
            || self.entries.len() > K2_UNCERTAINTY_CLEANUP_PAGE_ENTRIES_V1
            || uncertainty_bytes_v1(self)?.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
            || self.page_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_page_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_PAGE_SCHEMA_V1,
            self.sequence,
            &self.entries,
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyCleanupManifestV1 {
    pub schema: String,
    pub experiment_root_sha256: String,
    pub governed_root_sha256: String,
    pub artifact_registry_root_sha256: String,
    pub classification_policy_root_sha256: String,
    pub page_roots_sha256: Vec<String>,
    pub entry_count: u64,
    pub aggregate_regular_bytes: u64,
    pub census_executable_sha256: String,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub manifest_root_sha256: String,
}

impl K2UncertaintyCleanupManifestV1 {
    #[allow(clippy::too_many_arguments)]
    fn seal(
        experiment_root_sha256: String,
        governed_root_sha256: String,
        artifact_registry_root_sha256: String,
        classification_policy_root_sha256: String,
        pages: &[K2UncertaintyCleanupManifestPageV1],
        aggregate_regular_bytes: u64,
        census_executable_sha256: String,
    ) -> K2CompositionResultV1<Self> {
        let mut value = Self {
            schema: K2_UNCERTAINTY_CLEANUP_MANIFEST_SCHEMA_V1.to_owned(),
            experiment_root_sha256,
            governed_root_sha256,
            artifact_registry_root_sha256,
            classification_policy_root_sha256,
            page_roots_sha256: pages
                .iter()
                .map(|page| page.page_root_sha256.clone())
                .collect(),
            entry_count: pages.iter().map(|page| page.entries.len() as u64).sum(),
            aggregate_regular_bytes,
            census_executable_sha256,
            authority: denied_authority_v1(),
            manifest_root_sha256: String::new(),
        };
        value.manifest_root_sha256 = value.expected_root()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.experiment_root_sha256,
            &self.governed_root_sha256,
            &self.artifact_registry_root_sha256,
            &self.classification_policy_root_sha256,
            &self.census_executable_sha256,
        ]
        .into_iter()
        .chain(self.page_roots_sha256.iter())
        {
            require_composition_root_v1(root)?;
        }
        require_denied_authority_v1(&self.authority)?;
        let unique_pages = self.page_roots_sha256.iter().collect::<BTreeSet<_>>();
        if self.schema != K2_UNCERTAINTY_CLEANUP_MANIFEST_SCHEMA_V1
            || self.page_roots_sha256.is_empty()
            || unique_pages.len() != self.page_roots_sha256.len()
            || self.entry_count == 0
            || self.entry_count > K2_UNCERTAINTY_CLEANUP_MAX_ENTRIES_V1 as u64
            || self.classification_policy_root_sha256 != cleanup_policy_root_v1()?
            || uncertainty_bytes_v1(self)?.len() >= K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1
            || self.manifest_root_sha256 != self.expected_root()?
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_manifest_invalid",
            ));
        }
        Ok(())
    }

    fn expected_root(&self) -> K2CompositionResultV1<String> {
        uncertainty_root_v1(&(
            K2_UNCERTAINTY_CLEANUP_MANIFEST_SCHEMA_V1,
            &self.experiment_root_sha256,
            &self.governed_root_sha256,
            &self.artifact_registry_root_sha256,
            &self.classification_policy_root_sha256,
            &self.page_roots_sha256,
            self.entry_count,
            self.aggregate_regular_bytes,
            &self.census_executable_sha256,
            &self.authority,
        ))
    }
}

pub fn census_self_formed_cleanup_artifacts_v1(
    governed_root: &Path,
    experiment_root_sha256: String,
    registry: Vec<K2UncertaintyCleanupRegistryEntryV1>,
    census_executable_sha256: String,
) -> K2CompositionResultV1<(
    K2UncertaintyCleanupManifestV1,
    Vec<K2UncertaintyCleanupManifestPageV1>,
)> {
    require_composition_root_v1(&experiment_root_sha256)?;
    require_composition_root_v1(&census_executable_sha256)?;
    validate_governed_root_v1(governed_root)?;
    let mut registry_by_path = BTreeMap::new();
    for entry in &registry {
        entry.validate()?;
        if registry_by_path
            .insert(entry.relative_path.clone(), entry)
            .is_some()
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_registry_duplicate_path",
            ));
        }
    }
    if registry.is_empty() || registry.len() > K2_UNCERTAINTY_CLEANUP_MAX_ENTRIES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_registry_denominator_invalid",
        ));
    }
    let artifact_registry_root_sha256 = cleanup_registry_root_v1(&registry)?;
    let classification_policy_root_sha256 = cleanup_policy_root_v1()?;
    let observed = walk_governed_root_v1(governed_root)?;
    if observed.len() != registry.len() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_registry_surface_incomplete",
        ));
    }
    let mut classified = Vec::with_capacity(observed.len());
    let mut aggregate_regular_bytes = 0_u64;
    for (relative_path, path, metadata) in observed {
        let registry_entry =
            registry_by_path
                .get(&relative_path)
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_unclassified_path",
                ))?;
        let file_kind = if metadata.is_file() {
            K2UncertaintyCleanupFileKindV1::Regular
        } else if metadata.is_dir() {
            K2UncertaintyCleanupFileKindV1::Directory
        } else {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_special_file_rejected",
            ));
        };
        let (content_sha256, size_bytes) = if metadata.is_file() {
            aggregate_regular_bytes = aggregate_regular_bytes.checked_add(metadata.len()).ok_or(
                K2CompositionErrorV1::Invalid("self_formed_cleanup_manifest_size_overflow"),
            )?;
            (Some(composition_sha256_file_v1(&path)?), metadata.len())
        } else {
            (None, 0)
        };
        classified.push(K2UncertaintyCleanupClassifiedPathV1::seal(
            registry_entry,
            file_kind,
            content_sha256,
            metadata.mode() & 0o7777,
            size_bytes,
            artifact_registry_root_sha256.clone(),
            classification_policy_root_sha256.clone(),
        )?);
    }
    let pages = paginate_entries_v1(classified)?;
    let manifest = K2UncertaintyCleanupManifestV1::seal(
        experiment_root_sha256,
        cleanup_filesystem_root_v1(governed_root)?,
        artifact_registry_root_sha256,
        classification_policy_root_sha256,
        &pages,
        aggregate_regular_bytes,
        census_executable_sha256,
    )?;
    validate_cleanup_manifest_pages_v1(&manifest, &pages)?;
    Ok((manifest, pages))
}

pub fn publish_self_formed_cleanup_manifest_v1(
    governed_root: &Path,
    control_root: &Path,
    manifest: &K2UncertaintyCleanupManifestV1,
    pages: &[K2UncertaintyCleanupManifestPageV1],
) -> K2CompositionResultV1<()> {
    publish_self_formed_cleanup_manifest_with_fault_v1(
        governed_root,
        control_root,
        manifest,
        pages,
        K2UncertaintyCleanupManifestFaultV1::None,
    )
}

pub(crate) fn publish_self_formed_cleanup_manifest_with_fault_v1(
    governed_root: &Path,
    control_root: &Path,
    manifest: &K2UncertaintyCleanupManifestV1,
    pages: &[K2UncertaintyCleanupManifestPageV1],
    fault: K2UncertaintyCleanupManifestFaultV1,
) -> K2CompositionResultV1<()> {
    validate_sibling_roots_v1(governed_root, control_root)?;
    validate_cleanup_manifest_pages_v1(manifest, pages)?;
    ensure_control_root_v1(control_root)?;
    let page_root = control_root.join(CLEANUP_PAGE_DIRECTORY_V1);
    ensure_control_root_v1(&page_root)?;
    for (page_index, page) in pages.iter().enumerate() {
        publish_control_bytes_v1(
            &page_root.join(format!("{}.json", page.page_root_sha256)),
            &uncertainty_bytes_v1(page)?,
        )?;
        fail_manifest_at_v1(
            fault,
            K2UncertaintyCleanupManifestFaultV1::AfterPage { page: page_index },
        )?;
    }
    fail_manifest_at_v1(fault, K2UncertaintyCleanupManifestFaultV1::BeforeDescriptor)?;
    publish_control_bytes_v1(
        &control_root.join(CLEANUP_MANIFEST_FILE_V1),
        &uncertainty_bytes_v1(manifest)?,
    )?;
    fail_manifest_at_v1(fault, K2UncertaintyCleanupManifestFaultV1::AfterDescriptor)
}

fn fail_manifest_at_v1(
    actual: K2UncertaintyCleanupManifestFaultV1,
    expected: K2UncertaintyCleanupManifestFaultV1,
) -> K2CompositionResultV1<()> {
    if actual == expected {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_manifest_injected_fault",
        ));
    }
    Ok(())
}

pub fn load_self_formed_cleanup_manifest_pages_v1(
    control_root: &Path,
    manifest: &K2UncertaintyCleanupManifestV1,
) -> K2CompositionResultV1<Vec<K2UncertaintyCleanupManifestPageV1>> {
    manifest.validate()?;
    let mut pages = Vec::with_capacity(manifest.page_roots_sha256.len());
    for root in &manifest.page_roots_sha256 {
        let bytes = fs::read(
            control_root
                .join(CLEANUP_PAGE_DIRECTORY_V1)
                .join(format!("{root}.json")),
        )
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_manifest_page"))?;
        let page: K2UncertaintyCleanupManifestPageV1 = uncertainty_decode_v1(&bytes)?;
        pages.push(page);
    }
    validate_cleanup_manifest_pages_v1(manifest, &pages)?;
    Ok(pages)
}

pub(crate) fn validate_cleanup_manifest_pages_v1(
    manifest: &K2UncertaintyCleanupManifestV1,
    pages: &[K2UncertaintyCleanupManifestPageV1],
) -> K2CompositionResultV1<()> {
    manifest.validate()?;
    let mut paths = BTreeSet::new();
    let mut entry_count = 0_u64;
    let mut aggregate_bytes = 0_u64;
    let mut registry = Vec::new();
    for (sequence, page) in pages.iter().enumerate() {
        page.validate()?;
        if page.sequence != sequence as u64
            || manifest.page_roots_sha256.get(sequence) != Some(&page.page_root_sha256)
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_page_order_invalid",
            ));
        }
        for entry in &page.entries {
            if !paths.insert(&entry.relative_path)
                || entry.artifact_registry_root_sha256 != manifest.artifact_registry_root_sha256
                || entry.classification_policy_root_sha256
                    != manifest.classification_policy_root_sha256
            {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_manifest_entry_binding_invalid",
                ));
            }
            entry_count += 1;
            aggregate_bytes = aggregate_bytes.checked_add(entry.size_bytes).ok_or(
                K2CompositionErrorV1::Invalid("self_formed_cleanup_manifest_size_overflow"),
            )?;
            registry.push(K2UncertaintyCleanupRegistryEntryV1 {
                relative_path: entry.relative_path.clone(),
                artifact_kind: entry.artifact_kind,
                producer_executable_sha256: entry.producer_executable_sha256.clone(),
                producing_journal_event_root_sha256: entry
                    .producing_journal_event_root_sha256
                    .clone(),
            });
        }
    }
    if pages.len() != manifest.page_roots_sha256.len()
        || entry_count != manifest.entry_count
        || aggregate_bytes != manifest.aggregate_regular_bytes
        || cleanup_registry_root_v1(&registry)? != manifest.artifact_registry_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_manifest_pages_invalid",
        ));
    }
    Ok(())
}

pub(crate) fn cleanup_filesystem_root_v1(root: &Path) -> K2CompositionResultV1<String> {
    let canonical = fs::canonicalize(root)
        .map_err(|_| K2CompositionErrorV1::Io("canonicalize_self_formed_cleanup_root"))?;
    uncertainty_root_v1(&(
        "nando.k2-self-formed-cleanup-filesystem-root.v1",
        canonical.as_os_str().as_encoded_bytes(),
    ))
}

pub(crate) fn validate_sibling_roots_v1(
    governed_root: &Path,
    control_root: &Path,
) -> K2CompositionResultV1<()> {
    let governed = fs::canonicalize(governed_root)
        .map_err(|_| K2CompositionErrorV1::Io("canonicalize_self_formed_governed_root"))?;
    let control = fs::canonicalize(control_root)
        .map_err(|_| K2CompositionErrorV1::Io("canonicalize_self_formed_control_root"))?;
    if governed == control || governed.starts_with(&control) || control.starts_with(&governed) {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_roots_not_siblings",
        ));
    }
    Ok(())
}

pub(crate) fn publish_control_bytes_v1(path: &Path, bytes: &[u8]) -> K2CompositionResultV1<()> {
    if bytes.len() > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_cleanup_protocol_too_large",
        ));
    }
    let parent = path.parent().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_cleanup_control_parent_missing",
    ))?;
    ensure_control_root_v1(parent)?;
    if path.exists() {
        let existing = fs::read(path)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_cleanup_control_file"))?;
        if existing != bytes {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_control_collision",
            ));
        }
        return Ok(());
    }
    let temporary =
        parent.join(format!(
            ".{}.tmp",
            path.file_name().and_then(|name| name.to_str()).ok_or(
                K2CompositionErrorV1::Invalid("self_formed_cleanup_control_name_invalid"),
            )?
        ));
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|_| K2CompositionErrorV1::Io("remove_self_formed_cleanup_stale_temp"))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_cleanup_control_temp"))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_cleanup_control_temp"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o400))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_cleanup_control_temp"))?;
    fs::rename(&temporary, path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_cleanup_control_file"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_cleanup_control_parent"))
}

fn paginate_entries_v1(
    entries: Vec<K2UncertaintyCleanupClassifiedPathV1>,
) -> K2CompositionResultV1<Vec<K2UncertaintyCleanupManifestPageV1>> {
    let mut pages = Vec::new();
    let mut current = Vec::new();
    for entry in entries {
        current.push(entry);
        let candidate =
            K2UncertaintyCleanupManifestPageV1::seal(pages.len() as u64, current.clone());
        if candidate.is_err() && current.len() > 1 {
            let last = current.pop().ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_cleanup_page_partition_invalid",
            ))?;
            pages.push(K2UncertaintyCleanupManifestPageV1::seal(
                pages.len() as u64,
                std::mem::take(&mut current),
            )?);
            current.push(last);
        }
    }
    if !current.is_empty() {
        pages.push(K2UncertaintyCleanupManifestPageV1::seal(
            pages.len() as u64,
            current,
        )?);
    }
    Ok(pages)
}

fn validate_governed_root_v1(root: &Path) -> K2CompositionResultV1<()> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_governed_root"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_governed_root_invalid",
        ));
    }
    Ok(())
}

pub(crate) fn walk_governed_root_v1(
    root: &Path,
) -> K2CompositionResultV1<Vec<(String, PathBuf, fs::Metadata)>> {
    let mut pending = vec![root.to_path_buf()];
    let mut observed = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_governed_directory"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_governed_entry"))?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| K2CompositionErrorV1::Io("stat_self_formed_governed_entry"))?;
            if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_symlink_or_special_file_rejected",
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| K2CompositionErrorV1::Invalid("self_formed_cleanup_path_escape"))?
                .to_str()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_path_not_utf8",
                ))?
                .to_owned();
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            observed.push((relative, path, metadata));
            if observed.len() > K2_UNCERTAINTY_CLEANUP_MAX_ENTRIES_V1 {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_cleanup_manifest_oversized",
                ));
            }
        }
    }
    observed.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(observed)
}

fn ensure_control_root_v1(root: &Path) -> K2CompositionResultV1<()> {
    fs::create_dir_all(root)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_cleanup_control_root"))?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700))
        .map_err(|_| K2CompositionErrorV1::Io("chmod_self_formed_cleanup_control_root"))
}
