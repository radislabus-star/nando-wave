use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use nando_operator_learning::{
    K2UncertaintyR8BAuthorizationReceiptV3, K2UncertaintyR8BAuthorizationRequestV3,
    K2UncertaintyR8BEvidenceKindV2, K2UncertaintyR8BExecutableIdentityV2,
    K2UncertaintyR8BExecutableManifestV2, K2UncertaintyR8BInputBindingV3,
    K2UncertaintyR8BInputRoleV3, K2UncertaintyR8BManagerIdentityV3,
    K2UncertaintyR8BManifestClassV2, K2UncertaintyR8BMeasuredReceiptV2,
    K2UncertaintyR8BPrivilegedProbeV3, K2UncertaintyR8BPublicationReceiptV3,
    K2UncertaintyR8BPublicationRequestV3, composition_root_v1, composition_sha256_bytes_v1,
    composition_sha256_file_v1, require_composition_root_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1, validate_self_formed_r8b_manager_identity_v3,
    validate_self_formed_r8b_privileged_probe_v3,
};

use super::{
    BinaryV2, CHILD_SELECTOR_V2, DelegatedChildOwnerV3, DelegatedLaunchContractV3,
    LinkedBinariesV2, SHA256SUM_V3, SUDO_V3, SYSTEMD_MANAGER_V3, TestEnvironmentV1,
    delegated_launch_argv_v3, freeze_directory_tree_v2, privileged_probe_argv_v3, root_v1,
    route_unit_v3, try_run_parent_process_v3, validate_delegated_launch_v3, write_new_read_only_v2,
};

#[path = "parent/resource.rs"]
mod resource;
use resource::{
    LoadedUnitResourceCapabilityV3, StoppedUnitResourceCapabilityV3,
    loaded_unit_resource_fixture_v3, stopped_unit_resource_fixture_v3,
};

#[path = "parent/postproduction.rs"]
mod postproduction;
use postproduction::FrozenPacketDirectoryV3;

#[path = "parent/preproduction.rs"]
mod preproduction;
use preproduction::{
    P00SourceCapabilityV3, P01ProducerChannelsCapabilityV3, P02PreProductionSnapshotV3,
    p00_source_fixture_v3, p01_channels_fixture_v3, p02_snapshot_fixture_v3,
};

type ParentResultV3<T> = Result<T, &'static str>;

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct ParentBindingV3 {
    role: String,
    root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct ParentTransitionReceiptV3 {
    schema: String,
    route_id_sha256: String,
    stage: String,
    previous_transition_root_sha256: String,
    bindings: Vec<ParentBindingV3>,
    receipt_root_sha256: String,
}

impl ParentTransitionReceiptV3 {
    fn seal(
        route_id_sha256: String,
        stage: &str,
        previous_transition_root_sha256: String,
        bindings: Vec<ParentBindingV3>,
    ) -> ParentResultV3<Self> {
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-parent-transition.v3".to_owned(),
            route_id_sha256,
            stage: stage.to_owned(),
            previous_transition_root_sha256,
            bindings,
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_parent_transition_root_failed")?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> ParentResultV3<()> {
        require_composition_root_v1(&self.route_id_sha256)
            .map_err(|_| "r8b_v8_parent_route_root_invalid")?;
        require_composition_root_v1(&self.previous_transition_root_sha256)
            .map_err(|_| "r8b_v8_parent_previous_root_invalid")?;
        require_composition_root_v1(&self.receipt_root_sha256)
            .map_err(|_| "r8b_v8_parent_receipt_root_invalid")?;
        let mut roles = BTreeSet::new();
        for binding in &self.bindings {
            require_composition_root_v1(&binding.root_sha256)
                .map_err(|_| "r8b_v8_parent_binding_root_invalid")?;
            if binding.role.is_empty()
                || !binding.role.is_ascii()
                || !roles.insert(binding.role.as_str())
            {
                return Err("r8b_v8_parent_binding_role_invalid");
            }
        }
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-parent-transition.v3"
            || self.stage.is_empty()
            || !self.stage.is_ascii()
            || self.bindings.is_empty()
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_parent_transition_root_failed")?
        {
            return Err("r8b_v8_parent_transition_invalid");
        }
        Ok(())
    }
}

struct ParentJournalV3 {
    root: PathBuf,
    route_id_sha256: String,
}

impl ParentJournalV3 {
    fn new(root: &Path, route_id_sha256: String) -> ParentResultV3<Self> {
        require_composition_root_v1(&route_id_sha256)
            .map_err(|_| "r8b_v8_parent_route_root_invalid")?;
        if !root.is_absolute()
            || fs::read_dir(root)
                .map_err(|_| "r8b_v8_parent_journal_missing")?
                .next()
                .is_some()
        {
            return Err("r8b_v8_parent_journal_not_empty");
        }
        Ok(Self {
            root: root.to_path_buf(),
            route_id_sha256,
        })
    }

    fn persist(&self, receipt: &ParentTransitionReceiptV3) -> ParentResultV3<()> {
        receipt.validate()?;
        if receipt.route_id_sha256 != self.route_id_sha256 {
            return Err("r8b_v8_parent_journal_route_mismatch");
        }
        let bytes =
            uncertainty_bytes_v1(receipt).map_err(|_| "r8b_v8_parent_transition_encode_failed")?;
        let path = self.root.join(format!("{}.json", receipt.stage));
        write_new_read_only_v2(&path, &bytes);
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "r8b_v8_parent_journal_sync_failed")?;
        if fs::read(path).map_err(|_| "r8b_v8_parent_transition_reopen_failed")? != bytes {
            return Err("r8b_v8_parent_transition_bytes_changed");
        }
        Ok(())
    }
}

fn binding_v3(role: &str, root_sha256: String) -> ParentBindingV3 {
    ParentBindingV3 {
        role: role.to_owned(),
        root_sha256,
    }
}

fn genesis_root_v3(route_id_sha256: &str) -> ParentResultV3<String> {
    composition_root_v1(&(
        "nando.k2-self-formed-r8b-parent-genesis.v3",
        route_id_sha256,
    ))
    .map_err(|_| "r8b_v8_parent_genesis_root_failed")
}

fn advance_v3(
    journal: &ParentJournalV3,
    previous: &ParentTransitionReceiptV3,
    stage: &str,
    bindings: Vec<ParentBindingV3>,
) -> ParentResultV3<ParentTransitionReceiptV3> {
    if previous.route_id_sha256 != journal.route_id_sha256 {
        return Err("r8b_v8_parent_transition_route_mismatch");
    }
    let receipt = ParentTransitionReceiptV3::seal(
        previous.route_id_sha256.clone(),
        stage,
        previous.receipt_root_sha256.clone(),
        bindings,
    )?;
    journal.persist(&receipt)?;
    Ok(receipt)
}

macro_rules! early_state_v3 {
    ($name:ident) => {
        struct $name {
            transition: ParentTransitionReceiptV3,
        }
    };
}

early_state_v3!(P00SourceValidatedV3);
early_state_v3!(P01ProducersClosedV3);

struct P05ProductionSurvivedV3 {
    transition: ParentTransitionReceiptV3,
    prior: P04BManagerReverifiedV3,
    survival: K2UncertaintyR8BMeasuredReceiptV2,
}

struct P02PreSnapshotFrozenV3 {
    transition: ParentTransitionReceiptV3,
    snapshot: P02PreProductionSnapshotV3,
}

struct P03AManagerBoundV3 {
    transition: ParentTransitionReceiptV3,
    snapshot: P02PreProductionSnapshotV3,
    manager_pre: ManagerPreProbeCapabilityV3,
}

struct P03BDelegatedRequestFrozenV3 {
    transition: ParentTransitionReceiptV3,
    snapshot: P02PreProductionSnapshotV3,
    launch: DelegatedLaunchCapabilityV3,
    manager_pre: ManagerPreProbeCapabilityV3,
}

struct P04AResourcesFrozenV3 {
    transition: ParentTransitionReceiptV3,
    snapshot: P02PreProductionSnapshotV3,
    manager_pre: ManagerPreProbeCapabilityV3,
    resources: LoadedUnitResourceCapabilityV3,
    producer_executable_sha256: String,
}

struct P04BManagerReverifiedV3 {
    transition: ParentTransitionReceiptV3,
    snapshot: P02PreProductionSnapshotV3,
    manager_pre: ManagerPreProbeCapabilityV3,
    resources: LoadedUnitResourceCapabilityV3,
    stopped: StoppedUnitResourceCapabilityV3,
    manager_post: ManagerPostProbeCapabilityV3,
    producer_executable_sha256: String,
}

struct P05HealthObservationV3 {
    successful_get_count: u64,
    maximum_latency_ns: u64,
}

impl P05HealthObservationV3 {
    fn new(successful_get_count: u64, maximum_latency_ns: u64) -> ParentResultV3<Self> {
        if !(1..=64).contains(&successful_get_count)
            || !(1..=5_000_000_000).contains(&maximum_latency_ns)
        {
            return Err("r8b_v8_p05_health_observation_invalid");
        }
        Ok(Self {
            successful_get_count,
            maximum_latency_ns,
        })
    }
}

