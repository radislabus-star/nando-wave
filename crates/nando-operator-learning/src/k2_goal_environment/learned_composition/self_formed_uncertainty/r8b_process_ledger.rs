use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1,
    composition_sha256_file_v1, require_composition_root_v1,
};
use super::{
    K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1, K2UncertaintyImmutablePublicationFaultV1,
    K2UncertaintyR8BExecutableIdentityV2, K2UncertaintyR8BProcessEventKindV2,
    K2UncertaintyR8BProcessEventV2, K2UncertaintyR8BProcessLedgerV2,
    K2UncertaintyR8BProducedReceiptV2, publish_immutable_file_v1, read_immutable_file_v1,
    require_private_directory_v1, sync_directory_v1, uncertainty_bytes_v1, uncertainty_decode_v1,
};

pub const K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2: &str = "NANDO_R8B_LEDGER_ROOT_V2";
pub const K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2: &str = "NANDO_R8B_ROUTE_ID_V2";

#[derive(Clone, Debug)]
pub struct K2UncertaintyR8BLedgerWriterV2 {
    root: PathBuf,
    route_id_sha256: String,
    writer_role: String,
    writer_executable_sha256: String,
    allowed_children: BTreeMap<String, K2UncertaintyR8BExecutableIdentityV2>,
}

