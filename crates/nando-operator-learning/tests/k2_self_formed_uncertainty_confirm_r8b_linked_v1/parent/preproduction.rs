use std::path::Component;

use super::super::{create_private_directory_v1, tree_snapshot_v1};
use super::*;

#[rustfmt::skip]
const P00_TOOL_PATHS_V3: [&str; 8] = [
    "/usr/bin/systemd-run", "/usr/bin/systemctl", "/usr/lib/systemd/systemd", "/usr/bin/strace",
    "/usr/bin/bwrap", "/usr/bin/prlimit", "/usr/lib/cargo/bin/sudo", "/usr/lib/cargo/bin/coreutils/sha256sum",
];
#[rustfmt::skip]
const P00_LINKED_ROLES_V3: [&str; 26] = [
    "M01_DEVELOPMENT_OWNER", "M02_GENERATOR", "M03_LEARNER", "M04_PROBE",
    "M05_SELECTOR", "M06_BASELINE", "M07_SELECTION_PREVERIFIER", "M08_CLOSURE_PLANNER",
    "M09_CLOSURE_VERIFIER", "M10_PUBLIC_COORDINATOR", "M11_PRIVATE_RESOLVER", "M12_SAFETY",
    "M13_WORKER", "M14_OBSERVER", "M15_FINAL_VERIFIER", "M16_ORACLE", "M17_CONTROL_EVALUATOR",
    "M18_TERMINAL_EVALUATOR", "M19_FRESH_CONTROL_CASE", "M20_CLEANUP_AUTHORIZER",
    "M21_CLEANUP_OWNER", "M22_CLEANUP_VERIFIER", "M23_DEVELOPMENT_RESULT_PUBLISHER",
    "M24_LINKED_RUNNER", "M25_R8B_AUTHORIZER", "M26_R8B_PUBLISHER",
];
#[rustfmt::skip]
const P00_SUITE_ROLES_V3: [&str; 5] = [
    "S01_CRATE_UNIT", "S02_RESTART", "S03_MODE_MATRIX",
    "S04_CLEANUP_NEGATIVE", "S05_AUTHORITY_PUBLICATION",
];

