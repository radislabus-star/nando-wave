use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{
    K2CompositionAuthorityBoundaryV1, K2CompositionErrorV1, K2CompositionResultV1,
    composition_sha256_bytes_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1, K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1,
    K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1, K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1,
    K2_UNCERTAINTY_RAW_PROBES_V1, K2UncertaintyFrontierPageV1, K2UncertaintyFrontierV1,
    K2UncertaintyProbeOutputV1, K2UncertaintyStateUniverseV1, denied_authority_v1,
    require_denied_authority_v1, uncertainty_bytes_v1, uncertainty_decode_v1, uncertainty_root_v1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum K2UncertaintyArtifactKindV1 {
    StateUniverse,
    Frontier,
    FrontierPage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyArtifactEntryV1 {
    pub schema: String,
    pub kind: K2UncertaintyArtifactKindV1,
    pub sequence: u64,
    pub relative_path: String,
    pub semantic_root_sha256: String,
    pub content_sha256: String,
    pub byte_len: u64,
    pub entry_root_sha256: String,
}

impl K2UncertaintyArtifactEntryV1 {
    fn seal(
        kind: K2UncertaintyArtifactKindV1,
        sequence: u64,
        relative_path: String,
        semantic_root_sha256: String,
        bytes: &[u8],
    ) -> K2CompositionResultV1<Self> {
        let content_sha256 = composition_sha256_bytes_v1(bytes);
        let byte_len = bytes.len() as u64;
        let entry_root_sha256 = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1,
            kind,
            sequence,
            &relative_path,
            &semantic_root_sha256,
            &content_sha256,
            byte_len,
        ))?;
        let entry = Self {
            schema: K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1.to_owned(),
            kind,
            sequence,
            relative_path,
            semantic_root_sha256,
            content_sha256,
            byte_len,
            entry_root_sha256,
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.semantic_root_sha256,
            &self.content_sha256,
            &self.entry_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let expected_path = match self.kind {
            K2UncertaintyArtifactKindV1::StateUniverse => "state-universe.json".to_owned(),
            K2UncertaintyArtifactKindV1::Frontier => "frontier.json".to_owned(),
            K2UncertaintyArtifactKindV1::FrontierPage => {
                format!("pages/page-{:04}.json", self.sequence.saturating_sub(2))
            }
        };
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1,
            self.kind,
            self.sequence,
            &self.relative_path,
            &self.semantic_root_sha256,
            &self.content_sha256,
            self.byte_len,
        ))?;
        if self.schema != K2_UNCERTAINTY_ARTIFACT_ENTRY_SCHEMA_V1
            || self.relative_path != expected_path
            || self.byte_len == 0
            || self.byte_len > K2_UNCERTAINTY_MAX_PROTOCOL_BYTES_V1 as u64
            || self.entry_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_artifact_entry_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct K2UncertaintyProbeArtifactsV1 {
    pub schema: String,
    pub case_id_sha256: String,
    pub probe_request_root_sha256: String,
    pub state_universe_root_sha256: String,
    pub frontier_root_sha256: String,
    pub entries: Vec<K2UncertaintyArtifactEntryV1>,
    pub authority: K2CompositionAuthorityBoundaryV1,
    pub artifacts_root_sha256: String,
}

impl K2UncertaintyProbeArtifactsV1 {
    pub fn validate(&self) -> K2CompositionResultV1<()> {
        for root in [
            &self.case_id_sha256,
            &self.probe_request_root_sha256,
            &self.state_universe_root_sha256,
            &self.frontier_root_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        let page_count =
            K2_UNCERTAINTY_RAW_PROBES_V1.div_ceil(K2_UNCERTAINTY_FRONTIER_PAGE_PROBES_V1);
        if self.entries.len() != page_count + 2 {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_artifact_count_invalid",
            ));
        }
        for (sequence, entry) in self.entries.iter().enumerate() {
            entry.validate()?;
            let expected_kind = match sequence {
                0 => K2UncertaintyArtifactKindV1::StateUniverse,
                1 => K2UncertaintyArtifactKindV1::Frontier,
                _ => K2UncertaintyArtifactKindV1::FrontierPage,
            };
            if entry.sequence != sequence as u64 || entry.kind != expected_kind {
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_probe_artifact_sequence_invalid",
                ));
            }
        }
        require_denied_authority_v1(&self.authority)?;
        let expected = uncertainty_root_v1(&(
            K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1,
            &self.case_id_sha256,
            &self.probe_request_root_sha256,
            &self.state_universe_root_sha256,
            &self.frontier_root_sha256,
            &self.entries,
            &self.authority,
        ))?;
        if self.schema != K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1
            || self.entries[0].semantic_root_sha256 != self.state_universe_root_sha256
            || self.entries[1].semantic_root_sha256 != self.frontier_root_sha256
            || self.artifacts_root_sha256 != expected
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_probe_artifacts_invalid",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum K2UncertaintyArtifactFaultV1 {
    None,
    BeforeRename(u64),
    AfterRename(u64),
}