impl P00SourceValidatedV3 {
    fn start(journal: &ParentJournalV3, source: P00SourceCapabilityV3) -> ParentResultV3<Self> {
        source.revalidate()?;
        if source.route_id_sha256() != journal.route_id_sha256 {
            return Err("r8b_v8_p00_source_route_mismatch");
        }
        let [
            source_inventory,
            input_bindings,
            executable_manifest,
            tool_manifest,
        ] = source.transition_roots()?;
        let transition = ParentTransitionReceiptV3::seal(
            journal.route_id_sha256.clone(),
            "p00-source-validated",
            genesis_root_v3(&journal.route_id_sha256)?,
            vec![
                binding_v3("source_inventory", source_inventory),
                binding_v3("input_bindings", input_bindings),
                binding_v3("executable_manifest", executable_manifest),
                binding_v3("tool_manifest", tool_manifest),
            ],
        )?;
        journal.persist(&transition)?;
        Ok(Self { transition })
    }

    fn close_producers(
        self,
        journal: &ParentJournalV3,
        channels: P01ProducerChannelsCapabilityV3,
    ) -> ParentResultV3<P01ProducersClosedV3> {
        channels.revalidate_for_route(&journal.route_id_sha256)?;
        Ok(P01ProducersClosedV3 {
            transition: advance_v3(
                journal,
                &self.transition,
                "p01-producers-closed",
                vec![binding_v3("producer_channels", channels.transition_root())],
            )?,
        })
    }
}

