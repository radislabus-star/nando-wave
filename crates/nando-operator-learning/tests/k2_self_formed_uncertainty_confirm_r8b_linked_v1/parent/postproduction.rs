use super::*;
use std::io::BufReader;

use nando_operator_learning::{
    K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3, K2_UNCERTAINTY_R8B_PUBLICATION_RECEIPT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3, K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3, K2UncertaintyR8BEvidenceKindV2,
    K2UncertaintyR8BLedgerSummaryV3, K2UncertaintyR8BObjectRoleV3,
    K2UncertaintyR8BPacketDescriptorV3, K2UncertaintyR8BPacketManifestV3,
    K2UncertaintyR8BScheduleAuthorityV3, seal_self_formed_r8b_ledger_header_v3,
    seal_self_formed_r8b_ledger_seal_v3, validate_self_formed_r8b_ledger_stream_v3,
    validate_self_formed_r8b_schedule_authority_v3,
};

#[derive(serde::Serialize)]
struct FrozenPacketDirectoryIdentityV3 {
    schema: String,
    route_id_sha256: String,
    canonical_path: String,
    device: u64,
    inode: u64,
    unix_mode: u32,
    manifest_root_sha256: String,
    ledger_seal_root_sha256: String,
}

pub(super) struct FrozenPacketDirectoryV3 {
    descriptor: File,
    canonical_path: PathBuf,
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) custody_root_sha256: String,
    pub(super) manifest_root_sha256: String,
    pub(super) ledger_seal_root_sha256: String,
    route_id_sha256: String,
    survival: K2UncertaintyR8BMeasuredReceiptV2,
}

impl FrozenPacketDirectoryV3 {
    pub(super) fn capture(
        path: &Path,
        route_id_sha256: &str,
        survival: &K2UncertaintyR8BMeasuredReceiptV2,
    ) -> ParentResultV3<Self> {
        require_composition_root_v1(route_id_sha256)
            .map_err(|_| "r8b_v8_packet_custody_root_invalid")?;
        survival
            .validate()
            .map_err(|_| "r8b_v8_packet_survival_invalid")?;
        let manifest = read_packet_manifest_v3(path)?;
        let ledger = read_packet_ledger_v3(path)?;
        validate_closed_packet_v3(path, route_id_sha256, survival, &manifest, &ledger)?;
        let manifest_root_sha256 = manifest.manifest_root_sha256.clone();
        let ledger_seal_root_sha256 = ledger
            .seal_root_sha256
            .ok_or("r8b_v8_packet_ledger_unsealed")?;
        let canonical_path =
            fs::canonicalize(path).map_err(|_| "r8b_v8_packet_custody_path_missing")?;
        if !path.is_absolute() || canonical_path != path {
            return Err("r8b_v8_packet_custody_path_invalid");
        }
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(path)
            .map_err(|_| "r8b_v8_packet_custody_open_failed")?;
        let metadata = descriptor
            .metadata()
            .map_err(|_| "r8b_v8_packet_custody_stat_failed")?;
        let canonical_text = canonical_path
            .to_str()
            .filter(|value| value.len() <= 240)
            .ok_or("r8b_v8_packet_custody_path_invalid")?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o7777 != 0o500 {
            return Err("r8b_v8_packet_custody_directory_invalid");
        }
        let identity = FrozenPacketDirectoryIdentityV3 {
            schema: "nando.k2-self-formed-r8b-p06-directory.v3".to_owned(),
            route_id_sha256: route_id_sha256.to_owned(),
            canonical_path: canonical_text.to_owned(),
            device: metadata.dev(),
            inode: metadata.ino(),
            unix_mode: 0o500,
            manifest_root_sha256: manifest_root_sha256.clone(),
            ledger_seal_root_sha256: ledger_seal_root_sha256.clone(),
        };
        let custody_root_sha256 =
            composition_root_v1(&identity).map_err(|_| "r8b_v8_packet_custody_root_failed")?;
        let mut flags = rustix::io::fcntl_getfd(&descriptor)
            .map_err(|_| "r8b_v8_packet_custody_flags_failed")?;
        flags.remove(rustix::io::FdFlags::CLOEXEC);
        rustix::io::fcntl_setfd(&descriptor, flags)
            .map_err(|_| "r8b_v8_packet_custody_flags_failed")?;
        let value = Self {
            descriptor,
            canonical_path,
            device: metadata.dev(),
            inode: metadata.ino(),
            custody_root_sha256,
            manifest_root_sha256,
            ledger_seal_root_sha256,
            route_id_sha256: route_id_sha256.to_owned(),
            survival: survival.clone(),
        };
        value.revalidate()?;
        Ok(value)
    }

    pub(super) fn revalidate(&self) -> ParentResultV3<()> {
        let descriptor = self
            .descriptor
            .metadata()
            .map_err(|_| "r8b_v8_packet_custody_descriptor_lost")?;
        let path = fs::symlink_metadata(&self.canonical_path)
            .map_err(|_| "r8b_v8_packet_custody_path_lost")?;
        let manifest = read_packet_manifest_v3(&self.canonical_path)?;
        let ledger = read_packet_ledger_v3(&self.canonical_path)?;
        validate_closed_packet_v3(
            &self.canonical_path,
            &self.route_id_sha256,
            &self.survival,
            &manifest,
            &ledger,
        )?;
        let identity = FrozenPacketDirectoryIdentityV3 {
            schema: "nando.k2-self-formed-r8b-p06-directory.v3".to_owned(),
            route_id_sha256: self.route_id_sha256.clone(),
            canonical_path: self
                .canonical_path
                .to_str()
                .ok_or("r8b_v8_packet_custody_path_invalid")?
                .to_owned(),
            device: self.device,
            inode: self.inode,
            unix_mode: 0o500,
            manifest_root_sha256: self.manifest_root_sha256.clone(),
            ledger_seal_root_sha256: self.ledger_seal_root_sha256.clone(),
        };
        if path.file_type().is_symlink()
            || !path.is_dir()
            || path.permissions().mode() & 0o7777 != 0o500
            || descriptor.dev() != self.device
            || descriptor.ino() != self.inode
            || path.dev() != self.device
            || path.ino() != self.inode
            || fs::canonicalize(&self.canonical_path)
                .map_err(|_| "r8b_v8_packet_custody_path_lost")?
                != self.canonical_path
            || manifest.manifest_root_sha256 != self.manifest_root_sha256
            || ledger.seal_root_sha256.as_ref() != Some(&self.ledger_seal_root_sha256)
            || composition_root_v1(&identity).map_err(|_| "r8b_v8_packet_custody_root_failed")?
                != self.custody_root_sha256
        {
            return Err("r8b_v8_packet_custody_changed");
        }
        Ok(())
    }