pub fn publish_self_formed_probe_output_v1(
    root: &Path,
    output: &K2UncertaintyProbeOutputV1,
) -> K2CompositionResultV1<K2UncertaintyProbeArtifactsV1> {
    publish_self_formed_probe_output_with_fault_v1(root, output, K2UncertaintyArtifactFaultV1::None)
}

pub fn publish_self_formed_probe_output_with_fault_v1(
    root: &Path,
    output: &K2UncertaintyProbeOutputV1,
    fault: K2UncertaintyArtifactFaultV1,
) -> K2CompositionResultV1<K2UncertaintyProbeArtifactsV1> {
    output.validate()?;
    fs::create_dir_all(root)
        .and_then(|_| fs::create_dir_all(root.join("pages")))
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_artifact_root"))?;
    let mut entries = Vec::with_capacity(output.pages.len() + 2);
    publish_value_v1(
        root,
        K2UncertaintyArtifactKindV1::StateUniverse,
        0,
        "state-universe.json",
        &output.state_universe.universe_root_sha256,
        &output.state_universe,
        fault,
        &mut entries,
    )?;
    publish_value_v1(
        root,
        K2UncertaintyArtifactKindV1::Frontier,
        1,
        "frontier.json",
        &output.frontier.frontier_root_sha256,
        &output.frontier,
        fault,
        &mut entries,
    )?;
    for (index, page) in output.pages.iter().enumerate() {
        let sequence = index as u64 + 2;
        publish_value_v1(
            root,
            K2UncertaintyArtifactKindV1::FrontierPage,
            sequence,
            &format!("pages/page-{index:04}.json"),
            &page.page_root_sha256,
            page,
            fault,
            &mut entries,
        )?;
    }
    let authority = denied_authority_v1();
    let artifacts_root_sha256 = uncertainty_root_v1(&(
        K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1,
        &output.frontier.case_id_sha256,
        &output.probe_request_root_sha256,
        &output.state_universe.universe_root_sha256,
        &output.frontier.frontier_root_sha256,
        &entries,
        &authority,
    ))?;
    let receipt = K2UncertaintyProbeArtifactsV1 {
        schema: K2_UNCERTAINTY_PROBE_ARTIFACTS_SCHEMA_V1.to_owned(),
        case_id_sha256: output.frontier.case_id_sha256.clone(),
        probe_request_root_sha256: output.probe_request_root_sha256.clone(),
        state_universe_root_sha256: output.state_universe.universe_root_sha256.clone(),
        frontier_root_sha256: output.frontier.frontier_root_sha256.clone(),
        entries,
        authority,
        artifacts_root_sha256,
    };
    receipt.validate()?;
    atomic_write_v1(
        root,
        "receipt.json",
        &uncertainty_bytes_v1(&receipt)?,
        u64::MAX,
        K2UncertaintyArtifactFaultV1::None,
    )?;
    Ok(receipt)
}