impl K2UncertaintyR8BLedgerWriterV2 {
    pub fn from_environment(
        writer_role: &str,
        writer_executable_sha256: String,
        allowed_children: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Option<Self>> {
        let Some(root) = std::env::var_os(K2_UNCERTAINTY_R8B_LEDGER_ROOT_ENV_V2) else {
            if std::env::var_os(K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2).is_some() {
                return Err(invalid_v1("self_formed_r8b_ledger_environment_partial"));
            }
            return Ok(None);
        };
        let route = std::env::var(K2_UNCERTAINTY_R8B_ROUTE_ID_ENV_V2)
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_environment_partial"))?;
        Self::new(
            PathBuf::from(root),
            route,
            writer_role,
            writer_executable_sha256,
            allowed_children,
        )
        .map(Some)
    }

    pub fn new(
        root: PathBuf,
        route_id_sha256: String,
        writer_role: &str,
        writer_executable_sha256: String,
        allowed_children: Vec<K2UncertaintyR8BExecutableIdentityV2>,
    ) -> K2CompositionResultV1<Self> {
        require_composition_root_v1(&route_id_sha256)?;
        require_composition_root_v1(&writer_executable_sha256)?;
        require_private_directory_v1(&root)?;
        if root
            != fs::canonicalize(&root)
                .map_err(|_| invalid_v1("self_formed_r8b_ledger_root_invalid"))?
            || writer_role.is_empty()
            || composition_sha256_file_v1(
                &std::env::current_exe()
                    .map_err(|_| invalid_v1("self_formed_r8b_writer_executable_missing"))?,
            )? != writer_executable_sha256
        {
            return Err(invalid_v1("self_formed_r8b_ledger_writer_invalid"));
        }
        let mut children = BTreeMap::new();
        for child in allowed_children {
            child.validate()?;
            if children.insert(child.role.clone(), child).is_some() {
                return Err(invalid_v1("self_formed_r8b_ledger_allowlist_invalid"));
            }
        }
        if children.is_empty() {
            return Err(invalid_v1("self_formed_r8b_ledger_allowlist_invalid"));
        }
        Ok(Self {
            root,
            route_id_sha256,
            writer_role: writer_role.to_owned(),
            writer_executable_sha256,
            allowed_children: children,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn child_started(
        &self,
        stage_id: &str,
        case_id_sha256: Option<String>,
        probe_ordinal: Option<u64>,
        child_role: &str,
        child_executable: &Path,
        request_root_sha256: String,
        stdin_sha256: String,
        started_monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        let child = self.require_child(child_role, child_executable)?;
        self.append_with_lock(|prefix| {
            let mut event = K2UncertaintyR8BProcessEventV2 {
                schema: String::new(),
                sequence: prefix.events.len() as u64,
                previous_event_root_sha256: prefix
                    .events
                    .last()
                    .map(|value| value.event_root_sha256.clone()),
                kind: K2UncertaintyR8BProcessEventKindV2::ChildStarted,
                route_id_sha256: self.route_id_sha256.clone(),
                stage_id: stage_id.to_owned(),
                case_id_sha256,
                probe_ordinal,
                writer_role: self.writer_role.clone(),
                writer_executable_sha256: self.writer_executable_sha256.clone(),
                role: child.role.clone(),
                executable_sha256: child.sha256.clone(),
                request_root_sha256,
                stdin_sha256,
                started_event_root_sha256: None,
                normal_exit: None,
                exit_code: None,
                stdout_byte_len: None,
                stdout_sha256: None,
                produced_receipts: Vec::new(),
                stderr_byte_len: None,
                stderr_sha256: None,
                started_monotonic_ns,
                finished_monotonic_ns: None,
                event_root_sha256: String::new(),
            };
            event.reseal()?;
            Ok(event)
        })
    }

    pub fn child_finished(
        &self,
        start: &K2UncertaintyR8BProcessEventV2,
        stdout: &[u8],
        stderr: &[u8],
        produced_receipts: Vec<K2UncertaintyR8BProducedReceiptV2>,
        finished_monotonic_ns: u64,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        self.append_with_lock(|prefix| {
            let observed_start = prefix
                .events
                .iter()
                .find(|event| event.event_root_sha256 == start.event_root_sha256);
            if observed_start != Some(start)
                || prefix.events.iter().any(|event| {
                    event.started_event_root_sha256.as_ref() == Some(&start.event_root_sha256)
                })
            {
                return Err(invalid_v1("self_formed_r8b_ledger_finish_start_invalid"));
            }
            let mut event = start.clone();
            event.sequence = prefix.events.len() as u64;
            event.previous_event_root_sha256 = prefix
                .events
                .last()
                .map(|value| value.event_root_sha256.clone());
            event.kind = K2UncertaintyR8BProcessEventKindV2::ChildFinished;
            event.started_event_root_sha256 = Some(start.event_root_sha256.clone());
            event.normal_exit = Some(true);
            event.exit_code = Some(0);
            event.stdout_byte_len = Some(stdout.len() as u64);
            event.stdout_sha256 = Some(composition_sha256_bytes_v1(stdout));
            event.produced_receipts = produced_receipts;
            event.stderr_byte_len = Some(stderr.len() as u64);
            event.stderr_sha256 = Some(composition_sha256_bytes_v1(stderr));
            event.finished_monotonic_ns = Some(finished_monotonic_ns);
            event.event_root_sha256.clear();
            event.reseal()?;
            Ok(event)
        })
    }

    pub fn complete_ledger(&self) -> K2CompositionResultV1<K2UncertaintyR8BProcessLedgerV2> {
        let directory =
            File::open(&self.root).map_err(|_| invalid_v1("self_formed_r8b_ledger_lock_open"))?;
        directory
            .lock()
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_lock"))?;
        let result = read_ledger_prefix_v2(&self.root, &self.route_id_sha256).and_then(|prefix| {
            K2UncertaintyR8BProcessLedgerV2::seal(prefix.route_id_sha256, prefix.events)
        });
        let _ = directory.unlock();
        result
    }

    fn require_child<'a>(
        &'a self,
        role: &str,
        path: &Path,
    ) -> K2CompositionResultV1<&'a K2UncertaintyR8BExecutableIdentityV2> {
        let child = self
            .allowed_children
            .get(role)
            .ok_or_else(|| invalid_v1("self_formed_r8b_ledger_child_not_allowed"))?;
        let metadata = fs::symlink_metadata(path)
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_child_missing"))?;
        if metadata.file_type().is_symlink()
            || path != Path::new(&child.canonical_path)
            || metadata.len() != child.byte_len
            || metadata.permissions().mode() & 0o7777 != child.unix_mode
            || composition_sha256_file_v1(path)? != child.sha256
        {
            return Err(invalid_v1("self_formed_r8b_ledger_child_identity_invalid"));
        }
        Ok(child)
    }

    fn append_with_lock(
        &self,
        build: impl FnOnce(
            &K2UncertaintyR8BProcessLedgerV2,
        ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2>,
    ) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV2> {
        let directory =
            File::open(&self.root).map_err(|_| invalid_v1("self_formed_r8b_ledger_lock_open"))?;
        directory
            .lock()
            .map_err(|_| invalid_v1("self_formed_r8b_ledger_lock"))?;
        let result = (|| {
            let prefix = read_ledger_prefix_v2(&self.root, &self.route_id_sha256)?;
            let event = build(&prefix)?;
            let path = self.root.join(format!("{:08}.json", event.sequence));
            publish_immutable_file_v1(
                &self.root,
                path.file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| invalid_v1("self_formed_r8b_ledger_event_path_invalid"))?,
                &uncertainty_bytes_v1(&event)?,
                0o400,
                event.sequence,
                K2UncertaintyImmutablePublicationFaultV1::None,
            )?;
            sync_directory_v1(&self.root)?;
            Ok(event)
        })();
        let _ = directory.unlock();
        result
    }
}

fn read_ledger_prefix_v2(
    root: &Path,
    route_id_sha256: &str,
) -> K2CompositionResultV1<K2UncertaintyR8BProcessLedgerV2> {
    let mut paths = fs::read_dir(root)
        .map_err(|_| invalid_v1("self_formed_r8b_ledger_read"))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid_v1("self_formed_r8b_ledger_entry_read"))?;
    paths.sort();
    let mut events = Vec::with_capacity(paths.len());
    for (sequence, path) in paths.into_iter().enumerate() {
        let expected = format!("{sequence:08}.json");
        if path.file_name().and_then(|value| value.to_str()) != Some(expected.as_str()) {
            return Err(invalid_v1("self_formed_r8b_ledger_natural_prefix_invalid"));
        }
        let file = read_immutable_file_v1(
            root,
            &expected,
            0o400,
            K2_UNCERTAINTY_IMMUTABLE_MAX_BYTES_V1,
        )?;
        let event: K2UncertaintyR8BProcessEventV2 = uncertainty_decode_v1(&file.bytes)?;
        if uncertainty_bytes_v1(&event)? != file.bytes || event.sequence != sequence as u64 {
            return Err(invalid_v1("self_formed_r8b_ledger_event_bytes_invalid"));
        }
        events.push(event);
    }
    K2UncertaintyR8BProcessLedgerV2::seal_natural_prefix(route_id_sha256.to_owned(), events)
}

fn invalid_v1(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