    pub(super) fn inherited_cwd(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.descriptor.as_raw_fd()))
    }
}

fn read_packet_manifest_v3(path: &Path) -> ParentResultV3<K2UncertaintyR8BPacketManifestV3> {
    let bytes = fs::read(path.join("packet-manifest.json"))
        .map_err(|_| "r8b_v8_packet_manifest_missing")?;
    let manifest: K2UncertaintyR8BPacketManifestV3 =
        uncertainty_decode_v1(&bytes).map_err(|_| "r8b_v8_packet_manifest_decode_failed")?;
    if uncertainty_bytes_v1(&manifest).map_err(|_| "r8b_v8_packet_manifest_encode_failed")? != bytes
    {
        return Err("r8b_v8_packet_manifest_noncanonical");
    }
    let mut canonical = manifest.clone();
    canonical.manifest_root_sha256.clear();
    if manifest.schema != K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3
        || manifest.manifest_root_sha256
            != uncertainty_root_v1(&canonical).map_err(|_| "r8b_v8_packet_manifest_root_failed")?
    {
        return Err("r8b_v8_packet_manifest_invalid");
    }
    Ok(manifest)
}

fn read_packet_ledger_v3(path: &Path) -> ParentResultV3<K2UncertaintyR8BLedgerSummaryV3> {
    let ledger =
        File::open(path.join("process-ledger.json")).map_err(|_| "r8b_v8_packet_ledger_missing")?;
    validate_self_formed_r8b_ledger_stream_v3(BufReader::new(ledger), true)
        .map_err(|_| "r8b_v8_packet_ledger_invalid")
}

fn validate_closed_packet_v3(
    path: &Path,
    route_id_sha256: &str,
    survival: &K2UncertaintyR8BMeasuredReceiptV2,
    manifest: &K2UncertaintyR8BPacketManifestV3,
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
) -> ParentResultV3<()> {
    let seal = ledger
        .seal_root_sha256
        .as_ref()
        .ok_or("r8b_v8_packet_ledger_unsealed")?;
    if manifest.route_id_sha256 != route_id_sha256
        || ledger.route_id_sha256 != route_id_sha256
        || manifest.c08_projection_root_sha256 != ledger.expected_projection_root_sha256
        || manifest.ledger_seal_root_sha256 != *seal
        || manifest.ledger_event_count != ledger.event_count
        || ledger.open_invocations != 0
        || ledger.fail_stopped
        || manifest.members.len() != 22
    {
        return Err("r8b_v8_packet_closure_binding_invalid");
    }
    let mut expected_paths = BTreeSet::from(["packet-manifest.json".to_owned()]);
    let mut evidence = BTreeSet::new();
    let mut roles = BTreeMap::new();
    let survival_bytes =
        uncertainty_bytes_v1(survival).map_err(|_| "r8b_v8_packet_survival_encode_failed")?;
    for descriptor in &manifest.members {
        let relative = Path::new(&descriptor.relative_path);
        require_composition_root_v1(&descriptor.content_sha256)
            .map_err(|_| "r8b_v8_packet_descriptor_root_invalid")?;
        require_composition_root_v1(&descriptor.semantic_root_sha256)
            .map_err(|_| "r8b_v8_packet_descriptor_root_invalid")?;
        if relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
            || !expected_paths.insert(descriptor.relative_path.clone())
        {
            return Err("r8b_v8_packet_descriptor_path_invalid");
        }
        *roles.entry(descriptor.object_role).or_insert(0_u64) += 1;
        if let Some(kind) = descriptor.evidence_kind {
            if descriptor.object_role != K2UncertaintyR8BObjectRoleV3::Evidence
                || !evidence.insert(kind)
            {
                return Err("r8b_v8_packet_evidence_census_invalid");
            }
        } else if descriptor.object_role == K2UncertaintyR8BObjectRoleV3::Evidence {
            return Err("r8b_v8_packet_evidence_census_invalid");
        }
        let member = path.join(relative);
        let metadata = fs::symlink_metadata(&member).map_err(|_| "r8b_v8_packet_member_missing")?;
        let bytes = fs::read(&member).map_err(|_| "r8b_v8_packet_member_read_failed")?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || descriptor.unix_mode != 0o400
            || descriptor.byte_len != bytes.len() as u64
            || descriptor.content_sha256 != composition_sha256_bytes_v1(&bytes)
        {
            return Err("r8b_v8_packet_member_changed");
        }
        if descriptor.evidence_kind == Some(K2UncertaintyR8BEvidenceKindV2::ProductionSurvival)
            && (descriptor.relative_path != "production/survival.json"
                || descriptor.semantic_root_sha256 != survival.receipt_root_sha256
                || bytes != survival_bytes)
        {
            return Err("r8b_v8_packet_survival_binding_invalid");
        }
    }
    let expected_roles = BTreeMap::from([
        (K2UncertaintyR8BObjectRoleV3::Evidence, 19),
        (
            K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
            1,
        ),
        (K2UncertaintyR8BObjectRoleV3::ResourceReceipt, 1),
        (K2UncertaintyR8BObjectRoleV3::ProcessLedger, 1),
    ]);
    if evidence != K2UncertaintyR8BEvidenceKindV2::ALL.into_iter().collect()
        || roles != expected_roles
        || packet_file_census_v3(path)? != expected_paths
        || !manifest
            .members
            .windows(2)
            .all(|pair| pair[0].relative_path < pair[1].relative_path)
    {
        return Err("r8b_v8_packet_exact_census_invalid");
    }
    Ok(())
}

fn packet_file_census_v3(root: &Path) -> ParentResultV3<BTreeSet<String>> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        let metadata =
            fs::symlink_metadata(&directory).map_err(|_| "r8b_v8_packet_directory_missing")?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o7777 != 0o500 {
            return Err("r8b_v8_packet_directory_invalid");
        }
        for entry in fs::read_dir(&directory).map_err(|_| "r8b_v8_packet_directory_read_failed")? {
            let member = entry
                .map_err(|_| "r8b_v8_packet_directory_read_failed")?
                .path();
            let metadata =
                fs::symlink_metadata(&member).map_err(|_| "r8b_v8_packet_member_missing")?;
            if metadata.is_dir() {
                pending.push(member);
            } else if metadata.is_file() && !metadata.file_type().is_symlink() {
                files.insert(
                    member
                        .strip_prefix(root)
                        .map_err(|_| "r8b_v8_packet_relative_path_invalid")?
                        .to_string_lossy()
                        .into_owned(),
                );
            } else {
                return Err("r8b_v8_packet_member_kind_invalid");
            }
        }
    }
    Ok(files)
}

