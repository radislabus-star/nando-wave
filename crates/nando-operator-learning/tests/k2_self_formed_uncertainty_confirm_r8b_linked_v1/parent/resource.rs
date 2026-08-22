use std::collections::BTreeMap;

use super::*;

const SYSTEMCTL_V3: &str = "/usr/bin/systemctl";
const PINNED_SYSTEMCTL_BYTE_LEN_V3: u64 = 302_112;
const PINNED_SYSTEMCTL_SHA256_V3: &str =
    "6394f5e8df92878184de9d4dfb7ac242471cb09daf89d23be4e81ae17e9c03b2";
const MAX_CAPTURE_BYTES_V3: usize = 1_048_576;
const MAX_ROUTE_NS_V3: u64 = 1_200_000_000_000;
const UNIT_PROPERTIES_V3: [&str; 11] = [
    "InvocationID",
    "MainPID",
    "ExecMainCode",
    "ExecMainStatus",
    "ActiveState",
    "SubState",
    "MemoryPeak",
    "MemorySwapPeak",
    "OOMPolicy",
    "OOMKills",
    "TasksCurrent",
];

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct SystemctlToolIdentityV3 {
    schema: String,
    path: String,
    unix_mode: u32,
    byte_len: u64,
    uid: u32,
    gid: u32,
    sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SystemctlToolCapabilityV3 {
    identity: SystemctlToolIdentityV3,
    identity_root_sha256: String,
}

impl SystemctlToolCapabilityV3 {
    fn bind(identity: SystemctlToolIdentityV3) -> ParentResultV3<Self> {
        validate_systemctl_identity_v3(&identity)?;
        let identity_root_sha256 =
            composition_root_v1(&identity).map_err(|_| "r8b_v8_systemctl_identity_root_failed")?;
        let value = Self {
            identity,
            identity_root_sha256,
        };
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        validate_systemctl_identity_v3(&self.identity)?;
        require_composition_root_v1(&self.identity_root_sha256)
            .map_err(|_| "r8b_v8_systemctl_identity_root_invalid")?;
        if self.identity_root_sha256
            != composition_root_v1(&self.identity)
                .map_err(|_| "r8b_v8_systemctl_identity_root_failed")?
        {
            return Err("r8b_v8_systemctl_identity_changed");
        }
        Ok(())
    }
}

fn validate_systemctl_identity_v3(value: &SystemctlToolIdentityV3) -> ParentResultV3<()> {
    require_composition_root_v1(&value.sha256)
        .map_err(|_| "r8b_v8_systemctl_identity_root_invalid")?;
    if value.schema != "nando.k2-self-formed-r8b-systemctl-tool.v3"
        || value.path != SYSTEMCTL_V3
        || value.unix_mode != 0o755
        || value.byte_len != PINNED_SYSTEMCTL_BYTE_LEN_V3
        || value.uid != 0
        || value.gid != 0
        || value.sha256 != PINNED_SYSTEMCTL_SHA256_V3
    {
        return Err("r8b_v8_systemctl_identity_invalid");
    }
    Ok(())
}

fn systemctl_tool_fixture_v3() -> SystemctlToolCapabilityV3 {
    SystemctlToolCapabilityV3::bind(SystemctlToolIdentityV3 {
        schema: "nando.k2-self-formed-r8b-systemctl-tool.v3".to_owned(),
        path: SYSTEMCTL_V3.to_owned(),
        unix_mode: 0o755,
        byte_len: PINNED_SYSTEMCTL_BYTE_LEN_V3,
        uid: 0,
        gid: 0,
        sha256: PINNED_SYSTEMCTL_SHA256_V3.to_owned(),
    })
    .expect("frozen systemctl tool fixture")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProcessExecutionV3 {
    argv: Vec<String>,
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    started_monotonic_ns: u64,
    finished_monotonic_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct SystemdSubmissionReceiptV3 {
    schema: String,
    route_id_sha256: String,
    unit: String,
    delegated_launch_capability_root_sha256: String,
    normalized_argv: Vec<String>,
    systemd_run_sha256: String,
    child_executable_sha256: String,
    exit_code: i32,
    stdout_byte_len: u64,
    stdout_sha256: String,
    stderr_byte_len: u64,
    stderr_sha256: String,
    started_monotonic_ns: u64,
    finished_monotonic_ns: u64,
    receipt_root_sha256: String,
}

impl SystemdSubmissionReceiptV3 {
    fn seal(
        launch: &DelegatedLaunchCapabilityV3,
        execution: ProcessExecutionV3,
    ) -> ParentResultV3<Self> {
        launch.revalidate()?;
        if execution.argv != launch.contract.normalized_argv
            || execution.exit_code != 0
            || execution.stdout.len() > MAX_CAPTURE_BYTES_V3
            || !execution.stderr.is_empty()
            || execution.started_monotonic_ns >= execution.finished_monotonic_ns
        {
            return Err("r8b_v8_systemd_submission_invalid");
        }
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-systemd-submission.v3".to_owned(),
            route_id_sha256: launch.contract.route_id_sha256.clone(),
            unit: launch.contract.unit.clone(),
            delegated_launch_capability_root_sha256: launch.capability_root_sha256.clone(),
            normalized_argv: execution.argv,
            systemd_run_sha256: launch.contract.systemd_run_sha256.clone(),
            child_executable_sha256: launch.contract.child_executable_sha256.clone(),
            exit_code: execution.exit_code,
            stdout_byte_len: execution.stdout.len() as u64,
            stdout_sha256: composition_sha256_bytes_v1(&execution.stdout),
            stderr_byte_len: execution.stderr.len() as u64,
            stderr_sha256: composition_sha256_bytes_v1(&execution.stderr),
            started_monotonic_ns: execution.started_monotonic_ns,
            finished_monotonic_ns: execution.finished_monotonic_ns,
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_systemd_submission_root_failed")?;
        value.revalidate_for_launch(launch)?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        require_roots_v3([
            &self.route_id_sha256,
            &self.delegated_launch_capability_root_sha256,
            &self.systemd_run_sha256,
            &self.child_executable_sha256,
            &self.stdout_sha256,
            &self.stderr_sha256,
            &self.receipt_root_sha256,
        ])?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-systemd-submission.v3"
            || self.unit != route_unit_v3(&self.route_id_sha256)
            || self.normalized_argv.len() != 22
            || self.normalized_argv[0] != "/usr/bin/systemd-run"
            || self.normalized_argv[4] != format!("--unit={}", self.unit)
            || self.exit_code != 0
            || self.stdout_byte_len > MAX_CAPTURE_BYTES_V3 as u64
            || self.stderr_byte_len != 0
            || self.stderr_sha256 != composition_sha256_bytes_v1(&[])
            || self.started_monotonic_ns >= self.finished_monotonic_ns
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_systemd_submission_root_failed")?
        {
            return Err("r8b_v8_systemd_submission_changed");
        }
        Ok(())
    }

    fn revalidate_for_launch(&self, launch: &DelegatedLaunchCapabilityV3) -> ParentResultV3<()> {
        self.revalidate()?;
        launch.revalidate()?;
        if self.route_id_sha256 != launch.contract.route_id_sha256
            || self.unit != launch.contract.unit
            || self.delegated_launch_capability_root_sha256 != launch.capability_root_sha256
            || self.normalized_argv != launch.contract.normalized_argv
            || self.systemd_run_sha256 != launch.contract.systemd_run_sha256
            || self.child_executable_sha256 != launch.contract.child_executable_sha256
        {
            return Err("r8b_v8_systemd_submission_launch_mismatch");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChildTerminalObservationV3 {
    unit: String,
    invocation_id: String,
    main_pid: u32,
    exec_main_code: String,
    exec_main_status: i32,
    finished_monotonic_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct ChildTerminalReceiptV3 {
    schema: String,
    route_id_sha256: String,
    unit: String,
    invocation_id: String,
    main_pid: u32,
    exec_main_code: String,
    exec_main_status: i32,
    finished_monotonic_ns: u64,
    receipt_root_sha256: String,
}

impl ChildTerminalReceiptV3 {
    fn seal(
        route_id_sha256: String,
        observation: ChildTerminalObservationV3,
    ) -> ParentResultV3<Self> {
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-child-terminal.v3".to_owned(),
            route_id_sha256,
            unit: observation.unit,
            invocation_id: observation.invocation_id,
            main_pid: observation.main_pid,
            exec_main_code: observation.exec_main_code,
            exec_main_status: observation.exec_main_status,
            finished_monotonic_ns: observation.finished_monotonic_ns,
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_child_terminal_root_failed")?;
        value.revalidate()?;
        Ok(value)
    }

    fn revalidate(&self) -> ParentResultV3<()> {
        require_roots_v3([&self.route_id_sha256, &self.receipt_root_sha256])?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-child-terminal.v3"
            || self.unit != route_unit_v3(&self.route_id_sha256)
            || !valid_invocation_id_v3(&self.invocation_id)
            || self.main_pid == 0
            || self.exec_main_code != "exited"
            || self.exec_main_status != 0
            || self.finished_monotonic_ns == 0
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_child_terminal_root_failed")?
        {
            return Err("r8b_v8_child_terminal_invalid");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedLoadedUnitV3 {
    invocation_id: String,
    main_pid: u32,
    exec_main_code: String,
    exec_main_status: i32,
    active_state: String,
    sub_state: String,
    memory_peak: u64,
    memory_swap_peak: u64,
    oom_policy: String,
    oom_kills: u64,
    tasks_current: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct LoadedMetricsReceiptV3 {
    schema: String,
    route_id_sha256: String,
    unit: String,
    systemctl_tool_root_sha256: String,
    normalized_argv: Vec<String>,
    exit_code: i32,
    stdout_byte_len: u64,
    stdout_sha256: String,
    stderr_byte_len: u64,
    stderr_sha256: String,
    invocation_id: String,
    main_pid: u32,
    exec_main_code: String,
    exec_main_status: i32,
    active_state: String,
    sub_state: String,
    memory_peak: u64,
    memory_swap_peak: u64,
    oom_policy: String,
    oom_kills: u64,
    tasks_current: u64,
    child_finished_monotonic_ns: u64,
    started_monotonic_ns: u64,
    finished_monotonic_ns: u64,
    receipt_root_sha256: String,
}

impl LoadedMetricsReceiptV3 {
    fn seal(
        route_id_sha256: String,
        tool: &SystemctlToolCapabilityV3,
        child: &ChildTerminalReceiptV3,
        execution: ProcessExecutionV3,
    ) -> ParentResultV3<Self> {
        tool.revalidate()?;
        child.revalidate()?;
        let unit = route_unit_v3(&route_id_sha256);
        let parsed = parse_loaded_unit_v3(&execution.stdout)?;
        if execution.argv != loaded_metrics_argv_v3(&unit)
            || execution.exit_code != 0
            || execution.stdout.len() > MAX_CAPTURE_BYTES_V3
            || !execution.stderr.is_empty()
            || child.route_id_sha256 != route_id_sha256
            || child.unit != unit
            || child.invocation_id != parsed.invocation_id
            || child.main_pid != parsed.main_pid
            || child.exec_main_code != parsed.exec_main_code
            || child.exec_main_status != parsed.exec_main_status
            || child.finished_monotonic_ns > execution.started_monotonic_ns
            || execution.started_monotonic_ns >= execution.finished_monotonic_ns
        {
            return Err("r8b_v8_loaded_metrics_completion_invalid");
        }
        validate_loaded_values_v3(&parsed)?;
        let mut value = Self {
            schema: "nando.k2-self-formed-r8b-loaded-metrics.v3".to_owned(),
            route_id_sha256,
            unit,
            systemctl_tool_root_sha256: tool.identity_root_sha256.clone(),
            normalized_argv: execution.argv,
            exit_code: execution.exit_code,
            stdout_byte_len: execution.stdout.len() as u64,
            stdout_sha256: composition_sha256_bytes_v1(&execution.stdout),
            stderr_byte_len: execution.stderr.len() as u64,
            stderr_sha256: composition_sha256_bytes_v1(&execution.stderr),
            invocation_id: parsed.invocation_id,
            main_pid: parsed.main_pid,
            exec_main_code: parsed.exec_main_code,
            exec_main_status: parsed.exec_main_status,
            active_state: parsed.active_state,
            sub_state: parsed.sub_state,
            memory_peak: parsed.memory_peak,
            memory_swap_peak: parsed.memory_swap_peak,
            oom_policy: parsed.oom_policy,
            oom_kills: parsed.oom_kills,
            tasks_current: parsed.tasks_current,
            child_finished_monotonic_ns: child.finished_monotonic_ns,
            started_monotonic_ns: execution.started_monotonic_ns,
            finished_monotonic_ns: execution.finished_monotonic_ns,
            receipt_root_sha256: String::new(),
        };
        value.receipt_root_sha256 =
            uncertainty_root_v1(&value).map_err(|_| "r8b_v8_loaded_metrics_root_failed")?;
        value.revalidate(tool, child)?;
        Ok(value)
    }

    fn revalidate(
        &self,
        tool: &SystemctlToolCapabilityV3,
        child: &ChildTerminalReceiptV3,
    ) -> ParentResultV3<()> {
        require_roots_v3([
            &self.route_id_sha256,
            &self.systemctl_tool_root_sha256,
            &self.stdout_sha256,
            &self.stderr_sha256,
            &self.receipt_root_sha256,
        ])?;
        tool.revalidate()?;
        child.revalidate()?;
        let parsed = ParsedLoadedUnitV3 {
            invocation_id: self.invocation_id.clone(),
            main_pid: self.main_pid,
            exec_main_code: self.exec_main_code.clone(),
            exec_main_status: self.exec_main_status,
            active_state: self.active_state.clone(),
            sub_state: self.sub_state.clone(),
            memory_peak: self.memory_peak,
            memory_swap_peak: self.memory_swap_peak,
            oom_policy: self.oom_policy.clone(),
            oom_kills: self.oom_kills,
            tasks_current: self.tasks_current,
        };
        validate_loaded_values_v3(&parsed)?;
        let mut canonical = self.clone();
        canonical.receipt_root_sha256.clear();
        if self.schema != "nando.k2-self-formed-r8b-loaded-metrics.v3"
            || self.unit != route_unit_v3(&self.route_id_sha256)
            || self.systemctl_tool_root_sha256 != tool.identity_root_sha256
            || self.normalized_argv != loaded_metrics_argv_v3(&self.unit)
            || self.exit_code != 0
            || self.stdout_byte_len > MAX_CAPTURE_BYTES_V3 as u64
            || self.stderr_byte_len != 0
            || self.stderr_sha256 != composition_sha256_bytes_v1(&[])
            || child.route_id_sha256 != self.route_id_sha256
            || child.unit != self.unit
            || child.invocation_id != self.invocation_id
            || child.main_pid != self.main_pid
            || child.exec_main_code != self.exec_main_code
            || child.exec_main_status != self.exec_main_status
            || child.finished_monotonic_ns != self.child_finished_monotonic_ns
            || self.child_finished_monotonic_ns > self.started_monotonic_ns
            || self.started_monotonic_ns >= self.finished_monotonic_ns
            || self.receipt_root_sha256
                != uncertainty_root_v1(&canonical)
                    .map_err(|_| "r8b_v8_loaded_metrics_root_failed")?
        {
            return Err("r8b_v8_loaded_metrics_changed");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct LoadedUnitCapabilityRootV3 {
    schema: String,
    submission: SystemdSubmissionReceiptV3,
    child_terminal: ChildTerminalReceiptV3,
    loaded_metrics: LoadedMetricsReceiptV3,
    systemctl_tool_identity_root_sha256: String,
}

pub(super) struct LoadedUnitResourceCapabilityV3 {
    submission: SystemdSubmissionReceiptV3,
    child_terminal: ChildTerminalReceiptV3,
    loaded_metrics: LoadedMetricsReceiptV3,
    systemctl_tool: SystemctlToolCapabilityV3,
    capability_root_sha256: String,
}

impl LoadedUnitResourceCapabilityV3 {
    fn bind(
        launch: &DelegatedLaunchCapabilityV3,
        systemctl_tool: SystemctlToolCapabilityV3,
        submission_execution: ProcessExecutionV3,
        child_observation: ChildTerminalObservationV3,
        metrics_execution: ProcessExecutionV3,
    ) -> ParentResultV3<Self> {
        launch.revalidate()?;
        systemctl_tool.revalidate()?;
        let submission = SystemdSubmissionReceiptV3::seal(launch, submission_execution)?;
        let child_terminal = ChildTerminalReceiptV3::seal(
            launch.contract.route_id_sha256.clone(),
            child_observation,
        )?;
        let loaded_metrics = LoadedMetricsReceiptV3::seal(
            launch.contract.route_id_sha256.clone(),
            &systemctl_tool,
            &child_terminal,
            metrics_execution,
        )?;
        if submission.finished_monotonic_ns > child_terminal.finished_monotonic_ns
            || loaded_metrics
                .finished_monotonic_ns
                .checked_sub(submission.started_monotonic_ns)
                .is_none_or(|elapsed| elapsed > MAX_ROUTE_NS_V3)
        {
            return Err("r8b_v8_resource_chronology_invalid");
        }
        let root = LoadedUnitCapabilityRootV3 {
            schema: "nando.k2-self-formed-r8b-loaded-unit-capability.v3".to_owned(),
            submission: submission.clone(),
            child_terminal: child_terminal.clone(),
            loaded_metrics: loaded_metrics.clone(),
            systemctl_tool_identity_root_sha256: systemctl_tool.identity_root_sha256.clone(),
        };
        let value = Self {
            submission,
            child_terminal,
            loaded_metrics,
            systemctl_tool,
            capability_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_resource_capability_root_failed")?,
        };
        value.revalidate_for_launch(launch)?;
        Ok(value)
    }

    pub(super) fn revalidate(&self) -> ParentResultV3<()> {
        self.submission.revalidate()?;
        self.child_terminal.revalidate()?;
        self.loaded_metrics
            .revalidate(&self.systemctl_tool, &self.child_terminal)?;
        if self.submission.route_id_sha256 != self.child_terminal.route_id_sha256
            || self.submission.route_id_sha256 != self.loaded_metrics.route_id_sha256
            || self.submission.unit != self.child_terminal.unit
            || self.submission.unit != self.loaded_metrics.unit
            || self.submission.finished_monotonic_ns > self.child_terminal.finished_monotonic_ns
            || self
                .loaded_metrics
                .finished_monotonic_ns
                .checked_sub(self.submission.started_monotonic_ns)
                .is_none_or(|elapsed| elapsed > MAX_ROUTE_NS_V3)
        {
            return Err("r8b_v8_resource_capability_mismatch");
        }
        let root = LoadedUnitCapabilityRootV3 {
            schema: "nando.k2-self-formed-r8b-loaded-unit-capability.v3".to_owned(),
            submission: self.submission.clone(),
            child_terminal: self.child_terminal.clone(),
            loaded_metrics: self.loaded_metrics.clone(),
            systemctl_tool_identity_root_sha256: self.systemctl_tool.identity_root_sha256.clone(),
        };
        if self.capability_root_sha256
            != composition_root_v1(&root).map_err(|_| "r8b_v8_resource_capability_root_failed")?
        {
            return Err("r8b_v8_resource_capability_changed");
        }
        Ok(())
    }

    pub(super) fn revalidate_for_launch(
        &self,
        launch: &DelegatedLaunchCapabilityV3,
    ) -> ParentResultV3<()> {
        self.revalidate()?;
        self.submission.revalidate_for_launch(launch)
    }

    pub(super) fn transition_roots(&self) -> [String; 3] {
        [
            self.submission.receipt_root_sha256.clone(),
            self.child_terminal.receipt_root_sha256.clone(),
            self.loaded_metrics.receipt_root_sha256.clone(),
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DescendantObservationV3 {
    cgroup_path: String,
    descendant_pids: Vec<u32>,
    observed_monotonic_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct StoppedUnitClosureRootV3 {
    schema: String,
    route_id_sha256: String,
    unit: String,
    loaded_metrics_root_sha256: String,
    systemctl_tool_root_sha256: String,
    stop_argv: Vec<String>,
    stop_exit_code: i32,
    stop_stdout_sha256: String,
    stop_stderr_sha256: String,
    stop_started_monotonic_ns: u64,
    stop_finished_monotonic_ns: u64,
    state_argv: Vec<String>,
    state_exit_code: i32,
    state_stdout_sha256: String,
    state_stderr_sha256: String,
    load_state: String,
    active_state: String,
    sub_state: String,
    state_started_monotonic_ns: u64,
    state_finished_monotonic_ns: u64,
    cgroup_path: String,
    descendant_pids: Vec<u32>,
    descendants_observed_monotonic_ns: u64,
}

pub(super) struct StoppedUnitResourceCapabilityV3 {
    root: StoppedUnitClosureRootV3,
    stop_root_sha256: String,
    inactive_root_sha256: String,
    residue_root_sha256: String,
    capability_root_sha256: String,
}

fn stopped_transition_roots_v3(root: &StoppedUnitClosureRootV3) -> ParentResultV3<[String; 3]> {
    Ok([
        composition_root_v1(&(
            "p04b-stop",
            &root.loaded_metrics_root_sha256,
            &root.stop_argv,
            root.stop_exit_code,
            &root.stop_stdout_sha256,
            &root.stop_stderr_sha256,
            root.stop_started_monotonic_ns,
            root.stop_finished_monotonic_ns,
        ))
        .map_err(|_| "r8b_v8_p04b_stop_root_failed")?,
        composition_root_v1(&(
            "p04b-inactive",
            &root.state_argv,
            root.state_exit_code,
            &root.state_stdout_sha256,
            &root.state_stderr_sha256,
            &root.load_state,
            &root.active_state,
            &root.sub_state,
            root.state_started_monotonic_ns,
            root.state_finished_monotonic_ns,
        ))
        .map_err(|_| "r8b_v8_p04b_inactive_root_failed")?,
        composition_root_v1(&(
            "p04b-residue",
            &root.cgroup_path,
            &root.descendant_pids,
            root.descendants_observed_monotonic_ns,
        ))
        .map_err(|_| "r8b_v8_p04b_residue_root_failed")?,
    ])
}

impl StoppedUnitResourceCapabilityV3 {
    fn bind(
        loaded: &LoadedUnitResourceCapabilityV3,
        stop: ProcessExecutionV3,
        state: ProcessExecutionV3,
        residue: DescendantObservationV3,
    ) -> ParentResultV3<Self> {
        loaded.revalidate()?;
        let route = loaded.loaded_metrics.route_id_sha256.clone();
        let unit = loaded.loaded_metrics.unit.clone();
        let parsed = parse_post_stop_state_v3(&state.stdout)?;
        let cgroup_suffix = format!("/{unit}");
        if stop.argv != stop_argv_v3(&unit)
            || stop.exit_code != 0
            || !stop.stdout.is_empty()
            || !stop.stderr.is_empty()
            || state.argv != post_stop_state_argv_v3(&unit)
            || state.exit_code != 0
            || !state.stderr.is_empty()
            || parsed.1 != "inactive"
            || parsed.2 != "dead"
            || !matches!(parsed.0.as_str(), "loaded" | "not-found")
            || !Path::new(&residue.cgroup_path).is_absolute()
            || !residue.cgroup_path.starts_with("/sys/fs/cgroup/")
            || !residue.cgroup_path.ends_with(&cgroup_suffix)
            || !residue.descendant_pids.is_empty()
            || loaded.loaded_metrics.finished_monotonic_ns > stop.started_monotonic_ns
            || stop.started_monotonic_ns >= stop.finished_monotonic_ns
            || stop.finished_monotonic_ns > state.started_monotonic_ns
            || state.started_monotonic_ns >= state.finished_monotonic_ns
            || state.finished_monotonic_ns > residue.observed_monotonic_ns
            || residue
                .observed_monotonic_ns
                .checked_sub(loaded.submission.started_monotonic_ns)
                .is_none_or(|elapsed| elapsed > MAX_ROUTE_NS_V3)
        {
            return Err("r8b_v8_p04b_stop_observation_invalid");
        }
        let root = StoppedUnitClosureRootV3 {
            schema: "nando.k2-self-formed-r8b-stopped-unit-capability.v3".to_owned(),
            route_id_sha256: route,
            unit,
            loaded_metrics_root_sha256: loaded.loaded_metrics.receipt_root_sha256.clone(),
            systemctl_tool_root_sha256: loaded.systemctl_tool.identity_root_sha256.clone(),
            stop_argv: stop.argv,
            stop_exit_code: stop.exit_code,
            stop_stdout_sha256: composition_sha256_bytes_v1(&stop.stdout),
            stop_stderr_sha256: composition_sha256_bytes_v1(&stop.stderr),
            stop_started_monotonic_ns: stop.started_monotonic_ns,
            stop_finished_monotonic_ns: stop.finished_monotonic_ns,
            state_argv: state.argv,
            state_exit_code: state.exit_code,
            state_stdout_sha256: composition_sha256_bytes_v1(&state.stdout),
            state_stderr_sha256: composition_sha256_bytes_v1(&state.stderr),
            load_state: parsed.0,
            active_state: parsed.1,
            sub_state: parsed.2,
            state_started_monotonic_ns: state.started_monotonic_ns,
            state_finished_monotonic_ns: state.finished_monotonic_ns,
            cgroup_path: residue.cgroup_path,
            descendant_pids: residue.descendant_pids,
            descendants_observed_monotonic_ns: residue.observed_monotonic_ns,
        };
        let [stop_root_sha256, inactive_root_sha256, residue_root_sha256] =
            stopped_transition_roots_v3(&root)?;
        let value = Self {
            stop_root_sha256,
            inactive_root_sha256,
            residue_root_sha256,
            capability_root_sha256: composition_root_v1(&root)
                .map_err(|_| "r8b_v8_p04b_capability_root_failed")?,
            root,
        };
        value.revalidate_for_loaded(loaded)?;
        Ok(value)
    }

    pub(super) fn revalidate_for_loaded(
        &self,
        loaded: &LoadedUnitResourceCapabilityV3,
    ) -> ParentResultV3<()> {
        loaded.revalidate()?;
        let [stop_root_sha256, inactive_root_sha256, residue_root_sha256] =
            stopped_transition_roots_v3(&self.root)?;
        let cgroup_suffix = format!("/{}", self.root.unit);
        if self.root.schema != "nando.k2-self-formed-r8b-stopped-unit-capability.v3"
            || self.root.route_id_sha256 != loaded.loaded_metrics.route_id_sha256
            || self.root.unit != loaded.loaded_metrics.unit
            || self.root.loaded_metrics_root_sha256 != loaded.loaded_metrics.receipt_root_sha256
            || self.root.systemctl_tool_root_sha256 != loaded.systemctl_tool.identity_root_sha256
            || self.root.stop_argv != stop_argv_v3(&self.root.unit)
            || self.root.stop_exit_code != 0
            || self.root.stop_stdout_sha256 != composition_sha256_bytes_v1(&[])
            || self.root.stop_stderr_sha256 != composition_sha256_bytes_v1(&[])
            || self.root.state_argv != post_stop_state_argv_v3(&self.root.unit)
            || self.root.state_exit_code != 0
            || self.root.state_stderr_sha256 != composition_sha256_bytes_v1(&[])
            || !matches!(self.root.load_state.as_str(), "loaded" | "not-found")
            || self.root.active_state != "inactive"
            || self.root.sub_state != "dead"
            || !self.root.cgroup_path.starts_with("/sys/fs/cgroup/")
            || !self.root.cgroup_path.ends_with(&cgroup_suffix)
            || !self.root.descendant_pids.is_empty()
            || self.root.stop_started_monotonic_ns < loaded.loaded_metrics.finished_monotonic_ns
            || self.root.stop_started_monotonic_ns >= self.root.stop_finished_monotonic_ns
            || self.root.stop_finished_monotonic_ns > self.root.state_started_monotonic_ns
            || self.root.state_started_monotonic_ns >= self.root.state_finished_monotonic_ns
            || self.root.state_finished_monotonic_ns > self.root.descendants_observed_monotonic_ns
            || self
                .root
                .descendants_observed_monotonic_ns
                .checked_sub(loaded.submission.started_monotonic_ns)
                .is_none_or(|elapsed| elapsed > MAX_ROUTE_NS_V3)
            || self.stop_root_sha256 != stop_root_sha256
            || self.inactive_root_sha256 != inactive_root_sha256
            || self.residue_root_sha256 != residue_root_sha256
            || self.capability_root_sha256
                != composition_root_v1(&self.root)
                    .map_err(|_| "r8b_v8_p04b_capability_root_failed")?
        {
            return Err("r8b_v8_p04b_capability_changed");
        }
        Ok(())
    }

    pub(super) fn transition_roots(&self) -> [String; 3] {
        [
            self.stop_root_sha256.clone(),
            self.inactive_root_sha256.clone(),
            self.residue_root_sha256.clone(),
        ]
    }

    pub(super) fn finished_monotonic_ns(&self) -> u64 {
        self.root.descendants_observed_monotonic_ns
    }
}

fn stop_argv_v3(unit: &str) -> Vec<String> {
    [SYSTEMCTL_V3, "--user", "--no-ask-password", "stop", unit]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn post_stop_state_argv_v3(unit: &str) -> Vec<String> {
    [
        SYSTEMCTL_V3,
        "--user",
        "--no-ask-password",
        "show",
        "--property=LoadState",
        "--property=ActiveState",
        "--property=SubState",
        unit,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn parse_post_stop_state_v3(bytes: &[u8]) -> ParentResultV3<(String, String, String)> {
    let text = std::str::from_utf8(bytes).map_err(|_| "r8b_v8_p04b_state_utf8_invalid")?;
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let (name, value) = line
            .split_once('=')
            .ok_or("r8b_v8_p04b_state_line_invalid")?;
        if value.is_empty() || values.insert(name, value).is_some() {
            return Err("r8b_v8_p04b_state_line_invalid");
        }
    }
    if values.len() != 3 {
        return Err("r8b_v8_p04b_state_set_invalid");
    }
    Ok((
        values
            .remove("LoadState")
            .ok_or("r8b_v8_p04b_load_state_missing")?
            .to_owned(),
        values
            .remove("ActiveState")
            .ok_or("r8b_v8_p04b_active_state_missing")?
            .to_owned(),
        values
            .remove("SubState")
            .ok_or("r8b_v8_p04b_sub_state_missing")?
            .to_owned(),
    ))
}

fn loaded_metrics_argv_v3(unit: &str) -> Vec<String> {
    let mut argv = vec![
        SYSTEMCTL_V3.to_owned(),
        "--user".to_owned(),
        "--no-pager".to_owned(),
    ];
    argv.extend(UNIT_PROPERTIES_V3.map(|property| format!("--property={property}")));
    argv.extend(["show".to_owned(), "--".to_owned(), unit.to_owned()]);
    argv
}

fn parse_loaded_unit_v3(bytes: &[u8]) -> ParentResultV3<ParsedLoadedUnitV3> {
    let text = std::str::from_utf8(bytes).map_err(|_| "r8b_v8_loaded_metrics_utf8_invalid")?;
    let Some(body) = text.strip_suffix('\n') else {
        return Err("r8b_v8_loaded_metrics_termination_invalid");
    };
    if body.is_empty() || body.contains('\r') || body.contains('\0') {
        return Err("r8b_v8_loaded_metrics_text_invalid");
    }
    let mut properties = BTreeMap::new();
    for line in body.split('\n') {
        let (name, value) = line
            .split_once('=')
            .ok_or("r8b_v8_loaded_metrics_row_invalid")?;
        if !UNIT_PROPERTIES_V3.contains(&name)
            || properties
                .insert(name.to_owned(), value.to_owned())
                .is_some()
        {
            return Err("r8b_v8_loaded_metrics_property_invalid");
        }
    }
    if properties.len() != UNIT_PROPERTIES_V3.len() {
        return Err("r8b_v8_loaded_metrics_property_missing");
    }
    let invocation_id = take_property_v3(&mut properties, "InvocationID")?;
    let exec_main_code_raw = take_property_v3(&mut properties, "ExecMainCode")?;
    let exec_main_code = match exec_main_code_raw.as_str() {
        "1" => "exited".to_owned(),
        _ => return Err("r8b_v8_loaded_metrics_exec_code_invalid"),
    };
    let value = ParsedLoadedUnitV3 {
        invocation_id,
        main_pid: parse_u32_v3(&take_property_v3(&mut properties, "MainPID")?)?,
        exec_main_code,
        exec_main_status: parse_i32_v3(&take_property_v3(&mut properties, "ExecMainStatus")?)?,
        active_state: take_property_v3(&mut properties, "ActiveState")?,
        sub_state: take_property_v3(&mut properties, "SubState")?,
        memory_peak: parse_u64_v3(&take_property_v3(&mut properties, "MemoryPeak")?)?,
        memory_swap_peak: parse_u64_v3(&take_property_v3(&mut properties, "MemorySwapPeak")?)?,
        oom_policy: take_property_v3(&mut properties, "OOMPolicy")?,
        oom_kills: parse_u64_v3(&take_property_v3(&mut properties, "OOMKills")?)?,
        tasks_current: parse_u64_v3(&take_property_v3(&mut properties, "TasksCurrent")?)?,
    };
    if !properties.is_empty() {
        return Err("r8b_v8_loaded_metrics_property_extra");
    }
    Ok(value)
}

fn validate_loaded_values_v3(value: &ParsedLoadedUnitV3) -> ParentResultV3<()> {
    if !valid_invocation_id_v3(&value.invocation_id)
        || value.main_pid == 0
        || value.exec_main_code != "exited"
        || value.exec_main_status != 0
        || value.active_state != "active"
        || value.sub_state != "exited"
        || value.memory_peak > 536_870_912
        || value.memory_swap_peak != 0
        || !matches!(value.oom_policy.as_str(), "continue" | "stop" | "kill")
        || value.oom_kills != 0
        || value.tasks_current > 256
    {
        return Err("r8b_v8_loaded_metrics_values_invalid");
    }
    Ok(())
}

fn take_property_v3(
    properties: &mut BTreeMap<String, String>,
    name: &str,
) -> ParentResultV3<String> {
    properties
        .remove(name)
        .ok_or("r8b_v8_loaded_metrics_property_missing")
}

fn parse_u64_v3(value: &str) -> ParentResultV3<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| "r8b_v8_loaded_metrics_integer_invalid")?;
    if parsed.to_string() != value {
        return Err("r8b_v8_loaded_metrics_integer_noncanonical");
    }
    Ok(parsed)
}

fn parse_u32_v3(value: &str) -> ParentResultV3<u32> {
    let parsed = parse_u64_v3(value)?;
    u32::try_from(parsed).map_err(|_| "r8b_v8_loaded_metrics_integer_overflow")
}

fn parse_i32_v3(value: &str) -> ParentResultV3<i32> {
    let parsed = value
        .parse::<i32>()
        .map_err(|_| "r8b_v8_loaded_metrics_integer_invalid")?;
    if parsed.to_string() != value {
        return Err("r8b_v8_loaded_metrics_integer_noncanonical");
    }
    Ok(parsed)
}

fn valid_invocation_id_v3(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_roots_v3<const N: usize>(roots: [&str; N]) -> ParentResultV3<()> {
    roots.into_iter().try_for_each(|root| {
        require_composition_root_v1(root).map_err(|_| "r8b_v8_resource_root_invalid")
    })
}

fn property_output_v3(invocation_id: &str, main_pid: u32) -> Vec<u8> {
    format!(
        "InvocationID={invocation_id}\n\
         MainPID={main_pid}\n\
         ExecMainCode=1\n\
         ExecMainStatus=0\n\
         ActiveState=active\n\
         SubState=exited\n\
         MemoryPeak=1048576\n\
         MemorySwapPeak=0\n\
         OOMPolicy=stop\n\
         OOMKills=0\n\
         TasksCurrent=0\n"
    )
    .into_bytes()
}

fn replace_property_v3(bytes: &[u8], name: &str, replacement: &str) -> Vec<u8> {
    let text = std::str::from_utf8(bytes).expect("fixture property bytes");
    let mut output = text
        .lines()
        .map(|line| {
            if line.starts_with(&format!("{name}=")) {
                replacement.to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output.into_bytes()
}

fn execution_fixtures_v3(
    launch: &DelegatedLaunchCapabilityV3,
) -> (
    ProcessExecutionV3,
    ChildTerminalObservationV3,
    ProcessExecutionV3,
) {
    let invocation_id = "0123456789abcdef0123456789abcdef".to_owned();
    let submission = ProcessExecutionV3 {
        argv: launch.contract.normalized_argv.clone(),
        exit_code: 0,
        stdout: format!("Running as unit: {}\n", launch.contract.unit).into_bytes(),
        stderr: Vec::new(),
        started_monotonic_ns: 10,
        finished_monotonic_ns: 20,
    };
    let child = ChildTerminalObservationV3 {
        unit: launch.contract.unit.clone(),
        invocation_id: invocation_id.clone(),
        main_pid: 4242,
        exec_main_code: "exited".to_owned(),
        exec_main_status: 0,
        finished_monotonic_ns: 30,
    };
    let metrics = ProcessExecutionV3 {
        argv: loaded_metrics_argv_v3(&launch.contract.unit),
        exit_code: 0,
        stdout: property_output_v3(&invocation_id, child.main_pid),
        stderr: Vec::new(),
        started_monotonic_ns: 40,
        finished_monotonic_ns: 50,
    };
    (submission, child, metrics)
}

fn bind_resource_fixture_v3(
    launch: &DelegatedLaunchCapabilityV3,
    submission: ProcessExecutionV3,
    child: ChildTerminalObservationV3,
    metrics: ProcessExecutionV3,
) -> ParentResultV3<LoadedUnitResourceCapabilityV3> {
    LoadedUnitResourceCapabilityV3::bind(
        launch,
        systemctl_tool_fixture_v3(),
        submission,
        child,
        metrics,
    )
}

pub(super) fn loaded_unit_resource_fixture_v3(
    launch: &DelegatedLaunchCapabilityV3,
) -> LoadedUnitResourceCapabilityV3 {
    let (submission, child, metrics) = execution_fixtures_v3(launch);
    bind_resource_fixture_v3(launch, submission, child, metrics)
        .expect("loaded unit resource fixture")
}

fn stopped_observations_fixture_v3(
    loaded: &LoadedUnitResourceCapabilityV3,
) -> (
    ProcessExecutionV3,
    ProcessExecutionV3,
    DescendantObservationV3,
) {
    let unit = &loaded.loaded_metrics.unit;
    (
        ProcessExecutionV3 {
            argv: stop_argv_v3(unit),
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            started_monotonic_ns: 60,
            finished_monotonic_ns: 65,
        },
        ProcessExecutionV3 {
            argv: post_stop_state_argv_v3(unit),
            exit_code: 0,
            stdout: b"LoadState=not-found\nActiveState=inactive\nSubState=dead\n".to_vec(),
            stderr: Vec::new(),
            started_monotonic_ns: 66,
            finished_monotonic_ns: 70,
        },
        DescendantObservationV3 {
            cgroup_path: format!("/sys/fs/cgroup/user.slice/{unit}"),
            descendant_pids: Vec::new(),
            observed_monotonic_ns: 80,
        },
    )
}

pub(super) fn stopped_unit_resource_fixture_v3(
    loaded: &LoadedUnitResourceCapabilityV3,
) -> StoppedUnitResourceCapabilityV3 {
    let (stop, state, residue) = stopped_observations_fixture_v3(loaded);
    StoppedUnitResourceCapabilityV3::bind(loaded, stop, state, residue)
        .expect("stopped unit resource fixture")
}

#[test]
fn r8b_v8_p04a_binds_exact_submission_terminal_metrics_and_tool() {
    let environment = TestEnvironmentV1::new("p04a-positive");
    let route = root_v1("p04a-positive");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);
    let resources = loaded_unit_resource_fixture_v3(&launch);
    resources
        .revalidate_for_launch(&launch)
        .expect("resource fixture");
    resources.transition_roots().iter().for_each(|root| {
        require_composition_root_v1(root).expect("resource fixture");
    });
}

#[test]
fn r8b_v8_p04a_rejects_unit_invocation_and_pid_drift() {
    let environment = TestEnvironmentV1::new("p04a-identity-negative");
    let route = root_v1("p04a-identity-negative");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);
    let (submission, mut child, metrics) = execution_fixtures_v3(&launch);
    child.unit = "nando-r8b-foreign.service".to_owned();
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());
    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.stdout = replace_property_v3(
        &metrics.stdout,
        "InvocationID",
        "InvocationID=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.stdout = replace_property_v3(&metrics.stdout, "MainPID", "MainPID=4243");
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());
}

#[test]
fn r8b_v8_p04a_rejects_disappeared_or_nonterminal_unit() {
    let environment = TestEnvironmentV1::new("p04a-state-negative");
    let route = root_v1("p04a-state-negative");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);

    for (name, value) in [
        ("ActiveState", "ActiveState=inactive"),
        ("SubState", "SubState=running"),
    ] {
        let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
        metrics.stdout = replace_property_v3(&metrics.stdout, name, value);
        assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());
    }
}

#[test]
fn r8b_v8_p04a_rejects_missing_duplicate_or_unknown_property() {
    let environment = TestEnvironmentV1::new("p04a-properties-negative");
    let route = root_v1("p04a-properties-negative");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.stdout = replace_property_v3(&metrics.stdout, "OOMKills", "");
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.stdout.extend_from_slice(b"MainPID=4242\n");
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.stdout.extend_from_slice(b"Foreign=1\n");
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());
}

#[test]
fn r8b_v8_p04a_rejects_chronology_argv_and_tool_substitution() {
    let environment = TestEnvironmentV1::new("p04a-route-negative");
    let route = root_v1("p04a-route-negative");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.started_monotonic_ns = child.finished_monotonic_ns - 1;
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());

    let (submission, child, mut metrics) = execution_fixtures_v3(&launch);
    metrics.argv.push("--all".to_owned());
    assert!(bind_resource_fixture_v3(&launch, submission, child, metrics).is_err());

    let mut tool = systemctl_tool_fixture_v3().identity;
    tool.sha256 = root_v1("substituted-systemctl");
    assert!(SystemctlToolCapabilityV3::bind(tool).is_err());
}

#[test]
fn r8b_v8_p04b_binds_exact_stop_inactive_state_and_zero_residue() {
    let environment = TestEnvironmentV1::new("p04b-positive");
    let route = root_v1("p04b-positive");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);
    let loaded = loaded_unit_resource_fixture_v3(&launch);
    let stopped = stopped_unit_resource_fixture_v3(&loaded);
    stopped
        .revalidate_for_loaded(&loaded)
        .expect("resource fixture");
    stopped
        .transition_roots()
        .iter()
        .for_each(|root| require_composition_root_v1(root).expect("resource fixture"));
}

#[test]
fn r8b_v8_p04b_rejects_broad_stop_state_drift_and_residue() {
    let environment = TestEnvironmentV1::new("p04b-negative");
    let route = root_v1("p04b-negative");
    let launch = delegated_launch_fixture_v3(&environment, "launch", &route);
    let loaded = loaded_unit_resource_fixture_v3(&launch);
    let (mut stop, state, residue) = stopped_observations_fixture_v3(&loaded);
    stop.argv.pop();
    assert!(StoppedUnitResourceCapabilityV3::bind(&loaded, stop, state, residue).is_err());
    let (stop, mut state, residue) = stopped_observations_fixture_v3(&loaded);
    state.stdout = b"LoadState=loaded\nActiveState=active\nSubState=running\n".to_vec();
    assert!(StoppedUnitResourceCapabilityV3::bind(&loaded, stop, state, residue).is_err());
    let (stop, state, mut residue) = stopped_observations_fixture_v3(&loaded);
    residue.descendant_pids.push(4242);
    assert!(StoppedUnitResourceCapabilityV3::bind(&loaded, stop, state, residue).is_err());
}