#[rustfmt::skip]
struct P00InputPathsV3 {
    development_seed: PathBuf, fixture_tree: PathBuf,
    linked_manifest: PathBuf, suite_manifest: PathBuf,
    process_ledger: PathBuf, exclusive_output: PathBuf,
    development_seed_semantic_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct ParentToolFileIdentityV3 {
    canonical_path: String,
    unix_mode: u32,
    byte_len: u64,
    content_sha256: String,
}

impl ParentToolFileIdentityV3 {
    fn capture(path: &str) -> ParentResultV3<Self> {
        let path = Path::new(path);
        let canonical = fs::canonicalize(path).map_err(|_| "r8b_v8_p00_tool_missing")?;
        let metadata = fs::symlink_metadata(path).map_err(|_| "r8b_v8_p00_tool_stat_failed")?;
        if canonical != path || !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err("r8b_v8_p00_tool_identity_invalid");
        }
        Ok(Self {
            canonical_path: path.to_string_lossy().into_owned(),
            unix_mode: metadata.permissions().mode() & 0o7777,
            byte_len: metadata.len(),
            content_sha256: composition_sha256_file_v1(path)
                .map_err(|_| "r8b_v8_p00_tool_hash_failed")?,
        })
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        if *self != Self::capture(&self.canonical_path)? {
            return Err("r8b_v8_p00_tool_changed");
        }
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct P00SourceCapabilityRootV3 {
    schema: String,
    route_id_sha256: String,
    inputs: Vec<K2UncertaintyR8BInputBindingV3>,
    linked_manifest: K2UncertaintyR8BExecutableManifestV2,
    suite_manifest: K2UncertaintyR8BExecutableManifestV2,
    tools: Vec<ParentToolFileIdentityV3>,
}

pub(super) struct P00SourceCapabilityV3 {
    root: P00SourceCapabilityRootV3,
    source_inventory_root_sha256: String,
    input_bindings_root_sha256: String,
    executable_manifest_root_sha256: String,
    tool_manifest_root_sha256: String,
}

impl P00SourceCapabilityV3 {
    #[rustfmt::skip]
    fn bind(
        route_id_sha256: String, paths: P00InputPathsV3,
        linked_manifest: K2UncertaintyR8BExecutableManifestV2,
        suite_manifest: K2UncertaintyR8BExecutableManifestV2,
    ) -> ParentResultV3<Self> {
        require_composition_root_v1(&route_id_sha256).map_err(|_| "r8b_v8_p00_route_invalid")?;
        validate_manifest_shape_v3(&linked_manifest, K2UncertaintyR8BManifestClassV2::Linked)?;
        validate_manifest_shape_v3(&suite_manifest, K2UncertaintyR8BManifestClassV2::Suite)?;
        validate_manifest_file_v3(&paths.linked_manifest, &linked_manifest)?;
        validate_manifest_file_v3(&paths.suite_manifest, &suite_manifest)?;
        let specifications = [
            (K2UncertaintyR8BInputRoleV3::DevelopmentSeed, paths.development_seed, 0o400,
                paths.development_seed_semantic_root_sha256),
            (K2UncertaintyR8BInputRoleV3::FixtureTree, paths.fixture_tree, 0o500,
                composition_root_v1(&"r8b-v8-fixture-tree").map_err(|_| "r8b_v8_p00_input_root_failed")?),
            (K2UncertaintyR8BInputRoleV3::LinkedManifest, paths.linked_manifest, 0o400,
                linked_manifest.manifest_root_sha256.clone()),
            (K2UncertaintyR8BInputRoleV3::SuiteManifest, paths.suite_manifest, 0o400,
                suite_manifest.manifest_root_sha256.clone()),
            (K2UncertaintyR8BInputRoleV3::ProcessLedger, paths.process_ledger, 0o600,
                composition_root_v1(&"r8b-v8-open-ledger").map_err(|_| "r8b_v8_p00_input_root_failed")?),
            (K2UncertaintyR8BInputRoleV3::ExclusiveOutput, paths.exclusive_output, 0o700,
                composition_root_v1(&"r8b-v8-exclusive-output").map_err(|_| "r8b_v8_p00_input_root_failed")?),
        ];
        let inputs = specifications.into_iter()
            .map(|(role, path, mode, semantic)| capture_input_v3(role, &path, mode, semantic))
            .collect::<ParentResultV3<Vec<_>>>()?;
        let tools = P00_TOOL_PATHS_V3.into_iter().map(ParentToolFileIdentityV3::capture)
            .collect::<ParentResultV3<Vec<_>>>()?;
        let root = P00SourceCapabilityRootV3 {
            schema: "nando.k2-self-formed-r8b-p00-source-capability.v3".to_owned(),
            route_id_sha256, inputs, linked_manifest, suite_manifest, tools,
        };
        Ok(Self {
            input_bindings_root_sha256: composition_root_v1(&root.inputs)
                .map_err(|_| "r8b_v8_p00_input_root_failed")?,
            executable_manifest_root_sha256: composition_root_v1(&(
                &root.linked_manifest.manifest_root_sha256, &root.suite_manifest.manifest_root_sha256,
            )).map_err(|_| "r8b_v8_p00_manifest_root_failed")?,
            tool_manifest_root_sha256: composition_root_v1(&root.tools)
                .map_err(|_| "r8b_v8_p00_tool_root_failed")?,
            source_inventory_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_p00_source_root_failed")?,
            root,
        })
    }

    pub(super) fn route_id_sha256(&self) -> &str {
        &self.root.route_id_sha256
    }

    pub(super) fn revalidate(&self) -> ParentResultV3<()> {
        validate_live_manifest_v3(
            &self.root.linked_manifest,
            K2UncertaintyR8BManifestClassV2::Linked,
        )?;
        validate_live_manifest_v3(
            &self.root.suite_manifest,
            K2UncertaintyR8BManifestClassV2::Suite,
        )?;
        for input in &self.root.inputs {
            let expected = capture_input_v3(
                input.role,
                Path::new(&input.canonical_path),
                input.unix_mode,
                input.semantic_root_sha256.clone(),
            )?;
            if *input != expected {
                return Err("r8b_v8_p00_input_changed");
            }
        }
        let linked = input_path_v3(
            &self.root.inputs,
            K2UncertaintyR8BInputRoleV3::LinkedManifest,
        )?;
        let suite = input_path_v3(
            &self.root.inputs,
            K2UncertaintyR8BInputRoleV3::SuiteManifest,
        )?;
        validate_manifest_file_v3(linked, &self.root.linked_manifest)?;
        validate_manifest_file_v3(suite, &self.root.suite_manifest)?;
        self.root
            .tools
            .iter()
            .try_for_each(ParentToolFileIdentityV3::revalidate)?;
        let [source, inputs, manifests, tools] = self.transition_roots()?;
        if source != self.source_inventory_root_sha256
            || inputs != self.input_bindings_root_sha256
            || manifests != self.executable_manifest_root_sha256
            || tools != self.tool_manifest_root_sha256
        {
            return Err("r8b_v8_p00_capability_changed");
        }
        Ok(())
    }