pub(super) fn close_packet_fixture_v3(
    path: &Path,
    route_id_sha256: &str,
    survival: &K2UncertaintyR8BMeasuredReceiptV2,
) -> ParentResultV3<()> {
    if fs::read_dir(path)
        .map_err(|_| "r8b_v8_packet_fixture_missing")?
        .next()
        .is_some()
    {
        return Err("r8b_v8_packet_fixture_not_empty");
    }
    let projection_root_sha256 = root_v1("p06-fixture-projection");
    let schedule = schedule_authority_fixture_v3(
        root_v1("p06-fixture-schedule"),
        (0..16)
            .map(|index| root_v1(&format!("p06-fixture-case-{index}")))
            .collect(),
    )
    .map_err(|_| "r8b_v8_packet_fixture_schedule_failed")?;
    let header = seal_self_formed_r8b_ledger_header_v3(
        route_id_sha256.to_owned(),
        projection_root_sha256.clone(),
        schedule,
    )
    .map_err(|_| "r8b_v8_packet_fixture_header_failed")?;
    let seal = seal_self_formed_r8b_ledger_seal_v3(
        route_id_sha256.to_owned(),
        0,
        header.header_root_sha256.clone(),
    )
    .map_err(|_| "r8b_v8_packet_fixture_seal_failed")?;
    let mut ledger_bytes =
        uncertainty_bytes_v1(&header).map_err(|_| "r8b_v8_packet_fixture_ledger_failed")?;
    ledger_bytes.push(b'\n');
    ledger_bytes
        .extend(uncertainty_bytes_v1(&seal).map_err(|_| "r8b_v8_packet_fixture_ledger_failed")?);
    ledger_bytes.push(b'\n');

    let mut members = Vec::new();
    for (index, kind) in K2UncertaintyR8BEvidenceKindV2::ALL.into_iter().enumerate() {
        let (relative_path, bytes, semantic_root_sha256) =
            if kind == K2UncertaintyR8BEvidenceKindV2::ProductionSurvival {
                (
                    "production/survival.json".to_owned(),
                    uncertainty_bytes_v1(survival)
                        .map_err(|_| "r8b_v8_packet_fixture_survival_failed")?,
                    survival.receipt_root_sha256.clone(),
                )
            } else {
                let bytes = uncertainty_bytes_v1(&("p06-fixture-evidence", kind, route_id_sha256))
                    .map_err(|_| "r8b_v8_packet_fixture_evidence_failed")?;
                (
                    format!("evidence/{index:02}.json"),
                    bytes,
                    composition_root_v1(&("p06-fixture-evidence", kind, route_id_sha256))
                        .map_err(|_| "r8b_v8_packet_fixture_evidence_failed")?,
                )
            };
        members.push(write_fixture_member_v3(
            path,
            relative_path,
            K2UncertaintyR8BObjectRoleV3::Evidence,
            Some(kind),
            &bytes,
            semantic_root_sha256,
        )?);
    }
    for (relative_path, object_role, bytes, semantic_root_sha256) in [
        (
            "downstream-invocations.json",
            K2UncertaintyR8BObjectRoleV3::DownstreamInvocationContract,
            uncertainty_bytes_v1(&("p06-fixture-c08", route_id_sha256))
                .map_err(|_| "r8b_v8_packet_fixture_c08_failed")?,
            projection_root_sha256.clone(),
        ),
        (
            "resource-receipt.json",
            K2UncertaintyR8BObjectRoleV3::ResourceReceipt,
            uncertainty_bytes_v1(&("p06-fixture-resource", route_id_sha256))
                .map_err(|_| "r8b_v8_packet_fixture_resource_failed")?,
            root_v1("p06-fixture-resource"),
        ),
        (
            "process-ledger.json",
            K2UncertaintyR8BObjectRoleV3::ProcessLedger,
            ledger_bytes,
            seal.seal_root_sha256.clone(),
        ),
    ] {
        members.push(write_fixture_member_v3(
            path,
            relative_path.to_owned(),
            object_role,
            None,
            &bytes,
            semantic_root_sha256,
        )?);
    }
    members.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let mut manifest = K2UncertaintyR8BPacketManifestV3 {
        schema: K2_UNCERTAINTY_R8B_PACKET_MANIFEST_SCHEMA_V3.to_owned(),
        route_id_sha256: route_id_sha256.to_owned(),
        c08_projection_root_sha256: projection_root_sha256,
        resource_receipt_root_sha256: root_v1("p06-fixture-resource"),
        ledger_seal_root_sha256: seal.seal_root_sha256,
        ledger_event_count: 0,
        m16_completion_event_roots_sha256: Vec::new(),
        m16_receipt_roots_sha256: Vec::new(),
        m17_completion_event_roots_sha256: Vec::new(),
        m17_receipt_roots_sha256: Vec::new(),
        members,
        manifest_root_sha256: String::new(),
    };
    manifest.manifest_root_sha256 =
        uncertainty_root_v1(&manifest).map_err(|_| "r8b_v8_packet_fixture_manifest_failed")?;
    write_new_read_only_v2(
        &path.join("packet-manifest.json"),
        &uncertainty_bytes_v1(&manifest).map_err(|_| "r8b_v8_packet_fixture_manifest_failed")?,
    );
    freeze_directory_tree_v2(path);
    Ok(())
}

fn schedule_authority_fixture_v3(
    schedule_grammar_root_sha256: String,
    mut case_ids_sha256: Vec<String>,
) -> ParentResultV3<K2UncertaintyR8BScheduleAuthorityV3> {
    case_ids_sha256.sort();
    let mut value = K2UncertaintyR8BScheduleAuthorityV3 {
        schema: K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3.to_owned(),
        formula: K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3.to_owned(),
        schedule_grammar_root_sha256,
        case_ids_sha256,
        minimum_representatives: 8,
        maximum_representatives: 1_792,
        authority_root_sha256: String::new(),
    };
    value.authority_root_sha256 =
        uncertainty_root_v1(&value).map_err(|_| "r8b_v8_packet_fixture_schedule_failed")?;
    validate_self_formed_r8b_schedule_authority_v3(&value)
        .map_err(|_| "r8b_v8_packet_fixture_schedule_failed")?;
    Ok(value)
}

