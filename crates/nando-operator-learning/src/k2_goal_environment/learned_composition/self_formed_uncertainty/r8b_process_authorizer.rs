use super::super::{
    K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_bytes_v1,
    require_composition_root_v1, valid_composition_path_v1,
};
use super::{
    K2_UNCERTAINTY_R8B_DOWNSTREAM_CONTRACT_SCHEMA_V3, K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V3, K2_UNCERTAINTY_R8B_RESOURCE_RECEIPT_SCHEMA_V3,
    K2_UNCERTAINTY_R8B_SCHEDULE_AUTHORITY_SCHEMA_V3, K2_UNCERTAINTY_R8B_SCHEDULE_FORMULA_V3,
    K2_UNCERTAINTY_R8B_STATIC_PROJECTION_SCHEMA_V3, K2UncertaintyR8BCompletionKindV3 as Completion,
    K2UncertaintyR8BDownstreamContractV3, K2UncertaintyR8BEvidenceKindV2 as EvidenceKind,
    K2UncertaintyR8BExpectedOutcomeV3 as ExpectedOutcome, K2UncertaintyR8BInputRoleV3 as InputRole,
    K2UncertaintyR8BInvocationPlanV3, K2UncertaintyR8BLaunchKindV3 as LaunchKind,
    K2UncertaintyR8BLedgerSummaryV3, K2UncertaintyR8BManagerIdentityV3,
    K2UncertaintyR8BObjectRoleV3 as ObjectRole, K2UncertaintyR8BOutputContractV3,
    K2UncertaintyR8BPrivilegedProbeV3, K2UncertaintyR8BProcessEventV3,
    K2UncertaintyR8BProducerRequestV3, K2UncertaintyR8BResourceReceiptV3,
    K2UncertaintyR8BScheduleAuthorityV3, K2UncertaintyR8BStaticProjectionV3,
    K2UncertaintyR8BToolRoleV3 as ToolRole, K2UncertaintyR8BUnitResourceObservationV3,
    K2UncertaintyR8BValidatedFactV3 as ValidatedFact, K2UncertaintyR8BValidatorV3 as Validator,
    denied_authority_v1, require_denied_authority_v1, uncertainty_bytes_v1, uncertainty_root_v1,
    valid_r8b_role_v3,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[path = "r8b_process_authorizer/projection.rs"]
mod projection;

pub use projection::{
    validate_self_formed_r8b_downstream_contract_v3,
    validate_self_formed_r8b_process_projections_v3,
    validate_self_formed_r8b_schedule_authority_v3,
};

const MAX_PATH_BYTES_V3: usize = 240;
const MAX_STDOUT_BYTES_V3: u64 = 1_048_576;
const MAX_STDERR_BYTES_V3: u64 = 65_536;
const MAX_EVENT_BYTES_V3: usize = 4_096;
const MAX_AUTHORITY_OUTPUTS_V3: usize = 4;
const MAX_PROCESS_IDENTIFIER_BYTES_V3: usize = 128;
const STRACE_V3: &str = "/usr/bin/strace";
const BWRAP_V3: &str = "/usr/bin/bwrap";
const PRLIMIT_V3: &str = "/usr/bin/prlimit";
const SYSTEMD_RUN_V3: &str = "/usr/bin/systemd-run";
const SYSTEMD_MANAGER_V3: &str = "/usr/lib/systemd/systemd";
const SUDO_V3: &str = "/usr/lib/cargo/bin/sudo";
const SHA256SUM_V3: &str = "/usr/lib/cargo/bin/coreutils/sha256sum";
#[rustfmt::skip]
pub(super) const PRODUCER_ROLES_V3: [&str; 6] = [
    "S01_CRATE_UNIT", "S02_RESTART", "S03_MODE_MATRIX",
    "S04_CLEANUP_NEGATIVE", "S05_AUTHORITY_PUBLICATION", "M24_LINKED_RUNNER",
];
#[rustfmt::skip]
const DYNAMIC_ROLES_V3: [&str; 7] = [
    "M03_LEARNER", "M04_PROBE", "M05_SELECTOR", "M06_BASELINE",
    "M07_SELECTION_PREVERIFIER", "M08_CLOSURE_PLANNER", "M09_CLOSURE_VERIFIER",
];
#[rustfmt::skip]
const DOWNSTREAM_ROLES_V3: [&str; 13] = [
    "M11_PRIVATE_RESOLVER", "M12_SAFETY", "M13_WORKER", "M14_OBSERVER",
    "M15_FINAL_VERIFIER", "M16_ORACLE", "M17_CONTROL_EVALUATOR",
    "M18_TERMINAL_EVALUATOR", "M19_FRESH_CONTROL_CASE", "M20_CLEANUP_AUTHORIZER",
    "M21_CLEANUP_OWNER", "M22_CLEANUP_VERIFIER", "M23_DEVELOPMENT_RESULT_PUBLISHER",
];

pub fn seal_self_formed_r8b_resource_receipt_v3(
    mut value: K2UncertaintyR8BResourceReceiptV3,
) -> K2CompositionResultV1<K2UncertaintyR8BResourceReceiptV3> {
    value.schema = K2_UNCERTAINTY_R8B_RESOURCE_RECEIPT_SCHEMA_V3.to_owned();
    value.authority = denied_authority_v1();
    value.receipt_root_sha256.clear();
    value.receipt_root_sha256 = uncertainty_root_v1(&value)?;
    validate_self_formed_r8b_resource_receipt_v3(&value)?;
    Ok(value)
}

pub fn validate_self_formed_r8b_delegated_resource_v3(
    ledger: &K2UncertaintyR8BLedgerSummaryV3,
    value: &K2UncertaintyR8BResourceReceiptV3,
) -> K2CompositionResultV1<()> {
    validate_self_formed_r8b_resource_receipt_v3(value)?;
    let mut rows = ledger.invocations.iter().filter(|row| {
        row.target_role == "M24_LINKED_RUNNER" && row.launch_kind == LaunchKind::UserSystemd
    });
    let invocation = rows
        .next()
        .ok_or_else(|| invalid("self_formed_r8b_v3_delegated_invocation_missing"))?;
    reject(
        rows.next().is_some()
            || ledger
                .request_roots_sha256
                .get(&invocation.invocation_id_sha256)
                != Some(&value.delegated_launch_request_root_sha256),
        "self_formed_r8b_v3_delegated_request_invalid",
    )
}

pub fn validate_self_formed_r8b_resource_receipt_v3(
    value: &K2UncertaintyR8BResourceReceiptV3,
) -> K2CompositionResultV1<()> {
    require_roots([
        &value.route_id_sha256,
        &value.delegated_launch_request_root_sha256,
        &value.pinned_systemd_sha256,
        &value.receipt_root_sha256,
    ])?;
    require_denied_authority_v1(&value.authority)?;
    let unit = self_formed_r8b_route_unit_v3(&value.route_id_sha256)?;
    validate_systemd_run_argv_v3(&value.normalized_systemd_run_argv, &unit)?;
    validate_self_formed_r8b_manager_identity_v3(&value.manager_pre)?;
    reject(
        value.manager_pre != value.manager_post,
        "self_formed_r8b_manager_identity_drift",
    )?;
    validate_self_formed_r8b_privileged_probe_v3(
        &value.probe_pre,
        value.manager_pre.main_pid,
        &value.pinned_systemd_sha256,
    )?;
    validate_self_formed_r8b_privileged_probe_v3(
        &value.probe_post,
        value.manager_post.main_pid,
        &value.pinned_systemd_sha256,
    )?;
    validate_unit_resource_v3(&value.unit, &unit)?;
    let mut canonical = value.clone();
    canonical.receipt_root_sha256.clear();
    reject(
        value.schema != K2_UNCERTAINTY_R8B_RESOURCE_RECEIPT_SCHEMA_V3
            || value.probe_pre.sudo_sha256 != value.probe_post.sudo_sha256
            || value.probe_pre.sha256sum_sha256 != value.probe_post.sha256sum_sha256
            || value.probe_pre.stdout_sha256 != value.probe_post.stdout_sha256
            || value.probe_pre.finished_monotonic_ns > value.unit.route_started_monotonic_ns
            || value.unit.route_finished_monotonic_ns > value.probe_post.started_monotonic_ns
            || value.sudo_frontends != 2
            || value.sha256sum_descendants != 2
            || value.external_network_calls != 0
            || value.receipt_root_sha256 != uncertainty_root_v1(&canonical)?,
        "self_formed_r8b_resource_receipt_invalid",
    )
}

pub fn self_formed_r8b_route_unit_v3(route_id_sha256: &str) -> K2CompositionResultV1<String> {
    require_composition_root_v1(route_id_sha256)?;
    Ok(format!("nando-r8b-{}.service", &route_id_sha256[..16]))
}

fn validate_systemd_run_argv_v3(argv: &[String], unit: &str) -> K2CompositionResultV1<()> {
    #[rustfmt::skip]
    let fixed = [
        (0, SYSTEMD_RUN_V3), (1, "--user"), (2, "--no-ask-password"),
        (3, "--expand-environment=no"), (5, "--service-type=exec"),
        (6, "--remain-after-exit"), (7, "--property=MemoryMax=536870912"),
        (8, "--property=MemorySwapMax=0"), (9, "--property=TasksMax=256"),
        (10, "--property=RuntimeMaxSec=1200"), (11, "--property=KillMode=control-group"),
        (12, "--property=PrivateNetwork=yes"), (13, "--property=RestrictAddressFamilies=AF_UNIX"),
        (18, "--ignored"), (19, "--exact"), (20, "r8b_v8_m24_linked_child"), (21, "--nocapture"),
    ];
    reject(
        argv.len() != 22
            || fixed
                .iter()
                .any(|(index, expected)| argv[*index] != *expected)
            || argv[4] != format!("--unit={unit}"),
        "self_formed_r8b_systemd_run_argv_invalid",
    )?;
    let prefixes = [
        "--property=LoadCredential=r8b-producer-request:",
        "--property=StandardOutput=file:",
        "--property=StandardError=file:",
        "",
    ];
    let paths = (14..=17)
        .zip(prefixes)
        .map(|(index, prefix)| argv[index].strip_prefix(prefix).unwrap_or_default())
        .collect::<Vec<_>>();
    reject(
        paths.iter().any(|path| {
            path.is_empty() || !Path::new(path).is_absolute() || path.len() > MAX_PATH_BYTES_V3
        }) || paths.iter().collect::<BTreeSet<_>>().len() != paths.len(),
        "self_formed_r8b_systemd_run_path_invalid",
    )
}

pub fn validate_self_formed_r8b_manager_identity_v3(
    value: &K2UncertaintyR8BManagerIdentityV3,
) -> K2CompositionResultV1<()> {
    reject(
        value.bus_peer_pid == 0
            || value.bus_peer_pid != value.main_pid
            || !value.pidfd_alive
            || value.start_ticks == 0
            || value.uid == 0
            || !value.bus_unique_name.starts_with(':')
            || value.boot_id.is_empty()
            || value.command != [SYSTEMD_MANAGER_V3.to_owned(), "--user".to_owned()]
            || value.exec_start != SYSTEMD_MANAGER_V3
            || value.user_unit != format!("user@{}.service", value.uid)
            || value.invocation_id.is_empty()
            || !Path::new(&value.fragment_path).is_absolute()
            || !Path::new(&value.control_group).is_absolute()
            || !value.cgroup.starts_with(&value.control_group)
            || !value.version.starts_with("systemd 259"),
        "self_formed_r8b_manager_identity_invalid",
    )
}

pub fn validate_self_formed_r8b_privileged_probe_v3(
    value: &K2UncertaintyR8BPrivilegedProbeV3,
    pid: u32,
    pinned_systemd_sha256: &str,
) -> K2CompositionResultV1<()> {
    require_roots([
        &value.sudo_sha256,
        &value.sha256sum_sha256,
        &value.stdout_sha256,
        &value.stderr_sha256,
        &value.live_image_sha256,
    ])?;
    let argv = vec![
        SUDO_V3.to_owned(),
        "--non-interactive".to_owned(),
        "--user=root".to_owned(),
        "--".to_owned(),
        SHA256SUM_V3.to_owned(),
        "--binary".to_owned(),
        "--zero".to_owned(),
        format!("/proc/{pid}/exe"),
    ];
    reject(
        value.argv != argv
            || value.exit_code != 0
            || value.stdout_byte_len != 65 + format!(" */proc/{pid}/exe").len() as u64
            || value.stderr_byte_len != 0
            || value.stderr_sha256 != composition_sha256_bytes_v1(&[])
            || value.live_image_sha256 != pinned_systemd_sha256
            || value.started_monotonic_ns >= value.finished_monotonic_ns,
        "self_formed_r8b_privileged_probe_invalid",
    )
}

fn validate_unit_resource_v3(
    value: &K2UncertaintyR8BUnitResourceObservationV3,
    unit: &str,
) -> K2CompositionResultV1<()> {
    let duration = value
        .route_finished_monotonic_ns
        .checked_sub(value.route_started_monotonic_ns);
    reject(
        value.unit != unit
            || value.stop_target != unit
            || value.invocation_id.is_empty()
            || value.main_pid == 0
            || value.exec_main_code != "exited"
            || value.exec_main_status != 0
            || value.active_state != "active"
            || value.sub_state != "exited"
            || !value.metrics_frozen_while_loaded
            || value.memory_peak > 536_870_912
            || value.memory_swap_peak != 0
            || value.oom_policy.is_empty()
            || value.oom_kills != 0
            || value.tasks_current > 256
            || duration.is_none_or(|elapsed| elapsed > 1_200_000_000_000)
            || value.stop_exit_code != 0
            || !value.inactive_after_stop
            || value.descendants_after_stop != 0,
        "self_formed_r8b_unit_resource_invalid",
    )
}

pub fn seal_self_formed_r8b_process_event_v3(
    mut event: K2UncertaintyR8BProcessEventV3,
) -> K2CompositionResultV1<K2UncertaintyR8BProcessEventV3> {
    event.schema = K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V3.to_owned();
    event.event_root_sha256.clear();
    event.event_root_sha256 = uncertainty_root_v1(&event)?;
    validate_self_formed_r8b_process_event_v3(&event)?;
    Ok(event)
}

pub fn validate_self_formed_r8b_producer_request_v3(
    request: &K2UncertaintyR8BProducerRequestV3,
) -> K2CompositionResultV1<()> {
    for root in [
        &request.route_id_sha256,
        &request.producer_executable_sha256,
        &request.schedule_grammar_root_sha256,
    ] {
        require_composition_root_v1(root)?;
    }
    reject(
        request.schema != K2_UNCERTAINTY_R8B_PRODUCER_REQUEST_SCHEMA_V3
            || !short_process_identifier_v3(&request.producer_role)
            || !short_process_identifier_v3(&request.test_selector),
        "self_formed_r8b_v3_request_identity_invalid",
    )?;
    let mut input_roles = BTreeSet::new();
    let mut input_paths = BTreeSet::new();
    for input in &request.inputs {
        require_composition_root_v1(&input.content_sha256)?;
        require_composition_root_v1(&input.semantic_root_sha256)?;
        let expected_mode = match input.role {
            InputRole::DevelopmentSeed | InputRole::LinkedManifest | InputRole::SuiteManifest => {
                0o400
            }
            InputRole::FixtureTree => 0o500,
            InputRole::ProcessLedger => 0o600,
            InputRole::ExclusiveOutput => 0o700,
        };
        reject(
            !bounded_process_path_v3(&input.canonical_path)
                || input.unix_mode != expected_mode
                || input.byte_len == 0
                || !input_roles.insert(input.role)
                || !input_paths.insert(input.canonical_path.as_str()),
            "self_formed_r8b_v3_input_binding_invalid",
        )?;
    }
    let required_inputs = [
        InputRole::DevelopmentSeed,
        InputRole::FixtureTree,
        InputRole::LinkedManifest,
        InputRole::SuiteManifest,
        InputRole::ProcessLedger,
        InputRole::ExclusiveOutput,
    ]
    .into_iter()
    .collect();
    reject(
        input_roles != required_inputs,
        "self_formed_r8b_v3_input_set_invalid",
    )?;
    let mut output_paths = BTreeSet::new();
    for output in &request.outputs {
        validate_output_contract_v3(output, false)?;
        reject(
            output.producer_role != request.producer_role
                || output.producer_executable_sha256 != request.producer_executable_sha256
                || !output_paths.insert(output.relative_path.as_str()),
            "self_formed_r8b_v3_output_owner_invalid",
        )?;
    }
    let mut invocation_ids = BTreeSet::new();
    for invocation in &request.invocation_plan {
        validate_invocation_plan_v3(invocation)?;
        reject(
            !invocation_ids.insert(invocation.invocation_id_sha256.as_str()),
            "self_formed_r8b_v3_invocation_duplicate",
        )?;
    }
    reject(
        request.invocation_plan.iter().any(|invocation| {
            invocation
                .parent_invocation_id_sha256
                .as_ref()
                .is_some_and(|parent| !invocation_ids.contains(parent.as_str()))
        }),
        "self_formed_r8b_v3_invocation_parent_missing",
    )?;
    validate_exact_producer_plan_v3(request)?;
    projection::validate_producer_shape_v3(request)?;
    let mut canonical = request.clone();
    canonical.request_root_sha256.clear();
    reject(
        request.request_root_sha256 != uncertainty_root_v1(&canonical)?,
        "self_formed_r8b_v3_request_root_invalid",
    )
}

pub fn validate_self_formed_r8b_process_event_v3(
    event: &K2UncertaintyR8BProcessEventV3,
) -> K2CompositionResultV1<()> {
    for root in [
        &event.previous_event_root_sha256,
        &event.route_id_sha256,
        &event.request_root_sha256,
        &event.stdin_sha256,
    ] {
        require_composition_root_v1(root)?;
    }
    if let Some(root) = &event.stdout_sha256 {
        require_composition_root_v1(root)?;
    }
    if let Some(root) = &event.stderr_sha256 {
        require_composition_root_v1(root)?;
    }
    validate_invocation_plan_v3(&event.invocation)?;
    let requested = event.completion.is_none();
    let terminal_fields = [
        event.started_event_root_sha256.is_some(),
        event.exit_code.is_some(),
        event.stdout_byte_len.is_some(),
        event.stdout_sha256.is_some(),
        event.stderr_byte_len.is_some(),
        event.stderr_sha256.is_some(),
    ];
    reject(
        event.schema != K2_UNCERTAINTY_R8B_PROCESS_EVENT_SCHEMA_V3
            || (requested && terminal_fields.iter().any(|value| *value))
            || (!requested && terminal_fields.iter().any(|value| !*value))
            || (requested && event.validated_output.is_some())
            || event
                .stdout_byte_len
                .is_some_and(|bytes| bytes > MAX_STDOUT_BYTES_V3)
            || event
                .stderr_byte_len
                .is_some_and(|bytes| bytes > MAX_STDERR_BYTES_V3),
        "self_formed_r8b_v3_process_event_shape_invalid",
    )?;
    if let Some(completion) = event.completion {
        let authority = completion == Completion::AuthoritySuccess;
        let diagnostic_exit_matches = event
            .invocation
            .expected_exit_predicate
            .as_ref()
            .is_some_and(|predicate| event.exit_code == Some(predicate.exact_exit_code));
        reject(
            authority != event.validated_output.is_some()
                || (authority && event.exit_code != Some(0))
                || (completion == Completion::DiagnosticExpectedFailure
                    && !diagnostic_exit_matches)
                || (authority
                    && event.invocation.expected_outcome != ExpectedOutcome::AuthoritySuccess)
                || (completion == Completion::DiagnosticExpectedFailure
                    && event.invocation.expected_outcome
                        != ExpectedOutcome::DiagnosticExpectedFailure),
            "self_formed_r8b_v3_completion_authority_invalid",
        )?;
    }
    if let Some(output) = &event.validated_output {
        for root in [
            &output.stdout_sha256,
            &output.semantic_root_sha256,
            &output.validator_executable_sha256,
        ] {
            require_composition_root_v1(root)?;
        }
        reject(
            output.stdout_byte_len > MAX_STDOUT_BYTES_V3
                || event.stdout_byte_len != Some(output.stdout_byte_len)
                || event.stdout_sha256.as_ref() != Some(&output.stdout_sha256)
                || !short_process_identifier_v3(&output.receipt_schema)
                || output.validator != event.invocation.validator
                || output.authority_outputs.len() > MAX_AUTHORITY_OUTPUTS_V3,
            "self_formed_r8b_v3_validated_output_invalid",
        )?;
        for descriptor in &output.authority_outputs {
            validate_output_contract_v3(descriptor, true)?;
            reject(
                descriptor.producer_role != event.invocation.target_role
                    || descriptor.producer_executable_sha256
                        != event.invocation.target_executable_sha256,
                "self_formed_r8b_v3_authority_output_owner_invalid",
            )?;
        }
        match (
            &output.fact,
            output.validator,
            event.invocation.target_role.as_str(),
        ) {
            (
                ValidatedFact::RepresentativeCount { count },
                Validator::RepresentativeCount,
                "M04_PROBE",
            ) if (8..=1792).contains(count) => {}
            (ValidatedFact::None, validator, _) if validator != Validator::RepresentativeCount => {}
            _ => return Err(invalid("self_formed_r8b_v3_validated_fact_invalid")),
        }
    }
    let mut canonical = event.clone();
    canonical.event_root_sha256.clear();
    reject(
        event.event_root_sha256 != uncertainty_root_v1(&canonical)?
            || uncertainty_bytes_v1(event)?.len() > MAX_EVENT_BYTES_V3,
        "self_formed_r8b_v3_process_event_root_invalid",
    )
}

fn validate_output_contract_v3(
    output: &K2UncertaintyR8BOutputContractV3,
    require_attestation: bool,
) -> K2CompositionResultV1<()> {
    require_composition_root_v1(&output.producer_executable_sha256)?;
    for root in &output.required_source_roots_sha256 {
        require_composition_root_v1(root)?;
    }
    if let Some(value) = &output.file_attestation {
        require_composition_root_v1(&value.content_sha256)?;
        require_composition_root_v1(&value.semantic_root_sha256)?;
    }
    let evidence = output.object_role == ObjectRole::Evidence;
    reject(
        !valid_composition_path_v1(&output.relative_path)
            || output.relative_path.len() > MAX_PATH_BYTES_V3
            || !short_process_identifier_v3(&output.receipt_schema)
            || !short_process_identifier_v3(&output.producer_role)
            || require_attestation != output.file_attestation.is_some()
            || output
                .file_attestation
                .as_ref()
                .is_some_and(|value| value.byte_len == 0 || value.unix_mode != 0o400)
            || evidence != output.evidence_kind.is_some()
            || (evidence
                && output.required_denominator
                    != output.evidence_kind.and_then(EvidenceKind::required))
            || (output.object_role == ObjectRole::DownstreamInvocationContract
                && (output.evidence_kind.is_some()
                    || output.validator != Validator::DownstreamInvocationContract))
            || output
                .required_source_roots_sha256
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != output.required_source_roots_sha256.len(),
        "self_formed_r8b_v3_output_contract_invalid",
    )
}

fn validate_invocation_plan_v3(
    invocation: &K2UncertaintyR8BInvocationPlanV3,
) -> K2CompositionResultV1<()> {
    for root in [
        Some(&invocation.invocation_id_sha256),
        invocation.parent_invocation_id_sha256.as_ref(),
        Some(&invocation.request_owner_executable_sha256),
        Some(&invocation.target_executable_sha256),
        invocation.case_id_sha256.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        require_composition_root_v1(root)?;
    }
    let expected_tools: &[(ToolRole, &str)] = match invocation.launch_kind {
        LaunchKind::Direct => &[],
        LaunchKind::StraceMediated => &[(ToolRole::Strace, STRACE_V3)],
        LaunchKind::BwrapPrlimitMediated => {
            &[(ToolRole::Bwrap, BWRAP_V3), (ToolRole::Prlimit, PRLIMIT_V3)]
        }
        LaunchKind::UserSystemd => &[(ToolRole::SystemdRun, SYSTEMD_RUN_V3)],
    };
    for tool in &invocation.tool_chain {
        require_composition_root_v1(&tool.sha256)?;
        reject(
            !bounded_process_path_v3(&tool.canonical_path),
            "self_formed_r8b_v3_tool_identity_invalid",
        )?;
    }
    let exit_predicate_valid = match (
        invocation.expected_outcome,
        invocation.expected_exit_predicate.as_ref(),
    ) {
        (ExpectedOutcome::AuthoritySuccess, None) => true,
        (ExpectedOutcome::DiagnosticExpectedFailure, Some(predicate)) => {
            (1..=255).contains(&predicate.exact_exit_code)
        }
        _ => false,
    };
    reject(
        !valid_r8b_role_v3(&invocation.request_owner_role)
            || !valid_r8b_role_v3(&invocation.target_role)
            || !short_process_identifier_v3(&invocation.stage)
            || !exit_predicate_valid
            || invocation
                .tool_chain
                .iter()
                .map(|tool| (tool.role, tool.canonical_path.as_str()))
                .ne(expected_tools.iter().copied()),
        "self_formed_r8b_v3_invocation_plan_invalid",
    )
}

type PlanShapeV3 = (
    &'static str,
    &'static str,
    &'static str,
    LaunchKind,
    ExpectedOutcome,
    Option<i32>,
    Validator,
    bool,
    bool,
    usize,
);

fn validate_exact_producer_plan_v3(
    request: &K2UncertaintyR8BProducerRequestV3,
) -> K2CompositionResultV1<()> {
    use ExpectedOutcome::{AuthoritySuccess as A, DiagnosticExpectedFailure as D};
    use LaunchKind::{BwrapPrlimitMediated as B, Direct as X, StraceMediated as S};
    use Validator::ConcreteReceipt as C;
    #[rustfmt::skip]
    let expected: &[PlanShapeV3] = match request.producer_role.as_str() {
        "S01_CRATE_UNIT" => &[],
        "S02_RESTART" => &[
            ("S02_RESTART", "M01_DEVELOPMENT_OWNER", "C02", S, A, None, C, false, false, 6),
            ("S02_RESTART", "M01_DEVELOPMENT_OWNER", "C02", S, D, Some(1), C, false, false, 4),
            ("S02_RESTART", "M02_GENERATOR", "C02", B, A, None, C, false, false, 3),
            ("M01_DEVELOPMENT_OWNER", "M02_GENERATOR", "C02", B, A, None, C, false, false, 3),
        ],
        "S03_MODE_MATRIX" => &[
            ("S03_MODE_MATRIX", "M01_DEVELOPMENT_OWNER", "C03", X, A, None, C, false, false, 1),
            ("S03_MODE_MATRIX", "M01_DEVELOPMENT_OWNER", "C03", X, D, Some(1), C, false, false, 1),
            ("M01_DEVELOPMENT_OWNER", "M02_GENERATOR", "C02", B, A, None, C, false, false, 1),
            ("S03_MODE_MATRIX", "M18_TERMINAL_EVALUATOR", "C03", X, A, None, C, false, false, 3),
        ],
        "S04_CLEANUP_NEGATIVE" => &[
            ("S04_CLEANUP_NEGATIVE", "M01_DEVELOPMENT_OWNER", "C04", X, A, None, C, false, false, 1),
            ("M01_DEVELOPMENT_OWNER", "M02_GENERATOR", "C02", B, A, None, C, false, false, 1),
            ("S04_CLEANUP_NEGATIVE", "M18_TERMINAL_EVALUATOR", "C04", X, A, None, C, false, false, 1),
            ("S04_CLEANUP_NEGATIVE", "M20_CLEANUP_AUTHORIZER", "C04", B, A, None, C, false, false, 1),
            ("S04_CLEANUP_NEGATIVE", "M21_CLEANUP_OWNER", "C04", B, A, None, C, false, false, 1),
            ("S04_CLEANUP_NEGATIVE", "M22_CLEANUP_VERIFIER", "C04", B, A, None, C, false, false, 1),
        ],
        "S05_AUTHORITY_PUBLICATION" => &[
            ("S05_AUTHORITY_PUBLICATION", "M26_R8B_PUBLISHER", "C05", X, D, Some(1), C, false, false, 2),
        ],
        "M24_LINKED_RUNNER" => &[
            ("M24_LINKED_RUNNER", "M01_DEVELOPMENT_OWNER", "C01", X, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M10_PUBLIC_COORDINATOR", "C06", X, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M11_PRIVATE_RESOLVER", "C09", B, A, None, C, true, true, 24),
            ("M24_LINKED_RUNNER", "M12_SAFETY", "C09", B, A, None, C, true, true, 24),
            ("M24_LINKED_RUNNER", "M13_WORKER", "C09", B, A, None, C, true, true, 24),
            ("M24_LINKED_RUNNER", "M14_OBSERVER", "C09", B, A, None, C, true, true, 24),
            ("M24_LINKED_RUNNER", "M15_FINAL_VERIFIER", "C09", B, A, None, C, true, false, 16),
            ("M24_LINKED_RUNNER", "M16_ORACLE", "C10", B, A, None, C, true, false, 16),
            ("M24_LINKED_RUNNER", "M19_FRESH_CONTROL_CASE", "C11", B, A, None, C, true, false, 12),
            ("M24_LINKED_RUNNER", "M17_CONTROL_EVALUATOR", "C12", X, A, None, C, false, false, 4),
            ("M24_LINKED_RUNNER", "M18_TERMINAL_EVALUATOR", "C14", X, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M20_CLEANUP_AUTHORIZER", "C16", B, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M21_CLEANUP_OWNER", "C17", B, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M22_CLEANUP_VERIFIER", "C18", B, A, None, C, false, false, 1),
            ("M24_LINKED_RUNNER", "M23_DEVELOPMENT_RESULT_PUBLISHER", "C20", B, A, None, C, false, false, 1),
        ],
        _ => return Err(invalid("self_formed_r8b_v3_producer_role_invalid")),
    };
    let mut counts = vec![0_usize; expected.len()];
    let mut target_hashes = BTreeMap::new();
    let mut tool_hashes = BTreeMap::new();
    let mut nested_parents = BTreeSet::new();
    for row in &request.invocation_plan {
        let index = expected
            .iter()
            .position(|shape| plan_shape_matches_v3(row, shape))
            .ok_or_else(|| invalid("self_formed_r8b_v3_producer_plan_substitution"))?;
        counts[index] += 1;
        reject(
            target_hashes
                .insert(
                    row.target_role.as_str(),
                    row.target_executable_sha256.as_str(),
                )
                .is_some_and(|hash| hash != row.target_executable_sha256.as_str()),
            "self_formed_r8b_v3_target_identity_drift",
        )?;
        for tool in &row.tool_chain {
            reject(
                tool_hashes
                    .insert(tool.canonical_path.as_str(), tool.sha256.as_str())
                    .is_some_and(|hash| hash != tool.sha256.as_str()),
                "self_formed_r8b_v3_tool_identity_drift",
            )?;
        }
        let parent = row.parent_invocation_id_sha256.as_ref().and_then(|id| {
            request
                .invocation_plan
                .iter()
                .find(|candidate| &candidate.invocation_id_sha256 == id)
        });
        if row.request_owner_role == request.producer_role {
            reject(
                parent.is_some()
                    || row.request_owner_executable_sha256 != request.producer_executable_sha256,
                "self_formed_r8b_v3_producer_writer_partition_invalid",
            )?;
        } else {
            reject(
                row.request_owner_role != "M01_DEVELOPMENT_OWNER"
                    || parent.is_none_or(|value| {
                        value.request_owner_role != request.producer_role
                            || value.target_role != "M01_DEVELOPMENT_OWNER"
                            || value.expected_outcome != ExpectedOutcome::AuthoritySuccess
                            || value.target_executable_sha256 != row.request_owner_executable_sha256
                    })
                    || !nested_parents.insert(
                        row.parent_invocation_id_sha256
                            .as_deref()
                            .unwrap_or_default(),
                    ),
                "self_formed_r8b_v3_nested_writer_partition_invalid",
            )?;
        }
    }
    reject(
        counts
            .iter()
            .zip(expected)
            .any(|(count, shape)| *count != shape.9),
        "self_formed_r8b_v3_producer_plan_cardinality_invalid",
    )
}

fn plan_shape_matches_v3(row: &K2UncertaintyR8BInvocationPlanV3, shape: &PlanShapeV3) -> bool {
    row.request_owner_role == shape.0
        && row.target_role == shape.1
        && row.stage == shape.2
        && row.launch_kind == shape.3
        && row.expected_outcome == shape.4
        && row
            .expected_exit_predicate
            .as_ref()
            .map(|value| value.exact_exit_code)
            == shape.5
        && row.validator == shape.6
        && row.case_id_sha256.is_some() == shape.7
        && row.probe_ordinal.is_some() == shape.8
}

fn bounded_process_path_v3(path: &str) -> bool {
    !path.is_empty() && path.len() <= MAX_PATH_BYTES_V3 && Path::new(path).is_absolute()
}

fn short_process_identifier_v3(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_PROCESS_IDENTIFIER_BYTES_V3 && value.is_ascii()
}

fn require_roots<'a>(roots: impl IntoIterator<Item = &'a String>) -> K2CompositionResultV1<()> {
    roots
        .into_iter()
        .try_for_each(|root| require_composition_root_v1(root))
}
fn reject(condition: bool, reason: &'static str) -> K2CompositionResultV1<()> {
    (!condition).then_some(()).ok_or_else(|| invalid(reason))
}
fn invalid(reason: &'static str) -> K2CompositionErrorV1 {
    K2CompositionErrorV1::Invalid(reason)
}