    pub(super) fn transition_roots(&self) -> ParentResultV3<[String; 4]> {
        Ok([
            composition_root_v1(&self.root).map_err(|_| "r8b_v8_p00_source_root_failed")?,
            composition_root_v1(&self.root.inputs).map_err(|_| "r8b_v8_p00_input_root_failed")?,
            composition_root_v1(&(
                &self.root.linked_manifest.manifest_root_sha256,
                &self.root.suite_manifest.manifest_root_sha256,
            ))
            .map_err(|_| "r8b_v8_p00_manifest_root_failed")?,
            composition_root_v1(&self.root.tools).map_err(|_| "r8b_v8_p00_tool_root_failed")?,
        ])
    }
}

fn input_path_v3(
    inputs: &[K2UncertaintyR8BInputBindingV3],
    role: K2UncertaintyR8BInputRoleV3,
) -> ParentResultV3<&Path> {
    inputs
        .iter()
        .find(|input| input.role == role)
        .map(|input| Path::new(&input.canonical_path))
        .ok_or("r8b_v8_p00_input_missing")
}

fn capture_input_v3(
    role: K2UncertaintyR8BInputRoleV3,
    path: &Path,
    expected_mode: u32,
    semantic_root_sha256: String,
) -> ParentResultV3<K2UncertaintyR8BInputBindingV3> {
    require_composition_root_v1(&semantic_root_sha256)
        .map_err(|_| "r8b_v8_p00_input_semantic_root_invalid")?;
    let canonical = fs::canonicalize(path).map_err(|_| "r8b_v8_p00_input_missing")?;
    let metadata = fs::symlink_metadata(path).map_err(|_| "r8b_v8_p00_input_stat_failed")?;
    if canonical != path || metadata.permissions().mode() & 0o7777 != expected_mode {
        return Err("r8b_v8_p00_input_identity_invalid");
    }
    let content_sha256 = if metadata.is_file() {
        composition_sha256_file_v1(path).map_err(|_| "r8b_v8_p00_input_hash_failed")?
    } else if metadata.is_dir() {
        composition_root_v1(&tree_snapshot_v1(path))
            .map_err(|_| "r8b_v8_p00_input_tree_root_failed")?
    } else {
        return Err("r8b_v8_p00_input_kind_invalid");
    };
    Ok(K2UncertaintyR8BInputBindingV3 {
        role,
        canonical_path: canonical.to_string_lossy().into_owned(),
        unix_mode: expected_mode,
        byte_len: metadata.len(),
        content_sha256,
        semantic_root_sha256,
    })
}

fn validate_manifest_file_v3(
    path: &Path,
    expected: &K2UncertaintyR8BExecutableManifestV2,
) -> ParentResultV3<()> {
    let bytes = fs::read(path).map_err(|_| "r8b_v8_p00_manifest_read_failed")?;
    let observed: K2UncertaintyR8BExecutableManifestV2 =
        uncertainty_decode_v1(&bytes).map_err(|_| "r8b_v8_p00_manifest_decode_failed")?;
    if observed != *expected
        || uncertainty_bytes_v1(&observed).map_err(|_| "r8b_v8_p00_manifest_encode_failed")?
            != bytes
    {
        return Err("r8b_v8_p00_manifest_bytes_changed");
    }
    Ok(())
}

fn validate_live_manifest_v3(
    manifest: &K2UncertaintyR8BExecutableManifestV2,
    class: K2UncertaintyR8BManifestClassV2,
) -> ParentResultV3<()> {
    validate_manifest_shape_v3(manifest, class)?;
    for identity in &manifest.identities {
        let path = Path::new(&identity.canonical_path);
        let metadata = fs::symlink_metadata(path).map_err(|_| "r8b_v8_p00_executable_missing")?;
        if fs::canonicalize(path).map_err(|_| "r8b_v8_p00_executable_missing")? != path
            || metadata.len() != identity.byte_len
            || metadata.permissions().mode() & 0o7777 != identity.unix_mode
            || composition_sha256_file_v1(path).map_err(|_| "r8b_v8_p00_executable_hash_failed")?
                != identity.sha256
        {
            return Err("r8b_v8_p00_executable_changed");
        }
    }
    Ok(())
}

fn validate_manifest_shape_v3(
    manifest: &K2UncertaintyR8BExecutableManifestV2,
    class: K2UncertaintyR8BManifestClassV2,
) -> ParentResultV3<()> {
    manifest
        .validate()
        .map_err(|_| "r8b_v8_p00_manifest_invalid")?;
    if manifest.class != class {
        return Err("r8b_v8_p00_manifest_class_invalid");
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct P01ExpectedFileV3 {
    relative_path: String,
    byte_len: u64,
    unix_mode: u32,
    content_sha256: String,
    semantic_root_sha256: String,
}

struct P01ChannelPlanV3 {
    producer_role: String,
    request_root_sha256: String,
    terminal_event_root_sha256: String,
    output_directory: PathBuf,
    expected_files: Vec<P01ExpectedFileV3>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct P01ClosedChannelRootV3 {
    schema: String,
    route_id_sha256: String,
    producer_role: String,
    request_root_sha256: String,
    terminal_event_root_sha256: String,
    canonical_output_directory: String,
    directory_device: u64,
    directory_inode: u64,
    expected_files: Vec<P01ExpectedFileV3>,
}

struct P01ClosedChannelCapabilityV3 {
    descriptor: File,
    root: P01ClosedChannelRootV3,
    capability_root_sha256: String,
}

impl P01ClosedChannelCapabilityV3 {
    fn close(route_id_sha256: &str, mut plan: P01ChannelPlanV3) -> ParentResultV3<Self> {
        for root in [
            route_id_sha256,
            &plan.request_root_sha256,
            &plan.terminal_event_root_sha256,
        ] {
            require_composition_root_v1(root).map_err(|_| "r8b_v8_p01_root_invalid")?;
        }
        if !P00_SUITE_ROLES_V3.contains(&plan.producer_role.as_str())
            || plan.expected_files.is_empty()
        {
            return Err("r8b_v8_p01_plan_invalid");
        }
        plan.expected_files
            .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        validate_expected_files_v3(&plan.output_directory, &plan.expected_files, 0o700)?;
        freeze_directory_tree_v2(&plan.output_directory);
        let canonical =
            fs::canonicalize(&plan.output_directory).map_err(|_| "r8b_v8_p01_output_missing")?;
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(&canonical)
            .map_err(|_| "r8b_v8_p01_output_open_failed")?;
        let metadata = descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p01_output_stat_failed")?;
        let root = P01ClosedChannelRootV3 {
            schema: "nando.k2-self-formed-r8b-p01-closed-channel.v3".to_owned(),
            route_id_sha256: route_id_sha256.to_owned(),
            producer_role: plan.producer_role,
            request_root_sha256: plan.request_root_sha256,
            terminal_event_root_sha256: plan.terminal_event_root_sha256,
            canonical_output_directory: canonical.to_string_lossy().into_owned(),
            directory_device: metadata.dev(),
            directory_inode: metadata.ino(),
            expected_files: plan.expected_files,
        };
        let value = Self {
            capability_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_p01_channel_root_failed")?,
            descriptor,
            root,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        let metadata = self
            .descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p01_output_stat_failed")?;
        let path = Path::new(&self.root.canonical_output_directory);
        let live = fs::metadata(path).map_err(|_| "r8b_v8_p01_output_missing")?;
        validate_expected_files_v3(path, &self.root.expected_files, 0o500)?;
        if self.root.schema != "nando.k2-self-formed-r8b-p01-closed-channel.v3"
            || metadata.dev() != self.root.directory_device
            || metadata.ino() != self.root.directory_inode
            || live.dev() != self.root.directory_device
            || live.ino() != self.root.directory_inode
            || self.capability_root_sha256
                != composition_root_v1(&self.root).map_err(|_| "r8b_v8_p01_channel_root_failed")?
        {
            return Err("r8b_v8_p01_channel_changed");
        }
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct P01ProducerChannelsRootV3 {
    schema: String,
    route_id_sha256: String,
    channel_roots_sha256: Vec<String>,
}

pub(super) struct P01ProducerChannelsCapabilityV3 {
    channels: Vec<P01ClosedChannelCapabilityV3>,
    root: P01ProducerChannelsRootV3,
    capability_root_sha256: String,
}

impl P01ProducerChannelsCapabilityV3 {
    fn close(route_id_sha256: String, plans: Vec<P01ChannelPlanV3>) -> ParentResultV3<Self> {
        let mut channels = plans
            .into_iter()
            .map(|plan| P01ClosedChannelCapabilityV3::close(&route_id_sha256, plan))
            .collect::<ParentResultV3<Vec<_>>>()?;
        channels.sort_by(|left, right| left.root.producer_role.cmp(&right.root.producer_role));
        if channels
            .iter()
            .map(|channel| channel.root.producer_role.as_str())
            .collect::<Vec<_>>()
            != P00_SUITE_ROLES_V3
        {
            return Err("r8b_v8_p01_producer_set_invalid");
        }
        let root = P01ProducerChannelsRootV3 {
            schema: "nando.k2-self-formed-r8b-p01-producer-channels.v3".to_owned(),
            route_id_sha256,
            channel_roots_sha256: channels
                .iter()
                .map(|channel| channel.capability_root_sha256.clone())
                .collect(),
        };
        let value = Self {
            capability_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_p01_channels_root_failed")?,
            channels,
            root,
        };
        value.revalidate()?;
        Ok(value)
    }

    pub(super) fn revalidate_for_route(&self, route_id_sha256: &str) -> ParentResultV3<()> {
        self.channels
            .iter()
            .try_for_each(P01ClosedChannelCapabilityV3::revalidate)?;
        if self.root.route_id_sha256 != route_id_sha256
            || self.root.channel_roots_sha256
                != self
                    .channels
                    .iter()
                    .map(|channel| channel.capability_root_sha256.clone())
                    .collect::<Vec<_>>()
            || self.capability_root_sha256
                != composition_root_v1(&self.root).map_err(|_| "r8b_v8_p01_channels_root_failed")?
        {
            return Err("r8b_v8_p01_channels_changed");
        }
        Ok(())
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        self.revalidate_for_route(&self.root.route_id_sha256)
    }

    pub(super) fn transition_root(&self) -> String {
        self.capability_root_sha256.clone()
    }
}

fn validate_expected_files_v3(
    root: &Path,
    expected: &[P01ExpectedFileV3],
    directory_mode: u32,
) -> ParentResultV3<()> {
    let root_metadata = fs::symlink_metadata(root).map_err(|_| "r8b_v8_p01_output_missing")?;
    if !root_metadata.is_dir() || root_metadata.permissions().mode() & 0o7777 != directory_mode {
        return Err("r8b_v8_p01_output_directory_invalid");
    }
    let mut actual = BTreeSet::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let metadata =
            fs::symlink_metadata(&directory).map_err(|_| "r8b_v8_p01_output_stat_failed")?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o7777 != directory_mode {
            return Err("r8b_v8_p01_output_directory_invalid");
        }
        for entry in fs::read_dir(&directory).map_err(|_| "r8b_v8_p01_output_read_failed")? {
            let path = entry.map_err(|_| "r8b_v8_p01_output_read_failed")?.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| "r8b_v8_p01_output_stat_failed")?;
            if metadata.is_dir() {
                pending.push(path);
            } else if metadata.is_file() && !metadata.file_type().is_symlink() {
                actual.insert(
                    path.strip_prefix(root)
                        .map_err(|_| "r8b_v8_p01_relative_path_invalid")?
                        .to_string_lossy()
                        .into_owned(),
                );
            } else {
                return Err("r8b_v8_p01_output_kind_invalid");
            }
        }
    }
    let expected_paths = expected
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<BTreeSet<_>>();
    if actual != expected_paths || actual.len() != expected.len() {
        return Err("r8b_v8_p01_output_set_invalid");
    }
    for file in expected {
        if Path::new(&file.relative_path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err("r8b_v8_p01_relative_path_invalid");
        }
        require_composition_root_v1(&file.content_sha256)
            .map_err(|_| "r8b_v8_p01_file_root_invalid")?;
        require_composition_root_v1(&file.semantic_root_sha256)
            .map_err(|_| "r8b_v8_p01_file_root_invalid")?;
        let path = root.join(&file.relative_path);
        let metadata = fs::symlink_metadata(&path).map_err(|_| "r8b_v8_p01_file_missing")?;
        if metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != file.unix_mode
            || metadata.len() != file.byte_len
            || composition_sha256_file_v1(&path).map_err(|_| "r8b_v8_p01_file_hash_failed")?
                != file.content_sha256
        {
            return Err("r8b_v8_p01_file_changed");
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct P02TreeNodeV3 {
    relative_path: String,
    kind: String,
    unix_mode: u32,
    byte_len: u64,
    content_sha256: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct P02SnapshotEntryV3 {
    canonical_path: String,
    device: u64,
    inode: u64,
    tree_root_sha256: String,
}

struct P02SnapshotPathCapabilityV3 {
    descriptor: File,
    entry: P02SnapshotEntryV3,
}

impl P02SnapshotPathCapabilityV3 {
    fn capture(path: &Path) -> ParentResultV3<Self> {
        let canonical = fs::canonicalize(path).map_err(|_| "r8b_v8_p02_path_missing")?;
        if canonical != path {
            return Err("r8b_v8_p02_path_noncanonical");
        }
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| "r8b_v8_p02_path_open_failed")?;
        let metadata = descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p02_path_stat_failed")?;
        let entry = P02SnapshotEntryV3 {
            canonical_path: canonical.to_string_lossy().into_owned(),
            device: metadata.dev(),
            inode: metadata.ino(),
            tree_root_sha256: snapshot_tree_root_v3(path)?,
        };
        Ok(Self { descriptor, entry })
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        let path = Path::new(&self.entry.canonical_path);
        let held = self
            .descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p02_path_stat_failed")?;
        let live = fs::metadata(path).map_err(|_| "r8b_v8_p02_path_missing")?;
        if held.dev() != self.entry.device
            || held.ino() != self.entry.inode
            || live.dev() != self.entry.device
            || live.ino() != self.entry.inode
            || snapshot_tree_root_v3(path)? != self.entry.tree_root_sha256
        {
            return Err("r8b_v8_p02_snapshot_drift");
        }
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct P02SnapshotRootV3 {
    schema: String,
    route_id_sha256: String,
    entries: Vec<P02SnapshotEntryV3>,
}

pub(super) struct P02PreProductionSnapshotV3 {
    paths: Vec<P02SnapshotPathCapabilityV3>,
    root: P02SnapshotRootV3,
    snapshot_root_sha256: String,
}

impl P02PreProductionSnapshotV3 {
    pub(super) fn capture(
        route_id_sha256: String,
        mut paths: Vec<PathBuf>,
    ) -> ParentResultV3<Self> {
        require_composition_root_v1(&route_id_sha256).map_err(|_| "r8b_v8_p02_route_invalid")?;
        paths.sort();
        paths.dedup();
        if paths.is_empty() || paths.len() > 64 {
            return Err("r8b_v8_p02_path_set_invalid");
        }
        let paths = paths
            .into_iter()
            .map(|path| P02SnapshotPathCapabilityV3::capture(&path))
            .collect::<ParentResultV3<Vec<_>>>()?;
        let root = P02SnapshotRootV3 {
            schema: "nando.k2-self-formed-r8b-p02-pre-production-snapshot.v3".to_owned(),
            route_id_sha256,
            entries: paths.iter().map(|path| path.entry.clone()).collect(),
        };
        let value = Self {
            snapshot_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_p02_snapshot_root_failed")?,
            paths,
            root,
        };
        value.revalidate_for_route(&value.root.route_id_sha256)?;
        Ok(value)
    }

    pub(super) fn revalidate_for_route(&self, route_id_sha256: &str) -> ParentResultV3<()> {
        self.paths
            .iter()
            .try_for_each(P02SnapshotPathCapabilityV3::revalidate)?;
        if self.root.route_id_sha256 != route_id_sha256
            || self.root.entries
                != self
                    .paths
                    .iter()
                    .map(|path| path.entry.clone())
                    .collect::<Vec<_>>()
            || self.snapshot_root_sha256
                != composition_root_v1(&self.root).map_err(|_| "r8b_v8_p02_snapshot_root_failed")?
        {
            return Err("r8b_v8_p02_snapshot_changed");
        }
        Ok(())
    }

    pub(super) fn transition_root(&self) -> String {
        self.snapshot_root_sha256.clone()
    }
}

fn snapshot_tree_root_v3(root: &Path) -> ParentResultV3<String> {
    let mut pending = vec![root.to_path_buf()];
    let mut nodes = Vec::new();
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).map_err(|_| "r8b_v8_p02_tree_stat_failed")?;
        if metadata.file_type().is_symlink() {
            return Err("r8b_v8_p02_tree_symlink");
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "r8b_v8_p02_tree_relative_failed")?
            .to_string_lossy()
            .into_owned();
        let (kind, content_sha256) = if metadata.is_file() {
            (
                "file",
                Some(composition_sha256_file_v1(&path).map_err(|_| "r8b_v8_p02_tree_hash_failed")?),
            )
        } else if metadata.is_dir() {
            for entry in fs::read_dir(&path).map_err(|_| "r8b_v8_p02_tree_read_failed")? {
                pending.push(entry.map_err(|_| "r8b_v8_p02_tree_read_failed")?.path());
            }
            ("directory", None)
        } else {
            return Err("r8b_v8_p02_tree_kind_invalid");
        };
        nodes.push(P02TreeNodeV3 {
            relative_path: relative,
            kind: kind.to_owned(),
            unix_mode: metadata.permissions().mode() & 0o7777,
            byte_len: metadata.len(),
            content_sha256,
        });
    }
    nodes.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    composition_root_v1(&nodes).map_err(|_| "r8b_v8_p02_tree_root_failed")
}

pub(super) fn p02_snapshot_fixture_v3(
    parent: &Path,
    label: &str,
    route_id_sha256: &str,
) -> P02PreProductionSnapshotV3 {
    let production = parent.join(format!("p02-production-{label}"));
    create_private_directory_v1(&production);
    let sentinel = production.join("production-sentinel.json");
    write_new_read_only_v2(&sentinel, b"{\"production\":\"untouched\"}\n");
    P02PreProductionSnapshotV3::capture(route_id_sha256.to_owned(), vec![production])
        .expect("R8B V8 preproduction fixture")
}

pub(super) fn p00_source_fixture_v3(
    parent: &Path,
    label: &str,
    route_id_sha256: &str,
) -> P00SourceCapabilityV3 {
    let root = parent.join(format!("p00-source-{label}"));
    create_private_directory_v1(&root);
    let linked_manifest = executable_manifest_fixture_v3(
        &root,
        "linked-executables",
        K2UncertaintyR8BManifestClassV2::Linked,
        &P00_LINKED_ROLES_V3,
    );
    let suite_manifest = executable_manifest_fixture_v3(
        &root,
        "suite-executables",
        K2UncertaintyR8BManifestClassV2::Suite,
        &P00_SUITE_ROLES_V3,
    );
    let development_seed = root.join("development-seed.json");
    write_new_read_only_v2(&development_seed, b"{\"seed\":\"development\"}\n");
    let fixture_tree = root.join("fixture-tree");
    create_private_directory_v1(&fixture_tree);
    fs::set_permissions(&fixture_tree, fs::Permissions::from_mode(0o500))
        .expect("R8B V8 preproduction fixture");
    let linked_manifest_path = root.join("linked-manifest.json");
    write_new_read_only_v2(
        &linked_manifest_path,
        &uncertainty_bytes_v1(&linked_manifest).expect("R8B V8 preproduction fixture"),
    );
    let suite_manifest_path = root.join("suite-manifest.json");
    write_new_read_only_v2(
        &suite_manifest_path,
        &uncertainty_bytes_v1(&suite_manifest).expect("R8B V8 preproduction fixture"),
    );
    let process_ledger = root.join("process-ledger.jsonl");
    fs::write(&process_ledger, b"{}\n").expect("R8B V8 preproduction fixture");
    fs::set_permissions(&process_ledger, fs::Permissions::from_mode(0o600))
        .expect("R8B V8 preproduction fixture");
    let exclusive_output = root.join("exclusive-output");
    create_private_directory_v1(&exclusive_output);
    P00SourceCapabilityV3::bind(
        route_id_sha256.to_owned(),
        P00InputPathsV3 {
            development_seed,
            fixture_tree,
            linked_manifest: linked_manifest_path,
            suite_manifest: suite_manifest_path,
            process_ledger,
            exclusive_output,
            development_seed_semantic_root_sha256: root_v1("development-seed"),
        },
        linked_manifest,
        suite_manifest,
    )
    .expect("R8B V8 preproduction fixture")
}

fn executable_manifest_fixture_v3(
    root: &Path,
    directory: &str,
    class: K2UncertaintyR8BManifestClassV2,
    roles: &[&str],
) -> K2UncertaintyR8BExecutableManifestV2 {
    let executables = root.join(directory);
    create_private_directory_v1(&executables);
    let identities = roles
        .iter()
        .enumerate()
        .map(|(index, role)| {
            let path = executables.join(format!("executable-{index}"));
            fs::write(&path, format!("{role}\n")).expect("R8B V8 preproduction fixture");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o500))
                .expect("R8B V8 preproduction fixture");
            let metadata = fs::metadata(&path).expect("R8B V8 preproduction fixture");
            K2UncertaintyR8BExecutableIdentityV2 {
                role: (*role).to_owned(),
                canonical_path: fs::canonicalize(&path)
                    .expect("R8B V8 preproduction fixture")
                    .to_string_lossy()
                    .into_owned(),
                byte_len: metadata.len(),
                unix_mode: metadata.permissions().mode() & 0o7777,
                sha256: composition_sha256_file_v1(&path).expect("R8B V8 preproduction fixture"),
            }
        })
        .collect();
    K2UncertaintyR8BExecutableManifestV2::seal(class, identities)
        .expect("R8B V8 preproduction fixture")
}

pub(super) fn p01_channels_fixture_v3(
    parent: &Path,
    label: &str,
    route_id_sha256: &str,
) -> P01ProducerChannelsCapabilityV3 {
    let root = parent.join(format!("p01-channels-{label}"));
    create_private_directory_v1(&root);
    let plans = P00_SUITE_ROLES_V3
        .into_iter()
        .enumerate()
        .map(|(index, role)| {
            let output = root.join(format!("producer-{index}"));
            create_private_directory_v1(&output);
            let receipt_parent = output.join("receipts");
            create_private_directory_v1(&receipt_parent);
            let relative_path = format!("receipts/suite-{index}.json");
            let path = output.join(&relative_path);
            let bytes = uncertainty_bytes_v1(&(role, route_id_sha256, index))
                .expect("R8B V8 preproduction fixture");
            write_new_read_only_v2(&path, &bytes);
            P01ChannelPlanV3 {
                producer_role: role.to_owned(),
                request_root_sha256: root_v1(&format!("p01-request-{role}")),
                terminal_event_root_sha256: root_v1(&format!("p01-terminal-{role}")),
                output_directory: output,
                expected_files: vec![P01ExpectedFileV3 {
                    relative_path,
                    byte_len: bytes.len() as u64,
                    unix_mode: 0o400,
                    content_sha256: composition_sha256_bytes_v1(&bytes),
                    semantic_root_sha256: root_v1(&format!("p01-receipt-{role}")),
                }],
            }
        })
        .collect();
    P01ProducerChannelsCapabilityV3::close(route_id_sha256.to_owned(), plans)
        .expect("R8B V8 preproduction fixture")
}

#[test]
fn r8b_v8_p01_closes_exact_five_producer_channels() {
    let environment = TestEnvironmentV1::new("p01-positive");
    let route = root_v1("p01-positive");
    let channels = p01_channels_fixture_v3(&environment.root, "positive", &route);
    channels
        .revalidate_for_route(&route)
        .expect("R8B V8 preproduction fixture");
    assert_eq!(channels.channels.len(), 5);
}

#[test]
fn r8b_v8_p01_rejects_missing_extra_or_changed_output() {
    let environment = TestEnvironmentV1::new("p01-negative");
    let route = root_v1("p01-negative");
    let channels = p01_channels_fixture_v3(&environment.root, "negative", &route);
    assert!(
        channels
            .revalidate_for_route(&root_v1("foreign-route"))
            .is_err()
    );
    let path = Path::new(&channels.channels[0].root.canonical_output_directory)
        .join(&channels.channels[0].root.expected_files[0].relative_path);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
        .expect("R8B V8 preproduction fixture");
    assert!(channels.revalidate_for_route(&route).is_err());
}

#[test]
fn r8b_v8_p02_snapshot_detects_content_and_inode_drift() {
    let environment = TestEnvironmentV1::new("p02-drift");
    let route = root_v1("p02-drift");
    let snapshot = p02_snapshot_fixture_v3(&environment.root, "drift", &route);
    snapshot
        .revalidate_for_route(&route)
        .expect("R8B V8 preproduction fixture");
    let production = Path::new(&snapshot.root.entries[0].canonical_path);
    let sentinel = production.join("production-sentinel.json");
    fs::set_permissions(&sentinel, fs::Permissions::from_mode(0o600))
        .expect("R8B V8 preproduction fixture");
    fs::write(&sentinel, b"{\"production\":\"corrupted\"}\n")
        .expect("R8B V8 preproduction fixture");
    assert!(snapshot.revalidate_for_route(&route).is_err());
}