fn write_fixture_member_v3(
    root: &Path,
    relative_path: String,
    object_role: K2UncertaintyR8BObjectRoleV3,
    evidence_kind: Option<K2UncertaintyR8BEvidenceKindV2>,
    bytes: &[u8],
    semantic_root_sha256: String,
) -> ParentResultV3<K2UncertaintyR8BPacketDescriptorV3> {
    let path = root.join(&relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "r8b_v8_packet_fixture_parent_failed")?;
    }
    write_new_read_only_v2(&path, bytes);
    Ok(K2UncertaintyR8BPacketDescriptorV3 {
        relative_path,
        object_role,
        evidence_kind,
        byte_len: bytes.len() as u64,
        unix_mode: 0o400,
        content_sha256: composition_sha256_bytes_v1(bytes),
        semantic_root_sha256,
    })
}

impl P08PublishedV3 {
    fn revalidate_publication(&self) -> ParentResultV3<()> {
        self.transition.validate()?;
        self.authorization
            .validate()
            .map_err(|_| "r8b_v8_p08_authorization_invalid")?;
        self.publication
            .validate()
            .map_err(|_| "r8b_v8_p08_publication_invalid")?;
        if fs::canonicalize(&self.publication_root_path)
            .map_err(|_| "r8b_v8_p08_publication_root_missing")?
            != self.publication_root_path
        {
            return Err("r8b_v8_p08_publication_root_changed");
        }
        let request = K2UncertaintyR8BPublicationRequestV3::seal(
            self.publication_root_path.to_string_lossy().into_owned(),
            self.authorization.clone(),
        )
        .map_err(|_| "r8b_v8_p08_publication_request_invalid")?;
        let authorization_bytes = uncertainty_bytes_v1(&self.authorization)
            .map_err(|_| "r8b_v8_p08_authorization_encode_failed")?;
        let published = self
            .publication_root_path
            .join(K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3);
        let metadata =
            fs::symlink_metadata(&published).map_err(|_| "r8b_v8_p08_published_file_missing")?;
        if self.authorization.receipt_root_sha256 != self.authorization_root_sha256
            || self.authorization.manifest_root_sha256 != self.manifest_root_sha256
            || self.authorization.ledger_seal_root_sha256 != self.ledger_seal_root_sha256
            || self.publication.receipt_root_sha256 != self.publication_root_sha256
            || request.request_root_sha256 != self.publication.request_root_sha256
            || self.publication.authorization_receipt_root_sha256
                != self.authorization.receipt_root_sha256
            || self.publication.content_sha256 != composition_sha256_bytes_v1(&authorization_bytes)
            || self.publication.byte_len != authorization_bytes.len() as u64
            || metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || fs::read(published).map_err(|_| "r8b_v8_p08_published_file_read_failed")?
                != authorization_bytes
            || self.transition.stage != "p08-published"
            || self.transition.bindings
                != vec![
                    binding_v3("packet", self.packet_root_sha256.clone()),
                    binding_v3("authorization", self.authorization_root_sha256.clone()),
                    binding_v3("publication_request", request.request_root_sha256),
                    binding_v3("publication", self.publication_root_sha256.clone()),
                ]
        {
            return Err("r8b_v8_p08_capability_changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct P09AuthorityReceiptV3 {
    schema: String,
    route_id_sha256: String,
    p08_transition_root_sha256: String,
    packet_root_sha256: String,
    authorization_root_sha256: String,
    publication_root_sha256: String,
    auditor_executable_sha256: String,
    diagnostics_directory: String,
    diagnostics_device: u64,
    diagnostics_inode: u64,
    authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1,
    receipt_root_sha256: String,
}

impl P09AuthorityReceiptV3 {
    fn validate(&self) -> ParentResultV3<()> {
        for root in [
            &self.route_id_sha256,
            &self.p08_transition_root_sha256,
            &self.packet_root_sha256,
            &self.authorization_root_sha256,
            &self.publication_root_sha256,
            &self.auditor_executable_sha256,
            &self.receipt_root_sha256,
        ] {
            require_composition_root_v1(root).map_err(|_| "r8b_v8_p09_authority_root_invalid")?;
        }
        self.authority
            .validate()
            .map_err(|_| "r8b_v8_p09_authority_boundary_invalid")?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-p09-authority.v3"
            || !Path::new(&self.diagnostics_directory).is_absolute()
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_p09_authority_root_failed")?
        {
            return Err("r8b_v8_p09_authority_invalid");
        }
        Ok(())
    }
}

pub(super) struct P09AuthorityV3 {
    descriptor: File,
    canonical_path: PathBuf,
    device: u64,
    inode: u64,
    receipt: P09AuthorityReceiptV3,
}

impl P09AuthorityV3 {
    fn bind(
        path: &Path,
        p08: &P08PublishedV3,
        adapter: &P09ProcessAdapterV3,
        diagnostics_directory: &Path,
    ) -> ParentResultV3<Self> {
        p08.revalidate_publication()?;
        adapter.revalidate()?;
        let diagnostics = validate_p09_diagnostics_directory_v3(diagnostics_directory)?;
        if fs::read_dir(diagnostics_directory)
            .map_err(|_| "r8b_v8_p09_diagnostics_missing")?
            .next()
            .is_some()
        {
            return Err("r8b_v8_p09_diagnostics_not_empty");
        }
        let canonical_path =
            fs::canonicalize(path).map_err(|_| "r8b_v8_p09_authority_file_missing")?;
        let descriptor = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| "r8b_v8_p09_authority_file_open_failed")?;
        let metadata = descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p09_authority_file_stat_failed")?;
        let bytes = fs::read(path).map_err(|_| "r8b_v8_p09_authority_file_read_failed")?;
        let receipt: P09AuthorityReceiptV3 =
            uncertainty_decode_v1(&bytes).map_err(|_| "r8b_v8_p09_authority_file_decode_failed")?;
        receipt.validate()?;
        if canonical_path != path
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || uncertainty_bytes_v1(&receipt)
                .map_err(|_| "r8b_v8_p09_authority_file_encode_failed")?
                != bytes
            || receipt.route_id_sha256 != p08.transition.route_id_sha256
            || receipt.p08_transition_root_sha256 != p08.transition.receipt_root_sha256
            || receipt.packet_root_sha256 != p08.packet_root_sha256
            || receipt.authorization_root_sha256 != p08.authorization_root_sha256
            || receipt.publication_root_sha256 != p08.publication_root_sha256
            || receipt.auditor_executable_sha256 != adapter.binary.sha256
            || receipt.diagnostics_directory != diagnostics_directory.to_string_lossy()
            || (receipt.diagnostics_device, receipt.diagnostics_inode) != diagnostics
        {
            return Err("r8b_v8_p09_authority_binding_invalid");
        }
        let value = Self {
            descriptor,
            canonical_path,
            device: metadata.dev(),
            inode: metadata.ino(),
            receipt,
        };
        value.revalidate_for(p08, adapter)?;
        Ok(value)
    }

    fn revalidate_for(
        &self,
        p08: &P08PublishedV3,
        adapter: &P09ProcessAdapterV3,
    ) -> ParentResultV3<()> {
        p08.revalidate_publication()?;
        adapter.revalidate()?;
        self.receipt.validate()?;
        let held = self
            .descriptor
            .metadata()
            .map_err(|_| "r8b_v8_p09_authority_file_lost")?;
        let live = fs::symlink_metadata(&self.canonical_path)
            .map_err(|_| "r8b_v8_p09_authority_file_lost")?;
        let bytes =
            fs::read(&self.canonical_path).map_err(|_| "r8b_v8_p09_authority_file_read_failed")?;
        let diagnostics =
            validate_p09_diagnostics_directory_v3(Path::new(&self.receipt.diagnostics_directory))?;
        if live.file_type().is_symlink()
            || !live.is_file()
            || held.dev() != self.device
            || held.ino() != self.inode
            || live.dev() != self.device
            || live.ino() != self.inode
            || live.nlink() != 1
            || live.permissions().mode() & 0o7777 != 0o400
            || uncertainty_bytes_v1(&self.receipt)
                .map_err(|_| "r8b_v8_p09_authority_file_encode_failed")?
                != bytes
            || self.receipt.route_id_sha256 != p08.transition.route_id_sha256
            || self.receipt.p08_transition_root_sha256 != p08.transition.receipt_root_sha256
            || self.receipt.packet_root_sha256 != p08.packet_root_sha256
            || self.receipt.authorization_root_sha256 != p08.authorization_root_sha256
            || self.receipt.publication_root_sha256 != p08.publication_root_sha256
            || self.receipt.auditor_executable_sha256 != adapter.binary.sha256
            || (
                self.receipt.diagnostics_device,
                self.receipt.diagnostics_inode,
            ) != diagnostics
        {
            return Err("r8b_v8_p09_authority_changed");
        }
        Ok(())
    }
}

fn validate_p09_diagnostics_directory_v3(path: &Path) -> ParentResultV3<(u64, u64)> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "r8b_v8_p09_diagnostics_missing")?;
    if !path.is_absolute()
        || fs::canonicalize(path).map_err(|_| "r8b_v8_p09_diagnostics_missing")? != path
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o7777 != 0o700
    {
        return Err("r8b_v8_p09_diagnostics_invalid");
    }
    Ok((metadata.dev(), metadata.ino()))
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct P09DiagnosticRequestV3 {
    schema: String,
    authority_root_sha256: String,
    route_id_sha256: String,
    p08_transition_root_sha256: String,
    packet_root_sha256: String,
    authorization_root_sha256: String,
    publication_root_sha256: String,
    diagnostics_directory: String,
    auditor_executable_sha256: String,
    request_root_sha256: String,
}

impl P09DiagnosticRequestV3 {
    fn seal(authority: &P09AuthorityV3) -> ParentResultV3<Self> {
        let grant = &authority.receipt;
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-p09-diagnostic-request.v3".to_owned(),
            authority_root_sha256: grant.receipt_root_sha256.clone(),
            route_id_sha256: grant.route_id_sha256.clone(),
            p08_transition_root_sha256: grant.p08_transition_root_sha256.clone(),
            packet_root_sha256: grant.packet_root_sha256.clone(),
            authorization_root_sha256: grant.authorization_root_sha256.clone(),
            publication_root_sha256: grant.publication_root_sha256.clone(),
            diagnostics_directory: grant.diagnostics_directory.clone(),
            auditor_executable_sha256: grant.auditor_executable_sha256.clone(),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_p09_request_root_failed")?;
        Ok(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct P09DiagnosticReceiptV3 {
    schema: String,
    request_root_sha256: String,
    authority_root_sha256: String,
    route_id_sha256: String,
    p08_transition_root_sha256: String,
    packet_root_sha256: String,
    authorization_root_sha256: String,
    publication_root_sha256: String,
    diagnostic_relative_path: String,
    diagnostic_byte_len: u64,
    diagnostic_content_sha256: String,
    auditor_executable_sha256: String,
    disposition: String,
    authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1,
    receipt_root_sha256: String,
}

impl P09DiagnosticReceiptV3 {
    fn validate(&self) -> ParentResultV3<()> {
        for root in [
            &self.request_root_sha256,
            &self.authority_root_sha256,
            &self.route_id_sha256,
            &self.p08_transition_root_sha256,
            &self.packet_root_sha256,
            &self.authorization_root_sha256,
            &self.publication_root_sha256,
            &self.diagnostic_content_sha256,
            &self.auditor_executable_sha256,
            &self.receipt_root_sha256,
        ] {
            require_composition_root_v1(root).map_err(|_| "r8b_v8_p09_receipt_root_invalid")?;
        }
        self.authority
            .validate()
            .map_err(|_| "r8b_v8_p09_receipt_authority_invalid")?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-p09-diagnostic-receipt.v3"
            || self.diagnostic_relative_path != "audit.json"
            || self.diagnostic_byte_len == 0
            || self.disposition != "DIAGNOSTIC_ONLY"
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical).map_err(|_| "r8b_v8_p09_receipt_root_failed")?
        {
            return Err("r8b_v8_p09_receipt_invalid");
        }
        Ok(())
    }
}

struct P09ProcessAdapterV3 {
    binary: BinaryV2,
}

struct AcceptedP09DiagnosticV3 {
    receipt: P09DiagnosticReceiptV3,
}

impl P09ProcessAdapterV3 {
    fn new(binary: BinaryV2) -> ParentResultV3<Self> {
        validate_adapter_binary_v3(&binary, "P09_DIAGNOSTIC_AUDITOR")?;
        Ok(Self { binary })
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        validate_adapter_binary_v3(&self.binary, "P09_DIAGNOSTIC_AUDITOR")
    }

    fn run(
        &self,
        p08: &P08PublishedV3,
        authority: &P09AuthorityV3,
    ) -> ParentResultV3<AcceptedP09DiagnosticV3> {
        p08.revalidate_publication()?;
        authority.revalidate_for(p08, self)?;
        let request = P09DiagnosticRequestV3::seal(authority)?;
        let output = try_run_parent_process_v3(&self.binary.path, None, &request, 60)?;
        p08.revalidate_publication()?;
        authority.revalidate_for(p08, self)?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err("r8b_v8_p09_process_rejected");
        }
        self.accept_output(p08, authority, &request, &output.stdout)
    }

    fn accept_output(
        &self,
        p08: &P08PublishedV3,
        authority: &P09AuthorityV3,
        request: &P09DiagnosticRequestV3,
        stdout: &[u8],
    ) -> ParentResultV3<AcceptedP09DiagnosticV3> {
        p08.revalidate_publication()?;
        authority.revalidate_for(p08, self)?;
        if P09DiagnosticRequestV3::seal(authority)? != *request {
            return Err("r8b_v8_p09_request_changed");
        }
        let receipt: P09DiagnosticReceiptV3 =
            uncertainty_decode_v1(stdout).map_err(|_| "r8b_v8_p09_receipt_decode_failed")?;
        receipt.validate()?;
        let diagnostic =
            Path::new(&request.diagnostics_directory).join(&receipt.diagnostic_relative_path);
        let metadata =
            fs::symlink_metadata(&diagnostic).map_err(|_| "r8b_v8_p09_diagnostic_missing")?;
        let bytes = fs::read(diagnostic).map_err(|_| "r8b_v8_p09_diagnostic_read_failed")?;
        if uncertainty_bytes_v1(&receipt).map_err(|_| "r8b_v8_p09_receipt_encode_failed")? != stdout
            || receipt.request_root_sha256 != request.request_root_sha256
            || receipt.authority_root_sha256 != request.authority_root_sha256
            || receipt.route_id_sha256 != request.route_id_sha256
            || receipt.p08_transition_root_sha256 != request.p08_transition_root_sha256
            || receipt.packet_root_sha256 != request.packet_root_sha256
            || receipt.authorization_root_sha256 != request.authorization_root_sha256
            || receipt.publication_root_sha256 != request.publication_root_sha256
            || receipt.auditor_executable_sha256 != self.binary.sha256
            || metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || receipt.diagnostic_byte_len != bytes.len() as u64
            || receipt.diagnostic_content_sha256 != composition_sha256_bytes_v1(&bytes)
        {
            return Err("r8b_v8_p09_receipt_binding_invalid");
        }
        Ok(AcceptedP09DiagnosticV3 { receipt })
    }
}

pub(super) struct P09AuditedV3 {
    transition: ParentTransitionReceiptV3,
    packet_root_sha256: String,
    authorization_root_sha256: String,
    publication_root_sha256: String,
    diagnostic: P09DiagnosticReceiptV3,
}

impl P08PublishedV3 {
    fn audit(
        self,
        journal: &ParentJournalV3,
        authority: P09AuthorityV3,
        adapter: &P09ProcessAdapterV3,
    ) -> ParentResultV3<P09AuditedV3> {
        let diagnostic = adapter.run(&self, &authority)?;
        self.record_diagnostic(journal, authority, diagnostic, adapter)
    }