impl P01ProducersClosedV3 {
    fn freeze_pre_snapshot(
        self,
        journal: &ParentJournalV3,
        snapshot: P02PreProductionSnapshotV3,
    ) -> ParentResultV3<P02PreSnapshotFrozenV3> {
        snapshot.revalidate_for_route(&journal.route_id_sha256)?;
        Ok(P02PreSnapshotFrozenV3 {
            transition: advance_v3(
                journal,
                &self.transition,
                "p02-pre-snapshot-frozen",
                vec![binding_v3(
                    "pre_production_snapshot",
                    snapshot.transition_root(),
                )],
            )?,
            snapshot,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManagerUnitIdentityObservationV3 {
    owner_uid: u32,
    user_unit: String,
    invocation_id: String,
    main_pid: u32,
    exec_start: String,
    fragment_path: String,
    control_group: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManagerIdentityObservationV3 {
    bus_peer_pid: u32,
    bus_unique_name: String,
    bus_owner_uid: u32,
    proc_pid: u32,
    proc_start_ticks_before: u64,
    proc_start_ticks_after: u64,
    proc_uid: u32,
    command: Vec<String>,
    cgroup: String,
    boot_id_before: String,
    boot_id_after: String,
    unit: ManagerUnitIdentityObservationV3,
    version: String,
}

struct PidfdBindingV3 {
    descriptor: OwnedFd,
    pid: u32,
    device: u64,
    inode: u64,
}

impl PidfdBindingV3 {
    fn bind(descriptor: OwnedFd, expected_pid: u32) -> ParentResultV3<Self> {
        let stat =
            rustix::fs::fstat(&descriptor).map_err(|_| "r8b_v8_manager_pidfd_stat_failed")?;
        let value = Self {
            descriptor,
            pid: expected_pid,
            device: stat.st_dev as u64,
            inode: stat.st_ino as u64,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        let stat =
            rustix::fs::fstat(&self.descriptor).map_err(|_| "r8b_v8_manager_pidfd_stat_failed")?;
        if self.pid == 0
            || pidfd_target_pid_v3(self.descriptor.as_raw_fd())? != self.pid
            || stat.st_dev as u64 != self.device
            || stat.st_ino as u64 != self.inode
            || !pidfd_alive_v3(&self.descriptor)?
        {
            return Err("r8b_v8_manager_pidfd_identity_invalid");
        }
        Ok(())
    }
}

fn pidfd_target_pid_v3(raw_fd: i32) -> ParentResultV3<u32> {
    let fdinfo = fs::read_to_string(format!("/proc/self/fdinfo/{raw_fd}"))
        .map_err(|_| "r8b_v8_manager_pidfd_info_failed")?;
    let mut pid = None;
    for line in fdinfo.lines() {
        let Some(value) = line.strip_prefix("Pid:") else {
            continue;
        };
        let value = value.trim();
        let parsed = value
            .parse::<u32>()
            .map_err(|_| "r8b_v8_manager_pidfd_pid_invalid")?;
        if value != parsed.to_string() || pid.replace(parsed).is_some() {
            return Err("r8b_v8_manager_pidfd_pid_invalid");
        }
    }
    pid.ok_or("r8b_v8_manager_pidfd_pid_missing")
}

fn pidfd_alive_v3(descriptor: &OwnedFd) -> ParentResultV3<bool> {
    let mut descriptors = [rustix::event::PollFd::new(
        descriptor,
        rustix::event::PollFlags::IN,
    )];
    let timeout = rustix::event::Timespec::default();
    // A live pidfd is not readable. Readability means the owned process exited.
    let ready = rustix::event::poll(&mut descriptors, Some(&timeout))
        .map_err(|_| "r8b_v8_manager_pidfd_poll_failed")?;
    Ok(ready == 0 && descriptors[0].revents().is_empty())
}

#[derive(serde::Serialize)]
struct ManagerIdentityCapabilityRootV3 {
    schema: String,
    identity: K2UncertaintyR8BManagerIdentityV3,
    pidfd_pid: u32,
    pidfd_device: u64,
    pidfd_inode: u64,
}

struct ManagerIdentityCapabilityV3 {
    identity: K2UncertaintyR8BManagerIdentityV3,
    pidfd: PidfdBindingV3,
    identity_root_sha256: String,
}

impl ManagerIdentityCapabilityV3 {
    fn bind(observation: ManagerIdentityObservationV3, pidfd: OwnedFd) -> ParentResultV3<Self> {
        if observation.bus_peer_pid == 0
            || observation.bus_peer_pid != observation.proc_pid
            || observation.bus_peer_pid != observation.unit.main_pid
            || observation.bus_owner_uid != observation.proc_uid
            || observation.bus_owner_uid != observation.unit.owner_uid
            || observation.proc_start_ticks_before == 0
            || observation.proc_start_ticks_before != observation.proc_start_ticks_after
            || observation.boot_id_before != observation.boot_id_after
        {
            return Err("r8b_v8_manager_observation_disagrees");
        }
        let pidfd = PidfdBindingV3::bind(pidfd, observation.bus_peer_pid)?;
        let identity = K2UncertaintyR8BManagerIdentityV3 {
            bus_peer_pid: observation.bus_peer_pid,
            bus_unique_name: observation.bus_unique_name,
            pidfd_alive: true,
            boot_id: observation.boot_id_before,
            start_ticks: observation.proc_start_ticks_before,
            uid: observation.proc_uid,
            command: observation.command,
            cgroup: observation.cgroup,
            user_unit: observation.unit.user_unit,
            invocation_id: observation.unit.invocation_id,
            main_pid: observation.unit.main_pid,
            exec_start: observation.unit.exec_start,
            fragment_path: observation.unit.fragment_path,
            control_group: observation.unit.control_group,
            version: observation.version,
        };
        validate_self_formed_r8b_manager_identity_v3(&identity)
            .map_err(|_| "r8b_v8_manager_identity_invalid")?;
        let identity_root_sha256 = composition_root_v1(&ManagerIdentityCapabilityRootV3 {
            schema: "nando.k2-self-formed-r8b-manager-identity-capability.v3".to_owned(),
            identity: identity.clone(),
            pidfd_pid: pidfd.pid,
            pidfd_device: pidfd.device,
            pidfd_inode: pidfd.inode,
        })
        .map_err(|_| "r8b_v8_manager_identity_root_failed")?;
        let value = Self {
            identity,
            pidfd,
            identity_root_sha256,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        validate_self_formed_r8b_manager_identity_v3(&self.identity)
            .map_err(|_| "r8b_v8_manager_identity_invalid")?;
        self.pidfd.revalidate()?;
        let identity_root_sha256 = composition_root_v1(&ManagerIdentityCapabilityRootV3 {
            schema: "nando.k2-self-formed-r8b-manager-identity-capability.v3".to_owned(),
            identity: self.identity.clone(),
            pidfd_pid: self.pidfd.pid,
            pidfd_device: self.pidfd.device,
            pidfd_inode: self.pidfd.inode,
        })
        .map_err(|_| "r8b_v8_manager_identity_root_failed")?;
        if self.identity.bus_peer_pid != self.pidfd.pid
            || self.identity_root_sha256 != identity_root_sha256
        {
            return Err("r8b_v8_manager_identity_capability_changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct BoundManagerToolV3 {
    path: String,
    unix_mode: u32,
    byte_len: u64,
    sha256: String,
}

const PINNED_SYSTEMD_SHA256_V3: &str =
    "3c4b78ddb68e29e23da0465dd273f1ee82f5b9439ebfcec9798b395c05a2c1e3";
const PINNED_SUDO_SHA256_V3: &str =
    "c11aad50d0ac8e7d8fd483a83884a2ad95a1a3f4fea399fa061f06f0b8ce65b6";
const PINNED_SHA256SUM_SHA256_V3: &str =
    "48893b0fb21436b54619db80486e83ef39dfccaf1aefe83dfa00c02d6146e8c0";

fn validate_bound_manager_tool_v3(
    value: &BoundManagerToolV3,
    expected_path: &str,
    expected_mode: u32,
    expected_byte_len: u64,
    expected_sha256: &str,
) -> ParentResultV3<()> {
    require_composition_root_v1(&value.sha256).map_err(|_| "r8b_v8_manager_tool_root_invalid")?;
    if value.path != expected_path
        || value.unix_mode != expected_mode
        || value.byte_len != expected_byte_len
        || value.sha256 != expected_sha256
    {
        return Err("r8b_v8_manager_tool_identity_invalid");
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct ManagerProbeToolIdentityV3 {
    schema: String,
    systemd: BoundManagerToolV3,
    sudo: BoundManagerToolV3,
    sha256sum: BoundManagerToolV3,
    identity_root_sha256: String,
}

impl ManagerProbeToolIdentityV3 {
    fn bind(
        systemd: BoundManagerToolV3,
        sudo: BoundManagerToolV3,
        sha256sum: BoundManagerToolV3,
    ) -> ParentResultV3<Self> {
        validate_bound_manager_tool_v3(
            &systemd,
            SYSTEMD_MANAGER_V3,
            0o755,
            141_776,
            PINNED_SYSTEMD_SHA256_V3,
        )?;
        validate_bound_manager_tool_v3(&sudo, SUDO_V3, 0o4755, 1_082_656, PINNED_SUDO_SHA256_V3)?;
        validate_bound_manager_tool_v3(
            &sha256sum,
            SHA256SUM_V3,
            0o755,
            11_352_352,
            PINNED_SHA256SUM_SHA256_V3,
        )?;
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-manager-probe-tools.v3".to_owned(),
            systemd,
            sudo,
            sha256sum,
            identity_root_sha256: String::new(),
        };
        value.identity_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_manager_tool_identity_root_failed")?;
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        validate_bound_manager_tool_v3(
            &self.systemd,
            SYSTEMD_MANAGER_V3,
            0o755,
            141_776,
            PINNED_SYSTEMD_SHA256_V3,
        )?;
        validate_bound_manager_tool_v3(
            &self.sudo,
            SUDO_V3,
            0o4755,
            1_082_656,
            PINNED_SUDO_SHA256_V3,
        )?;
        validate_bound_manager_tool_v3(
            &self.sha256sum,
            SHA256SUM_V3,
            0o755,
            11_352_352,
            PINNED_SHA256SUM_SHA256_V3,
        )?;
        let mut canonical = self.clone();
        canonical.identity_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-manager-probe-tools.v3"
            || self.identity_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_manager_tool_identity_root_failed")?
        {
            return Err("r8b_v8_manager_tool_identity_changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct PrivilegedProbeRequestV3 {
    schema: String,
    manager_pid: u32,
    tool_identity_root_sha256: String,
    argv: Vec<String>,
    request_root_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrivilegedProbeExecutionV3 {
    argv: Vec<String>,
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    started_monotonic_ns: u64,
    finished_monotonic_ns: u64,
}

impl PrivilegedProbeRequestV3 {
    fn new(
        manager: &ManagerIdentityCapabilityV3,
        tools: &ManagerProbeToolIdentityV3,
    ) -> ParentResultV3<Self> {
        manager.revalidate()?;
        tools.revalidate()?;
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-privileged-probe-request.v3".to_owned(),
            manager_pid: manager.identity.bus_peer_pid,
            tool_identity_root_sha256: tools.identity_root_sha256.clone(),
            argv: privileged_probe_argv_v3(manager.identity.bus_peer_pid),
            request_root_sha256: String::new(),
        };
        value.request_root_sha256 = uncertainty_root_v1(&value)
            .map_err(|_| "r8b_v8_privileged_probe_request_root_failed")?;
        value.revalidate(manager, tools)?;
        Ok(value)
    }

    fn revalidate(
        &self,
        manager: &ManagerIdentityCapabilityV3,
        tools: &ManagerProbeToolIdentityV3,
    ) -> ParentResultV3<()> {
        let mut canonical = self.clone();
        canonical.request_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-privileged-probe-request.v3"
            || self.manager_pid != manager.identity.bus_peer_pid
            || self.tool_identity_root_sha256 != tools.identity_root_sha256
            || self.argv != privileged_probe_argv_v3(self.manager_pid)
            || self.request_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_privileged_probe_request_root_failed")?
        {
            return Err("r8b_v8_privileged_probe_request_invalid");
        }
        Ok(())
    }

    fn complete(
        &self,
        tools: &ManagerProbeToolIdentityV3,
        execution: PrivilegedProbeExecutionV3,
    ) -> ParentResultV3<K2UncertaintyR8BPrivilegedProbeV3> {
        let target = format!("/proc/{}/exe", self.manager_pid);
        let live_image_sha256 = parse_privileged_probe_stdout_v3(&execution.stdout, &target)?;
        if execution.argv != self.argv
            || execution.exit_code != 0
            || !execution.stderr.is_empty()
            || execution.started_monotonic_ns >= execution.finished_monotonic_ns
            || live_image_sha256 != tools.systemd.sha256
        {
            return Err("r8b_v8_privileged_probe_completion_invalid");
        }
        let value = K2UncertaintyR8BPrivilegedProbeV3 {
            sudo_sha256: tools.sudo.sha256.clone(),
            sha256sum_sha256: tools.sha256sum.sha256.clone(),
            argv: execution.argv,
            exit_code: execution.exit_code,
            stdout_byte_len: execution.stdout.len() as u64,
            stdout_sha256: composition_sha256_bytes_v1(&execution.stdout),
            stderr_byte_len: execution.stderr.len() as u64,
            stderr_sha256: composition_sha256_bytes_v1(&execution.stderr),
            live_image_sha256,
            started_monotonic_ns: execution.started_monotonic_ns,
            finished_monotonic_ns: execution.finished_monotonic_ns,
        };
        validate_self_formed_r8b_privileged_probe_v3(
            &value,
            self.manager_pid,
            &tools.systemd.sha256,
        )
        .map_err(|_| "r8b_v8_privileged_probe_receipt_invalid")?;
        Ok(value)
    }
}

fn parse_privileged_probe_stdout_v3(
    stdout: &[u8],
    expected_target: &str,
) -> ParentResultV3<String> {
    let target = expected_target.as_bytes();
    let expected_len = 64usize
        .checked_add(2)
        .and_then(|value| value.checked_add(target.len()))
        .and_then(|value| value.checked_add(1))
        .ok_or("r8b_v8_privileged_probe_stdout_oversized")?;
    if stdout.len() != expected_len
        || stdout.get(64..66) != Some(b" *")
        || stdout.get(66..66 + target.len()) != Some(target)
        || stdout.last() != Some(&0)
    {
        return Err("r8b_v8_privileged_probe_stdout_invalid");
    }
    let live_image_sha256 =
        std::str::from_utf8(&stdout[..64]).map_err(|_| "r8b_v8_privileged_probe_hash_invalid")?;
    require_composition_root_v1(live_image_sha256)
        .map_err(|_| "r8b_v8_privileged_probe_hash_invalid")?;
    Ok(live_image_sha256.to_owned())
}

struct ManagerPreProbeCapabilityV3 {
    manager: ManagerIdentityCapabilityV3,
    tools: ManagerProbeToolIdentityV3,
    request: PrivilegedProbeRequestV3,
    probe: K2UncertaintyR8BPrivilegedProbeV3,
    probe_root_sha256: String,
    capability_root_sha256: String,
}

impl ManagerPreProbeCapabilityV3 {
    fn bind(
        manager: ManagerIdentityCapabilityV3,
        tools: ManagerProbeToolIdentityV3,
        execution: PrivilegedProbeExecutionV3,
    ) -> ParentResultV3<Self> {
        let request = PrivilegedProbeRequestV3::new(&manager, &tools)?;
        let probe = request.complete(&tools, execution)?;
        let probe_root_sha256 =
            uncertainty_root_v1(&probe).map_err(|_| "r8b_v8_privileged_probe_root_failed")?;
        let capability_root_sha256 = composition_root_v1(&(
            "nando.k2-self-formed-r8b-manager-pre-probe-capability.v3",
            &manager.identity_root_sha256,
            &tools.identity_root_sha256,
            &request.request_root_sha256,
            &probe_root_sha256,
        ))
        .map_err(|_| "r8b_v8_manager_pre_probe_root_failed")?;
        let value = Self {
            manager,
            tools,
            request,
            probe,
            probe_root_sha256,
            capability_root_sha256,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        self.manager.revalidate()?;
        self.tools.revalidate()?;
        self.request.revalidate(&self.manager, &self.tools)?;
        validate_self_formed_r8b_privileged_probe_v3(
            &self.probe,
            self.manager.identity.bus_peer_pid,
            &self.tools.systemd.sha256,
        )
        .map_err(|_| "r8b_v8_privileged_probe_receipt_invalid")?;
        let probe_root_sha256 =
            uncertainty_root_v1(&self.probe).map_err(|_| "r8b_v8_privileged_probe_root_failed")?;
        let capability_root_sha256 = composition_root_v1(&(
            "nando.k2-self-formed-r8b-manager-pre-probe-capability.v3",
            &self.manager.identity_root_sha256,
            &self.tools.identity_root_sha256,
            &self.request.request_root_sha256,
            &probe_root_sha256,
        ))
        .map_err(|_| "r8b_v8_manager_pre_probe_root_failed")?;
        if self.probe.sudo_sha256 != self.tools.sudo.sha256
            || self.probe.sha256sum_sha256 != self.tools.sha256sum.sha256
            || self.probe_root_sha256 != probe_root_sha256
            || self.capability_root_sha256 != capability_root_sha256
        {
            return Err("r8b_v8_manager_pre_probe_capability_changed");
        }
        Ok(())
    }
}

struct ManagerPostProbeCapabilityV3 {
    manager: ManagerIdentityCapabilityV3,
    tools: ManagerProbeToolIdentityV3,
    request: PrivilegedProbeRequestV3,
    probe: K2UncertaintyR8BPrivilegedProbeV3,
    probe_root_sha256: String,
    capability_root_sha256: String,
}

impl ManagerPostProbeCapabilityV3 {
    fn bind(
        pre: &ManagerPreProbeCapabilityV3,
        manager: ManagerIdentityCapabilityV3,
        execution: PrivilegedProbeExecutionV3,
        after_monotonic_ns: u64,
    ) -> ParentResultV3<Self> {
        pre.revalidate()?;
        manager.revalidate()?;
        let tools = pre.tools.clone();
        let request = PrivilegedProbeRequestV3::new(&manager, &tools)?;
        let probe = request.complete(&tools, execution)?;
        let probe_root_sha256 =
            uncertainty_root_v1(&probe).map_err(|_| "r8b_v8_manager_post_probe_root_failed")?;
        let capability_root_sha256 = composition_root_v1(&(
            "nando.k2-self-formed-r8b-manager-post-probe-capability.v3",
            &pre.capability_root_sha256,
            &manager.identity_root_sha256,
            &tools.identity_root_sha256,
            &request.request_root_sha256,
            &probe_root_sha256,
            after_monotonic_ns,
        ))
        .map_err(|_| "r8b_v8_manager_post_capability_root_failed")?;
        let value = Self {
            manager,
            tools,
            request,
            probe,
            probe_root_sha256,
            capability_root_sha256,
        };
        value.revalidate_for_pre(pre, after_monotonic_ns)?;
        Ok(value)
    }

    fn revalidate_for_pre(
        &self,
        pre: &ManagerPreProbeCapabilityV3,
        after_monotonic_ns: u64,
    ) -> ParentResultV3<()> {
        pre.revalidate()?;
        self.manager.revalidate()?;
        self.tools.revalidate()?;
        self.request.revalidate(&self.manager, &self.tools)?;
        validate_self_formed_r8b_privileged_probe_v3(
            &self.probe,
            self.manager.identity.bus_peer_pid,
            &self.tools.systemd.sha256,
        )
        .map_err(|_| "r8b_v8_manager_post_probe_invalid")?;
        let probe_root_sha256 = uncertainty_root_v1(&self.probe)
            .map_err(|_| "r8b_v8_manager_post_probe_root_failed")?;
        let capability_root_sha256 = composition_root_v1(&(
            "nando.k2-self-formed-r8b-manager-post-probe-capability.v3",
            &pre.capability_root_sha256,
            &self.manager.identity_root_sha256,
            &self.tools.identity_root_sha256,
            &self.request.request_root_sha256,
            &probe_root_sha256,
            after_monotonic_ns,
        ))
        .map_err(|_| "r8b_v8_manager_post_capability_root_failed")?;
        if self.manager.identity != pre.manager.identity
            || self.tools != pre.tools
            || self.probe.stdout_sha256 != pre.probe.stdout_sha256
            || self.probe.live_image_sha256 != pre.probe.live_image_sha256
            || self.probe.started_monotonic_ns < after_monotonic_ns
            || self.probe.started_monotonic_ns >= self.probe.finished_monotonic_ns
            || self.probe_root_sha256 != probe_root_sha256
            || self.capability_root_sha256 != capability_root_sha256
        {
            return Err("r8b_v8_manager_post_capability_changed");
        }
        Ok(())
    }

    fn transition_roots(&self) -> [String; 2] {
        [
            self.manager.identity_root_sha256.clone(),
            self.capability_root_sha256.clone(),
        ]
    }
}

impl P02PreSnapshotFrozenV3 {
    fn bind_manager(
        self,
        journal: &ParentJournalV3,
        manager_pre: ManagerPreProbeCapabilityV3,
    ) -> ParentResultV3<P03AManagerBoundV3> {
        manager_pre.revalidate()?;
        let transition = advance_v3(
            journal,
            &self.transition,
            "p03a-manager-bound",
            vec![
                binding_v3(
                    "manager_identity_pre",
                    manager_pre.manager.identity_root_sha256.clone(),
                ),
                binding_v3(
                    "manager_live_image_pre",
                    manager_pre.capability_root_sha256.clone(),
                ),
            ],
        )?;
        Ok(P03AManagerBoundV3 {
            transition,
            snapshot: self.snapshot,
            manager_pre,
        })
    }
}

#[derive(serde::Serialize)]
struct DelegatedLaunchIdentityV3 {
    schema: String,
    route_id_sha256: String,
    normalized_argv: Vec<String>,
    systemd_run_sha256: String,
    child_executable_sha256: String,
    credential_request_root_sha256: String,
    credential_content_sha256: String,
    credential_byte_len: u64,
    credential_device: u64,
    credential_inode: u64,
}

struct DelegatedLaunchCapabilityV3 {
    contract: DelegatedLaunchContractV3,
    credential: File,
    credential_request_root_sha256: String,
    credential_content_sha256: String,
    credential_byte_len: u64,
    credential_device: u64,
    credential_inode: u64,
    capability_root_sha256: String,
}

impl DelegatedLaunchCapabilityV3 {
    #[allow(clippy::too_many_arguments)]
    fn bind(
        route_id_sha256: String,
        child: &BinaryV2,
        credential_path: &Path,
        credential_request_root_sha256: String,
        expected_credential_bytes: &[u8],
        stdout_path: PathBuf,
        stderr_path: PathBuf,
    ) -> ParentResultV3<Self> {
        require_composition_root_v1(&route_id_sha256)
            .map_err(|_| "r8b_v8_delegated_route_invalid")?;
        require_composition_root_v1(&credential_request_root_sha256)
            .map_err(|_| "r8b_v8_delegated_credential_root_invalid")?;
        validate_adapter_binary_v3(child, "M24_LINKED_RUNNER")?;
        let credential = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW)
            .open(credential_path)
            .map_err(|_| "r8b_v8_delegated_credential_open_failed")?;
        let credential_metadata = credential
            .metadata()
            .map_err(|_| "r8b_v8_delegated_credential_stat_failed")?;
        let credential_bytes =
            fs::read(credential_path).map_err(|_| "r8b_v8_delegated_credential_read_failed")?;
        let credential_parent = credential_path
            .parent()
            .ok_or("r8b_v8_delegated_credential_parent_missing")?;
        let parent_metadata = fs::symlink_metadata(credential_parent)
            .map_err(|_| "r8b_v8_delegated_credential_parent_missing")?;
        if credential_bytes != expected_credential_bytes
            || credential_bytes.is_empty()
            || credential_bytes.len() > 1_048_576
            || !credential_metadata.is_file()
            || credential_metadata.nlink() != 1
            || credential_metadata.permissions().mode() & 0o7777 != 0o400
            || parent_metadata.file_type().is_symlink()
            || !parent_metadata.is_dir()
            || parent_metadata.permissions().mode() & 0o7777 != 0o500
            || fs::canonicalize(credential_path)
                .map_err(|_| "r8b_v8_delegated_credential_path_invalid")?
                != credential_path
        {
            return Err("r8b_v8_delegated_credential_invalid");
        }
        validate_fresh_output_path_v3(&stdout_path)?;
        validate_fresh_output_path_v3(&stderr_path)?;
        let systemd_run = PathBuf::from("/usr/bin/systemd-run");
        let systemd_run_sha256 = composition_sha256_file_v1(&systemd_run)
            .map_err(|_| "r8b_v8_systemd_run_hash_failed")?;
        let mut contract = DelegatedLaunchContractV3 {
            route_id_sha256: route_id_sha256.clone(),
            unit: route_unit_v3(&route_id_sha256),
            request_owner_role: "M24_LINKED_RUNNER".to_owned(),
            child_owner: DelegatedChildOwnerV3::UserSystemdManager,
            systemd_run_sha256: systemd_run_sha256.clone(),
            child_executable: child.path.clone(),
            child_executable_sha256: child.sha256.clone(),
            credential_path: credential_path.to_path_buf(),
            stdout_path,
            stderr_path,
            selector: CHILD_SELECTOR_V2.to_owned(),
            normalized_argv: Vec::new(),
        };
        contract.normalized_argv = delegated_launch_argv_v3(&contract);
        validate_delegated_launch_v3(&contract)?;
        let credential_content_sha256 = composition_sha256_bytes_v1(&credential_bytes);
        let identity = DelegatedLaunchIdentityV3 {
            schema: "nando.k2-self-formed-r8b-delegated-launch-capability.v3".to_owned(),
            route_id_sha256,
            normalized_argv: contract.normalized_argv.clone(),
            systemd_run_sha256,
            child_executable_sha256: child.sha256.clone(),
            credential_request_root_sha256: credential_request_root_sha256.clone(),
            credential_content_sha256: credential_content_sha256.clone(),
            credential_byte_len: credential_metadata.len(),
            credential_device: credential_metadata.dev(),
            credential_inode: credential_metadata.ino(),
        };
        let value = Self {
            contract,
            credential,
            credential_request_root_sha256,
            credential_content_sha256,
            credential_byte_len: credential_metadata.len(),
            credential_device: credential_metadata.dev(),
            credential_inode: credential_metadata.ino(),
            capability_root_sha256: composition_root_v1(&identity)
                .map_err(|_| "r8b_v8_delegated_capability_root_failed")?,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        validate_delegated_launch_v3(&self.contract)?;
        let metadata = self
            .credential
            .metadata()
            .map_err(|_| "r8b_v8_delegated_credential_descriptor_lost")?;
        let path_metadata = fs::symlink_metadata(&self.contract.credential_path)
            .map_err(|_| "r8b_v8_delegated_credential_path_lost")?;
        if metadata.dev() != self.credential_device
            || metadata.ino() != self.credential_inode
            || metadata.len() != self.credential_byte_len
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || path_metadata.file_type().is_symlink()
            || path_metadata.dev() != self.credential_device
            || path_metadata.ino() != self.credential_inode
            || composition_sha256_bytes_v1(
                &fs::read(&self.contract.credential_path)
                    .map_err(|_| "r8b_v8_delegated_credential_read_failed")?,
            ) != self.credential_content_sha256
            || composition_sha256_file_v1(Path::new("/usr/bin/systemd-run"))
                .map_err(|_| "r8b_v8_systemd_run_hash_failed")?
                != self.contract.systemd_run_sha256
            || composition_sha256_file_v1(&self.contract.child_executable)
                .map_err(|_| "r8b_v8_delegated_child_hash_failed")?
                != self.contract.child_executable_sha256
        {
            return Err("r8b_v8_delegated_capability_changed");
        }
        validate_fresh_output_path_v3(&self.contract.stdout_path)?;
        validate_fresh_output_path_v3(&self.contract.stderr_path)
    }
}

fn validate_fresh_output_path_v3(path: &Path) -> ParentResultV3<()> {
    let parent = path
        .parent()
        .ok_or("r8b_v8_delegated_output_parent_missing")?;
    let metadata =
        fs::symlink_metadata(parent).map_err(|_| "r8b_v8_delegated_output_parent_missing")?;
    let path_absent = matches!(
        fs::symlink_metadata(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    );
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().len() > 240
        || fs::canonicalize(parent).map_err(|_| "r8b_v8_delegated_output_parent_invalid")? != parent
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.permissions().mode() & 0o7777 != 0o700
        || !path_absent
    {
        return Err("r8b_v8_delegated_output_path_invalid");
    }
    Ok(())
}

impl P03AManagerBoundV3 {
    fn freeze_delegated_request(
        self,
        journal: &ParentJournalV3,
        launch: DelegatedLaunchCapabilityV3,
    ) -> ParentResultV3<P03BDelegatedRequestFrozenV3> {
        self.snapshot
            .revalidate_for_route(&journal.route_id_sha256)?;
        self.manager_pre.revalidate()?;
        launch.revalidate()?;
        if launch.contract.route_id_sha256 != self.transition.route_id_sha256 {
            return Err("r8b_v8_delegated_launch_route_mismatch");
        }
        let transition = advance_v3(
            journal,
            &self.transition,
            "p03b-delegated-request-frozen",
            vec![
                binding_v3(
                    "delegated_launch_request",
                    launch.capability_root_sha256.clone(),
                ),
                binding_v3(
                    "producer_request",
                    launch.credential_request_root_sha256.clone(),
                ),
            ],
        )?;
        Ok(P03BDelegatedRequestFrozenV3 {
            transition,
            snapshot: self.snapshot,
            launch,
            manager_pre: self.manager_pre,
        })
    }
}

impl P03BDelegatedRequestFrozenV3 {
    fn freeze_resources(
        self,
        journal: &ParentJournalV3,
        resources: LoadedUnitResourceCapabilityV3,
    ) -> ParentResultV3<P04AResourcesFrozenV3> {
        self.manager_pre.revalidate()?;
        resources.revalidate_for_launch(&self.launch)?;
        let producer_executable_sha256 = self.launch.contract.child_executable_sha256.clone();
        let [submission, child_terminal, loaded_metrics] = resources.transition_roots();
        let transition = advance_v3(
            journal,
            &self.transition,
            "p04a-resources-frozen",
            vec![
                binding_v3("systemd_submission", submission),
                binding_v3("child_terminal", child_terminal),
                binding_v3("loaded_unit_metrics", loaded_metrics),
            ],
        )?;
        Ok(P04AResourcesFrozenV3 {
            transition,
            snapshot: self.snapshot,
            manager_pre: self.manager_pre,
            resources,
            producer_executable_sha256,
        })
    }
}

impl P04AResourcesFrozenV3 {
    fn reverify_manager(
        self,
        journal: &ParentJournalV3,
        stopped: StoppedUnitResourceCapabilityV3,
        manager_post: ManagerPostProbeCapabilityV3,
    ) -> ParentResultV3<P04BManagerReverifiedV3> {
        self.manager_pre.revalidate()?;
        self.resources.revalidate()?;
        stopped.revalidate_for_loaded(&self.resources)?;
        manager_post.revalidate_for_pre(&self.manager_pre, stopped.finished_monotonic_ns())?;
        let [stop, inactive, residue] = stopped.transition_roots();
        let [manager_identity_post, manager_live_image_post] = manager_post.transition_roots();
        Ok(P04BManagerReverifiedV3 {
            transition: advance_v3(
                journal,
                &self.transition,
                "p04b-manager-reverified",
                vec![
                    binding_v3("unit_stop", stop),
                    binding_v3("unit_inactive", inactive),
                    binding_v3("cgroup_empty", residue),
                    binding_v3("manager_identity_post", manager_identity_post),
                    binding_v3("manager_live_image_post", manager_live_image_post),
                ],
            )?,
            snapshot: self.snapshot,
            manager_pre: self.manager_pre,
            resources: self.resources,
            stopped,
            manager_post,
            producer_executable_sha256: self.producer_executable_sha256,
        })
    }
}

impl P04BManagerReverifiedV3 {
    fn prove_production_survival(
        self,
        journal: &ParentJournalV3,
        health: P05HealthObservationV3,
    ) -> ParentResultV3<P05ProductionSurvivedV3> {
        self.snapshot
            .revalidate_for_route(&journal.route_id_sha256)?;
        self.manager_pre.revalidate()?;
        self.resources.revalidate()?;
        self.stopped.revalidate_for_loaded(&self.resources)?;
        self.manager_post
            .revalidate_for_pre(&self.manager_pre, self.stopped.finished_monotonic_ns())?;
        let survival = K2UncertaintyR8BMeasuredReceiptV2::seal(
            K2UncertaintyR8BEvidenceKindV2::ProductionSurvival,
            journal.route_id_sha256.clone(),
            vec![
                self.snapshot.transition_root(),
                self.transition.receipt_root_sha256.clone(),
            ],
            1,
            BTreeMap::from([
                (
                    "bounded_parent_health_get_count".to_owned(),
                    health.successful_get_count,
                ),
                (
                    "maximum_health_latency_ns".to_owned(),
                    health.maximum_latency_ns,
                ),
                ("stable_projection_equal".to_owned(), 1),
            ]),
            self.producer_executable_sha256.clone(),
        )
        .map_err(|_| "r8b_v8_p05_survival_receipt_invalid")?;
        let survival_root_sha256 = survival.receipt_root_sha256.clone();
        let transition = advance_v3(
            journal,
            &self.transition,
            "p05-production-survived",
            vec![binding_v3("production_survival", survival_root_sha256)],
        )?;
        Ok(P05ProductionSurvivedV3 {
            transition,
            prior: self,
            survival,
        })
    }
}

impl P05ProductionSurvivedV3 {
    fn revalidate(&self, journal: &ParentJournalV3) -> ParentResultV3<()> {
        let prior = &self.prior;
        prior
            .snapshot
            .revalidate_for_route(&journal.route_id_sha256)?;
        prior.manager_pre.revalidate()?;
        prior.resources.revalidate()?;
        prior.stopped.revalidate_for_loaded(&prior.resources)?;
        prior
            .manager_post
            .revalidate_for_pre(&prior.manager_pre, prior.stopped.finished_monotonic_ns())?;
        self.transition.validate()?;
        self.survival
            .validate()
            .map_err(|_| "r8b_v8_p05_survival_receipt_invalid")?;
        if self.transition.route_id_sha256 != journal.route_id_sha256
            || self.transition.previous_transition_root_sha256
                != prior.transition.receipt_root_sha256
            || self.survival.kind != K2UncertaintyR8BEvidenceKindV2::ProductionSurvival
            || self.survival.route_id_sha256 != journal.route_id_sha256
            || self.survival.source_roots_sha256
                != vec![
                    prior.snapshot.transition_root(),
                    prior.transition.receipt_root_sha256.clone(),
                ]
            || self.survival.producer_executable_sha256 != prior.producer_executable_sha256
            || self.transition.bindings
                != vec![binding_v3(
                    "production_survival",
                    self.survival.receipt_root_sha256.clone(),
                )]
        {
            return Err("r8b_v8_p05_survival_capability_changed");
        }
        Ok(())
    }
}

fn validate_adapter_binary_v3(binary: &BinaryV2, role: &str) -> ParentResultV3<()> {
    if binary.role != role
        || !binary.path.is_absolute()
        || fs::canonicalize(&binary.path).map_err(|_| "r8b_v8_parent_adapter_binary_missing")?
            != binary.path
        || composition_sha256_file_v1(&binary.path)
            .map_err(|_| "r8b_v8_parent_adapter_hash_failed")?
            != binary.sha256
    {
        return Err("r8b_v8_parent_adapter_identity_invalid");
    }
    Ok(())
}

struct M25ProcessAdapterV3 {
    binary: BinaryV2,
}

impl M25ProcessAdapterV3 {
    fn new(binary: &BinaryV2) -> ParentResultV3<Self> {
        validate_adapter_binary_v3(binary, "M25_R8B_AUTHORIZER")?;
        Ok(Self {
            binary: binary.clone(),
        })
    }

    fn run(
        &self,
        packet: &FrozenPacketDirectoryV3,
        request: &K2UncertaintyR8BAuthorizationRequestV3,
    ) -> ParentResultV3<K2UncertaintyR8BAuthorizationReceiptV3> {
        request
            .validate()
            .map_err(|_| "r8b_v8_m25_request_invalid")?;
        if request.authorizer_executable_sha256 != self.binary.sha256 {
            return Err("r8b_v8_m25_request_identity_mismatch");
        }
        packet.revalidate()?;
        let inherited_cwd = packet.inherited_cwd();
        let output =
            try_run_parent_process_v3(&self.binary.path, Some(&inherited_cwd), request, 1_200)?;
        packet.revalidate()?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err("r8b_v8_m25_authorization_rejected");
        }
        self.accept_output(request, &output.stdout)
    }

    fn accept_output(
        &self,
        request: &K2UncertaintyR8BAuthorizationRequestV3,
        stdout: &[u8],
    ) -> ParentResultV3<K2UncertaintyR8BAuthorizationReceiptV3> {
        request
            .validate()
            .map_err(|_| "r8b_v8_m25_request_invalid")?;
        let receipt: K2UncertaintyR8BAuthorizationReceiptV3 =
            uncertainty_decode_v1(stdout).map_err(|_| "r8b_v8_m25_receipt_decode_failed")?;
        receipt
            .validate()
            .map_err(|_| "r8b_v8_m25_receipt_invalid")?;
        if request.authorizer_executable_sha256 != self.binary.sha256
            || uncertainty_bytes_v1(&receipt).map_err(|_| "r8b_v8_m25_receipt_encode_failed")?
                != stdout
            || receipt.request_root_sha256 != request.request_root_sha256
            || receipt.route_id_sha256 != request.route_id_sha256
            || receipt.manifest_root_sha256 != request.manifest_root_sha256
        {
            return Err("r8b_v8_m25_receipt_binding_invalid");
        }
        Ok(receipt)
    }
}

struct M26ProcessAdapterV3 {
    binary: BinaryV2,
}

impl M26ProcessAdapterV3 {
    fn new(binary: &BinaryV2) -> ParentResultV3<Self> {
        validate_adapter_binary_v3(binary, "M26_R8B_PUBLISHER")?;
        Ok(Self {
            binary: binary.clone(),
        })
    }

    fn run(
        &self,
        request: &K2UncertaintyR8BPublicationRequestV3,
    ) -> ParentResultV3<K2UncertaintyR8BPublicationReceiptV3> {
        request
            .validate()
            .map_err(|_| "r8b_v8_m26_request_invalid")?;
        if request.publisher_executable_sha256 != self.binary.sha256 {
            return Err("r8b_v8_m26_request_identity_mismatch");
        }
        let output = try_run_parent_process_v3(&self.binary.path, None, request, 60)?;
        if !output.status.success() || !output.stderr.is_empty() {
            return Err("r8b_v8_m26_publication_rejected");
        }
        self.accept_output(request, &output.stdout)
    }

    fn accept_output(
        &self,
        request: &K2UncertaintyR8BPublicationRequestV3,
        stdout: &[u8],
    ) -> ParentResultV3<K2UncertaintyR8BPublicationReceiptV3> {
        request
            .validate()
            .map_err(|_| "r8b_v8_m26_request_invalid")?;
        let receipt: K2UncertaintyR8BPublicationReceiptV3 =
            uncertainty_decode_v1(stdout).map_err(|_| "r8b_v8_m26_receipt_decode_failed")?;
        receipt
            .validate()
            .map_err(|_| "r8b_v8_m26_receipt_invalid")?;
        let authorization_bytes = uncertainty_bytes_v1(&request.authorization)
            .map_err(|_| "r8b_v8_m26_authorization_encode_failed")?;
        let published_path = Path::new(&request.publication_root).join(&receipt.relative_path);
        let metadata =
            fs::symlink_metadata(&published_path).map_err(|_| "r8b_v8_m26_publication_missing")?;
        if request.publisher_executable_sha256 != self.binary.sha256
            || uncertainty_bytes_v1(&receipt).map_err(|_| "r8b_v8_m26_receipt_encode_failed")?
                != stdout
            || receipt.request_root_sha256 != request.request_root_sha256
            || receipt.authorization_receipt_root_sha256
                != request.authorization.receipt_root_sha256
            || receipt.publisher_executable_sha256 != self.binary.sha256
            || receipt.byte_len != authorization_bytes.len() as u64
            || receipt.content_sha256 != composition_sha256_bytes_v1(&authorization_bytes)
            || metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o7777 != 0o400
            || fs::read(published_path).map_err(|_| "r8b_v8_m26_publication_read_failed")?
                != authorization_bytes
        {
            return Err("r8b_v8_m26_publication_binding_invalid");
        }
        Ok(receipt)
    }
}

struct P06FrozenPacketV3 {
    transition: ParentTransitionReceiptV3,
    packet: FrozenPacketDirectoryV3,
    packet_root_sha256: String,
    manifest_root_sha256: String,
    ledger_seal_root_sha256: String,
    production_survival: K2UncertaintyR8BMeasuredReceiptV2,
}

impl P05ProductionSurvivedV3 {
    fn freeze_packet(
        self,
        journal: &ParentJournalV3,
        packet_path: &Path,
    ) -> ParentResultV3<P06FrozenPacketV3> {
        self.revalidate(journal)?;
        let packet = FrozenPacketDirectoryV3::capture(
            packet_path,
            &self.transition.route_id_sha256,
            &self.survival,
        )?;
        let packet_root_sha256 = packet.custody_root_sha256.clone();
        let manifest_root_sha256 = packet.manifest_root_sha256.clone();
        let ledger_seal_root_sha256 = packet.ledger_seal_root_sha256.clone();
        let transition = advance_v3(
            journal,
            &self.transition,
            "p06-packet-frozen",
            vec![
                binding_v3("packet", packet_root_sha256.clone()),
                binding_v3("packet_manifest", manifest_root_sha256.clone()),
                binding_v3("ledger_seal", ledger_seal_root_sha256.clone()),
            ],
        )?;
        Ok(P06FrozenPacketV3 {
            transition,
            packet,
            packet_root_sha256,
            manifest_root_sha256,
            ledger_seal_root_sha256,
            production_survival: self.survival,
        })
    }
}

struct P07AuthorizedV3 {
    transition: ParentTransitionReceiptV3,
    packet_root_sha256: String,
    manifest_root_sha256: String,
    ledger_seal_root_sha256: String,
    authorization: K2UncertaintyR8BAuthorizationReceiptV3,
}

impl P06FrozenPacketV3 {
    fn authorize(
        self,
        journal: &ParentJournalV3,
        adapter: &M25ProcessAdapterV3,
    ) -> ParentResultV3<P07AuthorizedV3> {
        let request = K2UncertaintyR8BAuthorizationRequestV3::seal(
            self.transition.route_id_sha256.clone(),
            self.manifest_root_sha256.clone(),
            adapter.binary.sha256.clone(),
        )
        .map_err(|_| "r8b_v8_m25_request_seal_failed")?;
        let authorization = adapter.run(&self.packet, &request)?;
        if authorization.ledger_seal_root_sha256 != self.ledger_seal_root_sha256 {
            return Err("r8b_v8_m25_ledger_binding_invalid");
        }
        let authorization_root_sha256 = authorization.receipt_root_sha256.clone();
        let transition = advance_v3(
            journal,
            &self.transition,
            "p07-authorized",
            vec![
                binding_v3("packet", self.packet_root_sha256.clone()),
                binding_v3("authorization_request", request.request_root_sha256),
                binding_v3("authorization", authorization_root_sha256),
            ],
        )?;
        Ok(P07AuthorizedV3 {
            transition,
            packet_root_sha256: self.packet_root_sha256,
            manifest_root_sha256: self.manifest_root_sha256,
            ledger_seal_root_sha256: self.ledger_seal_root_sha256,
            authorization,
        })
    }
}

struct P08PublishedV3 {
    transition: ParentTransitionReceiptV3,
    packet_root_sha256: String,
    manifest_root_sha256: String,
    ledger_seal_root_sha256: String,
    authorization_root_sha256: String,
    publication_root_sha256: String,
    authorization: K2UncertaintyR8BAuthorizationReceiptV3,
    publication: K2UncertaintyR8BPublicationReceiptV3,
    publication_root_path: PathBuf,
}

impl P07AuthorizedV3 {
    fn publish(
        self,
        journal: &ParentJournalV3,
        adapter: &M26ProcessAdapterV3,
        publication_root: &Path,
    ) -> ParentResultV3<P08PublishedV3> {
        if fs::canonicalize(publication_root).map_err(|_| "r8b_v8_publication_root_missing")?
            != publication_root
        {
            return Err("r8b_v8_publication_root_invalid");
        }
        let request = K2UncertaintyR8BPublicationRequestV3::seal(
            publication_root.to_string_lossy().into_owned(),
            self.authorization,
        )
        .map_err(|_| "r8b_v8_m26_request_seal_failed")?;
        let publication = adapter.run(&request)?;
        let authorization_root_sha256 = request.authorization.receipt_root_sha256.clone();
        let publication_root_sha256 = publication.receipt_root_sha256.clone();
        let transition = advance_v3(
            journal,
            &self.transition,
            "p08-published",
            vec![
                binding_v3("packet", self.packet_root_sha256.clone()),
                binding_v3("authorization", authorization_root_sha256.clone()),
                binding_v3("publication_request", request.request_root_sha256),
                binding_v3("publication", publication_root_sha256.clone()),
            ],
        )?;
        Ok(P08PublishedV3 {
            transition,
            packet_root_sha256: self.packet_root_sha256,
            manifest_root_sha256: self.manifest_root_sha256,
            ledger_seal_root_sha256: self.ledger_seal_root_sha256,
            authorization_root_sha256,
            publication_root_sha256,
            authorization: request.authorization,
            publication,
            publication_root_path: publication_root.to_path_buf(),
        })
    }
}

fn route_to_p06_v3(
    journal: &ParentJournalV3,
    packet_path: &Path,
    manager_pre: ManagerPreProbeCapabilityV3,
    launch: DelegatedLaunchCapabilityV3,
) -> ParentResultV3<P06FrozenPacketV3> {
    let source_label = composition_sha256_bytes_v1(packet_path.as_os_str().as_encoded_bytes());
    let source = p00_source_fixture_v3(
        packet_path
            .parent()
            .ok_or("r8b_v8_p00_fixture_parent_missing")?,
        &source_label,
        &journal.route_id_sha256,
    );
    let channels = p01_channels_fixture_v3(
        packet_path
            .parent()
            .ok_or("r8b_v8_p01_fixture_parent_missing")?,
        &source_label,
        &journal.route_id_sha256,
    );
    let snapshot = p02_snapshot_fixture_v3(
        packet_path
            .parent()
            .ok_or("r8b_v8_p02_fixture_parent_missing")?,
        &source_label,
        &journal.route_id_sha256,
    );
    let resources = loaded_unit_resource_fixture_v3(&launch);
    let stopped = stopped_unit_resource_fixture_v3(&resources);
    let manager_post = manager_post_probe_fixture_v3(&manager_pre, stopped.finished_monotonic_ns());
    let p05 = P00SourceValidatedV3::start(journal, source)?
        .close_producers(journal, channels)?
        .freeze_pre_snapshot(journal, snapshot)?
        .bind_manager(journal, manager_pre)?
        .freeze_delegated_request(journal, launch)?
        .freeze_resources(journal, resources)?
        .reverify_manager(journal, stopped, manager_post)?
        .prove_production_survival(journal, P05HealthObservationV3::new(1, 1)?)?;
    postproduction::close_packet_fixture_v3(packet_path, &journal.route_id_sha256, &p05.survival)?;
    p05.freeze_packet(journal, packet_path)
}

fn packet_staging_fixture_v3(environment: &TestEnvironmentV1, relative: &str) -> PathBuf {
    environment.private_child(relative)
}

fn manager_identity_fixture_v3() -> (ManagerIdentityObservationV3, OwnedFd) {
    let process = rustix::process::getpid();
    let pid = u32::try_from(process.as_raw_pid()).expect("R8B V8 parent fixture");
    let uid = rustix::process::getuid().as_raw();
    let control_group = format!("/user.slice/user-{uid}.slice/user@{uid}.service");
    let observation = ManagerIdentityObservationV3 {
        bus_peer_pid: pid,
        bus_unique_name: format!(":1.{pid}"),
        bus_owner_uid: uid,
        proc_pid: pid,
        proc_start_ticks_before: 1,
        proc_start_ticks_after: 1,
        proc_uid: uid,
        command: vec![SYSTEMD_MANAGER_V3.to_owned(), "--user".to_owned()],
        cgroup: format!("{control_group}/init.scope"),
        boot_id_before: "00000000-0000-4000-8000-000000000001".to_owned(),
        boot_id_after: "00000000-0000-4000-8000-000000000001".to_owned(),
        unit: ManagerUnitIdentityObservationV3 {
            owner_uid: uid,
            user_unit: format!("user@{uid}.service"),
            invocation_id: "manager-invocation".to_owned(),
            main_pid: pid,
            exec_start: SYSTEMD_MANAGER_V3.to_owned(),
            fragment_path: "/usr/lib/systemd/system/user@.service".to_owned(),
            control_group,
        },
        version: "systemd 259 (259.5-0ubuntu3.4)".to_owned(),
    };
    let pidfd = rustix::process::pidfd_open(process, rustix::process::PidfdFlags::empty())
        .expect("R8B V8 parent fixture");
    (observation, pidfd)
}

fn manager_identity_capability_fixture_v3() -> ManagerIdentityCapabilityV3 {
    let (observation, pidfd) = manager_identity_fixture_v3();
    ManagerIdentityCapabilityV3::bind(observation, pidfd).expect("R8B V8 parent fixture")
}

fn manager_probe_tools_fixture_v3() -> ManagerProbeToolIdentityV3 {
    ManagerProbeToolIdentityV3::bind(
        BoundManagerToolV3 {
            path: SYSTEMD_MANAGER_V3.to_owned(),
            unix_mode: 0o755,
            byte_len: 141_776,
            sha256: PINNED_SYSTEMD_SHA256_V3.to_owned(),
        },
        BoundManagerToolV3 {
            path: SUDO_V3.to_owned(),
            unix_mode: 0o4755,
            byte_len: 1_082_656,
            sha256: PINNED_SUDO_SHA256_V3.to_owned(),
        },
        BoundManagerToolV3 {
            path: SHA256SUM_V3.to_owned(),
            unix_mode: 0o755,
            byte_len: 11_352_352,
            sha256: PINNED_SHA256SUM_SHA256_V3.to_owned(),
        },
    )
    .expect("R8B V8 parent fixture")
}

fn privileged_probe_execution_fixture_v3(
    request: &PrivilegedProbeRequestV3,
    live_image_sha256: &str,
) -> PrivilegedProbeExecutionV3 {
    let mut stdout = format!("{live_image_sha256} */proc/{}/exe", request.manager_pid).into_bytes();
    stdout.push(0);
    PrivilegedProbeExecutionV3 {
        argv: request.argv.clone(),
        exit_code: 0,
        stdout,
        stderr: Vec::new(),
        started_monotonic_ns: 1,
        finished_monotonic_ns: 2,
    }
}

fn manager_pre_probe_fixture_v3() -> ManagerPreProbeCapabilityV3 {
    let manager = manager_identity_capability_fixture_v3();
    let tools = manager_probe_tools_fixture_v3();
    let request = PrivilegedProbeRequestV3::new(&manager, &tools).expect("R8B V8 parent fixture");
    let execution = privileged_probe_execution_fixture_v3(&request, &tools.systemd.sha256);
    ManagerPreProbeCapabilityV3::bind(manager, tools, execution).expect("R8B V8 parent fixture")
}

fn manager_post_probe_fixture_v3(
    pre: &ManagerPreProbeCapabilityV3,
    after_monotonic_ns: u64,
) -> ManagerPostProbeCapabilityV3 {
    let (observation, pidfd) = manager_identity_fixture_v3();
    let manager =
        ManagerIdentityCapabilityV3::bind(observation, pidfd).expect("R8B V8 parent fixture");
    let request =
        PrivilegedProbeRequestV3::new(&manager, &pre.tools).expect("R8B V8 parent fixture");
    let mut execution = privileged_probe_execution_fixture_v3(&request, &pre.tools.systemd.sha256);
    execution.started_monotonic_ns = after_monotonic_ns + 10;
    execution.finished_monotonic_ns = after_monotonic_ns + 11;
    ManagerPostProbeCapabilityV3::bind(pre, manager, execution, after_monotonic_ns)
        .expect("R8B V8 parent fixture")
}

fn delegated_launch_fixture_v3(
    environment: &TestEnvironmentV1,
    label: &str,
    route_id_sha256: &str,
) -> DelegatedLaunchCapabilityV3 {
    let credential_directory = environment.private_child(&format!("{label}-credential"));
    let credential_path = credential_directory.join("producer-request.json");
    let credential_bytes = uncertainty_bytes_v1(&(
        "nando.k2-self-formed-r8b-delegated-credential-fixture.v3",
        route_id_sha256,
    ))
    .expect("R8B V8 parent fixture");
    write_new_read_only_v2(&credential_path, &credential_bytes);
    freeze_directory_tree_v2(&credential_directory);
    let output_directory = environment.private_child(&format!("{label}-output"));
    let child_path = fs::canonicalize(std::env::current_exe().expect("R8B V8 parent fixture"))
        .expect("R8B V8 parent fixture");
    let child = BinaryV2 {
        role: "M24_LINKED_RUNNER",
        sha256: composition_sha256_file_v1(&child_path).expect("R8B V8 parent fixture"),
        path: child_path,
    };
    DelegatedLaunchCapabilityV3::bind(
        route_id_sha256.to_owned(),
        &child,
        &credential_path,
        root_v1(&format!("{label}-producer-request")),
        &credential_bytes,
        output_directory.join("stdout.log"),
        output_directory.join("stderr.log"),
    )
    .expect("R8B V8 parent fixture")
}

#[test]
fn r8b_v8_parent_manager_capability_rejects_pid_and_start_drift() {
    let (mut observation, pidfd) = manager_identity_fixture_v3();
    observation.proc_pid = observation
        .proc_pid
        .checked_add(1)
        .expect("R8B V8 parent fixture");
    assert!(ManagerIdentityCapabilityV3::bind(observation, pidfd).is_err());

    let (mut observation, pidfd) = manager_identity_fixture_v3();
    observation.proc_start_ticks_after += 1;
    assert!(ManagerIdentityCapabilityV3::bind(observation, pidfd).is_err());

    let (mut observation, pidfd) = manager_identity_fixture_v3();
    let foreign_pid = observation
        .bus_peer_pid
        .checked_add(1)
        .expect("R8B V8 parent fixture");
    observation.bus_peer_pid = foreign_pid;
    observation.proc_pid = foreign_pid;
    observation.unit.main_pid = foreign_pid;
    assert!(ManagerIdentityCapabilityV3::bind(observation, pidfd).is_err());
}

#[test]
fn r8b_v8_parent_privileged_probe_rejects_argv_output_and_hash_drift() {
    let manager = manager_identity_capability_fixture_v3();
    let tools = manager_probe_tools_fixture_v3();
    let request = PrivilegedProbeRequestV3::new(&manager, &tools).expect("R8B V8 parent fixture");

    let mut execution = privileged_probe_execution_fixture_v3(&request, &tools.systemd.sha256);
    execution.argv.push("--help".to_owned());
    assert!(request.complete(&tools, execution).is_err());

    let mut execution = privileged_probe_execution_fixture_v3(&request, &tools.systemd.sha256);
    execution.stdout.push(b'x');
    assert!(request.complete(&tools, execution).is_err());

    let execution = privileged_probe_execution_fixture_v3(&request, &root_v1("wrong-image"));
    assert!(request.complete(&tools, execution).is_err());
}

#[test]
fn r8b_v8_parent_p00_p06_binds_durable_directory_custody_and_exact_adapters() {
    let environment = TestEnvironmentV1::new("typed-parent-route");
    let route = root_v1("typed-parent-route");
    let journal_root = environment.private_child("parent-journal");
    let journal =
        ParentJournalV3::new(&journal_root, route.clone()).expect("R8B V8 parent fixture");
    let packet_path = packet_staging_fixture_v3(&environment, "packet");
    let manager_pre = manager_pre_probe_fixture_v3();
    manager_pre.revalidate().expect("R8B V8 parent fixture");
    let manager_identity_root = manager_pre.manager.identity_root_sha256.clone();
    let manager_pre_probe_root = manager_pre.capability_root_sha256.clone();
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);
    launch.revalidate().expect("R8B V8 parent fixture");
    assert_eq!(launch.contract.selector, CHILD_SELECTOR_V2);
    assert_eq!(
        launch.contract.normalized_argv,
        delegated_launch_argv_v3(&launch.contract)
    );
    assert_eq!(launch.contract.normalized_argv.len(), 22);
    assert!(!launch.contract.stdout_path.exists());
    assert!(!launch.contract.stderr_path.exists());
    let mut wrong_selector = launch.contract.clone();
    wrong_selector.selector = "r8b_v7_m24_linked_child".to_owned();
    wrong_selector.normalized_argv = delegated_launch_argv_v3(&wrong_selector);
    assert!(validate_delegated_launch_v3(&wrong_selector).is_err());
    let p06 = route_to_p06_v3(&journal, &packet_path, manager_pre, launch)
        .expect("R8B V8 parent fixture");
    p06.packet.revalidate().expect("R8B V8 parent fixture");
    p06.production_survival
        .validate()
        .expect("R8B V8 parent fixture");
    assert_eq!(
        p06.production_survival.kind,
        K2UncertaintyR8BEvidenceKindV2::ProductionSurvival
    );
    require_composition_root_v1(&p06.packet_root_sha256).expect("R8B V8 parent fixture");
    assert_eq!(
        fs::metadata(p06.packet.inherited_cwd())
            .expect("R8B V8 parent fixture")
            .dev(),
        p06.packet.device
    );
    assert_eq!(
        fs::metadata(p06.packet.inherited_cwd())
            .expect("R8B V8 parent fixture")
            .ino(),
        p06.packet.inode
    );

    let binaries = LinkedBinariesV2::from_cargo();
    let m25 = M25ProcessAdapterV3::new(binaries.get("M25_R8B_AUTHORIZER"))
        .expect("R8B V8 parent fixture");
    let m26 =
        M26ProcessAdapterV3::new(binaries.get("M26_R8B_PUBLISHER")).expect("R8B V8 parent fixture");
    assert!(M25ProcessAdapterV3::new(&m26.binary).is_err());
    assert!(M26ProcessAdapterV3::new(&m25.binary).is_err());
    let authorize: fn(
        P06FrozenPacketV3,
        &ParentJournalV3,
        &M25ProcessAdapterV3,
    ) -> ParentResultV3<P07AuthorizedV3> = P06FrozenPacketV3::authorize;
    let publish: fn(
        P07AuthorizedV3,
        &ParentJournalV3,
        &M26ProcessAdapterV3,
        &Path,
    ) -> ParentResultV3<P08PublishedV3> = P07AuthorizedV3::publish;
    let _wired_process_route = (authorize, publish);
    let request = K2UncertaintyR8BAuthorizationRequestV3::seal(
        route,
        p06.manifest_root_sha256.clone(),
        m25.binary.sha256.clone(),
    )
    .expect("R8B V8 parent fixture");
    assert_eq!(request.manifest_root_sha256, p06.manifest_root_sha256);
    assert_eq!(request.authorizer_executable_sha256, m25.binary.sha256);
    let p03a: ParentTransitionReceiptV3 = uncertainty_decode_v1(
        &fs::read(journal_root.join("p03a-manager-bound.json")).expect("R8B V8 parent fixture"),
    )
    .expect("R8B V8 parent fixture");
    assert_eq!(
        p03a.bindings,
        vec![
            binding_v3("manager_identity_pre", manager_identity_root),
            binding_v3("manager_live_image_pre", manager_pre_probe_root),
        ]
    );
    assert_eq!(
        fs::read_dir(journal_root)
            .expect("R8B V8 parent fixture")
            .count(),
        9
    );
}

#[test]
fn r8b_v8_parent_rejects_empty_or_foreign_transition_inputs() {
    let environment = TestEnvironmentV1::new("typed-parent-negative");
    let journal_root = environment.private_child("parent-journal");
    let journal = ParentJournalV3::new(&journal_root, root_v1("typed-parent-negative"))
        .expect("R8B V8 parent fixture");
    let foreign_source =
        p00_source_fixture_v3(&environment.root, "p00-foreign", &root_v1("foreign-route"));
    assert!(P00SourceValidatedV3::start(&journal, foreign_source).is_err());
    assert_eq!(
        fs::read_dir(&journal_root)
            .expect("R8B V8 parent fixture")
            .count(),
        0
    );

    let packet_path = packet_staging_fixture_v3(&environment, "packet");
    let launch =
        delegated_launch_fixture_v3(&environment, "negative-launch", &journal.route_id_sha256);
    let manager_pre = manager_pre_probe_fixture_v3();
    let p06 = route_to_p06_v3(&journal, &packet_path, manager_pre, launch)
        .expect("R8B V8 parent fixture");
    let moved = environment.root.join("moved-packet");
    fs::rename(&packet_path, &moved).expect("R8B V8 parent fixture");
    let replacement = environment.private_child("packet");
    freeze_directory_tree_v2(&replacement);
    assert!(p06.packet.revalidate().is_err());
    assert_eq!(
        fs::read_dir(journal_root)
            .expect("R8B V8 parent fixture")
            .count(),
        9
    );
}

#[test]
fn r8b_v8_parent_p05_rejects_unmeasured_or_unbounded_health() {
    assert!(P05HealthObservationV3::new(0, 1).is_err());
    assert!(P05HealthObservationV3::new(1, 5_000_000_001).is_err());
}