pub fn reopen_self_formed_probe_output_v1(
    root: &Path,
    receipt: &K2UncertaintyProbeArtifactsV1,
) -> K2CompositionResultV1<K2UncertaintyProbeOutputV1> {
    receipt.validate()?;
    for entry in &receipt.entries {
        let bytes = fs::read(root.join(&entry.relative_path))
            .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_artifact"))?;
        if bytes.len() as u64 != entry.byte_len
            || composition_sha256_bytes_v1(&bytes) != entry.content_sha256
        {
            return Err(K2CompositionErrorV1::Invalid(
                "self_formed_artifact_content_mismatch",
            ));
        }
    }
    let state_universe: K2UncertaintyStateUniverseV1 = read_value_v1(root, &receipt.entries[0])?;
    let frontier: K2UncertaintyFrontierV1 = read_value_v1(root, &receipt.entries[1])?;
    let pages = receipt.entries[2..]
        .iter()
        .map(|entry| read_value_v1::<K2UncertaintyFrontierPageV1>(root, entry))
        .collect::<K2CompositionResultV1<Vec<_>>>()?;
    let mut output = K2UncertaintyProbeOutputV1 {
        schema: super::K2_UNCERTAINTY_PROBE_OUTPUT_SCHEMA_V1.to_owned(),
        probe_request_root_sha256: receipt.probe_request_root_sha256.clone(),
        state_universe,
        pages,
        frontier,
        authority: denied_authority_v1(),
        output_root_sha256: String::new(),
    };
    output.reseal()?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn publish_value_v1<T: Serialize>(
    root: &Path,
    kind: K2UncertaintyArtifactKindV1,
    sequence: u64,
    relative_path: &str,
    semantic_root_sha256: &str,
    value: &T,
    fault: K2UncertaintyArtifactFaultV1,
    entries: &mut Vec<K2UncertaintyArtifactEntryV1>,
) -> K2CompositionResultV1<()> {
    let bytes = uncertainty_bytes_v1(value)?;
    let entry = K2UncertaintyArtifactEntryV1::seal(
        kind,
        sequence,
        relative_path.to_owned(),
        semantic_root_sha256.to_owned(),
        &bytes,
    )?;
    atomic_write_v1(root, relative_path, &bytes, sequence, fault)?;
    entries.push(entry);
    Ok(())
}

fn read_value_v1<T: serde::de::DeserializeOwned + Serialize>(
    root: &Path,
    entry: &K2UncertaintyArtifactEntryV1,
) -> K2CompositionResultV1<T> {
    let bytes = fs::read(root.join(&entry.relative_path))
        .map_err(|_| K2CompositionErrorV1::Io("read_self_formed_artifact"))?;
    uncertainty_decode_v1(&bytes)
}

fn atomic_write_v1(
    root: &Path,
    relative_path: &str,
    bytes: &[u8],
    sequence: u64,
    fault: K2UncertaintyArtifactFaultV1,
) -> K2CompositionResultV1<()> {
    let path = root.join(relative_path);
    if path.exists() {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_artifact_identity_exists",
        ));
    }
    let parent = path.parent().ok_or(K2CompositionErrorV1::Invalid(
        "self_formed_artifact_parent_missing",
    ))?;
    let name =
        path.file_name()
            .and_then(|value| value.to_str())
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_artifact_name_invalid",
            ))?;
    let temporary = parent.join(format!(".{name}.tmp"));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| K2CompositionErrorV1::Io("create_self_formed_artifact_temp"))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_artifact_temp"))?;
    if fault == K2UncertaintyArtifactFaultV1::BeforeRename(sequence) {
        let _ = fs::remove_file(&temporary);
        return Err(K2CompositionErrorV1::Io(
            "self_formed_artifact_fault_before_rename",
        ));
    }
    fs::rename(&temporary, &path)
        .map_err(|_| K2CompositionErrorV1::Io("rename_self_formed_artifact"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| K2CompositionErrorV1::Io("sync_self_formed_artifact_directory"))?;
    if fault == K2UncertaintyArtifactFaultV1::AfterRename(sequence) {
        return Err(K2CompositionErrorV1::Io(
            "self_formed_artifact_fault_after_rename",
        ));
    }
    Ok(())
}