    fn record_diagnostic(
        self,
        journal: &ParentJournalV3,
        authority: P09AuthorityV3,
        diagnostic: AcceptedP09DiagnosticV3,
        adapter: &P09ProcessAdapterV3,
    ) -> ParentResultV3<P09AuditedV3> {
        self.revalidate_publication()?;
        authority.revalidate_for(&self, adapter)?;
        let diagnostic = diagnostic.receipt;
        let transition = advance_v3(
            journal,
            &self.transition,
            "p09-audited",
            vec![
                binding_v3("p09_authority", authority.receipt.receipt_root_sha256),
                binding_v3("diagnostic_request", diagnostic.request_root_sha256.clone()),
                binding_v3("diagnostic_receipt", diagnostic.receipt_root_sha256.clone()),
            ],
        )?;
        Ok(P09AuditedV3 {
            transition,
            packet_root_sha256: self.packet_root_sha256,
            authorization_root_sha256: self.authorization_root_sha256,
            publication_root_sha256: self.publication_root_sha256,
            diagnostic,
        })
    }
}

fn p06_survival_fixture_v3(
    route_id_sha256: &str,
    label: &str,
) -> K2UncertaintyR8BMeasuredReceiptV2 {
    K2UncertaintyR8BMeasuredReceiptV2::seal(
        K2UncertaintyR8BEvidenceKindV2::ProductionSurvival,
        route_id_sha256.to_owned(),
        vec![
            root_v1(&format!("{label}-pre")),
            root_v1(&format!("{label}-post")),
        ],
        1,
        BTreeMap::from([("stable_projection_equal".to_owned(), 1)]),
        root_v1(&format!("{label}-producer")),
    )
    .expect("R8B V8 postproduction fixture")
}

#[test]
fn r8b_v8_p06_rejects_extra_noncanonical_or_foreign_survival_packet() {
    let environment = TestEnvironmentV1::new("p06-closure-negative");
    let route = root_v1("p06-closure-negative");
    let survival = p06_survival_fixture_v3(&route, "p06-primary");

    let extra = environment.private_child("extra-packet");
    close_packet_fixture_v3(&extra, &route, &survival).expect("R8B V8 postproduction fixture");
    fs::set_permissions(&extra, fs::Permissions::from_mode(0o700))
        .expect("R8B V8 postproduction fixture");
    write_new_read_only_v2(&extra.join("extra.json"), b"{}\n");
    freeze_directory_tree_v2(&extra);
    assert!(FrozenPacketDirectoryV3::capture(&extra, &route, &survival).is_err());

    let noncanonical = environment.private_child("noncanonical-packet");
    close_packet_fixture_v3(&noncanonical, &route, &survival)
        .expect("R8B V8 postproduction fixture");
    let manifest = noncanonical.join("packet-manifest.json");
    fs::set_permissions(&manifest, fs::Permissions::from_mode(0o600))
        .expect("R8B V8 postproduction fixture");
    let mut bytes = fs::read(&manifest).expect("R8B V8 postproduction fixture");
    bytes.push(b'\n');
    fs::write(&manifest, bytes).expect("R8B V8 postproduction fixture");
    fs::set_permissions(&manifest, fs::Permissions::from_mode(0o400))
        .expect("R8B V8 postproduction fixture");
    assert!(FrozenPacketDirectoryV3::capture(&noncanonical, &route, &survival).is_err());

    let foreign = environment.private_child("foreign-survival-packet");
    close_packet_fixture_v3(&foreign, &route, &survival).expect("R8B V8 postproduction fixture");
    let replacement = p06_survival_fixture_v3(&route, "p06-foreign");
    assert!(FrozenPacketDirectoryV3::capture(&foreign, &route, &replacement).is_err());
}

fn p07_authorization_fixture_v3(
    request: &K2UncertaintyR8BAuthorizationRequestV3,
    route_id_sha256: String,
    publisher_executable_sha256: String,
) -> K2UncertaintyR8BAuthorizationReceiptV3 {
    let mut packet_member_roots_sha256 = (0..22)
        .map(|index| root_v1(&format!("p07-member-{index:02}")))
        .collect::<Vec<_>>();
    packet_member_roots_sha256.sort();
    let mut receipt = K2UncertaintyR8BAuthorizationReceiptV3 {
        schema: K2_UNCERTAINTY_R8B_AUTHORIZATION_RECEIPT_SCHEMA_V3.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        route_id_sha256,
        manifest_root_sha256: request.manifest_root_sha256.clone(),
        c08_projection_root_sha256: root_v1("p07-c08"),
        resource_receipt_root_sha256: root_v1("p07-resource"),
        ledger_seal_root_sha256: root_v1("p07-ledger"),
        packet_member_roots_sha256,
        publisher_executable_sha256,
        disposition: "R8B_FROZEN".to_owned(),
        authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 =
        uncertainty_root_v1(&receipt).expect("R8B V8 postproduction fixture");
    receipt.validate().expect("R8B V8 postproduction fixture");
    receipt
}

#[test]
fn r8b_v8_p07_accepts_only_canonical_request_bound_m25_receipt() {
    let binaries = LinkedBinariesV2::from_cargo();
    let adapter = M25ProcessAdapterV3::new(binaries.get("M25_R8B_AUTHORIZER"))
        .expect("R8B V8 postproduction fixture");
    let route = root_v1("p07-receipt");
    let request = K2UncertaintyR8BAuthorizationRequestV3::seal(
        route.clone(),
        root_v1("p07-manifest"),
        adapter.binary.sha256.clone(),
    )
    .expect("R8B V8 postproduction fixture");
    let receipt = p07_authorization_fixture_v3(&request, route, root_v1("p07-publisher"));
    let bytes = uncertainty_bytes_v1(&receipt).expect("R8B V8 postproduction fixture");
    assert_eq!(
        adapter
            .accept_output(&request, &bytes)
            .expect("R8B V8 postproduction fixture"),
        receipt
    );

    let mut noncanonical = bytes;
    noncanonical.push(b'\n');
    assert!(adapter.accept_output(&request, &noncanonical).is_err());
    let foreign = p07_authorization_fixture_v3(
        &request,
        root_v1("p07-foreign-route"),
        root_v1("p07-publisher"),
    );
    assert!(
        adapter
            .accept_output(
                &request,
                &uncertainty_bytes_v1(&foreign).expect("R8B V8 postproduction fixture")
            )
            .is_err()
    );
}

#[test]
fn r8b_v8_p08_accepts_only_exact_published_m25_bytes() {
    let environment = TestEnvironmentV1::new("p08-receipt");
    let (p08, m26, request) = p08_capability_fixture_v3(&environment, "p08");
    let receipt = p08.publication.clone();
    let bytes = uncertainty_bytes_v1(&receipt).expect("R8B V8 postproduction fixture");
    assert_eq!(
        m26.accept_output(&request, &bytes)
            .expect("R8B V8 postproduction fixture"),
        receipt
    );

    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert!(m26.accept_output(&request, &noncanonical).is_err());
    let published = p08
        .publication_root_path
        .join(K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3);
    fs::set_permissions(&published, fs::Permissions::from_mode(0o600))
        .expect("R8B V8 postproduction fixture");
    fs::write(&published, b"changed").expect("R8B V8 postproduction fixture");
    fs::set_permissions(&published, fs::Permissions::from_mode(0o400))
        .expect("R8B V8 postproduction fixture");
    assert!(m26.accept_output(&request, &bytes).is_err());
}

fn p08_capability_fixture_v3(
    environment: &TestEnvironmentV1,
    label: &str,
) -> (
    P08PublishedV3,
    M26ProcessAdapterV3,
    K2UncertaintyR8BPublicationRequestV3,
) {
    let publication_root = environment.private_child(&format!("{label}-publication"));
    let binaries = LinkedBinariesV2::from_cargo();
    let m25 = M25ProcessAdapterV3::new(binaries.get("M25_R8B_AUTHORIZER"))
        .expect("R8B V8 postproduction fixture");
    let m26 = M26ProcessAdapterV3::new(binaries.get("M26_R8B_PUBLISHER"))
        .expect("R8B V8 postproduction fixture");
    let route = root_v1(&format!("{label}-route"));
    let authorization_request = K2UncertaintyR8BAuthorizationRequestV3::seal(
        route.clone(),
        root_v1(&format!("{label}-manifest")),
        m25.binary.sha256.clone(),
    )
    .expect("R8B V8 postproduction fixture");
    let authorization = p07_authorization_fixture_v3(
        &authorization_request,
        route.clone(),
        m26.binary.sha256.clone(),
    );
    let request = K2UncertaintyR8BPublicationRequestV3::seal(
        publication_root.to_string_lossy().into_owned(),
        authorization,
    )
    .expect("R8B V8 postproduction fixture");
    let authorization_bytes =
        uncertainty_bytes_v1(&request.authorization).expect("R8B V8 postproduction fixture");
    write_new_read_only_v2(
        &publication_root.join(K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3),
        &authorization_bytes,
    );
    let mut receipt = K2UncertaintyR8BPublicationReceiptV3 {
        schema: K2_UNCERTAINTY_R8B_PUBLICATION_RECEIPT_SCHEMA_V3.to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        authorization_receipt_root_sha256: request.authorization.receipt_root_sha256.clone(),
        relative_path: K2_UNCERTAINTY_R8B_RECEIPT_PATH_V3.to_owned(),
        unix_mode: 0o400,
        byte_len: authorization_bytes.len() as u64,
        content_sha256: composition_sha256_bytes_v1(&authorization_bytes),
        publisher_executable_sha256: m26.binary.sha256.clone(),
        disposition: "R8B_FROZEN".to_owned(),
        authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 =
        uncertainty_root_v1(&receipt).expect("R8B V8 postproduction fixture");
    receipt.validate().expect("R8B V8 postproduction fixture");
    let packet_root_sha256 = root_v1(&format!("{label}-packet"));
    let transition = ParentTransitionReceiptV3::seal(
        route,
        "p08-published",
        root_v1(&format!("{label}-p07")),
        vec![
            binding_v3("packet", packet_root_sha256.clone()),
            binding_v3(
                "authorization",
                request.authorization.receipt_root_sha256.clone(),
            ),
            binding_v3("publication_request", request.request_root_sha256.clone()),
            binding_v3("publication", receipt.receipt_root_sha256.clone()),
        ],
    )
    .expect("R8B V8 postproduction fixture");
    let p08 = P08PublishedV3 {
        transition,
        packet_root_sha256,
        manifest_root_sha256: request.authorization.manifest_root_sha256.clone(),
        ledger_seal_root_sha256: request.authorization.ledger_seal_root_sha256.clone(),
        authorization_root_sha256: request.authorization.receipt_root_sha256.clone(),
        publication_root_sha256: receipt.receipt_root_sha256.clone(),
        authorization: request.authorization.clone(),
        publication: receipt,
        publication_root_path: publication_root,
    };
    p08.revalidate_publication()
        .expect("R8B V8 postproduction fixture");
    (p08, m26, request)
}

fn p09_adapter_fixture_v3() -> P09ProcessAdapterV3 {
    let path = fs::canonicalize(std::env::current_exe().expect("R8B V8 postproduction fixture"))
        .expect("R8B V8 postproduction fixture");
    P09ProcessAdapterV3::new(BinaryV2 {
        role: "P09_DIAGNOSTIC_AUDITOR",
        sha256: composition_sha256_file_v1(&path).expect("R8B V8 postproduction fixture"),
        path,
    })
    .expect("R8B V8 postproduction fixture")
}

fn p09_authority_fixture_v3(
    environment: &TestEnvironmentV1,
    p08: &P08PublishedV3,
    adapter: &P09ProcessAdapterV3,
    diagnostics: &Path,
) -> P09AuthorityV3 {
    let metadata = fs::metadata(diagnostics).expect("R8B V8 postproduction fixture");
    let mut receipt = P09AuthorityReceiptV3 {
        schema: "nando.k2-self-formed-r8b-p09-authority.v3".to_owned(),
        route_id_sha256: p08.transition.route_id_sha256.clone(),
        p08_transition_root_sha256: p08.transition.receipt_root_sha256.clone(),
        packet_root_sha256: p08.packet_root_sha256.clone(),
        authorization_root_sha256: p08.authorization_root_sha256.clone(),
        publication_root_sha256: p08.publication_root_sha256.clone(),
        auditor_executable_sha256: adapter.binary.sha256.clone(),
        diagnostics_directory: diagnostics.to_string_lossy().into_owned(),
        diagnostics_device: metadata.dev(),
        diagnostics_inode: metadata.ino(),
        authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 =
        uncertainty_root_v1(&receipt).expect("R8B V8 postproduction fixture");
    receipt.validate().expect("R8B V8 postproduction fixture");
    let directory = environment.private_child("p09-authority");
    let path = directory.join("authorization.json");
    write_new_read_only_v2(
        &path,
        &uncertainty_bytes_v1(&receipt).expect("R8B V8 postproduction fixture"),
    );
    P09AuthorityV3::bind(&path, p08, adapter, diagnostics).expect("R8B V8 postproduction fixture")
}

fn p09_diagnostic_fixture_v3(
    request: &P09DiagnosticRequestV3,
    adapter: &P09ProcessAdapterV3,
) -> P09DiagnosticReceiptV3 {
    let bytes = b"{\"diagnostic\":\"ok\"}\n";
    write_new_read_only_v2(
        &Path::new(&request.diagnostics_directory).join("audit.json"),
        bytes,
    );
    let mut receipt = P09DiagnosticReceiptV3 {
        schema: "nando.k2-self-formed-r8b-p09-diagnostic-receipt.v3".to_owned(),
        request_root_sha256: request.request_root_sha256.clone(),
        authority_root_sha256: request.authority_root_sha256.clone(),
        route_id_sha256: request.route_id_sha256.clone(),
        p08_transition_root_sha256: request.p08_transition_root_sha256.clone(),
        packet_root_sha256: request.packet_root_sha256.clone(),
        authorization_root_sha256: request.authorization_root_sha256.clone(),
        publication_root_sha256: request.publication_root_sha256.clone(),
        diagnostic_relative_path: "audit.json".to_owned(),
        diagnostic_byte_len: bytes.len() as u64,
        diagnostic_content_sha256: composition_sha256_bytes_v1(bytes),
        auditor_executable_sha256: adapter.binary.sha256.clone(),
        disposition: "DIAGNOSTIC_ONLY".to_owned(),
        authority: nando_operator_learning::K2CompositionAuthorityBoundaryV1::denied(),
        receipt_root_sha256: String::new(),
    };
    receipt.receipt_root_sha256 =
        uncertainty_root_v1(&receipt).expect("R8B V8 postproduction fixture");
    receipt.validate().expect("R8B V8 postproduction fixture");
    receipt
}

#[test]
fn r8b_v8_p09_requires_separate_authority_and_preserves_p08_bytes() {
    let environment = TestEnvironmentV1::new("p09-route");
    let (p08, _, _) = p08_capability_fixture_v3(&environment, "p09-primary");
    let adapter = p09_adapter_fixture_v3();
    let diagnostics = environment.private_child("p09-diagnostics");
    let authority = p09_authority_fixture_v3(&environment, &p08, &adapter, &diagnostics);
    let request = P09DiagnosticRequestV3::seal(&authority).expect("R8B V8 postproduction fixture");
    let receipt = p09_diagnostic_fixture_v3(&request, &adapter);
    let bytes = uncertainty_bytes_v1(&receipt).expect("R8B V8 postproduction fixture");

    let mut noncanonical = bytes.clone();
    noncanonical.push(b'\n');
    assert!(
        adapter
            .accept_output(&p08, &authority, &request, &noncanonical)
            .is_err()
    );
    let (foreign, _, _) = p08_capability_fixture_v3(&environment, "p09-foreign");
    assert!(authority.revalidate_for(&foreign, &adapter).is_err());
    let accepted = adapter
        .accept_output(&p08, &authority, &request, &bytes)
        .expect("R8B V8 postproduction fixture");
    let journal_root = environment.private_child("p09-journal");
    let journal = ParentJournalV3::new(&journal_root, p08.transition.route_id_sha256.clone())
        .expect("R8B V8 postproduction fixture");
    let audited = p08
        .record_diagnostic(&journal, authority, accepted, &adapter)
        .expect("R8B V8 postproduction fixture");
    assert_eq!(audited.transition.stage, "p09-audited");
    assert_eq!(audited.diagnostic.disposition, "DIAGNOSTIC_ONLY");
    require_composition_root_v1(&audited.packet_root_sha256)
        .expect("R8B V8 postproduction fixture");
    require_composition_root_v1(&audited.authorization_root_sha256)
        .expect("R8B V8 postproduction fixture");
    require_composition_root_v1(&audited.publication_root_sha256)
        .expect("R8B V8 postproduction fixture");
    let _process_route: fn(
        P08PublishedV3,
        &ParentJournalV3,
        P09AuthorityV3,
        &P09ProcessAdapterV3,
    ) -> ParentResultV3<P09AuditedV3> = P08PublishedV3::audit;
}
