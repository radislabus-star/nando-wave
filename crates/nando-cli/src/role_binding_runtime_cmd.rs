use nando_core::wave::{
    WavePredictorActiveCenter, WavePredictorHebbianConfig, WavePredictorHebbianField,
    WavePredictorRoleBindingEvalTask, WavePredictorRoleBindingOffloadAction,
    WavePredictorRoleBindingOffloadPolicy, WavePredictorRoleBindingOffloadRuntime,
    WavePredictorRoleBindingPackageInfo, WavePredictorRoleBindingPreparedFringe,
};
use nando_core::{SURFACE_WAVE_DIM, SurfaceWave4096};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT: &str =
    "target/nando-wave/slot32-role-binding/role-binding-release-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-registry-v1.json";
const DEFAULT_ROLE_BINDING_PROFILE_RUNTIME_SMOKE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-runtime-smoke-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT: &str = "target/nando-wave/slot32-role-binding/role-binding-binary-eval-pack-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_REPLAY_SUITE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-replay-suite-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_FALLBACK_SMOKE_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-fallback-smoke-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_WORKER_SCALING_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-worker-scaling-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_WORKER_REPLAY_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-worker-replay-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_LB_REPLAY_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-lb-replay-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_PROFILE_LB_THROUGHPUT_REPORT: &str =
    "target/nando-wave/role-binding-profile-runtime/profile-lb-throughput-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-shadow-v1.trace.jsonl";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_SHADOW_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-shadow-v1.product-proof.json";
const DEFAULT_ROLE_BINDING_REAL_TRAFFIC_INGEST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/real-traffic-ingest-v1.report.json";
const DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_EVENTS_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-events-v1.events.jsonl";
const DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_INGEST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-events-v1.report.json";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_EVENTS_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.events.jsonl";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.report.json";
const DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_SHADOW_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/codex-history-route-candidates-v1.shadow-report.json";
const DEFAULT_REAL_TRAFFIC_CPU_ROUTE_FORECAST_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/cpu-route-forecast-v1.report.json";
const DEFAULT_REAL_TRAFFIC_ROUTE_GAP_CATALOG_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/route-gap-catalog-v1.report.json";
const DEFAULT_REAL_TRAFFIC_ROUTE_GAP_CATALOG_AGENT_CONTROL_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/route-gap-catalog-agent-control-v1.report.json";
const DEFAULT_REAL_TRAFFIC_ROUTE_GAP_PAYLOAD_READINESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/route-gap-payload-readiness-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.trace.jsonl";
const DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-payload-dry-run-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_PACKAGE_PATH: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-seed0.nwrb";
const DEFAULT_PLANNING_NEXT_STEP_PROFILE_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-planning-next-step-v1.json";
const DEFAULT_PLANNING_NEXT_STEP_PROFILE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-profile-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.trace.jsonl";
const DEFAULT_PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-output-evidence-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.trace.jsonl";
const DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_REPORT: &str = "target/nando-wave/real-traffic-shadow/planning-next-step-local-accept-calibration-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/planning-next-step-artifact-progress-v1.verification-hook-audit.report.json";
const DEFAULT_REAL_TRAFFIC_FEEDBACK_LOOP_EXTENDED_REPORT: &str = "target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-conditional-agent-control-v1.report.json";
const DEFAULT_REAL_TRAFFIC_CPU_OPERATOR_CATALOG_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/cpu-operator-catalog-v1.report.json";
const DEFAULT_AGENT_CONTROL_PACKAGE_PATH: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-seed0.nwrb";
const DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-agent-control-v1.json";
const DEFAULT_AGENT_CONTROL_PROFILE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-profile-v1.report.json";
const DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-payload-dry-run-v1.trace.jsonl";
const DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-payload-dry-run-v1.report.json";
const DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.trace.jsonl";
const DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.report.json";
const DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/agent-control-output-evidence-v1.verification-hook-audit.report.json";
const DEFAULT_AGENT_CONTROL_ADMISSION_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-admission-calibration-v1.report.json";
const DEFAULT_AGENT_CONTROL_SAFE_POLICY_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.trace.jsonl";
const DEFAULT_AGENT_CONTROL_SAFE_POLICY_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.report.json";
const DEFAULT_AGENT_CONTROL_SAFE_POLICY_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/agent-control-safe-policy-v1.verification-hook-audit.report.json";
const DEFAULT_EDIT_PAYLOAD_READINESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-payload-readiness-v1.report.json";
const DEFAULT_EDIT_PAYLOAD_DRY_RUN_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.trace.jsonl";
const DEFAULT_EDIT_PAYLOAD_DRY_RUN_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.report.json";
const DEFAULT_EDIT_PAYLOAD_DRY_RUN_SHADOW_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-payload-dry-run-v1.shadow-report.json";
const DEFAULT_CONDITIONAL_PAYLOAD_READINESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/conditional-payload-readiness-v1.report.json";
const DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.trace.jsonl";
const DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/conditional-payload-dry-run-v1.report.json";
const DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.trace.jsonl";
const DEFAULT_EDIT_OUTPUT_EVIDENCE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-output-evidence-v1.report.json";
const DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/conditional-output-evidence-v1.trace.jsonl";
const DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/conditional-output-evidence-v1.report.json";
const DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/conditional-output-evidence-v1.verification-hook-audit.report.json";
const DEFAULT_CONDITIONAL_SAFE_POLICY_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-conditional-safe-policy-v1.json";
const DEFAULT_CONDITIONAL_SAFE_POLICY_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.trace.jsonl";
const DEFAULT_CONDITIONAL_SAFE_POLICY_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.report.json";
const DEFAULT_CONDITIONAL_SAFE_POLICY_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/conditional-safe-policy-v1.verification-hook-audit.report.json";
const DEFAULT_MIXED_PAYLOAD_READINESS_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-payload-readiness-v1.report.json";
const DEFAULT_MIXED_PAYLOAD_DRY_RUN_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/mixed-payload-dry-run-v1.trace.jsonl";
const DEFAULT_MIXED_PAYLOAD_DRY_RUN_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-payload-dry-run-v1.report.json";
const DEFAULT_MIXED_OUTPUT_EVIDENCE_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1.trace.jsonl";
const DEFAULT_MIXED_OUTPUT_EVIDENCE_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1.report.json";
const DEFAULT_MIXED_OUTPUT_EVIDENCE_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/mixed-output-evidence-v1.verification-hook-audit.report.json";
const DEFAULT_MIXED_LOCAL_ACCEPT_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-local-accept-calibration-v1.report.json";
const DEFAULT_MIXED_SAFE_POLICY_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-mixed-safe-policy-v1.json";
const DEFAULT_MIXED_SAFE_POLICY_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.trace.jsonl";
const DEFAULT_MIXED_SAFE_POLICY_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.report.json";
const DEFAULT_MIXED_SAFE_POLICY_AUDIT_REPORT: &str = "target/nando-wave/real-traffic-shadow/mixed-safe-policy-v1.verification-hook-audit.report.json";
const DEFAULT_MIXED_SAFE_POLICY_V2_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-mixed-safe-policy-v2.json";
const DEFAULT_MIXED_SAFE_POLICY_V2_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.trace.jsonl";
const DEFAULT_MIXED_SAFE_POLICY_V2_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/mixed-safe-policy-v2.report.json";
const DEFAULT_EDIT_LOCAL_ACCEPT_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-local-accept-calibration-v1.report.json";
const DEFAULT_EDIT_SAFE_POLICY_REGISTRY_CONFIG: &str =
    "target/nando-wave/real-traffic-shadow/profile-registry-edit-safe-policy-v1.json";
const DEFAULT_EDIT_SAFE_POLICY_TRACE_JSONL: &str =
    "target/nando-wave/real-traffic-shadow/edit-safe-policy-v1.trace.jsonl";
const DEFAULT_EDIT_SAFE_POLICY_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-safe-policy-v1.report.json";
const DEFAULT_EDIT_SAFE_POLICY_AUDIT_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-safe-policy-v1.verification-hook-audit.report.json";
const DEFAULT_CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/conditional-local-accept-calibration-v1.report.json";
const DEFAULT_EDIT_ADMISSION_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/edit-admission-calibration-v1.report.json";
const DEFAULT_PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/planning-next-step-admission-calibration-v1.report.json";
const DEFAULT_REAL_TRAFFIC_VERIFICATION_HOOK_AUDIT_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/verification-hook-audit-v1.report.json";
const DEFAULT_REAL_TRAFFIC_FEEDBACK_LOOP_REPORT: &str =
    "target/nando-wave/real-traffic-shadow/cpu-route-feedback-loop-v1.report.json";
const DEFAULT_REAL_TRAFFIC_MIN_SAFE_POLICY_TRUE_SUPPORT: usize = 3;
const ROLE_BINDING_EVAL_PACK_BINARY_MAGIC: [u8; 8] = *b"NWRE0001";
const HTTP_READ_TIMEOUT_SECS: u64 = 10;
const MAX_HTTP_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE: usize = 128;
const DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES: usize = 4;
const DEFAULT_PROFILE_WORKER_COUNT: usize = 2;
const DEFAULT_THROUGHPUT_CLIENT_THREADS: usize = 4;
const DEFAULT_THROUGHPUT_SEQUENCE_REPETITIONS: usize = 1;
const REAL_TRAFFIC_EDIT_PAGE_BITS: u32 = 12;
const REAL_TRAFFIC_EDIT_PAGE_SIZE: u32 = 4096;
const REAL_TRAFFIC_EDIT_ROLE_BASE: u32 = 0;
const REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT: u8 = 16;
const REAL_TRAFFIC_EDIT_DEMO_PAGE: u32 = 18;
const REAL_TRAFFIC_EDIT_DEMO_BASE: u32 = REAL_TRAFFIC_EDIT_DEMO_PAGE << REAL_TRAFFIC_EDIT_PAGE_BITS;
const REAL_TRAFFIC_EDIT_OUTPUT_SLOT_COUNT: u8 = 17;
const REAL_TRAFFIC_EDIT_OPERATOR_PAIR_SHIFT: u32 = 5;
const REAL_TRAFFIC_EDIT_TOP_ROLE_L1_LANES: usize = 32;
const REAL_TRAFFIC_EDIT_STATE_DELTA_LANES_PER_SIDE: usize = 24;
const REAL_TRAFFIC_EDIT_END_TOKEN: &str = "__EDIT_END__";
const REAL_TRAFFIC_CONDITIONAL_PAGE_SIZE: u32 = 4096;
const REAL_TRAFFIC_CONDITIONAL_ROLE_BASE: u32 = 0;
const REAL_TRAFFIC_CONDITIONAL_OPERATOR_PAIR_BASE: u32 = 35 << 12;
const REAL_TRAFFIC_CONDITIONAL_CONDITION_ROLE_SLOT: u8 = 0;
const REAL_TRAFFIC_CONDITIONAL_EVIDENCE_ROLE_SLOT: u8 = 1;
const REAL_TRAFFIC_CONDITIONAL_ALLOWED_ROLE_SLOT: u8 = 2;
const REAL_TRAFFIC_CONDITIONAL_REFUSED_ROLE_SLOT: u8 = 3;
const REAL_TRAFFIC_CONDITIONAL_OPERATOR_PAIR_SHIFT: u32 = 5;
const REAL_TRAFFIC_CONDITIONAL_TOP_ROLE_L1_LANES: usize = 32;
const REAL_TRAFFIC_CONDITIONAL_STATE_DELTA_LANES_PER_SIDE: usize = 24;
const REAL_TRAFFIC_MIXED_PAGE_SIZE: u32 = 4096;
const REAL_TRAFFIC_MIXED_ROLE_BASE: u32 = 0;
const REAL_TRAFFIC_MIXED_OPERATOR_PAIR_BASE: u32 = 33 << 12;
const REAL_TRAFFIC_MIXED_SOURCE_ROLE_SLOT: u8 = 0;
const REAL_TRAFFIC_MIXED_DESTINATION_ROLE_SLOT: u8 = 1;
const REAL_TRAFFIC_MIXED_ACTION_ROLE_SLOT: u8 = 2;
const REAL_TRAFFIC_MIXED_INVARIANT_ROLE_SLOT: u8 = 3;
const REAL_TRAFFIC_MIXED_OPERATOR_PAIR_SHIFT: u32 = 5;
const REAL_TRAFFIC_MIXED_TOP_ROLE_L1_LANES: usize = 32;
const REAL_TRAFFIC_MIXED_STATE_DELTA_LANES_PER_SIDE: usize = 24;
const REAL_TRAFFIC_PLANNING_PAGE_SIZE: u32 = 4096;
const REAL_TRAFFIC_PLANNING_ROLE_BASE: u32 = 0;
const REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_BASE: u32 = 37 << 12;
const REAL_TRAFFIC_PLANNING_GOAL_ROLE_SLOT: u8 = 0;
const REAL_TRAFFIC_PLANNING_STATE_ROLE_SLOT: u8 = 1;
const REAL_TRAFFIC_PLANNING_EVIDENCE_ROLE_SLOT: u8 = 2;
const REAL_TRAFFIC_PLANNING_NEXT_ACTION_ROLE_SLOT: u8 = 3;
const REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_SHIFT: u32 = 5;
const REAL_TRAFFIC_PLANNING_TOP_ROLE_L1_LANES: usize = 32;
const REAL_TRAFFIC_PLANNING_STATE_DELTA_LANES_PER_SIDE: usize = 24;
const REAL_TRAFFIC_PLANNING_ROUTE_KEY: &str = "planning_next_step";
const REAL_TRAFFIC_PLANNING_PROFILE_ID: &str = "route_gap_planning_next_step_profile_v1";
const REAL_TRAFFIC_PLANNING_WRONG_TOKEN: &str = "__PLANNING_WRONG_STEP__";
const REAL_TRAFFIC_PLANNING_DISABLED_THRESHOLD: i32 = i32::MAX;
const REAL_TRAFFIC_AGENT_CONTROL_ACTION_BASE: u32 = 0;
const REAL_TRAFFIC_AGENT_CONTROL_ACTION_COUNT: u32 = 4096;
const REAL_TRAFFIC_AGENT_CONTROL_ROLE_BASE: u32 = 4096;
const REAL_TRAFFIC_AGENT_CONTROL_ROLE_STRIDE: u32 = 4096;
const REAL_TRAFFIC_AGENT_CONTROL_ROLE_COUNT: u8 = 1;
const REAL_TRAFFIC_AGENT_CONTROL_ACTION_CENTER: u32 = REAL_TRAFFIC_AGENT_CONTROL_ACTION_BASE + 17;
const REAL_TRAFFIC_AGENT_CONTROL_INTENT_SLOT: u8 = 0;
const REAL_TRAFFIC_AGENT_CONTROL_OUTPUT_SLOT: u8 = 0;
const REAL_TRAFFIC_AGENT_CONTROL_THRESHOLD: i32 = 32_768;

pub(crate) fn run_role_binding_profile_registry_from_release_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let release_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_RELEASE_SUITE_REPORT));
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));

    let release_suite =
        read_json_file::<RoleBindingRuntimeReleaseSuiteReport>(&release_suite_path)?;
    if !release_suite.gate_pass {
        return Err(format!(
            "release-suite is not green: verdict={}",
            release_suite.verdict
        ));
    }

    let mut profiles = Vec::new();
    for row in release_suite.rows {
        profiles.push(RoleBindingProfileConfig {
            profile_id: role_binding_profile_id(&row.label, row.seed),
            profile_kind: "role_binding_nwrb".to_owned(),
            operator_classes: role_binding_operator_classes(&row.label),
            package_path: row.package_path,
            runtime_bytes_estimate: 0,
            edge_count: row.package_edge_count,
            slot_count: 32,
            threshold: row.margin_threshold,
            acceptance_policy: default_profile_acceptance_policy(),
            accepted_route_keys: vec![
                role_binding_profile_id(&row.label, row.seed),
                format!("{}-seed{}", row.label, row.seed),
            ],
        });
    }

    let config = RoleBindingProfileRegistryConfig {
        schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
        profiles,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "serving registry generated from a green release-suite report; it references only .nwrb runtime packages and does not load .nwreb eval packs in serving mode".to_owned(),
    };
    validate_registry_config(&config)?;
    write_json_file(&config_path, &config)?;

    println!(
        "role-binding-profile-registry-from-release-v1: ROLE_BINDING_PROFILE_REGISTRY_FROM_RELEASE_V1_PASS"
    );
    println!("  release_suite: {}", release_suite_path.display());
    println!("  registry_config: {}", config_path.display());
    println!("  profiles: {}", config.profiles.len());
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    Ok(())
}

pub(crate) fn run_role_binding_profile_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:18090".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;

    println!("role-binding-profile-serve-v1: serving .nwrb profile registry");
    println!("  bind_addr: {bind_addr}");
    println!("  registry_config: {}", config_path.display());
    println!("  profiles: {}", registry.profile_count());
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /profiles");
    println!("  endpoint: POST /score");
    println!("  endpoint: POST /score-compact");
    println!("  endpoint: POST /replay");
    println!("  endpoint: GET /metrics");
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    println!("  corpus_jsonl_loaded: false");

    let stats = serve_role_binding_profile_requests(
        listener,
        Arc::new(registry),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-profile-serve-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  score_requests: {}", stats.score_requests);
    println!("  replay_requests: {}", stats.replay_requests);
    println!("  false_local_accepts: {}", stats.false_local_accepts);
    Ok(())
}

pub(crate) fn run_role_binding_profile_lb_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "role-binding-profile-lb-serve-v1 requires lb-config-json".to_owned())?;
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:18190".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let load_balancer = RoleBindingProfileLoadBalancerRuntime::from_config_path(&config_path)?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind {bind_addr}: {error}"))?;

    println!("role-binding-profile-lb-serve-v1: serving profile load-balancer");
    println!("  bind_addr: {bind_addr}");
    println!("  lb_config: {}", config_path.display());
    println!("  upstreams: {}", load_balancer.upstream_count());
    println!("  profiles: {}", load_balancer.profile_count());
    println!("  endpoint: GET /health");
    println!("  endpoint: GET /profiles");
    println!("  endpoint: POST /score");
    println!("  endpoint: POST /replay");
    println!("  endpoint: GET /metrics");
    println!("  compiler_used: false");
    println!("  eval_packs_loaded: false");
    println!("  corpus_jsonl_loaded: false");

    let stats = serve_role_binding_profile_lb_requests(
        listener,
        Arc::new(load_balancer),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-profile-lb-serve-v1: stopped");
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  score_requests: {}", stats.score_requests);
    println!("  replay_requests: {}", stats.replay_requests);
    println!("  false_local_accepts: {}", stats.false_local_accepts);
    Ok(())
}

pub(crate) fn run_role_binding_profile_runtime_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_RUNTIME_SMOKE_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let local_score = registry
        .self_test_score_request()
        .ok_or_else(|| "registry has no scorable positive role-binding edge".to_owned())?;
    let fallback_score = RoleBindingProfileScoreRequest {
        request_id: "smoke_fallback".to_owned(),
        route_key: local_score.route_key.clone(),
        profile_id: local_score.profile_id.clone(),
        exact_cache_key: Some("smoke_fallback_cache_key".to_owned()),
        active_fringe: Vec::new(),
        slots: local_score.slots.clone(),
        expect_local_operator: Some(false),
    };
    let replay = RoleBindingProfileReplayRequest {
        request_id: "smoke_replay".to_owned(),
        requests: vec![local_score.clone(), fallback_score.clone()],
    };

    let bind_addr = reserve_local_bind_addr()?;
    let mut server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, Some(7))?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid smoke bind addr {bind_addr}: {error}"))?;
    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let profiles = send_json_get::<RoleBindingProfileProfilesResponse>(addr, "/profiles")?;
    let score = send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &local_score)?;
    let replay_response =
        send_json_post::<_, RoleBindingProfileReplayResponse>(addr, "/replay", &replay)?;
    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    server.wait()?;

    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && profiles.profile_count > 0
        && score.accepted
        && !score.fallback
        && score.energy_margin >= score.threshold
        && replay_response.exact_cache_incremental_reduction_milli >= 200
        && replay_response.false_local_accepts == 0
        && metrics.false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024;

    let report = RoleBindingProfileRuntimeSmokeReport {
        schema_version: "nando_role_binding_profile_runtime_smoke_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_RUNTIME_SMOKE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        profile_count: profiles.profile_count,
        endpoint_health_pass: health.status == "ok",
        endpoint_profiles_pass: profiles.profile_count > 0,
        endpoint_score_pass: score.accepted && !score.fallback,
        endpoint_replay_pass: replay_response.exact_cache_incremental_reduction_milli >= 200,
        endpoint_metrics_pass: metrics.false_local_accepts == 0,
        exact_cache_llm_calls: replay_response.exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls: replay_response.exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli: replay_response
            .exact_cache_incremental_reduction_milli,
        false_local_accepts: replay_response.false_local_accepts,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        claim_boundary: "HTTP serving smoke over .nwrb role-binding profiles: /health, /profiles, /score, /replay, /metrics, exact-cache comparison, counters, and latency/RSS reporting; not real Codex traffic, not .nwreb serving, not compiler/training/corpus in hot path".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-runtime-smoke-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile runtime smoke failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_replay_suite_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REPLAY_SUITE_REPORT));
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }

    let bind_addr = reserve_local_bind_addr()?;
    let _server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, None)?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid replay bind addr {bind_addr}: {error}"))?;
    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let profiles = send_json_get::<RoleBindingProfileProfilesResponse>(addr, "/profiles")?;

    let mut rows = Vec::new();
    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !registry
            .profiles
            .iter()
            .any(|profile| profile.config.profile_id == profile_id)
        {
            return Err(format!(
                "profile {} from binary suite is missing in serving registry",
                profile_id
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let _source_package_path = eval_pack.source_package_path.as_deref().unwrap_or("");
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut row_unique_sequences = 0usize;
        let mut row_exact_cache_llm_calls = 0usize;
        let mut row_exact_cache_plus_nando_llm_calls = 0usize;
        let mut row_false_local_accepts = 0usize;
        let mut row_missed_expected_local = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;

        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            row_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if row_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let response = send_replay_batch(addr, &profile_id, batch_index, &mut batch)?;
                total_http_replay_batches += 1;
                batch_index += 1;
                row_exact_cache_llm_calls += response.exact_cache_llm_calls;
                row_exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                row_false_local_accepts += response.false_local_accepts;
                row_missed_expected_local += response.missed_expected_local;
                local_operator_calls += response.local_operator_calls;
                fallback_to_llm_calls += response.fallback_to_llm_calls;
            }
        }
        if !batch.is_empty() {
            let response = send_replay_batch(addr, &profile_id, batch_index, &mut batch)?;
            total_http_replay_batches += 1;
            row_exact_cache_llm_calls += response.exact_cache_llm_calls;
            row_exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            row_false_local_accepts += response.false_local_accepts;
            row_missed_expected_local += response.missed_expected_local;
            local_operator_calls += response.local_operator_calls;
            fallback_to_llm_calls += response.fallback_to_llm_calls;
        }

        let row_reduction_milli = reduction_milli(
            row_exact_cache_llm_calls,
            row_exact_cache_plus_nando_llm_calls,
        );
        rows.push(RoleBindingProfileReplaySuiteRow {
            profile_id,
            label: suite_row.label.clone(),
            seed: suite_row.seed,
            binary_eval_pack_path: suite_row.binary_eval_pack_path.clone(),
            unique_sequences_replayed: row_unique_sequences,
            no_cache_llm_calls: row_unique_sequences * 2,
            exact_cache_llm_calls: row_exact_cache_llm_calls,
            exact_cache_plus_nando_llm_calls: row_exact_cache_plus_nando_llm_calls,
            exact_cache_incremental_reduction_milli: row_reduction_milli,
            false_local_accepts: row_false_local_accepts,
            missed_expected_local: row_missed_expected_local,
        });
        total_unique_sequences += row_unique_sequences;
        no_cache_llm_calls += row_unique_sequences * 2;
        exact_cache_llm_calls += row_exact_cache_llm_calls;
        exact_cache_plus_nando_llm_calls += row_exact_cache_plus_nando_llm_calls;
        false_local_accepts += row_false_local_accepts;
        missed_expected_local += row_missed_expected_local;
    }

    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && profiles.profile_count == registry.profile_count()
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024
        && metrics.p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileReplaySuiteReport {
        schema_version: "nando_role_binding_profile_replay_suite_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_REPLAY_SUITE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        profile_count: profiles.profile_count,
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded_in_serving_worker: false,
        corpus_jsonl_loaded_in_serving_worker: false,
        eval_packs_used_by_replay_client: true,
        python_demo_used: false,
        target_center_id_training_used: false,
        proof_rule_id_training_authority_used: false,
        concrete_x_lookup_used: false,
        local_out_t_runtime_extension_used: false,
        rows,
        claim_boundary: "HTTP replay-suite over the serving-only .nwrb role-binding profile runtime. .nwreb eval packs are read only by the replay client to generate requests; the serving worker loads no eval packs, corpora, compiler, training state, or Python demo code.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-replay-suite-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    println!(
        "  runtime_bytes_estimate: {}",
        report.runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile replay suite failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_fallback_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_FALLBACK_SMOKE_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let bind_addr = reserve_local_bind_addr()?;
    let _server = RoleBindingProfileServeProcess::start(&config_path, &bind_addr, None)?;
    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid fallback smoke bind addr {bind_addr}: {error}"))?;

    let health = send_json_get::<RoleBindingProfileHealthResponse>(addr, "/health")?;
    let mut local_request = registry
        .self_test_score_request()
        .ok_or_else(|| "failed to build local profile score request".to_owned())?;
    local_request.request_id = "fallback_smoke_local_accept".to_owned();
    local_request.expect_local_operator = Some(true);
    let local_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &local_request)?;

    let mut bad_route_request = local_request.clone();
    bad_route_request.request_id = "fallback_smoke_bad_route".to_owned();
    bad_route_request.profile_id = None;
    bad_route_request.route_key = Some("__missing_route__".to_owned());
    bad_route_request.exact_cache_key = Some("fallback_smoke_bad_route".to_owned());
    bad_route_request.expect_local_operator = Some(false);
    let bad_route_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &bad_route_request)?;

    let mut low_margin_request = local_request.clone();
    low_margin_request.request_id = "fallback_smoke_low_margin".to_owned();
    low_margin_request.exact_cache_key = Some("fallback_smoke_low_margin".to_owned());
    for active in &mut low_margin_request.active_fringe {
        active.strength = active.strength.signum();
    }
    for slot in &mut low_margin_request.slots {
        for impulse in &mut slot.positive_impulses {
            impulse.signed_strength = impulse.signed_strength.signum();
        }
        for impulse in &mut slot.negative_impulses {
            impulse.signed_strength = impulse.signed_strength.signum();
        }
    }
    low_margin_request.expect_local_operator = Some(true);
    let low_margin_response =
        send_json_post::<_, RoleBindingProfileScoreResponse>(addr, "/score", &low_margin_request)?;

    let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(addr, "/metrics")?;
    let local_accept_pass = local_response.accepted
        && !local_response.fallback
        && local_response.action == "local_operator"
        && !local_response.false_local_accept;
    let bad_route_pass = !bad_route_response.accepted
        && bad_route_response.fallback
        && bad_route_response.fallback_reason.as_deref() == Some("profile_not_found")
        && !bad_route_response.false_local_accept;
    let low_margin_pass = !low_margin_response.accepted
        && low_margin_response.fallback
        && low_margin_response.fallback_reason.as_deref() == Some("margin_below_threshold")
        && low_margin_response.strict_ordered_pass
        && low_margin_response.energy_margin < low_margin_response.threshold
        && !low_margin_response.false_local_accept;
    let pass = health.status == "ok"
        && !health.compiler_used
        && !health.eval_packs_loaded
        && !health.corpus_jsonl_loaded
        && local_accept_pass
        && bad_route_pass
        && low_margin_pass
        && metrics.local_operator_calls == 1
        && metrics.fallback_to_llm_calls == 2
        && metrics.false_local_accepts == 0
        && metrics.runtime_bytes_estimate <= 3 * 1024 * 1024
        && metrics.p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileFallbackSmokeReport {
        schema_version: "nando_role_binding_profile_fallback_smoke_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_FALLBACK_SMOKE_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        profile_count: health.profile_count,
        local_accept_pass,
        bad_route_fallback_pass: bad_route_pass,
        low_margin_fallback_pass: low_margin_pass,
        local_action: local_response.action.clone(),
        bad_route_fallback_reason: bad_route_response.fallback_reason.clone(),
        low_margin_fallback_reason: low_margin_response.fallback_reason.clone(),
        local_energy_margin: local_response.energy_margin,
        low_margin_energy_margin: low_margin_response.energy_margin,
        low_margin_threshold: low_margin_response.threshold,
        local_operator_calls: metrics.local_operator_calls,
        fallback_to_llm_calls: metrics.fallback_to_llm_calls,
        false_local_accepts: metrics.false_local_accepts,
        p50_latency_ns: metrics.p50_latency_ns,
        p90_latency_ns: metrics.p90_latency_ns,
        p99_latency_ns: metrics.p99_latency_ns,
        rss_bytes: metrics.rss_bytes,
        runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "HTTP fallback smoke over the serving-only .nwrb role-binding profile runtime: verifies local accept, missing-route fallback, and low-margin fallback; not worker scaling or real Codex traffic.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-fallback-smoke-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  profile_count: {}", report.profile_count);
    println!("  local_accept_pass: {}", report.local_accept_pass);
    println!(
        "  bad_route_fallback_pass: {}",
        report.bad_route_fallback_pass
    );
    println!(
        "  low_margin_fallback_pass: {}",
        report.low_margin_fallback_pass
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  p99_latency_ns: {}", report.p99_latency_ns);
    if pass {
        Ok(())
    } else {
        Err("role-binding profile fallback smoke failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_worker_scaling_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_WORKER_SCALING_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for product scaling proof".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }

    let shard_dir = report_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-worker-shards-v1");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct WorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid worker bind addr {bind_addr}: {error}"))?;
        workers.push(WorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let mut rows = Vec::new();
    let mut total_profile_count = 0usize;
    let mut total_local_operator_calls = 0usize;
    let mut total_fallback_to_llm_calls = 0usize;
    let mut total_false_local_accepts = 0usize;
    let mut total_wrong_worker_route_fallbacks = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut all_workers_serving_only = true;
    let mut all_profile_score_pass = true;
    let mut all_wrong_worker_routes_fallback = true;

    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let profiles_response =
            send_json_get::<RoleBindingProfileProfilesResponse>(worker.addr, "/profiles")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !profiles_response.compiler_used
            && !profiles_response.eval_packs_loaded
            && !profiles_response.corpus_jsonl_loaded
            && !profiles_response.python_demo_used;

        let mut local_accepts = 0usize;
        let mut worker_false_local_accepts = 0usize;
        for profile in &worker.registry.profiles {
            let mut request = worker
                .registry
                .self_test_score_request_for_profile(&profile.config.profile_id)
                .ok_or_else(|| {
                    format!(
                        "failed to build worker {} score request for {}",
                        worker.worker_id, profile.config.profile_id
                    )
                })?;
            request.request_id = format!(
                "worker{}_local_accept_{}",
                worker.worker_id, profile.config.profile_id
            );
            request.expect_local_operator = Some(true);
            let response = send_json_post::<_, RoleBindingProfileScoreResponse>(
                worker.addr,
                "/score",
                &request,
            )?;
            let pass = response.accepted
                && !response.fallback
                && response.action == "local_operator"
                && !response.false_local_accept;
            all_profile_score_pass &= pass;
            local_accepts += usize::from(pass);
            worker_false_local_accepts += usize::from(response.false_local_accept);
        }

        let other_route = workers
            .iter()
            .find(|candidate| candidate.worker_id != worker.worker_id)
            .and_then(|candidate| candidate.registry.profiles.first())
            .and_then(|profile| profile.config.accepted_route_keys.first())
            .cloned()
            .ok_or_else(|| "failed to choose wrong-worker route".to_owned())?;
        let first_profile = worker
            .registry
            .profiles
            .first()
            .ok_or_else(|| format!("worker {} has no profiles", worker.worker_id))?;
        let mut wrong_worker_request = worker
            .registry
            .self_test_score_request_for_profile(&first_profile.config.profile_id)
            .ok_or_else(|| {
                format!(
                    "failed to build wrong-worker route request for worker {}",
                    worker.worker_id
                )
            })?;
        wrong_worker_request.request_id = format!("worker{}_wrong_route", worker.worker_id);
        wrong_worker_request.profile_id = None;
        wrong_worker_request.route_key = Some(other_route);
        wrong_worker_request.expect_local_operator = Some(false);
        let wrong_worker_response = send_json_post::<_, RoleBindingProfileScoreResponse>(
            worker.addr,
            "/score",
            &wrong_worker_request,
        )?;
        let wrong_worker_fallback = !wrong_worker_response.accepted
            && wrong_worker_response.fallback
            && wrong_worker_response.fallback_reason.as_deref() == Some("profile_not_found")
            && !wrong_worker_response.false_local_accept;
        all_wrong_worker_routes_fallback &= wrong_worker_fallback;
        worker_false_local_accepts += usize::from(wrong_worker_response.false_local_accept);

        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        total_profile_count += health.profile_count;
        total_local_operator_calls += metrics.local_operator_calls;
        total_fallback_to_llm_calls += metrics.fallback_to_llm_calls;
        total_false_local_accepts += metrics.false_local_accepts;
        total_wrong_worker_route_fallbacks += usize::from(wrong_worker_fallback);
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);

        rows.push(RoleBindingProfileWorkerScalingRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: profiles_response
                .profiles
                .iter()
                .map(|profile| profile.profile_id.clone())
                .collect(),
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
            package_bytes: health.package_bytes,
            edge_count: health.edge_count,
            local_accepts,
            wrong_worker_route_fallbacks: usize::from(wrong_worker_fallback),
            false_local_accepts: worker_false_local_accepts,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            compiler_used: health.compiler_used,
            eval_packs_loaded: health.eval_packs_loaded,
            corpus_jsonl_loaded: health.corpus_jsonl_loaded,
            python_demo_used: health.python_demo_used,
        });
    }

    let pass = workers.len() == worker_count
        && total_profile_count == base_config.profiles.len()
        && total_local_operator_calls == base_config.profiles.len()
        && total_wrong_worker_route_fallbacks == worker_count
        && total_false_local_accepts == 0
        && all_workers_serving_only
        && all_profile_score_pass
        && all_wrong_worker_routes_fallback
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileWorkerScalingReport {
        schema_version: "nando_role_binding_profile_worker_scaling_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_WORKER_SCALING_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_WORKER_SCALING_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        worker_count,
        total_profile_count,
        total_local_operator_calls,
        total_fallback_to_llm_calls,
        wrong_worker_route_fallbacks: total_wrong_worker_route_fallbacks,
        false_local_accepts: total_false_local_accepts,
        max_worker_runtime_bytes_estimate,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        all_profile_score_pass,
        all_wrong_worker_routes_fallback,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        rows,
        claim_boundary: "Product acceptance check for serving-only .nwrb profile worker sharding: multiple workers load separate profile shards, score local accepts, and reject wrong-worker routes. This is not real Codex traffic or external load-balancer proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-worker-scaling-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!("  total_profile_count: {}", report.total_profile_count);
    println!(
        "  total_local_operator_calls: {}",
        report.total_local_operator_calls
    );
    println!(
        "  wrong_worker_route_fallbacks: {}",
        report.wrong_worker_route_fallbacks
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    println!(
        "  max_worker_p99_latency_ns: {}",
        report.max_worker_p99_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile worker scaling failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_worker_replay_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_WORKER_REPLAY_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for sharded replay proof".to_owned());
    }
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }

    let shard_dir = report_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("profile-worker-replay-shards-v1");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct WorkerReplayHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    let mut profile_to_worker = HashMap::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb replay worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        for profile in &registry.profiles {
            if profile_to_worker
                .insert(profile.config.profile_id.clone(), worker_id)
                .is_some()
            {
                return Err(format!(
                    "duplicate profile_id {}",
                    profile.config.profile_id
                ));
            }
        }
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid worker replay bind addr {bind_addr}: {error}"))?;
        workers.push(WorkerReplayHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let mut worker_rows = workers
        .iter()
        .map(|worker| RoleBindingProfileWorkerReplayRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: worker.registry.profile_count(),
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            unique_sequences_replayed: 0,
            http_replay_batches: 0,
            no_cache_llm_calls: 0,
            exact_cache_llm_calls: 0,
            exact_cache_plus_nando_llm_calls: 0,
            exact_cache_incremental_reduction_milli: 0,
            local_operator_calls: 0,
            fallback_to_llm_calls: 0,
            false_local_accepts: 0,
            missed_expected_local: 0,
            p50_latency_ns: 0,
            p90_latency_ns: 0,
            p99_latency_ns: 0,
            rss_bytes: 0,
            runtime_bytes_estimate: 0,
        })
        .collect::<Vec<_>>();

    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;

    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let worker_id = *profile_to_worker
            .get(&profile_id)
            .ok_or_else(|| format!("profile {profile_id} missing from worker shards"))?;
        let worker = workers
            .iter()
            .find(|worker| worker.worker_id == worker_id)
            .ok_or_else(|| format!("worker {worker_id} missing"))?;
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut profile_unique_sequences = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            profile_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if profile_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let response =
                    send_replay_batch(worker.addr, &profile_id, batch_index, &mut batch)?;
                total_http_replay_batches += 1;
                batch_index += 1;
                let row = &mut worker_rows[worker_id];
                row.http_replay_batches += 1;
                row.exact_cache_llm_calls += response.exact_cache_llm_calls;
                row.exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                row.local_operator_calls += response.local_operator_calls;
                row.fallback_to_llm_calls += response.fallback_to_llm_calls;
                row.false_local_accepts += response.false_local_accepts;
                row.missed_expected_local += response.missed_expected_local;
            }
        }
        if !batch.is_empty() {
            let response = send_replay_batch(worker.addr, &profile_id, batch_index, &mut batch)?;
            total_http_replay_batches += 1;
            let row = &mut worker_rows[worker_id];
            row.http_replay_batches += 1;
            row.exact_cache_llm_calls += response.exact_cache_llm_calls;
            row.exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            row.local_operator_calls += response.local_operator_calls;
            row.fallback_to_llm_calls += response.fallback_to_llm_calls;
            row.false_local_accepts += response.false_local_accepts;
            row.missed_expected_local += response.missed_expected_local;
        }
        let row = &mut worker_rows[worker_id];
        row.unique_sequences_replayed += profile_unique_sequences;
        row.no_cache_llm_calls += profile_unique_sequences * 2;
        total_unique_sequences += profile_unique_sequences;
        no_cache_llm_calls += profile_unique_sequences * 2;
    }

    let mut all_workers_serving_only = true;
    let mut total_runtime_bytes_estimate = 0usize;
    let mut total_rss_bytes = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        let row = &mut worker_rows[worker.worker_id];
        row.p50_latency_ns = metrics.p50_latency_ns;
        row.p90_latency_ns = metrics.p90_latency_ns;
        row.p99_latency_ns = metrics.p99_latency_ns;
        row.rss_bytes = metrics.rss_bytes;
        row.runtime_bytes_estimate = metrics.runtime_bytes_estimate;
        row.exact_cache_incremental_reduction_milli = reduction_milli(
            row.exact_cache_llm_calls,
            row.exact_cache_plus_nando_llm_calls,
        );
        total_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        total_rss_bytes += metrics.rss_bytes;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        exact_cache_llm_calls += row.exact_cache_llm_calls;
        exact_cache_plus_nando_llm_calls += row.exact_cache_plus_nando_llm_calls;
        local_operator_calls += row.local_operator_calls;
        fallback_to_llm_calls += row.fallback_to_llm_calls;
        false_local_accepts += row.false_local_accepts;
        missed_expected_local += row.missed_expected_local;
    }
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let pass = workers.len() == worker_count
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && missed_expected_local == 0
        && all_workers_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileWorkerReplayReport {
        schema_version: "nando_role_binding_profile_worker_replay_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_WORKER_REPLAY_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        worker_count,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        total_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        rows: worker_rows,
        claim_boundary: "Sharded HTTP replay over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the replay client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code. This is not real Codex traffic or external load-balancer proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-worker-replay-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    println!(
        "  max_worker_p99_latency_ns: {}",
        report.max_worker_p99_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile worker replay failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_lb_replay_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_LB_REPLAY_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let batch_unique_sequences = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid batch_unique_sequences '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_BATCH_UNIQUE_SEQUENCES);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for load-balancer replay proof".to_owned());
    }
    if batch_unique_sequences == 0 {
        return Err("batch_unique_sequences must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    let parity_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path_with_reference(&config_path)?;
    let (packed_score_parity_checks, packed_score_parity_mismatches) =
        verify_role_binding_profile_packed_score_parity(
            &parity_registry,
            &binary_suite,
            max_unique_sequences_per_profile,
        )?;

    let root_dir = report_path.parent().unwrap_or_else(|| Path::new("."));
    let shard_dir = root_dir.join("profile-lb-replay-shards-v1");
    let lb_config_path = root_dir.join("profile-lb-replay-v1.lb.json");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct LbReplayWorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb load-balancer replay worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid lb replay worker bind addr {bind_addr}: {error}"))?;
        workers.push(LbReplayWorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let lb_config = RoleBindingProfileLoadBalancerConfig {
        schema_version: "nando_role_binding_profile_lb_v1".to_owned(),
        upstreams: workers
            .iter()
            .map(|worker| RoleBindingProfileLoadBalancerUpstreamConfig {
                worker_id: worker.worker_id,
                upstream_addr: worker.addr.to_string(),
                shard_config_path: worker.config_path.clone(),
                profile_ids: worker
                    .registry
                    .profiles
                    .iter()
                    .map(|profile| profile.config.profile_id.clone())
                    .collect(),
                accepted_route_keys: worker
                    .registry
                    .profiles
                    .iter()
                    .flat_map(|profile| profile.config.accepted_route_keys.clone())
                    .collect(),
            })
            .collect(),
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "Local external load-balancer config: routes requests to serving-only .nwrb profile workers; the proxy itself loads no .nwrb/.nwreb packages, corpora, compiler, training state, or Python demo code.".to_owned(),
    };
    validate_load_balancer_config(&lb_config)?;
    write_json_file(&lb_config_path, &lb_config)?;

    let lb_bind_addr = reserve_local_bind_addr()?;
    let load_balancer_process =
        RoleBindingProfileLoadBalancerProcess::start(&lb_config_path, &lb_bind_addr, None)?;
    let lb_addr = lb_bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid lb replay bind addr {lb_bind_addr}: {error}"))?;
    let lb_health =
        send_json_get::<RoleBindingProfileLoadBalancerHealthResponse>(lb_addr, "/health")?;
    let lb_profiles =
        send_json_get::<RoleBindingProfileLoadBalancerProfilesResponse>(lb_addr, "/profiles")?;

    let mut total_unique_sequences = 0usize;
    let mut total_http_replay_batches = 0usize;
    let mut no_cache_llm_calls = 0usize;
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut replay_client_wall_latencies_ns: Vec<u128> = Vec::new();

    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !lb_config
            .upstreams
            .iter()
            .any(|upstream| upstream.profile_ids.iter().any(|id| id == &profile_id))
        {
            return Err(format!(
                "profile {profile_id} from binary suite is missing in lb config"
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        let mut profile_unique_sequences = 0usize;
        let mut batch = Vec::new();
        let mut batch_index = 0usize;
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            profile_unique_sequences += 1;
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                0,
            ));
            batch.push(sequence_to_profile_request(
                &profile_id,
                sequence,
                &cache_key,
                1,
            ));
            if profile_unique_sequences.is_multiple_of(batch_unique_sequences) {
                let (response, wall_latency_ns) =
                    send_replay_batch_timed(lb_addr, &profile_id, batch_index, &mut batch)?;
                replay_client_wall_latencies_ns.push(wall_latency_ns);
                total_http_replay_batches += 1;
                batch_index += 1;
                exact_cache_llm_calls += response.exact_cache_llm_calls;
                exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
                local_operator_calls += response.local_operator_calls;
                fallback_to_llm_calls += response.fallback_to_llm_calls;
                false_local_accepts += response.false_local_accepts;
                missed_expected_local += response.missed_expected_local;
            }
        }
        if !batch.is_empty() {
            let (response, wall_latency_ns) =
                send_replay_batch_timed(lb_addr, &profile_id, batch_index, &mut batch)?;
            replay_client_wall_latencies_ns.push(wall_latency_ns);
            total_http_replay_batches += 1;
            exact_cache_llm_calls += response.exact_cache_llm_calls;
            exact_cache_plus_nando_llm_calls += response.exact_cache_plus_nando_llm_calls;
            local_operator_calls += response.local_operator_calls;
            fallback_to_llm_calls += response.fallback_to_llm_calls;
            false_local_accepts += response.false_local_accepts;
            missed_expected_local += response.missed_expected_local;
        }
        total_unique_sequences += profile_unique_sequences;
        no_cache_llm_calls += profile_unique_sequences * 2;
    }

    let lb_metrics = send_json_get::<RoleBindingProfileMetricsResponse>(lb_addr, "/metrics")?;
    drop(load_balancer_process);
    let mut all_workers_serving_only = true;
    let mut worker_rows = Vec::new();
    let mut total_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut total_worker_rss_bytes = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut max_worker_core_score_p50_latency_ns = 0u128;
    let mut max_worker_core_score_p90_latency_ns = 0u128;
    let mut max_worker_core_score_p99_latency_ns = 0u128;
    let mut max_worker_score_p50_latency_ns = 0u128;
    let mut max_worker_score_p90_latency_ns = 0u128;
    let mut max_worker_score_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        total_worker_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        total_worker_rss_bytes += metrics.rss_bytes;
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        max_worker_core_score_p50_latency_ns =
            max_worker_core_score_p50_latency_ns.max(metrics.core_score_p50_latency_ns);
        max_worker_core_score_p90_latency_ns =
            max_worker_core_score_p90_latency_ns.max(metrics.core_score_p90_latency_ns);
        max_worker_core_score_p99_latency_ns =
            max_worker_core_score_p99_latency_ns.max(metrics.core_score_p99_latency_ns);
        max_worker_score_p50_latency_ns =
            max_worker_score_p50_latency_ns.max(metrics.worker_score_p50_latency_ns);
        max_worker_score_p90_latency_ns =
            max_worker_score_p90_latency_ns.max(metrics.worker_score_p90_latency_ns);
        max_worker_score_p99_latency_ns =
            max_worker_score_p99_latency_ns.max(metrics.worker_score_p99_latency_ns);
        worker_rows.push(RoleBindingProfileLoadBalancerReplayWorkerRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            local_operator_calls: metrics.local_operator_calls,
            fallback_to_llm_calls: metrics.fallback_to_llm_calls,
            false_local_accepts: metrics.false_local_accepts,
            missed_expected_local: metrics.missed_expected_local,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            core_score_p50_latency_ns: metrics.core_score_p50_latency_ns,
            core_score_p90_latency_ns: metrics.core_score_p90_latency_ns,
            core_score_p99_latency_ns: metrics.core_score_p99_latency_ns,
            worker_score_p50_latency_ns: metrics.worker_score_p50_latency_ns,
            worker_score_p90_latency_ns: metrics.worker_score_p90_latency_ns,
            worker_score_p99_latency_ns: metrics.worker_score_p99_latency_ns,
            lb_upstream_roundtrip_p50_latency_ns: metrics.lb_upstream_roundtrip_p50_latency_ns,
            lb_upstream_roundtrip_p90_latency_ns: metrics.lb_upstream_roundtrip_p90_latency_ns,
            lb_upstream_roundtrip_p99_latency_ns: metrics.lb_upstream_roundtrip_p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        });
    }

    let load_balancer_serving_only = !lb_health.compiler_used
        && !lb_health.eval_packs_loaded
        && !lb_health.corpus_jsonl_loaded
        && !lb_health.python_demo_used
        && !lb_metrics.compiler_used
        && !lb_metrics.eval_packs_loaded
        && !lb_metrics.corpus_jsonl_loaded
        && !lb_metrics.python_demo_used;
    let exact_cache_incremental_reduction_milli =
        reduction_milli(exact_cache_llm_calls, exact_cache_plus_nando_llm_calls);
    let estimated_lb_overhead_p99_ns = lb_metrics
        .p99_latency_ns
        .saturating_sub(max_worker_score_p99_latency_ns);
    let pass = lb_health.status == "ok"
        && lb_profiles.profile_count == base_config.profiles.len()
        && workers.len() == worker_count
        && total_unique_sequences > 0
        && exact_cache_incremental_reduction_milli >= 200
        && false_local_accepts == 0
        && missed_expected_local == 0
        && packed_score_parity_mismatches == 0
        && all_workers_serving_only
        && load_balancer_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && lb_metrics.p99_latency_ns <= 3_000_000
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileLoadBalancerReplayReport {
        schema_version: "nando_role_binding_profile_lb_replay_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_LB_REPLAY_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_LB_REPLAY_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        lb_config_path: lb_config_path.display().to_string(),
        lb_bind_addr: lb_addr.to_string(),
        worker_count,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: total_unique_sequences,
        http_replay_batches: total_http_replay_batches,
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        load_balancer_p50_latency_ns: lb_metrics.p50_latency_ns,
        load_balancer_p90_latency_ns: lb_metrics.p90_latency_ns,
        load_balancer_p99_latency_ns: lb_metrics.p99_latency_ns,
        core_score_p50_latency_ns: max_worker_core_score_p50_latency_ns,
        core_score_p90_latency_ns: max_worker_core_score_p90_latency_ns,
        core_score_p99_latency_ns: max_worker_core_score_p99_latency_ns,
        worker_score_p50_latency_ns: max_worker_score_p50_latency_ns,
        worker_score_p90_latency_ns: max_worker_score_p90_latency_ns,
        worker_score_p99_latency_ns: max_worker_score_p99_latency_ns,
        lb_upstream_roundtrip_p50_latency_ns: lb_metrics.lb_upstream_roundtrip_p50_latency_ns,
        lb_upstream_roundtrip_p90_latency_ns: lb_metrics.lb_upstream_roundtrip_p90_latency_ns,
        lb_upstream_roundtrip_p99_latency_ns: lb_metrics.lb_upstream_roundtrip_p99_latency_ns,
        replay_client_wall_p50_latency_ns: percentile(&replay_client_wall_latencies_ns, 50),
        replay_client_wall_p90_latency_ns: percentile(&replay_client_wall_latencies_ns, 90),
        replay_client_wall_p99_latency_ns: percentile(&replay_client_wall_latencies_ns, 99),
        estimated_lb_overhead_p99_ns,
        packed_score_parity_checks,
        packed_score_parity_mismatches,
        load_balancer_rss_bytes: lb_metrics.rss_bytes,
        total_worker_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_worker_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        load_balancer_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        workers: worker_rows,
        claim_boundary: "Local external load-balancer replay over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the replay client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code; the load-balancer loads only route-to-upstream metadata. This is not real Codex/API traffic, not concurrent throughput proof, and not cheap-VPS deployment.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-lb-replay-v1: {}", report.verdict);
    println!("  registry_config: {}", report.registry_config_path);
    println!("  binary_suite_report: {}", report.binary_suite_report_path);
    println!("  lb_config: {}", report.lb_config_path);
    println!("  report: {}", report_path.display());
    println!("  worker_count: {}", report.worker_count);
    println!(
        "  unique_sequences_replayed: {}",
        report.unique_sequences_replayed
    );
    println!("  exact_cache_llm_calls: {}", report.exact_cache_llm_calls);
    println!(
        "  exact_cache_plus_nando_llm_calls: {}",
        report.exact_cache_plus_nando_llm_calls
    );
    println!(
        "  exact_cache_incremental_reduction_milli: {}",
        report.exact_cache_incremental_reduction_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!(
        "  load_balancer_p99_latency_ns: {}",
        report.load_balancer_p99_latency_ns
    );
    println!(
        "  core_score_p99_latency_ns: {}",
        report.core_score_p99_latency_ns
    );
    println!(
        "  worker_score_p99_latency_ns: {}",
        report.worker_score_p99_latency_ns
    );
    println!(
        "  lb_upstream_roundtrip_p99_latency_ns: {}",
        report.lb_upstream_roundtrip_p99_latency_ns
    );
    println!(
        "  replay_client_wall_p99_latency_ns: {}",
        report.replay_client_wall_p99_latency_ns
    );
    println!(
        "  packed_score_parity_checks: {}",
        report.packed_score_parity_checks
    );
    println!(
        "  packed_score_parity_mismatches: {}",
        report.packed_score_parity_mismatches
    );
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile load-balancer replay failed".to_owned())
    }
}

pub(crate) fn run_role_binding_profile_lb_throughput_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_LB_THROUGHPUT_REPORT));
    let worker_count = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid worker_count '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROFILE_WORKER_COUNT);
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_REPLAY_MAX_UNIQUE_SEQUENCES_PER_PROFILE);
    let client_threads = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid client_threads '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_THROUGHPUT_CLIENT_THREADS);
    let sequence_repetitions = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid sequence_repetitions '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(DEFAULT_THROUGHPUT_SEQUENCE_REPETITIONS);
    if worker_count < 2 {
        return Err("worker_count must be at least 2 for throughput proof".to_owned());
    }
    if client_threads == 0 {
        return Err("client_threads must be positive".to_owned());
    }
    if sequence_repetitions == 0 {
        return Err("sequence_repetitions must be positive".to_owned());
    }

    let base_config = read_json_file::<RoleBindingProfileRegistryConfig>(&config_path)?;
    validate_registry_config(&base_config)?;
    if worker_count > base_config.profiles.len() {
        return Err(format!(
            "worker_count {} exceeds profile_count {}",
            worker_count,
            base_config.profiles.len()
        ));
    }
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    let parity_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path_with_reference(&config_path)?;
    let (packed_score_parity_checks, packed_score_parity_mismatches) =
        verify_role_binding_profile_packed_score_parity(
            &parity_registry,
            &binary_suite,
            max_unique_sequences_per_profile,
        )?;

    let root_dir = report_path.parent().unwrap_or_else(|| Path::new("."));
    let shard_dir = root_dir.join("profile-lb-throughput-shards-v1");
    let lb_config_path = root_dir.join("profile-lb-throughput-v1.lb.json");
    let mut shard_profiles = vec![Vec::<RoleBindingProfileConfig>::new(); worker_count];
    for (index, profile) in base_config.profiles.iter().cloned().enumerate() {
        shard_profiles[index % worker_count].push(profile);
    }

    struct LbThroughputWorkerHarness {
        worker_id: usize,
        config_path: PathBuf,
        registry: RoleBindingProfileRuntimeRegistry,
        addr: SocketAddr,
        _server: RoleBindingProfileServeProcess,
    }

    let mut workers = Vec::new();
    for (worker_id, profiles) in shard_profiles.into_iter().enumerate() {
        if profiles.is_empty() {
            return Err(format!("worker {worker_id} has no profiles"));
        }
        let shard_config_path = shard_dir.join(format!("worker-{worker_id}.registry.json"));
        let shard_config = RoleBindingProfileRegistryConfig {
            schema_version: "nando_role_binding_profile_registry_v1".to_owned(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
            claim_boundary: format!(
                "Serving-only .nwrb throughput worker shard {worker_id}; no eval packs, corpus, compiler, training state, or Python demo code."
            ),
        };
        write_json_file(&shard_config_path, &shard_config)?;
        let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&shard_config_path)?;
        let bind_addr = reserve_local_bind_addr()?;
        let server = RoleBindingProfileServeProcess::start(&shard_config_path, &bind_addr, None)?;
        let addr = bind_addr.parse::<SocketAddr>().map_err(|error| {
            format!("invalid lb throughput worker bind addr {bind_addr}: {error}")
        })?;
        workers.push(LbThroughputWorkerHarness {
            worker_id,
            config_path: shard_config_path,
            registry,
            addr,
            _server: server,
        });
    }

    let lb_config = RoleBindingProfileLoadBalancerConfig {
        schema_version: "nando_role_binding_profile_lb_v1".to_owned(),
        upstreams: workers
            .iter()
            .map(|worker| RoleBindingProfileLoadBalancerUpstreamConfig {
                worker_id: worker.worker_id,
                upstream_addr: worker.addr.to_string(),
                shard_config_path: worker.config_path.clone(),
                profile_ids: worker
                    .registry
                    .profiles
                    .iter()
                    .map(|profile| profile.config.profile_id.clone())
                    .collect(),
                accepted_route_keys: worker
                    .registry
                    .profiles
                    .iter()
                    .flat_map(|profile| profile.config.accepted_route_keys.clone())
                    .collect(),
            })
            .collect(),
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        claim_boundary: "Local external load-balancer config for bounded concurrent throughput proof; routes requests to serving-only .nwrb profile workers.".to_owned(),
    };
    validate_load_balancer_config(&lb_config)?;
    write_json_file(&lb_config_path, &lb_config)?;

    let request_set = build_role_binding_profile_throughput_requests(
        &binary_suite,
        &lb_config,
        max_unique_sequences_per_profile,
    )?;
    if request_set.requests.is_empty() {
        return Err("throughput request set is empty".to_owned());
    }
    let total_score_requests = request_set
        .requests
        .len()
        .checked_mul(sequence_repetitions)
        .ok_or_else(|| "throughput request count overflow".to_owned())?;

    let lb_bind_addr = reserve_local_bind_addr()?;
    let load_balancer_process =
        RoleBindingProfileLoadBalancerProcess::start(&lb_config_path, &lb_bind_addr, None)?;
    let lb_addr = lb_bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid lb throughput bind addr {lb_bind_addr}: {error}"))?;
    let lb_health =
        send_json_get::<RoleBindingProfileLoadBalancerHealthResponse>(lb_addr, "/health")?;
    let lb_profiles =
        send_json_get::<RoleBindingProfileLoadBalancerProfilesResponse>(lb_addr, "/profiles")?;

    println!("role-binding-profile-lb-throughput-v1: starting bounded pressure");
    println!("  worker_count: {worker_count}");
    println!("  client_threads: {client_threads}");
    println!(
        "  unique_sequences_replayed: {}",
        request_set.unique_sequences
    );
    println!("  score_requests: {total_score_requests}");

    let requests = Arc::new(request_set.requests);
    let wall_start = Instant::now();
    let mut handles = Vec::new();
    for client_id in 0..client_threads {
        let requests = Arc::clone(&requests);
        handles.push(thread::spawn(
            move || -> RoleBindingProfileThroughputClientRow {
                let mut row = RoleBindingProfileThroughputClientRow {
                    client_id,
                    score_requests: 0,
                    local_operator_calls: 0,
                    fallback_to_llm_calls: 0,
                    false_local_accepts: 0,
                    missed_expected_local: 0,
                    unexpected_local_accepts: 0,
                    errors: 0,
                    first_error: None,
                    p50_latency_ns: 0,
                    p90_latency_ns: 0,
                    p99_latency_ns: 0,
                };
                let mut latencies = Vec::new();
                for ordinal in (client_id..total_score_requests).step_by(client_threads) {
                    let request = &requests[ordinal % requests.len()];
                    let start = Instant::now();
                    match send_json_post::<_, RoleBindingProfileScoreResponse>(
                        lb_addr, "/score", request,
                    ) {
                        Ok(response) => {
                            latencies.push(start.elapsed().as_nanos());
                            row.score_requests += 1;
                            row.local_operator_calls += usize::from(response.accepted);
                            row.fallback_to_llm_calls += usize::from(response.fallback);
                            row.false_local_accepts += usize::from(response.false_local_accept);
                            row.missed_expected_local += usize::from(
                                request.expect_local_operator == Some(true) && !response.accepted,
                            );
                            row.unexpected_local_accepts += usize::from(
                                request.expect_local_operator == Some(false) && response.accepted,
                            );
                        }
                        Err(error) => {
                            latencies.push(start.elapsed().as_nanos());
                            row.errors += 1;
                            if row.first_error.is_none() {
                                row.first_error = Some(error);
                            }
                        }
                    }
                }
                row.p50_latency_ns = percentile(&latencies, 50);
                row.p90_latency_ns = percentile(&latencies, 90);
                row.p99_latency_ns = percentile(&latencies, 99);
                println!(
                    "  throughput_client_done: id={} requests={} errors={} p99_ns={}",
                    row.client_id, row.score_requests, row.errors, row.p99_latency_ns
                );
                row
            },
        ));
    }

    let mut client_rows = Vec::new();
    for handle in handles {
        client_rows.push(
            handle
                .join()
                .map_err(|_| "throughput client thread panicked".to_owned())?,
        );
    }
    let total_wall_latency_ns = wall_start.elapsed().as_nanos();

    let lb_metrics = send_json_get::<RoleBindingProfileMetricsResponse>(lb_addr, "/metrics")?;
    drop(load_balancer_process);
    let mut all_workers_serving_only = true;
    let mut worker_rows = Vec::new();
    let mut total_worker_runtime_bytes_estimate = 0usize;
    let mut max_worker_runtime_bytes_estimate = 0usize;
    let mut total_worker_rss_bytes = 0usize;
    let mut max_worker_rss_bytes = 0usize;
    let mut max_worker_p99_latency_ns = 0u128;
    let mut max_worker_core_score_p50_latency_ns = 0u128;
    let mut max_worker_core_score_p90_latency_ns = 0u128;
    let mut max_worker_core_score_p99_latency_ns = 0u128;
    let mut max_worker_score_p50_latency_ns = 0u128;
    let mut max_worker_score_p90_latency_ns = 0u128;
    let mut max_worker_score_p99_latency_ns = 0u128;
    for worker in &workers {
        let health = send_json_get::<RoleBindingProfileHealthResponse>(worker.addr, "/health")?;
        let metrics = send_json_get::<RoleBindingProfileMetricsResponse>(worker.addr, "/metrics")?;
        all_workers_serving_only &= !health.compiler_used
            && !health.eval_packs_loaded
            && !health.corpus_jsonl_loaded
            && !health.python_demo_used
            && !metrics.compiler_used
            && !metrics.eval_packs_loaded
            && !metrics.corpus_jsonl_loaded
            && !metrics.python_demo_used;
        total_worker_runtime_bytes_estimate += metrics.runtime_bytes_estimate;
        max_worker_runtime_bytes_estimate =
            max_worker_runtime_bytes_estimate.max(metrics.runtime_bytes_estimate);
        total_worker_rss_bytes += metrics.rss_bytes;
        max_worker_rss_bytes = max_worker_rss_bytes.max(metrics.rss_bytes);
        max_worker_p99_latency_ns = max_worker_p99_latency_ns.max(metrics.p99_latency_ns);
        max_worker_core_score_p50_latency_ns =
            max_worker_core_score_p50_latency_ns.max(metrics.core_score_p50_latency_ns);
        max_worker_core_score_p90_latency_ns =
            max_worker_core_score_p90_latency_ns.max(metrics.core_score_p90_latency_ns);
        max_worker_core_score_p99_latency_ns =
            max_worker_core_score_p99_latency_ns.max(metrics.core_score_p99_latency_ns);
        max_worker_score_p50_latency_ns =
            max_worker_score_p50_latency_ns.max(metrics.worker_score_p50_latency_ns);
        max_worker_score_p90_latency_ns =
            max_worker_score_p90_latency_ns.max(metrics.worker_score_p90_latency_ns);
        max_worker_score_p99_latency_ns =
            max_worker_score_p99_latency_ns.max(metrics.worker_score_p99_latency_ns);
        worker_rows.push(RoleBindingProfileLoadBalancerReplayWorkerRow {
            worker_id: worker.worker_id,
            shard_config_path: worker.config_path.display().to_string(),
            bind_addr: worker.addr.to_string(),
            profile_count: health.profile_count,
            profile_ids: worker
                .registry
                .profiles
                .iter()
                .map(|profile| profile.config.profile_id.clone())
                .collect(),
            local_operator_calls: metrics.local_operator_calls,
            fallback_to_llm_calls: metrics.fallback_to_llm_calls,
            false_local_accepts: metrics.false_local_accepts,
            missed_expected_local: metrics.missed_expected_local,
            p50_latency_ns: metrics.p50_latency_ns,
            p90_latency_ns: metrics.p90_latency_ns,
            p99_latency_ns: metrics.p99_latency_ns,
            core_score_p50_latency_ns: metrics.core_score_p50_latency_ns,
            core_score_p90_latency_ns: metrics.core_score_p90_latency_ns,
            core_score_p99_latency_ns: metrics.core_score_p99_latency_ns,
            worker_score_p50_latency_ns: metrics.worker_score_p50_latency_ns,
            worker_score_p90_latency_ns: metrics.worker_score_p90_latency_ns,
            worker_score_p99_latency_ns: metrics.worker_score_p99_latency_ns,
            lb_upstream_roundtrip_p50_latency_ns: metrics.lb_upstream_roundtrip_p50_latency_ns,
            lb_upstream_roundtrip_p90_latency_ns: metrics.lb_upstream_roundtrip_p90_latency_ns,
            lb_upstream_roundtrip_p99_latency_ns: metrics.lb_upstream_roundtrip_p99_latency_ns,
            rss_bytes: metrics.rss_bytes,
            runtime_bytes_estimate: metrics.runtime_bytes_estimate,
        });
    }

    let load_balancer_serving_only = !lb_health.compiler_used
        && !lb_health.eval_packs_loaded
        && !lb_health.corpus_jsonl_loaded
        && !lb_health.python_demo_used
        && !lb_metrics.compiler_used
        && !lb_metrics.eval_packs_loaded
        && !lb_metrics.corpus_jsonl_loaded
        && !lb_metrics.python_demo_used;
    let score_requests: usize = client_rows.iter().map(|row| row.score_requests).sum();
    let local_operator_calls: usize = client_rows.iter().map(|row| row.local_operator_calls).sum();
    let fallback_to_llm_calls: usize = client_rows
        .iter()
        .map(|row| row.fallback_to_llm_calls)
        .sum();
    let false_local_accepts: usize = client_rows.iter().map(|row| row.false_local_accepts).sum();
    let missed_expected_local: usize = client_rows
        .iter()
        .map(|row| row.missed_expected_local)
        .sum();
    let unexpected_local_accepts: usize = client_rows
        .iter()
        .map(|row| row.unexpected_local_accepts)
        .sum();
    let client_errors: usize = client_rows.iter().map(|row| row.errors).sum();
    let mut client_latencies = Vec::new();
    for row in &client_rows {
        client_latencies.push(row.p50_latency_ns);
        client_latencies.push(row.p90_latency_ns);
        client_latencies.push(row.p99_latency_ns);
    }
    let throughput_requests_per_second_milli = if total_wall_latency_ns == 0 {
        0
    } else {
        (score_requests as u128)
            .saturating_mul(1_000_000_000_000)
            .checked_div(total_wall_latency_ns)
            .unwrap_or(0)
    };
    let estimated_lb_overhead_p99_ns = lb_metrics
        .p99_latency_ns
        .saturating_sub(max_worker_score_p99_latency_ns);
    let pass = lb_health.status == "ok"
        && lb_profiles.profile_count == base_config.profiles.len()
        && score_requests == total_score_requests
        && client_errors == 0
        && false_local_accepts == 0
        && missed_expected_local == 0
        && unexpected_local_accepts == 0
        && packed_score_parity_mismatches == 0
        && all_workers_serving_only
        && load_balancer_serving_only
        && max_worker_runtime_bytes_estimate <= 3 * 1024 * 1024
        && lb_metrics.p99_latency_ns <= 3_000_000
        && max_worker_p99_latency_ns <= 3_000_000;

    let report = RoleBindingProfileLoadBalancerThroughputReport {
        schema_version: "nando_role_binding_profile_lb_throughput_report_v1".to_owned(),
        verdict: if pass {
            "ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_PASS"
        } else {
            "ROLE_BINDING_PROFILE_LB_THROUGHPUT_V1_FAIL"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        binary_suite_report_path: binary_suite_path.display().to_string(),
        lb_config_path: lb_config_path.display().to_string(),
        lb_bind_addr: lb_addr.to_string(),
        worker_count,
        client_threads,
        sequence_repetitions,
        total_profile_count: base_config.profiles.len(),
        unique_sequences_replayed: request_set.unique_sequences,
        score_requests,
        expected_local_sequences: request_set.expected_local_sequences,
        expected_fallback_sequences: request_set.expected_fallback_sequences,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        unexpected_local_accepts,
        client_errors,
        total_wall_latency_ns,
        throughput_requests_per_second_milli,
        client_p50_latency_ns: percentile(&client_latencies, 50),
        client_p90_latency_ns: percentile(&client_latencies, 90),
        client_p99_latency_ns: percentile(&client_latencies, 99),
        load_balancer_p50_latency_ns: lb_metrics.p50_latency_ns,
        load_balancer_p90_latency_ns: lb_metrics.p90_latency_ns,
        load_balancer_p99_latency_ns: lb_metrics.p99_latency_ns,
        core_score_p50_latency_ns: max_worker_core_score_p50_latency_ns,
        core_score_p90_latency_ns: max_worker_core_score_p90_latency_ns,
        core_score_p99_latency_ns: max_worker_core_score_p99_latency_ns,
        worker_score_p50_latency_ns: max_worker_score_p50_latency_ns,
        worker_score_p90_latency_ns: max_worker_score_p90_latency_ns,
        worker_score_p99_latency_ns: max_worker_score_p99_latency_ns,
        lb_upstream_roundtrip_p50_latency_ns: lb_metrics.lb_upstream_roundtrip_p50_latency_ns,
        lb_upstream_roundtrip_p90_latency_ns: lb_metrics.lb_upstream_roundtrip_p90_latency_ns,
        lb_upstream_roundtrip_p99_latency_ns: lb_metrics.lb_upstream_roundtrip_p99_latency_ns,
        estimated_lb_overhead_p99_ns,
        packed_score_parity_checks,
        packed_score_parity_mismatches,
        load_balancer_rss_bytes: lb_metrics.rss_bytes,
        total_worker_runtime_bytes_estimate,
        max_worker_runtime_bytes_estimate,
        total_worker_rss_bytes,
        max_worker_rss_bytes,
        max_worker_p99_latency_ns,
        all_workers_serving_only,
        load_balancer_serving_only,
        eval_packs_used_by_replay_client: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        clients: client_rows,
        workers: worker_rows,
        claim_boundary: "Bounded concurrent local load-balancer /score pressure over multiple serving-only .nwrb profile workers. .nwreb eval packs are read only by the pressure client to generate requests; workers load no eval packs, corpora, compiler, training state, or Python demo code; the load-balancer loads only route-to-upstream metadata. This is not real Codex/API traffic, not a long-running daemon soak, and not cheap-VPS deployment.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-profile-lb-throughput-v1: {}", report.verdict);
    println!("  report: {}", report_path.display());
    println!("  score_requests: {}", report.score_requests);
    println!(
        "  throughput_requests_per_second_milli: {}",
        report.throughput_requests_per_second_milli
    );
    println!("  false_local_accepts: {}", report.false_local_accepts);
    println!("  client_errors: {}", report.client_errors);
    println!(
        "  load_balancer_p99_latency_ns: {}",
        report.load_balancer_p99_latency_ns
    );
    println!("  client_p99_latency_ns: {}", report.client_p99_latency_ns);
    println!(
        "  max_worker_runtime_bytes_estimate: {}",
        report.max_worker_runtime_bytes_estimate
    );
    if pass {
        Ok(())
    } else {
        Err("role-binding profile load-balancer throughput failed".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_record_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let record_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "role-binding-real-traffic-record-v1 requires record-json".to_owned())?;
    let record = read_json_file::<RoleBindingRealTrafficTraceRow>(&record_path)?;
    validate_real_traffic_trace_row(&record)?;
    append_jsonl_row(&trace_path, &record)?;
    println!("role-binding-real-traffic-record-v1: REAL_TRAFFIC_TRACE_ROW_RECORDED");
    println!("  trace: {}", trace_path.display());
    println!("  record: {}", record_path.display());
    println!("  trace_id: {}", record.trace_id);
    println!("  llm_call: {}", record.llm_call);
    println!(
        "  has_nando_shadow_request: {}",
        record.nando_shadow_request.is_some()
    );
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_record_serve_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let bind_addr = args.next().unwrap_or_else(|| "127.0.0.1:0".to_owned());
    let request_limit = parse_optional_usize(args.next(), "request_limit")?;
    let listener = TcpListener::bind(&bind_addr)
        .map_err(|error| format!("failed to bind real-traffic recorder {bind_addr}: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read real-traffic recorder addr: {error}"))?;
    println!("role-binding-real-traffic-record-serve-v1: REAL_TRAFFIC_RECORDER_LISTENING");
    println!("  addr: {addr}");
    println!("  trace: {}", trace_path.display());
    println!(
        "  request_limit: {}",
        request_limit
            .map(|limit| limit.to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    let stats = serve_real_traffic_record_requests(
        listener,
        trace_path.clone(),
        RoleBindingProfileServeConfig { request_limit },
    )?;
    println!("role-binding-real-traffic-record-serve-v1: REAL_TRAFFIC_RECORDER_STOPPED");
    println!("  trace: {}", trace_path.display());
    println!("  rows_written: {}", stats.rows_written);
    println!("  requests_handled: {}", stats.requests_handled);
    println!("  bad_requests: {}", stats.bad_requests);
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_ingest_events_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let events_path = args.next().map(PathBuf::from).ok_or_else(|| {
        "role-binding-real-traffic-ingest-events-v1 requires events-jsonl".to_owned()
    })?;
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_INGEST_REPORT));

    let events = read_real_traffic_event_jsonl(&events_path)?;
    let mut rows = Vec::with_capacity(events.len());
    let mut llm_calls = 0usize;
    let mut operator_candidate_events = 0usize;
    let mut synthetic_events = 0usize;
    let mut events_without_shadow_request = 0usize;

    for event in events {
        llm_calls += usize::from(event.llm_call);
        operator_candidate_events += usize::from(event.nando_shadow_request.is_some());
        synthetic_events += usize::from(event.synthetic_source == Some(true));
        events_without_shadow_request += usize::from(event.nando_shadow_request.is_none());
        let row = RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event.event_id,
            traffic_source: event.traffic_source,
            time_ms: event.time_ms,
            request_fingerprint: event.request_fingerprint,
            response_fingerprint: event.response_fingerprint,
            tool_call_fingerprints: event.tool_call_fingerprints,
            verification_source: event.verification_source,
            llm_call: event.llm_call,
            exact_cache_key: event.exact_cache_key,
            provider_cache_hit: event.provider_cache_hit,
            provider_cost_microusd: event.provider_cost_microusd,
            nando_shadow_request: event.nando_shadow_request,
            verified_safe_accept: event.verified_safe_accept,
            synthetic_source: event.synthetic_source,
            notes: event.notes,
        };
        validate_real_traffic_trace_row(&row)?;
        rows.push(row);
    }
    write_real_traffic_trace_jsonl(&trace_path, &rows)?;

    let pass = !rows.is_empty() && synthetic_events == 0 && operator_candidate_events > 0;
    let report = RoleBindingRealTrafficIngestReport {
        schema_version: "nando_role_binding_real_traffic_ingest_report_v1".to_owned(),
        verdict: if pass {
            "REAL_TRAFFIC_INGEST_V1_READY_FOR_SHADOW"
        } else {
            "REAL_TRAFFIC_INGEST_V1_REVIEW"
        }
        .to_owned(),
        events_path: events_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        total_events: rows.len(),
        llm_calls,
        operator_candidate_events,
        events_without_shadow_request,
        synthetic_events,
        rows_written: rows.len(),
        claim_boundary: "Batch ingestion only converts agent/API events into the real-traffic shadow trace contract. It does not prove savings. Synthetic events and traces without nando_shadow_request stay REVIEW.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-ingest-events-v1: {}",
        report.verdict
    );
    println!("  events: {}", events_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_events: {}", report.total_events);
    println!("  llm_calls: {}", report.llm_calls);
    println!(
        "  operator_candidate_events: {}",
        report.operator_candidate_events
    );
    println!("  synthetic_events: {}", report.synthetic_events);
    if pass {
        Ok(())
    } else {
        Err("real-traffic ingest report is review-only".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_codex_history_ingest_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let events_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_EVENTS_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_REAL_TRAFFIC_INGEST_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut events = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        events.push(RoleBindingRealTrafficEventRow {
            schema_version: "nando_role_binding_real_traffic_event_v1".to_owned(),
            event_id: format!("codex_history::{}::{}::{}", row.session_id, row.ts, index),
            traffic_source: Some("codex_history_local".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(format!("fnv1a64:{fingerprint:016x}")),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "local Codex history prompt fingerprint; raw text not written".to_owned(),
            ),
            llm_call: true,
            exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request: None,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(
                "real local Codex prompt history; no prepared Nando shadow request yet".to_owned(),
            ),
        });
    }
    write_real_traffic_event_jsonl(&events_path, &events)?;

    let report = RoleBindingCodexHistoryIngestReport {
        schema_version: "nando_role_binding_codex_history_ingest_report_v1".to_owned(),
        verdict: if events.is_empty() {
            "CODEX_HISTORY_EVENTS_V1_REVIEW"
        } else {
            "CODEX_HISTORY_EVENTS_V1_READY"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        events_path: events_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        events_written: events.len(),
        llm_calls: events.len(),
        nando_shadow_requests: 0,
        synthetic_events: 0,
        raw_text_written: false,
        claim_boundary: "Codex history ingestion creates privacy-safe real event fingerprints only. It does not prove savings because nando_shadow_request is not inferred from raw text.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-codex-history-ingest-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  events: {}", events_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_history_rows: {}", report.total_history_rows);
    println!("  events_written: {}", report.events_written);
    println!("  raw_text_written: {}", report.raw_text_written);
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_codex_history_route_candidates_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let events_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_EVENTS_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut events = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut candidate_events = 0usize;
    let mut no_candidate_events = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_route::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let candidate = route_catalog.classify_request_text(&row.text);
        let nando_shadow_request = candidate.as_ref().map(|candidate| {
            *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
            RoleBindingProfileScoreRequest {
                request_id: event_id.clone(),
                route_key: Some(candidate.route_key.clone()),
                profile_id: Some(candidate.profile_id.clone()),
                exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
                active_fringe: Vec::new(),
                slots: Vec::new(),
                expect_local_operator: Some(false),
            }
        });
        candidate_events += usize::from(nando_shadow_request.is_some());
        no_candidate_events += usize::from(nando_shadow_request.is_none());
        events.push(RoleBindingRealTrafficEventRow {
            schema_version: "nando_role_binding_real_traffic_event_v1".to_owned(),
            event_id,
            traffic_source: Some("codex_history_local_route_only".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(format!("fnv1a64:{fingerprint:016x}")),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "route-only candidate from local Codex prompt; raw text not written".to_owned(),
            ),
            llm_call: true,
            exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(
                "route-only Nando candidate; empty payload forces safe fallback until request builder exists"
                    .to_owned(),
            ),
        });
    }
    write_real_traffic_event_jsonl(&events_path, &events)?;
    let route_counts = route_counts
        .into_iter()
        .map(
            |(route_key, candidate_events)| RoleBindingCodexHistoryRouteCandidateCount {
                route_key,
                candidate_events,
            },
        )
        .collect::<Vec<_>>();
    let report = RoleBindingCodexHistoryRouteCandidateReport {
        schema_version: "nando_role_binding_codex_history_route_candidate_report_v1".to_owned(),
        verdict: "CODEX_HISTORY_ROUTE_CANDIDATES_V1_REVIEW".to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        events_path: events_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        events_written: events.len(),
        candidate_events,
        no_candidate_events,
        route_counts,
        raw_text_written: false,
        full_shadow_request_payload_built: false,
        claim_boundary: "Route-only candidate adapter over local Codex prompt text. It selects route/profile without response, target, or proof labels, but emits empty payloads so scoring must fallback. This is not a savings proof.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-codex-history-route-candidates-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  events: {}", events_path.display());
    println!("  report: {}", report_path.display());
    println!("  events_written: {}", report.events_written);
    println!("  candidate_events: {}", report.candidate_events);
    println!("  no_candidate_events: {}", report.no_candidate_events);
    println!("  full_shadow_request_payload_built: false");
    Err("codex history route candidates are review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_shadow_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_REAL_TRAFFIC_SHADOW_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut exact_cache_seen = HashSet::new();
    let mut exact_cache_hits = 0usize;
    let mut provider_cache_hits = 0usize;
    let mut total_llm_calls = 0usize;
    let mut operator_candidate_calls = 0usize;
    let mut nando_shadow_accepts = 0usize;
    let mut nando_shadow_fallbacks = 0usize;
    let mut verified_safe_accepts = 0usize;
    let mut unverified_shadow_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut incremental_savings_over_exact_cache = 0usize;
    let mut estimated_cost_saved_microusd = 0u128;
    let mut shadow_score_latencies_ns = Vec::new();
    let mut operator_accumulators =
        BTreeMap::<String, RoleBindingRealTrafficOperatorAccumulator>::new();
    let mut rows = Vec::new();

    for trace in &trace_rows {
        if trace.llm_call {
            total_llm_calls += 1;
        }
        let exact_cache_hit = trace
            .exact_cache_key
            .as_ref()
            .map(|key| !exact_cache_seen.insert(key.clone()))
            .unwrap_or(false);
        exact_cache_hits += usize::from(exact_cache_hit);
        provider_cache_hits += usize::from(trace.provider_cache_hit.unwrap_or(false));

        let mut shadow_accepted = false;
        let mut shadow_fallback = false;
        let mut shadow_action = "not_routable".to_owned();
        let mut shadow_latency_ns = 0u128;
        let mut energy_margin = 0i32;
        let mut min_slot_margin = 0i32;
        let mut false_local_accept = false;
        let mut verified_safe_accept = false;
        let mut unverified_accept = false;

        if let Some(request) = &trace.nando_shadow_request {
            operator_candidate_calls += 1;
            let response = score_role_binding_profile_request(&registry, request);
            shadow_accepted = response.accepted;
            shadow_fallback = response.fallback;
            shadow_action = response.action.clone();
            shadow_latency_ns = response.latency_ns;
            energy_margin = response.energy_margin;
            min_slot_margin = response.min_slot_margin;
            false_local_accept = trace
                .verified_safe_accept
                .map(|safe| response.accepted && !safe)
                .unwrap_or(response.false_local_accept);
            verified_safe_accept = response.accepted && trace.verified_safe_accept == Some(true);
            unverified_accept = response.accepted && trace.verified_safe_accept.is_none();

            shadow_score_latencies_ns.push(response.latency_ns);
            nando_shadow_accepts += usize::from(response.accepted);
            nando_shadow_fallbacks += usize::from(response.fallback);
            false_accepts += usize::from(false_local_accept);
            verified_safe_accepts += usize::from(verified_safe_accept);
            unverified_shadow_accepts += usize::from(unverified_accept);
            missed_expected_local += usize::from(
                trace.verified_safe_accept == Some(true)
                    && response.fallback
                    && request.expect_local_operator != Some(false),
            );
            if verified_safe_accept && !exact_cache_hit {
                incremental_savings_over_exact_cache += 1;
                estimated_cost_saved_microusd +=
                    u128::from(trace.provider_cost_microusd.unwrap_or(0));
            }

            let operator_key = real_traffic_operator_key(request, &shadow_action);
            let route_key = request
                .route_key
                .clone()
                .unwrap_or_else(|| "none".to_owned());
            let profile_id = request
                .profile_id
                .clone()
                .unwrap_or_else(|| "route_selected".to_owned());
            let accumulator = operator_accumulators
                .entry(operator_key)
                .or_insert_with(|| RoleBindingRealTrafficOperatorAccumulator {
                    route_key,
                    profile_id,
                    action: shadow_action.clone(),
                    ..RoleBindingRealTrafficOperatorAccumulator::default()
                });
            accumulator.candidate_calls += 1;
            accumulator.llm_calls += usize::from(trace.llm_call);
            accumulator.exact_cache_hits += usize::from(exact_cache_hit);
            accumulator.nando_shadow_accepts += usize::from(shadow_accepted);
            accumulator.verified_safe_accepts += usize::from(verified_safe_accept);
            accumulator.unverified_shadow_accepts += usize::from(unverified_accept);
            accumulator.false_accepts += usize::from(false_local_accept);
            accumulator.incremental_savings_over_exact_cache +=
                usize::from(verified_safe_accept && !exact_cache_hit);
            if verified_safe_accept && !exact_cache_hit {
                accumulator.estimated_cost_saved_microusd +=
                    u128::from(trace.provider_cost_microusd.unwrap_or(0));
            }
            accumulator.latencies_ns.push(shadow_latency_ns);
        }

        rows.push(RoleBindingRealTrafficShadowRow {
            trace_id: trace.trace_id.clone(),
            traffic_source: trace.traffic_source.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            tool_call_fingerprints: trace.tool_call_fingerprints.clone(),
            verification_source: trace.verification_source.clone(),
            llm_call: trace.llm_call,
            exact_cache_hit,
            provider_cache_hit: trace.provider_cache_hit.unwrap_or(false),
            nando_routable: trace.nando_shadow_request.is_some(),
            nando_shadow_accepted: shadow_accepted,
            nando_shadow_fallback: shadow_fallback,
            nando_shadow_action: shadow_action,
            verified_safe_accept,
            unverified_shadow_accept: unverified_accept,
            false_local_accept,
            shadow_score_latency_ns: shadow_latency_ns,
            energy_margin,
            min_slot_margin,
        });
    }

    let exact_cache_plus_nando_llm_calls = total_llm_calls
        .saturating_sub(exact_cache_hits)
        .saturating_sub(incremental_savings_over_exact_cache);
    let operator_rankings =
        real_traffic_operator_rankings(operator_accumulators, operator_candidate_calls);
    let pass = false_accepts == 0
        && unverified_shadow_accepts == 0
        && verified_safe_accepts > 0
        && incremental_savings_over_exact_cache > 0
        && operator_candidate_calls > 0
        && total_llm_calls > 0
        && trace_rows
            .iter()
            .all(|row| row.synthetic_source != Some(true))
        && !trace_rows.is_empty();

    let report = RoleBindingRealTrafficShadowReport {
        schema_version: "nando_role_binding_real_traffic_shadow_report_v1".to_owned(),
        verdict: if pass {
            "REAL_TRAFFIC_SHADOW_V1_PASS"
        } else {
            "REAL_TRAFFIC_SHADOW_V1_REVIEW"
        }
        .to_owned(),
        registry_config_path: config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        traffic_source: common_traffic_source(&trace_rows),
        time_window: real_traffic_time_window(&trace_rows),
        total_requests: trace_rows.len(),
        total_llm_calls,
        operator_candidate_calls,
        exact_cache_hits,
        provider_cache_hits,
        nando_shadow_accepts,
        nando_shadow_fallbacks,
        verified_safe_accepts,
        unverified_shadow_accepts,
        false_accepts,
        missed_expected_local,
        incremental_savings_over_exact_cache,
        exact_cache_llm_calls: total_llm_calls.saturating_sub(exact_cache_hits),
        exact_cache_plus_nando_llm_calls,
        incremental_reduction_vs_exact_cache_milli: reduction_milli(
            total_llm_calls.saturating_sub(exact_cache_hits),
            exact_cache_plus_nando_llm_calls,
        ),
        estimated_cost_saved_microusd,
        p50_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 50),
        p90_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 90),
        p99_shadow_score_latency_ns: percentile(&shadow_score_latencies_ns, 99),
        runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
        rss_bytes: current_rss_bytes(),
        serving_only_registry: true,
        compiler_used: false,
        eval_packs_loaded: false,
        corpus_jsonl_loaded: false,
        python_demo_used: false,
        synthetic_trace_used: trace_rows.iter().any(|row| row.synthetic_source == Some(true)),
        operator_rankings,
        rows,
        claim_boundary: "Real-traffic shadow analysis over recorded JSONL traces. Nando does not replace the LLM in this mode; verified savings count only when a shadow local accept is marked verified_safe_accept=true and is not an exact-cache hit. Synthetic traces force REVIEW and cannot be used as market savings claim.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!("role-binding-real-traffic-shadow-v1: {}", report.verdict);
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  total_requests: {}", report.total_requests);
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!("  nando_shadow_accepts: {}", report.nando_shadow_accepts);
    println!("  verified_safe_accepts: {}", report.verified_safe_accepts);
    println!("  false_accepts: {}", report.false_accepts);
    println!(
        "  incremental_reduction_vs_exact_cache_milli: {}",
        report.incremental_reduction_vs_exact_cache_milli
    );
    println!(
        "  p99_shadow_score_latency_ns: {}",
        report.p99_shadow_score_latency_ns
    );
    if pass {
        Ok(())
    } else {
        Err("real-traffic shadow report is review-only".to_owned())
    }
}

pub(crate) fn run_role_binding_real_traffic_cpu_route_forecast_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let route_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_REPORT));
    let shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CODEX_HISTORY_ROUTE_CANDIDATES_SHADOW_REPORT));
    let forecast_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_CPU_ROUTE_FORECAST_REPORT));

    let route_report =
        read_json_file::<RoleBindingCodexHistoryRouteCandidateReport>(&route_report_path)?;
    let shadow_report = read_json_file::<RoleBindingRealTrafficShadowReport>(&shadow_report_path)?;

    let rankings_by_route = shadow_report
        .operator_rankings
        .iter()
        .map(|ranking| (ranking.route_key.as_str(), ranking))
        .collect::<BTreeMap<_, _>>();
    let mut routes = route_report
        .route_counts
        .iter()
        .map(|route_count| {
            let ranking = rankings_by_route.get(route_count.route_key.as_str());
            let exact_cache_hits_inside_route = ranking
                .map(|ranking| ranking.exact_cache_hits)
                .unwrap_or_default();
            let current_accepts = ranking
                .map(|ranking| ranking.nando_shadow_accepts)
                .unwrap_or_default();
            let current_verified_safe_accepts = ranking
                .map(|ranking| ranking.verified_safe_accepts)
                .unwrap_or_default();
            let current_false_accepts = ranking
                .map(|ranking| ranking.false_accepts)
                .unwrap_or_default();
            let current_incremental_savings = ranking
                .map(|ranking| ranking.incremental_savings_over_exact_cache)
                .unwrap_or_default();
            let non_exact_candidate_calls = route_count
                .candidate_events
                .saturating_sub(exact_cache_hits_inside_route);
            let forecast_accept_25_percent_calls =
                projected_accepts(non_exact_candidate_calls, 250);
            let forecast_accept_50_percent_calls =
                projected_accepts(non_exact_candidate_calls, 500);
            let forecast_accept_80_percent_calls =
                projected_accepts(non_exact_candidate_calls, 800);
            let (recommended_cpu_work, recommended_payload_builder) =
                cpu_route_builder_recommendation(&route_count.route_key);
            RoleBindingCpuRouteForecastRow {
                route_key: route_count.route_key.clone(),
                profile_id: ranking
                    .map(|ranking| ranking.profile_id.clone())
                    .unwrap_or_else(|| route_count.route_key.clone()),
                candidate_events: route_count.candidate_events,
                candidate_share_milli_of_all_llm_calls: ratio_milli(
                    route_count.candidate_events,
                    shadow_report.total_llm_calls,
                ),
                candidate_share_milli_of_candidate_zone: ratio_milli(
                    route_count.candidate_events,
                    route_report.candidate_events,
                ),
                exact_cache_hits_inside_route,
                non_exact_candidate_calls,
                current_accepts,
                current_verified_safe_accepts,
                current_false_accepts,
                current_incremental_savings,
                payload_builder_status: "missing_request_side_active_fringe_and_slots".to_owned(),
                recommended_cpu_work,
                recommended_payload_builder,
                forecast_accept_25_percent_calls,
                forecast_accept_50_percent_calls,
                forecast_accept_80_percent_calls,
                priority_rank: 0,
            }
        })
        .collect::<Vec<_>>();
    routes.sort_by(|left, right| {
        right
            .non_exact_candidate_calls
            .cmp(&left.non_exact_candidate_calls)
            .then_with(|| left.route_key.cmp(&right.route_key))
    });
    for (index, route) in routes.iter_mut().enumerate() {
        route.priority_rank = index + 1;
    }

    let forecast_25_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_25_percent_calls)
        .sum();
    let forecast_50_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_50_percent_calls)
        .sum();
    let forecast_80_percent_additional_savings = routes
        .iter()
        .map(|route| route.forecast_accept_80_percent_calls)
        .sum();
    let forecast_25_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_25_percent_additional_savings);
    let forecast_50_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_50_percent_additional_savings);
    let forecast_80_percent_total_calls_removed = shadow_report
        .exact_cache_hits
        .saturating_add(forecast_80_percent_additional_savings);

    let market_claim_allowed = shadow_report.verified_safe_accepts > 0
        && shadow_report.incremental_savings_over_exact_cache > 0
        && !shadow_report.synthetic_trace_used
        && shadow_report.false_accepts == 0;
    let report = RoleBindingCpuRouteForecastReport {
        schema_version: "nando_role_binding_cpu_route_forecast_v1".to_owned(),
        verdict: if route_report.candidate_events > 0 {
            "CPU_ROUTE_FORECAST_V1_REVIEW"
        } else {
            "CPU_ROUTE_FORECAST_V1_NO_ROUTE_CANDIDATES"
        }
        .to_owned(),
        route_report_path: route_report_path.display().to_string(),
        shadow_report_path: shadow_report_path.display().to_string(),
        traffic_source: shadow_report.traffic_source.clone(),
        total_llm_calls: shadow_report.total_llm_calls,
        exact_cache_hits: shadow_report.exact_cache_hits,
        exact_cache_coverage_milli: ratio_milli(
            shadow_report.exact_cache_hits,
            shadow_report.total_llm_calls,
        ),
        operator_candidate_calls: route_report.candidate_events,
        operator_candidate_coverage_milli: ratio_milli(
            route_report.candidate_events,
            shadow_report.total_llm_calls,
        ),
        no_candidate_calls: route_report.no_candidate_events,
        current_nando_accepts: shadow_report.nando_shadow_accepts,
        current_verified_safe_accepts: shadow_report.verified_safe_accepts,
        current_false_accepts: shadow_report.false_accepts,
        current_incremental_savings_over_exact_cache: shadow_report
            .incremental_savings_over_exact_cache,
        current_incremental_reduction_vs_exact_cache_milli: shadow_report
            .incremental_reduction_vs_exact_cache_milli,
        full_shadow_request_payload_built: route_report.full_shadow_request_payload_built,
        market_claim_allowed,
        forecast_25_percent_additional_savings,
        forecast_50_percent_additional_savings,
        forecast_80_percent_additional_savings,
        forecast_25_percent_total_calls_removed,
        forecast_50_percent_total_calls_removed,
        forecast_80_percent_total_calls_removed,
        forecast_25_percent_total_reduction_milli: ratio_milli(
            forecast_25_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        forecast_50_percent_total_reduction_milli: ratio_milli(
            forecast_50_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        forecast_80_percent_total_reduction_milli: ratio_milli(
            forecast_80_percent_total_calls_removed,
            shadow_report.total_llm_calls,
        ),
        routes,
        claim_boundary: "Forecast only. It ranks real request-side CPU route candidates and estimates capacity if future payload builders create verified_safe_accepts. It is not market savings: current local accepts are zero and route candidates have empty active_fringe/slots.".to_owned(),
        next_engineering_debt: "Build request-side payload builders for priority routes: route/profile candidate -> active_fringe + slots, without response text, target labels, proof labels, or expected answer.".to_owned(),
    };

    write_json_file(&forecast_report_path, &report)?;
    println!(
        "role-binding-real-traffic-cpu-route-forecast-v1: {}",
        report.verdict
    );
    println!("  route_report: {}", route_report_path.display());
    println!("  shadow_report: {}", shadow_report_path.display());
    println!("  forecast_report: {}", forecast_report_path.display());
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!("  exact_cache_hits: {}", report.exact_cache_hits);
    println!(
        "  operator_candidate_calls: {}",
        report.operator_candidate_calls
    );
    println!(
        "  operator_candidate_coverage_milli: {}",
        report.operator_candidate_coverage_milli
    );
    println!("  current_nando_accepts: {}", report.current_nando_accepts);
    println!(
        "  forecast_50_percent_additional_savings: {}",
        report.forecast_50_percent_additional_savings
    );
    println!("  market_claim_allowed: {}", report.market_claim_allowed);
    Err("CPU route forecast is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_route_gap_catalog_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_ROUTE_GAP_CATALOG_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let sampled_llm_calls = history_rows.len().saturating_sub(skip);
    let mut existing_route_candidate_events = 0usize;
    let mut no_candidate_events = 0usize;
    let mut existing_route_counts = BTreeMap::<String, usize>::new();
    let mut gap_family_counts = BTreeMap::<String, usize>::new();

    for row in history_rows.iter().skip(skip) {
        if let Some(candidate) = route_catalog.classify_request_text(&row.text) {
            existing_route_candidate_events += 1;
            *existing_route_counts
                .entry(candidate.route_key)
                .or_insert(0) += 1;
        } else {
            no_candidate_events += 1;
            *gap_family_counts
                .entry(route_gap_family_key(&row.text).to_owned())
                .or_insert(0) += 1;
        }
    }

    let mut existing_routes = existing_route_counts
        .into_iter()
        .map(|(name, count)| RoleBindingNamedCount { name, count })
        .collect::<Vec<_>>();
    existing_routes.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });

    let mut families = gap_family_counts
        .into_iter()
        .map(|(family_key, candidate_events)| {
            let meta = route_gap_family_metadata(&family_key);
            RoleBindingRouteGapCatalogFamilyRow {
                priority_rank: 0,
                family_key,
                candidate_events,
                coverage_milli_of_all_llm_calls: ratio_milli(candidate_events, sampled_llm_calls),
                coverage_milli_of_no_candidate_zone: ratio_milli(
                    candidate_events,
                    no_candidate_events,
                ),
                cpu_operator_readiness: meta.cpu_operator_readiness.to_owned(),
                recommended_profile_line: meta.recommended_profile_line.to_owned(),
                recommended_payload_builder: meta.recommended_payload_builder.to_owned(),
                recommended_verifier: meta.recommended_verifier.to_owned(),
                claim_boundary: meta.claim_boundary.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    families.sort_by(|left, right| {
        right
            .candidate_events
            .cmp(&left.candidate_events)
            .then_with(|| left.family_key.cmp(&right.family_key))
    });
    for (index, family) in families.iter_mut().enumerate() {
        family.priority_rank = index + 1;
    }

    let top_gap_family = families.first().map(|family| family.family_key.clone());
    let top_three_no_candidate_events = families
        .iter()
        .take(3)
        .map(|family| family.candidate_events)
        .sum();
    let report = RoleBindingRouteGapCatalogReport {
        schema_version: "nando_role_binding_real_traffic_route_gap_catalog_v1".to_owned(),
        verdict: if no_candidate_events > 0 {
            "ROUTE_GAP_CATALOG_V1_REVIEW"
        } else {
            "ROUTE_GAP_CATALOG_V1_NO_GAP"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        sampled_llm_calls,
        existing_route_candidate_events,
        existing_route_coverage_milli: ratio_milli(
            existing_route_candidate_events,
            sampled_llm_calls,
        ),
        no_candidate_events,
        no_candidate_coverage_milli: ratio_milli(no_candidate_events, sampled_llm_calls),
        top_gap_family,
        top_three_no_candidate_events,
        top_three_no_candidate_coverage_milli: ratio_milli(
            top_three_no_candidate_events,
            sampled_llm_calls,
        ),
        existing_routes,
        no_candidate_families: families,
        raw_text_written: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Route-gap catalog only. It classifies privacy-safe no-candidate request families from real Codex prompts, writes no raw prompt text, enables no local accepts, and does not prove savings. Each family still needs a request-side payload builder, deterministic verifier, shadow/audit pass, and false_accepts=0 before it can contribute to CPU Routability 80.".to_owned(),
        next_engineering_debt: "Build the top no-candidate operator family as a separate profile route, then run payload readiness, dry-run scoring, output evidence, calibration, shadow, audit, and feedback-loop reports. Do not merge families into edit/conditional/mixed just to inflate routability.".to_owned(),
    };

    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-route-gap-catalog-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  sampled_llm_calls: {}", report.sampled_llm_calls);
    println!(
        "  existing_route_candidate_events: {}",
        report.existing_route_candidate_events
    );
    println!("  no_candidate_events: {}", report.no_candidate_events);
    if let Some(top_gap_family) = &report.top_gap_family {
        println!("  top_gap_family: {top_gap_family}");
    }
    println!("  raw_text_written: false");
    println!("  local_accepts_enabled: false");
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_route_gap_payload_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_ROUTE_GAP_PAYLOAD_READINESS_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let sampled_llm_calls = history_rows.len().saturating_sub(skip);
    let mut existing_route_candidate_events = 0usize;
    let mut no_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut family_accumulators = BTreeMap::<String, RouteGapPayloadFamilyAccumulator>::new();
    let mut rows = Vec::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        if route_catalog.classify_request_text(&row.text).is_some() {
            existing_route_candidate_events += 1;
            continue;
        }

        no_candidate_events += 1;
        let family_key = route_gap_family_key(&row.text);
        let meta = route_gap_family_metadata(family_key);
        let readiness = analyze_route_gap_payload_readiness(family_key, &row.text);
        payload_ready_events += usize::from(readiness.payload_ready);
        let accumulator = family_accumulators
            .entry(family_key.to_owned())
            .or_default();
        accumulator.candidate_events += 1;
        accumulator.payload_ready_events += usize::from(readiness.payload_ready);
        accumulator.request_signal_events += usize::from(readiness.has_request_signal);
        accumulator.context_signal_events += usize::from(readiness.has_context_signal);
        accumulator.evidence_signal_events += usize::from(readiness.has_evidence_signal);
        accumulator.verifier_signal_events += usize::from(readiness.has_verifier_signal);
        accumulator
            .builder_kind_counts
            .entry(readiness.recommended_builder_kind.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        rows.push(RoleBindingRouteGapPayloadReadinessEventRow {
            event_id: format!(
                "codex_history_route_gap_payload_readiness::{}::{}::{}",
                row.session_id, row.ts, index
            ),
            request_fingerprint: format!("fnv1a64:{fingerprint:016x}"),
            family_key: family_key.to_owned(),
            cpu_operator_readiness: meta.cpu_operator_readiness.to_owned(),
            has_request_signal: readiness.has_request_signal,
            has_context_signal: readiness.has_context_signal,
            has_evidence_signal: readiness.has_evidence_signal,
            has_verifier_signal: readiness.has_verifier_signal,
            payload_ready: readiness.payload_ready,
            recommended_payload_builder: meta.recommended_payload_builder.to_owned(),
            recommended_verifier: meta.recommended_verifier.to_owned(),
            recommended_builder_kind: readiness.recommended_builder_kind,
            missing_reasons: readiness.missing_reasons,
        });
    }

    let mut families = family_accumulators
        .into_iter()
        .map(|(family_key, accumulator)| {
            let meta = route_gap_family_metadata(&family_key);
            let dominant_builder_kind = accumulator
                .builder_kind_counts
                .iter()
                .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
                .map(|(name, _)| name.clone())
                .unwrap_or_else(|| "no_rows".to_owned());
            RoleBindingRouteGapPayloadReadinessFamilyRow {
                priority_rank: 0,
                family_key,
                candidate_events: accumulator.candidate_events,
                payload_ready_events: accumulator.payload_ready_events,
                payload_ready_rate_milli: ratio_milli(
                    accumulator.payload_ready_events,
                    accumulator.candidate_events,
                ),
                request_signal_events: accumulator.request_signal_events,
                context_signal_events: accumulator.context_signal_events,
                evidence_signal_events: accumulator.evidence_signal_events,
                verifier_signal_events: accumulator.verifier_signal_events,
                cpu_operator_readiness: meta.cpu_operator_readiness.to_owned(),
                recommended_profile_line: meta.recommended_profile_line.to_owned(),
                recommended_payload_builder: meta.recommended_payload_builder.to_owned(),
                recommended_verifier: meta.recommended_verifier.to_owned(),
                dominant_builder_kind,
                claim_boundary: meta.claim_boundary.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    families.sort_by(|left, right| {
        right
            .payload_ready_events
            .cmp(&left.payload_ready_events)
            .then_with(|| right.candidate_events.cmp(&left.candidate_events))
            .then_with(|| left.family_key.cmp(&right.family_key))
    });
    for (index, family) in families.iter_mut().enumerate() {
        family.priority_rank = index + 1;
    }

    let top_payload_ready_family = families
        .iter()
        .find(|family| family.payload_ready_events > 0)
        .map(|family| family.family_key.clone());
    let report = RoleBindingRouteGapPayloadReadinessReport {
        schema_version: "nando_role_binding_route_gap_payload_readiness_v1".to_owned(),
        verdict: if payload_ready_events > 0 {
            "ROUTE_GAP_PAYLOAD_READINESS_V1_REVIEW_READY_FAMILIES_FOUND"
        } else if no_candidate_events > 0 {
            "ROUTE_GAP_PAYLOAD_READINESS_V1_REVIEW_NO_READY_FAMILIES"
        } else {
            "ROUTE_GAP_PAYLOAD_READINESS_V1_NO_GAP"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        total_history_rows: history_rows.len(),
        max_events,
        sampled_llm_calls,
        existing_route_candidate_events,
        no_candidate_events,
        payload_ready_events,
        payload_ready_rate_milli: ratio_milli(payload_ready_events, no_candidate_events),
        top_payload_ready_family,
        families,
        rows,
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Route-gap payload readiness only. It classifies privacy-safe no-candidate real Codex prompts into request-side builder readiness, writes fingerprints/features/counts but no raw prompt text, uses no response/target/proof labels, enables no local accepts, and cannot prove savings.".to_owned(),
        next_engineering_debt: "Pick the top deterministic ready family, implement its request-side active_fringe/slot dry-run builder and deterministic verifier, then rerun shadow, hook audit, safe-policy calibration, feedback-loop, and CPU operator catalog. Do not promote answer_or_explain/project_context_dialogue without grounded evidence.".to_owned(),
    };

    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-route-gap-payload-readiness-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  sampled_llm_calls: {}", report.sampled_llm_calls);
    println!(
        "  existing_route_candidate_events: {}",
        report.existing_route_candidate_events
    );
    println!("  no_candidate_events: {}", report.no_candidate_events);
    println!("  payload_ready_events: {}", report.payload_ready_events);
    if let Some(family) = &report.top_payload_ready_family {
        println!("  top_payload_ready_family: {family}");
    }
    println!("  raw_text_written: false");
    println!("  local_accepts_enabled: false");
    Err("route-gap payload readiness is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let profile_registered = registry_config
        .profiles
        .iter()
        .any(|profile| profile.profile_id == REAL_TRAFFIC_PLANNING_PROFILE_ID);
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut planning_next_step_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut readiness_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;
    let mut builder_status_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_planning_next_step_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "not planning_next_step route-gap candidate".to_owned();

        if route_catalog.classify_request_text(&row.text).is_none()
            && route_gap_family_key(&row.text) == REAL_TRAFFIC_PLANNING_ROUTE_KEY
        {
            planning_next_step_candidate_events += 1;
            let readiness =
                analyze_route_gap_payload_readiness(REAL_TRAFFIC_PLANNING_ROUTE_KEY, &row.text);
            if readiness.payload_ready {
                payload_ready_events += 1;
                let built =
                    build_planning_next_step_dry_run_request(&event_id, &fingerprint, &row.text);
                match built {
                    Some(request) => {
                        let active_fringe_centers = request.active_fringe.len();
                        let slots = request.slots.len();
                        let positive_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.positive_impulses.len())
                            .sum::<usize>();
                        let negative_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.negative_impulses.len())
                            .sum::<usize>();
                        let scoreable = active_fringe_centers > 0 && slots > 0;
                        payload_built_events += 1;
                        scoreable_payload_events += usize::from(scoreable);
                        active_fringe_centers_total += active_fringe_centers;
                        slots_total += slots;
                        positive_impulses_total += positive_impulses;
                        negative_impulses_total += negative_impulses;
                        let builder_status = if scoreable && profile_registered {
                            "scoreable_payload_built_profile_registered"
                        } else if scoreable {
                            "scoreable_payload_built_profile_missing"
                        } else {
                            "payload_built_but_not_scoreable"
                        }
                        .to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingPlanningNextStepPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: REAL_TRAFFIC_PLANNING_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                        });
                        notes = format!(
                            "request-side planning-next-step payload built; status={builder_status}; verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingPlanningNextStepPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: REAL_TRAFFIC_PLANNING_ROUTE_KEY.to_owned(),
                            profile_id: REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            profile_registered,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "readiness_rejected".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = format!(
                    "planning_next_step route-gap candidate rejected by readiness gate: {}",
                    readiness.missing_reasons.join(",")
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some(
                "codex_history_local_planning_next_step_payload_dry_run".to_owned(),
            ),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side planning-next-step payload dry-run from local Codex prompt only; raw text, response text, target labels, and proof labels not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let shadow_score_ready = profile_registered && scoreable_payload_events > 0;
    let report = RoleBindingPlanningNextStepPayloadDryRunReport {
        schema_version: "nando_role_binding_planning_next_step_payload_dry_run_v1".to_owned(),
        verdict: if shadow_score_ready {
            "PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PROFILE_READY"
        } else if scoreable_payload_events > 0 {
            "PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_PROFILE_MISSING"
        } else {
            "PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        planning_next_step_candidate_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        profile_registered,
        shadow_score_ready,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side dry-run payload builder only. It emits active_fringe/slots for planning_next_step route-gap rows from prompt text only, keeps verified_safe_accept=None and expect_local_operator=false, and cannot prove savings. A registered planning .nwrb profile plus deterministic plan/artifact verifier are required before any local accept.".to_owned(),
        next_engineering_debt: "Build a planning-next-step .nwrb profile package, rerun shadow so profile_missing fallback becomes real score, then attach plan_step_artifact_progress_verifier_v1 before any safe-policy promotion or market claim.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  planning_next_step_candidate_events: {}",
        report.planning_next_step_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  profile_registered: {}", report.profile_registered);
    println!("  local_accepts_enabled: false");
    Err(
        "planning-next-step payload dry-run is review-only; build profile+verifier before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_profile_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let dry_run_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PACKAGE_PATH));
    let registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PROFILE_REPORT));

    let mut registry = read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_path)?;
    validate_registry_config(&registry)?;
    let trace_rows = read_real_traffic_trace_jsonl(&dry_run_trace_path)?;
    let build = build_planning_next_step_role_binding_package_from_trace(&trace_rows)?;
    if let Some(parent) = package_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create planning-next-step package directory {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&package_path, &build.package_bytes).map_err(|error| {
        format!(
            "failed to write planning-next-step package {}: {error}",
            package_path.display()
        )
    })?;
    let package_info =
        WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&build.package_bytes)
            .map_err(|error| format!("failed to inspect planning-next-step package: {error:?}"))?;
    let policy =
        WavePredictorRoleBindingOffloadPolicy::new(REAL_TRAFFIC_PLANNING_DISABLED_THRESHOLD)
            .map_err(|error| format!("invalid planning-next-step disabled policy: {error:?}"))?;
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
        &build.package_bytes,
        policy,
    )
    .map_err(|error| format!("failed to load planning-next-step package: {error:?}"))?;

    let planning_requests = planning_next_step_scoreable_requests(&trace_rows);
    let mut energy_margins = Vec::with_capacity(planning_requests.len());
    let mut min_slot_margins = Vec::with_capacity(planning_requests.len());
    let mut positive_margin_rows = 0usize;
    let mut strict_ordered_pass_rows = 0usize;
    let mut unexpected_local_accepts_under_disabled_threshold = 0usize;
    for request in &planning_requests {
        let prepared = sdk.prepare_active_fringe_from_iter(
            request
                .active_fringe
                .iter()
                .map(|active| (active.center_id, active.strength)),
        );
        let mut energy_margin = 0i32;
        let mut min_slot_margin = i32::MAX;
        let mut strict_ordered_pass = true;
        for slot in &request.slots {
            let (positive_score, negative_score) =
                score_role_binding_profile_slot(&sdk, &prepared, slot);
            let slot_margin = positive_score - negative_score;
            energy_margin = energy_margin.saturating_add(slot_margin);
            min_slot_margin = min_slot_margin.min(slot_margin);
            strict_ordered_pass &= slot_margin > 0;
        }
        if min_slot_margin == i32::MAX {
            continue;
        }
        positive_margin_rows += usize::from(energy_margin > 0);
        strict_ordered_pass_rows += usize::from(strict_ordered_pass);
        unexpected_local_accepts_under_disabled_threshold += usize::from(profile_accepts_score(
            &default_profile_acceptance_policy(),
            strict_ordered_pass,
            energy_margin,
            REAL_TRAFFIC_PLANNING_DISABLED_THRESHOLD,
        ));
        energy_margins.push(energy_margin);
        min_slot_margins.push(min_slot_margin);
    }

    let profile = RoleBindingProfileConfig {
        profile_id: REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
        profile_kind: "role_binding_nwrb".to_owned(),
        operator_classes: vec![
            "project_planning".to_owned(),
            "state_transition".to_owned(),
            "route_gap".to_owned(),
        ],
        package_path: package_path.clone(),
        runtime_bytes_estimate: sdk.bytes_estimate(),
        edge_count: package_info.edge_count,
        slot_count: 2,
        threshold: REAL_TRAFFIC_PLANNING_DISABLED_THRESHOLD,
        acceptance_policy: default_profile_acceptance_policy(),
        accepted_route_keys: vec![
            REAL_TRAFFIC_PLANNING_ROUTE_KEY.to_owned(),
            REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
            "goal_state_transition_payload_builder_v1".to_owned(),
        ],
    };
    registry
        .profiles
        .retain(|existing| existing.profile_id != profile.profile_id);
    registry.profiles.push(profile);
    registry.claim_boundary = "serving registry overlay for planning-next-step .nwrb profile; generated from request-side dry-run payloads with threshold=i32::MAX so scoring telemetry is available but local accepts remain disabled until deterministic plan/artifact verification exists".to_owned();
    validate_registry_config(&registry)?;
    write_json_file(&registry_path, &registry)?;

    let mut sorted_energy = energy_margins.clone();
    let mut sorted_min_slot = min_slot_margins.clone();
    sorted_energy.sort_unstable();
    sorted_min_slot.sort_unstable();
    let report = RoleBindingPlanningNextStepProfileReport {
        schema_version: "nando_role_binding_planning_next_step_profile_v1".to_owned(),
        verdict: if unexpected_local_accepts_under_disabled_threshold == 0
            && build.package_training_requests > 0
            && package_info.edge_count > 0
        {
            "PLANNING_NEXT_STEP_PROFILE_V1_REVIEW_PROFILE_READY_ACCEPTS_DISABLED"
        } else {
            "PLANNING_NEXT_STEP_PROFILE_V1_REVIEW_REPAIR_REQUIRED"
        }
        .to_owned(),
        base_registry_path: base_registry_path.display().to_string(),
        dry_run_trace_path: dry_run_trace_path.display().to_string(),
        package_path: package_path.display().to_string(),
        registry_path: registry_path.display().to_string(),
        profile_id: REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: build.package_bytes.len(),
        edge_count: package_info.edge_count,
        runtime_bytes_estimate: sdk.bytes_estimate(),
        threshold: REAL_TRAFFIC_PLANNING_DISABLED_THRESHOLD,
        trace_rows_read: trace_rows.len(),
        scoreable_payload_events: planning_requests.len(),
        package_training_requests: build.package_training_requests,
        positive_updates: build.positive_updates,
        negative_updates: build.negative_updates,
        changed_edges: build.changed_edges,
        positive_margin_rows,
        strict_ordered_pass_rows,
        unexpected_local_accepts_under_disabled_threshold,
        median_energy_margin: percentile_i32_sorted(&sorted_energy, 50),
        p10_energy_margin: percentile_i32_sorted(&sorted_energy, 10),
        min_energy_margin: sorted_energy.first().copied().unwrap_or(0),
        median_min_slot_margin: percentile_i32_sorted(&sorted_min_slot, 50),
        p10_min_slot_margin: percentile_i32_sorted(&sorted_min_slot, 10),
        min_slot_margin: sorted_min_slot.first().copied().unwrap_or(0),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled_on_real_traffic: false,
        market_claim_allowed: false,
        claim_boundary: "Profile generator only. It compiles request-side planning-next-step payload geometry into a .nwrb package and registry overlay with threshold=i32::MAX, so shadow can measure real score/margins but cannot local-accept. Verified CPU savings require deterministic plan/artifact evidence, safe-policy calibration, shadow/audit pass, provider cost, and false_accepts=0.".to_owned(),
        next_engineering_debt: "Run planning dry-run shadow with this overlay registry, then build plan_step_artifact_progress_verifier_v1 before lowering thresholds or promoting any local accept path.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-profile-v1: {}",
        report.verdict
    );
    println!("  base_registry: {}", base_registry_path.display());
    println!("  dry_run_trace: {}", dry_run_trace_path.display());
    println!("  package: {}", package_path.display());
    println!("  registry: {}", registry_path.display());
    println!("  report: {}", report_path.display());
    println!("  edge_count: {}", report.edge_count);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  median_energy_margin: {}", report.median_energy_margin);
    println!(
        "  unexpected_local_accepts_under_disabled_threshold: {}",
        report.unexpected_local_accepts_under_disabled_threshold
    );
    println!("  local_accepts_enabled_on_real_traffic: false");
    Err("planning-next-step profile is review-only; attach verifier before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| {
            row.nando_shadow_request.as_ref().is_some_and(|request| {
                request.profile_id.as_deref() == Some(REAL_TRAFFIC_PLANNING_PROFILE_ID)
            })
        })
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_planning_next_step_output_verification,
    )?;

    let mut enriched_rows = Vec::with_capacity(trace_rows.len());
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;

    for mut row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            enriched_rows.push(row);
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_PLANNING_PROFILE_ID) {
            enriched_rows.push(row);
            continue;
        }
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_output_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "planning output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_planning_next_step_output_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        deterministic_verification_events += usize::from(evidence.verifier_applicable);
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        verifier_not_applicable_events += usize::from(!evidence.verifier_applicable);
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "planning output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_planning_next_step_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: enriched_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Planning-next-step output evidence join only. It reads local Codex final answers at analysis time, writes response fingerprints and explicit deterministic verification results, writes no raw prompt/response text, and intentionally refuses true verification until an artifact-progress verifier proves the plan step changed project state.".to_owned(),
        next_engineering_debt: "Build plan_step_artifact_progress_verifier_v1 over git diff/report/test/commit artifacts, then run shadow/audit again before any planning threshold can be lowered.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    Err("planning-next-step output evidence is review-only; artifact verifier required".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_artifact_progress_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_OUTPUT_EVIDENCE_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_REPORT));

    let mut trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| {
            row.nando_shadow_request.as_ref().is_some_and(|request| {
                request.profile_id.as_deref() == Some(REAL_TRAFFIC_PLANNING_PROFILE_ID)
            })
        })
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let artifact_index = build_codex_session_planning_artifact_progress_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
    )?;

    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut artifact_evidence_matched_events = 0usize;
    let mut no_session_artifact_match_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;
    let mut tool_call_fingerprint_events = 0usize;

    for row in &mut trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_PLANNING_PROFILE_ID) {
            continue;
        }
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = artifact_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_artifact_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "planning artifact-progress evidence missing: no matching Codex turn found",
            ));
            continue;
        };
        artifact_evidence_matched_events += 1;
        if row.response_fingerprint.is_none() {
            row.response_fingerprint = evidence.response_fingerprint.clone();
        }
        row.tool_call_fingerprints = evidence.tool_call_fingerprints.clone();
        tool_call_fingerprint_events += usize::from(!row.tool_call_fingerprints.is_empty());
        row.verification_source = Some(
            "codex_session_tool_call_fingerprints_plus_plan_step_artifact_progress_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        verifier_not_applicable_events += usize::from(!evidence.verifier_applicable);
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "planning artifact-progress evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &trace_rows)?;
    let report = RoleBindingPlanningNextStepArtifactProgressReport {
        schema_version: "nando_role_binding_planning_next_step_artifact_progress_v1".to_owned(),
        verdict: if verified_true_events > 0 {
            "PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_V1_REVIEW_TOOL_BACKED_TRUE_LABELS_FOUND"
        } else if artifact_evidence_matched_events > 0 {
            "PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_V1_REVIEW_ARTIFACT_EVIDENCE_ATTACHED"
        } else {
            "PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_V1_REVIEW_NO_ARTIFACT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: trace_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: artifact_index.session_files_scanned,
        codex_turns_indexed: artifact_index.codex_turns_indexed,
        tool_events_indexed: artifact_index.tool_events_indexed,
        artifact_evidence_matched_events,
        no_session_artifact_match_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        tool_call_fingerprint_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        tool_outputs_written: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Planning artifact-progress verification only. It scans local Codex session tool events, writes tool-call fingerprints instead of raw tool output, and marks true only for successful nando-wave project-progress tools such as apply_patch, commits, generated reports, or structural gates. It does not enable local accepts or prove market savings by itself.".to_owned(),
        next_engineering_debt: "Run shadow/audit over the artifact-progress trace, then calibrate a planning safe policy only if true labels exist, provider cost is attached, and false accepts remain zero.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-artifact-progress-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  artifact_evidence_matched_events: {}",
        report.artifact_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!(
        "  tool_call_fingerprint_events: {}",
        report.tool_call_fingerprint_events
    );
    Err("planning artifact-progress verification is review-only; run shadow/audit".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_local_accept_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_TRACE_JSONL));
    let report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_REPORT)
    });

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut scored_rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut no_score_rows = 0usize;

    for row in &trace_rows {
        let Some(label) = row.verified_safe_accept else {
            continue;
        };
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_PLANNING_PROFILE_ID) {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let Some(score) = score_role_binding_profile_request_detailed(&registry, request) else {
            no_score_rows += 1;
            continue;
        };
        let current_response = score_role_binding_profile_request(&registry, request);
        let progress_slot_margin = score.slot_margins.first().copied().unwrap_or(0);
        let boundary_slot_margin = score.slot_margins.get(1).copied().unwrap_or(0);
        scored_rows.push(RoleBindingEditLocalAcceptCalibrationRow {
            trace_id: row.trace_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            response_fingerprint: row.response_fingerprint.clone(),
            verifier_label: label,
            production_accepted: current_response.accepted,
            production_fallback_reason: current_response.fallback_reason,
            energy_margin: score.energy_margin,
            min_slot_margin: score.min_slot_margin,
            marker_slot_margin: progress_slot_margin,
            end_slot_margin: boundary_slot_margin,
            slot_count: score.slot_margins.len(),
        });
    }

    let current_policy =
        evaluate_edit_calibration_policy("current_disabled_profile_policy", &scored_rows, |row| {
            row.production_accepted
        });
    let energy_positive_policy =
        evaluate_edit_calibration_policy("energy_positive_no_slot_order", &scored_rows, |row| {
            row.energy_margin >= 1
        });
    let strict_positive_policy = evaluate_edit_calibration_policy(
        "strict_positive_slots_and_energy_positive",
        &scored_rows,
        |row| row.min_slot_margin > 0 && row.energy_margin >= 1,
    );
    let progress_slot_policy =
        evaluate_edit_calibration_policy("progress_slot_positive_only", &scored_rows, |row| {
            row.marker_slot_margin > 0 && row.energy_margin >= 1
        });
    let boundary_slot_policy =
        evaluate_edit_calibration_policy("boundary_slot_positive_only", &scored_rows, |row| {
            row.end_slot_margin > 0 && row.energy_margin >= 1
        });
    let best_energy_threshold_policy = best_single_threshold_policy(
        "best_energy_margin_threshold_request_side_only",
        &scored_rows,
        |row| row.energy_margin,
    );
    let best_min_slot_threshold_policy = best_single_threshold_policy(
        "best_min_slot_margin_threshold_request_side_only",
        &scored_rows,
        |row| row.min_slot_margin,
    );
    let best_progress_slot_threshold_policy = best_single_threshold_policy(
        "best_progress_slot_margin_threshold_request_side_only",
        &scored_rows,
        |row| row.marker_slot_margin,
    );
    let best_boundary_slot_threshold_policy = best_single_threshold_policy(
        "best_boundary_slot_margin_threshold_request_side_only",
        &scored_rows,
        |row| row.end_slot_margin,
    );
    let margin_collision_diagnostics = planning_margin_collision_diagnostics(&scored_rows);
    let request_side_margin_only_accepts_all_true_without_false = margin_collision_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.safe_accepts_all_true_rows);
    let policies = vec![
        current_policy,
        energy_positive_policy,
        strict_positive_policy,
        progress_slot_policy,
        boundary_slot_policy,
        best_energy_threshold_policy,
        best_min_slot_threshold_policy,
        best_progress_slot_threshold_policy,
        best_boundary_slot_threshold_policy,
    ];
    let safe_policy_found = policies
        .iter()
        .any(|policy| policy.false_accepts == 0 && policy.true_accepts > 0);
    let best_safe_true_accepts = policies
        .iter()
        .filter(|policy| policy.false_accepts == 0)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let report = RoleBindingEditLocalAcceptCalibrationReport {
        schema_version: "nando_role_binding_planning_next_step_local_accept_calibration_v1"
            .to_owned(),
        verdict: if safe_policy_found {
            "PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND"
        } else {
            "PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_REQUEST_SIDE_POLICY"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        hook_ready_rows,
        scored_rows: scored_rows.len(),
        label_true_rows,
        label_false_rows,
        no_score_rows,
        safe_policy_found,
        best_safe_true_accepts,
        policies,
        rows: scored_rows,
        margin_collision_diagnostics,
        request_side_margin_only_accepts_all_true_without_false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Planning-next-step calibration only. It evaluates request-side score/readout policies against tool-backed artifact-progress labels, writes only fingerprints and margins, enables no local accepts, and cannot be used as a market savings claim. Tool-call fingerprints are verifier evidence, not runtime admission features.".to_owned(),
        next_engineering_debt: if safe_policy_found {
            "Do not promote from this singleton calibration alone. Require more non-synthetic true labels, provider cost, shadow/audit with false_accepts=0, and a separate promoted registry/trace before counting CPU savings.".to_owned()
        } else {
            "Do not lower the planning threshold. Current planning score geometry does not separate tool-backed true progress from false/unverified planning rows; improve request-side admission or payload features before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-local-accept-calibration-v1: {}",
        report.verdict
    );
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!("  safe_policy_found: {}", report.safe_policy_found);
    println!(
        "  best_safe_true_accepts: {}",
        report.best_safe_true_accepts
    );
    Err("planning-next-step local accept calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_planning_next_step_admission_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_TRACE_JSONL));
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;

    for trace in &trace_rows {
        let Some(label) = trace.verified_safe_accept else {
            continue;
        };
        let Some(request) = &trace.nando_shadow_request else {
            continue;
        };
        if request.profile_id.as_deref() != Some(REAL_TRAFFIC_PLANNING_PROFILE_ID) {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let request_fingerprint = trace.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            continue;
        };
        let features = extract_planning_next_step_admission_features(prompt_text);
        rows.push(RoleBindingPlanningNextStepAdmissionCalibrationRow {
            trace_id: trace.trace_id.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            verifier_label: label,
            features,
        });
    }

    let minimum_true_support = 2usize;
    let policies = planning_next_step_admission_policy_reports(&rows, minimum_true_support);
    let robust_safe_policy_found = policies.iter().any(|policy| policy.robust_safe);
    let singleton_safe_policy_found = policies.iter().any(|policy| policy.singleton_safe);
    let best_robust_true_accepts = policies
        .iter()
        .filter(|policy| policy.robust_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let best_singleton_true_accepts = policies
        .iter()
        .filter(|policy| policy.singleton_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let feature_counts = planning_next_step_admission_feature_counts(&rows);
    let report = RoleBindingPlanningNextStepAdmissionCalibrationReport {
        schema_version: "nando_role_binding_planning_next_step_admission_calibration_v1"
            .to_owned(),
        verdict: if robust_safe_policy_found {
            "PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND"
        } else if singleton_safe_policy_found {
            "PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY"
        } else {
            "PLANNING_NEXT_STEP_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY"
        }
        .to_owned(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        hook_ready_rows,
        rows_with_prompt_features: rows.len(),
        history_prompt_missing_rows,
        label_true_rows,
        label_false_rows,
        minimum_true_support,
        robust_safe_policy_found,
        singleton_safe_policy_found,
        best_robust_true_accepts,
        best_singleton_true_accepts,
        feature_counts,
        policies,
        rows,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Planning-next-step admission calibration only. It reads real request text at analysis time, writes only fingerprints/features/counts, uses verification labels only to evaluate request-side gates, enables no local accepts, and cannot be used as a market savings claim.".to_owned(),
        next_engineering_debt: if robust_safe_policy_found {
            "Use the robust request-side admission candidate only in a separate promoted shadow trace with provider cost, false_accepts=0, unverified_shadow_accepts=0, and explicit rollback. It still needs shadow/audit before counting CPU savings.".to_owned()
        } else {
            "Current planning prompt-side features do not robustly separate tool-backed progress from false planning rows. Improve admission features or capture richer request-side state before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-planning-next-step-admission-calibration-v1: {}",
        report.verdict
    );
    println!("  evidence_trace: {}", evidence_trace_path.display());
    println!("  history: {}", history_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!(
        "  rows_with_prompt_features: {}",
        report.rows_with_prompt_features
    );
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!(
        "  robust_safe_policy_found: {}",
        report.robust_safe_policy_found
    );
    println!(
        "  best_robust_true_accepts: {}",
        report.best_robust_true_accepts
    );
    Err("planning-next-step admission calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_cpu_operator_catalog_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let feedback_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        let extended = PathBuf::from(DEFAULT_REAL_TRAFFIC_FEEDBACK_LOOP_EXTENDED_REPORT);
        if extended.exists() {
            extended
        } else {
            PathBuf::from(DEFAULT_REAL_TRAFFIC_FEEDBACK_LOOP_REPORT)
        }
    });
    let route_gap_report_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        let agent_control =
            PathBuf::from(DEFAULT_REAL_TRAFFIC_ROUTE_GAP_CATALOG_AGENT_CONTROL_REPORT);
        if agent_control.exists() {
            agent_control
        } else {
            PathBuf::from(DEFAULT_REAL_TRAFFIC_ROUTE_GAP_CATALOG_REPORT)
        }
    });
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_CPU_OPERATOR_CATALOG_REPORT));
    let route_gap_payload_readiness_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_ROUTE_GAP_PAYLOAD_READINESS_REPORT));

    let feedback = read_json_file::<RoleBindingFeedbackLoopReport>(&feedback_report_path)?;
    let route_gap = read_json_file::<RoleBindingRouteGapCatalogReport>(&route_gap_report_path)?;
    let route_gap_payload_readiness = if route_gap_payload_readiness_report_path.exists() {
        Some(read_json_file::<RoleBindingRouteGapPayloadReadinessReport>(
            &route_gap_payload_readiness_report_path,
        )?)
    } else {
        None
    };
    let route_gap_readiness_by_family = route_gap_payload_readiness
        .as_ref()
        .map(|report| {
            report
                .families
                .iter()
                .map(|family| (family.family_key.as_str(), family))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let target_verified_cpu_calls =
        projected_accepts(feedback.total_llm_calls, feedback.target_routability_milli);
    let mut rows = Vec::new();

    for route in &feedback.routes {
        let recommended_verifier = existing_route_verifier(route.route_key.as_str());
        let priority_score = cpu_operator_priority_score(
            route.candidate_events,
            route.scoreable_payload_events,
            route.verification_hook_ready_events,
            route.verified_cpu_accept_eligible_events,
            route.false_accepts,
            "existing_profile",
        );
        rows.push(RoleBindingCpuOperatorCatalogRow {
            priority_rank: 0,
            source_kind: "existing_profile_route".to_owned(),
            route_or_family_key: route.route_key.clone(),
            profile_id: Some(route.profile_id.clone()),
            candidate_events: route.candidate_events,
            payload_ready_events: route.payload_ready_events,
            scoreable_payload_events: route.scoreable_payload_events,
            verification_hook_ready_events: route.verification_hook_ready_events,
            verified_cpu_accept_eligible_events: route.verified_cpu_accept_eligible_events,
            false_accepts: route.false_accepts,
            cpu_operator_readiness: "existing_profile".to_owned(),
            recommended_profile_line: format!("existing_profile:{}", route.profile_id),
            recommended_payload_builder: route.payload_builder.clone(),
            recommended_verifier,
            next_action: route.next_action.clone(),
            priority_score,
            market_claim_allowed: false,
            claim_boundary: "Existing routed profile. Only hook-backed local accepts with false_accepts=0 count toward CPU Routability; candidate or scoreable rows alone are not market savings.".to_owned(),
        });
    }

    for family in &route_gap.no_candidate_families {
        let readiness = route_gap_readiness_by_family
            .get(family.family_key.as_str())
            .copied();
        let payload_ready_events = readiness
            .map(|family| family.payload_ready_events)
            .unwrap_or_default();
        let priority_score = cpu_operator_priority_score(
            family.candidate_events,
            payload_ready_events,
            0,
            0,
            0,
            family.cpu_operator_readiness.as_str(),
        );
        rows.push(RoleBindingCpuOperatorCatalogRow {
            priority_rank: 0,
            source_kind: "route_gap_family".to_owned(),
            route_or_family_key: family.family_key.clone(),
            profile_id: None,
            candidate_events: family.candidate_events,
            payload_ready_events,
            scoreable_payload_events: 0,
            verification_hook_ready_events: 0,
            verified_cpu_accept_eligible_events: 0,
            false_accepts: 0,
            cpu_operator_readiness: family.cpu_operator_readiness.clone(),
            recommended_profile_line: family.recommended_profile_line.clone(),
            recommended_payload_builder: family.recommended_payload_builder.clone(),
            recommended_verifier: family.recommended_verifier.clone(),
            next_action: format!(
                "Build {} + {}; keep local accepts disabled until deterministic verification exists.",
                family.recommended_payload_builder, family.recommended_verifier
            ),
            priority_score,
            market_claim_allowed: false,
            claim_boundary: family.claim_boundary.clone(),
        });
    }

    rows.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| right.candidate_events.cmp(&left.candidate_events))
            .then_with(|| left.route_or_family_key.cmp(&right.route_or_family_key))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.priority_rank = index + 1;
    }

    let top_actionable_rows = rows
        .iter()
        .filter(|row| {
            row.cpu_operator_readiness.starts_with("medium")
                || row.cpu_operator_readiness.starts_with("high")
                || row.cpu_operator_readiness == "existing_profile"
        })
        .take(8)
        .cloned()
        .collect::<Vec<_>>();
    let top_gap_family = route_gap.top_gap_family.clone();
    let report = RoleBindingCpuOperatorCatalogReport {
        schema_version: "nando_role_binding_cpu_operator_catalog_v1".to_owned(),
        verdict: "CPU_OPERATOR_CATALOG_V1_REVIEW".to_owned(),
        feedback_report_path: feedback_report_path.display().to_string(),
        route_gap_report_path: route_gap_report_path.display().to_string(),
        route_gap_payload_readiness_report_path: route_gap_payload_readiness.as_ref().map(|_| {
            route_gap_payload_readiness_report_path
                .display()
                .to_string()
        }),
        total_llm_calls: feedback.total_llm_calls,
        exact_cache_hits: feedback.exact_cache_hits,
        existing_operator_candidate_calls: feedback.operator_candidate_calls,
        no_candidate_calls: route_gap.no_candidate_events,
        route_gap_payload_ready_events: route_gap_payload_readiness
            .as_ref()
            .map(|report| report.payload_ready_events)
            .unwrap_or_default(),
        current_verified_cpu_accepts: feedback.verified_cpu_accept_eligible_events,
        target_verified_cpu_accepts: target_verified_cpu_calls,
        verified_gap_to_80_calls: target_verified_cpu_calls
            .saturating_sub(feedback.verified_cpu_accept_eligible_events),
        top_gap_family,
        top_actionable_rows,
        rows,
        raw_text_written: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "CPU operator catalog only. It ranks existing profile routes and no-candidate route-gap families from non-synthetic Codex traffic reports, writes no raw prompt/response text, enables no local accepts, and cannot prove market savings. Rows become savings only after request-side payload, deterministic verifier evidence, shadow accept, provider-cost evidence, and false_accepts=0.".to_owned(),
        next_engineering_debt: "Pick the highest-volume row whose verifier can be deterministic. Build its request-side payload builder and verifier as a separate route, then rerun shadow/audit/feedback-loop. Do not promote answer_or_explain or project_context_dialogue without grounded evidence.".to_owned(),
    };

    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-cpu-operator-catalog-v1: {}",
        report.verdict
    );
    println!("  feedback_report: {}", feedback_report_path.display());
    println!("  route_gap_report: {}", route_gap_report_path.display());
    if route_gap_payload_readiness.is_some() {
        println!(
            "  route_gap_payload_readiness_report: {}",
            route_gap_payload_readiness_report_path.display()
        );
    }
    println!("  report: {}", report_path.display());
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!(
        "  existing_operator_candidate_calls: {}",
        report.existing_operator_candidate_calls
    );
    println!("  no_candidate_calls: {}", report.no_candidate_calls);
    println!(
        "  route_gap_payload_ready_events: {}",
        report.route_gap_payload_ready_events
    );
    println!(
        "  current_verified_cpu_accepts: {}",
        report.current_verified_cpu_accepts
    );
    println!(
        "  verified_gap_to_80_calls: {}",
        report.verified_gap_to_80_calls
    );
    if let Some(row) = report.rows.first() {
        println!(
            "  top_catalog_row: {} ({})",
            row.route_or_family_key, row.source_kind
        );
    }
    Err("CPU operator catalog is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_agent_control_profile_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let package_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PACKAGE_PATH));
    let registry_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REPORT));

    let mut registry = read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_path)?;
    validate_registry_config(&registry)?;

    let package_bytes = build_agent_control_role_binding_package()?;
    fs::create_dir_all(package_path.parent().ok_or_else(|| {
        format!(
            "agent control package path has no parent: {}",
            package_path.display()
        )
    })?)
    .map_err(|error| {
        format!(
            "failed to create agent control package directory {}: {error}",
            package_path.display()
        )
    })?;
    fs::write(&package_path, &package_bytes).map_err(|error| {
        format!(
            "failed to write agent control package {}: {error}",
            package_path.display()
        )
    })?;
    let package_info =
        WavePredictorRoleBindingOffloadRuntime::inspect_package_bytes(&package_bytes)
            .map_err(|error| format!("failed to inspect agent control package: {error:?}"))?;
    let policy = WavePredictorRoleBindingOffloadPolicy::new(REAL_TRAFFIC_AGENT_CONTROL_THRESHOLD)
        .map_err(|error| format!("invalid agent control policy: {error:?}"))?;
    let sdk = WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
        &package_bytes,
        policy,
    )
    .map_err(|error| format!("failed to load agent control package: {error:?}"))?;
    let (sample_margin, sample_local_accept) = agent_control_sample_decision(&sdk)?;
    if !sample_local_accept {
        return Err(format!(
            "agent control sample did not local-accept: margin={sample_margin}"
        ));
    }

    let profile = RoleBindingProfileConfig {
        profile_id: "role_binding_agent_control_seed0".to_owned(),
        profile_kind: "role_binding_nwrb".to_owned(),
        operator_classes: vec!["agent_control".to_owned(), "dialogue_state".to_owned()],
        package_path: package_path.clone(),
        runtime_bytes_estimate: sdk.bytes_estimate(),
        edge_count: package_info.edge_count,
        slot_count: 1,
        threshold: REAL_TRAFFIC_AGENT_CONTROL_THRESHOLD,
        acceptance_policy: default_profile_acceptance_policy(),
        accepted_route_keys: vec![
            "role_binding_agent_control_seed0".to_owned(),
            "agent_control_stop".to_owned(),
            "agent_control_continue".to_owned(),
            "agent_control_ack".to_owned(),
        ],
    };
    registry
        .profiles
        .retain(|existing| existing.profile_id != profile.profile_id);
    registry.profiles.push(profile);
    registry.claim_boundary = "serving registry overlay for agent-control .nwrb profile; generated from base registry plus one control-plane role-binding package; no corpus, compiler, eval pack, or local-accept claim is loaded by this command".to_owned();
    write_json_file(&registry_path, &registry)?;

    let report = RoleBindingAgentControlProfileReport {
        schema_version: "nando_role_binding_agent_control_profile_v1".to_owned(),
        verdict: "AGENT_CONTROL_PROFILE_V1_REVIEW_PROFILE_READY".to_owned(),
        base_registry_path: base_registry_path.display().to_string(),
        package_path: package_path.display().to_string(),
        registry_path: registry_path.display().to_string(),
        profile_id: "role_binding_agent_control_seed0".to_owned(),
        package_fingerprint64: package_info.fingerprint64,
        package_bytes: package_bytes.len(),
        edge_count: package_info.edge_count,
        runtime_bytes_estimate: sdk.bytes_estimate(),
        threshold: REAL_TRAFFIC_AGENT_CONTROL_THRESHOLD,
        sample_margin,
        sample_local_accept,
        raw_text_written: false,
        local_accepts_enabled_on_real_traffic: false,
        market_claim_allowed: false,
        claim_boundary: "Profile generator only. It creates a real .nwrb control-plane profile and registry overlay, validates one SDK sample, writes no raw prompt text, and enables no real-traffic local accepts. Verified CPU savings require a separate request-side payload trace, deterministic verifier, shadow pass, audit pass, provider cost, and false_accepts=0.".to_owned(),
        next_engineering_debt: "Run route-candidates and route-gap catalog with the overlay registry, then build agent-control payload dry-run and output/tool verification before any safe-policy promotion.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-agent-control-profile-v1: {}",
        report.verdict
    );
    println!("  base_registry: {}", base_registry_path.display());
    println!("  package: {}", package_path.display());
    println!("  registry: {}", registry_path.display());
    println!("  report: {}", report_path.display());
    println!("  edge_count: {}", report.edge_count);
    println!("  sample_margin: {}", report.sample_margin);
    println!("  market_claim_allowed: false");
    Ok(())
}

pub(crate) fn run_role_binding_real_traffic_agent_control_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut agent_control_candidate_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut intent_counts = BTreeMap::<String, usize>::new();
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_agent_control_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "no agent_control route candidate".to_owned();

        if let Some(candidate) = route_catalog.classify_request_text(&row.text)
            && candidate.route_key.contains("agent_control")
        {
            agent_control_candidate_events += 1;
            let intent = agent_control_intent_kind(&row.text);
            *intent_counts.entry(intent.to_owned()).or_insert(0) += 1;
            if let Some(request) =
                build_agent_control_dry_run_request(&event_id, &fingerprint, &candidate)
            {
                payload_built_events += 1;
                let active_fringe_centers = request.active_fringe.len();
                let slots = request.slots.len();
                let positive_impulses = request
                    .slots
                    .iter()
                    .map(|slot| slot.positive_impulses.len())
                    .sum::<usize>();
                let negative_impulses = request
                    .slots
                    .iter()
                    .map(|slot| slot.negative_impulses.len())
                    .sum::<usize>();
                let scoreable = active_fringe_centers > 0 && slots > 0;
                scoreable_payload_events += usize::from(scoreable);
                active_fringe_centers_total += active_fringe_centers;
                slots_total += slots;
                positive_impulses_total += positive_impulses;
                negative_impulses_total += negative_impulses;
                nando_shadow_request = Some(request);
                notes = format!(
                    "request-side agent-control payload built; intent={intent}; verified accepts disabled"
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_local_agent_control_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side agent-control payload dry-run from local Codex prompt only; raw text, response text, target labels, and proof labels not written".to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let report = RoleBindingAgentControlPayloadDryRunReport {
        schema_version: "nando_role_binding_agent_control_payload_dry_run_v1".to_owned(),
        verdict: if scoreable_payload_events > 0 {
            "AGENT_CONTROL_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT"
        } else {
            "AGENT_CONTROL_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        agent_control_candidate_events,
        payload_built_events,
        scoreable_payload_events,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        intent_counts: intent_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Request-side dry-run payload builder only. It emits active_fringe/slots for agent-control rows from prompt text only, sets verified_safe_accept=None, and therefore cannot prove savings. Shadow local accepts from this trace are unverified until a deterministic control verifier attaches output/tool evidence.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on this trace, then attach deterministic control-plane verification before any safe-policy promotion.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-agent-control-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  agent_control_candidate_events: {}",
        report.agent_control_candidate_events
    );
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  raw_text_written: false");
    Err("agent-control payload dry-run is review-only; run verifier before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_agent_control_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_agent_control_output_verification,
    )?;

    let mut enriched_rows = Vec::with_capacity(trace_rows.len());
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;

    for mut row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            enriched_rows.push(row);
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_output_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "agent-control output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_agent_control_output_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        deterministic_verification_events += usize::from(evidence.verifier_applicable);
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        verifier_not_applicable_events += usize::from(!evidence.verifier_applicable);
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "agent-control output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_agent_control_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "AGENT_CONTROL_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "AGENT_CONTROL_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: enriched_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Agent-control output evidence join only. It reads local Codex session final answers at analysis time, writes fingerprints and explicit deterministic verification results, writes no raw prompt/response text, does not enable local accepts, and cannot prove market savings by itself. Continue/execute intents are rejected unless a later tool/state verifier proves they were only control-plane transitions.".to_owned(),
        next_engineering_debt: "Run shadow analysis and verification-hook audit over the evidence-enriched trace. If continue rows become false accepts, split the control profile or change request-side admission before any safe-policy promotion.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-agent-control-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    Err("agent-control output evidence is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_agent_control_admission_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_ADMISSION_CALIBRATION_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;

    for trace in &trace_rows {
        let Some(label) = trace.verified_safe_accept else {
            continue;
        };
        if trace.nando_shadow_request.is_none() {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let request_fingerprint = trace.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            continue;
        };
        let features = extract_agent_control_admission_features(prompt_text);
        rows.push(RoleBindingAgentControlAdmissionCalibrationRow {
            trace_id: trace.trace_id.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            verifier_label: label,
            features,
        });
    }

    let minimum_true_support = 3usize;
    let policies = agent_control_admission_policy_reports(&rows, minimum_true_support);
    let robust_safe_policy_found = policies.iter().any(|policy| policy.robust_safe);
    let singleton_safe_policy_found = policies.iter().any(|policy| policy.singleton_safe);
    let best_robust_true_accepts = policies
        .iter()
        .filter(|policy| policy.robust_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let best_singleton_true_accepts = policies
        .iter()
        .filter(|policy| policy.singleton_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let feature_counts = agent_control_admission_feature_counts(&rows);
    let report = RoleBindingAgentControlAdmissionCalibrationReport {
        schema_version: "nando_role_binding_agent_control_admission_calibration_v1".to_owned(),
        verdict: if robust_safe_policy_found {
            "AGENT_CONTROL_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND"
        } else if singleton_safe_policy_found {
            "AGENT_CONTROL_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY"
        } else {
            "AGENT_CONTROL_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY"
        }
        .to_owned(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        hook_ready_rows,
        rows_with_prompt_features: rows.len(),
        history_prompt_missing_rows,
        label_true_rows,
        label_false_rows,
        minimum_true_support,
        robust_safe_policy_found,
        singleton_safe_policy_found,
        best_robust_true_accepts,
        best_singleton_true_accepts,
        feature_counts,
        policies,
        rows,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Agent-control admission calibration only. It reads real request text at analysis time, writes only fingerprints/features/counts, uses verification labels only to evaluate request-side gates, enables no local accepts, and cannot be used as market savings claim.".to_owned(),
        next_engineering_debt: if robust_safe_policy_found {
            "Promote the candidate only through a separate request-side-admitted shadow trace with provider cost, false_accepts=0, unverified_shadow_accepts=0, and an explicit route-specific claim boundary.".to_owned()
        } else {
            "Current control-plane request features do not provide a robust safe admission gate. Split stop/ack/continue or attach live agent-state/tool-state evidence before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-agent-control-admission-calibration-v1: {}",
        report.verdict
    );
    println!("  evidence_trace: {}", evidence_trace_path.display());
    println!("  history: {}", history_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!(
        "  rows_with_prompt_features: {}",
        report.rows_with_prompt_features
    );
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!(
        "  robust_safe_policy_found: {}",
        report.robust_safe_policy_found
    );
    println!(
        "  best_robust_true_accepts: {}",
        report.best_robust_true_accepts
    );
    Err("agent-control admission calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_agent_control_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_ADMISSION_CALIBRATION_REPORT));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_SAFE_POLICY_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let calibration = read_json_file::<RoleBindingAgentControlAdmissionCalibrationReport>(
        &calibration_report_path,
    )?;
    let Some(calibration_policy) = select_supported_agent_control_admission_policy(&calibration)
    else {
        return Err(
            "agent-control calibration report has no supported hard-stop safe policy candidate"
                .to_owned(),
        );
    };
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut agent_control_candidate_rows = 0usize;
    let mut request_side_policy_evaluated_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_reject_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;
    let mut scoreable_candidate_calls = 0usize;

    for row in &mut trace_rows {
        let is_agent_control = row
            .nando_shadow_request
            .as_ref()
            .and_then(|request| request.route_key.as_deref())
            .is_some_and(|route| route.contains("agent_control"));
        if !is_agent_control {
            continue;
        }
        agent_control_candidate_rows += 1;
        let scoreable = row
            .nando_shadow_request
            .as_ref()
            .is_some_and(|request| !request.active_fringe.is_empty() && !request.slots.is_empty());
        scoreable_candidate_calls += usize::from(scoreable);
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; agent_control_safe_policy_promote_v1 policy={} policy_accept=false reason=history_prompt_missing",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                calibration_policy.policy_name
            ));
            continue;
        };
        request_side_policy_evaluated_rows += 1;
        let features = extract_agent_control_admission_features(prompt_text);
        let policy_accept =
            agent_control_admission_policy_accepts(&calibration_policy.policy_name, &features)
                .unwrap_or(false);
        if policy_accept {
            policy_accept_rows += 1;
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
            row.provider_cost_microusd = Some(provider_cost_microusd);
            provider_cost_events_written += 1;
            if let Some(request) = &mut row.nando_shadow_request {
                request.expect_local_operator = Some(true);
                let score = score_role_binding_profile_request(&registry, request);
                runtime_acceptance_mismatches += usize::from(!score.accepted);
                no_score_rows +=
                    usize::from(score.fallback_reason.as_deref().is_some_and(|reason| {
                        matches!(
                            reason,
                            "profile_not_found"
                                | "input_outside_profile_contract"
                                | "no_scorable_slots"
                        )
                    }));
            }
        } else {
            policy_reject_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
        }
        row.notes = Some(format!(
            "{}; agent_control_safe_policy_promote_v1 policy={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            calibration_policy.policy_name,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingAgentControlSafePolicyPromoteReport {
        schema_version: "nando_role_binding_agent_control_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "AGENT_CONTROL_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "AGENT_CONTROL_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        selected_policy_name: calibration_policy.policy_name.clone(),
        selected_policy_true_accepts: calibration_policy.true_accepts,
        selected_policy_false_accepts: calibration_policy.false_accepts,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        agent_control_candidate_rows,
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows,
        history_prompt_missing_rows,
        policy_accept_rows,
        policy_reject_rows,
        policy_accept_verified_true_rows,
        policy_accept_verified_false_rows,
        policy_accept_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        profile_acceptance_policy_changed: false,
        broad_agent_control_profile_promoted: false,
        local_accepts_enabled_by_request_side_policy_only: true,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It rewrites the evidence-backed agent-control trace so only the selected request-side hard-stop policy keeps nando_shadow_request; rejected agent-control rows are forced to fallback by removing the shadow request. It does not change the .nwrb profile acceptance policy and does not prove market savings until shadow plus verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted trace, then route the safe-policy audit into the CPU feedback loop. The broad agent-control profile remains blocked unless split/admission proves zero false accepts.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-agent-control-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  selected_policy_name: {}", report.selected_policy_name);
    println!("  policy_accept_rows: {}", report.policy_accept_rows);
    println!(
        "  policy_accept_verified_true_rows: {}",
        report.policy_accept_verified_true_rows
    );
    println!(
        "  policy_accept_verified_false_rows: {}",
        report.policy_accept_verified_false_rows
    );
    println!(
        "  policy_accept_unverified_rows: {}",
        report.policy_accept_unverified_rows
    );
    println!(
        "  runtime_acceptance_mismatches: {}",
        report.runtime_acceptance_mismatches
    );
    Err(
        "agent-control safe-policy promotion is review-only; run shadow/audit before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_edit_payload_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_READINESS_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut rows = Vec::new();
    let mut candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut missing_scope_or_file = 0usize;
    let mut missing_marker = 0usize;
    let mut missing_length_or_shape = 0usize;
    let mut missing_edit_intent = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();
    let mut builder_kind_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let Some(candidate) = route_catalog.classify_request_text(&row.text) else {
            continue;
        };
        if !candidate.route_key.contains("edit_marker_length") {
            continue;
        }
        candidate_events += 1;
        *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
        let readiness = analyze_edit_payload_readiness(&row.text);
        payload_ready_events += usize::from(readiness.payload_ready);
        missing_scope_or_file += usize::from(!readiness.has_scope_or_file);
        missing_marker += usize::from(!readiness.has_marker);
        missing_length_or_shape += usize::from(!readiness.has_length_or_shape);
        missing_edit_intent += usize::from(!readiness.has_edit_intent);
        *builder_kind_counts
            .entry(readiness.recommended_builder_kind.clone())
            .or_insert(0) += 1;
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        rows.push(RoleBindingEditPayloadReadinessRow {
            event_id: format!(
                "codex_history_edit_readiness::{}::{}::{}",
                row.session_id, row.ts, index
            ),
            request_fingerprint: format!("fnv1a64:{fingerprint:016x}"),
            route_key: candidate.route_key,
            profile_id: candidate.profile_id,
            has_edit_intent: readiness.has_edit_intent,
            has_scope_or_file: readiness.has_scope_or_file,
            has_marker: readiness.has_marker,
            has_length_or_shape: readiness.has_length_or_shape,
            has_code_or_patch_signal: readiness.has_code_or_patch_signal,
            payload_ready: readiness.payload_ready,
            recommended_builder_kind: readiness.recommended_builder_kind,
            missing_reasons: readiness.missing_reasons,
        });
    }

    let report = RoleBindingEditPayloadReadinessReport {
        schema_version: "nando_role_binding_edit_payload_readiness_v1".to_owned(),
        verdict: if payload_ready_events > 0 {
            "EDIT_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND"
        } else {
            "EDIT_PAYLOAD_READINESS_V1_REVIEW_NO_READY_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        candidate_events,
        payload_ready_events,
        payload_ready_rate_milli: ratio_milli(payload_ready_events, candidate_events),
        missing_scope_or_file,
        missing_marker,
        missing_length_or_shape,
        missing_edit_intent,
        route_counts: route_counts
            .into_iter()
            .map(|(route_key, count)| RoleBindingNamedCount { name: route_key, count })
            .collect(),
        builder_kind_counts: builder_kind_counts
            .into_iter()
            .map(|(kind, count)| RoleBindingNamedCount { name: kind, count })
            .collect(),
        raw_text_written: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows,
        claim_boundary: "Request-side edit payload readiness only. This reads local Codex prompt text at analysis time, writes no raw text, and does not create local accepts. Payload-ready means the request has enough request-side structure to attempt a future active_fringe/slot builder; it is not verified savings.".to_owned(),
        next_engineering_debt: "Use ready rows to build edit_marker_length_payload_builder_v1 that emits active_fringe and slots from request text only, then verify shadow accepts with false_accepts=0.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-payload-readiness-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  candidate_events: {}", report.candidate_events);
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!(
        "  payload_ready_rate_milli: {}",
        report.payload_ready_rate_milli
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("edit payload readiness is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut edit_route_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut readiness_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;
    let mut builder_status_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_edit_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "no edit_marker_length route candidate".to_owned();

        if let Some(candidate) = route_catalog.classify_request_text(&row.text)
            && candidate.route_key.contains("edit_marker_length")
        {
            edit_route_candidate_events += 1;
            let readiness = analyze_edit_payload_readiness(&row.text);
            if readiness.payload_ready {
                payload_ready_events += 1;
                let built = build_edit_marker_length_dry_run_request(
                    &event_id,
                    &fingerprint,
                    &candidate,
                    &row.text,
                );
                match built {
                    Some(request) => {
                        let active_fringe_centers = request.active_fringe.len();
                        let slots = request.slots.len();
                        let positive_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.positive_impulses.len())
                            .sum::<usize>();
                        let negative_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.negative_impulses.len())
                            .sum::<usize>();
                        let scoreable = active_fringe_centers > 0 && slots > 0;
                        payload_built_events += 1;
                        scoreable_payload_events += usize::from(scoreable);
                        active_fringe_centers_total += active_fringe_centers;
                        slots_total += slots;
                        positive_impulses_total += positive_impulses;
                        negative_impulses_total += negative_impulses;
                        let builder_status = if scoreable {
                            "scoreable_payload_built"
                        } else {
                            "payload_built_but_not_scoreable"
                        }
                        .to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingEditPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                        });
                        notes = format!(
                            "request-side dry-run edit payload built; status={builder_status}; verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingEditPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "readiness_rejected".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = format!(
                    "edit route candidate rejected by readiness gate: {}",
                    readiness.missing_reasons.join(",")
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_local_edit_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side edit payload dry-run from local Codex prompt only; raw text, response text, target labels, and proof labels not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let report = RoleBindingEditPayloadDryRunReport {
        schema_version: "nando_role_binding_edit_payload_dry_run_v1".to_owned(),
        verdict: if scoreable_payload_events > 0 {
            "EDIT_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT"
        } else {
            "EDIT_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        edit_route_candidate_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side dry-run payload builder only. It emits non-empty active_fringe/slots for ready edit-route rows from prompt text only, sets verified_safe_accept=None and expect_local_operator=false, and therefore cannot prove savings. Any local accept in the following shadow run must be treated as unverified/false, not as a market claim.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 on this trace, inspect fallback/false-accept behavior, then add verification hooks before enabling any local accept.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  edit_route_candidate_events: {}",
        report.edit_route_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("edit payload dry-run is review-only; run shadow analysis before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_conditional_payload_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_PAYLOAD_READINESS_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut rows = Vec::new();
    let mut candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut missing_condition_signal = 0usize;
    let mut missing_branch_signal = 0usize;
    let mut missing_evidence_signal = 0usize;
    let mut missing_branch_tokens = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();
    let mut builder_kind_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let Some(candidate) = route_catalog.classify_request_text(&row.text) else {
            continue;
        };
        if !candidate.route_key.contains("conditional_branch") {
            continue;
        }
        candidate_events += 1;
        *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
        let readiness = analyze_conditional_payload_readiness(&row.text);
        payload_ready_events += usize::from(readiness.payload_ready);
        missing_condition_signal += usize::from(!readiness.has_condition_signal);
        missing_branch_signal += usize::from(!readiness.has_branch_signal);
        missing_evidence_signal += usize::from(!readiness.has_evidence_signal);
        missing_branch_tokens += usize::from(!readiness.has_branch_tokens);
        *builder_kind_counts
            .entry(readiness.recommended_builder_kind.clone())
            .or_insert(0) += 1;
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        rows.push(RoleBindingConditionalPayloadReadinessRow {
            event_id: format!(
                "codex_history_conditional_readiness::{}::{}::{}",
                row.session_id, row.ts, index
            ),
            request_fingerprint: format!("fnv1a64:{fingerprint:016x}"),
            route_key: candidate.route_key,
            profile_id: candidate.profile_id,
            has_condition_signal: readiness.has_condition_signal,
            has_branch_signal: readiness.has_branch_signal,
            has_evidence_signal: readiness.has_evidence_signal,
            has_branch_tokens: readiness.has_branch_tokens,
            payload_ready: readiness.payload_ready,
            recommended_builder_kind: readiness.recommended_builder_kind,
            missing_reasons: readiness.missing_reasons,
        });
    }

    let report = RoleBindingConditionalPayloadReadinessReport {
        schema_version: "nando_role_binding_conditional_payload_readiness_v1".to_owned(),
        verdict: if payload_ready_events > 0 {
            "CONDITIONAL_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND"
        } else {
            "CONDITIONAL_PAYLOAD_READINESS_V1_REVIEW_NO_READY_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        candidate_events,
        payload_ready_events,
        payload_ready_rate_milli: ratio_milli(payload_ready_events, candidate_events),
        missing_condition_signal,
        missing_branch_signal,
        missing_evidence_signal,
        missing_branch_tokens,
        route_counts: route_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        builder_kind_counts: builder_kind_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows,
        claim_boundary: "Request-side conditional payload readiness only. It reads local Codex prompt text at analysis time, writes no raw text, and does not use response, target, proof, or expected answer labels. Payload-ready means there are enough request-side branch/evidence signals to attempt a dry-run active_fringe/slot builder; it is not verified savings.".to_owned(),
        next_engineering_debt: "Use ready rows to build conditional_branch_payload_builder_v1 that emits active_fringe and slots from request text only, then run shadow and verification-hook audits before any local accept.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-conditional-payload-readiness-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  candidate_events: {}", report.candidate_events);
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!(
        "  payload_ready_rate_milli: {}",
        report.payload_ready_rate_milli
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("conditional payload readiness is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_conditional_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut conditional_route_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut readiness_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;
    let mut builder_status_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_conditional_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "no conditional_branch route candidate".to_owned();

        if let Some(candidate) = route_catalog.classify_request_text(&row.text)
            && candidate.route_key.contains("conditional_branch")
        {
            conditional_route_candidate_events += 1;
            let readiness = analyze_conditional_payload_readiness(&row.text);
            if readiness.payload_ready {
                payload_ready_events += 1;
                let built = build_conditional_branch_dry_run_request(
                    &event_id,
                    &fingerprint,
                    &candidate,
                    &row.text,
                );
                match built {
                    Some(request) => {
                        let active_fringe_centers = request.active_fringe.len();
                        let slots = request.slots.len();
                        let positive_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.positive_impulses.len())
                            .sum::<usize>();
                        let negative_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.negative_impulses.len())
                            .sum::<usize>();
                        let scoreable = active_fringe_centers > 0 && slots > 0;
                        payload_built_events += 1;
                        scoreable_payload_events += usize::from(scoreable);
                        active_fringe_centers_total += active_fringe_centers;
                        slots_total += slots;
                        positive_impulses_total += positive_impulses;
                        negative_impulses_total += negative_impulses;
                        let builder_status = if scoreable {
                            "scoreable_payload_built"
                        } else {
                            "payload_built_but_not_scoreable"
                        }
                        .to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingConditionalPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                        });
                        notes = format!(
                            "request-side dry-run conditional payload built; status={builder_status}; verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingConditionalPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "readiness_rejected".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = format!(
                    "conditional route candidate rejected by readiness gate: {}",
                    readiness.missing_reasons.join(",")
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_local_conditional_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side conditional payload dry-run from local Codex prompt only; raw text, response text, target labels, and proof labels not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let report = RoleBindingConditionalPayloadDryRunReport {
        schema_version: "nando_role_binding_conditional_payload_dry_run_v1".to_owned(),
        verdict: if scoreable_payload_events > 0 {
            "CONDITIONAL_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT"
        } else {
            "CONDITIONAL_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        conditional_route_candidate_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side dry-run conditional payload builder only. It emits active_fringe/slots for ready conditional-route rows from prompt text only, sets verified_safe_accept=None and expect_local_operator=false, and therefore cannot prove savings. Any local accept in the following shadow run is unverified and must not become a market claim.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook audit on this trace. If local accepts stay zero, either improve conditional branch extraction or attach deterministic response/tool verification before calibration.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-conditional-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  conditional_route_candidate_events: {}",
        report.conditional_route_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("conditional payload dry-run is review-only; run shadow analysis before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_mixed_payload_readiness_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_PAYLOAD_READINESS_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut rows = Vec::new();
    let mut candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut missing_action_signal = 0usize;
    let mut missing_source_signal = 0usize;
    let mut missing_destination_signal = 0usize;
    let mut missing_mapping_signal = 0usize;
    let mut missing_map_tokens = 0usize;
    let mut route_counts = BTreeMap::<String, usize>::new();
    let mut builder_kind_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let Some(candidate) = route_catalog.classify_request_text(&row.text) else {
            continue;
        };
        if !candidate.route_key.contains("mixed_map") {
            continue;
        }
        candidate_events += 1;
        *route_counts.entry(candidate.route_key.clone()).or_insert(0) += 1;
        let readiness = analyze_mixed_payload_readiness(&row.text);
        payload_ready_events += usize::from(readiness.payload_ready);
        missing_action_signal += usize::from(!readiness.has_action_signal);
        missing_source_signal += usize::from(!readiness.has_source_signal);
        missing_destination_signal += usize::from(!readiness.has_destination_signal);
        missing_mapping_signal += usize::from(!readiness.has_mapping_signal);
        missing_map_tokens += usize::from(!readiness.has_map_tokens);
        *builder_kind_counts
            .entry(readiness.recommended_builder_kind.clone())
            .or_insert(0) += 1;
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        rows.push(RoleBindingMixedPayloadReadinessRow {
            event_id: format!(
                "codex_history_mixed_readiness::{}::{}::{}",
                row.session_id, row.ts, index
            ),
            request_fingerprint: format!("fnv1a64:{fingerprint:016x}"),
            route_key: candidate.route_key,
            profile_id: candidate.profile_id,
            has_action_signal: readiness.has_action_signal,
            has_source_signal: readiness.has_source_signal,
            has_destination_signal: readiness.has_destination_signal,
            has_mapping_signal: readiness.has_mapping_signal,
            has_map_tokens: readiness.has_map_tokens,
            payload_ready: readiness.payload_ready,
            recommended_builder_kind: readiness.recommended_builder_kind,
            missing_reasons: readiness.missing_reasons,
        });
    }

    let report = RoleBindingMixedPayloadReadinessReport {
        schema_version: "nando_role_binding_mixed_payload_readiness_v1".to_owned(),
        verdict: if payload_ready_events > 0 {
            "MIXED_PAYLOAD_READINESS_V1_REVIEW_READY_CANDIDATES_FOUND"
        } else {
            "MIXED_PAYLOAD_READINESS_V1_REVIEW_NO_READY_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        candidate_events,
        payload_ready_events,
        payload_ready_rate_milli: ratio_milli(payload_ready_events, candidate_events),
        missing_action_signal,
        missing_source_signal,
        missing_destination_signal,
        missing_mapping_signal,
        missing_map_tokens,
        route_counts: route_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        builder_kind_counts: builder_kind_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows,
        claim_boundary: "Request-side mixed-map payload readiness only. It reads local Codex prompt text at analysis time, writes no raw text, and does not use response, target, proof, or expected answer labels. Payload-ready means enough request-side action/source/destination/mapping signals exist to attempt dry-run active_fringe/slot construction; it is not verified savings.".to_owned(),
        next_engineering_debt: "Use ready rows to build mixed_map_payload_builder_v1 that emits active_fringe and slots from request text only, then run shadow and verification-hook audits before any local accept.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-payload-readiness-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  report: {}", report_path.display());
    println!("  candidate_events: {}", report.candidate_events);
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!(
        "  payload_ready_rate_milli: {}",
        report.payload_ready_rate_milli
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("mixed payload readiness is review-only; it is not verified savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_mixed_payload_dry_run_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_PAYLOAD_DRY_RUN_REPORT));
    let max_events = args
        .next()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid max_events '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(1000);

    let registry_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&registry_config_path)?;
    validate_registry_config(&registry_config)?;
    let route_catalog = CodexHistoryRouteCatalog::from_registry(&registry_config)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let skip = history_rows.len().saturating_sub(max_events);
    let mut trace_rows = Vec::with_capacity(history_rows.len().saturating_sub(skip));
    let mut report_rows = Vec::new();
    let mut mixed_route_candidate_events = 0usize;
    let mut payload_ready_events = 0usize;
    let mut payload_built_events = 0usize;
    let mut scoreable_payload_events = 0usize;
    let mut builder_rejected_events = 0usize;
    let mut readiness_rejected_events = 0usize;
    let mut active_fringe_centers_total = 0usize;
    let mut slots_total = 0usize;
    let mut positive_impulses_total = 0usize;
    let mut negative_impulses_total = 0usize;
    let mut builder_status_counts = BTreeMap::<String, usize>::new();

    for (index, row) in history_rows.iter().enumerate().skip(skip) {
        let fingerprint = stable_real_traffic_fingerprint64(row.text.as_bytes());
        let event_id = format!(
            "codex_history_mixed_payload_dry_run::{}::{}::{}",
            row.session_id, row.ts, index
        );
        let request_fingerprint = format!("fnv1a64:{fingerprint:016x}");
        let exact_cache_key = Some(format!("codex_history_request:{fingerprint:016x}"));
        let mut nando_shadow_request = None;
        let mut notes = "no mixed_map route candidate".to_owned();

        if let Some(candidate) = route_catalog.classify_request_text(&row.text)
            && candidate.route_key.contains("mixed_map")
        {
            mixed_route_candidate_events += 1;
            let readiness = analyze_mixed_payload_readiness(&row.text);
            if readiness.payload_ready {
                payload_ready_events += 1;
                let built =
                    build_mixed_map_dry_run_request(&event_id, &fingerprint, &candidate, &row.text);
                match built {
                    Some(request) => {
                        let active_fringe_centers = request.active_fringe.len();
                        let slots = request.slots.len();
                        let positive_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.positive_impulses.len())
                            .sum::<usize>();
                        let negative_impulses = request
                            .slots
                            .iter()
                            .map(|slot| slot.negative_impulses.len())
                            .sum::<usize>();
                        let scoreable = active_fringe_centers > 0 && slots > 0;
                        payload_built_events += 1;
                        scoreable_payload_events += usize::from(scoreable);
                        active_fringe_centers_total += active_fringe_centers;
                        slots_total += slots;
                        positive_impulses_total += positive_impulses;
                        negative_impulses_total += negative_impulses;
                        let builder_status = if scoreable {
                            "scoreable_payload_built"
                        } else {
                            "payload_built_but_not_scoreable"
                        }
                        .to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingMixedPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: true,
                            scoreable,
                            builder_status: builder_status.clone(),
                            active_fringe_centers,
                            slots,
                            positive_impulses,
                            negative_impulses,
                        });
                        notes = format!(
                            "request-side dry-run mixed payload built; status={builder_status}; verified accepts disabled"
                        );
                        nando_shadow_request = Some(request);
                    }
                    None => {
                        builder_rejected_events += 1;
                        let builder_status = "builder_rejected_request_side_features".to_owned();
                        *builder_status_counts
                            .entry(builder_status.clone())
                            .or_insert(0) += 1;
                        report_rows.push(RoleBindingMixedPayloadDryRunRow {
                            event_id: event_id.clone(),
                            request_fingerprint: request_fingerprint.clone(),
                            route_key: candidate.route_key.clone(),
                            profile_id: candidate.profile_id.clone(),
                            readiness_payload_ready: true,
                            payload_built: false,
                            scoreable: false,
                            builder_status: builder_status.clone(),
                            active_fringe_centers: 0,
                            slots: 0,
                            positive_impulses: 0,
                            negative_impulses: 0,
                        });
                        notes = builder_status;
                    }
                }
            } else {
                readiness_rejected_events += 1;
                let builder_status = "readiness_rejected".to_owned();
                *builder_status_counts
                    .entry(builder_status.clone())
                    .or_insert(0) += 1;
                notes = format!(
                    "mixed route candidate rejected by readiness gate: {}",
                    readiness.missing_reasons.join(",")
                );
            }
        }

        trace_rows.push(RoleBindingRealTrafficTraceRow {
            schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
            trace_id: event_id,
            traffic_source: Some("codex_history_local_mixed_payload_dry_run".to_owned()),
            time_ms: Some(row.ts.saturating_mul(1000)),
            request_fingerprint: Some(request_fingerprint),
            response_fingerprint: None,
            tool_call_fingerprints: Vec::new(),
            verification_source: Some(
                "request-side mixed payload dry-run from local Codex prompt only; raw text, response text, target labels, and proof labels not written"
                    .to_owned(),
            ),
            llm_call: true,
            exact_cache_key,
            provider_cache_hit: None,
            provider_cost_microusd: None,
            nando_shadow_request,
            verified_safe_accept: None,
            synthetic_source: Some(false),
            notes: Some(notes),
        });
    }

    write_real_traffic_trace_jsonl(&trace_path, &trace_rows)?;
    let report = RoleBindingMixedPayloadDryRunReport {
        schema_version: "nando_role_binding_mixed_payload_dry_run_v1".to_owned(),
        verdict: if scoreable_payload_events > 0 {
            "MIXED_PAYLOAD_DRY_RUN_V1_REVIEW_SCOREABLE_PAYLOADS_BUILT"
        } else {
            "MIXED_PAYLOAD_DRY_RUN_V1_REVIEW_NO_SCOREABLE_PAYLOADS"
        }
        .to_owned(),
        history_path: history_path.display().to_string(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        max_events,
        total_history_rows: history_rows.len(),
        trace_rows_written: trace_rows.len(),
        mixed_route_candidate_events,
        payload_ready_events,
        payload_built_events,
        scoreable_payload_events,
        builder_rejected_events,
        readiness_rejected_events,
        active_fringe_centers_total,
        slots_total,
        positive_impulses_total,
        negative_impulses_total,
        builder_status_counts: builder_status_counts
            .into_iter()
            .map(|(name, count)| RoleBindingNamedCount { name, count })
            .collect(),
        raw_text_written: false,
        response_text_used: false,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        rows: report_rows,
        claim_boundary: "Request-side dry-run mixed payload builder only. It emits active_fringe/slots for ready mixed-route rows from prompt text only, sets verified_safe_accept=None and expect_local_operator=false, and therefore cannot prove savings. Any local accept in the following shadow run is unverified and must not become a market claim.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook audit on this trace. If local accepts stay zero, attach deterministic response/tool verification before calibration.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-payload-dry-run-v1: {}",
        report.verdict
    );
    println!("  history: {}", history_path.display());
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  mixed_route_candidate_events: {}",
        report.mixed_route_candidate_events
    );
    println!("  payload_ready_events: {}", report.payload_ready_events);
    println!("  payload_built_events: {}", report.payload_built_events);
    println!(
        "  scoreable_payload_events: {}",
        report.scoreable_payload_events
    );
    println!("  raw_text_written: {}", report.raw_text_written);
    Err("mixed payload dry-run is review-only; run shadow analysis before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_edit_output_verification,
    )?;

    let mut enriched_rows = Vec::with_capacity(trace_rows.len());
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;

    for mut row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            enriched_rows.push(row);
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_output_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "edit output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_edit_output_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        deterministic_verification_events += 1;
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        if !evidence.verifier_applicable {
            verifier_not_applicable_events += 1;
        }
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "edit output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_edit_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "EDIT_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "EDIT_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: enriched_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Edit output evidence join only. It reads local Codex session final answers at analysis time, writes fingerprints and explicit deterministic verification results, writes no raw prompt/response text, does not enable local accepts, and cannot prove market savings by itself.".to_owned(),
        next_engineering_debt: "Run shadow analysis and verification-hook audit over the evidence-enriched trace; only hook-backed true verifications with local accepts and provider cost can count as verified CPU savings.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    Err("edit output evidence is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_conditional_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| row.request_fingerprint.as_deref())
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_conditional_output_verification,
    )?;

    let mut enriched_rows = Vec::with_capacity(trace_rows.len());
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;

    for mut row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            enriched_rows.push(row);
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(&request_fingerprint)
        else {
            no_session_output_match_events += 1;
            row.notes = Some(append_trace_note(
                row.notes.as_deref(),
                "conditional output evidence missing: no matching Codex final answer found",
            ));
            enriched_rows.push(row);
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(
            "codex_session_final_answer_fingerprint_plus_deterministic_conditional_output_verifier_v1"
                .to_owned(),
        );
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        deterministic_verification_events += 1;
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
        if !evidence.verifier_applicable {
            verifier_not_applicable_events += 1;
        }
        row.notes = Some(append_trace_note(
            row.notes.as_deref(),
            &format!(
                "conditional output evidence attached; verifier_status={}",
                evidence.verifier_status
            ),
        ));
        enriched_rows.push(row);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &enriched_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_conditional_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "CONDITIONAL_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "CONDITIONAL_OUTPUT_EVIDENCE_V1_REVIEW_NO_OUTPUT_EVIDENCE"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: enriched_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Conditional output evidence join only. It reads local Codex session final answers at analysis time, writes fingerprints and explicit deterministic verification results, writes no raw prompt/response text, does not enable local accepts, and cannot prove market savings by itself.".to_owned(),
        next_engineering_debt: "Run shadow analysis and verification-hook audit over the conditional evidence trace. Only hook-backed true verifications with local accepts, provider cost, non-synthetic traces, and false_accepts=0 can count as verified CPU savings.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-conditional-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    Err("conditional output evidence is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_mixed_output_evidence_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let input_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let sessions_root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/sessions"));
    let output_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_REPORT));

    let mut trace_rows = read_real_traffic_trace_jsonl(&input_trace_path)?;
    let wanted_request_fingerprints = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| row.request_fingerprint.clone())
        .collect::<HashSet<_>>();
    let session_ids = trace_rows
        .iter()
        .filter(|row| row.nando_shadow_request.is_some())
        .filter_map(|row| codex_history_session_id_from_trace_id(&row.trace_id))
        .collect::<HashSet<_>>();
    let session_index = build_codex_session_output_evidence_index(
        &sessions_root,
        &session_ids,
        &wanted_request_fingerprints,
        deterministic_mixed_output_verification,
    )?;

    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut output_evidence_matched_events = 0usize;
    let mut no_session_output_match_events = 0usize;
    let mut verifier_not_applicable_events = 0usize;
    let mut deterministic_verification_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;

    for row in &mut trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        operator_candidate_calls += 1;
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        let Some(request_fingerprint) = &row.request_fingerprint else {
            no_session_output_match_events += 1;
            continue;
        };
        let Some(evidence) = session_index
            .by_request_fingerprint
            .get(request_fingerprint)
        else {
            no_session_output_match_events += 1;
            continue;
        };
        output_evidence_matched_events += 1;
        row.response_fingerprint = Some(evidence.response_fingerprint.clone());
        row.verification_source = Some(format!(
            "codex_session_final_answer_mixed_verifier:{}",
            evidence.verifier_status
        ));
        deterministic_verification_events += usize::from(evidence.verifier_applicable);
        verifier_not_applicable_events += usize::from(!evidence.verifier_applicable);
        row.verified_safe_accept = Some(evidence.verified_safe_accept);
        verified_true_events += usize::from(evidence.verified_safe_accept);
        verified_false_events += usize::from(!evidence.verified_safe_accept);
    }

    write_real_traffic_trace_jsonl(&output_trace_path, &trace_rows)?;
    let report = RoleBindingEditOutputEvidenceReport {
        schema_version: "nando_role_binding_mixed_output_evidence_v1".to_owned(),
        verdict: if output_evidence_matched_events > 0 {
            "MIXED_OUTPUT_EVIDENCE_V1_REVIEW_EVIDENCE_ATTACHED"
        } else {
            "MIXED_OUTPUT_EVIDENCE_V1_REVIEW_NO_EVIDENCE_MATCH"
        }
        .to_owned(),
        input_trace_path: input_trace_path.display().to_string(),
        sessions_root: sessions_root.display().to_string(),
        output_trace_path: output_trace_path.display().to_string(),
        total_trace_rows: trace_rows.len(),
        operator_candidate_calls,
        scoreable_candidate_calls,
        session_ids_requested: session_ids.len(),
        session_files_scanned: session_index.session_files_scanned,
        codex_turns_indexed: session_index.codex_turns_indexed,
        output_evidence_matched_events,
        no_session_output_match_events,
        deterministic_verification_events,
        verifier_not_applicable_events,
        verified_true_events,
        verified_false_events,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_verification: true,
        target_labels_used: false,
        proof_labels_used: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Mixed output evidence join only. It reads local Codex session final answers at analysis time, writes fingerprints and deterministic verification results, writes no raw prompt/response text, does not enable local accepts, and cannot prove market savings by itself.".to_owned(),
        next_engineering_debt: "Run shadow analysis and verification-hook audit over the mixed evidence trace. Only hook-backed true verifications with local accepts, provider cost, non-synthetic traces, and false_accepts=0 can count as verified CPU savings.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-output-evidence-v1: {}",
        report.verdict
    );
    println!("  input_trace: {}", input_trace_path.display());
    println!("  sessions_root: {}", sessions_root.display());
    println!("  output_trace: {}", output_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  output_evidence_matched_events: {}",
        report.output_evidence_matched_events
    );
    println!("  verified_true_events: {}", report.verified_true_events);
    println!("  verified_false_events: {}", report.verified_false_events);
    println!("  raw_response_text_written: false");
    Err("mixed output evidence is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_local_accept_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_LOCAL_ACCEPT_CALIBRATION_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut scored_rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut no_score_rows = 0usize;

    for row in &trace_rows {
        let Some(label) = row.verified_safe_accept else {
            continue;
        };
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let Some(score) = score_role_binding_profile_request_detailed(&registry, request) else {
            no_score_rows += 1;
            continue;
        };
        let current_response = score_role_binding_profile_request(&registry, request);
        let marker_slot_margin = score.slot_margins.first().copied().unwrap_or(0);
        let end_slot_margin = score.slot_margins.get(1).copied().unwrap_or(0);
        scored_rows.push(RoleBindingEditLocalAcceptCalibrationRow {
            trace_id: row.trace_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            response_fingerprint: row.response_fingerprint.clone(),
            verifier_label: label,
            production_accepted: current_response.accepted,
            production_fallback_reason: current_response.fallback_reason,
            energy_margin: score.energy_margin,
            min_slot_margin: score.min_slot_margin,
            marker_slot_margin,
            end_slot_margin,
            slot_count: score.slot_margins.len(),
        });
    }

    let current_policy =
        evaluate_edit_calibration_policy("current_strict_all_slots", &scored_rows, |row| {
            row.production_accepted
        });
    let energy_only_policy =
        evaluate_edit_calibration_policy("energy_only_no_slot_order", &scored_rows, |row| {
            row.energy_margin >= 1
        });
    let marker_slot_policy =
        evaluate_edit_calibration_policy("marker_slot_only_ignore_end_slot", &scored_rows, |row| {
            row.marker_slot_margin > 0 && row.energy_margin >= 1
        });
    let strict_without_zero_end_policy = evaluate_edit_calibration_policy(
        "strict_slots_but_ignore_zero_end_slot",
        &scored_rows,
        |row| row.marker_slot_margin > 0 && row.end_slot_margin >= 0 && row.energy_margin >= 1,
    );
    let best_marker_threshold_policy =
        best_single_threshold_policy("best_marker_slot_margin_threshold", &scored_rows, |row| {
            row.marker_slot_margin
        });
    let best_energy_threshold_policy =
        best_single_threshold_policy("best_energy_margin_threshold", &scored_rows, |row| {
            row.energy_margin
        });
    let policies = vec![
        current_policy,
        energy_only_policy,
        marker_slot_policy,
        strict_without_zero_end_policy,
        best_marker_threshold_policy,
        best_energy_threshold_policy,
    ];
    let safe_policy_found = policies
        .iter()
        .any(|policy| policy.false_accepts == 0 && policy.true_accepts > 0);
    let best_safe_true_accepts = policies
        .iter()
        .filter(|policy| policy.false_accepts == 0)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let report = RoleBindingEditLocalAcceptCalibrationReport {
        schema_version: "nando_role_binding_edit_local_accept_calibration_v1".to_owned(),
        verdict: if safe_policy_found {
            "EDIT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND"
        } else {
            "EDIT_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        hook_ready_rows,
        scored_rows: scored_rows.len(),
        label_true_rows,
        label_false_rows,
        no_score_rows,
        safe_policy_found,
        best_safe_true_accepts,
        policies,
        rows: scored_rows,
        margin_collision_diagnostics: Vec::new(),
        request_side_margin_only_accepts_all_true_without_false: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Calibration only. It evaluates readout/admission policies against evidence-backed real Codex labels, writes fingerprints and margins only, enables no local accepts, and cannot be used as a market savings claim.".to_owned(),
        next_engineering_debt: if safe_policy_found {
            "Promote the safe policy only behind a separate non-synthetic shadow gate with false_accepts=0, provider cost, and rollback.".to_owned()
        } else {
            "Do not relax score/readout thresholds. The current edit payload geometry does not separate verifier-true from verifier-false rows; build a request-side admission gate or improve payload features before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-local-accept-calibration-v1: {}",
        report.verdict
    );
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!("  safe_policy_found: {}", report.safe_policy_found);
    println!(
        "  best_safe_true_accepts: {}",
        report.best_safe_true_accepts
    );
    Err("edit local accept calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_conditional_local_accept_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut scored_rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut no_score_rows = 0usize;

    for row in &trace_rows {
        let Some(label) = row.verified_safe_accept else {
            continue;
        };
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let Some(score) = score_role_binding_profile_request_detailed(&registry, request) else {
            no_score_rows += 1;
            continue;
        };
        let current_response = score_role_binding_profile_request(&registry, request);
        let branch_slot_margin = score.slot_margins.first().copied().unwrap_or(0);
        let evidence_slot_margin = score.slot_margins.get(1).copied().unwrap_or(0);
        scored_rows.push(RoleBindingEditLocalAcceptCalibrationRow {
            trace_id: row.trace_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            response_fingerprint: row.response_fingerprint.clone(),
            verifier_label: label,
            production_accepted: current_response.accepted,
            production_fallback_reason: current_response.fallback_reason,
            energy_margin: score.energy_margin,
            min_slot_margin: score.min_slot_margin,
            marker_slot_margin: branch_slot_margin,
            end_slot_margin: evidence_slot_margin,
            slot_count: score.slot_margins.len(),
        });
    }

    let current_policy =
        evaluate_edit_calibration_policy("current_strict_all_slots", &scored_rows, |row| {
            row.production_accepted
        });
    let energy_only_policy =
        evaluate_edit_calibration_policy("energy_only_no_slot_order", &scored_rows, |row| {
            row.energy_margin >= 1
        });
    let branch_slot_policy = evaluate_edit_calibration_policy(
        "branch_slot_only_ignore_evidence_slot",
        &scored_rows,
        |row| row.marker_slot_margin > 0 && row.energy_margin >= 1,
    );
    let strict_without_zero_evidence_policy = evaluate_edit_calibration_policy(
        "strict_slots_but_ignore_zero_evidence_slot",
        &scored_rows,
        |row| row.marker_slot_margin > 0 && row.end_slot_margin >= 0 && row.energy_margin >= 1,
    );
    let best_branch_threshold_policy =
        best_single_threshold_policy("best_branch_slot_margin_threshold", &scored_rows, |row| {
            row.marker_slot_margin
        });
    let best_energy_threshold_policy =
        best_single_threshold_policy("best_energy_margin_threshold", &scored_rows, |row| {
            row.energy_margin
        });
    let policies = vec![
        current_policy,
        energy_only_policy,
        branch_slot_policy,
        strict_without_zero_evidence_policy,
        best_branch_threshold_policy,
        best_energy_threshold_policy,
    ];
    let safe_policy_found = policies
        .iter()
        .any(|policy| policy.false_accepts == 0 && policy.true_accepts > 0);
    let best_safe_true_accepts = policies
        .iter()
        .filter(|policy| policy.false_accepts == 0)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let report = RoleBindingEditLocalAcceptCalibrationReport {
        schema_version: "nando_role_binding_conditional_local_accept_calibration_v1".to_owned(),
        verdict: if safe_policy_found {
            "CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND"
        } else {
            "CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        hook_ready_rows,
        scored_rows: scored_rows.len(),
        label_true_rows,
        label_false_rows,
        no_score_rows,
        safe_policy_found,
        best_safe_true_accepts,
        policies,
        rows: scored_rows,
        margin_collision_diagnostics: Vec::new(),
        request_side_margin_only_accepts_all_true_without_false: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Conditional calibration only. It evaluates readout policies against evidence-backed real Codex conditional labels, writes fingerprints and margins only, enables no local accepts, and cannot be used as a market savings claim.".to_owned(),
        next_engineering_debt: if safe_policy_found {
            "Promote the safe policy only behind a separate non-synthetic shadow trace rewrite with false_accepts=0, provider cost, rollback, and explicit admission rules.".to_owned()
        } else {
            "Do not relax score/readout thresholds. The current conditional payload geometry does not separate verifier-true from verifier-false rows; improve branch extraction/admission before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-conditional-local-accept-calibration-v1: {}",
        report.verdict
    );
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!("  safe_policy_found: {}", report.safe_policy_found);
    println!(
        "  best_safe_true_accepts: {}",
        report.best_safe_true_accepts
    );
    Err("conditional local accept calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_conditional_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONDITIONAL_SAFE_POLICY_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let calibration =
        read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(&calibration_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let request_side_policy_name = "conditional_gate_terms_prompt_len_ge_300";
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_conditional_safe_policy_from_evidence(
        &base_registry,
        &trace_rows,
        &history_by_fingerprint,
        request_side_policy_name,
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();

    let conditional_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("conditional_branch"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if conditional_profile_ids.is_empty() {
        return Err(
            "conditional safe policy promotion found no conditional profile ids in trace"
                .to_owned(),
        );
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if conditional_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "conditional safe policy promotion found no matching profiles in registry for {:?}",
            conditional_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut conditional_candidate_rows = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut request_side_policy_evaluated_rows = 0usize;
    let mut request_side_policy_accept_rows = 0usize;
    let mut request_side_policy_reject_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;
    let mut runtime_policy_accept_rows = 0usize;
    let mut runtime_policy_verified_true_rows = 0usize;
    let mut runtime_policy_verified_false_rows = 0usize;
    let mut runtime_policy_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let is_conditional = row
            .nando_shadow_request
            .as_ref()
            .and_then(|request| request.route_key.as_deref())
            .is_some_and(|route| route.contains("conditional_branch"));
        if !is_conditional {
            continue;
        }
        conditional_candidate_rows += 1;
        let scoreable = row
            .nando_shadow_request
            .as_ref()
            .is_some_and(|request| !request.active_fringe.is_empty() && !request.slots.is_empty());
        scoreable_candidate_calls += usize::from(scoreable);
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; conditional_safe_policy_promote_v1 request_policy={} policy_accept=false reason=history_prompt_missing",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                request_side_policy_name
            ));
            continue;
        };
        request_side_policy_evaluated_rows += 1;
        let request_policy_accept =
            conditional_safe_policy_accepts(request_side_policy_name, prompt_text).unwrap_or(false);
        if !request_policy_accept {
            request_side_policy_reject_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; conditional_safe_policy_promote_v1 request_policy={} provider_cost_estimate_microusd={} policy_accept=false",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                request_side_policy_name,
                provider_cost_microusd
            ));
            continue;
        }

        request_side_policy_accept_rows += 1;
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;
        let Some(request) = &mut row.nando_shadow_request else {
            no_score_rows += 1;
            continue;
        };
        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let runtime_policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            threshold,
        );
        request.expect_local_operator = Some(runtime_policy_accept);
        if runtime_policy_accept {
            runtime_policy_accept_rows += 1;
            runtime_policy_verified_true_rows +=
                usize::from(row.verified_safe_accept == Some(true));
            runtime_policy_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            runtime_policy_unverified_rows += usize::from(row.verified_safe_accept.is_none());
        }
        let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
        runtime_acceptance_mismatches +=
            usize::from(runtime_response.accepted != runtime_policy_accept);
        row.notes = Some(format!(
            "{}; conditional_safe_policy_promote_v1 request_policy={} runtime_policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            request_side_policy_name,
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            runtime_policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingConditionalSafePolicyPromoteReport {
        schema_version: "nando_role_binding_conditional_safe_policy_promote_v1".to_owned(),
        verdict: if runtime_policy_accept_rows > 0
            && runtime_policy_verified_false_rows == 0
            && runtime_policy_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "CONDITIONAL_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "CONDITIONAL_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        calibration_verdict: calibration.verdict,
        request_side_policy_name: request_side_policy_name.to_owned(),
        selected_policy_name: policy.policy_name,
        selected_policy_source: policy.selection_source,
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy.accepts,
        selected_policy_true_accepts: policy.true_accepts,
        selected_policy_false_accepts: policy.false_accepts,
        selected_policy_unverified_accepts: policy.unverified_accepts,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        conditional_candidate_rows,
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows,
        request_side_policy_accept_rows,
        request_side_policy_reject_rows,
        history_prompt_missing_rows,
        runtime_policy_accept_rows,
        runtime_policy_verified_true_rows,
        runtime_policy_verified_false_rows,
        runtime_policy_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled_by_request_side_policy_and_score: true,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It creates a promoted conditional serving registry and rewrites the evidence-backed trace so only request-side gate-like conditional rows keep nando_shadow_request. Offline labels/evidence choose the positive threshold, but serving uses only request text admission plus energy score >= threshold. It does not prove market savings until shadow plus verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted conditional registry/trace, then feed that safe-policy audit into CPU route feedback. The broad conditional route remains blocked by calibration false accepts.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-conditional-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!(
        "  request_side_policy_name: {}",
        report.request_side_policy_name
    );
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!(
        "  request_side_policy_accept_rows: {}",
        report.request_side_policy_accept_rows
    );
    println!(
        "  runtime_policy_accept_rows: {}",
        report.runtime_policy_accept_rows
    );
    println!(
        "  runtime_policy_verified_true_rows: {}",
        report.runtime_policy_verified_true_rows
    );
    println!(
        "  runtime_policy_verified_false_rows: {}",
        report.runtime_policy_verified_false_rows
    );
    println!(
        "  runtime_policy_unverified_rows: {}",
        report.runtime_policy_unverified_rows
    );
    Err(
        "conditional safe-policy promotion is review-only; run shadow/audit before claims"
            .to_owned(),
    )
}

pub(crate) fn run_role_binding_real_traffic_mixed_local_accept_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_LOCAL_ACCEPT_CALIBRATION_REPORT));

    let registry = RoleBindingProfileRuntimeRegistry::from_config_path(&registry_config_path)?;
    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let mut scored_rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut no_score_rows = 0usize;

    for row in &trace_rows {
        let Some(label) = row.verified_safe_accept else {
            continue;
        };
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let Some(score) = score_role_binding_profile_request_detailed(&registry, request) else {
            no_score_rows += 1;
            continue;
        };
        let current_response = score_role_binding_profile_request(&registry, request);
        let destination_slot_margin = score.slot_margins.first().copied().unwrap_or(0);
        let action_slot_margin = score.slot_margins.get(1).copied().unwrap_or(0);
        scored_rows.push(RoleBindingEditLocalAcceptCalibrationRow {
            trace_id: row.trace_id.clone(),
            request_fingerprint: row.request_fingerprint.clone(),
            response_fingerprint: row.response_fingerprint.clone(),
            verifier_label: label,
            production_accepted: current_response.accepted,
            production_fallback_reason: current_response.fallback_reason,
            energy_margin: score.energy_margin,
            min_slot_margin: score.min_slot_margin,
            marker_slot_margin: destination_slot_margin,
            end_slot_margin: action_slot_margin,
            slot_count: score.slot_margins.len(),
        });
    }

    let current_policy =
        evaluate_edit_calibration_policy("current_strict_all_slots", &scored_rows, |row| {
            row.production_accepted
        });
    let energy_only_policy =
        evaluate_edit_calibration_policy("energy_only_no_slot_order", &scored_rows, |row| {
            row.energy_margin >= 1
        });
    let destination_slot_policy = evaluate_edit_calibration_policy(
        "destination_slot_only_ignore_action_slot",
        &scored_rows,
        |row| row.marker_slot_margin > 0 && row.energy_margin >= 1,
    );
    let action_slot_policy = evaluate_edit_calibration_policy(
        "action_slot_only_ignore_destination_slot",
        &scored_rows,
        |row| row.end_slot_margin > 0 && row.energy_margin >= 1,
    );
    let best_destination_threshold_policy = best_single_threshold_policy(
        "best_destination_slot_margin_threshold",
        &scored_rows,
        |row| row.marker_slot_margin,
    );
    let best_energy_threshold_policy =
        best_single_threshold_policy("best_energy_margin_threshold", &scored_rows, |row| {
            row.energy_margin
        });
    let policies = vec![
        current_policy,
        energy_only_policy,
        destination_slot_policy,
        action_slot_policy,
        best_destination_threshold_policy,
        best_energy_threshold_policy,
    ];
    let safe_policy_found = policies
        .iter()
        .any(|policy| policy.false_accepts == 0 && policy.true_accepts > 0);
    let best_safe_true_accepts = policies
        .iter()
        .filter(|policy| policy.false_accepts == 0)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let report = RoleBindingEditLocalAcceptCalibrationReport {
        schema_version: "nando_role_binding_mixed_local_accept_calibration_v1".to_owned(),
        verdict: if safe_policy_found {
            "MIXED_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_SAFE_POLICY_CANDIDATE_FOUND"
        } else {
            "MIXED_LOCAL_ACCEPT_CALIBRATION_V1_REVIEW_NO_SAFE_READOUT_POLICY"
        }
        .to_owned(),
        registry_config_path: registry_config_path.display().to_string(),
        trace_path: trace_path.display().to_string(),
        hook_ready_rows,
        scored_rows: scored_rows.len(),
        label_true_rows,
        label_false_rows,
        no_score_rows,
        safe_policy_found,
        best_safe_true_accepts,
        policies,
        rows: scored_rows,
        margin_collision_diagnostics: Vec::new(),
        request_side_margin_only_accepts_all_true_without_false: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Mixed calibration only. It evaluates readout policies against evidence-backed real Codex mixed labels, writes fingerprints and margins only, enables no local accepts, and cannot be used as a market savings claim.".to_owned(),
        next_engineering_debt: if safe_policy_found {
            "Promote the safe policy only behind a separate non-synthetic shadow trace rewrite with false_accepts=0, provider cost, rollback, and explicit admission rules.".to_owned()
        } else {
            "Do not relax score/readout thresholds. The current mixed payload geometry does not separate verifier-true from verifier-false rows; improve map extraction/admission before enabling local accepts.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-local-accept-calibration-v1: {}",
        report.verdict
    );
    println!("  registry_config: {}", registry_config_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!("  safe_policy_found: {}", report.safe_policy_found);
    println!(
        "  best_safe_true_accepts: {}",
        report.best_safe_true_accepts
    );
    Err("mixed local accept calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_mixed_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let calibration =
        read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(&calibration_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let Some(calibration_policy) = select_supported_mixed_safe_policy(&calibration) else {
        return Err(
            "mixed calibration report has no supported safe policy candidate for runtime promotion"
                .to_owned(),
        );
    };
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_mixed_promotion_policy_from_evidence(
        &base_registry,
        &trace_rows,
        calibration_policy,
        "mixed_map",
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();
    let mixed_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("mixed_map"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if mixed_profile_ids.is_empty() {
        return Err("mixed safe policy promotion found no mixed profile ids in trace".to_owned());
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if mixed_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "mixed safe policy promotion found no matching profiles in registry for {:?}",
            mixed_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut scoreable_candidate_calls = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let Some(request) = &mut row.nando_shadow_request else {
            continue;
        };
        let is_mixed = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains("mixed_map"));
        if !is_mixed {
            continue;
        }
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;

        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            threshold,
        );
        request.expect_local_operator = Some(policy_accept);
        if policy_accept {
            policy_accept_rows += 1;
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
        }
        let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
        runtime_acceptance_mismatches += usize::from(runtime_response.accepted != policy_accept);
        row.notes = Some(format!(
            "{}; mixed_safe_policy_promote_v1 policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_mixed_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "MIXED_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "MIXED_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: None,
        request_side_policy_name: None,
        calibration_policy_name: calibration_policy.policy_name.clone(),
        calibration_policy_threshold: calibration_policy.threshold,
        selected_policy_name: policy.policy_name.clone(),
        selected_policy_source: policy.selection_source.clone(),
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy.accepts,
        selected_policy_true_accepts: policy.true_accepts,
        selected_policy_false_accepts: policy.false_accepts,
        selected_policy_unverified_accepts: policy.unverified_accepts,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows: 0,
        request_side_policy_accept_rows: 0,
        request_side_policy_reject_rows: 0,
        history_prompt_missing_rows: 0,
        policy_accept_rows,
        policy_accept_verified_true_rows,
        policy_accept_verified_false_rows,
        policy_accept_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It creates a promoted serving registry with an explicit mixed-map acceptance policy and rewrites a shadow trace with provider-cost estimates. Offline labels/evidence may choose the threshold, but serving uses only request-side score >= threshold. It does not prove market savings until role-binding-real-traffic-shadow-v1 and verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted registry/trace. Only a shadow PASS with provider cost, non-synthetic rows, and false_accepts=0 can advance verified CPU routability.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  selected_policy_name: {}", report.selected_policy_name);
    println!(
        "  selected_policy_source: {}",
        report.selected_policy_source
    );
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!("  policy_accept_rows: {}", report.policy_accept_rows);
    println!(
        "  policy_accept_verified_true_rows: {}",
        report.policy_accept_verified_true_rows
    );
    println!(
        "  policy_accept_verified_false_rows: {}",
        report.policy_accept_verified_false_rows
    );
    println!(
        "  policy_accept_unverified_rows: {}",
        report.policy_accept_unverified_rows
    );
    Err("mixed safe policy promotion is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_mixed_safe_policy_promote_v2<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_V2_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_V2_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_V2_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let calibration =
        read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(&calibration_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let Some(calibration_policy) = select_supported_mixed_safe_policy(&calibration) else {
        return Err(
            "mixed calibration report has no supported safe policy candidate for runtime promotion"
                .to_owned(),
        );
    };
    let request_side_policy_name = "mixed_no_goal_control_prompt";
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_mixed_promotion_policy_from_request_side_evidence(
        &base_registry,
        &trace_rows,
        &history_by_fingerprint,
        calibration_policy,
        "mixed_map",
        request_side_policy_name,
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();
    let mixed_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("mixed_map"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if mixed_profile_ids.is_empty() {
        return Err("mixed safe policy promotion found no mixed profile ids in trace".to_owned());
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if mixed_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "mixed safe policy promotion found no matching profiles in registry for {:?}",
            mixed_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut scoreable_candidate_calls = 0usize;
    let mut request_side_policy_evaluated_rows = 0usize;
    let mut request_side_policy_accept_rows = 0usize;
    let mut request_side_policy_reject_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let is_mixed = row
            .nando_shadow_request
            .as_ref()
            .and_then(|request| request.route_key.as_deref())
            .is_some_and(|route| route.contains("mixed_map"));
        if !is_mixed {
            continue;
        }
        let scoreable = row
            .nando_shadow_request
            .as_ref()
            .is_some_and(|request| !request.active_fringe.is_empty() && !request.slots.is_empty());
        scoreable_candidate_calls += usize::from(scoreable);
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; mixed_safe_policy_promote_v2 request_policy={} policy_accept=false reason=history_prompt_missing",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                request_side_policy_name
            ));
            continue;
        };
        request_side_policy_evaluated_rows += 1;
        let request_policy_accept =
            mixed_safe_policy_accepts(request_side_policy_name, prompt_text).unwrap_or(false);
        if !request_policy_accept {
            request_side_policy_reject_rows += 1;
            row.nando_shadow_request = None;
            row.provider_cost_microusd = None;
            row.verified_safe_accept = None;
            row.notes = Some(format!(
                "{}; mixed_safe_policy_promote_v2 request_policy={} provider_cost_estimate_microusd={} policy_accept=false",
                row.notes
                    .clone()
                    .unwrap_or_else(|| "real_codex_trace".to_owned()),
                request_side_policy_name,
                provider_cost_microusd
            ));
            continue;
        }

        request_side_policy_accept_rows += 1;
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;
        let Some(request) = &mut row.nando_shadow_request else {
            no_score_rows += 1;
            continue;
        };
        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            threshold,
        );
        request.expect_local_operator = Some(policy_accept);
        if policy_accept {
            policy_accept_rows += 1;
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
        }
        let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
        runtime_acceptance_mismatches += usize::from(runtime_response.accepted != policy_accept);
        row.notes = Some(format!(
            "{}; mixed_safe_policy_promote_v2 request_policy={} runtime_policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            request_side_policy_name,
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_mixed_safe_policy_promote_v2".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "MIXED_SAFE_POLICY_PROMOTE_V2_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "MIXED_SAFE_POLICY_PROMOTE_V2_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: Some(history_path.display().to_string()),
        request_side_policy_name: Some(request_side_policy_name.to_owned()),
        calibration_policy_name: calibration_policy.policy_name.clone(),
        calibration_policy_threshold: calibration_policy.threshold,
        selected_policy_name: policy.policy_name.clone(),
        selected_policy_source: policy.selection_source.clone(),
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy.accepts,
        selected_policy_true_accepts: policy.true_accepts,
        selected_policy_false_accepts: policy.false_accepts,
        selected_policy_unverified_accepts: policy.unverified_accepts,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows,
        request_side_policy_accept_rows,
        request_side_policy_reject_rows,
        history_prompt_missing_rows,
        policy_accept_rows,
        policy_accept_verified_true_rows,
        policy_accept_verified_false_rows,
        policy_accept_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It creates a promoted serving registry with request-side mixed-map admission plus energy threshold. Offline labels/evidence choose the threshold, but serving uses only request text admission plus score >= threshold. It does not prove market savings until shadow plus verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the v2 promoted registry/trace, then feed that audit into CPU route feedback. Goal/control prompts stay fallback until a separate route proves them safe.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-mixed-safe-policy-promote-v2: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  request_side_policy_name: {}", request_side_policy_name);
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!(
        "  request_side_policy_accept_rows: {}",
        report.request_side_policy_accept_rows
    );
    println!("  policy_accept_rows: {}", report.policy_accept_rows);
    println!(
        "  policy_accept_verified_true_rows: {}",
        report.policy_accept_verified_true_rows
    );
    println!(
        "  policy_accept_verified_false_rows: {}",
        report.policy_accept_verified_false_rows
    );
    println!(
        "  policy_accept_unverified_rows: {}",
        report.policy_accept_unverified_rows
    );
    Err("mixed safe policy v2 promotion is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_safe_policy_promote_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let base_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_PROFILE_REGISTRY_CONFIG));
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_LOCAL_ACCEPT_CALIBRATION_REPORT));
    let promoted_registry_config_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_REGISTRY_CONFIG));
    let promoted_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_TRACE_JSONL));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_REPORT));
    let provider_cost_microusd = args
        .next()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|error| format!("invalid provider_cost_microusd '{}': {error}", value))
        })
        .transpose()?
        .unwrap_or(100);

    let mut promoted_config =
        read_json_file::<RoleBindingProfileRegistryConfig>(&base_registry_config_path)?;
    validate_registry_config(&promoted_config)?;
    let calibration =
        read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(&calibration_report_path)?;
    let mut trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let Some(calibration_policy) = select_supported_mixed_safe_policy(&calibration) else {
        return Err(
            "edit calibration report has no supported safe policy candidate for runtime promotion"
                .to_owned(),
        );
    };
    let base_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&base_registry_config_path)?;
    let policy = select_mixed_promotion_policy_from_evidence(
        &base_registry,
        &trace_rows,
        calibration_policy,
        "edit_marker_length",
    )?;
    let threshold = policy.threshold;
    let acceptance_policy = "energy_threshold_only".to_owned();
    let edit_profile_ids = trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.as_ref())
        .filter(|request| {
            request
                .route_key
                .as_deref()
                .is_some_and(|route| route.contains("edit_marker_length"))
        })
        .filter_map(|request| request.profile_id.clone())
        .collect::<BTreeSet<_>>();
    if edit_profile_ids.is_empty() {
        return Err("edit safe policy promotion found no edit profile ids in trace".to_owned());
    }
    let mut promoted_profile_ids = Vec::new();
    for profile in &mut promoted_config.profiles {
        if edit_profile_ids.contains(&profile.profile_id) {
            profile.threshold = threshold;
            profile.acceptance_policy = acceptance_policy.clone();
            promoted_profile_ids.push(profile.profile_id.clone());
        }
    }
    if promoted_profile_ids.is_empty() {
        return Err(format!(
            "edit safe policy promotion found no matching profiles in registry for {:?}",
            edit_profile_ids
        ));
    }
    validate_registry_config(&promoted_config)?;
    write_json_file(&promoted_registry_config_path, &promoted_config)?;
    let promoted_registry =
        RoleBindingProfileRuntimeRegistry::from_config_path(&promoted_registry_config_path)?;

    let mut scoreable_candidate_calls = 0usize;
    let mut policy_accept_rows = 0usize;
    let mut policy_accept_verified_true_rows = 0usize;
    let mut policy_accept_verified_false_rows = 0usize;
    let mut policy_accept_unverified_rows = 0usize;
    let mut provider_cost_events_written = 0usize;
    let mut runtime_acceptance_mismatches = 0usize;
    let mut no_score_rows = 0usize;

    for row in &mut trace_rows {
        let Some(request) = &mut row.nando_shadow_request else {
            continue;
        };
        let is_edit = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains("edit_marker_length"));
        if !is_edit {
            continue;
        }
        scoreable_candidate_calls +=
            usize::from(!request.active_fringe.is_empty() && !request.slots.is_empty());
        row.provider_cost_microusd = Some(provider_cost_microusd);
        provider_cost_events_written += 1;

        let Some(score) = score_role_binding_profile_request_detailed(&promoted_registry, request)
        else {
            no_score_rows += 1;
            request.expect_local_operator = Some(false);
            continue;
        };
        let strict_ordered_pass = score.slot_margins.iter().all(|margin| *margin > 0);
        let policy_accept = profile_accepts_score(
            &acceptance_policy,
            strict_ordered_pass,
            score.energy_margin,
            threshold,
        );
        request.expect_local_operator = Some(policy_accept);
        if policy_accept {
            policy_accept_rows += 1;
            policy_accept_verified_true_rows += usize::from(row.verified_safe_accept == Some(true));
            policy_accept_verified_false_rows +=
                usize::from(row.verified_safe_accept == Some(false));
            policy_accept_unverified_rows += usize::from(row.verified_safe_accept.is_none());
        }
        let runtime_response = score_role_binding_profile_request(&promoted_registry, request);
        runtime_acceptance_mismatches += usize::from(runtime_response.accepted != policy_accept);
        row.notes = Some(format!(
            "{}; edit_safe_policy_promote_v1 policy={} threshold={} provider_cost_estimate_microusd={} policy_accept={}",
            row.notes
                .clone()
                .unwrap_or_else(|| "real_codex_trace".to_owned()),
            acceptance_policy,
            threshold,
            provider_cost_microusd,
            policy_accept
        ));
    }

    write_real_traffic_trace_jsonl(&promoted_trace_path, &trace_rows)?;
    let report = RoleBindingMixedSafePolicyPromoteReport {
        schema_version: "nando_role_binding_edit_safe_policy_promote_v1".to_owned(),
        verdict: if policy_accept_rows > 0
            && policy_accept_verified_false_rows == 0
            && policy_accept_unverified_rows == 0
            && runtime_acceptance_mismatches == 0
        {
            "EDIT_SAFE_POLICY_PROMOTE_V1_REVIEW_PROMOTED_TRACE_READY"
        } else {
            "EDIT_SAFE_POLICY_PROMOTE_V1_REVIEW_REQUIRES_SHADOW_AUDIT"
        }
        .to_owned(),
        base_registry_config_path: base_registry_config_path.display().to_string(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        calibration_report_path: calibration_report_path.display().to_string(),
        promoted_registry_config_path: promoted_registry_config_path.display().to_string(),
        promoted_trace_path: promoted_trace_path.display().to_string(),
        history_path: None,
        request_side_policy_name: None,
        calibration_policy_name: calibration_policy.policy_name.clone(),
        calibration_policy_threshold: calibration_policy.threshold,
        selected_policy_name: policy.policy_name.clone(),
        selected_policy_source: policy.selection_source.clone(),
        selected_policy_threshold: threshold,
        selected_acceptance_policy: acceptance_policy,
        selected_policy_accepts: policy.accepts,
        selected_policy_true_accepts: policy.true_accepts,
        selected_policy_false_accepts: policy.false_accepts,
        selected_policy_unverified_accepts: policy.unverified_accepts,
        promoted_profile_ids,
        provider_cost_microusd,
        trace_rows_written: trace_rows.len(),
        scoreable_candidate_calls,
        request_side_policy_evaluated_rows: 0,
        request_side_policy_accept_rows: 0,
        request_side_policy_reject_rows: 0,
        history_prompt_missing_rows: 0,
        policy_accept_rows,
        policy_accept_verified_true_rows,
        policy_accept_verified_false_rows,
        policy_accept_unverified_rows,
        provider_cost_events_written,
        no_score_rows,
        runtime_acceptance_mismatches,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        market_claim_allowed: false,
        claim_boundary: "Promotion artifact only. It creates a promoted serving registry with an explicit edit-route acceptance policy and rewrites a shadow trace with provider-cost estimates. Offline labels/evidence may choose the threshold, but serving uses only request-side score >= threshold. It does not prove market savings until role-binding-real-traffic-shadow-v1 and verification-hook audit pass with false_accepts=0 and unverified_shadow_accepts=0.".to_owned(),
        next_engineering_debt: "Run role-binding-real-traffic-shadow-v1 and verification-hook-audit-v1 on the promoted edit registry/trace. Only a shadow PASS with provider cost, non-synthetic rows, and false_accepts=0 can advance verified CPU routability.".to_owned(),
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-safe-policy-promote-v1: {}",
        report.verdict
    );
    println!(
        "  promoted_registry: {}",
        promoted_registry_config_path.display()
    );
    println!("  promoted_trace: {}", promoted_trace_path.display());
    println!("  report: {}", report_path.display());
    println!("  selected_policy_name: {}", report.selected_policy_name);
    println!(
        "  selected_policy_source: {}",
        report.selected_policy_source
    );
    println!(
        "  selected_policy_threshold: {}",
        report.selected_policy_threshold
    );
    println!("  policy_accept_rows: {}", report.policy_accept_rows);
    println!(
        "  policy_accept_verified_true_rows: {}",
        report.policy_accept_verified_true_rows
    );
    println!(
        "  policy_accept_verified_false_rows: {}",
        report.policy_accept_verified_false_rows
    );
    println!(
        "  policy_accept_unverified_rows: {}",
        report.policy_accept_unverified_rows
    );
    Err("edit safe policy promotion is review-only; run shadow/audit before claims".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_edit_admission_calibration_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let evidence_trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_OUTPUT_EVIDENCE_TRACE_JSONL));
    let history_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/ubu/.codex/history.jsonl"));
    let report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_ADMISSION_CALIBRATION_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&evidence_trace_path)?;
    let history_rows = read_codex_history_jsonl(&history_path)?;
    let history_by_fingerprint = history_rows
        .iter()
        .map(|row| {
            (
                format!(
                    "fnv1a64:{:016x}",
                    stable_real_traffic_fingerprint64(row.text.as_bytes())
                ),
                row.text.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    let mut hook_ready_rows = 0usize;
    let mut label_true_rows = 0usize;
    let mut label_false_rows = 0usize;
    let mut history_prompt_missing_rows = 0usize;

    for trace in &trace_rows {
        let Some(label) = trace.verified_safe_accept else {
            continue;
        };
        if trace.nando_shadow_request.is_none() {
            continue;
        }
        hook_ready_rows += 1;
        label_true_rows += usize::from(label);
        label_false_rows += usize::from(!label);
        let request_fingerprint = trace.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            history_prompt_missing_rows += 1;
            continue;
        };
        let features = extract_edit_admission_features(prompt_text);
        rows.push(RoleBindingEditAdmissionCalibrationRow {
            trace_id: trace.trace_id.clone(),
            request_fingerprint: trace.request_fingerprint.clone(),
            response_fingerprint: trace.response_fingerprint.clone(),
            verifier_label: label,
            features,
        });
    }

    let minimum_true_support = 2usize;
    let policies = edit_admission_policy_reports(&rows, minimum_true_support);
    let robust_safe_policy_found = policies.iter().any(|policy| policy.robust_safe);
    let singleton_safe_policy_found = policies.iter().any(|policy| policy.singleton_safe);
    let best_robust_true_accepts = policies
        .iter()
        .filter(|policy| policy.robust_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let best_singleton_true_accepts = policies
        .iter()
        .filter(|policy| policy.singleton_safe)
        .map(|policy| policy.true_accepts)
        .max()
        .unwrap_or(0);
    let feature_counts = edit_admission_feature_counts(&rows);
    let report = RoleBindingEditAdmissionCalibrationReport {
        schema_version: "nando_role_binding_edit_admission_calibration_v1".to_owned(),
        verdict: if robust_safe_policy_found {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_ROBUST_POLICY_CANDIDATE_FOUND"
        } else if singleton_safe_policy_found {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_SINGLETON_ONLY_NO_ROBUST_POLICY"
        } else {
            "EDIT_ADMISSION_CALIBRATION_V1_REVIEW_NO_SAFE_POLICY"
        }
        .to_owned(),
        evidence_trace_path: evidence_trace_path.display().to_string(),
        history_path: history_path.display().to_string(),
        hook_ready_rows,
        rows_with_prompt_features: rows.len(),
        history_prompt_missing_rows,
        label_true_rows,
        label_false_rows,
        minimum_true_support,
        robust_safe_policy_found,
        singleton_safe_policy_found,
        best_robust_true_accepts,
        best_singleton_true_accepts,
        feature_counts,
        policies,
        rows,
        raw_prompt_text_written: false,
        raw_response_text_written: false,
        response_text_used_for_features: false,
        target_labels_used_for_runtime: false,
        proof_labels_used_for_runtime: false,
        local_accepts_enabled: false,
        market_claim_allowed: false,
        claim_boundary: "Admission calibration only. It reads real request text at analysis time, writes only fingerprints/features/counts, uses verification labels only to evaluate candidate request-side gates, enables no local accepts, and cannot be used as market savings claim.".to_owned(),
        next_engineering_debt: if robust_safe_policy_found {
            "Promote the robust admission candidate only through a separate shadow trace rewrite and false_accepts=0 gate; do not count singleton policies as product proof.".to_owned()
        } else {
            "Current real edit request-side features do not provide a robust safe admission gate. Leave edit local accepts disabled and either improve edit features with more real evidence or build the conditional/mixed payload builders.".to_owned()
        },
    };
    write_json_file(&report_path, &report)?;
    println!(
        "role-binding-real-traffic-edit-admission-calibration-v1: {}",
        report.verdict
    );
    println!("  evidence_trace: {}", evidence_trace_path.display());
    println!("  history: {}", history_path.display());
    println!("  report: {}", report_path.display());
    println!("  hook_ready_rows: {}", report.hook_ready_rows);
    println!(
        "  rows_with_prompt_features: {}",
        report.rows_with_prompt_features
    );
    println!("  label_true_rows: {}", report.label_true_rows);
    println!("  label_false_rows: {}", report.label_false_rows);
    println!(
        "  robust_safe_policy_found: {}",
        report.robust_safe_policy_found
    );
    println!(
        "  singleton_safe_policy_found: {}",
        report.singleton_safe_policy_found
    );
    println!(
        "  best_robust_true_accepts: {}",
        report.best_robust_true_accepts
    );
    Err("edit admission calibration is review-only".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_verification_hook_audit_v1<I>(
    mut args: I,
) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let trace_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_TRACE_JSONL));
    let shadow_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_SHADOW_REPORT));
    let audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_VERIFICATION_HOOK_AUDIT_REPORT));

    let trace_rows = read_real_traffic_trace_jsonl(&trace_path)?;
    let shadow_report = read_json_file::<RoleBindingRealTrafficShadowReport>(&shadow_report_path)?;
    let shadow_by_trace = shadow_report
        .rows
        .iter()
        .map(|row| (row.trace_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();

    let mut route_accumulators =
        BTreeMap::<String, RoleBindingVerificationHookRouteAccumulator>::new();
    let mut total_llm_calls = 0usize;
    let mut operator_candidate_calls = 0usize;
    let mut scoreable_candidate_calls = 0usize;
    let mut local_accepts_disabled_events = 0usize;
    let mut local_accepts_enabled_events = 0usize;
    let mut response_fingerprint_events = 0usize;
    let mut tool_call_fingerprint_events = 0usize;
    let mut verification_source_events = 0usize;
    let mut explicit_verified_safe_accept_events = 0usize;
    let mut verified_true_events = 0usize;
    let mut verified_false_events = 0usize;
    let mut provider_cost_events = 0usize;
    let mut candidates_missing_output_evidence = 0usize;
    let mut candidates_missing_verification_source = 0usize;
    let mut candidates_missing_explicit_verification = 0usize;
    let mut candidates_missing_provider_cost = 0usize;
    let mut verification_hook_ready_events = 0usize;
    let mut verified_cpu_accept_eligible_events = 0usize;

    for row in &trace_rows {
        total_llm_calls += usize::from(row.llm_call);
        response_fingerprint_events += usize::from(nonempty_option(&row.response_fingerprint));
        tool_call_fingerprint_events += usize::from(!row.tool_call_fingerprints.is_empty());
        verification_source_events += usize::from(nonempty_option(&row.verification_source));
        explicit_verified_safe_accept_events += usize::from(row.verified_safe_accept.is_some());
        verified_true_events += usize::from(row.verified_safe_accept == Some(true));
        verified_false_events += usize::from(row.verified_safe_accept == Some(false));
        provider_cost_events += usize::from(row.provider_cost_microusd.is_some());

        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        operator_candidate_calls += 1;
        let route_key = request
            .route_key
            .clone()
            .unwrap_or_else(|| "none".to_owned());
        let profile_id = request
            .profile_id
            .clone()
            .unwrap_or_else(|| "route_selected".to_owned());
        let route = route_accumulators
            .entry(route_key.clone())
            .or_insert_with(|| RoleBindingVerificationHookRouteAccumulator {
                route_key,
                profile_id,
                ..RoleBindingVerificationHookRouteAccumulator::default()
            });
        route.candidate_calls += 1;

        let scoreable = !request.active_fringe.is_empty() && !request.slots.is_empty();
        scoreable_candidate_calls += usize::from(scoreable);
        route.scoreable_candidate_calls += usize::from(scoreable);

        let local_accepts_disabled = request.expect_local_operator == Some(false);
        local_accepts_disabled_events += usize::from(local_accepts_disabled);
        local_accepts_enabled_events += usize::from(!local_accepts_disabled);
        route.local_accepts_disabled_events += usize::from(local_accepts_disabled);
        route.local_accepts_enabled_events += usize::from(!local_accepts_disabled);

        let has_output_evidence = trace_row_has_output_evidence(row);
        let has_verification_source = nonempty_option(&row.verification_source);
        let has_explicit_verification = row.verified_safe_accept.is_some();
        let has_provider_cost = row.provider_cost_microusd.is_some();
        candidates_missing_output_evidence += usize::from(!has_output_evidence);
        candidates_missing_verification_source += usize::from(!has_verification_source);
        candidates_missing_explicit_verification += usize::from(!has_explicit_verification);
        candidates_missing_provider_cost += usize::from(!has_provider_cost);
        route.candidates_missing_output_evidence += usize::from(!has_output_evidence);
        route.candidates_missing_verification_source += usize::from(!has_verification_source);
        route.candidates_missing_explicit_verification += usize::from(!has_explicit_verification);
        route.candidates_missing_provider_cost += usize::from(!has_provider_cost);

        let hook_ready =
            has_output_evidence && has_verification_source && has_explicit_verification;
        verification_hook_ready_events += usize::from(hook_ready);
        route.verification_hook_ready_events += usize::from(hook_ready);

        if let Some(shadow) = shadow_by_trace.get(row.trace_id.as_str()) {
            route.shadow_accepts += usize::from(shadow.nando_shadow_accepted);
            route.shadow_fallbacks += usize::from(shadow.nando_shadow_fallback);
            route.false_accepts += usize::from(shadow.false_local_accept);
            let eligible = hook_ready
                && has_provider_cost
                && shadow.nando_shadow_accepted
                && row.verified_safe_accept == Some(true)
                && row.synthetic_source != Some(true);
            verified_cpu_accept_eligible_events += usize::from(eligible);
            route.verified_cpu_accept_eligible_events += usize::from(eligible);
        }
    }

    let routes = route_accumulators
        .into_values()
        .map(RoleBindingVerificationHookRouteRow::from)
        .collect::<Vec<_>>();
    let all_trace_rows_matched_shadow = trace_rows.iter().all(|row| {
        shadow_by_trace.contains_key(row.trace_id.as_str()) || row.nando_shadow_request.is_none()
    });
    let market_claim_allowed = verified_cpu_accept_eligible_events > 0
        && shadow_report.false_accepts == 0
        && shadow_report.unverified_shadow_accepts == 0
        && shadow_report.verified_safe_accepts > 0
        && !shadow_report.synthetic_trace_used
        && shadow_report.incremental_savings_over_exact_cache > 0;
    let report = RoleBindingVerificationHookAuditReport {
        schema_version: "nando_role_binding_real_traffic_verification_hook_audit_v1".to_owned(),
        verdict: if verification_hook_ready_events > 0 {
            "VERIFICATION_HOOK_AUDIT_V1_REVIEW_READY_HOOKS_FOUND"
        } else {
            "VERIFICATION_HOOK_AUDIT_V1_REVIEW_MISSING_HOOKS"
        }
        .to_owned(),
        trace_path: trace_path.display().to_string(),
        shadow_report_path: shadow_report_path.display().to_string(),
        total_requests: trace_rows.len(),
        total_llm_calls,
        operator_candidate_calls,
        scoreable_candidate_calls,
        local_accepts_disabled_events,
        local_accepts_enabled_events,
        response_fingerprint_events,
        tool_call_fingerprint_events,
        verification_source_events,
        explicit_verified_safe_accept_events,
        verified_true_events,
        verified_false_events,
        provider_cost_events,
        candidates_missing_output_evidence,
        candidates_missing_verification_source,
        candidates_missing_explicit_verification,
        candidates_missing_provider_cost,
        verification_hook_ready_events,
        verified_cpu_accept_eligible_events,
        all_trace_rows_matched_shadow,
        shadow_accepts: shadow_report.nando_shadow_accepts,
        shadow_fallbacks: shadow_report.nando_shadow_fallbacks,
        shadow_false_accepts: shadow_report.false_accepts,
        shadow_incremental_savings_over_exact_cache: shadow_report.incremental_savings_over_exact_cache,
        market_claim_allowed,
        routes,
        claim_boundary: "Verification-hook audit only. A real CPU accept can be counted only when the trace row has output evidence, verification_source, explicit verified_safe_accept=true, provider cost for savings, a shadow local accept, non-synthetic source, false_accepts=0, and unverified_shadow_accepts=0. Missing hooks or unverified accepts keep the route in REVIEW.".to_owned(),
        next_engineering_debt: "Attach real response/tool-call evidence and deterministic edit-output verification to scoreable request payloads before enabling local accepts.".to_owned(),
    };
    write_json_file(&audit_report_path, &report)?;
    println!(
        "role-binding-real-traffic-verification-hook-audit-v1: {}",
        report.verdict
    );
    println!("  trace: {}", trace_path.display());
    println!("  shadow_report: {}", shadow_report_path.display());
    println!("  audit_report: {}", audit_report_path.display());
    println!(
        "  operator_candidate_calls: {}",
        report.operator_candidate_calls
    );
    println!(
        "  scoreable_candidate_calls: {}",
        report.scoreable_candidate_calls
    );
    println!(
        "  verification_hook_ready_events: {}",
        report.verification_hook_ready_events
    );
    println!(
        "  verified_cpu_accept_eligible_events: {}",
        report.verified_cpu_accept_eligible_events
    );
    println!("  market_claim_allowed: {}", report.market_claim_allowed);
    Err("verification hook audit is review-only; it is not market savings".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_feedback_loop_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let forecast_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_CPU_ROUTE_FORECAST_REPORT));
    let edit_dry_run_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDIT_PAYLOAD_DRY_RUN_REPORT));
    let verification_audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_VERIFICATION_HOOK_AUDIT_REPORT));
    let feedback_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REAL_TRAFFIC_FEEDBACK_LOOP_REPORT));
    let planning_next_step_dry_run_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_PAYLOAD_DRY_RUN_REPORT));
    let planning_next_step_local_accept_calibration_report_path =
        args.next().map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_LOCAL_ACCEPT_CALIBRATION_REPORT)
        });
    let planning_next_step_verification_audit_report_path =
        args.next().map(PathBuf::from).unwrap_or_else(|| {
            PathBuf::from(DEFAULT_PLANNING_NEXT_STEP_ARTIFACT_PROGRESS_AUDIT_REPORT)
        });
    let agent_control_admission_calibration_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_ADMISSION_CALIBRATION_REPORT));
    let agent_control_safe_policy_audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_CONTROL_SAFE_POLICY_AUDIT_REPORT));
    let mixed_safe_policy_audit_report_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_MIXED_SAFE_POLICY_AUDIT_REPORT));

    let forecast = read_json_file::<RoleBindingCpuRouteForecastReport>(&forecast_report_path)?;
    let edit_dry_run =
        read_json_file::<RoleBindingEditPayloadDryRunReport>(&edit_dry_run_report_path)?;
    let edit_local_accept_calibration_report_path =
        PathBuf::from(DEFAULT_EDIT_LOCAL_ACCEPT_CALIBRATION_REPORT);
    let edit_local_accept_calibration = if edit_local_accept_calibration_report_path.exists() {
        Some(
            read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(
                &edit_local_accept_calibration_report_path,
            )?,
        )
    } else {
        None
    };
    let conditional_dry_run_report_path = PathBuf::from(DEFAULT_CONDITIONAL_PAYLOAD_DRY_RUN_REPORT);
    let conditional_dry_run = if conditional_dry_run_report_path.exists() {
        Some(read_json_file::<RoleBindingConditionalPayloadDryRunReport>(
            &conditional_dry_run_report_path,
        )?)
    } else {
        None
    };
    let mixed_dry_run_report_path = PathBuf::from(DEFAULT_MIXED_PAYLOAD_DRY_RUN_REPORT);
    let mixed_dry_run = if mixed_dry_run_report_path.exists() {
        Some(read_json_file::<RoleBindingMixedPayloadDryRunReport>(
            &mixed_dry_run_report_path,
        )?)
    } else {
        None
    };
    let agent_control_dry_run_report_path =
        PathBuf::from(DEFAULT_AGENT_CONTROL_PAYLOAD_DRY_RUN_REPORT);
    let agent_control_dry_run = if agent_control_dry_run_report_path.exists() {
        Some(
            read_json_file::<RoleBindingAgentControlPayloadDryRunReport>(
                &agent_control_dry_run_report_path,
            )?,
        )
    } else {
        None
    };
    let verification_audit =
        read_json_file::<RoleBindingVerificationHookAuditReport>(&verification_audit_report_path)?;
    let agent_control_audit_report_path =
        PathBuf::from(DEFAULT_AGENT_CONTROL_OUTPUT_EVIDENCE_AUDIT_REPORT);
    let agent_control_verification_audit = if agent_control_audit_report_path.exists() {
        Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
            &agent_control_audit_report_path,
        )?)
    } else {
        None
    };
    let agent_control_safe_policy_verification_audit =
        if agent_control_safe_policy_audit_report_path.exists() {
            Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
                &agent_control_safe_policy_audit_report_path,
            )?)
        } else {
            None
        };
    let agent_control_admission_calibration = if agent_control_admission_calibration_report_path
        .exists()
    {
        Some(read_json_file::<
            RoleBindingAgentControlAdmissionCalibrationReport,
        >(&agent_control_admission_calibration_report_path)?)
    } else {
        None
    };
    let edit_safe_policy_audit_report_path = PathBuf::from(DEFAULT_EDIT_SAFE_POLICY_AUDIT_REPORT);
    let edit_safe_policy_verification_audit = if edit_safe_policy_audit_report_path.exists() {
        Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
            &edit_safe_policy_audit_report_path,
        )?)
    } else {
        None
    };
    let conditional_audit_report_path =
        PathBuf::from(DEFAULT_CONDITIONAL_OUTPUT_EVIDENCE_AUDIT_REPORT);
    let conditional_verification_audit = if conditional_audit_report_path.exists() {
        Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
            &conditional_audit_report_path,
        )?)
    } else {
        None
    };
    let conditional_safe_policy_audit_report_path =
        PathBuf::from(DEFAULT_CONDITIONAL_SAFE_POLICY_AUDIT_REPORT);
    let conditional_safe_policy_verification_audit =
        if conditional_safe_policy_audit_report_path.exists() {
            Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
                &conditional_safe_policy_audit_report_path,
            )?)
        } else {
            None
        };
    let conditional_local_accept_calibration_report_path =
        PathBuf::from(DEFAULT_CONDITIONAL_LOCAL_ACCEPT_CALIBRATION_REPORT);
    let conditional_local_accept_calibration =
        if conditional_local_accept_calibration_report_path.exists() {
            Some(
                read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(
                    &conditional_local_accept_calibration_report_path,
                )?,
            )
        } else {
            None
        };
    let mixed_audit_report_path = PathBuf::from(DEFAULT_MIXED_OUTPUT_EVIDENCE_AUDIT_REPORT);
    let mixed_verification_audit = if mixed_audit_report_path.exists() {
        Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
            &mixed_audit_report_path,
        )?)
    } else {
        None
    };
    let mixed_safe_policy_verification_audit = if mixed_safe_policy_audit_report_path.exists() {
        Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
            &mixed_safe_policy_audit_report_path,
        )?)
    } else {
        None
    };
    let mixed_local_accept_calibration_report_path =
        PathBuf::from(DEFAULT_MIXED_LOCAL_ACCEPT_CALIBRATION_REPORT);
    let mixed_local_accept_calibration = if mixed_local_accept_calibration_report_path.exists() {
        Some(
            read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(
                &mixed_local_accept_calibration_report_path,
            )?,
        )
    } else {
        None
    };
    let planning_next_step_dry_run = if planning_next_step_dry_run_report_path.exists() {
        Some(read_json_file::<
            RoleBindingPlanningNextStepPayloadDryRunReport,
        >(&planning_next_step_dry_run_report_path)?)
    } else {
        None
    };
    let planning_next_step_verification_audit =
        if planning_next_step_verification_audit_report_path.exists() {
            Some(read_json_file::<RoleBindingVerificationHookAuditReport>(
                &planning_next_step_verification_audit_report_path,
            )?)
        } else {
            None
        };
    let planning_next_step_local_accept_calibration =
        if planning_next_step_local_accept_calibration_report_path.exists() {
            Some(
                read_json_file::<RoleBindingEditLocalAcceptCalibrationReport>(
                    &planning_next_step_local_accept_calibration_report_path,
                )?,
            )
        } else {
            None
        };
    let mut verification_by_route = verification_audit
        .routes
        .iter()
        .map(|row| (row.route_key.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    if let Some(agent_control_audit) = &agent_control_verification_audit {
        for row in &agent_control_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(agent_control_safe_policy_audit) = &agent_control_safe_policy_verification_audit {
        for row in &agent_control_safe_policy_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(edit_safe_policy_audit) = &edit_safe_policy_verification_audit {
        for row in &edit_safe_policy_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(conditional_audit) = &conditional_verification_audit {
        for row in &conditional_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(conditional_safe_policy_audit) = &conditional_safe_policy_verification_audit {
        for row in &conditional_safe_policy_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(mixed_audit) = &mixed_verification_audit {
        for row in &mixed_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(mixed_safe_policy_audit) = &mixed_safe_policy_verification_audit {
        for row in &mixed_safe_policy_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    if let Some(planning_next_step_audit) = &planning_next_step_verification_audit {
        for row in &planning_next_step_audit.routes {
            verification_by_route.insert(row.route_key.as_str(), row);
        }
    }
    let effective_mixed_verification_audit = mixed_safe_policy_verification_audit
        .as_ref()
        .or(mixed_verification_audit.as_ref());
    let effective_edit_verification_audit = edit_safe_policy_verification_audit
        .as_ref()
        .unwrap_or(&verification_audit);
    let effective_conditional_verification_audit = conditional_safe_policy_verification_audit
        .as_ref()
        .or(conditional_verification_audit.as_ref());
    let effective_agent_control_verification_audit = agent_control_safe_policy_verification_audit
        .as_ref()
        .or(agent_control_verification_audit.as_ref());
    let effective_planning_next_step_verification_audit =
        planning_next_step_verification_audit.as_ref();

    let target_routability_milli = 800usize;
    let target_verified_cpu_calls =
        projected_accepts(forecast.total_llm_calls, target_routability_milli);
    let verification_hook_ready_events = effective_edit_verification_audit
        .verification_hook_ready_events
        + effective_conditional_verification_audit
            .map(|report| report.verification_hook_ready_events)
            .unwrap_or_default()
        + effective_mixed_verification_audit
            .map(|report| report.verification_hook_ready_events)
            .unwrap_or_default()
        + effective_agent_control_verification_audit
            .map(|report| report.verification_hook_ready_events)
            .unwrap_or_default()
        + effective_planning_next_step_verification_audit
            .map(|report| report.verification_hook_ready_events)
            .unwrap_or_default();
    let verified_cpu_accept_eligible_events = effective_edit_verification_audit
        .verified_cpu_accept_eligible_events
        + effective_conditional_verification_audit
            .map(|report| report.verified_cpu_accept_eligible_events)
            .unwrap_or_default()
        + effective_mixed_verification_audit
            .map(|report| report.verified_cpu_accept_eligible_events)
            .unwrap_or_default()
        + effective_agent_control_verification_audit
            .map(|report| report.verified_cpu_accept_eligible_events)
            .unwrap_or_default()
        + effective_planning_next_step_verification_audit
            .map(|report| report.verified_cpu_accept_eligible_events)
            .unwrap_or_default();
    let planning_next_step_candidate_calls = effective_planning_next_step_verification_audit
        .map(|report| report.operator_candidate_calls)
        .unwrap_or_default();
    let operator_candidate_calls =
        forecast.operator_candidate_calls + planning_next_step_candidate_calls;
    let routing_gap_to_80_calls =
        target_verified_cpu_calls.saturating_sub(operator_candidate_calls);
    let verified_gap_to_80_calls =
        target_verified_cpu_calls.saturating_sub(verified_cpu_accept_eligible_events);
    let mut route_rows = Vec::new();

    for route in &forecast.routes {
        let verification = verification_by_route.get(route.route_key.as_str()).copied();
        let is_edit_route = route.route_key.contains("edit_marker_length");
        let is_conditional_route = route.route_key.contains("conditional_branch");
        let is_mixed_route = route.route_key.contains("mixed_map");
        let is_agent_control_route = route.route_key.contains("agent_control");
        let local_accept_calibration = if is_edit_route {
            edit_local_accept_calibration.as_ref()
        } else if is_conditional_route {
            conditional_local_accept_calibration.as_ref()
        } else if is_mixed_route {
            mixed_local_accept_calibration.as_ref()
        } else {
            None
        };
        let local_accept_calibration_ran = local_accept_calibration.is_some()
            || (is_agent_control_route && agent_control_admission_calibration.is_some());
        let local_accept_safe_policy_found = if is_agent_control_route {
            agent_control_admission_calibration
                .as_ref()
                .map(|report| report.robust_safe_policy_found)
                .unwrap_or(false)
        } else {
            local_accept_calibration
                .map(|report| report.safe_policy_found)
                .unwrap_or(false)
        };
        let local_accept_best_safe_true_accepts = if is_agent_control_route {
            agent_control_admission_calibration
                .as_ref()
                .map(|report| report.best_robust_true_accepts)
                .unwrap_or_default()
        } else {
            local_accept_calibration
                .map(|report| report.best_safe_true_accepts)
                .unwrap_or_default()
        };
        let local_accept_minimum_true_support = DEFAULT_REAL_TRAFFIC_MIN_SAFE_POLICY_TRUE_SUPPORT;
        let local_accept_support_qualified = local_accept_safe_policy_found
            && local_accept_best_safe_true_accepts >= local_accept_minimum_true_support;
        let payload_ready_events = if is_edit_route {
            edit_dry_run.payload_ready_events
        } else if is_conditional_route {
            conditional_dry_run
                .as_ref()
                .map(|report| report.payload_ready_events)
                .unwrap_or_default()
        } else if is_mixed_route {
            mixed_dry_run
                .as_ref()
                .map(|report| report.payload_ready_events)
                .unwrap_or_default()
        } else if is_agent_control_route {
            agent_control_dry_run
                .as_ref()
                .map(|report| report.payload_built_events)
                .unwrap_or_default()
        } else {
            0
        };
        let payload_built_events = if is_edit_route {
            edit_dry_run.payload_built_events
        } else if is_conditional_route {
            conditional_dry_run
                .as_ref()
                .map(|report| report.payload_built_events)
                .unwrap_or_default()
        } else if is_mixed_route {
            mixed_dry_run
                .as_ref()
                .map(|report| report.payload_built_events)
                .unwrap_or_default()
        } else if is_agent_control_route {
            agent_control_dry_run
                .as_ref()
                .map(|report| report.payload_built_events)
                .unwrap_or_default()
        } else {
            0
        };
        let scoreable_payload_events = verification
            .map(|row| row.scoreable_candidate_calls)
            .or_else(|| {
                if is_conditional_route {
                    conditional_dry_run
                        .as_ref()
                        .map(|report| report.scoreable_payload_events)
                } else if is_mixed_route {
                    mixed_dry_run
                        .as_ref()
                        .map(|report| report.scoreable_payload_events)
                } else if is_agent_control_route {
                    agent_control_dry_run
                        .as_ref()
                        .map(|report| report.scoreable_payload_events)
                } else {
                    None
                }
            })
            .unwrap_or_default();
        let verification_hook_ready_events = verification
            .map(|row| row.verification_hook_ready_events)
            .unwrap_or_default();
        let verified_cpu_accept_eligible_events = verification
            .map(|row| row.verified_cpu_accept_eligible_events)
            .unwrap_or_default();
        let false_accepts = verification
            .map(|row| row.false_accepts)
            .unwrap_or_default();
        let stage = feedback_route_stage(FeedbackRouteStageInputs {
            payload_ready_events,
            payload_built_events,
            scoreable_payload_events,
            verification_hook_ready_events,
            verified_cpu_accept_eligible_events,
            false_accepts,
            local_accept_calibration_ran,
            local_accept_safe_policy_found,
            local_accept_support_qualified,
        });
        let next_action = feedback_route_next_action(&stage);
        route_rows.push(RoleBindingFeedbackLoopRouteRow {
            priority_rank: route.priority_rank,
            route_key: route.route_key.clone(),
            profile_id: route.profile_id.clone(),
            candidate_events: route.candidate_events,
            non_exact_candidate_calls: route.non_exact_candidate_calls,
            payload_builder: route.recommended_payload_builder.clone(),
            stage,
            next_action,
            payload_ready_events,
            payload_built_events,
            scoreable_payload_events,
            verification_hook_ready_events,
            local_accept_calibration_ran,
            local_accept_safe_policy_found,
            local_accept_minimum_true_support,
            local_accept_support_qualified,
            local_accept_best_safe_true_accepts,
            verified_cpu_accept_eligible_events,
            false_accepts,
            candidate_share_milli_of_all_llm_calls: route.candidate_share_milli_of_all_llm_calls,
            scoreable_share_milli_of_all_llm_calls: ratio_milli(
                scoreable_payload_events,
                forecast.total_llm_calls,
            ),
            verified_share_milli_of_all_llm_calls: ratio_milli(
                verified_cpu_accept_eligible_events,
                forecast.total_llm_calls,
            ),
        });
    }

    if planning_next_step_dry_run.is_some()
        || planning_next_step_verification_audit.is_some()
        || planning_next_step_local_accept_calibration.is_some()
    {
        let verification = verification_by_route
            .get(REAL_TRAFFIC_PLANNING_ROUTE_KEY)
            .copied();
        let payload_ready_events = planning_next_step_dry_run
            .as_ref()
            .map(|report| report.payload_ready_events)
            .unwrap_or_default();
        let payload_built_events = planning_next_step_dry_run
            .as_ref()
            .map(|report| report.payload_built_events)
            .unwrap_or_default();
        let scoreable_payload_events = verification
            .map(|row| row.scoreable_candidate_calls)
            .or_else(|| {
                planning_next_step_dry_run
                    .as_ref()
                    .map(|report| report.scoreable_payload_events)
            })
            .unwrap_or_default();
        let verification_hook_ready_events = verification
            .map(|row| row.verification_hook_ready_events)
            .unwrap_or_default();
        let verified_cpu_accept_eligible_events = verification
            .map(|row| row.verified_cpu_accept_eligible_events)
            .unwrap_or_default();
        let false_accepts = verification
            .map(|row| row.false_accepts)
            .unwrap_or_default();
        let candidate_events = verification
            .map(|row| row.candidate_calls)
            .or_else(|| {
                planning_next_step_dry_run
                    .as_ref()
                    .map(|report| report.planning_next_step_candidate_events)
            })
            .unwrap_or_default();
        let local_accept_calibration_ran = planning_next_step_local_accept_calibration.is_some();
        let local_accept_safe_policy_found = planning_next_step_local_accept_calibration
            .as_ref()
            .map(|report| report.safe_policy_found)
            .unwrap_or(false);
        let local_accept_best_safe_true_accepts = planning_next_step_local_accept_calibration
            .as_ref()
            .map(|report| report.best_safe_true_accepts)
            .unwrap_or_default();
        let local_accept_minimum_true_support = DEFAULT_REAL_TRAFFIC_MIN_SAFE_POLICY_TRUE_SUPPORT;
        let local_accept_support_qualified = local_accept_safe_policy_found
            && local_accept_best_safe_true_accepts >= local_accept_minimum_true_support;
        let stage = feedback_route_stage(FeedbackRouteStageInputs {
            payload_ready_events,
            payload_built_events,
            scoreable_payload_events,
            verification_hook_ready_events,
            verified_cpu_accept_eligible_events,
            false_accepts,
            local_accept_calibration_ran,
            local_accept_safe_policy_found,
            local_accept_support_qualified,
        });
        let next_action = feedback_route_next_action(&stage);
        route_rows.push(RoleBindingFeedbackLoopRouteRow {
            priority_rank: route_rows.len() + 1,
            route_key: REAL_TRAFFIC_PLANNING_ROUTE_KEY.to_owned(),
            profile_id: REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned(),
            candidate_events,
            non_exact_candidate_calls: candidate_events,
            payload_builder: "planning_next_step_payload_builder_v1".to_owned(),
            stage,
            next_action,
            payload_ready_events,
            payload_built_events,
            scoreable_payload_events,
            verification_hook_ready_events,
            local_accept_calibration_ran,
            local_accept_safe_policy_found,
            local_accept_minimum_true_support,
            local_accept_support_qualified,
            local_accept_best_safe_true_accepts,
            verified_cpu_accept_eligible_events,
            false_accepts,
            candidate_share_milli_of_all_llm_calls: ratio_milli(
                candidate_events,
                forecast.total_llm_calls,
            ),
            scoreable_share_milli_of_all_llm_calls: ratio_milli(
                scoreable_payload_events,
                forecast.total_llm_calls,
            ),
            verified_share_milli_of_all_llm_calls: ratio_milli(
                verified_cpu_accept_eligible_events,
                forecast.total_llm_calls,
            ),
        });
    }

    let scoreable_candidate_calls = route_rows
        .iter()
        .map(|row| row.scoreable_payload_events)
        .sum();
    let report = RoleBindingFeedbackLoopReport {
        schema_version: "nando_role_binding_real_traffic_feedback_loop_v1".to_owned(),
        verdict: "CPU_ROUTE_FEEDBACK_LOOP_V1_REVIEW".to_owned(),
        forecast_report_path: forecast_report_path.display().to_string(),
        edit_dry_run_report_path: edit_dry_run_report_path.display().to_string(),
        edit_local_accept_calibration_report_path: edit_local_accept_calibration
            .as_ref()
            .map(|_| edit_local_accept_calibration_report_path.display().to_string()),
        edit_safe_policy_verification_audit_report_path: edit_safe_policy_verification_audit
            .as_ref()
            .map(|_| edit_safe_policy_audit_report_path.display().to_string()),
        planning_next_step_dry_run_report_path: planning_next_step_dry_run
            .as_ref()
            .map(|_| planning_next_step_dry_run_report_path.display().to_string()),
        planning_next_step_local_accept_calibration_report_path:
            planning_next_step_local_accept_calibration.as_ref().map(|_| {
                planning_next_step_local_accept_calibration_report_path
                    .display()
                    .to_string()
            }),
        planning_next_step_verification_audit_report_path:
            planning_next_step_verification_audit.as_ref().map(|_| {
                planning_next_step_verification_audit_report_path
                    .display()
                    .to_string()
            }),
        agent_control_dry_run_report_path: agent_control_dry_run
            .as_ref()
            .map(|_| agent_control_dry_run_report_path.display().to_string()),
        agent_control_verification_audit_report_path: agent_control_verification_audit
            .as_ref()
            .map(|_| agent_control_audit_report_path.display().to_string()),
        agent_control_safe_policy_verification_audit_report_path:
            agent_control_safe_policy_verification_audit
                .as_ref()
                .map(|_| {
                    agent_control_safe_policy_audit_report_path
                        .display()
                        .to_string()
                }),
        agent_control_admission_calibration_report_path: agent_control_admission_calibration
            .as_ref()
            .map(|_| {
                agent_control_admission_calibration_report_path
                    .display()
                    .to_string()
            }),
        conditional_dry_run_report_path: conditional_dry_run
            .as_ref()
            .map(|_| conditional_dry_run_report_path.display().to_string()),
        conditional_local_accept_calibration_report_path: conditional_local_accept_calibration
            .as_ref()
            .map(|_| {
                conditional_local_accept_calibration_report_path
                    .display()
                    .to_string()
            }),
        conditional_verification_audit_report_path: conditional_verification_audit
            .as_ref()
            .map(|_| conditional_audit_report_path.display().to_string()),
        conditional_safe_policy_verification_audit_report_path:
            conditional_safe_policy_verification_audit
                .as_ref()
                .map(|_| {
                    conditional_safe_policy_audit_report_path
                        .display()
                        .to_string()
                }),
        mixed_dry_run_report_path: mixed_dry_run
            .as_ref()
            .map(|_| mixed_dry_run_report_path.display().to_string()),
        mixed_local_accept_calibration_report_path: mixed_local_accept_calibration
            .as_ref()
            .map(|_| mixed_local_accept_calibration_report_path.display().to_string()),
        mixed_verification_audit_report_path: mixed_verification_audit
            .as_ref()
            .map(|_| mixed_audit_report_path.display().to_string()),
        mixed_safe_policy_verification_audit_report_path: mixed_safe_policy_verification_audit
            .as_ref()
            .map(|_| mixed_safe_policy_audit_report_path.display().to_string()),
        verification_audit_report_path: verification_audit_report_path.display().to_string(),
        total_llm_calls: forecast.total_llm_calls,
        exact_cache_hits: forecast.exact_cache_hits,
        exact_cache_coverage_milli: forecast.exact_cache_coverage_milli,
        operator_candidate_calls,
        operator_candidate_coverage_milli: ratio_milli(operator_candidate_calls, forecast.total_llm_calls),
        scoreable_candidate_calls,
        scoreable_candidate_coverage_milli: ratio_milli(
            scoreable_candidate_calls,
            forecast.total_llm_calls,
        ),
        verification_hook_ready_events,
        verification_hook_coverage_milli: ratio_milli(verification_hook_ready_events, forecast.total_llm_calls),
        verified_cpu_accept_eligible_events,
        verified_cpu_routability_milli: ratio_milli(
            verified_cpu_accept_eligible_events,
            forecast.total_llm_calls,
        ),
        target_routability_milli,
        target_verified_cpu_calls,
        routing_gap_to_80_calls,
        verified_gap_to_80_calls,
        no_candidate_calls: forecast
            .no_candidate_calls
            .saturating_sub(planning_next_step_candidate_calls),
        market_claim_allowed: false,
        routes: route_rows,
        claim_boundary: "Feedback loop only. Exact-cache coverage, route candidate coverage, scoreable payloads, verification hooks, and verified CPU accepts are separate stages. CPU Routability 80 is not achieved until verified_cpu_routability_milli >= 800 on non-synthetic real traffic with false_accepts=0.".to_owned(),
        next_engineering_debt: "Promote any agent-control admission candidate only through a separate request-side-admitted shadow trace with false_accepts=0; improve edit/conditional request-side admission or payload geometry after failed local-accept calibration; attach mixed output evidence after mixed payload dry-run; add provider cost evidence before any savings claim.".to_owned(),
    };
    write_json_file(&feedback_report_path, &report)?;
    println!(
        "role-binding-real-traffic-feedback-loop-v1: {}",
        report.verdict
    );
    println!("  forecast_report: {}", forecast_report_path.display());
    println!(
        "  edit_dry_run_report: {}",
        edit_dry_run_report_path.display()
    );
    if let Some(path) = &report.edit_local_accept_calibration_report_path {
        println!("  edit_local_accept_calibration_report: {path}");
    }
    if let Some(path) = &report.edit_safe_policy_verification_audit_report_path {
        println!("  edit_safe_policy_verification_audit_report: {path}");
    }
    if let Some(path) = &report.planning_next_step_dry_run_report_path {
        println!("  planning_next_step_dry_run_report: {path}");
    }
    if let Some(path) = &report.planning_next_step_local_accept_calibration_report_path {
        println!("  planning_next_step_local_accept_calibration_report: {path}");
    }
    if let Some(path) = &report.planning_next_step_verification_audit_report_path {
        println!("  planning_next_step_verification_audit_report: {path}");
    }
    if let Some(path) = &report.conditional_dry_run_report_path {
        println!("  conditional_dry_run_report: {path}");
    }
    if let Some(path) = &report.conditional_local_accept_calibration_report_path {
        println!("  conditional_local_accept_calibration_report: {path}");
    }
    if let Some(path) = &report.conditional_verification_audit_report_path {
        println!("  conditional_verification_audit_report: {path}");
    }
    if let Some(path) = &report.conditional_safe_policy_verification_audit_report_path {
        println!("  conditional_safe_policy_verification_audit_report: {path}");
    }
    if let Some(path) = &report.mixed_dry_run_report_path {
        println!("  mixed_dry_run_report: {path}");
    }
    if let Some(path) = &report.mixed_local_accept_calibration_report_path {
        println!("  mixed_local_accept_calibration_report: {path}");
    }
    if let Some(path) = &report.mixed_verification_audit_report_path {
        println!("  mixed_verification_audit_report: {path}");
    }
    if let Some(path) = &report.mixed_safe_policy_verification_audit_report_path {
        println!("  mixed_safe_policy_verification_audit_report: {path}");
    }
    if let Some(path) = &report.agent_control_admission_calibration_report_path {
        println!("  agent_control_admission_calibration_report: {path}");
    }
    if let Some(path) = &report.agent_control_safe_policy_verification_audit_report_path {
        println!("  agent_control_safe_policy_verification_audit_report: {path}");
    }
    println!(
        "  verification_audit_report: {}",
        verification_audit_report_path.display()
    );
    println!("  feedback_report: {}", feedback_report_path.display());
    println!("  total_llm_calls: {}", report.total_llm_calls);
    println!(
        "  operator_candidate_calls: {}",
        report.operator_candidate_calls
    );
    println!(
        "  scoreable_candidate_calls: {}",
        report.scoreable_candidate_calls
    );
    println!(
        "  verification_hook_ready_events: {}",
        report.verification_hook_ready_events
    );
    println!(
        "  verified_cpu_routability_milli: {}",
        report.verified_cpu_routability_milli
    );
    println!(
        "  verified_gap_to_80_calls: {}",
        report.verified_gap_to_80_calls
    );
    Err("CPU route feedback loop is review-only; routability 80 is not achieved".to_owned())
}

pub(crate) fn run_role_binding_real_traffic_shadow_smoke_v1<I>(mut args: I) -> Result<(), String>
where
    I: Iterator<Item = String>,
{
    let binary_suite_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROLE_BINDING_BINARY_EVAL_PACK_SUITE_REPORT));
    let trace_path = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(
            "target/nando-wave/real-traffic-shadow/real-traffic-shadow-smoke-v1.trace.jsonl",
        )
    });
    let max_unique_sequences_per_profile = args
        .next()
        .map(|value| {
            value.parse::<usize>().map_err(|error| {
                format!(
                    "invalid max_unique_sequences_per_profile '{}': {error}",
                    value
                )
            })
        })
        .transpose()?
        .unwrap_or(8);
    let binary_suite =
        read_json_file::<RoleBindingProfileBinaryEvalPackSuiteReport>(&binary_suite_path)?;
    if !binary_suite.gate_pass {
        return Err(format!(
            "binary eval-pack suite is not green: verdict={}",
            binary_suite.verdict
        ));
    }
    if let Some(parent) = trace_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create real-traffic shadow smoke trace dir {}: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(&trace_path, "").map_err(|error| {
        format!(
            "failed to reset real-traffic shadow smoke trace {}: {error}",
            trace_path.display()
        )
    })?;
    let mut rows_written = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let eval_pack =
            parse_profile_binary_eval_pack(Path::new(&suite_row.binary_eval_pack_path))?;
        let profile_limit = max_unique_sequences_per_profile.min(eval_pack.sequences.len());
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            let request = sequence_to_profile_request(&profile_id, sequence, &cache_key, 0);
            let row = RoleBindingRealTrafficTraceRow {
                schema_version: "nando_role_binding_real_traffic_trace_v1".to_owned(),
                trace_id: format!("synthetic_smoke::{}::{}", profile_id, sequence.task_id),
                traffic_source: Some("synthetic_binary_eval_pack_smoke".to_owned()),
                time_ms: None,
                request_fingerprint: Some(format!(
                    "synthetic_request::{}::{}",
                    profile_id, sequence.task_id
                )),
                response_fingerprint: Some(format!(
                    "synthetic_response::{}::{}",
                    profile_id, sequence.task_id
                )),
                tool_call_fingerprints: Vec::new(),
                verification_source: Some("binary_eval_pack_sequence_expectation".to_owned()),
                llm_call: true,
                exact_cache_key: Some(cache_key),
                provider_cache_hit: Some(false),
                provider_cost_microusd: Some(100),
                nando_shadow_request: Some(request),
                verified_safe_accept: Some(sequence.expect_local_operator),
                synthetic_source: Some(true),
                notes: Some("synthetic smoke only; forbidden as market savings claim".to_owned()),
            };
            append_jsonl_row(&trace_path, &row)?;
            rows_written += 1;
        }
    }
    println!("role-binding-real-traffic-shadow-smoke-v1: REAL_TRAFFIC_SHADOW_SMOKE_TRACE_READY");
    println!("  binary_suite: {}", binary_suite_path.display());
    println!("  trace: {}", trace_path.display());
    println!("  rows_written: {rows_written}");
    println!("  synthetic_source: true");
    Ok(())
}

#[derive(Clone, Debug)]
struct RoleBindingProfileRuntime {
    config: RoleBindingProfileConfig,
    runtime: WavePredictorRoleBindingOffloadRuntime,
    package_info: WavePredictorRoleBindingPackageInfo,
    package_bytes: usize,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileRuntimeRegistry {
    profiles: Vec<RoleBindingProfileRuntime>,
    route_index: HashMap<String, usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RoleBindingProfileRuntimeLoadMode {
    ServingPackedOnly,
    WithReference,
}

impl RoleBindingProfileRuntimeRegistry {
    fn from_config_path(config_path: &Path) -> Result<Self, String> {
        Self::from_config_path_with_mode(
            config_path,
            RoleBindingProfileRuntimeLoadMode::ServingPackedOnly,
        )
    }

    fn from_config_path_with_reference(config_path: &Path) -> Result<Self, String> {
        Self::from_config_path_with_mode(
            config_path,
            RoleBindingProfileRuntimeLoadMode::WithReference,
        )
    }

    fn from_config_path_with_mode(
        config_path: &Path,
        load_mode: RoleBindingProfileRuntimeLoadMode,
    ) -> Result<Self, String> {
        let config = read_json_file::<RoleBindingProfileRegistryConfig>(config_path)?;
        validate_registry_config(&config)?;
        let mut profiles = Vec::new();
        let mut route_index = HashMap::new();
        for profile in config.profiles {
            if profile
                .package_path
                .extension()
                .and_then(|ext| ext.to_str())
                != Some("nwrb")
            {
                return Err(format!(
                    "serving registry may load only .nwrb packages: {}",
                    profile.package_path.display()
                ));
            }
            let package_bytes = fs::read(&profile.package_path).map_err(|error| {
                format!("failed to read {}: {error}", profile.package_path.display())
            })?;
            let policy =
                WavePredictorRoleBindingOffloadPolicy::new(profile.threshold).map_err(|error| {
                    format!("invalid threshold for {}: {error:?}", profile.profile_id)
                })?;
            let runtime = match load_mode {
                RoleBindingProfileRuntimeLoadMode::ServingPackedOnly => {
                    WavePredictorRoleBindingOffloadRuntime::from_package_bytes_serving_packed_only(
                        &package_bytes,
                        policy,
                    )
                }
                RoleBindingProfileRuntimeLoadMode::WithReference => {
                    WavePredictorRoleBindingOffloadRuntime::from_package_bytes_with_reference(
                        &package_bytes,
                        policy,
                    )
                }
            }
            .map_err(|error| {
                format!(
                    "failed to load .nwrb package {}: {error:?}",
                    profile.package_path.display()
                )
            })?;
            let package_info = runtime.package_info();
            let index = profiles.len();
            for route_key in &profile.accepted_route_keys {
                if route_index.insert(route_key.clone(), index).is_some() {
                    return Err(format!("duplicate accepted route_key: {route_key}"));
                }
            }
            profiles.push(RoleBindingProfileRuntime {
                config: profile,
                runtime,
                package_info,
                package_bytes: package_bytes.len(),
            });
        }
        if profiles.is_empty() {
            return Err("profile registry has no profiles".to_owned());
        }
        Ok(Self {
            profiles,
            route_index,
        })
    }

    fn profile_count(&self) -> usize {
        self.profiles.len()
    }

    fn total_runtime_bytes_estimate(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.runtime.bytes_estimate())
            .sum()
    }

    fn total_package_bytes(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.package_bytes)
            .sum()
    }

    fn total_edge_count(&self) -> usize {
        self.profiles
            .iter()
            .map(|profile| profile.package_info.edge_count)
            .sum()
    }

    fn profile_summaries(&self) -> Vec<RoleBindingProfileSummary> {
        self.profiles
            .iter()
            .map(|profile| RoleBindingProfileSummary {
                profile_id: profile.config.profile_id.clone(),
                profile_kind: profile.config.profile_kind.clone(),
                operator_classes: profile.config.operator_classes.clone(),
                package_path: profile.config.package_path.display().to_string(),
                package_fingerprint64: profile.package_info.fingerprint64,
                package_bytes: profile.package_bytes,
                runtime_bytes_estimate: profile.runtime.bytes_estimate(),
                edge_count: profile.package_info.edge_count,
                slot_count: profile.config.slot_count,
                threshold: profile.config.threshold,
                acceptance_policy: profile.config.acceptance_policy.clone(),
                accepted_route_keys: profile.config.accepted_route_keys.clone(),
            })
            .collect()
    }

    fn select_profile(
        &self,
        request: &RoleBindingProfileScoreRequest,
    ) -> Option<&RoleBindingProfileRuntime> {
        if let Some(profile_id) = request.profile_id.as_deref() {
            return self
                .profiles
                .iter()
                .find(|profile| profile.config.profile_id == profile_id);
        }
        let route_key = request.route_key.as_deref()?;
        let index = self.route_index.get(route_key)?;
        self.profiles.get(*index)
    }

    fn self_test_score_request(&self) -> Option<RoleBindingProfileScoreRequest> {
        let profile_id = self.profiles.first()?.config.profile_id.clone();
        self.self_test_score_request_for_profile(&profile_id)
    }

    fn self_test_score_request_for_profile(
        &self,
        profile_id: &str,
    ) -> Option<RoleBindingProfileScoreRequest> {
        let smoke_strength = 64_i16;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.config.profile_id == profile_id)?;
        let edge = profile.runtime.sample_positive_edge()?;
        let estimated_margin =
            i32::from(smoke_strength) * i32::from(smoke_strength) * i32::from(edge.weight);
        if estimated_margin < profile.runtime.policy().local_margin_threshold {
            return None;
        }
        let lane_id = 17_u16;
        let signed_strength = if edge.sign_key == 0 { 1 } else { -1 };
        let role_center = profile
            .package_info
            .role_base
            .checked_add(u32::from(edge.slot_id).saturating_mul(profile.package_info.role_stride))?
            .checked_add(u32::from(lane_id))?;
        Some(RoleBindingProfileScoreRequest {
            request_id: "smoke_local".to_owned(),
            route_key: profile.config.accepted_route_keys.first().cloned(),
            profile_id: Some(profile.config.profile_id.clone()),
            exact_cache_key: Some("smoke_local_cache_key".to_owned()),
            active_fringe: vec![
                RoleBindingProfileActiveCenterRow {
                    center_id: edge.action_center,
                    strength: smoke_strength,
                },
                RoleBindingProfileActiveCenterRow {
                    center_id: role_center,
                    strength: smoke_strength,
                },
            ],
            slots: vec![RoleBindingProfileScoreSlotRow {
                binding_output_slot: Some(edge.output_slot_id),
                positive_impulses: vec![RoleBindingProfileImpulseRow {
                    lane_id,
                    signed_strength,
                }],
                negative_impulses: vec![RoleBindingProfileImpulseRow {
                    lane_id: lane_id.saturating_add(1),
                    signed_strength,
                }],
            }],
            expect_local_operator: Some(true),
        })
    }
}

#[derive(Debug)]
struct RoleBindingProfileLoadBalancerRuntime {
    upstreams: Vec<RoleBindingProfileLoadBalancerUpstreamRuntime>,
    profile_index: HashMap<String, usize>,
    route_index: HashMap<String, usize>,
}

#[derive(Debug)]
struct RoleBindingProfileLoadBalancerUpstreamRuntime {
    config: RoleBindingProfileLoadBalancerUpstreamConfig,
    addr: SocketAddr,
}

impl RoleBindingProfileLoadBalancerRuntime {
    fn from_config_path(config_path: &Path) -> Result<Self, String> {
        let config = read_json_file::<RoleBindingProfileLoadBalancerConfig>(config_path)?;
        validate_load_balancer_config(&config)?;
        let mut upstreams = Vec::new();
        let mut profile_index = HashMap::new();
        let mut route_index = HashMap::new();
        for upstream in config.upstreams {
            let index = upstreams.len();
            let addr = upstream
                .upstream_addr
                .parse::<SocketAddr>()
                .map_err(|error| {
                    format!(
                        "invalid load-balancer upstream addr {}: {error}",
                        upstream.upstream_addr
                    )
                })?;
            for profile_id in &upstream.profile_ids {
                if profile_index.insert(profile_id.clone(), index).is_some() {
                    return Err(format!("duplicate load-balancer profile_id: {profile_id}"));
                }
            }
            for route_key in &upstream.accepted_route_keys {
                if route_index.insert(route_key.clone(), index).is_some() {
                    return Err(format!("duplicate load-balancer route_key: {route_key}"));
                }
            }
            upstreams.push(RoleBindingProfileLoadBalancerUpstreamRuntime {
                config: upstream,
                addr,
            });
        }
        if upstreams.is_empty() {
            return Err("load-balancer config has no upstreams".to_owned());
        }
        Ok(Self {
            upstreams,
            profile_index,
            route_index,
        })
    }

    fn upstream_count(&self) -> usize {
        self.upstreams.len()
    }

    fn profile_count(&self) -> usize {
        self.profile_index.len()
    }

    fn select_upstream(
        &self,
        request: &RoleBindingProfileScoreRequest,
    ) -> Option<&RoleBindingProfileLoadBalancerUpstreamRuntime> {
        self.select_upstream_index(request)
            .and_then(|index| self.upstreams.get(index))
    }

    fn select_upstream_index(&self, request: &RoleBindingProfileScoreRequest) -> Option<usize> {
        if let Some(profile_id) = request.profile_id.as_deref() {
            return self.profile_index.get(profile_id).copied();
        }
        let route_key = request.route_key.as_deref()?;
        self.route_index.get(route_key).copied()
    }

    fn profile_rows(&self) -> Vec<RoleBindingProfileLoadBalancerProfileRow> {
        self.upstreams
            .iter()
            .flat_map(|upstream| {
                upstream.config.profile_ids.iter().map(move |profile_id| {
                    RoleBindingProfileLoadBalancerProfileRow {
                        profile_id: profile_id.clone(),
                        worker_id: upstream.config.worker_id,
                        upstream_addr: upstream.config.upstream_addr.clone(),
                        shard_config_path: upstream.config.shard_config_path.display().to_string(),
                    }
                })
            })
            .collect()
    }
}

fn serve_role_binding_profile_lb_requests(
    listener: TcpListener,
    load_balancer: Arc<RoleBindingProfileLoadBalancerRuntime>,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingProfileServeStats, String> {
    let mut stats = RoleBindingProfileServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream.map_err(|error| format!("failed to accept lb request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set lb tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set lb read timeout: {error}"))?;
        match handle_role_binding_profile_lb_http_request(&mut stream, &load_balancer, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
                write_http_json_error(&mut stream, error.status_code, &error.message)?;
            }
        }
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
    }
    Ok(stats)
}

fn handle_role_binding_profile_lb_http_request(
    stream: &mut TcpStream,
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    stats: &mut RoleBindingProfileServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
    let request = read_http_request(stream)?;
    let (head, body) = request
        .split_once("\r\n\r\n")
        .ok_or_else(|| http_error(400, "missing HTTP body"))?;
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP method"))?;
    let path = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP path"))?;
    let version = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP version"))?;
    if version != "HTTP/1.1" {
        return Err(http_error(400, "expected HTTP/1.1"));
    }

    match (method, path) {
        ("GET", "/health") => {
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileLoadBalancerHealthResponse::from_runtime(load_balancer),
            )
        }
        ("GET", "/profiles") => {
            stats.requests_handled += 1;
            stats.profiles_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileLoadBalancerProfilesResponse::from_runtime(load_balancer),
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileMetricsResponse::from_load_balancer(load_balancer, stats),
            )
        }
        ("POST", "/score") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse lb score JSON: {error}"))
                })?;
            let response = score_role_binding_profile_lb_request(load_balancer, &score_request);
            stats.record_score(&response);
            write_http_json(stream, 200, &response)
        }
        ("POST", "/replay") => {
            let replay_request = serde_json::from_str::<RoleBindingProfileReplayRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse lb replay JSON: {error}"))
                })?;
            let response =
                replay_role_binding_profile_lb_requests(load_balancer, &replay_request, stats);
            write_http_json(stream, 200, &response)
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported load-balancer HTTP route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported load-balancer HTTP method: {method} {path}"),
        )),
    }
}

fn serve_role_binding_profile_requests(
    listener: TcpListener,
    registry: Arc<RoleBindingProfileRuntimeRegistry>,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingProfileServeStats, String> {
    let mut stats = RoleBindingProfileServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream.map_err(|error| format!("failed to accept request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set read timeout: {error}"))?;
        match handle_role_binding_profile_http_request(&mut stream, &registry, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
                write_http_json_error(&mut stream, error.status_code, &error.message)?;
            }
        }
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
    }
    Ok(stats)
}

fn handle_role_binding_profile_http_request(
    stream: &mut TcpStream,
    registry: &RoleBindingProfileRuntimeRegistry,
    stats: &mut RoleBindingProfileServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
    let request = read_http_request(stream)?;
    let (head, body) = request
        .split_once("\r\n\r\n")
        .ok_or_else(|| http_error(400, "missing HTTP body"))?;
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP method"))?;
    let path = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP path"))?;
    let version = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP version"))?;
    if version != "HTTP/1.1" {
        return Err(http_error(400, "expected HTTP/1.1"));
    }

    match (method, path) {
        ("GET", "/health") => {
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileHealthResponse::from_registry(registry),
            )
        }
        ("GET", "/profiles") => {
            stats.requests_handled += 1;
            stats.profiles_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileProfilesResponse::from_registry(registry),
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingProfileMetricsResponse::from_registry(registry, stats),
            )
        }
        ("POST", "/score") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| http_error(400, format!("failed to parse score JSON: {error}")))?;
            let response = score_role_binding_profile_request(registry, &score_request);
            stats.record_score(&response);
            write_http_json(stream, 200, &response)
        }
        ("POST", "/score-compact") => {
            let score_request = serde_json::from_str::<RoleBindingProfileScoreRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse compact score JSON: {error}"))
                })?;
            let response = score_role_binding_profile_request(registry, &score_request);
            stats.record_score(&response);
            write_http_json(
                stream,
                200,
                &RoleBindingProfileScoreCompactResponse::from_score_response(&response),
            )
        }
        ("POST", "/replay") => {
            let replay_request = serde_json::from_str::<RoleBindingProfileReplayRequest>(body)
                .map_err(|error| {
                    http_error(400, format!("failed to parse replay JSON: {error}"))
                })?;
            let response = replay_role_binding_profile_requests(registry, &replay_request, stats);
            write_http_json(stream, 200, &response)
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported HTTP route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported HTTP method: {method} {path}"),
        )),
    }
}

fn serve_real_traffic_record_requests(
    listener: TcpListener,
    trace_path: PathBuf,
    config: RoleBindingProfileServeConfig,
) -> Result<RoleBindingRealTrafficRecordServeStats, String> {
    let mut stats = RoleBindingRealTrafficRecordServeStats::default();
    for stream in listener.incoming() {
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
        let mut stream = stream
            .map_err(|error| format!("failed to accept real-traffic recorder request: {error}"))?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to set recorder tcp nodelay: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set recorder read timeout: {error}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
            .map_err(|error| format!("failed to set recorder write timeout: {error}"))?;
        match handle_real_traffic_record_http_request(&mut stream, &trace_path, &mut stats) {
            Ok(()) => {}
            Err(error) => {
                stats.bad_requests += 1;
                write_http_json_error(&mut stream, error.status_code, &error.message)?;
            }
        }
        if config
            .request_limit
            .is_some_and(|limit| stats.total_requests() >= limit)
        {
            break;
        }
    }
    Ok(stats)
}

fn handle_real_traffic_record_http_request(
    stream: &mut TcpStream,
    trace_path: &Path,
    stats: &mut RoleBindingRealTrafficRecordServeStats,
) -> Result<(), RoleBindingProfileHttpError> {
    let request = read_http_request(stream)?;
    let (head, body) = request
        .split_once("\r\n\r\n")
        .ok_or_else(|| http_error(400, "missing HTTP body"))?;
    let request_line = head
        .lines()
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP request line"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP method"))?;
    let path = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP path"))?;
    let version = parts
        .next()
        .ok_or_else(|| http_error(400, "missing HTTP version"))?;
    if version != "HTTP/1.1" {
        return Err(http_error(400, "expected HTTP/1.1"));
    }

    match (method, path) {
        ("GET", "/health") => {
            stats.requests_handled += 1;
            stats.health_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordHealthResponse {
                    schema_version: "nando_role_binding_real_traffic_record_health_v1".to_owned(),
                    status: "ok".to_owned(),
                    runtime: "nando-wave-real-traffic-recorder".to_owned(),
                    trace_path: trace_path.display().to_string(),
                    compiler_used: false,
                    eval_packs_loaded: false,
                    corpus_jsonl_loaded: false,
                    python_demo_used: false,
                },
            )
        }
        ("GET", "/metrics") => {
            stats.requests_handled += 1;
            stats.metrics_requests += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordMetricsResponse::from_stats(trace_path, stats),
            )
        }
        ("POST", "/trace") => {
            let row = serde_json::from_str::<RoleBindingRealTrafficTraceRow>(body)
                .map_err(|error| http_error(400, format!("failed to parse trace JSON: {error}")))?;
            validate_real_traffic_trace_row(&row)
                .map_err(|error| http_error(400, format!("invalid trace row: {error}")))?;
            append_jsonl_row(trace_path, &row)
                .map_err(|error| http_error(500, format!("failed to append trace row: {error}")))?;
            stats.requests_handled += 1;
            stats.trace_requests += 1;
            stats.rows_written += 1;
            write_http_json(
                stream,
                200,
                &RoleBindingRealTrafficRecordTraceResponse {
                    schema_version: "nando_role_binding_real_traffic_record_trace_response_v1"
                        .to_owned(),
                    status: "recorded".to_owned(),
                    trace_path: trace_path.display().to_string(),
                    trace_id: row.trace_id,
                    rows_written: stats.rows_written,
                    synthetic_source: row.synthetic_source.unwrap_or(false),
                },
            )
        }
        ("POST", _) | ("GET", _) => Err(http_error(
            404,
            format!("unsupported real-traffic recorder route: {method} {path}"),
        )),
        _ => Err(http_error(
            405,
            format!("unsupported real-traffic recorder method: {method} {path}"),
        )),
    }
}

fn score_role_binding_profile_request(
    registry: &RoleBindingProfileRuntimeRegistry,
    request: &RoleBindingProfileScoreRequest,
) -> RoleBindingProfileScoreResponse {
    let start = Instant::now();
    let Some(profile) = registry.select_profile(request) else {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "profile_not_found",
            start.elapsed().as_nanos(),
        );
    };
    if request.active_fringe.is_empty() || request.slots.is_empty() {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "input_outside_profile_contract",
            start.elapsed().as_nanos(),
        );
    }
    let prepared = profile.runtime.prepare_active_fringe_from_iter(
        request
            .active_fringe
            .iter()
            .map(|active| (active.center_id, active.strength)),
    );
    let mut energy_margin = 0i32;
    let mut min_slot_margin = i32::MAX;
    let mut strict_ordered_pass = true;
    let core_start = Instant::now();
    for slot in &request.slots {
        let (positive_score, negative_score) =
            score_role_binding_profile_slot(&profile.runtime, &prepared, slot);
        let slot_margin = positive_score - negative_score;
        energy_margin += slot_margin;
        min_slot_margin = min_slot_margin.min(slot_margin);
        strict_ordered_pass &= slot_margin > 0;
    }
    let core_score_latency_ns = core_start.elapsed().as_nanos();
    if min_slot_margin == i32::MAX {
        return RoleBindingProfileScoreResponse::fallback(
            request,
            "no_scorable_slots",
            start.elapsed().as_nanos(),
        );
    }
    let threshold = profile.runtime.policy().local_margin_threshold;
    let accepted = profile_accepts_score(
        &profile.config.acceptance_policy,
        strict_ordered_pass,
        energy_margin,
        threshold,
    );
    let fallback_reason = profile_fallback_reason(
        &profile.config.acceptance_policy,
        strict_ordered_pass,
        energy_margin,
        threshold,
    );
    let false_local_accept = accepted && request.expect_local_operator == Some(false);
    let worker_score_latency_ns = start.elapsed().as_nanos();
    RoleBindingProfileScoreResponse {
        schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
        request_id: request.request_id.clone(),
        route_key: request.route_key.clone(),
        profile_id: Some(profile.config.profile_id.clone()),
        accepted,
        fallback: !accepted,
        fallback_reason,
        action: if accepted {
            "local_operator"
        } else {
            "fallback_to_llm"
        }
        .to_owned(),
        energy_margin,
        min_slot_margin,
        threshold,
        strict_ordered_pass,
        false_local_accept,
        latency_ns: worker_score_latency_ns,
        core_score_latency_ns,
        worker_score_latency_ns,
        lb_upstream_roundtrip_latency_ns: 0,
        lb_total_latency_ns: 0,
        package_fingerprint64: Some(profile.package_info.fingerprint64),
        runtime_bytes_estimate: profile.runtime.bytes_estimate(),
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

fn score_role_binding_profile_lb_request(
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    request: &RoleBindingProfileScoreRequest,
) -> RoleBindingProfileScoreResponse {
    let start = Instant::now();
    let Some(upstream) = load_balancer.select_upstream(request) else {
        let mut response = RoleBindingProfileScoreResponse::fallback(
            request,
            "load_balancer_route_not_found",
            start.elapsed().as_nanos(),
        );
        response.lb_total_latency_ns = response.latency_ns;
        return response;
    };
    let upstream_start = Instant::now();
    match send_json_post::<_, RoleBindingProfileScoreCompactResponse>(
        upstream.addr,
        "/score-compact",
        request,
    ) {
        Ok(compact_response) => {
            let upstream_roundtrip_latency_ns = upstream_start.elapsed().as_nanos();
            let lb_total_latency_ns = start.elapsed().as_nanos();
            let fallback_reason = if compact_response.accepted {
                None
            } else {
                Some("compact_worker_fallback".to_owned())
            };
            RoleBindingProfileScoreResponse {
                schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
                request_id: request.request_id.clone(),
                route_key: request.route_key.clone(),
                profile_id: request.profile_id.clone(),
                accepted: compact_response.accepted,
                fallback: compact_response.fallback,
                fallback_reason,
                action: compact_response.action,
                energy_margin: compact_response.energy_margin,
                min_slot_margin: compact_response.min_slot_margin,
                threshold: 0,
                strict_ordered_pass: compact_response.strict_ordered_pass,
                false_local_accept: compact_response.false_local_accept,
                latency_ns: lb_total_latency_ns,
                core_score_latency_ns: 0,
                worker_score_latency_ns: 0,
                lb_upstream_roundtrip_latency_ns: upstream_roundtrip_latency_ns,
                lb_total_latency_ns,
                package_fingerprint64: None,
                runtime_bytes_estimate: 0,
                forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
            }
        }
        Err(_) => {
            let upstream_roundtrip_latency_ns = upstream_start.elapsed().as_nanos();
            let mut response = RoleBindingProfileScoreResponse::fallback(
                request,
                "load_balancer_upstream_error",
                start.elapsed().as_nanos(),
            );
            response.lb_upstream_roundtrip_latency_ns = upstream_roundtrip_latency_ns;
            response.lb_total_latency_ns = response.latency_ns;
            response
        }
    }
}

fn score_role_binding_profile_slot(
    runtime: &WavePredictorRoleBindingOffloadRuntime,
    prepared: &WavePredictorRoleBindingPreparedFringe,
    slot: &RoleBindingProfileScoreSlotRow,
) -> (i32, i32) {
    let positive_score = slot
        .positive_impulses
        .iter()
        .map(|impulse| {
            runtime.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                slot.binding_output_slot,
            )
        })
        .sum::<i32>();
    let negative_score = slot
        .negative_impulses
        .iter()
        .map(|impulse| {
            runtime.score_alignment_prepared(
                prepared,
                impulse.lane_id,
                impulse.signed_strength,
                slot.binding_output_slot,
            )
        })
        .sum::<i32>();
    (positive_score, negative_score)
}

fn score_role_binding_profile_request_detailed(
    registry: &RoleBindingProfileRuntimeRegistry,
    request: &RoleBindingProfileScoreRequest,
) -> Option<RoleBindingProfileDetailedScore> {
    let profile = registry.select_profile(request)?;
    if request.active_fringe.is_empty() || request.slots.is_empty() {
        return None;
    }
    let prepared = profile.runtime.prepare_active_fringe_from_iter(
        request
            .active_fringe
            .iter()
            .map(|active| (active.center_id, active.strength)),
    );
    let mut energy_margin = 0i32;
    let mut min_slot_margin = i32::MAX;
    let mut slot_margins = Vec::with_capacity(request.slots.len());
    for slot in &request.slots {
        let (positive_score, negative_score) =
            score_role_binding_profile_slot(&profile.runtime, &prepared, slot);
        let slot_margin = positive_score - negative_score;
        energy_margin += slot_margin;
        min_slot_margin = min_slot_margin.min(slot_margin);
        slot_margins.push(slot_margin);
    }
    if min_slot_margin == i32::MAX {
        return None;
    }
    Some(RoleBindingProfileDetailedScore {
        energy_margin,
        min_slot_margin,
        slot_margins,
    })
}

fn replay_role_binding_profile_requests(
    registry: &RoleBindingProfileRuntimeRegistry,
    replay: &RoleBindingProfileReplayRequest,
    stats: &mut RoleBindingProfileServeStats,
) -> RoleBindingProfileReplayResponse {
    let mut seen_cache_keys = HashSet::new();
    let no_cache_llm_calls = replay.requests.len();
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut rows = Vec::new();

    for request in &replay.requests {
        let cache_key = request
            .exact_cache_key
            .clone()
            .unwrap_or_else(|| request.request_id.clone());
        if !seen_cache_keys.insert(cache_key.clone()) {
            rows.push(RoleBindingProfileReplayRow {
                request_id: request.request_id.clone(),
                exact_cache_key: cache_key,
                exact_cache_hit: true,
                action: "exact_cache_hit".to_owned(),
                accepted: false,
                fallback: false,
                margin: 0,
            });
            continue;
        }

        exact_cache_llm_calls += 1;
        let response = score_role_binding_profile_request(registry, request);
        stats.record_score(&response);
        false_local_accepts += usize::from(response.false_local_accept);
        missed_expected_local +=
            usize::from(request.expect_local_operator == Some(true) && !response.accepted);
        local_operator_calls += usize::from(response.accepted);
        fallback_to_llm_calls += usize::from(response.fallback);
        exact_cache_plus_nando_llm_calls += usize::from(!response.accepted);
        rows.push(RoleBindingProfileReplayRow {
            request_id: request.request_id.clone(),
            exact_cache_key: cache_key,
            exact_cache_hit: false,
            action: response.action,
            accepted: response.accepted,
            fallback: response.fallback,
            margin: response.energy_margin,
        });
    }

    stats.requests_handled += 1;
    stats.replay_requests += 1;
    let exact_cache_incremental_reduction_milli =
        ((exact_cache_llm_calls - exact_cache_plus_nando_llm_calls) * 1000)
            .checked_div(exact_cache_llm_calls)
            .unwrap_or(0);
    RoleBindingProfileReplayResponse {
        schema_version: "nando_role_binding_profile_replay_response_v1".to_owned(),
        request_id: replay.request_id.clone(),
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
        p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
        p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
        rows,
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

fn replay_role_binding_profile_lb_requests(
    load_balancer: &RoleBindingProfileLoadBalancerRuntime,
    replay: &RoleBindingProfileReplayRequest,
    stats: &mut RoleBindingProfileServeStats,
) -> RoleBindingProfileReplayResponse {
    let mut seen_cache_keys = HashSet::new();
    let no_cache_llm_calls = replay.requests.len();
    let mut exact_cache_llm_calls = 0usize;
    let mut exact_cache_plus_nando_llm_calls = 0usize;
    let mut false_local_accepts = 0usize;
    let mut missed_expected_local = 0usize;
    let mut local_operator_calls = 0usize;
    let mut fallback_to_llm_calls = 0usize;
    let mut rows = Vec::new();

    for request in &replay.requests {
        let cache_key = request
            .exact_cache_key
            .clone()
            .unwrap_or_else(|| request.request_id.clone());
        if !seen_cache_keys.insert(cache_key.clone()) {
            rows.push(RoleBindingProfileReplayRow {
                request_id: request.request_id.clone(),
                exact_cache_key: cache_key,
                exact_cache_hit: true,
                action: "exact_cache_hit".to_owned(),
                accepted: false,
                fallback: false,
                margin: 0,
            });
            continue;
        }

        exact_cache_llm_calls += 1;
        let response = score_role_binding_profile_lb_request(load_balancer, request);
        stats.record_score(&response);
        false_local_accepts += usize::from(response.false_local_accept);
        missed_expected_local +=
            usize::from(request.expect_local_operator == Some(true) && !response.accepted);
        local_operator_calls += usize::from(response.accepted);
        fallback_to_llm_calls += usize::from(response.fallback);
        exact_cache_plus_nando_llm_calls += usize::from(!response.accepted);
        rows.push(RoleBindingProfileReplayRow {
            request_id: request.request_id.clone(),
            exact_cache_key: cache_key,
            exact_cache_hit: false,
            action: response.action,
            accepted: response.accepted,
            fallback: response.fallback,
            margin: response.energy_margin,
        });
    }

    stats.requests_handled += 1;
    stats.replay_requests += 1;
    let exact_cache_incremental_reduction_milli =
        ((exact_cache_llm_calls - exact_cache_plus_nando_llm_calls) * 1000)
            .checked_div(exact_cache_llm_calls)
            .unwrap_or(0);
    RoleBindingProfileReplayResponse {
        schema_version: "nando_role_binding_profile_replay_response_v1".to_owned(),
        request_id: replay.request_id.clone(),
        no_cache_llm_calls,
        exact_cache_llm_calls,
        exact_cache_plus_nando_llm_calls,
        exact_cache_incremental_reduction_milli,
        local_operator_calls,
        fallback_to_llm_calls,
        false_local_accepts,
        missed_expected_local,
        p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
        p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
        p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
        rows,
        forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RoleBindingProfileServeConfig {
    request_limit: Option<usize>,
}

#[derive(Clone, Debug, Default)]
struct RoleBindingProfileServeStats {
    requests_handled: usize,
    score_requests: usize,
    replay_requests: usize,
    health_requests: usize,
    profiles_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    score_latencies_ns: Vec<u128>,
    core_score_latencies_ns: Vec<u128>,
    worker_score_latencies_ns: Vec<u128>,
    lb_upstream_roundtrip_latencies_ns: Vec<u128>,
}

impl RoleBindingProfileServeStats {
    fn total_requests(&self) -> usize {
        self.requests_handled + self.bad_requests
    }

    fn record_score(&mut self, response: &RoleBindingProfileScoreResponse) {
        self.requests_handled += 1;
        self.score_requests += 1;
        self.local_operator_calls += usize::from(response.accepted);
        self.fallback_to_llm_calls += usize::from(response.fallback);
        self.false_local_accepts += usize::from(response.false_local_accept);
        self.missed_expected_local +=
            usize::from(response.fallback && response.strict_ordered_pass);
        self.score_latencies_ns.push(response.latency_ns);
        if response.core_score_latency_ns > 0 {
            self.core_score_latencies_ns
                .push(response.core_score_latency_ns);
        }
        if response.worker_score_latency_ns > 0 {
            self.worker_score_latencies_ns
                .push(response.worker_score_latency_ns);
        }
        if response.lb_upstream_roundtrip_latency_ns > 0 {
            self.lb_upstream_roundtrip_latencies_ns
                .push(response.lb_upstream_roundtrip_latency_ns);
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RoleBindingRealTrafficRecordServeStats {
    requests_handled: usize,
    health_requests: usize,
    trace_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    rows_written: usize,
}

impl RoleBindingRealTrafficRecordServeStats {
    fn total_requests(&self) -> usize {
        self.requests_handled + self.bad_requests
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileRegistryConfig {
    schema_version: String,
    profiles: Vec<RoleBindingProfileConfig>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileConfig {
    profile_id: String,
    profile_kind: String,
    operator_classes: Vec<String>,
    package_path: PathBuf,
    runtime_bytes_estimate: usize,
    edge_count: usize,
    slot_count: u8,
    threshold: i32,
    #[serde(default = "default_profile_acceptance_policy")]
    acceptance_policy: String,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerConfig {
    schema_version: String,
    upstreams: Vec<RoleBindingProfileLoadBalancerUpstreamConfig>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerUpstreamConfig {
    worker_id: usize,
    upstream_addr: String,
    shard_config_path: PathBuf,
    profile_ids: Vec<String>,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRuntimeReleaseSuiteReport {
    verdict: String,
    gate_pass: bool,
    rows: Vec<RoleBindingRuntimeReleaseSuiteRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRuntimeReleaseSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_path: PathBuf,
    package_bytes: usize,
    package_edge_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileBinaryEvalPackSuiteReport {
    verdict: String,
    gate_pass: bool,
    rows: Vec<RoleBindingProfileBinaryEvalPackSuiteRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileBinaryEvalPackSuiteRow {
    label: String,
    seed: u8,
    margin_threshold: i32,
    package_fingerprint64: u64,
    binary_eval_pack_path: String,
    sequence_count: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileBinaryEvalPack {
    package_fingerprint64: Option<u64>,
    source_package_path: Option<String>,
    generation_method: String,
    sequences: Vec<RoleBindingProfileSequenceEvalRow>,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileSequenceEvalRow {
    task_id: String,
    active_fringe: Vec<RoleBindingProfileActiveCenterRow>,
    slots: Vec<RoleBindingProfileScoreSlotRow>,
    expect_local_operator: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileActiveCenterRow {
    center_id: u32,
    strength: i16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileImpulseRow {
    lane_id: u16,
    signed_strength: i16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreSlotRow {
    binding_output_slot: Option<u8>,
    positive_impulses: Vec<RoleBindingProfileImpulseRow>,
    negative_impulses: Vec<RoleBindingProfileImpulseRow>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreRequest {
    request_id: String,
    route_key: Option<String>,
    profile_id: Option<String>,
    exact_cache_key: Option<String>,
    active_fringe: Vec<RoleBindingProfileActiveCenterRow>,
    slots: Vec<RoleBindingProfileScoreSlotRow>,
    expect_local_operator: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayRequest {
    request_id: String,
    requests: Vec<RoleBindingProfileScoreRequest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreResponse {
    schema_version: String,
    request_id: String,
    route_key: Option<String>,
    profile_id: Option<String>,
    accepted: bool,
    fallback: bool,
    fallback_reason: Option<String>,
    action: String,
    energy_margin: i32,
    min_slot_margin: i32,
    threshold: i32,
    strict_ordered_pass: bool,
    false_local_accept: bool,
    latency_ns: u128,
    #[serde(default, skip_serializing)]
    core_score_latency_ns: u128,
    #[serde(default, skip_serializing)]
    worker_score_latency_ns: u128,
    #[serde(default, skip_serializing)]
    lb_upstream_roundtrip_latency_ns: u128,
    #[serde(default, skip_serializing)]
    lb_total_latency_ns: u128,
    package_fingerprint64: Option<u64>,
    runtime_bytes_estimate: usize,
    forbidden_flags: RoleBindingProfileForbiddenFlags,
}

impl RoleBindingProfileScoreResponse {
    fn fallback(request: &RoleBindingProfileScoreRequest, reason: &str, latency_ns: u128) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_score_response_v1".to_owned(),
            request_id: request.request_id.clone(),
            route_key: request.route_key.clone(),
            profile_id: request.profile_id.clone(),
            accepted: false,
            fallback: true,
            fallback_reason: Some(reason.to_owned()),
            action: "fallback_to_llm".to_owned(),
            energy_margin: 0,
            min_slot_margin: 0,
            threshold: 0,
            strict_ordered_pass: false,
            false_local_accept: false,
            latency_ns,
            core_score_latency_ns: 0,
            worker_score_latency_ns: latency_ns,
            lb_upstream_roundtrip_latency_ns: 0,
            lb_total_latency_ns: 0,
            package_fingerprint64: None,
            runtime_bytes_estimate: 0,
            forbidden_flags: RoleBindingProfileForbiddenFlags::clean(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileScoreCompactResponse {
    accepted: bool,
    fallback: bool,
    action: String,
    energy_margin: i32,
    min_slot_margin: i32,
    strict_ordered_pass: bool,
    false_local_accept: bool,
}

impl RoleBindingProfileScoreCompactResponse {
    fn from_score_response(response: &RoleBindingProfileScoreResponse) -> Self {
        Self {
            accepted: response.accepted,
            fallback: response.fallback,
            action: response.action.clone(),
            energy_margin: response.energy_margin,
            min_slot_margin: response.min_slot_margin,
            strict_ordered_pass: response.strict_ordered_pass,
            false_local_accept: response.false_local_accept,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayResponse {
    schema_version: String,
    request_id: String,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rows: Vec<RoleBindingProfileReplayRow>,
    forbidden_flags: RoleBindingProfileForbiddenFlags,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplayRow {
    request_id: String,
    exact_cache_key: String,
    exact_cache_hit: bool,
    action: String,
    accepted: bool,
    fallback: bool,
    margin: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficTraceRow {
    schema_version: String,
    trace_id: String,
    traffic_source: Option<String>,
    time_ms: Option<u64>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    #[serde(default)]
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_key: Option<String>,
    provider_cache_hit: Option<bool>,
    provider_cost_microusd: Option<u64>,
    nando_shadow_request: Option<RoleBindingProfileScoreRequest>,
    verified_safe_accept: Option<bool>,
    synthetic_source: Option<bool>,
    notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficEventRow {
    schema_version: String,
    event_id: String,
    traffic_source: Option<String>,
    time_ms: Option<u64>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    #[serde(default)]
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_key: Option<String>,
    provider_cache_hit: Option<bool>,
    provider_cost_microusd: Option<u64>,
    nando_shadow_request: Option<RoleBindingProfileScoreRequest>,
    verified_safe_accept: Option<bool>,
    synthetic_source: Option<bool>,
    notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficIngestReport {
    schema_version: String,
    verdict: String,
    events_path: String,
    trace_path: String,
    total_events: usize,
    llm_calls: usize,
    operator_candidate_events: usize,
    events_without_shadow_request: usize,
    synthetic_events: usize,
    rows_written: usize,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRow {
    session_id: String,
    ts: u64,
    text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryIngestReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    events_path: String,
    total_history_rows: usize,
    max_events: usize,
    events_written: usize,
    llm_calls: usize,
    nando_shadow_requests: usize,
    synthetic_events: usize,
    raw_text_written: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRouteCandidateReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    events_path: String,
    total_history_rows: usize,
    max_events: usize,
    events_written: usize,
    candidate_events: usize,
    no_candidate_events: usize,
    route_counts: Vec<RoleBindingCodexHistoryRouteCandidateCount>,
    raw_text_written: bool,
    full_shadow_request_payload_built: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCodexHistoryRouteCandidateCount {
    route_key: String,
    candidate_events: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficShadowReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    trace_path: String,
    traffic_source: String,
    time_window: String,
    total_requests: usize,
    total_llm_calls: usize,
    operator_candidate_calls: usize,
    exact_cache_hits: usize,
    provider_cache_hits: usize,
    nando_shadow_accepts: usize,
    nando_shadow_fallbacks: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    missed_expected_local: usize,
    incremental_savings_over_exact_cache: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    incremental_reduction_vs_exact_cache_milli: usize,
    estimated_cost_saved_microusd: u128,
    p50_shadow_score_latency_ns: u128,
    p90_shadow_score_latency_ns: u128,
    p99_shadow_score_latency_ns: u128,
    runtime_bytes_estimate: usize,
    rss_bytes: usize,
    serving_only_registry: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    synthetic_trace_used: bool,
    operator_rankings: Vec<RoleBindingRealTrafficOperatorRanking>,
    rows: Vec<RoleBindingRealTrafficShadowRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficShadowRow {
    trace_id: String,
    traffic_source: Option<String>,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    tool_call_fingerprints: Vec<String>,
    verification_source: Option<String>,
    llm_call: bool,
    exact_cache_hit: bool,
    provider_cache_hit: bool,
    nando_routable: bool,
    nando_shadow_accepted: bool,
    nando_shadow_fallback: bool,
    nando_shadow_action: String,
    verified_safe_accept: bool,
    unverified_shadow_accept: bool,
    false_local_accept: bool,
    shadow_score_latency_ns: u128,
    energy_margin: i32,
    min_slot_margin: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    trace_path: String,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordMetricsResponse {
    schema_version: String,
    trace_path: String,
    requests_handled: usize,
    health_requests: usize,
    trace_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    rows_written: usize,
}

impl RoleBindingRealTrafficRecordMetricsResponse {
    fn from_stats(trace_path: &Path, stats: &RoleBindingRealTrafficRecordServeStats) -> Self {
        Self {
            schema_version: "nando_role_binding_real_traffic_record_metrics_v1".to_owned(),
            trace_path: trace_path.display().to_string(),
            requests_handled: stats.requests_handled,
            health_requests: stats.health_requests,
            trace_requests: stats.trace_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            rows_written: stats.rows_written,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficRecordTraceResponse {
    schema_version: String,
    status: String,
    trace_path: String,
    trace_id: String,
    rows_written: usize,
    synthetic_source: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRealTrafficOperatorRanking {
    operator_key: String,
    route_key: String,
    profile_id: String,
    action: String,
    candidate_calls: usize,
    traffic_share_milli: usize,
    llm_calls: usize,
    exact_cache_hits: usize,
    nando_shadow_accepts: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    incremental_savings_over_exact_cache: usize,
    estimated_cost_saved_microusd: u128,
    local_accept_rate_milli: usize,
    verified_accept_rate_milli: usize,
    p99_shadow_score_latency_ns: u128,
    value_score_microusd_per_ms: u128,
}

#[derive(Clone, Debug, Default)]
struct RoleBindingRealTrafficOperatorAccumulator {
    route_key: String,
    profile_id: String,
    action: String,
    candidate_calls: usize,
    llm_calls: usize,
    exact_cache_hits: usize,
    nando_shadow_accepts: usize,
    verified_safe_accepts: usize,
    unverified_shadow_accepts: usize,
    false_accepts: usize,
    incremental_savings_over_exact_cache: usize,
    estimated_cost_saved_microusd: u128,
    latencies_ns: Vec<u128>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuRouteForecastReport {
    schema_version: String,
    verdict: String,
    route_report_path: String,
    shadow_report_path: String,
    traffic_source: String,
    total_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_coverage_milli: usize,
    operator_candidate_calls: usize,
    operator_candidate_coverage_milli: usize,
    no_candidate_calls: usize,
    current_nando_accepts: usize,
    current_verified_safe_accepts: usize,
    current_false_accepts: usize,
    current_incremental_savings_over_exact_cache: usize,
    current_incremental_reduction_vs_exact_cache_milli: usize,
    full_shadow_request_payload_built: bool,
    market_claim_allowed: bool,
    forecast_25_percent_additional_savings: usize,
    forecast_50_percent_additional_savings: usize,
    forecast_80_percent_additional_savings: usize,
    forecast_25_percent_total_calls_removed: usize,
    forecast_50_percent_total_calls_removed: usize,
    forecast_80_percent_total_calls_removed: usize,
    forecast_25_percent_total_reduction_milli: usize,
    forecast_50_percent_total_reduction_milli: usize,
    forecast_80_percent_total_reduction_milli: usize,
    routes: Vec<RoleBindingCpuRouteForecastRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuRouteForecastRow {
    priority_rank: usize,
    route_key: String,
    profile_id: String,
    candidate_events: usize,
    candidate_share_milli_of_all_llm_calls: usize,
    candidate_share_milli_of_candidate_zone: usize,
    exact_cache_hits_inside_route: usize,
    non_exact_candidate_calls: usize,
    current_accepts: usize,
    current_verified_safe_accepts: usize,
    current_false_accepts: usize,
    current_incremental_savings: usize,
    payload_builder_status: String,
    recommended_cpu_work: String,
    recommended_payload_builder: String,
    forecast_accept_25_percent_calls: usize,
    forecast_accept_50_percent_calls: usize,
    forecast_accept_80_percent_calls: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRouteGapCatalogReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    total_history_rows: usize,
    max_events: usize,
    sampled_llm_calls: usize,
    existing_route_candidate_events: usize,
    existing_route_coverage_milli: usize,
    no_candidate_events: usize,
    no_candidate_coverage_milli: usize,
    top_gap_family: Option<String>,
    top_three_no_candidate_events: usize,
    top_three_no_candidate_coverage_milli: usize,
    existing_routes: Vec<RoleBindingNamedCount>,
    no_candidate_families: Vec<RoleBindingRouteGapCatalogFamilyRow>,
    raw_text_written: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRouteGapCatalogFamilyRow {
    priority_rank: usize,
    family_key: String,
    candidate_events: usize,
    coverage_milli_of_all_llm_calls: usize,
    coverage_milli_of_no_candidate_zone: usize,
    cpu_operator_readiness: String,
    recommended_profile_line: String,
    recommended_payload_builder: String,
    recommended_verifier: String,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRouteGapPayloadReadinessReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    total_history_rows: usize,
    max_events: usize,
    sampled_llm_calls: usize,
    existing_route_candidate_events: usize,
    no_candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    top_payload_ready_family: Option<String>,
    families: Vec<RoleBindingRouteGapPayloadReadinessFamilyRow>,
    rows: Vec<RoleBindingRouteGapPayloadReadinessEventRow>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRouteGapPayloadReadinessFamilyRow {
    priority_rank: usize,
    family_key: String,
    candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    request_signal_events: usize,
    context_signal_events: usize,
    evidence_signal_events: usize,
    verifier_signal_events: usize,
    cpu_operator_readiness: String,
    recommended_profile_line: String,
    recommended_payload_builder: String,
    recommended_verifier: String,
    dominant_builder_kind: String,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingRouteGapPayloadReadinessEventRow {
    event_id: String,
    request_fingerprint: String,
    family_key: String,
    cpu_operator_readiness: String,
    has_request_signal: bool,
    has_context_signal: bool,
    has_evidence_signal: bool,
    has_verifier_signal: bool,
    payload_ready: bool,
    recommended_payload_builder: String,
    recommended_verifier: String,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepPayloadDryRunReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    trace_path: String,
    max_events: usize,
    total_history_rows: usize,
    trace_rows_written: usize,
    planning_next_step_candidate_events: usize,
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    builder_rejected_events: usize,
    readiness_rejected_events: usize,
    profile_registered: bool,
    shadow_score_ready: bool,
    active_fringe_centers_total: usize,
    slots_total: usize,
    positive_impulses_total: usize,
    negative_impulses_total: usize,
    builder_status_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingPlanningNextStepPayloadDryRunRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepPayloadDryRunRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    readiness_payload_ready: bool,
    payload_built: bool,
    scoreable: bool,
    profile_registered: bool,
    builder_status: String,
    active_fringe_centers: usize,
    slots: usize,
    positive_impulses: usize,
    negative_impulses: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepProfileReport {
    schema_version: String,
    verdict: String,
    base_registry_path: String,
    dry_run_trace_path: String,
    package_path: String,
    registry_path: String,
    profile_id: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    edge_count: usize,
    runtime_bytes_estimate: usize,
    threshold: i32,
    trace_rows_read: usize,
    scoreable_payload_events: usize,
    package_training_requests: usize,
    positive_updates: usize,
    negative_updates: usize,
    changed_edges: usize,
    positive_margin_rows: usize,
    strict_ordered_pass_rows: usize,
    unexpected_local_accepts_under_disabled_threshold: usize,
    median_energy_margin: i32,
    p10_energy_margin: i32,
    min_energy_margin: i32,
    median_min_slot_margin: i32,
    p10_min_slot_margin: i32,
    min_slot_margin: i32,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled_on_real_traffic: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug)]
struct PlanningNextStepPackageBuild {
    package_bytes: Vec<u8>,
    package_training_requests: usize,
    positive_updates: usize,
    negative_updates: usize,
    changed_edges: usize,
}

#[derive(Clone, Debug, Default)]
struct RouteGapPayloadFamilyAccumulator {
    candidate_events: usize,
    payload_ready_events: usize,
    request_signal_events: usize,
    context_signal_events: usize,
    evidence_signal_events: usize,
    verifier_signal_events: usize,
    builder_kind_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug)]
struct RouteGapPayloadReadiness {
    has_request_signal: bool,
    has_context_signal: bool,
    has_evidence_signal: bool,
    has_verifier_signal: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuOperatorCatalogReport {
    schema_version: String,
    verdict: String,
    feedback_report_path: String,
    route_gap_report_path: String,
    route_gap_payload_readiness_report_path: Option<String>,
    total_llm_calls: usize,
    exact_cache_hits: usize,
    existing_operator_candidate_calls: usize,
    no_candidate_calls: usize,
    route_gap_payload_ready_events: usize,
    current_verified_cpu_accepts: usize,
    target_verified_cpu_accepts: usize,
    verified_gap_to_80_calls: usize,
    top_gap_family: Option<String>,
    top_actionable_rows: Vec<RoleBindingCpuOperatorCatalogRow>,
    rows: Vec<RoleBindingCpuOperatorCatalogRow>,
    raw_text_written: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingCpuOperatorCatalogRow {
    priority_rank: usize,
    source_kind: String,
    route_or_family_key: String,
    profile_id: Option<String>,
    candidate_events: usize,
    payload_ready_events: usize,
    scoreable_payload_events: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    false_accepts: usize,
    cpu_operator_readiness: String,
    recommended_profile_line: String,
    recommended_payload_builder: String,
    recommended_verifier: String,
    next_action: String,
    priority_score: i64,
    market_claim_allowed: bool,
    claim_boundary: String,
}

#[derive(Clone, Copy, Debug)]
struct RouteGapFamilyMetadata {
    cpu_operator_readiness: &'static str,
    recommended_profile_line: &'static str,
    recommended_payload_builder: &'static str,
    recommended_verifier: &'static str,
    claim_boundary: &'static str,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlProfileReport {
    schema_version: String,
    verdict: String,
    base_registry_path: String,
    package_path: String,
    registry_path: String,
    profile_id: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    edge_count: usize,
    runtime_bytes_estimate: usize,
    threshold: i32,
    sample_margin: i32,
    sample_local_accept: bool,
    raw_text_written: bool,
    local_accepts_enabled_on_real_traffic: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlPayloadDryRunReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    trace_path: String,
    max_events: usize,
    total_history_rows: usize,
    trace_rows_written: usize,
    agent_control_candidate_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    active_fringe_centers_total: usize,
    slots_total: usize,
    positive_impulses_total: usize,
    negative_impulses_total: usize,
    intent_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadReadinessReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    max_events: usize,
    total_history_rows: usize,
    candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    missing_scope_or_file: usize,
    missing_marker: usize,
    missing_length_or_shape: usize,
    missing_edit_intent: usize,
    route_counts: Vec<RoleBindingNamedCount>,
    builder_kind_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingEditPayloadReadinessRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadReadinessRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    has_edit_intent: bool,
    has_scope_or_file: bool,
    has_marker: bool,
    has_length_or_shape: bool,
    has_code_or_patch_signal: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadDryRunReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    trace_path: String,
    max_events: usize,
    total_history_rows: usize,
    trace_rows_written: usize,
    edit_route_candidate_events: usize,
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    builder_rejected_events: usize,
    readiness_rejected_events: usize,
    active_fringe_centers_total: usize,
    slots_total: usize,
    positive_impulses_total: usize,
    negative_impulses_total: usize,
    builder_status_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingEditPayloadDryRunRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditPayloadDryRunRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    readiness_payload_ready: bool,
    payload_built: bool,
    scoreable: bool,
    builder_status: String,
    active_fringe_centers: usize,
    slots: usize,
    positive_impulses: usize,
    negative_impulses: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingConditionalPayloadReadinessReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    max_events: usize,
    total_history_rows: usize,
    candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    missing_condition_signal: usize,
    missing_branch_signal: usize,
    missing_evidence_signal: usize,
    missing_branch_tokens: usize,
    route_counts: Vec<RoleBindingNamedCount>,
    builder_kind_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingConditionalPayloadReadinessRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingConditionalPayloadReadinessRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    has_condition_signal: bool,
    has_branch_signal: bool,
    has_evidence_signal: bool,
    has_branch_tokens: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingConditionalPayloadDryRunReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    trace_path: String,
    max_events: usize,
    total_history_rows: usize,
    trace_rows_written: usize,
    conditional_route_candidate_events: usize,
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    builder_rejected_events: usize,
    readiness_rejected_events: usize,
    active_fringe_centers_total: usize,
    slots_total: usize,
    positive_impulses_total: usize,
    negative_impulses_total: usize,
    builder_status_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingConditionalPayloadDryRunRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingConditionalPayloadDryRunRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    readiness_payload_ready: bool,
    payload_built: bool,
    scoreable: bool,
    builder_status: String,
    active_fringe_centers: usize,
    slots: usize,
    positive_impulses: usize,
    negative_impulses: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingMixedPayloadReadinessReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    max_events: usize,
    total_history_rows: usize,
    candidate_events: usize,
    payload_ready_events: usize,
    payload_ready_rate_milli: usize,
    missing_action_signal: usize,
    missing_source_signal: usize,
    missing_destination_signal: usize,
    missing_mapping_signal: usize,
    missing_map_tokens: usize,
    route_counts: Vec<RoleBindingNamedCount>,
    builder_kind_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingMixedPayloadReadinessRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingMixedPayloadReadinessRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    has_action_signal: bool,
    has_source_signal: bool,
    has_destination_signal: bool,
    has_mapping_signal: bool,
    has_map_tokens: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingMixedPayloadDryRunReport {
    schema_version: String,
    verdict: String,
    history_path: String,
    registry_config_path: String,
    trace_path: String,
    max_events: usize,
    total_history_rows: usize,
    trace_rows_written: usize,
    mixed_route_candidate_events: usize,
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    builder_rejected_events: usize,
    readiness_rejected_events: usize,
    active_fringe_centers_total: usize,
    slots_total: usize,
    positive_impulses_total: usize,
    negative_impulses_total: usize,
    builder_status_counts: Vec<RoleBindingNamedCount>,
    raw_text_written: bool,
    response_text_used: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    rows: Vec<RoleBindingMixedPayloadDryRunRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingMixedPayloadDryRunRow {
    event_id: String,
    request_fingerprint: String,
    route_key: String,
    profile_id: String,
    readiness_payload_ready: bool,
    payload_built: bool,
    scoreable: bool,
    builder_status: String,
    active_fringe_centers: usize,
    slots: usize,
    positive_impulses: usize,
    negative_impulses: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditOutputEvidenceReport {
    schema_version: String,
    verdict: String,
    input_trace_path: String,
    sessions_root: String,
    output_trace_path: String,
    total_trace_rows: usize,
    operator_candidate_calls: usize,
    scoreable_candidate_calls: usize,
    session_ids_requested: usize,
    session_files_scanned: usize,
    codex_turns_indexed: usize,
    output_evidence_matched_events: usize,
    no_session_output_match_events: usize,
    deterministic_verification_events: usize,
    verifier_not_applicable_events: usize,
    verified_true_events: usize,
    verified_false_events: usize,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_verification: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepArtifactProgressReport {
    schema_version: String,
    verdict: String,
    input_trace_path: String,
    sessions_root: String,
    output_trace_path: String,
    total_trace_rows: usize,
    operator_candidate_calls: usize,
    scoreable_candidate_calls: usize,
    session_ids_requested: usize,
    session_files_scanned: usize,
    codex_turns_indexed: usize,
    tool_events_indexed: usize,
    artifact_evidence_matched_events: usize,
    no_session_artifact_match_events: usize,
    verifier_not_applicable_events: usize,
    verified_true_events: usize,
    verified_false_events: usize,
    tool_call_fingerprint_events: usize,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    tool_outputs_written: bool,
    target_labels_used: bool,
    proof_labels_used: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditLocalAcceptCalibrationReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    trace_path: String,
    hook_ready_rows: usize,
    scored_rows: usize,
    label_true_rows: usize,
    label_false_rows: usize,
    no_score_rows: usize,
    safe_policy_found: bool,
    best_safe_true_accepts: usize,
    policies: Vec<RoleBindingEditLocalAcceptPolicyReport>,
    rows: Vec<RoleBindingEditLocalAcceptCalibrationRow>,
    #[serde(default)]
    margin_collision_diagnostics: Vec<RoleBindingLocalAcceptMarginCollisionReport>,
    #[serde(default)]
    request_side_margin_only_accepts_all_true_without_false: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditLocalAcceptPolicyReport {
    policy_name: String,
    accepts: usize,
    true_accepts: usize,
    false_accepts: usize,
    missed_true: usize,
    safe: bool,
    threshold: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingLocalAcceptMarginCollisionReport {
    margin_name: String,
    min_true_margin: Option<i32>,
    false_rows_at_or_above_min_true_margin: usize,
    safe_accepts_all_true_rows: bool,
    best_safe_true_accepts: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingMixedSafePolicyPromoteReport {
    schema_version: String,
    verdict: String,
    base_registry_config_path: String,
    evidence_trace_path: String,
    calibration_report_path: String,
    promoted_registry_config_path: String,
    promoted_trace_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    history_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_side_policy_name: Option<String>,
    calibration_policy_name: String,
    calibration_policy_threshold: Option<i32>,
    selected_policy_name: String,
    selected_policy_source: String,
    selected_policy_threshold: i32,
    selected_acceptance_policy: String,
    selected_policy_accepts: usize,
    selected_policy_true_accepts: usize,
    selected_policy_false_accepts: usize,
    selected_policy_unverified_accepts: usize,
    promoted_profile_ids: Vec<String>,
    provider_cost_microusd: u64,
    trace_rows_written: usize,
    scoreable_candidate_calls: usize,
    #[serde(default)]
    request_side_policy_evaluated_rows: usize,
    #[serde(default)]
    request_side_policy_accept_rows: usize,
    #[serde(default)]
    request_side_policy_reject_rows: usize,
    #[serde(default)]
    history_prompt_missing_rows: usize,
    policy_accept_rows: usize,
    policy_accept_verified_true_rows: usize,
    policy_accept_verified_false_rows: usize,
    policy_accept_unverified_rows: usize,
    provider_cost_events_written: usize,
    no_score_rows: usize,
    runtime_acceptance_mismatches: usize,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingConditionalSafePolicyPromoteReport {
    schema_version: String,
    verdict: String,
    base_registry_config_path: String,
    evidence_trace_path: String,
    calibration_report_path: String,
    promoted_registry_config_path: String,
    promoted_trace_path: String,
    history_path: String,
    calibration_verdict: String,
    request_side_policy_name: String,
    selected_policy_name: String,
    selected_policy_source: String,
    selected_policy_threshold: i32,
    selected_acceptance_policy: String,
    selected_policy_accepts: usize,
    selected_policy_true_accepts: usize,
    selected_policy_false_accepts: usize,
    selected_policy_unverified_accepts: usize,
    promoted_profile_ids: Vec<String>,
    provider_cost_microusd: u64,
    trace_rows_written: usize,
    conditional_candidate_rows: usize,
    scoreable_candidate_calls: usize,
    request_side_policy_evaluated_rows: usize,
    request_side_policy_accept_rows: usize,
    request_side_policy_reject_rows: usize,
    history_prompt_missing_rows: usize,
    runtime_policy_accept_rows: usize,
    runtime_policy_verified_true_rows: usize,
    runtime_policy_verified_false_rows: usize,
    runtime_policy_unverified_rows: usize,
    provider_cost_events_written: usize,
    no_score_rows: usize,
    runtime_acceptance_mismatches: usize,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_features: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    local_accepts_enabled_by_request_side_policy_and_score: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlSafePolicyPromoteReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    evidence_trace_path: String,
    calibration_report_path: String,
    promoted_trace_path: String,
    history_path: String,
    selected_policy_name: String,
    selected_policy_true_accepts: usize,
    selected_policy_false_accepts: usize,
    provider_cost_microusd: u64,
    trace_rows_written: usize,
    agent_control_candidate_rows: usize,
    scoreable_candidate_calls: usize,
    request_side_policy_evaluated_rows: usize,
    history_prompt_missing_rows: usize,
    policy_accept_rows: usize,
    policy_reject_rows: usize,
    policy_accept_verified_true_rows: usize,
    policy_accept_verified_false_rows: usize,
    policy_accept_unverified_rows: usize,
    provider_cost_events_written: usize,
    no_score_rows: usize,
    runtime_acceptance_mismatches: usize,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_features: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    profile_acceptance_policy_changed: bool,
    broad_agent_control_profile_promoted: bool,
    local_accepts_enabled_by_request_side_policy_only: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug)]
struct RoleBindingMixedPromotionPolicySelection {
    policy_name: String,
    selection_source: String,
    threshold: i32,
    accepts: usize,
    true_accepts: usize,
    false_accepts: usize,
    unverified_accepts: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditLocalAcceptCalibrationRow {
    trace_id: String,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    verifier_label: bool,
    production_accepted: bool,
    production_fallback_reason: Option<String>,
    energy_margin: i32,
    min_slot_margin: i32,
    marker_slot_margin: i32,
    end_slot_margin: i32,
    slot_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditAdmissionCalibrationReport {
    schema_version: String,
    verdict: String,
    evidence_trace_path: String,
    history_path: String,
    hook_ready_rows: usize,
    rows_with_prompt_features: usize,
    history_prompt_missing_rows: usize,
    label_true_rows: usize,
    label_false_rows: usize,
    minimum_true_support: usize,
    robust_safe_policy_found: bool,
    singleton_safe_policy_found: bool,
    best_robust_true_accepts: usize,
    best_singleton_true_accepts: usize,
    feature_counts: Vec<RoleBindingEditAdmissionFeatureCount>,
    policies: Vec<RoleBindingEditAdmissionPolicyReport>,
    rows: Vec<RoleBindingEditAdmissionCalibrationRow>,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_features: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditAdmissionFeatureCount {
    feature: String,
    label_true_count: usize,
    label_false_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditAdmissionPolicyReport {
    policy_name: String,
    accepts: usize,
    true_accepts: usize,
    false_accepts: usize,
    missed_true: usize,
    singleton_safe: bool,
    robust_safe: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditAdmissionCalibrationRow {
    trace_id: String,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    verifier_label: bool,
    features: RoleBindingEditAdmissionFeatures,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlAdmissionCalibrationReport {
    schema_version: String,
    verdict: String,
    evidence_trace_path: String,
    history_path: String,
    hook_ready_rows: usize,
    rows_with_prompt_features: usize,
    history_prompt_missing_rows: usize,
    label_true_rows: usize,
    label_false_rows: usize,
    minimum_true_support: usize,
    robust_safe_policy_found: bool,
    singleton_safe_policy_found: bool,
    best_robust_true_accepts: usize,
    best_singleton_true_accepts: usize,
    feature_counts: Vec<RoleBindingEditAdmissionFeatureCount>,
    policies: Vec<RoleBindingEditAdmissionPolicyReport>,
    rows: Vec<RoleBindingAgentControlAdmissionCalibrationRow>,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_features: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlAdmissionCalibrationRow {
    trace_id: String,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    verifier_label: bool,
    features: RoleBindingAgentControlAdmissionFeatures,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepAdmissionCalibrationReport {
    schema_version: String,
    verdict: String,
    evidence_trace_path: String,
    history_path: String,
    hook_ready_rows: usize,
    rows_with_prompt_features: usize,
    history_prompt_missing_rows: usize,
    label_true_rows: usize,
    label_false_rows: usize,
    minimum_true_support: usize,
    robust_safe_policy_found: bool,
    singleton_safe_policy_found: bool,
    best_robust_true_accepts: usize,
    best_singleton_true_accepts: usize,
    feature_counts: Vec<RoleBindingEditAdmissionFeatureCount>,
    policies: Vec<RoleBindingEditAdmissionPolicyReport>,
    rows: Vec<RoleBindingPlanningNextStepAdmissionCalibrationRow>,
    raw_prompt_text_written: bool,
    raw_response_text_written: bool,
    response_text_used_for_features: bool,
    target_labels_used_for_runtime: bool,
    proof_labels_used_for_runtime: bool,
    local_accepts_enabled: bool,
    market_claim_allowed: bool,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepAdmissionCalibrationRow {
    trace_id: String,
    request_fingerprint: Option<String>,
    response_fingerprint: Option<String>,
    verifier_label: bool,
    features: RoleBindingPlanningNextStepAdmissionFeatures,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingPlanningNextStepAdmissionFeatures {
    request_len: usize,
    line_count: usize,
    token_count: usize,
    starts_goal: bool,
    starts_continue_or_next: bool,
    starts_direct_action: bool,
    has_direct_action_words: bool,
    has_next_or_plan_words: bool,
    has_git_commit_terms: bool,
    has_patch_apply_terms: bool,
    has_project_artifact_terms: bool,
    has_nando_wave_terms: bool,
    has_goal_control_terms: bool,
    has_report_or_failure_terms: bool,
    has_question_mark: bool,
    has_code_diff_lines: bool,
    has_file_like_token: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingAgentControlAdmissionFeatures {
    request_len: usize,
    token_count: usize,
    intent_stop: bool,
    intent_continue: bool,
    intent_short_ack: bool,
    has_stop_word: bool,
    has_stoy_word: bool,
    has_ostanov_word: bool,
    has_pause_word: bool,
    one_token_lowercase_stop: bool,
    stop_uppercase_goal_control: bool,
    has_exclamation: bool,
    has_question_mark: bool,
    has_work_words: bool,
    has_goal_or_plan_words: bool,
    all_capsish: bool,
    tokens_le_1: bool,
    tokens_le_2: bool,
    tokens_le_3: bool,
    tokens_le_4: bool,
    chars_le_12: bool,
    chars_le_20: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingEditAdmissionFeatures {
    request_len: usize,
    line_count: usize,
    starts_goal: bool,
    starts_what: bool,
    starts_yes: bool,
    has_runtime_terms: bool,
    has_goal_terms: bool,
    has_next_terms: bool,
    has_direct_edit_command: bool,
    has_report_markers: bool,
    has_proof_boundary_terms: bool,
    has_code_diff_lines: bool,
    has_file_like_token: bool,
    has_question_mark: bool,
    marker_len: usize,
    marker_present: bool,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileDetailedScore {
    energy_margin: i32,
    min_slot_margin: i32,
    slot_margins: Vec<i32>,
}

#[derive(Clone, Debug)]
struct CodexSessionOutputEvidenceIndex {
    by_request_fingerprint: BTreeMap<String, CodexSessionOutputEvidence>,
    session_files_scanned: usize,
    codex_turns_indexed: usize,
}

#[derive(Clone, Debug)]
struct CodexSessionOutputEvidence {
    response_fingerprint: String,
    verified_safe_accept: bool,
    verifier_applicable: bool,
    verifier_status: String,
}

#[derive(Clone, Debug)]
struct CodexSessionPlanningArtifactProgressIndex {
    by_request_fingerprint: BTreeMap<String, CodexSessionPlanningArtifactProgressEvidence>,
    session_files_scanned: usize,
    codex_turns_indexed: usize,
    tool_events_indexed: usize,
}

#[derive(Clone, Debug)]
struct CodexSessionPlanningArtifactProgressEvidence {
    response_fingerprint: Option<String>,
    tool_call_fingerprints: Vec<String>,
    verified_safe_accept: bool,
    verifier_applicable: bool,
    verifier_status: String,
}

#[derive(Clone, Debug, Default)]
struct PlanningArtifactTurnEvidence {
    progress_tool_fingerprints: Vec<String>,
    successful_progress_kinds: BTreeSet<String>,
    validation_tool_fingerprints: Vec<String>,
    successful_validation_kinds: BTreeSet<String>,
    pending_tool_kinds: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingVerificationHookAuditReport {
    schema_version: String,
    verdict: String,
    trace_path: String,
    shadow_report_path: String,
    total_requests: usize,
    total_llm_calls: usize,
    operator_candidate_calls: usize,
    scoreable_candidate_calls: usize,
    local_accepts_disabled_events: usize,
    local_accepts_enabled_events: usize,
    response_fingerprint_events: usize,
    tool_call_fingerprint_events: usize,
    verification_source_events: usize,
    explicit_verified_safe_accept_events: usize,
    verified_true_events: usize,
    verified_false_events: usize,
    provider_cost_events: usize,
    candidates_missing_output_evidence: usize,
    candidates_missing_verification_source: usize,
    candidates_missing_explicit_verification: usize,
    candidates_missing_provider_cost: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    all_trace_rows_matched_shadow: bool,
    shadow_accepts: usize,
    shadow_fallbacks: usize,
    shadow_false_accepts: usize,
    shadow_incremental_savings_over_exact_cache: usize,
    market_claim_allowed: bool,
    routes: Vec<RoleBindingVerificationHookRouteRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Default)]
struct RoleBindingVerificationHookRouteAccumulator {
    route_key: String,
    profile_id: String,
    candidate_calls: usize,
    scoreable_candidate_calls: usize,
    local_accepts_disabled_events: usize,
    local_accepts_enabled_events: usize,
    candidates_missing_output_evidence: usize,
    candidates_missing_verification_source: usize,
    candidates_missing_explicit_verification: usize,
    candidates_missing_provider_cost: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    shadow_accepts: usize,
    shadow_fallbacks: usize,
    false_accepts: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingVerificationHookRouteRow {
    route_key: String,
    profile_id: String,
    candidate_calls: usize,
    scoreable_candidate_calls: usize,
    local_accepts_disabled_events: usize,
    local_accepts_enabled_events: usize,
    candidates_missing_output_evidence: usize,
    candidates_missing_verification_source: usize,
    candidates_missing_explicit_verification: usize,
    candidates_missing_provider_cost: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    shadow_accepts: usize,
    shadow_fallbacks: usize,
    false_accepts: usize,
}

impl From<RoleBindingVerificationHookRouteAccumulator> for RoleBindingVerificationHookRouteRow {
    fn from(value: RoleBindingVerificationHookRouteAccumulator) -> Self {
        Self {
            route_key: value.route_key,
            profile_id: value.profile_id,
            candidate_calls: value.candidate_calls,
            scoreable_candidate_calls: value.scoreable_candidate_calls,
            local_accepts_disabled_events: value.local_accepts_disabled_events,
            local_accepts_enabled_events: value.local_accepts_enabled_events,
            candidates_missing_output_evidence: value.candidates_missing_output_evidence,
            candidates_missing_verification_source: value.candidates_missing_verification_source,
            candidates_missing_explicit_verification: value
                .candidates_missing_explicit_verification,
            candidates_missing_provider_cost: value.candidates_missing_provider_cost,
            verification_hook_ready_events: value.verification_hook_ready_events,
            verified_cpu_accept_eligible_events: value.verified_cpu_accept_eligible_events,
            shadow_accepts: value.shadow_accepts,
            shadow_fallbacks: value.shadow_fallbacks,
            false_accepts: value.false_accepts,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingFeedbackLoopReport {
    schema_version: String,
    verdict: String,
    forecast_report_path: String,
    edit_dry_run_report_path: String,
    edit_local_accept_calibration_report_path: Option<String>,
    edit_safe_policy_verification_audit_report_path: Option<String>,
    planning_next_step_dry_run_report_path: Option<String>,
    planning_next_step_local_accept_calibration_report_path: Option<String>,
    planning_next_step_verification_audit_report_path: Option<String>,
    agent_control_dry_run_report_path: Option<String>,
    agent_control_verification_audit_report_path: Option<String>,
    agent_control_safe_policy_verification_audit_report_path: Option<String>,
    agent_control_admission_calibration_report_path: Option<String>,
    conditional_dry_run_report_path: Option<String>,
    conditional_local_accept_calibration_report_path: Option<String>,
    conditional_verification_audit_report_path: Option<String>,
    conditional_safe_policy_verification_audit_report_path: Option<String>,
    mixed_dry_run_report_path: Option<String>,
    mixed_local_accept_calibration_report_path: Option<String>,
    mixed_verification_audit_report_path: Option<String>,
    mixed_safe_policy_verification_audit_report_path: Option<String>,
    verification_audit_report_path: String,
    total_llm_calls: usize,
    exact_cache_hits: usize,
    exact_cache_coverage_milli: usize,
    operator_candidate_calls: usize,
    operator_candidate_coverage_milli: usize,
    scoreable_candidate_calls: usize,
    scoreable_candidate_coverage_milli: usize,
    verification_hook_ready_events: usize,
    verification_hook_coverage_milli: usize,
    verified_cpu_accept_eligible_events: usize,
    verified_cpu_routability_milli: usize,
    target_routability_milli: usize,
    target_verified_cpu_calls: usize,
    routing_gap_to_80_calls: usize,
    verified_gap_to_80_calls: usize,
    no_candidate_calls: usize,
    market_claim_allowed: bool,
    routes: Vec<RoleBindingFeedbackLoopRouteRow>,
    claim_boundary: String,
    next_engineering_debt: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingFeedbackLoopRouteRow {
    priority_rank: usize,
    route_key: String,
    profile_id: String,
    candidate_events: usize,
    non_exact_candidate_calls: usize,
    payload_builder: String,
    stage: String,
    next_action: String,
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    verification_hook_ready_events: usize,
    local_accept_calibration_ran: bool,
    local_accept_safe_policy_found: bool,
    local_accept_minimum_true_support: usize,
    local_accept_support_qualified: bool,
    local_accept_best_safe_true_accepts: usize,
    verified_cpu_accept_eligible_events: usize,
    false_accepts: usize,
    candidate_share_milli_of_all_llm_calls: usize,
    scoreable_share_milli_of_all_llm_calls: usize,
    verified_share_milli_of_all_llm_calls: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingNamedCount {
    name: String,
    count: usize,
}

#[derive(Clone, Debug)]
struct EditPayloadReadiness {
    has_edit_intent: bool,
    has_scope_or_file: bool,
    has_marker: bool,
    has_length_or_shape: bool,
    has_code_or_patch_signal: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct ConditionalPayloadReadiness {
    has_condition_signal: bool,
    has_branch_signal: bool,
    has_evidence_signal: bool,
    has_branch_tokens: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct MixedPayloadReadiness {
    has_action_signal: bool,
    has_source_signal: bool,
    has_destination_signal: bool,
    has_mapping_signal: bool,
    has_map_tokens: bool,
    payload_ready: bool,
    recommended_builder_kind: String,
    missing_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct ConditionalBranchTokens {
    condition_token: String,
    evidence_token: String,
    allowed_token: String,
    refused_token: String,
}

#[derive(Clone, Debug)]
struct MixedMapTokens {
    source_token: String,
    destination_token: String,
    action_token: String,
    invariant_token: String,
}

#[derive(Clone, Debug)]
struct CodexHistoryRouteCatalog {
    control: Option<CodexHistoryRouteCandidate>,
    edit: Option<CodexHistoryRouteCandidate>,
    conditional: Option<CodexHistoryRouteCandidate>,
    mixed: Option<CodexHistoryRouteCandidate>,
    planning: Option<CodexHistoryRouteCandidate>,
}

impl CodexHistoryRouteCatalog {
    fn from_registry(config: &RoleBindingProfileRegistryConfig) -> Result<Self, String> {
        let mut catalog = Self {
            control: None,
            edit: None,
            conditional: None,
            mixed: None,
            planning: None,
        };
        for profile in &config.profiles {
            let route_key = profile
                .accepted_route_keys
                .first()
                .cloned()
                .ok_or_else(|| format!("profile {} has no route key", profile.profile_id))?;
            let candidate = CodexHistoryRouteCandidate {
                route_key,
                profile_id: profile.profile_id.clone(),
            };
            if profile
                .operator_classes
                .iter()
                .any(|class| class == "agent_control" || class == "dialogue_state")
            {
                catalog.control.get_or_insert(candidate);
            } else if profile.operator_classes.iter().any(|class| class == "edit") {
                catalog.edit.get_or_insert(candidate);
            } else if profile
                .operator_classes
                .iter()
                .any(|class| class == "condition_route" || class == "verify_repair")
            {
                catalog.conditional.get_or_insert(candidate);
            } else if profile
                .operator_classes
                .iter()
                .any(|class| class == "move_copy" || class == "order" || class == "compose")
            {
                catalog.mixed.get_or_insert(candidate);
            } else if profile.operator_classes.iter().any(|class| {
                class == "project_planning" || class == "state_transition" || class == "route_gap"
            }) {
                catalog.planning.get_or_insert(candidate);
            }
        }
        Ok(catalog)
    }

    fn classify_request_text(&self, text: &str) -> Option<CodexHistoryRouteCandidate> {
        let lower = text.to_lowercase();
        if self.control.is_some()
            && (has_agent_control_stop_intent(&lower)
                || has_agent_control_continue_intent(&lower)
                || has_short_decision_ack_intent(&lower))
        {
            return self.control.clone();
        }
        let has_direct_edit_intent = contains_any(
            &lower,
            &[
                "исправ",
                "почини",
                "чини",
                "замени",
                "перепиши",
                "patch",
                "commit",
                "refactor",
                "рефактор",
            ],
        );
        let has_conditional_intent = contains_any(
            &lower,
            &[
                "если",
                "проверь",
                "провер",
                "gate",
                "verify",
                "услов",
                "false_accept",
                "fallback",
                "margin",
                "review",
                "pass",
            ],
        );
        let has_code_surface = contains_any(
            &lower,
            &[
                "код", "файл", "diff", "clippy", "cargo", "git", "runtime", "ошиб", "тест",
            ],
        );
        let has_planning_next_step_intent =
            route_gap_family_key(text) == REAL_TRAFFIC_PLANNING_ROUTE_KEY;

        if has_direct_edit_intent {
            return self.edit.clone().or_else(|| self.mixed.clone());
        }
        if has_planning_next_step_intent {
            return self.planning.clone().or_else(|| self.mixed.clone());
        }
        if has_conditional_intent {
            return self.conditional.clone().or_else(|| self.mixed.clone());
        }
        if has_code_surface {
            return self.edit.clone().or_else(|| self.mixed.clone());
        }
        if contains_any(
            &lower,
            &[
                "сделай",
                "добавь",
                "запиши",
                "обнови",
                "перенеси",
                "route",
                "trace",
                "оператор",
                "рантайм",
                "runtime",
                "shadow",
                "traffic",
                "трафик",
            ],
        ) {
            return self.mixed.clone().or_else(|| self.edit.clone());
        }
        None
    }
}

#[derive(Clone, Debug)]
struct CodexHistoryRouteCandidate {
    route_key: String,
    profile_id: String,
}

fn has_agent_control_stop_intent(lower: &str) -> bool {
    contains_any(
        lower,
        &["стоп", "стой", "останов", "не делай", "не трогай", "пауза"],
    )
}

fn has_agent_control_continue_intent(lower: &str) -> bool {
    normalized_token_count(lower) <= 6
        && contains_any(
            lower,
            &[
                "делай",
                "выполни",
                "выполняй",
                "продолжай",
                "поехали",
                "идем дальше",
                "идём дальше",
                "go ahead",
            ],
        )
        && !contains_any(
            lower,
            &[
                "код",
                "файл",
                "commit",
                "коммит",
                "patch",
                "diff",
                ".rs",
                ".py",
                ".md",
                ".json",
            ],
        )
}

fn has_short_decision_ack_intent(lower: &str) -> bool {
    normalized_token_count(lower) <= 3
        && contains_any(
            lower,
            &[
                "да",
                "нет",
                "ок",
                "ага",
                "понял",
                "хорошо",
                "ладно",
                "можно",
            ],
        )
}

fn normalized_token_count(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| {
            !token
                .trim_matches(|ch: char| !ch.is_alphanumeric())
                .is_empty()
        })
        .count()
}

fn route_gap_family_key(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if has_agent_control_stop_intent(&lower) {
        return "agent_control_stop";
    }
    if has_agent_control_continue_intent(&lower) {
        return "agent_continue_execute";
    }
    if contains_any(
        &lower,
        &[
            "найди",
            "поищи",
            "ссылк",
            "pdf",
            "документ",
            "документы",
            "где лежит",
            "где найти",
        ],
    ) {
        return "retrieval_lookup";
    }
    if contains_any(
        &lower,
        &["коммит", "пуш", "push", "ветк", "branch", "status"],
    ) {
        return "git_control";
    }
    if contains_any(
        &lower,
        &[
            "p99",
            "latency",
            "метрик",
            "report",
            "отчет",
            "отчёт",
            "accuracy",
            "false_accept",
            "эконом",
            "savings",
            "coverage",
            "milli",
        ],
    ) {
        return "metrics_report_readout";
    }
    if contains_any(
        &lower,
        &[
            "читай",
            "посмотри",
            "ознакомь",
            "прочитай",
            "inspect",
            "read",
        ],
    ) {
        return "read_inspect";
    }
    if contains_any(
        &lower,
        &[
            "сервер",
            "daemon",
            "демон",
            "worker",
            "lb",
            "http",
            "hostworld",
            "vps",
            "nginx",
            "systemd",
        ],
    ) {
        return "serving_ops";
    }
    if contains_any(
        &lower,
        &[
            "датасет",
            "корпус",
            "задач",
            "jsonl",
            "batch",
            "negative",
            "near_negative",
        ],
    ) {
        return "dataset_corpus";
    }
    if contains_any(
        &lower,
        &[
            "литератур",
            "стать",
            "paper",
            "arxiv",
            "фурье",
            "fourier",
            "hopfield",
            "kuramoto",
            "hrr",
            "vsa",
        ],
    ) {
        return "research_literature";
    }
    if contains_any(
        &lower,
        &[
            "дальше",
            "план",
            "следующий",
            "что делать",
            "roadmap",
            "цель",
            "goal",
        ],
    ) {
        return "planning_next_step";
    }
    if contains_any(
        &lower,
        &["коротко", "простын", "без воды", "не пиши", "кратко"],
    ) {
        return "style_brevity";
    }
    if contains_any(
        &lower,
        &[
            "что",
            "почему",
            "зачем",
            "как",
            "сколько",
            "где",
            "когда",
            "можно",
            "можешь",
            "какой",
            "какая",
            "какие",
            "объясни",
            "расскажи",
            "оцени",
            "вердикт",
        ],
    ) {
        return "answer_or_explain";
    }
    if contains_any(
        &lower,
        &["спасибо", "молодец", "горжусь", "люблю", "брат", "дорогой"],
    ) {
        return "social_affect";
    }
    let token_count = normalized_token_count(&lower);
    if has_short_decision_ack_intent(&lower) {
        return "short_decision_ack";
    }
    if token_count <= 12
        || contains_any(
            &lower,
            &[
                "проект",
                "модель",
                "архитект",
                "nando",
                "wave",
                "llmwave",
                "l1",
                "l2",
                "l3",
                "нам",
                "тут",
                "там",
                "сейчас",
            ],
        )
    {
        return "project_context_dialogue";
    }
    "uncatalogued"
}

fn route_gap_family_metadata(family_key: &str) -> RouteGapFamilyMetadata {
    match family_key {
        "agent_control_stop" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "high_control_policy_candidate",
            recommended_profile_line: "agent_control_operator",
            recommended_payload_builder: "agent_control_intent_payload_builder_v1",
            recommended_verifier: "no_mutating_tool_after_stop_verifier_v1",
            claim_boundary: "Can become CPU-safe only for control-plane decisions such as stop/pause/fallback; it must not answer task content.",
        },
        "agent_continue_execute" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_goal_state_transition_candidate",
            recommended_profile_line: "agent_continuation_operator",
            recommended_payload_builder: "active_goal_next_step_payload_builder_v1",
            recommended_verifier: "artifact_progress_and_no_drift_verifier_v1",
            claim_boundary: "Can continue only when an active goal and next runnable step are explicit in artifacts; otherwise fallback to LLM planning.",
        },
        "retrieval_lookup" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_evidence_retrieval_candidate",
            recommended_profile_line: "local_evidence_lookup_operator",
            recommended_payload_builder: "local_path_or_link_lookup_payload_builder_v1",
            recommended_verifier: "source_path_or_url_presence_verifier_v1",
            claim_boundary: "Can route local/source lookup and provenance checks; external freshness still requires source retrieval.",
        },
        "git_control" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_workspace_policy_candidate",
            recommended_profile_line: "workspace_command_operator",
            recommended_payload_builder: "git_command_intent_payload_builder_v1",
            recommended_verifier: "git_status_and_command_outcome_verifier_v1",
            claim_boundary: "Needs live workspace evidence and command outcome verification; no automatic mutation without explicit user intent.",
        },
        "metrics_report_readout" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_structured_readout_candidate",
            recommended_profile_line: "metrics_summary_operator",
            recommended_payload_builder: "metrics_report_payload_builder_v1",
            recommended_verifier: "numeric_report_field_verifier_v1",
            claim_boundary: "Can summarize existing reports only when metric fields are already present; cannot invent missing measurements.",
        },
        "read_inspect" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_read_only_tool_candidate",
            recommended_profile_line: "read_inspect_operator",
            recommended_payload_builder: "read_inspect_request_payload_builder_v1",
            recommended_verifier: "read_only_path_and_excerpt_verifier_v1",
            claim_boundary: "Can route read-only inspections, but final answers still need source-grounded evidence and path checks.",
        },
        "serving_ops" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_ops_diagnostic_candidate",
            recommended_profile_line: "serving_ops_operator",
            recommended_payload_builder: "serving_ops_metric_payload_builder_v1",
            recommended_verifier: "service_health_metric_verifier_v1",
            claim_boundary: "Needs daemon/LB/worker evidence; no server mutation claim without live health and rollback checks.",
        },
        "dataset_corpus" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_data_quality_candidate",
            recommended_profile_line: "dataset_quality_operator",
            recommended_payload_builder: "dataset_contract_payload_builder_v1",
            recommended_verifier: "jsonl_schema_balance_verifier_v1",
            claim_boundary: "Can verify structural dataset gates; semantic accept still needs heldout and shortcut checks.",
        },
        "research_literature" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "low_requires_external_evidence",
            recommended_profile_line: "literature_triage_operator",
            recommended_payload_builder: "literature_question_payload_builder_v1",
            recommended_verifier: "citation_and_primary_source_verifier_v1",
            claim_boundary: "Requires current source retrieval before claims; CPU can triage route shape, not replace literature evidence.",
        },
        "planning_next_step" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_state_transition_candidate",
            recommended_profile_line: "project_planning_operator",
            recommended_payload_builder: "goal_state_transition_payload_builder_v1",
            recommended_verifier: "plan_step_artifact_progress_verifier_v1",
            claim_boundary: "Can rank next engineering step only against explicit project artifacts and current goal state.",
        },
        "style_brevity" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "high_response_policy_candidate",
            recommended_profile_line: "response_style_operator",
            recommended_payload_builder: "style_constraint_payload_builder_v1",
            recommended_verifier: "response_length_and_format_verifier_v1",
            claim_boundary: "Can enforce style constraints on generated responses; it is not a semantic task solver.",
        },
        "answer_or_explain" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "low_needs_knowledge_evidence",
            recommended_profile_line: "short_answer_evidence_operator",
            recommended_payload_builder: "question_shape_payload_builder_v1",
            recommended_verifier: "grounded_answer_evidence_verifier_v1",
            claim_boundary: "High-volume but not locally safe by default; requires grounded evidence or fallback to LLM.",
        },
        "social_affect" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "low_optional_policy_candidate",
            recommended_profile_line: "conversation_ack_operator",
            recommended_payload_builder: "affect_ack_payload_builder_v1",
            recommended_verifier: "no_task_claim_ack_verifier_v1",
            claim_boundary: "Can only acknowledge tone; it must not claim task progress or produce technical answers.",
        },
        "short_decision_ack" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "medium_dialogue_state_candidate",
            recommended_profile_line: "short_decision_operator",
            recommended_payload_builder: "short_ack_decision_payload_builder_v1",
            recommended_verifier: "active_turn_state_transition_verifier_v1",
            claim_boundary: "Can update dialogue control state only when the previous assistant turn defines the decision; it must not infer hidden task content.",
        },
        "project_context_dialogue" => RouteGapFamilyMetadata {
            cpu_operator_readiness: "low_stateful_dialogue_candidate",
            recommended_profile_line: "project_context_dialogue_operator",
            recommended_payload_builder: "active_project_state_payload_builder_v1",
            recommended_verifier: "workspace_artifact_or_goal_state_verifier_v1",
            claim_boundary: "Can only route against explicit active project state and artifacts; free-form reasoning still falls back to LLM.",
        },
        _ => RouteGapFamilyMetadata {
            cpu_operator_readiness: "unknown_requires_manual_review",
            recommended_profile_line: "uncatalogued_operator_backlog",
            recommended_payload_builder: "manual_route_discovery_payload_builder_v1",
            recommended_verifier: "manual_route_claim_boundary_verifier_v1",
            claim_boundary: "Uncatalogued prompts require manual route discovery before any CPU accept path exists.",
        },
    }
}

fn build_planning_next_step_role_binding_package_from_trace(
    trace_rows: &[RoleBindingRealTrafficTraceRow],
) -> Result<PlanningNextStepPackageBuild, String> {
    let config = WavePredictorHebbianConfig {
        state_delta_binding_action_base: Some(REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_BASE),
        state_delta_binding_action_count: REAL_TRAFFIC_PLANNING_PAGE_SIZE,
        state_delta_binding_role_base: Some(REAL_TRAFFIC_PLANNING_ROLE_BASE),
        state_delta_binding_role_stride: REAL_TRAFFIC_PLANNING_PAGE_SIZE,
        state_delta_binding_role_count: 4,
        state_delta_binding_slot_scoped_action_page_bits: 12,
        state_delta_binding_slot_scoped_action_page_mask: 1_u64
            << (REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_BASE >> 12),
        state_delta_binding_slot_scoped_action_source_bits:
            REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_SHIFT as u8,
        weight_limit: 2_048,
        ..WavePredictorHebbianConfig::default()
    };
    let role_end = REAL_TRAFFIC_PLANNING_ROLE_BASE
        + REAL_TRAFFIC_PLANNING_PAGE_SIZE * u32::from(config.state_delta_binding_role_count);
    let action_end = REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_BASE + REAL_TRAFFIC_PLANNING_PAGE_SIZE;
    let center_count = role_end.max(action_end) as usize;
    let mut field = WavePredictorHebbianField::new(center_count, config);
    let mut package_training_requests = 0usize;
    let mut positive_updates = 0usize;
    let mut negative_updates = 0usize;
    let mut changed_edges = 0usize;

    for request in planning_next_step_scoreable_requests(trace_rows) {
        let active_fringe = request
            .active_fringe
            .iter()
            .map(|active| WavePredictorActiveCenter {
                center_id: active.center_id,
                strength: active.strength,
            })
            .collect::<Vec<_>>();
        if active_fringe.is_empty() {
            continue;
        }
        package_training_requests += 1;
        for slot in &request.slots {
            for impulse in &slot.positive_impulses {
                let changed = field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &active_fringe,
                    slot.binding_output_slot,
                    16,
                );
                positive_updates += 1;
                changed_edges += changed;
            }
            for impulse in &slot.negative_impulses {
                let changed = field.adjust_state_delta_role_binding(
                    impulse.lane_id,
                    impulse.signed_strength,
                    &active_fringe,
                    slot.binding_output_slot,
                    -16,
                );
                negative_updates += 1;
                changed_edges += changed;
            }
        }
    }

    if package_training_requests == 0 {
        return Err(
            "planning-next-step package builder found no scoreable dry-run requests".to_owned(),
        );
    }
    if changed_edges == 0 {
        return Err("planning-next-step package builder produced no role-binding edges".to_owned());
    }
    let package_bytes = field
        .compile_flat_role_binding_table()
        .to_bytes()
        .map_err(|error| {
            format!("failed to serialize planning-next-step .nwrb package: {error:?}")
        })?;
    Ok(PlanningNextStepPackageBuild {
        package_bytes,
        package_training_requests,
        positive_updates,
        negative_updates,
        changed_edges,
    })
}

fn planning_next_step_scoreable_requests(
    trace_rows: &[RoleBindingRealTrafficTraceRow],
) -> Vec<RoleBindingProfileScoreRequest> {
    trace_rows
        .iter()
        .filter_map(|row| row.nando_shadow_request.clone())
        .filter(|request| {
            request.profile_id.as_deref() == Some(REAL_TRAFFIC_PLANNING_PROFILE_ID)
                && !request.active_fringe.is_empty()
                && !request.slots.is_empty()
        })
        .collect()
}

fn build_agent_control_role_binding_package() -> Result<Vec<u8>, String> {
    let config = WavePredictorHebbianConfig {
        state_delta_binding_action_base: Some(REAL_TRAFFIC_AGENT_CONTROL_ACTION_BASE),
        state_delta_binding_action_count: REAL_TRAFFIC_AGENT_CONTROL_ACTION_COUNT,
        state_delta_binding_role_base: Some(REAL_TRAFFIC_AGENT_CONTROL_ROLE_BASE),
        state_delta_binding_role_stride: REAL_TRAFFIC_AGENT_CONTROL_ROLE_STRIDE,
        state_delta_binding_role_count: REAL_TRAFFIC_AGENT_CONTROL_ROLE_COUNT,
        weight_limit: 1_024,
        ..WavePredictorHebbianConfig::default()
    };
    let center_count = (REAL_TRAFFIC_AGENT_CONTROL_ROLE_BASE
        + REAL_TRAFFIC_AGENT_CONTROL_ROLE_STRIDE * u32::from(REAL_TRAFFIC_AGENT_CONTROL_ROLE_COUNT))
        as usize;
    let mut field = WavePredictorHebbianField::new(center_count, config);
    let lane = agent_control_intent_lane();
    let active_fringe = agent_control_active_fringe(lane);
    let changed = field.adjust_state_delta_role_binding(
        lane,
        1,
        &active_fringe,
        Some(REAL_TRAFFIC_AGENT_CONTROL_OUTPUT_SLOT),
        16,
    );
    if changed == 0 {
        return Err("agent control package builder produced no role-binding edge".to_owned());
    }
    field
        .compile_flat_role_binding_table()
        .to_bytes()
        .map_err(|error| format!("failed to serialize agent control .nwrb package: {error:?}"))
}

fn agent_control_sample_decision(
    sdk: &WavePredictorRoleBindingOffloadRuntime,
) -> Result<(i32, bool), String> {
    let target_lane = agent_control_intent_lane();
    let wrong_lane = first_active_surface_lane("__agent_control_wrong__");
    let active_fringe = agent_control_active_fringe(target_lane);
    let task = WavePredictorRoleBindingEvalTask {
        target_lane_id: target_lane,
        target_signed_strength: 1,
        wrong_lane_id: wrong_lane,
        wrong_signed_strength: 1,
        active_fringe: &active_fringe,
        binding_output_slot: Some(REAL_TRAFFIC_AGENT_CONTROL_OUTPUT_SLOT),
        expect_local_operator: true,
    };
    let decision = sdk.decide_task(&task);
    Ok((
        decision.margin,
        decision.action == WavePredictorRoleBindingOffloadAction::LocalOperator,
    ))
}

fn build_agent_control_dry_run_request(
    event_id: &str,
    fingerprint: &u64,
    candidate: &CodexHistoryRouteCandidate,
) -> Option<RoleBindingProfileScoreRequest> {
    let target_lane = agent_control_intent_lane();
    let wrong_lane = first_active_surface_lane("__agent_control_wrong__");
    let active_fringe = agent_control_active_fringe(target_lane)
        .into_iter()
        .map(|active| RoleBindingProfileActiveCenterRow {
            center_id: active.center_id,
            strength: active.strength,
        })
        .collect::<Vec<_>>();
    let slot = RoleBindingProfileScoreSlotRow {
        binding_output_slot: Some(REAL_TRAFFIC_AGENT_CONTROL_OUTPUT_SLOT),
        positive_impulses: vec![RoleBindingProfileImpulseRow {
            lane_id: target_lane,
            signed_strength: 1,
        }],
        negative_impulses: vec![RoleBindingProfileImpulseRow {
            lane_id: wrong_lane,
            signed_strength: 1,
        }],
    };
    if active_fringe.is_empty()
        || slot.positive_impulses.is_empty()
        || slot.negative_impulses.is_empty()
    {
        return None;
    }
    Some(RoleBindingProfileScoreRequest {
        request_id: event_id.to_owned(),
        route_key: Some(candidate.route_key.clone()),
        profile_id: Some(candidate.profile_id.clone()),
        exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
        active_fringe,
        slots: vec![slot],
        // Dry-run pressure: the CPU path is expected to accept, but the trace
        // remains unverified until a deterministic control verifier attaches
        // output/tool evidence.
        expect_local_operator: Some(true),
    })
}

fn agent_control_intent_kind(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    if has_agent_control_stop_intent(&lower) {
        "stop"
    } else if has_agent_control_continue_intent(&lower) {
        "continue"
    } else if has_short_decision_ack_intent(&lower) {
        "short_ack"
    } else {
        "unknown_control"
    }
}

fn agent_control_intent_lane() -> u16 {
    first_active_surface_lane("__agent_control_intent__")
}

fn first_active_surface_lane(input: &str) -> u16 {
    let wave = SurfaceWave4096::compile(input);
    wave.lanes()
        .iter()
        .enumerate()
        .filter_map(|(lane, value)| {
            let magnitude = value.abs();
            (magnitude > 0).then_some((magnitude, lane as u16))
        })
        .max_by(|left, right| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
        .map(|(_, lane)| lane)
        .unwrap_or(0)
}

fn agent_control_active_fringe(lane: u16) -> Vec<WavePredictorActiveCenter> {
    vec![
        WavePredictorActiveCenter {
            center_id: REAL_TRAFFIC_AGENT_CONTROL_ACTION_CENTER,
            strength: 8,
        },
        WavePredictorActiveCenter {
            center_id: REAL_TRAFFIC_AGENT_CONTROL_ROLE_BASE
                + u32::from(REAL_TRAFFIC_AGENT_CONTROL_INTENT_SLOT)
                    * REAL_TRAFFIC_AGENT_CONTROL_ROLE_STRIDE
                + u32::from(lane),
            strength: 8,
        },
    ]
}

fn build_planning_next_step_dry_run_request(
    event_id: &str,
    fingerprint: &u64,
    text: &str,
) -> Option<RoleBindingProfileScoreRequest> {
    let tokens = extract_planning_next_step_tokens(text)?;
    let mut active_fringe = Vec::new();
    active_fringe.extend(planning_request_operator_centers());
    active_fringe.extend(planning_role_surface_centers(
        REAL_TRAFFIC_PLANNING_GOAL_ROLE_SLOT,
        &tokens.goal_token,
    ));
    active_fringe.extend(planning_role_surface_centers(
        REAL_TRAFFIC_PLANNING_STATE_ROLE_SLOT,
        &tokens.state_token,
    ));
    active_fringe.extend(planning_role_surface_centers(
        REAL_TRAFFIC_PLANNING_EVIDENCE_ROLE_SLOT,
        &tokens.evidence_token,
    ));
    active_fringe.extend(planning_role_surface_centers(
        REAL_TRAFFIC_PLANNING_NEXT_ACTION_ROLE_SLOT,
        &tokens.next_action_token,
    ));
    let active_fringe = merge_profile_active_centers(active_fringe);

    let mut slots = Vec::new();
    if let Some(slot) = planning_request_score_slot(
        0,
        &tokens.next_action_token,
        REAL_TRAFFIC_PLANNING_WRONG_TOKEN,
    ) {
        slots.push(slot);
    }
    if let Some(slot) =
        planning_request_score_slot(1, &tokens.evidence_token, REAL_TRAFFIC_PLANNING_WRONG_TOKEN)
    {
        slots.push(slot);
    }
    if active_fringe.is_empty() || slots.is_empty() {
        return None;
    }

    Some(RoleBindingProfileScoreRequest {
        request_id: event_id.to_owned(),
        route_key: Some(REAL_TRAFFIC_PLANNING_ROUTE_KEY.to_owned()),
        profile_id: Some(REAL_TRAFFIC_PLANNING_PROFILE_ID.to_owned()),
        exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
        active_fringe,
        slots,
        // Dry-run only: planning profile and deterministic verifier are not
        // registered yet, so local accept must remain disabled.
        expect_local_operator: Some(false),
    })
}

fn planning_request_operator_centers() -> Vec<RoleBindingProfileActiveCenterRow> {
    [
        (0, REAL_TRAFFIC_PLANNING_NEXT_ACTION_ROLE_SLOT),
        (1, REAL_TRAFFIC_PLANNING_EVIDENCE_ROLE_SLOT),
        (0, REAL_TRAFFIC_PLANNING_GOAL_ROLE_SLOT),
        (1, REAL_TRAFFIC_PLANNING_STATE_ROLE_SLOT),
    ]
    .into_iter()
    .map(
        |(output_slot, role_slot)| RoleBindingProfileActiveCenterRow {
            center_id: REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_BASE
                + planning_request_operator_pair_lane(output_slot, role_slot),
            strength: 8,
        },
    )
    .collect()
}

fn planning_request_operator_pair_lane(output_slot: u8, role_slot: u8) -> u32 {
    (u32::from(output_slot) << REAL_TRAFFIC_PLANNING_OPERATOR_PAIR_SHIFT) | u32::from(role_slot)
}

fn planning_role_surface_centers(
    role_slot: u8,
    token: &str,
) -> Vec<RoleBindingProfileActiveCenterRow> {
    let slot_base = REAL_TRAFFIC_PLANNING_ROLE_BASE
        + u32::from(role_slot).saturating_mul(REAL_TRAFFIC_PLANNING_PAGE_SIZE);
    surface_lane_centers_folded_for_profile(
        token,
        slot_base,
        REAL_TRAFFIC_PLANNING_PAGE_SIZE,
        REAL_TRAFFIC_PLANNING_TOP_ROLE_L1_LANES,
    )
}

fn planning_request_score_slot(
    binding_output_slot: u8,
    correct_token: &str,
    wrong_token: &str,
) -> Option<RoleBindingProfileScoreSlotRow> {
    if correct_token == wrong_token {
        return None;
    }
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        REAL_TRAFFIC_PLANNING_STATE_DELTA_LANES_PER_SIDE,
    );
    let negative_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        REAL_TRAFFIC_PLANNING_STATE_DELTA_LANES_PER_SIDE,
    );
    if positive_impulses.is_empty() || negative_impulses.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreSlotRow {
        binding_output_slot: Some(binding_output_slot),
        positive_impulses,
        negative_impulses,
    })
}

fn extract_planning_next_step_tokens(text: &str) -> Option<PlanningNextStepTokens> {
    let tokens = extract_request_side_edit_tokens(text, 24);
    if tokens.len() < 2 {
        return None;
    }
    let lower = text.to_lowercase();
    let goal_token = first_token_matching_any(
        &tokens,
        &[
            "goal",
            "цель",
            "roadmap",
            "routability",
            "operator",
            "runtime",
            "nando",
            "wave",
            "llmwave",
        ],
    )
    .or_else(|| tokens.first().cloned())?;
    let state_token = first_token_matching_any(
        &tokens,
        &[
            "report",
            "отчет",
            "отчёт",
            "trace",
            "artifact",
            "commit",
            "git",
            "status",
            "target/",
            "docs/",
            "crates/",
        ],
    )
    .or_else(|| {
        tokens
            .iter()
            .find(|token| token.as_str() != goal_token)
            .cloned()
    })?;
    let evidence_token = first_token_matching_any(
        &tokens,
        &[
            "gate",
            "audit",
            "report",
            "metrics",
            "p99",
            "false_accept",
            "latency",
            "результат",
            "провер",
            "коммит",
        ],
    )
    .unwrap_or_else(|| state_token.clone());
    let next_action_token = first_matching_branch_token(
        &lower,
        &[
            "дальше",
            "следующий",
            "план",
            "делай",
            "build",
            "builder",
            "payload",
            "verifier",
            "shadow",
            "audit",
            "commit",
            "route",
        ],
    )
    .or_else(|| {
        tokens
            .iter()
            .find(|token| {
                let token_lower = token.to_lowercase();
                contains_any(
                    &token_lower,
                    &[
                        "next",
                        "plan",
                        "route",
                        "payload",
                        "builder",
                        "verifier",
                        "audit",
                        "shadow",
                        "commit",
                        "план",
                        "дальше",
                        "след",
                    ],
                )
            })
            .cloned()
    })
    .or_else(|| {
        tokens
            .iter()
            .find(|token| token.as_str() != goal_token && token.as_str() != state_token)
            .cloned()
    })?;
    Some(PlanningNextStepTokens {
        goal_token,
        state_token,
        evidence_token,
        next_action_token,
    })
}

fn first_token_matching_any(tokens: &[String], needles: &[&str]) -> Option<String> {
    tokens
        .iter()
        .find(|token| {
            let lower = token.to_lowercase();
            contains_any(&lower, needles)
        })
        .cloned()
}

#[derive(Clone, Debug)]
struct PlanningNextStepTokens {
    goal_token: String,
    state_token: String,
    evidence_token: String,
    next_action_token: String,
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplaySuiteReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded_in_serving_worker: bool,
    corpus_jsonl_loaded_in_serving_worker: bool,
    eval_packs_used_by_replay_client: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    rows: Vec<RoleBindingProfileReplaySuiteRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileFallbackSmokeReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    profile_count: usize,
    local_accept_pass: bool,
    bad_route_fallback_pass: bool,
    low_margin_fallback_pass: bool,
    local_action: String,
    bad_route_fallback_reason: Option<String>,
    low_margin_fallback_reason: Option<String>,
    local_energy_margin: i32,
    low_margin_energy_margin: i32,
    low_margin_threshold: i32,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerScalingReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    worker_count: usize,
    total_profile_count: usize,
    total_local_operator_calls: usize,
    total_fallback_to_llm_calls: usize,
    wrong_worker_route_fallbacks: usize,
    false_local_accepts: usize,
    max_worker_runtime_bytes_estimate: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    all_profile_score_pass: bool,
    all_wrong_worker_routes_fallback: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    rows: Vec<RoleBindingProfileWorkerScalingRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerScalingRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    runtime_bytes_estimate: usize,
    package_bytes: usize,
    edge_count: usize,
    local_accepts: usize,
    wrong_worker_route_fallbacks: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerReplayReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    worker_count: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    total_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    rows: Vec<RoleBindingProfileWorkerReplayRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileWorkerReplayRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerReplayReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    lb_config_path: String,
    lb_bind_addr: String,
    worker_count: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    http_replay_batches: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    load_balancer_p50_latency_ns: u128,
    load_balancer_p90_latency_ns: u128,
    load_balancer_p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    replay_client_wall_p50_latency_ns: u128,
    replay_client_wall_p90_latency_ns: u128,
    replay_client_wall_p99_latency_ns: u128,
    estimated_lb_overhead_p99_ns: u128,
    packed_score_parity_checks: usize,
    packed_score_parity_mismatches: usize,
    load_balancer_rss_bytes: usize,
    total_worker_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_worker_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    load_balancer_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    workers: Vec<RoleBindingProfileLoadBalancerReplayWorkerRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerThroughputReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    binary_suite_report_path: String,
    lb_config_path: String,
    lb_bind_addr: String,
    worker_count: usize,
    client_threads: usize,
    sequence_repetitions: usize,
    total_profile_count: usize,
    unique_sequences_replayed: usize,
    score_requests: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    unexpected_local_accepts: usize,
    client_errors: usize,
    total_wall_latency_ns: u128,
    throughput_requests_per_second_milli: u128,
    client_p50_latency_ns: u128,
    client_p90_latency_ns: u128,
    client_p99_latency_ns: u128,
    load_balancer_p50_latency_ns: u128,
    load_balancer_p90_latency_ns: u128,
    load_balancer_p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    estimated_lb_overhead_p99_ns: u128,
    packed_score_parity_checks: usize,
    packed_score_parity_mismatches: usize,
    load_balancer_rss_bytes: usize,
    total_worker_runtime_bytes_estimate: usize,
    max_worker_runtime_bytes_estimate: usize,
    total_worker_rss_bytes: usize,
    max_worker_rss_bytes: usize,
    max_worker_p99_latency_ns: u128,
    all_workers_serving_only: bool,
    load_balancer_serving_only: bool,
    eval_packs_used_by_replay_client: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    clients: Vec<RoleBindingProfileThroughputClientRow>,
    workers: Vec<RoleBindingProfileLoadBalancerReplayWorkerRow>,
    claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileThroughputClientRow {
    client_id: usize,
    score_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    unexpected_local_accepts: usize,
    errors: usize,
    first_error: Option<String>,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerReplayWorkerRow {
    worker_id: usize,
    shard_config_path: String,
    bind_addr: String,
    profile_count: usize,
    profile_ids: Vec<String>,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileReplaySuiteRow {
    profile_id: String,
    label: String,
    seed: u8,
    binary_eval_pack_path: String,
    unique_sequences_replayed: usize,
    no_cache_llm_calls: usize,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    profile_count: usize,
    runtime_bytes_estimate: usize,
    package_bytes: usize,
    edge_count: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerHealthResponse {
    schema_version: String,
    status: String,
    runtime: String,
    upstream_count: usize,
    profile_count: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileLoadBalancerHealthResponse {
    fn from_runtime(load_balancer: &RoleBindingProfileLoadBalancerRuntime) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_lb_health_v1".to_owned(),
            status: "ok".to_owned(),
            runtime: "nando-wave-profile-load-balancer".to_owned(),
            upstream_count: load_balancer.upstream_count(),
            profile_count: load_balancer.profile_count(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

impl RoleBindingProfileHealthResponse {
    fn from_registry(registry: &RoleBindingProfileRuntimeRegistry) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_health_v1".to_owned(),
            status: "ok".to_owned(),
            runtime: "nando-wave-profile-runtime".to_owned(),
            profile_count: registry.profile_count(),
            runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
            package_bytes: registry.total_package_bytes(),
            edge_count: registry.total_edge_count(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileProfilesResponse {
    schema_version: String,
    profile_count: usize,
    profiles: Vec<RoleBindingProfileSummary>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerProfilesResponse {
    schema_version: String,
    upstream_count: usize,
    profile_count: usize,
    profiles: Vec<RoleBindingProfileLoadBalancerProfileRow>,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileLoadBalancerProfileRow {
    profile_id: String,
    worker_id: usize,
    upstream_addr: String,
    shard_config_path: String,
}

impl RoleBindingProfileLoadBalancerProfilesResponse {
    fn from_runtime(load_balancer: &RoleBindingProfileLoadBalancerRuntime) -> Self {
        let profiles = load_balancer.profile_rows();
        Self {
            schema_version: "nando_role_binding_profile_lb_profiles_v1".to_owned(),
            upstream_count: load_balancer.upstream_count(),
            profile_count: profiles.len(),
            profiles,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

impl RoleBindingProfileProfilesResponse {
    fn from_registry(registry: &RoleBindingProfileRuntimeRegistry) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_profiles_v1".to_owned(),
            profile_count: registry.profile_count(),
            profiles: registry.profile_summaries(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileSummary {
    profile_id: String,
    profile_kind: String,
    operator_classes: Vec<String>,
    package_path: String,
    package_fingerprint64: u64,
    package_bytes: usize,
    runtime_bytes_estimate: usize,
    edge_count: usize,
    slot_count: u8,
    threshold: i32,
    acceptance_policy: String,
    accepted_route_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileMetricsResponse {
    schema_version: String,
    profile_count: usize,
    requests_handled: usize,
    score_requests: usize,
    replay_requests: usize,
    health_requests: usize,
    profiles_requests: usize,
    metrics_requests: usize,
    bad_requests: usize,
    local_operator_calls: usize,
    fallback_to_llm_calls: usize,
    false_local_accepts: usize,
    missed_expected_local: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    core_score_p50_latency_ns: u128,
    core_score_p90_latency_ns: u128,
    core_score_p99_latency_ns: u128,
    worker_score_p50_latency_ns: u128,
    worker_score_p90_latency_ns: u128,
    worker_score_p99_latency_ns: u128,
    lb_upstream_roundtrip_p50_latency_ns: u128,
    lb_upstream_roundtrip_p90_latency_ns: u128,
    lb_upstream_roundtrip_p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileMetricsResponse {
    fn from_registry(
        registry: &RoleBindingProfileRuntimeRegistry,
        stats: &RoleBindingProfileServeStats,
    ) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_metrics_v1".to_owned(),
            profile_count: registry.profile_count(),
            requests_handled: stats.requests_handled,
            score_requests: stats.score_requests,
            replay_requests: stats.replay_requests,
            health_requests: stats.health_requests,
            profiles_requests: stats.profiles_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            local_operator_calls: stats.local_operator_calls,
            fallback_to_llm_calls: stats.fallback_to_llm_calls,
            false_local_accepts: stats.false_local_accepts,
            missed_expected_local: stats.missed_expected_local,
            p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
            p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
            p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
            core_score_p50_latency_ns: percentile(&stats.core_score_latencies_ns, 50),
            core_score_p90_latency_ns: percentile(&stats.core_score_latencies_ns, 90),
            core_score_p99_latency_ns: percentile(&stats.core_score_latencies_ns, 99),
            worker_score_p50_latency_ns: percentile(&stats.worker_score_latencies_ns, 50),
            worker_score_p90_latency_ns: percentile(&stats.worker_score_latencies_ns, 90),
            worker_score_p99_latency_ns: percentile(&stats.worker_score_latencies_ns, 99),
            lb_upstream_roundtrip_p50_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                50,
            ),
            lb_upstream_roundtrip_p90_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                90,
            ),
            lb_upstream_roundtrip_p99_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                99,
            ),
            rss_bytes: current_rss_bytes(),
            runtime_bytes_estimate: registry.total_runtime_bytes_estimate(),
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }

    fn from_load_balancer(
        load_balancer: &RoleBindingProfileLoadBalancerRuntime,
        stats: &RoleBindingProfileServeStats,
    ) -> Self {
        Self {
            schema_version: "nando_role_binding_profile_metrics_v1".to_owned(),
            profile_count: load_balancer.profile_count(),
            requests_handled: stats.requests_handled,
            score_requests: stats.score_requests,
            replay_requests: stats.replay_requests,
            health_requests: stats.health_requests,
            profiles_requests: stats.profiles_requests,
            metrics_requests: stats.metrics_requests,
            bad_requests: stats.bad_requests,
            local_operator_calls: stats.local_operator_calls,
            fallback_to_llm_calls: stats.fallback_to_llm_calls,
            false_local_accepts: stats.false_local_accepts,
            missed_expected_local: stats.missed_expected_local,
            p50_latency_ns: percentile(&stats.score_latencies_ns, 50),
            p90_latency_ns: percentile(&stats.score_latencies_ns, 90),
            p99_latency_ns: percentile(&stats.score_latencies_ns, 99),
            core_score_p50_latency_ns: percentile(&stats.core_score_latencies_ns, 50),
            core_score_p90_latency_ns: percentile(&stats.core_score_latencies_ns, 90),
            core_score_p99_latency_ns: percentile(&stats.core_score_latencies_ns, 99),
            worker_score_p50_latency_ns: percentile(&stats.worker_score_latencies_ns, 50),
            worker_score_p90_latency_ns: percentile(&stats.worker_score_latencies_ns, 90),
            worker_score_p99_latency_ns: percentile(&stats.worker_score_latencies_ns, 99),
            lb_upstream_roundtrip_p50_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                50,
            ),
            lb_upstream_roundtrip_p90_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                90,
            ),
            lb_upstream_roundtrip_p99_latency_ns: percentile(
                &stats.lb_upstream_roundtrip_latencies_ns,
                99,
            ),
            rss_bytes: current_rss_bytes(),
            runtime_bytes_estimate: 0,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileForbiddenFlags {
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
}

impl RoleBindingProfileForbiddenFlags {
    fn clean() -> Self {
        Self {
            target_center_id_training_used: false,
            proof_rule_id_training_authority_used: false,
            concrete_x_lookup_used: false,
            local_out_t_runtime_extension_used: false,
            compiler_used: false,
            eval_packs_loaded: false,
            corpus_jsonl_loaded: false,
            python_demo_used: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RoleBindingProfileRuntimeSmokeReport {
    schema_version: String,
    verdict: String,
    registry_config_path: String,
    profile_count: usize,
    endpoint_health_pass: bool,
    endpoint_profiles_pass: bool,
    endpoint_score_pass: bool,
    endpoint_replay_pass: bool,
    endpoint_metrics_pass: bool,
    exact_cache_llm_calls: usize,
    exact_cache_plus_nando_llm_calls: usize,
    exact_cache_incremental_reduction_milli: usize,
    false_local_accepts: usize,
    p50_latency_ns: u128,
    p90_latency_ns: u128,
    p99_latency_ns: u128,
    rss_bytes: usize,
    runtime_bytes_estimate: usize,
    compiler_used: bool,
    eval_packs_loaded: bool,
    corpus_jsonl_loaded: bool,
    python_demo_used: bool,
    target_center_id_training_used: bool,
    proof_rule_id_training_authority_used: bool,
    concrete_x_lookup_used: bool,
    local_out_t_runtime_extension_used: bool,
    claim_boundary: String,
}

#[derive(Clone, Debug)]
struct RoleBindingProfileHttpError {
    status_code: u16,
    message: String,
}

struct RoleBindingProfileServeProcess {
    child: Child,
}

impl RoleBindingProfileServeProcess {
    fn start(
        config_path: &Path,
        bind_addr: &str,
        request_limit: Option<usize>,
    ) -> Result<Self, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("failed to locate current executable: {error}"))?;
        let mut command = Command::new(exe);
        command
            .arg("role-binding-profile-serve-v1")
            .arg(config_path)
            .arg(bind_addr);
        if let Some(request_limit) = request_limit {
            command.arg(request_limit.to_string());
        }
        let child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("failed to spawn role-binding profile server: {error}"))?;
        thread::sleep(Duration::from_millis(100));
        Ok(Self { child })
    }

    fn wait(&mut self) -> Result<(), String> {
        let status = self
            .child
            .wait()
            .map_err(|error| format!("failed to wait for role-binding profile server: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("role-binding profile server exited with {status}"))
        }
    }
}

impl Drop for RoleBindingProfileServeProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

struct RoleBindingProfileLoadBalancerProcess {
    child: Child,
}

impl RoleBindingProfileLoadBalancerProcess {
    fn start(
        config_path: &Path,
        bind_addr: &str,
        request_limit: Option<usize>,
    ) -> Result<Self, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("failed to locate current executable: {error}"))?;
        let mut command = Command::new(exe);
        command
            .arg("role-binding-profile-lb-serve-v1")
            .arg(config_path)
            .arg(bind_addr);
        if let Some(request_limit) = request_limit {
            command.arg(request_limit.to_string());
        }
        let child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                format!("failed to spawn role-binding profile load-balancer: {error}")
            })?;
        thread::sleep(Duration::from_millis(100));
        Ok(Self { child })
    }
}

impl Drop for RoleBindingProfileLoadBalancerProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn send_replay_batch(
    addr: SocketAddr,
    profile_id: &str,
    batch_index: usize,
    batch: &mut Vec<RoleBindingProfileScoreRequest>,
) -> Result<RoleBindingProfileReplayResponse, String> {
    let replay = RoleBindingProfileReplayRequest {
        request_id: format!("{profile_id}_batch_{batch_index}"),
        requests: std::mem::take(batch),
    };
    send_json_post::<_, RoleBindingProfileReplayResponse>(addr, "/replay", &replay)
}

fn send_replay_batch_timed(
    addr: SocketAddr,
    profile_id: &str,
    batch_index: usize,
    batch: &mut Vec<RoleBindingProfileScoreRequest>,
) -> Result<(RoleBindingProfileReplayResponse, u128), String> {
    let start = Instant::now();
    let response = send_replay_batch(addr, profile_id, batch_index, batch)?;
    Ok((response, start.elapsed().as_nanos()))
}

fn verify_role_binding_profile_packed_score_parity(
    registry: &RoleBindingProfileRuntimeRegistry,
    binary_suite: &RoleBindingProfileBinaryEvalPackSuiteReport,
    max_unique_sequences_per_profile: usize,
) -> Result<(usize, usize), String> {
    let mut checks = 0usize;
    let mut mismatches = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        let profile = registry
            .profiles
            .iter()
            .find(|profile| profile.config.profile_id == profile_id)
            .ok_or_else(|| format!("profile {profile_id} missing from parity registry"))?;
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let prepared = profile.runtime.prepare_active_fringe_from_iter(
                sequence
                    .active_fringe
                    .iter()
                    .map(|active| (active.center_id, active.strength)),
            );
            for slot in &sequence.slots {
                for impulse in slot
                    .positive_impulses
                    .iter()
                    .chain(slot.negative_impulses.iter())
                {
                    let packed = profile.runtime.score_alignment_prepared(
                        &prepared,
                        impulse.lane_id,
                        impulse.signed_strength,
                        slot.binding_output_slot,
                    );
                    let reference = profile.runtime.score_alignment_prepared_reference(
                        &prepared,
                        impulse.lane_id,
                        impulse.signed_strength,
                        slot.binding_output_slot,
                    );
                    checks += 1;
                    mismatches += usize::from(packed != reference);
                }
            }
        }
    }
    Ok((checks, mismatches))
}

fn sequence_to_profile_request(
    profile_id: &str,
    sequence: &RoleBindingProfileSequenceEvalRow,
    exact_cache_key: &str,
    repeat_index: usize,
) -> RoleBindingProfileScoreRequest {
    RoleBindingProfileScoreRequest {
        request_id: format!("{}::r{}", sequence.task_id, repeat_index),
        route_key: Some(profile_id.to_owned()),
        profile_id: Some(profile_id.to_owned()),
        exact_cache_key: Some(exact_cache_key.to_owned()),
        active_fringe: sequence.active_fringe.clone(),
        slots: sequence.slots.clone(),
        expect_local_operator: Some(sequence.expect_local_operator),
    }
}

#[derive(Clone, Debug)]
struct RoleBindingProfileThroughputRequestSet {
    requests: Vec<RoleBindingProfileScoreRequest>,
    unique_sequences: usize,
    expected_local_sequences: usize,
    expected_fallback_sequences: usize,
}

fn build_role_binding_profile_throughput_requests(
    binary_suite: &RoleBindingProfileBinaryEvalPackSuiteReport,
    lb_config: &RoleBindingProfileLoadBalancerConfig,
    max_unique_sequences_per_profile: usize,
) -> Result<RoleBindingProfileThroughputRequestSet, String> {
    let mut requests = Vec::new();
    let mut unique_sequences = 0usize;
    let mut expected_local_sequences = 0usize;
    let mut expected_fallback_sequences = 0usize;
    for suite_row in &binary_suite.rows {
        let profile_id = role_binding_profile_id(&suite_row.label, suite_row.seed);
        if !lb_config
            .upstreams
            .iter()
            .any(|upstream| upstream.profile_ids.iter().any(|id| id == &profile_id))
        {
            return Err(format!(
                "profile {profile_id} from binary suite is missing in throughput lb config"
            ));
        }
        let eval_pack_path = PathBuf::from(&suite_row.binary_eval_pack_path);
        let eval_pack = parse_profile_binary_eval_pack(&eval_pack_path)?;
        if eval_pack.package_fingerprint64 != Some(suite_row.package_fingerprint64) {
            return Err(format!(
                "binary eval-pack fingerprint mismatch for {} seed{}: pack={:?} suite={}",
                suite_row.label,
                suite_row.seed,
                eval_pack.package_fingerprint64,
                suite_row.package_fingerprint64
            ));
        }
        if eval_pack.generation_method.trim().is_empty() {
            return Err(format!(
                "binary eval-pack generation_method is empty for {}",
                eval_pack_path.display()
            ));
        }
        let profile_limit = if max_unique_sequences_per_profile == 0 {
            eval_pack.sequences.len()
        } else {
            max_unique_sequences_per_profile.min(eval_pack.sequences.len())
        };
        for sequence in eval_pack.sequences.iter().take(profile_limit) {
            let cache_key = format!("{}::{}", profile_id, sequence.task_id);
            let mut request = sequence_to_profile_request(&profile_id, sequence, &cache_key, 0);
            request.request_id = format!("{}::{}::throughput", profile_id, sequence.task_id);
            expected_local_sequences += usize::from(sequence.expect_local_operator);
            expected_fallback_sequences += usize::from(!sequence.expect_local_operator);
            unique_sequences += 1;
            requests.push(request);
        }
    }
    Ok(RoleBindingProfileThroughputRequestSet {
        requests,
        unique_sequences,
        expected_local_sequences,
        expected_fallback_sequences,
    })
}

fn parse_profile_binary_eval_pack(path: &Path) -> Result<RoleBindingProfileBinaryEvalPack, String> {
    let bytes = fs::read(path).map_err(|error| {
        format!(
            "failed to read binary eval-pack {}: {error}",
            path.display()
        )
    })?;
    let mut reader = RoleBindingProfileBinaryEvalPackReader::new(path, &bytes);
    reader.expect_magic()?;
    let package_fingerprint64 = match reader.read_u64()? {
        0 => None,
        value => Some(value),
    };
    let source_package_path = match reader.read_string()?.as_str() {
        "" => None,
        value => Some(value.to_owned()),
    };
    let generation_method = reader.read_string()?;
    let task_count = reader.read_u32()? as usize;
    let sequence_count = reader.read_u32()? as usize;
    reader.skip_tasks(task_count)?;
    let mut sequences = Vec::with_capacity(sequence_count);
    for _ in 0..sequence_count {
        let task_id = reader.read_string()?;
        let expect_local_operator = reader.read_bool()?;
        let active_fringe = reader.read_active_fringe()?;
        let slot_count = reader.read_u32()? as usize;
        let mut slots = Vec::with_capacity(slot_count);
        for _ in 0..slot_count {
            slots.push(RoleBindingProfileScoreSlotRow {
                binding_output_slot: reader.read_optional_slot()?,
                positive_impulses: reader.read_impulses()?,
                negative_impulses: reader.read_impulses()?,
            });
        }
        sequences.push(RoleBindingProfileSequenceEvalRow {
            task_id,
            active_fringe,
            slots,
            expect_local_operator,
        });
    }
    reader.finish()?;
    Ok(RoleBindingProfileBinaryEvalPack {
        package_fingerprint64,
        source_package_path,
        generation_method,
        sequences,
    })
}

struct RoleBindingProfileBinaryEvalPackReader<'a> {
    path: &'a Path,
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RoleBindingProfileBinaryEvalPackReader<'a> {
    fn new(path: &'a Path, bytes: &'a [u8]) -> Self {
        Self {
            path,
            bytes,
            offset: 0,
        }
    }

    fn expect_magic(&mut self) -> Result<(), String> {
        let magic = self.take(ROLE_BINDING_EVAL_PACK_BINARY_MAGIC.len())?;
        if magic != ROLE_BINDING_EVAL_PACK_BINARY_MAGIC {
            return Err(format!(
                "invalid role-binding binary eval-pack magic in {}",
                self.path.display()
            ));
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), String> {
        if self.offset != self.bytes.len() {
            return Err(format!(
                "trailing bytes in role-binding binary eval-pack {}: offset={} len={}",
                self.path.display(),
                self.offset,
                self.bytes.len()
            ));
        }
        Ok(())
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| "role-binding binary eval-pack offset overflow".to_owned())?;
        let slice = self.bytes.get(self.offset..end).ok_or_else(|| {
            format!(
                "truncated role-binding binary eval-pack {} at offset {}",
                self.path.display(),
                self.offset
            )
        })?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(u16::from_le_bytes(buf))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let mut buf = [0u8; 2];
        buf.copy_from_slice(self.take(2)?);
        Ok(i16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let mut buf = [0u8; 4];
        buf.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(self.take(8)?);
        Ok(u64::from_le_bytes(buf))
    }

    fn read_bool(&mut self) -> Result<bool, String> {
        match self.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(format!(
                "invalid bool value {value} in role-binding binary eval-pack {}",
                self.path.display()
            )),
        }
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| {
                format!(
                    "invalid utf8 string in role-binding binary eval-pack {}: {error}",
                    self.path.display()
                )
            })
    }

    fn read_optional_slot(&mut self) -> Result<Option<u8>, String> {
        match self.read_u16()? {
            u16::MAX => Ok(None),
            value => u8::try_from(value)
                .map(Some)
                .map_err(|_| format!("invalid output slot {value} in {}", self.path.display())),
        }
    }

    fn read_active_fringe(&mut self) -> Result<Vec<RoleBindingProfileActiveCenterRow>, String> {
        let len = self.read_u32()? as usize;
        let mut active_fringe = Vec::with_capacity(len);
        for _ in 0..len {
            active_fringe.push(RoleBindingProfileActiveCenterRow {
                center_id: self.read_u32()?,
                strength: self.read_i16()?,
            });
        }
        Ok(active_fringe)
    }

    fn read_impulses(&mut self) -> Result<Vec<RoleBindingProfileImpulseRow>, String> {
        let len = self.read_u32()? as usize;
        let mut impulses = Vec::with_capacity(len);
        for _ in 0..len {
            impulses.push(RoleBindingProfileImpulseRow {
                lane_id: self.read_u16()?,
                signed_strength: self.read_i16()?,
            });
        }
        Ok(impulses)
    }

    fn skip_tasks(&mut self, task_count: usize) -> Result<(), String> {
        for _ in 0..task_count {
            let _task_id = self.read_string()?;
            let _target_lane_id = self.read_u16()?;
            let _target_signed_strength = self.read_i16()?;
            let _wrong_lane_id = self.read_u16()?;
            let _wrong_signed_strength = self.read_i16()?;
            let _binding_output_slot = self.read_optional_slot()?;
            let _expect_local_operator = self.read_bool()?;
            let _active_fringe = self.read_active_fringe()?;
        }
        Ok(())
    }
}

fn validate_registry_config(config: &RoleBindingProfileRegistryConfig) -> Result<(), String> {
    if config.schema_version != "nando_role_binding_profile_registry_v1" {
        return Err(format!(
            "unsupported role-binding profile registry schema: {}",
            config.schema_version
        ));
    }
    if config.compiler_used
        || config.eval_packs_loaded
        || config.corpus_jsonl_loaded
        || config.python_demo_used
    {
        return Err("serving registry provenance flags must be false".to_owned());
    }
    if config.profiles.is_empty() {
        return Err("serving registry requires at least one profile".to_owned());
    }
    let mut profile_ids = HashSet::new();
    let mut routes = HashSet::new();
    for profile in &config.profiles {
        if profile.profile_id.trim().is_empty() {
            return Err("profile_id must not be empty".to_owned());
        }
        if !profile_ids.insert(profile.profile_id.clone()) {
            return Err(format!("duplicate profile_id: {}", profile.profile_id));
        }
        if profile.profile_kind != "role_binding_nwrb" {
            return Err(format!(
                "unsupported profile_kind for {}: {}",
                profile.profile_id, profile.profile_kind
            ));
        }
        if profile.threshold <= 0 {
            return Err(format!(
                "profile {} threshold must be positive",
                profile.profile_id
            ));
        }
        if !profile_acceptance_policy_is_supported(&profile.acceptance_policy) {
            return Err(format!(
                "profile {} has unsupported acceptance_policy: {}",
                profile.profile_id, profile.acceptance_policy
            ));
        }
        if profile.accepted_route_keys.is_empty() {
            return Err(format!(
                "profile {} needs at least one accepted_route_key",
                profile.profile_id
            ));
        }
        for route in &profile.accepted_route_keys {
            if !routes.insert(route.clone()) {
                return Err(format!("duplicate accepted_route_key: {route}"));
            }
        }
    }
    Ok(())
}

fn validate_load_balancer_config(
    config: &RoleBindingProfileLoadBalancerConfig,
) -> Result<(), String> {
    if config.schema_version != "nando_role_binding_profile_lb_v1" {
        return Err(format!(
            "unsupported role-binding profile load-balancer schema: {}",
            config.schema_version
        ));
    }
    if config.compiler_used
        || config.eval_packs_loaded
        || config.corpus_jsonl_loaded
        || config.python_demo_used
    {
        return Err("load-balancer provenance flags must be false".to_owned());
    }
    if config.upstreams.is_empty() {
        return Err("load-balancer config requires at least one upstream".to_owned());
    }
    let mut worker_ids = HashSet::new();
    let mut profile_ids = HashSet::new();
    let mut route_keys = HashSet::new();
    for upstream in &config.upstreams {
        if !worker_ids.insert(upstream.worker_id) {
            return Err(format!(
                "duplicate load-balancer worker_id: {}",
                upstream.worker_id
            ));
        }
        if upstream.upstream_addr.trim().is_empty() {
            return Err(format!(
                "load-balancer worker {} upstream_addr must not be empty",
                upstream.worker_id
            ));
        }
        upstream
            .upstream_addr
            .parse::<SocketAddr>()
            .map_err(|error| {
                format!(
                    "invalid load-balancer worker {} upstream_addr {}: {error}",
                    upstream.worker_id, upstream.upstream_addr
                )
            })?;
        if upstream.profile_ids.is_empty() {
            return Err(format!(
                "load-balancer worker {} needs at least one profile_id",
                upstream.worker_id
            ));
        }
        if upstream.accepted_route_keys.is_empty() {
            return Err(format!(
                "load-balancer worker {} needs at least one accepted_route_key",
                upstream.worker_id
            ));
        }
        for profile_id in &upstream.profile_ids {
            if profile_id.trim().is_empty() {
                return Err(format!(
                    "load-balancer worker {} has empty profile_id",
                    upstream.worker_id
                ));
            }
            if !profile_ids.insert(profile_id.clone()) {
                return Err(format!("duplicate load-balancer profile_id: {profile_id}"));
            }
        }
        for route_key in &upstream.accepted_route_keys {
            if route_key.trim().is_empty() {
                return Err(format!(
                    "load-balancer worker {} has empty route_key",
                    upstream.worker_id
                ));
            }
            if !route_keys.insert(route_key.clone()) {
                return Err(format!("duplicate load-balancer route_key: {route_key}"));
            }
        }
    }
    Ok(())
}

fn role_binding_profile_id(label: &str, seed: u8) -> String {
    let label = label.strip_prefix("sdk_").unwrap_or(label);
    format!("role_binding_{label}_seed{seed}")
}

fn role_binding_operator_classes(label: &str) -> Vec<String> {
    if label.contains("conditional") {
        return vec!["condition_route".to_owned(), "verify_repair".to_owned()];
    }
    if label.contains("edit") {
        return vec!["edit".to_owned()];
    }
    vec![
        "select".to_owned(),
        "move_copy".to_owned(),
        "order".to_owned(),
        "compose".to_owned(),
    ]
}

fn default_profile_acceptance_policy() -> String {
    "strict_ordered_energy_threshold".to_owned()
}

fn profile_acceptance_policy_is_supported(policy: &str) -> bool {
    matches!(
        policy,
        "strict_ordered_energy_threshold" | "energy_threshold_only"
    )
}

fn profile_accepts_score(
    acceptance_policy: &str,
    strict_ordered_pass: bool,
    energy_margin: i32,
    threshold: i32,
) -> bool {
    match acceptance_policy {
        "energy_threshold_only" => energy_margin >= threshold,
        _ => strict_ordered_pass && energy_margin >= threshold,
    }
}

fn profile_fallback_reason(
    acceptance_policy: &str,
    strict_ordered_pass: bool,
    energy_margin: i32,
    threshold: i32,
) -> Option<String> {
    if profile_accepts_score(
        acceptance_policy,
        strict_ordered_pass,
        energy_margin,
        threshold,
    ) {
        None
    } else if acceptance_policy == "strict_ordered_energy_threshold" && !strict_ordered_pass {
        Some("strict_slot_check_failed".to_owned())
    } else {
        Some("margin_below_threshold".to_owned())
    }
}

fn parse_optional_usize(value: Option<String>, name: &str) -> Result<Option<usize>, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|error| format!("invalid {name} '{}': {error}", value))
        })
        .transpose()
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse JSON {}: {error}", path.display()))
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode JSON {}: {error}", path.display()))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn read_http_request(stream: &mut TcpStream) -> Result<String, RoleBindingProfileHttpError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|error| http_error(400, format!("failed to read request: {error}")))?;
        if read == 0 {
            if bytes.is_empty() {
                return Err(http_connection_closed());
            }
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_HTTP_REQUEST_BYTES {
            return Err(http_error(413, "HTTP request body too large"));
        }
        if let Some(header_end) = find_header_end(&bytes) {
            let head = std::str::from_utf8(&bytes[..header_end])
                .map_err(|error| http_error(400, format!("invalid HTTP header utf8: {error}")))?;
            let content_length = http_content_length(head)?;
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    String::from_utf8(bytes)
        .map_err(|error| http_error(400, format!("invalid HTTP request utf8: {error}")))
}

fn http_connection_closed() -> RoleBindingProfileHttpError {
    http_error(499, "connection closed before request")
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn http_content_length(head: &str) -> Result<usize, RoleBindingProfileHttpError> {
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .map_err(|error| http_error(400, format!("invalid content-length: {error}")));
        }
    }
    Ok(0)
}

fn write_http_json<T>(
    stream: &mut TcpStream,
    status_code: u16,
    value: &T,
) -> Result<(), RoleBindingProfileHttpError>
where
    T: Serialize,
{
    let body = serde_json::to_string(value)
        .map_err(|error| http_error(500, format!("failed to encode response JSON: {error}")))?;
    let response = format!(
        "HTTP/1.1 {status_code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| http_error(500, format!("failed to write response: {error}")))
}

fn write_http_json_error(
    stream: &mut TcpStream,
    status_code: u16,
    message: &str,
) -> Result<(), String> {
    let body = serde_json::json!({
        "schema_version": "nando_role_binding_profile_error_v1",
        "status": "error",
        "message": message,
    });
    write_http_json(stream, status_code, &body).map_err(|error| error.message)
}

fn send_json_get<T>(addr: SocketAddr, path: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = send_http_request(addr, "GET", path, None)?;
    serde_json::from_str(&response.body).map_err(|error| {
        format!(
            "failed to parse GET {path} response: {error}; body={}",
            response.body
        )
    })
}

fn send_json_post<T, U>(addr: SocketAddr, path: &str, body: &T) -> Result<U, String>
where
    T: Serialize,
    U: for<'de> Deserialize<'de>,
{
    let body = serde_json::to_string(body)
        .map_err(|error| format!("failed to encode POST {path} request: {error}"))?;
    let response = send_http_request(addr, "POST", path, Some(&body))?;
    serde_json::from_str(&response.body).map_err(|error| {
        format!(
            "failed to parse POST {path} response: {error}; body={}",
            response.body
        )
    })
}

fn send_http_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let mut stream = connect_with_retry(addr)?;
    let body = body.unwrap_or("");
    send_http_request_on_stream(&mut stream, addr, method, path, body, "close")
}

fn send_http_request_on_stream(
    stream: &mut TcpStream,
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: &str,
    connection: &str,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: {connection}\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to send HTTP request: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("failed to flush HTTP request: {error}"))?;
    read_http_response_from_stream(stream, method, path)
}

fn read_http_response_from_stream(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
) -> Result<RoleBindingProfileHttpResponse, String> {
    let mut response_bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                if response_bytes.is_empty() {
                    return Err("upstream closed before HTTP response".to_owned());
                }
                break;
            }
            Ok(read) => {
                response_bytes.extend_from_slice(&buffer[..read]);
                if let Some(header_end) = find_header_end(&response_bytes) {
                    let head = std::str::from_utf8(&response_bytes[..header_end])
                        .map_err(|error| format!("invalid HTTP response header utf8: {error}"))?;
                    let content_length =
                        http_content_length(head).map_err(|error| error.message)?;
                    if response_bytes.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            Err(error)
                if error.kind() == ErrorKind::ConnectionReset && !response_bytes.is_empty() =>
            {
                break;
            }
            Err(error) => return Err(format!("failed to read HTTP response: {error}")),
        }
    }
    let response = String::from_utf8(response_bytes)
        .map_err(|error| format!("invalid HTTP response utf8: {error}"))?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| format!("malformed HTTP response: {response}"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| "missing HTTP response status line".to_owned())?;
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("malformed HTTP response status line: {status_line}"))?
        .parse::<u16>()
        .map_err(|error| format!("invalid HTTP status code: {error}"))?;
    if !(200..300).contains(&status_code) {
        return Err(format!(
            "HTTP {method} {path} failed: status={status_code} body={body}"
        ));
    }
    Ok(RoleBindingProfileHttpResponse {
        body: body.to_owned(),
    })
}

#[derive(Clone, Debug)]
struct RoleBindingProfileHttpResponse {
    body: String,
}

fn connect_with_retry(addr: SocketAddr) -> Result<TcpStream, String> {
    let mut last_error = None;
    for _ in 0..50 {
        match TcpStream::connect(addr) {
            Ok(stream) => {
                stream
                    .set_nodelay(true)
                    .map_err(|error| format!("failed to set client tcp nodelay: {error}"))?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
                    .map_err(|error| format!("failed to set client read timeout: {error}"))?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(HTTP_READ_TIMEOUT_SECS)))
                    .map_err(|error| format!("failed to set client write timeout: {error}"))?;
                return Ok(stream);
            }
            Err(error) => {
                last_error = Some(error);
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    Err(format!(
        "failed to connect to smoke server {addr}: {}",
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "unknown connection error".to_owned())
    ))
}

fn reserve_local_bind_addr() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to reserve local smoke port: {error}"))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read reserved local smoke addr: {error}"))?;
    drop(listener);
    Ok(addr.to_string())
}

fn http_error(status_code: u16, message: impl Into<String>) -> RoleBindingProfileHttpError {
    RoleBindingProfileHttpError {
        status_code,
        message: message.into(),
    }
}

fn real_traffic_operator_key(request: &RoleBindingProfileScoreRequest, action: &str) -> String {
    let route_key = request.route_key.as_deref().unwrap_or("none");
    let profile_id = request.profile_id.as_deref().unwrap_or("route_selected");
    format!("route={route_key}|profile={profile_id}|action={action}")
}

fn ratio_milli(numerator: usize, denominator: usize) -> usize {
    numerator
        .saturating_mul(1000)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn projected_accepts(non_exact_candidate_calls: usize, accept_rate_milli: usize) -> usize {
    non_exact_candidate_calls
        .saturating_mul(accept_rate_milli)
        .checked_div(1000)
        .unwrap_or(0)
}

type RoleBindingCalibrationMarginAccessor = fn(&RoleBindingEditLocalAcceptCalibrationRow) -> i32;

fn evaluate_edit_calibration_policy<F>(
    policy_name: &str,
    rows: &[RoleBindingEditLocalAcceptCalibrationRow],
    accepts: F,
) -> RoleBindingEditLocalAcceptPolicyReport
where
    F: Fn(&RoleBindingEditLocalAcceptCalibrationRow) -> bool,
{
    let mut accepted = 0usize;
    let mut true_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_true = 0usize;
    for row in rows {
        let accept = accepts(row);
        accepted += usize::from(accept);
        true_accepts += usize::from(accept && row.verifier_label);
        false_accepts += usize::from(accept && !row.verifier_label);
        missed_true += usize::from(!accept && row.verifier_label);
    }
    RoleBindingEditLocalAcceptPolicyReport {
        policy_name: policy_name.to_owned(),
        accepts: accepted,
        true_accepts,
        false_accepts,
        missed_true,
        safe: false_accepts == 0 && true_accepts > 0,
        threshold: None,
    }
}

fn planning_margin_collision_diagnostics(
    rows: &[RoleBindingEditLocalAcceptCalibrationRow],
) -> Vec<RoleBindingLocalAcceptMarginCollisionReport> {
    let margin_accessors: [(&str, RoleBindingCalibrationMarginAccessor); 4] = [
        (
            "energy_margin",
            |row: &RoleBindingEditLocalAcceptCalibrationRow| row.energy_margin,
        ),
        (
            "min_slot_margin",
            |row: &RoleBindingEditLocalAcceptCalibrationRow| row.min_slot_margin,
        ),
        (
            "marker_slot_margin",
            |row: &RoleBindingEditLocalAcceptCalibrationRow| row.marker_slot_margin,
        ),
        (
            "end_slot_margin",
            |row: &RoleBindingEditLocalAcceptCalibrationRow| row.end_slot_margin,
        ),
    ];
    margin_accessors
        .into_iter()
        .map(|(margin_name, value)| {
            let min_true_margin = rows
                .iter()
                .filter(|row| row.verifier_label)
                .map(value)
                .min();
            let false_rows_at_or_above_min_true_margin = min_true_margin
                .map(|threshold| {
                    rows.iter()
                        .filter(|row| !row.verifier_label && value(row) >= threshold)
                        .count()
                })
                .unwrap_or(0);
            let best_policy = best_single_threshold_policy(
                &format!("best_{margin_name}_threshold_request_side_only"),
                rows,
                value,
            );
            RoleBindingLocalAcceptMarginCollisionReport {
                margin_name: margin_name.to_owned(),
                min_true_margin,
                false_rows_at_or_above_min_true_margin,
                safe_accepts_all_true_rows: min_true_margin.is_some()
                    && false_rows_at_or_above_min_true_margin == 0,
                best_safe_true_accepts: if best_policy.false_accepts == 0 {
                    best_policy.true_accepts
                } else {
                    0
                },
            }
        })
        .collect()
}

fn best_single_threshold_policy<F>(
    policy_name: &str,
    rows: &[RoleBindingEditLocalAcceptCalibrationRow],
    value: F,
) -> RoleBindingEditLocalAcceptPolicyReport
where
    F: Fn(&RoleBindingEditLocalAcceptCalibrationRow) -> i32,
{
    let mut thresholds = rows.iter().map(&value).collect::<Vec<_>>();
    thresholds.sort_unstable();
    thresholds.dedup();
    thresholds.push(i32::MAX);
    let mut best: Option<RoleBindingEditLocalAcceptPolicyReport> = None;
    for threshold in thresholds {
        let mut policy =
            evaluate_edit_calibration_policy(policy_name, rows, |row| value(row) >= threshold);
        policy.threshold = Some(threshold);
        let replace = best.as_ref().is_none_or(|current| {
            (policy.false_accepts == 0 && current.false_accepts > 0)
                || (policy.false_accepts == current.false_accepts
                    && policy.true_accepts > current.true_accepts)
                || (policy.false_accepts == current.false_accepts
                    && policy.true_accepts == current.true_accepts
                    && policy.accepts < current.accepts)
        });
        if replace {
            best = Some(policy);
        }
    }
    best.unwrap_or_else(|| RoleBindingEditLocalAcceptPolicyReport {
        policy_name: policy_name.to_owned(),
        accepts: 0,
        true_accepts: 0,
        false_accepts: 0,
        missed_true: 0,
        safe: false,
        threshold: None,
    })
}

fn select_supported_mixed_safe_policy(
    calibration: &RoleBindingEditLocalAcceptCalibrationReport,
) -> Option<&RoleBindingEditLocalAcceptPolicyReport> {
    calibration
        .policies
        .iter()
        .filter(|policy| {
            policy.safe
                && policy.false_accepts == 0
                && policy.true_accepts > 0
                && policy.threshold.is_some()
                && policy.policy_name == "best_energy_margin_threshold"
        })
        .max_by(|left, right| {
            left.true_accepts
                .cmp(&right.true_accepts)
                .then_with(|| right.accepts.cmp(&left.accepts))
                .then_with(|| left.policy_name.cmp(&right.policy_name))
        })
}

fn select_supported_agent_control_admission_policy(
    calibration: &RoleBindingAgentControlAdmissionCalibrationReport,
) -> Option<&RoleBindingEditAdmissionPolicyReport> {
    const PREFERRED_POLICIES: &[&str] = &[
        "strict_control_stop_forms",
        "hard_stop_exclamation_caps_or_one_token",
        "hard_stop_exclamation_len_le_3",
        "hard_stop_exclamation_len_le_4",
    ];
    PREFERRED_POLICIES.iter().find_map(|policy_name| {
        calibration.policies.iter().find(|policy| {
            policy.policy_name == *policy_name
                && policy.robust_safe
                && policy.false_accepts == 0
                && policy.true_accepts >= calibration.minimum_true_support
        })
    })
}

fn agent_control_strict_control_stop_forms(
    features: &RoleBindingAgentControlAdmissionFeatures,
) -> bool {
    let hard_stop_exclamation = features.intent_stop
        && features.has_ostanov_word
        && features.has_exclamation
        && features.tokens_le_3
        && !features.has_work_words
        && (features.tokens_le_1 || features.all_capsish);
    let pause_request = features.intent_stop
        && features.has_pause_word
        && !features.has_work_words
        && !features.has_question_mark;
    hard_stop_exclamation
        || pause_request
        || features.stop_uppercase_goal_control
        || features.one_token_lowercase_stop
}

fn agent_control_admission_policy_accepts(
    policy_name: &str,
    features: &RoleBindingAgentControlAdmissionFeatures,
) -> Option<bool> {
    let accepts = match policy_name {
        "all_hook_ready_rows" => true,
        "strict_control_stop_forms" => agent_control_strict_control_stop_forms(features),
        "stop_intent" => features.intent_stop,
        "continue_intent" => features.intent_continue,
        "short_ack_intent" => features.intent_short_ack,
        "stop_no_work_tokens_le_2" => {
            features.intent_stop && !features.has_work_words && features.tokens_le_2
        }
        "stop_no_work_tokens_le_4" => {
            features.intent_stop && !features.has_work_words && features.tokens_le_4
        }
        "hard_stop_exclamation_len_le_4" => {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_4
                && !features.has_work_words
        }
        "hard_stop_exclamation_len_le_3" => {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_3
                && !features.has_work_words
        }
        "hard_stop_exclamation_caps_or_one_token" => {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_3
                && !features.has_work_words
                && (features.tokens_le_1 || features.all_capsish)
        }
        "one_token_ostanov" => {
            features.intent_stop && features.has_ostanov_word && features.tokens_le_1
        }
        "one_token_stop_word" => {
            features.intent_stop && features.has_stop_word && features.tokens_le_1
        }
        "short_ack_no_work_tokens_le_2" => {
            features.intent_short_ack && !features.has_work_words && features.tokens_le_2
        }
        "short_ack_no_work_chars_le_12" => {
            features.intent_short_ack && !features.has_work_words && features.chars_le_12
        }
        "continue_no_work_tokens_le_4" => {
            features.intent_continue && !features.has_work_words && features.tokens_le_4
        }
        "caps_stop_no_question" => {
            features.intent_stop
                && features.all_capsish
                && !features.has_question_mark
                && !features.has_work_words
        }
        _ => return None,
    };
    Some(accepts)
}

fn conditional_safe_policy_accepts(policy_name: &str, text: &str) -> Option<bool> {
    let lower = text.to_lowercase();
    let has_gate_terms = contains_any(
        &lower,
        &[
            "gate",
            "гейт",
            "audit",
            "hook",
            "verdict",
            "report",
            "отчет",
            "отчёт",
        ],
    );
    let accepts = match policy_name {
        "conditional_gate_terms_prompt_len_ge_300" => has_gate_terms && text.len() >= 300,
        _ => return None,
    };
    Some(accepts)
}

fn mixed_safe_policy_accepts(policy_name: &str, text: &str) -> Option<bool> {
    let lower = text.to_lowercase();
    let trimmed = lower.trim_start();
    let has_goal_control_terms = contains_any(
        &lower,
        &[
            "goal",
            "objective",
            "цель",
            "подцель",
            "stop",
            "стоп",
            "останов",
        ],
    ) || trimmed.starts_with("goal:")
        || trimmed.starts_with("цель:");
    let accepts = match policy_name {
        "mixed_no_goal_control_prompt" => !has_goal_control_terms,
        _ => return None,
    };
    Some(accepts)
}

fn select_conditional_safe_policy_from_evidence(
    registry: &RoleBindingProfileRuntimeRegistry,
    trace_rows: &[RoleBindingRealTrafficTraceRow],
    history_by_fingerprint: &BTreeMap<String, &str>,
    request_side_policy_name: &str,
) -> Result<RoleBindingMixedPromotionPolicySelection, String> {
    let mut scored_rows = Vec::new();
    for row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        let is_conditional = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains("conditional_branch"));
        if !is_conditional || request.active_fringe.is_empty() || request.slots.is_empty() {
            continue;
        }
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            continue;
        };
        if !conditional_safe_policy_accepts(request_side_policy_name, prompt_text).unwrap_or(false)
        {
            continue;
        }
        let Some(score) = score_role_binding_profile_request_detailed(registry, request) else {
            continue;
        };
        scored_rows.push((score.energy_margin, row.verified_safe_accept));
    }
    if scored_rows.is_empty() {
        return Err(format!(
            "conditional safe policy selection found no admitted scoreable rows for policy {request_side_policy_name}"
        ));
    }

    let mut thresholds = scored_rows
        .iter()
        .map(|(energy_margin, _)| *energy_margin)
        .filter(|threshold| *threshold > 0)
        .collect::<Vec<_>>();
    thresholds.sort_unstable();
    thresholds.dedup();
    let mut best_market_safe: Option<RoleBindingMixedPromotionPolicySelection> = None;
    for threshold in thresholds {
        let selection = evaluate_mixed_energy_promotion_threshold(
            "conditional_gate_not_short_energy_margin_threshold",
            "request_side_gate_terms_plus_evidence_trace_safe_threshold",
            threshold,
            &scored_rows,
        );
        let market_safe = selection.true_accepts > 0
            && selection.false_accepts == 0
            && selection.unverified_accepts == 0;
        if !market_safe {
            continue;
        }
        let replace = best_market_safe.as_ref().is_none_or(|current| {
            selection.true_accepts > current.true_accepts
                || (selection.true_accepts == current.true_accepts
                    && selection.threshold < current.threshold)
        });
        if replace {
            best_market_safe = Some(selection);
        }
    }
    best_market_safe.ok_or_else(|| {
        "conditional safe policy selection found no positive threshold with true_accepts>0, false_accepts=0, and unverified_accepts=0".to_owned()
    })
}

fn select_mixed_promotion_policy_from_evidence(
    registry: &RoleBindingProfileRuntimeRegistry,
    trace_rows: &[RoleBindingRealTrafficTraceRow],
    calibration_policy: &RoleBindingEditLocalAcceptPolicyReport,
    route_filter: &str,
) -> Result<RoleBindingMixedPromotionPolicySelection, String> {
    let calibration_threshold = calibration_policy
        .threshold
        .ok_or_else(|| "selected mixed calibration policy has no threshold".to_owned())?;
    let mut scored_rows = Vec::new();
    for row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        let is_target_route = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains(route_filter));
        if !is_target_route || request.active_fringe.is_empty() || request.slots.is_empty() {
            continue;
        }
        let Some(score) = score_role_binding_profile_request_detailed(registry, request) else {
            continue;
        };
        scored_rows.push((score.energy_margin, row.verified_safe_accept));
    }
    if scored_rows.is_empty() {
        return Err(format!(
            "safe policy selection found no scoreable rows for route filter {route_filter}"
        ));
    }

    let mut thresholds = scored_rows
        .iter()
        .map(|(energy_margin, _)| *energy_margin)
        .collect::<Vec<_>>();
    thresholds.push(calibration_threshold);
    thresholds.sort_unstable();
    thresholds.dedup();

    let mut best_market_safe: Option<RoleBindingMixedPromotionPolicySelection> = None;
    for threshold in thresholds {
        let selection = evaluate_mixed_energy_promotion_threshold(
            "market_safe_energy_margin_threshold",
            "evidence_trace_market_safe_threshold",
            threshold,
            &scored_rows,
        );
        let market_safe = selection.true_accepts > 0
            && selection.false_accepts == 0
            && selection.unverified_accepts == 0;
        if !market_safe {
            continue;
        }
        let replace = best_market_safe.as_ref().is_none_or(|current| {
            selection.true_accepts > current.true_accepts
                || (selection.true_accepts == current.true_accepts
                    && selection.threshold < current.threshold)
        });
        if replace {
            best_market_safe = Some(selection);
        }
    }
    if let Some(selection) = best_market_safe {
        return Ok(selection);
    }

    Ok(evaluate_mixed_energy_promotion_threshold(
        &calibration_policy.policy_name,
        "calibration_report_fallback_requires_shadow_audit",
        calibration_threshold,
        &scored_rows,
    ))
}

fn select_mixed_promotion_policy_from_request_side_evidence(
    registry: &RoleBindingProfileRuntimeRegistry,
    trace_rows: &[RoleBindingRealTrafficTraceRow],
    history_by_fingerprint: &BTreeMap<String, &str>,
    calibration_policy: &RoleBindingEditLocalAcceptPolicyReport,
    route_filter: &str,
    request_side_policy_name: &str,
) -> Result<RoleBindingMixedPromotionPolicySelection, String> {
    let calibration_threshold = calibration_policy
        .threshold
        .ok_or_else(|| "selected mixed calibration policy has no threshold".to_owned())?;
    let mut scored_rows = Vec::new();
    for row in trace_rows {
        let Some(request) = &row.nando_shadow_request else {
            continue;
        };
        let is_target_route = request
            .route_key
            .as_deref()
            .is_some_and(|route| route.contains(route_filter));
        if !is_target_route || request.active_fringe.is_empty() || request.slots.is_empty() {
            continue;
        }
        let request_fingerprint = row.request_fingerprint.clone().unwrap_or_default();
        let Some(prompt_text) = history_by_fingerprint.get(&request_fingerprint) else {
            continue;
        };
        if !mixed_safe_policy_accepts(request_side_policy_name, prompt_text).unwrap_or(false) {
            continue;
        }
        let Some(score) = score_role_binding_profile_request_detailed(registry, request) else {
            continue;
        };
        scored_rows.push((score.energy_margin, row.verified_safe_accept));
    }
    if scored_rows.is_empty() {
        return Err(format!(
            "request-side mixed safe policy selection found no scoreable admitted rows for route filter {route_filter}"
        ));
    }

    let mut thresholds = scored_rows
        .iter()
        .map(|(energy_margin, _)| *energy_margin)
        .collect::<Vec<_>>();
    thresholds.push(calibration_threshold);
    thresholds.sort_unstable();
    thresholds.dedup();

    let mut best_market_safe: Option<RoleBindingMixedPromotionPolicySelection> = None;
    for threshold in thresholds {
        let selection = evaluate_mixed_energy_promotion_threshold(
            "mixed_request_side_energy_margin_threshold",
            "request_side_no_goal_control_plus_evidence_trace_threshold",
            threshold,
            &scored_rows,
        );
        let market_safe = selection.true_accepts > 0
            && selection.false_accepts == 0
            && selection.unverified_accepts == 0;
        if !market_safe {
            continue;
        }
        let replace = best_market_safe.as_ref().is_none_or(|current| {
            selection.true_accepts > current.true_accepts
                || (selection.true_accepts == current.true_accepts
                    && selection.threshold < current.threshold)
        });
        if replace {
            best_market_safe = Some(selection);
        }
    }
    if let Some(selection) = best_market_safe {
        return Ok(selection);
    }

    Ok(evaluate_mixed_energy_promotion_threshold(
        &calibration_policy.policy_name,
        "request_side_calibration_report_fallback_requires_shadow_audit",
        calibration_threshold,
        &scored_rows,
    ))
}

fn evaluate_mixed_energy_promotion_threshold(
    policy_name: &str,
    selection_source: &str,
    threshold: i32,
    scored_rows: &[(i32, Option<bool>)],
) -> RoleBindingMixedPromotionPolicySelection {
    let mut accepts = 0usize;
    let mut true_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut unverified_accepts = 0usize;
    for (energy_margin, verified_safe_accept) in scored_rows {
        if *energy_margin < threshold {
            continue;
        }
        accepts += 1;
        match verified_safe_accept {
            Some(true) => true_accepts += 1,
            Some(false) => false_accepts += 1,
            None => unverified_accepts += 1,
        }
    }
    RoleBindingMixedPromotionPolicySelection {
        policy_name: policy_name.to_owned(),
        selection_source: selection_source.to_owned(),
        threshold,
        accepts,
        true_accepts,
        false_accepts,
        unverified_accepts,
    }
}

fn extract_edit_admission_features(text: &str) -> RoleBindingEditAdmissionFeatures {
    let lower = text.to_lowercase();
    let trimmed = text.trim_start();
    let tokens =
        extract_request_side_edit_tokens(text, usize::from(REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT));
    let marker = extract_request_side_marker_token(text, &tokens);
    RoleBindingEditAdmissionFeatures {
        request_len: text.len(),
        line_count: text.lines().count().max(1),
        starts_goal: trimmed.starts_with("Цель") || trimmed.to_lowercase().starts_with("goal"),
        starts_what: trimmed.starts_with("Что") || trimmed.to_lowercase().starts_with("what"),
        starts_yes: trimmed.starts_with("Да") || trimmed.to_lowercase().starts_with("yes"),
        has_runtime_terms: contains_any(&lower, &["runtime", "рантайм", "profile", "operator"]),
        has_goal_terms: contains_any(&lower, &["цель", "goal"]),
        has_next_terms: contains_any(&lower, &["следующ", "next"]),
        has_direct_edit_command: contains_any(
            &lower,
            &[
                "сделай",
                "добавь",
                "исправ",
                "чини",
                "почини",
                "перепиши",
                "обнови",
                "замени",
                "edit",
                "fix",
                "patch",
                "update",
            ],
        ),
        has_report_markers: contains_any(
            &lower,
            &[
                "verdict:",
                "what changed",
                "что изменил",
                "diagnostic read",
                "current smoke",
                "result:",
                "результат:",
            ],
        ),
        has_proof_boundary_terms: contains_any(
            &lower,
            &[
                "lookup",
                "target_id",
                "proof_rule_id",
                "local_out_t",
                "false_accept",
            ],
        ),
        has_code_diff_lines: text
            .lines()
            .any(|line| line.trim_start().starts_with('+') || line.contains(" +fn ")),
        has_file_like_token: contains_file_like_token(text),
        has_question_mark: text.contains('?') || text.contains('؟'),
        marker_len: marker.as_deref().map(str::len).unwrap_or_default(),
        marker_present: marker.is_some(),
    }
}

fn extract_agent_control_admission_features(
    text: &str,
) -> RoleBindingAgentControlAdmissionFeatures {
    let lower = text.to_lowercase();
    let token_count = normalized_token_count(&lower);
    let letter_count = text.chars().filter(|ch| ch.is_alphabetic()).count();
    let uppercase_count = text
        .chars()
        .filter(|ch| ch.is_alphabetic() && ch.is_uppercase())
        .count();
    let trimmed = text.trim();
    let raw_tokens = trimmed.split_whitespace().collect::<Vec<_>>();
    let intent = agent_control_intent_kind(text);
    RoleBindingAgentControlAdmissionFeatures {
        request_len: trimmed.len(),
        token_count,
        intent_stop: intent == "stop",
        intent_continue: intent == "continue",
        intent_short_ack: intent == "short_ack",
        has_stop_word: lower.contains("стоп"),
        has_stoy_word: lower.contains("стой"),
        has_ostanov_word: lower.contains("останов"),
        has_pause_word: lower.contains("пауза"),
        one_token_lowercase_stop: raw_tokens.len() == 1 && raw_tokens[0] == "стоп",
        stop_uppercase_goal_control: raw_tokens.len() == 2
            && raw_tokens[0] == "СТОП"
            && raw_tokens[1] == "GOAL",
        has_exclamation: text.contains('!'),
        has_question_mark: text.contains('?') || text.contains('؟'),
        has_work_words: contains_any(
            &lower,
            &[
                "код",
                "файл",
                "commit",
                "коммит",
                "patch",
                "diff",
                "cargo",
                "clippy",
                "провер",
                "чини",
                "исправ",
                "сделай",
            ],
        ),
        has_goal_or_plan_words: contains_any(
            &lower,
            &["goal", "цель", "план", "дальше", "следующ", "next"],
        ),
        all_capsish: letter_count > 0 && uppercase_count.saturating_mul(2) >= letter_count,
        tokens_le_1: token_count <= 1,
        tokens_le_2: token_count <= 2,
        tokens_le_3: token_count <= 3,
        tokens_le_4: token_count <= 4,
        chars_le_12: trimmed.len() <= 12,
        chars_le_20: trimmed.len() <= 20,
    }
}

fn extract_planning_next_step_admission_features(
    text: &str,
) -> RoleBindingPlanningNextStepAdmissionFeatures {
    let lower = text.to_lowercase();
    let trimmed = text.trim_start();
    let token_count = normalized_token_count(&lower);
    RoleBindingPlanningNextStepAdmissionFeatures {
        request_len: text.trim().len(),
        line_count: text.lines().count().max(1),
        token_count,
        starts_goal: trimmed.starts_with("Цель") || trimmed.to_lowercase().starts_with("goal"),
        starts_continue_or_next: contains_any(
            &trimmed.to_lowercase(),
            &["дальше", "следующ", "continue", "next"],
        ),
        starts_direct_action: contains_any(
            &trimmed.to_lowercase(),
            &[
                "делай",
                "давай",
                "сделай",
                "сделат",
                "выполни",
                "запусти",
                "чини",
                "почини",
                "исправ",
                "добавь",
                "добавит",
                "перепечат",
                "пиши",
                "write",
                "run",
                "do ",
                "fix",
                "add",
            ],
        ),
        has_direct_action_words: contains_any(
            &lower,
            &[
                "делай",
                "давай",
                "сделай",
                "сделат",
                "выполни",
                "запусти",
                "продолж",
                "чини",
                "почини",
                "исправ",
                "добавь",
                "добавит",
                "обнови",
                "перепечат",
                "запуш",
                "коммит",
                "commit",
                "run",
                "do ",
                "fix",
                "add",
                "update",
                "push",
            ],
        ),
        has_next_or_plan_words: contains_any(
            &lower,
            &[
                "дальше",
                "следующ",
                "план",
                "цель",
                "goal",
                "next",
                "continue",
            ],
        ),
        has_git_commit_terms: contains_any(
            &lower,
            &["git", "commit", "коммит", "закоммит", "запуш", "push"],
        ),
        has_patch_apply_terms: contains_any(
            &lower,
            &[
                "patch",
                "apply_patch",
                "diff",
                "исправ",
                "чини",
                "почини",
                "правк",
            ],
        ),
        has_project_artifact_terms: contains_any(
            &lower,
            &[
                "artifact",
                "артефакт",
                "report",
                "отчёт",
                "отчет",
                "cargo",
                "clippy",
                "test",
                "тест",
                "файл",
                "docs/",
                "crates/",
                ".rs",
                ".md",
                ".json",
                ".jsonl",
            ],
        ) || contains_file_like_token(text),
        has_nando_wave_terms: contains_any(
            &lower,
            &[
                "nando",
                "wave",
                "llmwave",
                "нандо",
                "волн",
                "runtime",
                "рантайм",
            ],
        ),
        has_goal_control_terms: contains_any(
            &lower,
            &[
                "/goal",
                " goal ",
                "goal:",
                "goal workflows",
                "цель:",
                "цели можно",
            ],
        ),
        has_report_or_failure_terms: contains_any(
            &lower,
            &[
                "verdict:",
                "report:",
                "failure",
                "failed",
                "ошибка",
                "провал",
                "не прошло",
                "не проходит",
                "review",
                "watch",
            ],
        ),
        has_question_mark: text.contains('?') || text.contains('؟'),
        has_code_diff_lines: text
            .lines()
            .any(|line| line.trim_start().starts_with('+') || line.contains(" +fn ")),
        has_file_like_token: contains_file_like_token(text),
    }
}

fn edit_admission_policy_reports(
    rows: &[RoleBindingEditAdmissionCalibrationRow],
    minimum_true_support: usize,
) -> Vec<RoleBindingEditAdmissionPolicyReport> {
    type EditAdmissionPredicate = fn(&RoleBindingEditAdmissionFeatures) -> bool;
    let policy_defs: Vec<(&str, EditAdmissionPredicate)> = vec![
        ("all_hook_ready_rows", |_| true),
        ("length_lt_1000", |features| features.request_len < 1000),
        ("length_lt_1800", |features| features.request_len < 1800),
        ("starts_goal", |features| features.starts_goal),
        ("starts_what", |features| features.starts_what),
        ("starts_yes", |features| features.starts_yes),
        ("runtime_and_length_lt_1000", |features| {
            features.has_runtime_terms && features.request_len < 1000
        }),
        ("runtime_and_length_lt_1800", |features| {
            features.has_runtime_terms && features.request_len < 1800
        }),
        ("starts_goal_and_length_lt_1000", |features| {
            features.starts_goal && features.request_len < 1000
        }),
        ("starts_goal_or_starts_what_length_lt_1800", |features| {
            (features.starts_goal || features.starts_what) && features.request_len < 1800
        }),
        ("starts_what_and_runtime", |features| {
            features.starts_what && features.has_runtime_terms
        }),
        ("direct_edit_command_and_length_lt_1000", |features| {
            features.has_direct_edit_command && features.request_len < 1000
        }),
        ("not_report_marker_and_length_lt_1800", |features| {
            !features.has_report_markers && features.request_len < 1800
        }),
        ("not_code_diff_and_length_lt_1800", |features| {
            !features.has_code_diff_lines && features.request_len < 1800
        }),
        ("marker_len_at_least_3_and_length_lt_1800", |features| {
            features.marker_len >= 3 && features.request_len < 1800
        }),
        ("question_mark_and_runtime", |features| {
            features.has_question_mark && features.has_runtime_terms
        }),
    ];
    policy_defs
        .into_iter()
        .map(|(name, predicate)| {
            evaluate_edit_admission_policy(name, rows, minimum_true_support, |row| {
                predicate(&row.features)
            })
        })
        .collect()
}

fn planning_next_step_admission_policy_reports(
    rows: &[RoleBindingPlanningNextStepAdmissionCalibrationRow],
    minimum_true_support: usize,
) -> Vec<RoleBindingEditAdmissionPolicyReport> {
    type PlanningAdmissionPredicate = fn(&RoleBindingPlanningNextStepAdmissionFeatures) -> bool;
    let policy_defs: Vec<(&str, PlanningAdmissionPredicate)> = vec![
        ("all_hook_ready_rows", |_| true),
        ("direct_action_words", |features| {
            features.has_direct_action_words
        }),
        ("git_commit_terms", |features| features.has_git_commit_terms),
        ("patch_apply_terms", |features| {
            features.has_patch_apply_terms
        }),
        ("project_artifact_terms", |features| {
            features.has_project_artifact_terms
        }),
        ("nando_wave_terms", |features| features.has_nando_wave_terms),
        ("git_or_patch_terms", |features| {
            features.has_git_commit_terms || features.has_patch_apply_terms
        }),
        ("direct_action_and_git_or_patch", |features| {
            features.has_direct_action_words
                && (features.has_git_commit_terms || features.has_patch_apply_terms)
        }),
        ("direct_action_and_project_artifact", |features| {
            features.has_direct_action_words && features.has_project_artifact_terms
        }),
        ("nando_wave_and_patch_apply", |features| {
            features.has_nando_wave_terms && features.has_patch_apply_terms
        }),
        ("nando_wave_and_project_artifact", |features| {
            features.has_nando_wave_terms && features.has_project_artifact_terms
        }),
        ("git_or_patch_no_report_failure", |features| {
            (features.has_git_commit_terms || features.has_patch_apply_terms)
                && !features.has_report_or_failure_terms
        }),
        ("project_artifact_no_report_failure", |features| {
            features.has_project_artifact_terms && !features.has_report_or_failure_terms
        }),
        ("direct_action_project_no_report_failure", |features| {
            features.has_direct_action_words
                && features.has_project_artifact_terms
                && !features.has_report_or_failure_terms
        }),
        (
            "direct_action_project_no_goal_control_no_failure",
            |features| {
                features.has_direct_action_words
                    && features.has_project_artifact_terms
                    && !features.has_goal_control_terms
                    && !features.has_report_or_failure_terms
            },
        ),
        ("direct_action_project_or_nando_no_failure", |features| {
            features.has_direct_action_words
                && (features.has_project_artifact_terms || features.has_nando_wave_terms)
                && !features.has_report_or_failure_terms
        }),
        ("concise_action_project_no_failure", |features| {
            features.has_direct_action_words
                && features.has_project_artifact_terms
                && !features.has_report_or_failure_terms
                && features.request_len < 1800
        }),
    ];
    policy_defs
        .into_iter()
        .map(|(name, predicate)| {
            evaluate_planning_next_step_admission_policy(name, rows, minimum_true_support, |row| {
                predicate(&row.features)
            })
        })
        .collect()
}

fn agent_control_admission_policy_reports(
    rows: &[RoleBindingAgentControlAdmissionCalibrationRow],
    minimum_true_support: usize,
) -> Vec<RoleBindingEditAdmissionPolicyReport> {
    type AgentControlAdmissionPredicate = fn(&RoleBindingAgentControlAdmissionFeatures) -> bool;
    let policy_defs: Vec<(&str, AgentControlAdmissionPredicate)> = vec![
        ("all_hook_ready_rows", |_| true),
        (
            "strict_control_stop_forms",
            agent_control_strict_control_stop_forms,
        ),
        ("stop_intent", |features| features.intent_stop),
        ("continue_intent", |features| features.intent_continue),
        ("short_ack_intent", |features| features.intent_short_ack),
        ("stop_no_work_tokens_le_2", |features| {
            features.intent_stop && !features.has_work_words && features.tokens_le_2
        }),
        ("stop_no_work_tokens_le_4", |features| {
            features.intent_stop && !features.has_work_words && features.tokens_le_4
        }),
        ("hard_stop_exclamation_len_le_4", |features| {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_4
                && !features.has_work_words
        }),
        ("hard_stop_exclamation_len_le_3", |features| {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_3
                && !features.has_work_words
        }),
        ("hard_stop_exclamation_caps_or_one_token", |features| {
            features.intent_stop
                && features.has_ostanov_word
                && features.has_exclamation
                && features.tokens_le_3
                && !features.has_work_words
                && (features.tokens_le_1 || features.all_capsish)
        }),
        ("one_token_ostanov", |features| {
            features.intent_stop && features.has_ostanov_word && features.tokens_le_1
        }),
        ("one_token_stop_word", |features| {
            features.intent_stop && features.has_stop_word && features.tokens_le_1
        }),
        ("short_ack_no_work_tokens_le_2", |features| {
            features.intent_short_ack && !features.has_work_words && features.tokens_le_2
        }),
        ("short_ack_no_work_chars_le_12", |features| {
            features.intent_short_ack && !features.has_work_words && features.chars_le_12
        }),
        ("continue_no_work_tokens_le_4", |features| {
            features.intent_continue && !features.has_work_words && features.tokens_le_4
        }),
        ("caps_stop_no_question", |features| {
            features.intent_stop
                && features.all_capsish
                && !features.has_question_mark
                && !features.has_work_words
        }),
    ];
    policy_defs
        .into_iter()
        .map(|(name, predicate)| {
            evaluate_agent_control_admission_policy(name, rows, minimum_true_support, |row| {
                predicate(&row.features)
            })
        })
        .collect()
}

fn evaluate_edit_admission_policy<F>(
    policy_name: &str,
    rows: &[RoleBindingEditAdmissionCalibrationRow],
    minimum_true_support: usize,
    accepts: F,
) -> RoleBindingEditAdmissionPolicyReport
where
    F: Fn(&RoleBindingEditAdmissionCalibrationRow) -> bool,
{
    let mut accepted = 0usize;
    let mut true_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_true = 0usize;
    for row in rows {
        let accept = accepts(row);
        accepted += usize::from(accept);
        true_accepts += usize::from(accept && row.verifier_label);
        false_accepts += usize::from(accept && !row.verifier_label);
        missed_true += usize::from(!accept && row.verifier_label);
    }
    RoleBindingEditAdmissionPolicyReport {
        policy_name: policy_name.to_owned(),
        accepts: accepted,
        true_accepts,
        false_accepts,
        missed_true,
        singleton_safe: false_accepts == 0 && true_accepts == 1,
        robust_safe: false_accepts == 0 && true_accepts >= minimum_true_support,
    }
}

fn evaluate_agent_control_admission_policy<F>(
    policy_name: &str,
    rows: &[RoleBindingAgentControlAdmissionCalibrationRow],
    minimum_true_support: usize,
    accepts: F,
) -> RoleBindingEditAdmissionPolicyReport
where
    F: Fn(&RoleBindingAgentControlAdmissionCalibrationRow) -> bool,
{
    let mut accepted = 0usize;
    let mut true_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_true = 0usize;
    for row in rows {
        let accept = accepts(row);
        accepted += usize::from(accept);
        true_accepts += usize::from(accept && row.verifier_label);
        false_accepts += usize::from(accept && !row.verifier_label);
        missed_true += usize::from(!accept && row.verifier_label);
    }
    RoleBindingEditAdmissionPolicyReport {
        policy_name: policy_name.to_owned(),
        accepts: accepted,
        true_accepts,
        false_accepts,
        missed_true,
        singleton_safe: false_accepts == 0 && true_accepts == 1,
        robust_safe: false_accepts == 0 && true_accepts >= minimum_true_support,
    }
}

fn evaluate_planning_next_step_admission_policy<F>(
    policy_name: &str,
    rows: &[RoleBindingPlanningNextStepAdmissionCalibrationRow],
    minimum_true_support: usize,
    accepts: F,
) -> RoleBindingEditAdmissionPolicyReport
where
    F: Fn(&RoleBindingPlanningNextStepAdmissionCalibrationRow) -> bool,
{
    let mut accepted = 0usize;
    let mut true_accepts = 0usize;
    let mut false_accepts = 0usize;
    let mut missed_true = 0usize;
    for row in rows {
        let accept = accepts(row);
        accepted += usize::from(accept);
        true_accepts += usize::from(accept && row.verifier_label);
        false_accepts += usize::from(accept && !row.verifier_label);
        missed_true += usize::from(!accept && row.verifier_label);
    }
    RoleBindingEditAdmissionPolicyReport {
        policy_name: policy_name.to_owned(),
        accepts: accepted,
        true_accepts,
        false_accepts,
        missed_true,
        singleton_safe: false_accepts == 0 && true_accepts == 1,
        robust_safe: false_accepts == 0 && true_accepts >= minimum_true_support,
    }
}

fn edit_admission_feature_counts(
    rows: &[RoleBindingEditAdmissionCalibrationRow],
) -> Vec<RoleBindingEditAdmissionFeatureCount> {
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    for row in rows {
        for feature in edit_admission_feature_names(&row.features) {
            let entry = counts.entry(feature).or_default();
            if row.verifier_label {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|(feature, (label_true_count, label_false_count))| {
            RoleBindingEditAdmissionFeatureCount {
                feature,
                label_true_count,
                label_false_count,
            }
        })
        .collect()
}

fn planning_next_step_admission_feature_counts(
    rows: &[RoleBindingPlanningNextStepAdmissionCalibrationRow],
) -> Vec<RoleBindingEditAdmissionFeatureCount> {
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    for row in rows {
        for feature in planning_next_step_admission_feature_names(&row.features) {
            let entry = counts.entry(feature).or_default();
            if row.verifier_label {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|(feature, (label_true_count, label_false_count))| {
            RoleBindingEditAdmissionFeatureCount {
                feature,
                label_true_count,
                label_false_count,
            }
        })
        .collect()
}

fn agent_control_admission_feature_counts(
    rows: &[RoleBindingAgentControlAdmissionCalibrationRow],
) -> Vec<RoleBindingEditAdmissionFeatureCount> {
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    for row in rows {
        for feature in agent_control_admission_feature_names(&row.features) {
            let entry = counts.entry(feature).or_default();
            if row.verifier_label {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
    }
    counts
        .into_iter()
        .map(|(feature, (label_true_count, label_false_count))| {
            RoleBindingEditAdmissionFeatureCount {
                feature,
                label_true_count,
                label_false_count,
            }
        })
        .collect()
}

fn agent_control_admission_feature_names(
    features: &RoleBindingAgentControlAdmissionFeatures,
) -> Vec<String> {
    let mut names = Vec::new();
    if features.intent_stop {
        names.push("intent_stop".to_owned());
    }
    if features.intent_continue {
        names.push("intent_continue".to_owned());
    }
    if features.intent_short_ack {
        names.push("intent_short_ack".to_owned());
    }
    if features.has_stop_word {
        names.push("has_stop_word".to_owned());
    }
    if features.has_stoy_word {
        names.push("has_stoy_word".to_owned());
    }
    if features.has_ostanov_word {
        names.push("has_ostanov_word".to_owned());
    }
    if features.has_pause_word {
        names.push("has_pause_word".to_owned());
    }
    if features.one_token_lowercase_stop {
        names.push("one_token_lowercase_stop".to_owned());
    }
    if features.stop_uppercase_goal_control {
        names.push("stop_uppercase_goal_control".to_owned());
    }
    if features.has_exclamation {
        names.push("has_exclamation".to_owned());
    }
    if features.has_question_mark {
        names.push("has_question_mark".to_owned());
    }
    if features.has_work_words {
        names.push("has_work_words".to_owned());
    }
    if features.has_goal_or_plan_words {
        names.push("has_goal_or_plan_words".to_owned());
    }
    if features.all_capsish {
        names.push("all_capsish".to_owned());
    }
    if features.tokens_le_1 {
        names.push("tokens_le_1".to_owned());
    }
    if features.tokens_le_2 {
        names.push("tokens_le_2".to_owned());
    }
    if features.tokens_le_3 {
        names.push("tokens_le_3".to_owned());
    }
    if features.tokens_le_4 {
        names.push("tokens_le_4".to_owned());
    }
    if features.chars_le_12 {
        names.push("chars_le_12".to_owned());
    }
    if features.chars_le_20 {
        names.push("chars_le_20".to_owned());
    }
    names
}

fn planning_next_step_admission_feature_names(
    features: &RoleBindingPlanningNextStepAdmissionFeatures,
) -> Vec<String> {
    let mut names = Vec::new();
    if features.request_len < 1000 {
        names.push("length_lt_1000".to_owned());
    }
    if features.request_len < 1800 {
        names.push("length_lt_1800".to_owned());
    }
    if features.line_count <= 3 {
        names.push("line_count_le_3".to_owned());
    }
    if features.token_count <= 30 {
        names.push("token_count_le_30".to_owned());
    }
    if features.starts_goal {
        names.push("starts_goal".to_owned());
    }
    if features.starts_continue_or_next {
        names.push("starts_continue_or_next".to_owned());
    }
    if features.starts_direct_action {
        names.push("starts_direct_action".to_owned());
    }
    if features.has_direct_action_words {
        names.push("has_direct_action_words".to_owned());
    }
    if features.has_next_or_plan_words {
        names.push("has_next_or_plan_words".to_owned());
    }
    if features.has_git_commit_terms {
        names.push("has_git_commit_terms".to_owned());
    }
    if features.has_patch_apply_terms {
        names.push("has_patch_apply_terms".to_owned());
    }
    if features.has_project_artifact_terms {
        names.push("has_project_artifact_terms".to_owned());
    }
    if features.has_nando_wave_terms {
        names.push("has_nando_wave_terms".to_owned());
    }
    if features.has_goal_control_terms {
        names.push("has_goal_control_terms".to_owned());
    }
    if features.has_report_or_failure_terms {
        names.push("has_report_or_failure_terms".to_owned());
    }
    if features.has_question_mark {
        names.push("has_question_mark".to_owned());
    }
    if features.has_code_diff_lines {
        names.push("has_code_diff_lines".to_owned());
    }
    if features.has_file_like_token {
        names.push("has_file_like_token".to_owned());
    }
    names
}

fn edit_admission_feature_names(features: &RoleBindingEditAdmissionFeatures) -> Vec<String> {
    let mut names = Vec::new();
    if features.request_len < 1000 {
        names.push("length_lt_1000".to_owned());
    }
    if features.request_len < 1800 {
        names.push("length_lt_1800".to_owned());
    }
    if features.starts_goal {
        names.push("starts_goal".to_owned());
    }
    if features.starts_what {
        names.push("starts_what".to_owned());
    }
    if features.starts_yes {
        names.push("starts_yes".to_owned());
    }
    if features.has_runtime_terms {
        names.push("has_runtime_terms".to_owned());
    }
    if features.has_goal_terms {
        names.push("has_goal_terms".to_owned());
    }
    if features.has_next_terms {
        names.push("has_next_terms".to_owned());
    }
    if features.has_direct_edit_command {
        names.push("has_direct_edit_command".to_owned());
    }
    if features.has_report_markers {
        names.push("has_report_markers".to_owned());
    }
    if features.has_proof_boundary_terms {
        names.push("has_proof_boundary_terms".to_owned());
    }
    if features.has_code_diff_lines {
        names.push("has_code_diff_lines".to_owned());
    }
    if features.has_file_like_token {
        names.push("has_file_like_token".to_owned());
    }
    if features.has_question_mark {
        names.push("has_question_mark".to_owned());
    }
    if features.marker_present {
        names.push("marker_present".to_owned());
    }
    if features.marker_len >= 3 {
        names.push("marker_len_at_least_3".to_owned());
    }
    names
}

fn cpu_route_builder_recommendation(route_key: &str) -> (String, String) {
    if route_key.contains("edit_marker_length") {
        (
            "CPU-process structured edit requests: detect edit intent, affected file/text marker, requested length/shape constraint, and deterministic patch slots from request text only.".to_owned(),
            "edit_marker_length_payload_builder_v1".to_owned(),
        )
    } else if route_key.contains("conditional_branch") {
        (
            "CPU-process check/gate/branch requests: extract condition, evidence slots, allowed/refused branch, and fallback threshold from request text only.".to_owned(),
            "conditional_branch_payload_builder_v1".to_owned(),
        )
    } else if route_key.contains("mixed_map") {
        (
            "CPU-process mapping/update/reorder requests: extract source slots, destination slots, ordered mapping action, and invariant checks from request text only.".to_owned(),
            "mixed_map_payload_builder_v1".to_owned(),
        )
    } else {
        (
            "CPU-process route candidates only after a request-side payload builder can emit active_fringe and slots without answer leakage.".to_owned(),
            "generic_request_side_payload_builder_v1".to_owned(),
        )
    }
}

struct FeedbackRouteStageInputs {
    payload_ready_events: usize,
    payload_built_events: usize,
    scoreable_payload_events: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    false_accepts: usize,
    local_accept_calibration_ran: bool,
    local_accept_safe_policy_found: bool,
    local_accept_support_qualified: bool,
}

fn feedback_route_stage(inputs: FeedbackRouteStageInputs) -> String {
    if inputs.false_accepts > 0 {
        "false_accepts_block_local_policy".to_owned()
    } else if inputs.verified_cpu_accept_eligible_events > 0 {
        "verified_cpu_accept_eligible".to_owned()
    } else if inputs.local_accept_safe_policy_found && !inputs.local_accept_support_qualified {
        "local_accept_calibration_support_insufficient".to_owned()
    } else if inputs.local_accept_safe_policy_found {
        "local_accept_calibration_safe_policy_candidate".to_owned()
    } else if inputs.local_accept_calibration_ran {
        "local_accept_calibration_failed".to_owned()
    } else if inputs.verification_hook_ready_events > 0 {
        "verification_hook_ready_waiting_local_accept".to_owned()
    } else if inputs.scoreable_payload_events > 0 {
        "scoreable_payload_missing_verification_hook".to_owned()
    } else if inputs.payload_built_events > 0 {
        "payload_built_not_scoreable".to_owned()
    } else if inputs.payload_ready_events > 0 {
        "payload_ready_builder_not_scoreable".to_owned()
    } else {
        "payload_builder_missing".to_owned()
    }
}

fn feedback_route_next_action(stage: &str) -> String {
    match stage {
        "false_accepts_block_local_policy" => {
            "Do not promote; split the route or tighten request-side admission until false_accepts=0."
                .to_owned()
        }
        "verified_cpu_accept_eligible" => {
            "Run non-synthetic soak with false_accepts=0 before any market claim.".to_owned()
        }
        "verification_hook_ready_waiting_local_accept" => {
            "Run local-accept calibration; if no safe policy exists, improve request-side admission or payload features.".to_owned()
        }
        "local_accept_calibration_safe_policy_candidate" => {
            "Promote the safe policy only through a separate shadow trace rewrite with provider cost, rollback, and false_accepts=0.".to_owned()
        }
        "local_accept_calibration_support_insufficient" => {
            "Collect more verifier-true rows or raise admission quality before promotion; singleton safe policies stay review-only.".to_owned()
        }
        "local_accept_calibration_failed" => {
            "Improve request-side admission or payload geometry before enabling local accepts.".to_owned()
        }
        "scoreable_payload_missing_verification_hook" => {
            "Attach response/tool-call evidence and deterministic output verification.".to_owned()
        }
        "payload_built_not_scoreable" => {
            "Fix active_fringe/slot impulse construction until scorer leaves empty-contract fallback."
                .to_owned()
        }
        "payload_ready_builder_not_scoreable" => {
            "Finish request-side active_fringe and slot builder for ready rows.".to_owned()
        }
        _ => "Build the request-side payload builder for this route family.".to_owned(),
    }
}

fn existing_route_verifier(route_key: &str) -> String {
    if route_key.contains("agent_control") {
        "deterministic_agent_control_output_verifier_v1".to_owned()
    } else if route_key.contains("conditional_branch") {
        "deterministic_conditional_output_verifier_v1".to_owned()
    } else if route_key.contains("edit_marker_length") {
        "deterministic_edit_output_verifier_v1".to_owned()
    } else if route_key.contains("mixed_map") {
        "deterministic_mixed_output_verifier_v1".to_owned()
    } else {
        "route_specific_deterministic_verifier_required".to_owned()
    }
}

fn cpu_operator_priority_score(
    candidate_events: usize,
    scoreable_payload_events: usize,
    verification_hook_ready_events: usize,
    verified_cpu_accept_eligible_events: usize,
    false_accepts: usize,
    readiness: &str,
) -> i64 {
    let readiness_weight = if readiness == "existing_profile" {
        220
    } else if readiness.starts_with("high") {
        260
    } else if readiness.starts_with("medium") {
        150
    } else if readiness.starts_with("low") {
        40
    } else {
        15
    };
    (candidate_events as i64 * readiness_weight)
        + (scoreable_payload_events as i64 * 250)
        + (verification_hook_ready_events as i64 * 500)
        + (verified_cpu_accept_eligible_events as i64 * 10_000)
        - (false_accepts as i64 * 100_000)
}

fn analyze_route_gap_payload_readiness(family_key: &str, text: &str) -> RouteGapPayloadReadiness {
    let lower = text.to_lowercase();
    let artifact_signal = has_route_gap_artifact_signal(text, &lower);
    let goal_signal = contains_any(
        &lower,
        &[
            "goal",
            "цель",
            "план",
            "следующий",
            "дальше",
            "roadmap",
            "executor",
            "route",
            "routability",
            "nando",
            "wave",
            "llmwave",
        ],
    );
    let metric_signal = contains_any(
        &lower,
        &[
            "p99",
            "latency",
            "метрик",
            "accuracy",
            "false_accept",
            "coverage",
            "milli",
            "bytes",
            "rss",
            "qps",
        ],
    );
    let verification_signal = contains_any(
        &lower,
        &[
            "провер",
            "verify",
            "вериф",
            "gate",
            "audit",
            "отчет",
            "отчёт",
            "report",
            "evidence",
            "результат",
            "result",
            "статус",
            "status",
            "коммит",
            "commit",
        ],
    ) || artifact_signal
        || metric_signal;

    let (has_request_signal, has_context_signal, has_evidence_signal, has_verifier_signal) =
        match family_key {
            "planning_next_step" => {
                let request = contains_any(
                    &lower,
                    &[
                        "дальше",
                        "план",
                        "следующий",
                        "что делать",
                        "goal",
                        "roadmap",
                    ],
                );
                let context = goal_signal || artifact_signal;
                let evidence =
                    artifact_signal || contains_any(&lower, &["сделал", "готово", "чек", "check"]);
                (request, context, evidence, verification_signal)
            }
            "read_inspect" => {
                let request = contains_any(
                    &lower,
                    &[
                        "читай",
                        "прочитай",
                        "посмотри",
                        "ознакомь",
                        "inspect",
                        "read",
                    ],
                );
                (
                    request,
                    artifact_signal,
                    artifact_signal,
                    verification_signal,
                )
            }
            "retrieval_lookup" => {
                let request = contains_any(
                    &lower,
                    &[
                        "найди",
                        "поищи",
                        "где лежит",
                        "где найти",
                        "lookup",
                        "search",
                    ],
                );
                let source_signal = artifact_signal
                    || contains_any(
                        &lower,
                        &["ссылк", "url", "pdf", "документ", "источник", "source"],
                    );
                (request, source_signal, source_signal, verification_signal)
            }
            "metrics_report_readout" => {
                let request =
                    metric_signal || contains_any(&lower, &["отчет", "отчёт", "report", "readout"]);
                (
                    request,
                    artifact_signal || metric_signal,
                    metric_signal,
                    verification_signal,
                )
            }
            "serving_ops" => {
                let request = contains_any(
                    &lower,
                    &[
                        "сервер",
                        "daemon",
                        "демон",
                        "worker",
                        "lb",
                        "http",
                        "hostworld",
                        "vps",
                        "nginx",
                        "systemd",
                    ],
                );
                let context =
                    artifact_signal || metric_signal || contains_any(&lower, &["health", "status"]);
                (request, context, context, verification_signal)
            }
            "git_control" => {
                let request = contains_any(
                    &lower,
                    &["git", "коммит", "commit", "пуш", "push", "status", "diff"],
                );
                (
                    request,
                    artifact_signal || request,
                    request,
                    verification_signal,
                )
            }
            "dataset_corpus" => {
                let request = contains_any(
                    &lower,
                    &["датасет", "корпус", "задач", "jsonl", "batch", "negative"],
                );
                let evidence = artifact_signal
                    || contains_any(&lower, &["schema", "валид", "balance", "строк"]);
                (request, evidence, evidence, verification_signal)
            }
            "style_brevity" => {
                let request = contains_any(
                    &lower,
                    &["коротко", "простын", "без воды", "не пиши", "кратко"],
                );
                (request, true, true, request)
            }
            "answer_or_explain" => {
                let request = contains_any(
                    &lower,
                    &[
                        "что",
                        "почему",
                        "зачем",
                        "как",
                        "сколько",
                        "можешь",
                        "объясни",
                        "оцени",
                    ],
                );
                (request, artifact_signal, false, false)
            }
            "project_context_dialogue" => {
                let request = goal_signal
                    || contains_any(&lower, &["проект", "модель", "архитект", "l1", "l2", "l3"]);
                (
                    request,
                    artifact_signal || goal_signal,
                    artifact_signal,
                    verification_signal,
                )
            }
            _ => {
                let request = normalized_token_count(&lower) > 0;
                (
                    request,
                    artifact_signal,
                    artifact_signal,
                    verification_signal,
                )
            }
        };
    let payload_ready =
        has_request_signal && has_context_signal && has_evidence_signal && has_verifier_signal;
    let mut missing_reasons = Vec::new();
    if !has_request_signal {
        missing_reasons.push("missing_request_signal".to_owned());
    }
    if !has_context_signal {
        missing_reasons.push("missing_context_signal".to_owned());
    }
    if !has_evidence_signal {
        missing_reasons.push("missing_evidence_signal".to_owned());
    }
    if !has_verifier_signal {
        missing_reasons.push("missing_verifier_signal".to_owned());
    }
    let recommended_builder_kind = if payload_ready {
        format!("{family_key}_payload_builder_v1_candidate")
    } else if has_request_signal && has_context_signal {
        format!("{family_key}_needs_evidence_or_verifier_signal")
    } else if has_request_signal {
        format!("{family_key}_router_needs_context_evidence_verifier")
    } else {
        format!("not_{family_key}_payload_ready")
    };
    RouteGapPayloadReadiness {
        has_request_signal,
        has_context_signal,
        has_evidence_signal,
        has_verifier_signal,
        payload_ready,
        recommended_builder_kind,
        missing_reasons,
    }
}

fn has_route_gap_artifact_signal(text: &str, lower: &str) -> bool {
    contains_file_like_token(text)
        || contains_marker_like_signal(text)
        || contains_any(
            lower,
            &[
                ".md",
                ".rs",
                ".json",
                ".jsonl",
                "docs/",
                "target/",
                "crates/",
                "src/",
                "файл",
                "док",
                "артефакт",
                "artifact",
                "report",
                "отчет",
                "отчёт",
                "trace",
                "лог",
                "log",
            ],
        )
}

fn analyze_edit_payload_readiness(text: &str) -> EditPayloadReadiness {
    let lower = text.to_lowercase();
    let has_edit_intent = contains_any(
        &lower,
        &[
            "edit",
            "rewrite",
            "replace",
            "patch",
            "fix",
            "update",
            "change",
            "refactor",
            "format",
            "shorten",
            "перепиши",
            "исправ",
            "измени",
            "замени",
            "сделай",
            "добавь",
            "убери",
            "сократи",
            "почини",
            "отредакт",
        ],
    );
    let has_scope_or_file = contains_file_like_token(text)
        || contains_any(
            &lower,
            &[
                "file",
                "path",
                "function",
                "module",
                "test",
                "doc",
                "readme",
                "код",
                "файл",
                "функц",
                "модул",
                "тест",
                "док",
                "строк",
            ],
        );
    let has_marker = contains_marker_like_signal(text)
        || contains_any(
            &lower,
            &[
                "this",
                "above",
                "below",
                "selected",
                "fragment",
                "snippet",
                "block",
                "это",
                "этот",
                "выше",
                "ниже",
                "фрагмент",
                "кусок",
                "блок",
                "вот это",
            ],
        );
    let has_length_or_shape = contains_any(
        &lower,
        &[
            "short",
            "shorter",
            "brief",
            "compact",
            "concise",
            "one line",
            "lines",
            "tokens",
            "json",
            "jsonl",
            "markdown",
            "table",
            "format",
            "schema",
            "коротко",
            "короче",
            "кратко",
            "без простын",
            "меньше",
            "строк",
            "формат",
            "таблиц",
            "схем",
        ],
    );
    let has_code_or_patch_signal = contains_any(
        &lower,
        &[
            "diff",
            "patch",
            "apply_patch",
            "cargo",
            "rust",
            "python",
            "json",
            "yaml",
            "toml",
            "markdown",
            "```",
            "код",
            "патч",
            "дифф",
        ],
    ) || contains_file_like_token(text);
    let payload_ready = has_edit_intent && has_scope_or_file && has_marker && has_length_or_shape;
    let mut missing_reasons = Vec::new();
    if !has_edit_intent {
        missing_reasons.push("missing_edit_intent".to_owned());
    }
    if !has_scope_or_file {
        missing_reasons.push("missing_scope_or_file".to_owned());
    }
    if !has_marker {
        missing_reasons.push("missing_marker".to_owned());
    }
    if !has_length_or_shape {
        missing_reasons.push("missing_length_or_shape".to_owned());
    }
    let recommended_builder_kind = if payload_ready {
        "edit_marker_length_payload_builder_v1_candidate".to_owned()
    } else if has_edit_intent && has_scope_or_file {
        "edit_scope_payload_builder_needs_marker_or_shape".to_owned()
    } else if has_edit_intent {
        "edit_intent_router_needs_scope_marker_shape".to_owned()
    } else {
        "not_edit_payload_ready".to_owned()
    };
    EditPayloadReadiness {
        has_edit_intent,
        has_scope_or_file,
        has_marker,
        has_length_or_shape,
        has_code_or_patch_signal,
        payload_ready,
        recommended_builder_kind,
        missing_reasons,
    }
}

fn analyze_conditional_payload_readiness(text: &str) -> ConditionalPayloadReadiness {
    let lower = text.to_lowercase();
    let has_condition_signal = contains_any(
        &lower,
        &[
            "если",
            "when",
            "if ",
            "if/then",
            "услов",
            "condition",
            "branch",
            "gate",
            "проверь",
            "провер",
            "verify",
        ],
    );
    let has_branch_signal = contains_any(
        &lower,
        &[
            "pass",
            "fail",
            "accept",
            "reject",
            "allow",
            "refuse",
            "fallback",
            "true",
            "false",
            "yes",
            "no",
            "иначе",
            "то ",
            "принять",
            "отклон",
            "запрет",
            "разреш",
        ],
    );
    let has_evidence_signal = contains_any(
        &lower,
        &[
            "evidence",
            "доказ",
            "метрик",
            "report",
            "verdict",
            "result",
            "результат",
            "false_accept",
            "margin",
            "latency",
            "p99",
            "trace",
            "hook",
            "audit",
        ],
    ) || contains_marker_like_signal(text)
        || contains_file_like_token(text);
    let has_branch_tokens = extract_conditional_branch_tokens(text).is_some();
    let payload_ready =
        has_condition_signal && has_branch_signal && has_evidence_signal && has_branch_tokens;
    let mut missing_reasons = Vec::new();
    if !has_condition_signal {
        missing_reasons.push("missing_condition_signal".to_owned());
    }
    if !has_branch_signal {
        missing_reasons.push("missing_branch_signal".to_owned());
    }
    if !has_evidence_signal {
        missing_reasons.push("missing_evidence_signal".to_owned());
    }
    if !has_branch_tokens {
        missing_reasons.push("missing_branch_tokens".to_owned());
    }
    let recommended_builder_kind = if payload_ready {
        "conditional_branch_payload_builder_v1_candidate".to_owned()
    } else if has_condition_signal && has_branch_signal {
        "conditional_branch_needs_evidence_or_branch_tokens".to_owned()
    } else if has_condition_signal {
        "conditional_router_needs_branch_and_evidence".to_owned()
    } else {
        "not_conditional_payload_ready".to_owned()
    };
    ConditionalPayloadReadiness {
        has_condition_signal,
        has_branch_signal,
        has_evidence_signal,
        has_branch_tokens,
        payload_ready,
        recommended_builder_kind,
        missing_reasons,
    }
}

fn analyze_mixed_payload_readiness(text: &str) -> MixedPayloadReadiness {
    let lower = text.to_lowercase();
    let has_action_signal = contains_any(
        &lower,
        &[
            "запиши",
            "обнови",
            "перенеси",
            "перенос",
            "добавь",
            "сделай",
            "собери",
            "сохрани",
            "map",
            "mapping",
            "route",
            "operator",
            "оператор",
            "runtime",
            "traffic",
            "trace",
        ],
    );
    let has_source_signal = contains_marker_like_signal(text)
        || contains_any(
            &lower,
            &[
                "это",
                "вот",
                "цель",
                "правило",
                "оператор",
                "trace",
                "route",
                "runtime",
                "архитект",
                "список",
            ],
        )
        || contains_file_like_token(text);
    let has_destination_signal = contains_any(
        &lower,
        &[
            "сюда",
            "в ",
            "to ",
            "into",
            "roadmap",
            ".md",
            "docs",
            "памят",
            "план",
            "док",
            "файл",
            "список",
            "правило",
            "goal",
            "архитект",
        ],
    ) || contains_file_like_token(text);
    let has_mapping_signal = contains_any(
        &lower,
        &[
            "->",
            "=>",
            "из ",
            "в ",
            "from",
            " to ",
            "map",
            "mapping",
            "route",
            "перенес",
            "связ",
            "оператор",
            "обнови",
            "запиши",
            "собери",
        ],
    );
    let has_map_tokens = extract_mixed_map_tokens(text).is_some();
    let payload_ready = has_action_signal
        && has_source_signal
        && has_destination_signal
        && has_mapping_signal
        && has_map_tokens;
    let mut missing_reasons = Vec::new();
    if !has_action_signal {
        missing_reasons.push("missing_action_signal".to_owned());
    }
    if !has_source_signal {
        missing_reasons.push("missing_source_signal".to_owned());
    }
    if !has_destination_signal {
        missing_reasons.push("missing_destination_signal".to_owned());
    }
    if !has_mapping_signal {
        missing_reasons.push("missing_mapping_signal".to_owned());
    }
    if !has_map_tokens {
        missing_reasons.push("missing_map_tokens".to_owned());
    }
    let recommended_builder_kind = if payload_ready {
        "mixed_map_payload_builder_v1_candidate".to_owned()
    } else if has_action_signal && has_source_signal {
        "mixed_map_needs_destination_or_mapping".to_owned()
    } else if has_action_signal {
        "mixed_router_needs_source_destination_mapping".to_owned()
    } else {
        "not_mixed_payload_ready".to_owned()
    };
    MixedPayloadReadiness {
        has_action_signal,
        has_source_signal,
        has_destination_signal,
        has_mapping_signal,
        has_map_tokens,
        payload_ready,
        recommended_builder_kind,
        missing_reasons,
    }
}

fn build_mixed_map_dry_run_request(
    event_id: &str,
    fingerprint: &u64,
    candidate: &CodexHistoryRouteCandidate,
    text: &str,
) -> Option<RoleBindingProfileScoreRequest> {
    let tokens = extract_mixed_map_tokens(text)?;
    let mut active_fringe = Vec::new();
    active_fringe.extend(mixed_request_operator_centers());
    active_fringe.extend(mixed_role_surface_centers(
        REAL_TRAFFIC_MIXED_SOURCE_ROLE_SLOT,
        &tokens.source_token,
    ));
    active_fringe.extend(mixed_role_surface_centers(
        REAL_TRAFFIC_MIXED_DESTINATION_ROLE_SLOT,
        &tokens.destination_token,
    ));
    active_fringe.extend(mixed_role_surface_centers(
        REAL_TRAFFIC_MIXED_ACTION_ROLE_SLOT,
        &tokens.action_token,
    ));
    active_fringe.extend(mixed_role_surface_centers(
        REAL_TRAFFIC_MIXED_INVARIANT_ROLE_SLOT,
        &tokens.invariant_token,
    ));
    let active_fringe = merge_profile_active_centers(active_fringe);

    let mut slots = Vec::new();
    if let Some(slot) = mixed_request_score_slot(0, &tokens.destination_token, &tokens.source_token)
    {
        slots.push(slot);
    }
    if let Some(slot) = mixed_request_score_slot(1, &tokens.action_token, &tokens.destination_token)
    {
        slots.push(slot);
    }
    if let Some(slot) = mixed_request_score_slot(2, &tokens.invariant_token, &tokens.source_token) {
        slots.push(slot);
    }
    if active_fringe.is_empty() || slots.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreRequest {
        request_id: event_id.to_owned(),
        route_key: Some(candidate.route_key.clone()),
        profile_id: Some(candidate.profile_id.clone()),
        exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
        active_fringe,
        slots,
        // Dry-run only: response verification has not proven a safe local operator.
        expect_local_operator: Some(false),
    })
}

fn mixed_request_operator_centers() -> Vec<RoleBindingProfileActiveCenterRow> {
    [
        (0, REAL_TRAFFIC_MIXED_DESTINATION_ROLE_SLOT),
        (1, REAL_TRAFFIC_MIXED_ACTION_ROLE_SLOT),
        (2, REAL_TRAFFIC_MIXED_INVARIANT_ROLE_SLOT),
        (0, REAL_TRAFFIC_MIXED_SOURCE_ROLE_SLOT),
    ]
    .into_iter()
    .map(
        |(output_slot, role_slot)| RoleBindingProfileActiveCenterRow {
            center_id: REAL_TRAFFIC_MIXED_OPERATOR_PAIR_BASE
                + mixed_request_operator_pair_lane(output_slot, role_slot),
            strength: 8,
        },
    )
    .collect()
}

fn mixed_request_operator_pair_lane(output_slot: u8, role_slot: u8) -> u32 {
    (u32::from(output_slot) << REAL_TRAFFIC_MIXED_OPERATOR_PAIR_SHIFT) | u32::from(role_slot)
}

fn mixed_role_surface_centers(
    role_slot: u8,
    token: &str,
) -> Vec<RoleBindingProfileActiveCenterRow> {
    let slot_base = REAL_TRAFFIC_MIXED_ROLE_BASE
        + u32::from(role_slot).saturating_mul(REAL_TRAFFIC_MIXED_PAGE_SIZE);
    surface_lane_centers_folded_for_profile(
        token,
        slot_base,
        REAL_TRAFFIC_MIXED_PAGE_SIZE,
        REAL_TRAFFIC_MIXED_TOP_ROLE_L1_LANES,
    )
}

fn mixed_request_score_slot(
    binding_output_slot: u8,
    correct_token: &str,
    wrong_token: &str,
) -> Option<RoleBindingProfileScoreSlotRow> {
    if correct_token == wrong_token {
        return None;
    }
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        REAL_TRAFFIC_MIXED_STATE_DELTA_LANES_PER_SIDE,
    );
    let negative_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        REAL_TRAFFIC_MIXED_STATE_DELTA_LANES_PER_SIDE,
    );
    if positive_impulses.is_empty() || negative_impulses.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreSlotRow {
        binding_output_slot: Some(binding_output_slot),
        positive_impulses,
        negative_impulses,
    })
}

fn extract_mixed_map_tokens(text: &str) -> Option<MixedMapTokens> {
    let tokens = extract_request_side_edit_tokens(text, 16);
    if tokens.len() < 2 {
        return None;
    }
    let lower = text.to_lowercase();
    let action_token = first_matching_branch_token(
        &lower,
        &[
            "запиши",
            "обнови",
            "перенеси",
            "перенос",
            "добавь",
            "сделай",
            "собери",
            "сохрани",
            "map",
            "mapping",
            "route",
            "operator",
            "оператор",
            "runtime",
            "traffic",
            "trace",
        ],
    )
    .unwrap_or_else(|| tokens[0].clone());
    let source_token = quoted_or_marked_chunks(text)
        .into_iter()
        .find_map(|chunk| {
            extract_request_side_edit_tokens(&chunk, 1)
                .into_iter()
                .next()
        })
        .or_else(|| {
            tokens
                .iter()
                .find(|token| token.as_str() != action_token)
                .cloned()
        })?;
    let destination_token = tokens
        .iter()
        .find(|token| {
            let lower = token.to_lowercase();
            token.contains('/')
                || token.contains('.')
                || contains_any(
                    &lower,
                    &[
                        "roadmap",
                        "docs",
                        "goal",
                        "план",
                        "правило",
                        "список",
                        "файл",
                        "памят",
                        "архитект",
                        "runtime",
                        "operator",
                        "оператор",
                    ],
                )
        })
        .cloned()
        .or_else(|| {
            tokens
                .iter()
                .rev()
                .find(|token| token.as_str() != source_token && token.as_str() != action_token)
                .cloned()
        })?;
    let invariant_token = tokens
        .iter()
        .find(|token| {
            let lower = token.to_lowercase();
            contains_any(
                &lower,
                &[
                    "operator",
                    "оператор",
                    "runtime",
                    "traffic",
                    "trace",
                    "goal",
                    "proof",
                    "архитект",
                    "route",
                    "map",
                ],
            )
        })
        .cloned()
        .unwrap_or_else(|| source_token.clone());
    if source_token == destination_token {
        return None;
    }
    Some(MixedMapTokens {
        source_token,
        destination_token,
        action_token,
        invariant_token,
    })
}

fn contains_marker_like_signal(text: &str) -> bool {
    text.contains('`')
        || text.contains('"')
        || text.contains('\'')
        || text.contains("```")
        || text.contains("->")
        || text.contains("=>")
}

fn contains_file_like_token(text: &str) -> bool {
    text.split(|ch: char| {
        ch.is_whitespace() || matches!(ch, ',' | ';' | ':' | '(' | ')' | '[' | ']')
    })
    .any(|token| {
        let token = token
            .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`' | '<' | '>' | '.' | ',' | ';'));
        contains_any(
            token,
            &[
                ".rs", ".py", ".md", ".json", ".jsonl", ".toml", ".yaml", ".yml", ".ts", ".tsx",
                ".js", ".jsx", ".html", ".css", ".sh", ".sql", ".txt",
            ],
        ) || token.contains('/')
    })
}

fn build_conditional_branch_dry_run_request(
    event_id: &str,
    fingerprint: &u64,
    candidate: &CodexHistoryRouteCandidate,
    text: &str,
) -> Option<RoleBindingProfileScoreRequest> {
    let branch = extract_conditional_branch_tokens(text)?;
    let mut active_fringe = Vec::new();
    active_fringe.extend(conditional_request_operator_centers());
    active_fringe.extend(conditional_role_surface_centers(
        REAL_TRAFFIC_CONDITIONAL_CONDITION_ROLE_SLOT,
        &branch.condition_token,
    ));
    active_fringe.extend(conditional_role_surface_centers(
        REAL_TRAFFIC_CONDITIONAL_EVIDENCE_ROLE_SLOT,
        &branch.evidence_token,
    ));
    active_fringe.extend(conditional_role_surface_centers(
        REAL_TRAFFIC_CONDITIONAL_ALLOWED_ROLE_SLOT,
        &branch.allowed_token,
    ));
    active_fringe.extend(conditional_role_surface_centers(
        REAL_TRAFFIC_CONDITIONAL_REFUSED_ROLE_SLOT,
        &branch.refused_token,
    ));
    let active_fringe = merge_profile_active_centers(active_fringe);
    let mut slots = Vec::new();
    if let Some(slot) =
        conditional_request_score_slot(0, &branch.allowed_token, &branch.refused_token)
    {
        slots.push(slot);
    }
    if let Some(slot) =
        conditional_request_score_slot(1, &branch.condition_token, &branch.refused_token)
    {
        slots.push(slot);
    }
    if active_fringe.is_empty() || slots.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreRequest {
        request_id: event_id.to_owned(),
        route_key: Some(candidate.route_key.clone()),
        profile_id: Some(candidate.profile_id.clone()),
        exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
        active_fringe,
        slots,
        // Dry-run only: response verification has not proven a safe local operator.
        expect_local_operator: Some(false),
    })
}

fn conditional_request_operator_centers() -> Vec<RoleBindingProfileActiveCenterRow> {
    [
        (0, REAL_TRAFFIC_CONDITIONAL_ALLOWED_ROLE_SLOT),
        (1, REAL_TRAFFIC_CONDITIONAL_CONDITION_ROLE_SLOT),
        (0, REAL_TRAFFIC_CONDITIONAL_EVIDENCE_ROLE_SLOT),
        (1, REAL_TRAFFIC_CONDITIONAL_REFUSED_ROLE_SLOT),
    ]
    .into_iter()
    .map(
        |(output_slot, role_slot)| RoleBindingProfileActiveCenterRow {
            center_id: REAL_TRAFFIC_CONDITIONAL_OPERATOR_PAIR_BASE
                + conditional_request_operator_pair_lane(output_slot, role_slot),
            strength: 8,
        },
    )
    .collect()
}

fn conditional_request_operator_pair_lane(output_slot: u8, role_slot: u8) -> u32 {
    (u32::from(output_slot) << REAL_TRAFFIC_CONDITIONAL_OPERATOR_PAIR_SHIFT) | u32::from(role_slot)
}

fn conditional_role_surface_centers(
    role_slot: u8,
    token: &str,
) -> Vec<RoleBindingProfileActiveCenterRow> {
    let slot_base = REAL_TRAFFIC_CONDITIONAL_ROLE_BASE
        + u32::from(role_slot).saturating_mul(REAL_TRAFFIC_CONDITIONAL_PAGE_SIZE);
    surface_lane_centers_folded_for_profile(
        token,
        slot_base,
        REAL_TRAFFIC_CONDITIONAL_PAGE_SIZE,
        REAL_TRAFFIC_CONDITIONAL_TOP_ROLE_L1_LANES,
    )
}

fn conditional_request_score_slot(
    binding_output_slot: u8,
    correct_token: &str,
    wrong_token: &str,
) -> Option<RoleBindingProfileScoreSlotRow> {
    if correct_token == wrong_token {
        return None;
    }
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        REAL_TRAFFIC_CONDITIONAL_STATE_DELTA_LANES_PER_SIDE,
    );
    let negative_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        REAL_TRAFFIC_CONDITIONAL_STATE_DELTA_LANES_PER_SIDE,
    );
    if positive_impulses.is_empty() || negative_impulses.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreSlotRow {
        binding_output_slot: Some(binding_output_slot),
        positive_impulses,
        negative_impulses,
    })
}

fn extract_conditional_branch_tokens(text: &str) -> Option<ConditionalBranchTokens> {
    let tokens = extract_request_side_edit_tokens(text, 12);
    if tokens.len() < 2 {
        return None;
    }
    let condition_token = quoted_or_marked_chunks(text)
        .into_iter()
        .find_map(|chunk| {
            extract_request_side_edit_tokens(&chunk, 1)
                .into_iter()
                .next()
        })
        .or_else(|| tokens.first().cloned())?;
    let lower = text.to_lowercase();
    let allowed_token = first_matching_branch_token(
        &lower,
        &[
            "pass",
            "accept",
            "allow",
            "true",
            "yes",
            "green",
            "ok",
            "принять",
            "разреш",
            "да",
        ],
    )
    .unwrap_or_else(|| condition_token.clone());
    let refused_token = first_matching_branch_token(
        &lower,
        &[
            "fallback",
            "fail",
            "reject",
            "refuse",
            "false",
            "no",
            "red",
            "block",
            "отклон",
            "запрет",
            "нет",
        ],
    )
    .or_else(|| {
        tokens
            .iter()
            .find(|token| token.as_str() != allowed_token)
            .cloned()
    })?;
    let evidence_token = tokens
        .iter()
        .find(|token| {
            let token_lower = token.to_lowercase();
            contains_any(
                &token_lower,
                &[
                    "report",
                    "trace",
                    "gate",
                    "audit",
                    "p99",
                    "margin",
                    "false_accept",
                    "verdict",
                ],
            ) || token.contains('/')
                || token.contains('.')
        })
        .cloned()
        .unwrap_or_else(|| condition_token.clone());
    if allowed_token == refused_token {
        return None;
    }
    Some(ConditionalBranchTokens {
        condition_token,
        evidence_token,
        allowed_token,
        refused_token,
    })
}

fn first_matching_branch_token(text: &str, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| text.contains(**candidate))
        .map(|candidate| (*candidate).to_owned())
}

fn build_edit_marker_length_dry_run_request(
    event_id: &str,
    fingerprint: &u64,
    candidate: &CodexHistoryRouteCandidate,
    text: &str,
) -> Option<RoleBindingProfileScoreRequest> {
    let tokens =
        extract_request_side_edit_tokens(text, usize::from(REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT));
    let marker = extract_request_side_marker_token(text, &tokens)?;
    let wrong_token = tokens
        .iter()
        .find(|token| token.as_str() != marker)
        .cloned()
        .unwrap_or_else(|| REAL_TRAFFIC_EDIT_END_TOKEN.to_owned());
    let mut active_fringe = Vec::new();
    active_fringe.extend(edit_request_operator_centers());
    for (slot_id, token) in tokens
        .iter()
        .take(usize::from(REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT))
        .enumerate()
    {
        let slot_base = REAL_TRAFFIC_EDIT_ROLE_BASE
            + (slot_id as u32).saturating_mul(REAL_TRAFFIC_EDIT_PAGE_SIZE);
        active_fringe.extend(surface_lane_centers_folded_for_profile(
            token,
            slot_base,
            REAL_TRAFFIC_EDIT_PAGE_SIZE,
            REAL_TRAFFIC_EDIT_TOP_ROLE_L1_LANES,
        ));
    }
    let marker_slot_base = REAL_TRAFFIC_EDIT_ROLE_BASE
        + u32::from(REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT) * REAL_TRAFFIC_EDIT_PAGE_SIZE;
    active_fringe.extend(surface_lane_centers_folded_for_profile(
        &marker,
        marker_slot_base,
        REAL_TRAFFIC_EDIT_PAGE_SIZE,
        REAL_TRAFFIC_EDIT_TOP_ROLE_L1_LANES,
    ));
    active_fringe.extend(surface_lane_centers_folded_for_profile(
        REAL_TRAFFIC_EDIT_END_TOKEN,
        marker_slot_base,
        REAL_TRAFFIC_EDIT_PAGE_SIZE,
        REAL_TRAFFIC_EDIT_TOP_ROLE_L1_LANES,
    ));
    let active_fringe = merge_profile_active_centers(active_fringe);
    let mut slots = Vec::new();
    if let Some(slot) = edit_request_score_slot(0, &marker, &wrong_token) {
        slots.push(slot);
    }
    if let Some(slot) = edit_request_score_slot(1, REAL_TRAFFIC_EDIT_END_TOKEN, &marker) {
        slots.push(slot);
    }
    if active_fringe.is_empty() || slots.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreRequest {
        request_id: event_id.to_owned(),
        route_key: Some(candidate.route_key.clone()),
        profile_id: Some(candidate.profile_id.clone()),
        exact_cache_key: Some(format!("codex_history_request:{fingerprint:016x}")),
        active_fringe,
        slots,
        // Dry-run only: any local accept is intentionally counted as false/unverified.
        expect_local_operator: Some(false),
    })
}

fn edit_request_operator_centers() -> Vec<RoleBindingProfileActiveCenterRow> {
    (0..REAL_TRAFFIC_EDIT_OUTPUT_SLOT_COUNT)
        .map(|output_slot| RoleBindingProfileActiveCenterRow {
            center_id: REAL_TRAFFIC_EDIT_DEMO_BASE
                + edit_request_operator_pair_lane(output_slot, REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT),
            strength: 8,
        })
        .collect()
}

fn edit_request_operator_pair_lane(output_slot: u8, role_slot: u8) -> u32 {
    (u32::from(output_slot) << REAL_TRAFFIC_EDIT_OPERATOR_PAIR_SHIFT) | u32::from(role_slot)
}

fn edit_request_score_slot(
    binding_output_slot: u8,
    correct_token: &str,
    wrong_token: &str,
) -> Option<RoleBindingProfileScoreSlotRow> {
    if correct_token == wrong_token {
        return None;
    }
    let base_wave = SurfaceWave4096::compile("");
    let target_wave = SurfaceWave4096::compile(correct_token);
    let wrong_wave = SurfaceWave4096::compile(wrong_token);
    let positive_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        target_wave.lanes(),
        wrong_wave.lanes(),
        REAL_TRAFFIC_EDIT_STATE_DELTA_LANES_PER_SIDE,
    );
    let negative_impulses = discriminative_profile_impulses(
        base_wave.lanes(),
        wrong_wave.lanes(),
        target_wave.lanes(),
        REAL_TRAFFIC_EDIT_STATE_DELTA_LANES_PER_SIDE,
    );
    if positive_impulses.is_empty() || negative_impulses.is_empty() {
        return None;
    }
    Some(RoleBindingProfileScoreSlotRow {
        binding_output_slot: Some(binding_output_slot),
        positive_impulses,
        negative_impulses,
    })
}

fn surface_lane_centers_folded_for_profile(
    input: &str,
    center_base: u32,
    center_span: u32,
    limit: usize,
) -> Vec<RoleBindingProfileActiveCenterRow> {
    let wave = SurfaceWave4096::compile(input);
    let mut by_lane: BTreeMap<u32, i16> = BTreeMap::new();
    for (lane, value) in wave.lanes().iter().enumerate() {
        let magnitude = value.abs();
        if magnitude == 0 {
            continue;
        }
        let folded_lane = lane as u32 % center_span;
        by_lane
            .entry(folded_lane)
            .and_modify(|current| *current = (*current).max(magnitude))
            .or_insert(magnitude);
    }
    let mut lanes = by_lane
        .into_iter()
        .map(|(lane, magnitude)| (i32::from(magnitude), lane))
        .collect::<Vec<_>>();
    lanes.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    lanes
        .into_iter()
        .take(limit)
        .map(|(magnitude, lane)| RoleBindingProfileActiveCenterRow {
            center_id: center_base + lane,
            strength: (magnitude as i16).clamp(1, 8),
        })
        .collect()
}

fn merge_profile_active_centers(
    centers: Vec<RoleBindingProfileActiveCenterRow>,
) -> Vec<RoleBindingProfileActiveCenterRow> {
    let mut by_center = BTreeMap::<u32, i16>::new();
    for center in centers {
        by_center
            .entry(center.center_id)
            .and_modify(|strength| *strength = (*strength).max(center.strength))
            .or_insert(center.strength);
    }
    by_center
        .into_iter()
        .map(|(center_id, strength)| RoleBindingProfileActiveCenterRow {
            center_id,
            strength,
        })
        .collect()
}

fn discriminative_profile_impulses(
    base: &[i16; SURFACE_WAVE_DIM],
    wanted: &[i16; SURFACE_WAVE_DIM],
    other: &[i16; SURFACE_WAVE_DIM],
    cap: usize,
) -> Vec<RoleBindingProfileImpulseRow> {
    let mut candidates = Vec::new();
    for lane in 0..SURFACE_WAVE_DIM {
        let wanted_delta = wanted[lane].saturating_sub(base[lane]);
        if wanted_delta == 0 {
            continue;
        }
        let other_delta = other[lane].saturating_sub(base[lane]);
        let wanted_abs = i32::from(wanted_delta).abs();
        let other_abs = i32::from(other_delta).abs();
        let same_direction = wanted_delta.signum() == other_delta.signum();
        if same_direction && wanted_abs <= other_abs {
            continue;
        }
        let separation = if same_direction {
            wanted_abs - other_abs
        } else {
            wanted_abs + other_abs
        };
        candidates.push((
            separation,
            lane as u16,
            clamp_profile_impulse_strength(wanted_delta),
        ));
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates
        .into_iter()
        .take(cap)
        .map(
            |(_, lane_id, signed_strength)| RoleBindingProfileImpulseRow {
                lane_id,
                signed_strength,
            },
        )
        .collect()
}

fn clamp_profile_impulse_strength(value: i16) -> i16 {
    let sign = if value < 0 { -1 } else { 1 };
    let magnitude = i32::from(value).abs().clamp(1, 8) as i16;
    sign * magnitude
}

fn extract_request_side_marker_token(text: &str, tokens: &[String]) -> Option<String> {
    for chunk in quoted_or_marked_chunks(text) {
        if let Some(token) = extract_request_side_edit_tokens(&chunk, 1)
            .into_iter()
            .next()
        {
            return Some(token);
        }
    }
    tokens
        .iter()
        .find(|token| token.contains('/') || token.contains('.') || token.contains('_'))
        .cloned()
        .or_else(|| tokens.first().cloned())
}

fn quoted_or_marked_chunks(text: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    for delimiter in ['`', '"', '\''] {
        let parts = text.split(delimiter).collect::<Vec<_>>();
        for index in (1..parts.len()).step_by(2) {
            let chunk = parts[index].trim();
            if !chunk.is_empty() {
                chunks.push(chunk.to_owned());
            }
        }
    }
    chunks
}

fn extract_request_side_edit_tokens(text: &str, limit: usize) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut seen = HashSet::new();
    for raw in
        text.split(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':')))
    {
        let token = raw
            .trim_matches(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | '`' | '<' | '>' | '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']'
                )
            })
            .to_lowercase();
        if token.len() < 2 || token.len() > 96 || is_low_information_edit_token(&token) {
            continue;
        }
        if seen.insert(token.clone()) {
            tokens.push(token);
        }
        if tokens.len() >= limit {
            break;
        }
    }
    tokens
}

fn is_low_information_edit_token(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "this"
            | "that"
            | "что"
            | "это"
            | "как"
            | "для"
            | "или"
            | "надо"
            | "нужно"
            | "сейчас"
            | "тут"
            | "там"
            | "вот"
            | "пожалуйста"
            | "давай"
            | "делай"
            | "сделай"
    )
}

fn real_traffic_operator_rankings(
    accumulators: BTreeMap<String, RoleBindingRealTrafficOperatorAccumulator>,
    total_operator_candidate_calls: usize,
) -> Vec<RoleBindingRealTrafficOperatorRanking> {
    let mut rankings = accumulators
        .into_iter()
        .map(|(operator_key, accumulator)| {
            let p99_latency_ns = percentile(&accumulator.latencies_ns, 99);
            let value_score_microusd_per_ms = if accumulator.false_accepts == 0
                && accumulator.unverified_shadow_accepts == 0
                && p99_latency_ns > 0
            {
                accumulator
                    .estimated_cost_saved_microusd
                    .saturating_mul(1_000_000)
                    .checked_div(p99_latency_ns)
                    .unwrap_or(0)
            } else {
                0
            };
            RoleBindingRealTrafficOperatorRanking {
                operator_key,
                route_key: accumulator.route_key,
                profile_id: accumulator.profile_id,
                action: accumulator.action,
                candidate_calls: accumulator.candidate_calls,
                traffic_share_milli: ratio_milli(
                    accumulator.candidate_calls,
                    total_operator_candidate_calls,
                ),
                llm_calls: accumulator.llm_calls,
                exact_cache_hits: accumulator.exact_cache_hits,
                nando_shadow_accepts: accumulator.nando_shadow_accepts,
                verified_safe_accepts: accumulator.verified_safe_accepts,
                unverified_shadow_accepts: accumulator.unverified_shadow_accepts,
                false_accepts: accumulator.false_accepts,
                incremental_savings_over_exact_cache: accumulator
                    .incremental_savings_over_exact_cache,
                estimated_cost_saved_microusd: accumulator.estimated_cost_saved_microusd,
                local_accept_rate_milli: ratio_milli(
                    accumulator.nando_shadow_accepts,
                    accumulator.candidate_calls,
                ),
                verified_accept_rate_milli: ratio_milli(
                    accumulator.verified_safe_accepts,
                    accumulator.candidate_calls,
                ),
                p99_shadow_score_latency_ns: p99_latency_ns,
                value_score_microusd_per_ms,
            }
        })
        .collect::<Vec<_>>();
    rankings.sort_by(|left, right| {
        right
            .value_score_microusd_per_ms
            .cmp(&left.value_score_microusd_per_ms)
            .then_with(|| {
                right
                    .incremental_savings_over_exact_cache
                    .cmp(&left.incremental_savings_over_exact_cache)
            })
            .then_with(|| right.candidate_calls.cmp(&left.candidate_calls))
            .then_with(|| left.operator_key.cmp(&right.operator_key))
    });
    rankings
}

fn validate_real_traffic_event_row(row: &RoleBindingRealTrafficEventRow) -> Result<(), String> {
    if row.schema_version != "nando_role_binding_real_traffic_event_v1" {
        return Err(format!(
            "unsupported real-traffic event schema_version '{}'",
            row.schema_version
        ));
    }
    if row.event_id.trim().is_empty() {
        return Err("real-traffic event row has empty event_id".to_owned());
    }
    if !row.llm_call && row.provider_cost_microusd.unwrap_or(0) > 0 {
        return Err(format!(
            "event_id={} has provider_cost_microusd but llm_call=false",
            row.event_id
        ));
    }
    if row.verified_safe_accept.is_some() && row.nando_shadow_request.is_none() {
        return Err(format!(
            "event_id={} has verified_safe_accept without nando_shadow_request",
            row.event_id
        ));
    }
    if row.verified_safe_accept.is_some() && !event_row_has_output_evidence(row) {
        return Err(format!(
            "event_id={} has verified_safe_accept without response/tool-call evidence",
            row.event_id
        ));
    }
    if row.verified_safe_accept.is_some() && !nonempty_option(&row.verification_source) {
        return Err(format!(
            "event_id={} has verified_safe_accept without verification_source",
            row.event_id
        ));
    }
    Ok(())
}

fn validate_real_traffic_trace_row(row: &RoleBindingRealTrafficTraceRow) -> Result<(), String> {
    if row.schema_version != "nando_role_binding_real_traffic_trace_v1" {
        return Err(format!(
            "unsupported real-traffic trace schema_version '{}'",
            row.schema_version
        ));
    }
    if row.trace_id.trim().is_empty() {
        return Err("real-traffic trace row has empty trace_id".to_owned());
    }
    if !row.llm_call && row.provider_cost_microusd.unwrap_or(0) > 0 {
        return Err(format!(
            "trace_id={} has provider_cost_microusd but llm_call=false",
            row.trace_id
        ));
    }
    if row.verified_safe_accept.is_some() && row.nando_shadow_request.is_none() {
        return Err(format!(
            "trace_id={} has verified_safe_accept without nando_shadow_request",
            row.trace_id
        ));
    }
    if row.verified_safe_accept.is_some() && !trace_row_has_output_evidence(row) {
        return Err(format!(
            "trace_id={} has verified_safe_accept without response/tool-call evidence",
            row.trace_id
        ));
    }
    if row.verified_safe_accept.is_some() && !nonempty_option(&row.verification_source) {
        return Err(format!(
            "trace_id={} has verified_safe_accept without verification_source",
            row.trace_id
        ));
    }
    Ok(())
}

fn event_row_has_output_evidence(row: &RoleBindingRealTrafficEventRow) -> bool {
    nonempty_option(&row.response_fingerprint) || !row.tool_call_fingerprints.is_empty()
}

fn trace_row_has_output_evidence(row: &RoleBindingRealTrafficTraceRow) -> bool {
    nonempty_option(&row.response_fingerprint) || !row.tool_call_fingerprints.is_empty()
}

fn codex_history_session_id_from_trace_id(trace_id: &str) -> Option<String> {
    let rest = trace_id
        .strip_prefix("codex_history_edit_payload_dry_run::")
        .or_else(|| trace_id.strip_prefix("codex_history_conditional_payload_dry_run::"))
        .or_else(|| trace_id.strip_prefix("codex_history_mixed_payload_dry_run::"))
        .or_else(|| trace_id.strip_prefix("codex_history_planning_next_step_payload_dry_run::"))
        .or_else(|| trace_id.strip_prefix("codex_history_agent_control_payload_dry_run::"))?;
    let (without_index, _) = rest.rsplit_once("::")?;
    let (session_id, _) = without_index.rsplit_once("::")?;
    Some(session_id.to_owned())
}

fn build_codex_session_output_evidence_index(
    sessions_root: &Path,
    session_ids: &HashSet<String>,
    wanted_request_fingerprints: &HashSet<String>,
    verifier: fn(&str, &str) -> (bool, bool, String),
) -> Result<CodexSessionOutputEvidenceIndex, String> {
    let mut session_files = Vec::new();
    collect_codex_session_jsonl_files(sessions_root, session_ids, &mut session_files)?;
    let mut by_request_fingerprint = BTreeMap::new();
    let mut codex_turns_indexed = 0usize;
    for session_file in &session_files {
        let text = fs::read_to_string(session_file).map_err(|error| {
            format!(
                "failed to read Codex session JSONL {}: {error}",
                session_file.display()
            )
        })?;
        let mut pending_request_fingerprint: Option<String> = None;
        let mut pending_prompt_text: Option<String> = None;
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse Codex session JSONL {} line {}: {error}",
                    session_file.display(),
                    line_index + 1
                )
            })?;
            if value.get("type").and_then(serde_json::Value::as_str) != Some("event_msg") {
                continue;
            }
            let Some(payload) = value.get("payload") else {
                continue;
            };
            match payload.get("type").and_then(serde_json::Value::as_str) {
                Some("user_message") => {
                    if let Some(message) =
                        payload.get("message").and_then(serde_json::Value::as_str)
                    {
                        let fingerprint = format!(
                            "fnv1a64:{:016x}",
                            stable_real_traffic_fingerprint64(message.as_bytes())
                        );
                        pending_prompt_text = Some(message.to_owned());
                        pending_request_fingerprint = Some(fingerprint);
                    }
                }
                Some("agent_message")
                    if payload.get("phase").and_then(serde_json::Value::as_str)
                        == Some("final_answer") =>
                {
                    let Some(request_fingerprint) = pending_request_fingerprint.take() else {
                        continue;
                    };
                    let Some(prompt_text) = pending_prompt_text.take() else {
                        continue;
                    };
                    if !wanted_request_fingerprints.contains(&request_fingerprint) {
                        continue;
                    }
                    let Some(response_text) =
                        payload.get("message").and_then(serde_json::Value::as_str)
                    else {
                        continue;
                    };
                    let response_fingerprint = format!(
                        "fnv1a64:{:016x}",
                        stable_real_traffic_fingerprint64(response_text.as_bytes())
                    );
                    let (verified_safe_accept, verifier_applicable, verifier_status) =
                        verifier(&prompt_text, response_text);
                    by_request_fingerprint.entry(request_fingerprint).or_insert(
                        CodexSessionOutputEvidence {
                            response_fingerprint,
                            verified_safe_accept,
                            verifier_applicable,
                            verifier_status,
                        },
                    );
                    codex_turns_indexed += 1;
                }
                Some("task_complete") => {
                    if let Some(response_text) = payload
                        .get("last_agent_message")
                        .and_then(serde_json::Value::as_str)
                    {
                        let Some(request_fingerprint) = pending_request_fingerprint.take() else {
                            pending_prompt_text = None;
                            continue;
                        };
                        let Some(prompt_text) = pending_prompt_text.take() else {
                            continue;
                        };
                        if wanted_request_fingerprints.contains(&request_fingerprint) {
                            let response_fingerprint = format!(
                                "fnv1a64:{:016x}",
                                stable_real_traffic_fingerprint64(response_text.as_bytes())
                            );
                            let (verified_safe_accept, verifier_applicable, verifier_status) =
                                verifier(&prompt_text, response_text);
                            by_request_fingerprint.entry(request_fingerprint).or_insert(
                                CodexSessionOutputEvidence {
                                    response_fingerprint,
                                    verified_safe_accept,
                                    verifier_applicable,
                                    verifier_status,
                                },
                            );
                            codex_turns_indexed += 1;
                        }
                    }
                    pending_request_fingerprint = None;
                    pending_prompt_text = None;
                }
                _ => {}
            }
        }
    }
    Ok(CodexSessionOutputEvidenceIndex {
        by_request_fingerprint,
        session_files_scanned: session_files.len(),
        codex_turns_indexed,
    })
}

fn build_codex_session_planning_artifact_progress_index(
    sessions_root: &Path,
    session_ids: &HashSet<String>,
    wanted_request_fingerprints: &HashSet<String>,
) -> Result<CodexSessionPlanningArtifactProgressIndex, String> {
    let mut session_files = Vec::new();
    collect_codex_session_jsonl_files(sessions_root, session_ids, &mut session_files)?;
    let mut by_request_fingerprint = BTreeMap::new();
    let mut codex_turns_indexed = 0usize;
    let mut tool_events_indexed = 0usize;

    for session_file in &session_files {
        let text = fs::read_to_string(session_file).map_err(|error| {
            format!(
                "failed to read Codex session JSONL {}: {error}",
                session_file.display()
            )
        })?;
        let mut pending_request_fingerprint: Option<String> = None;
        let mut pending_prompt_text: Option<String> = None;
        let mut pending_artifacts = PlanningArtifactTurnEvidence::default();

        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value = serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
                format!(
                    "failed to parse Codex session JSONL {} line {}: {error}",
                    session_file.display(),
                    line_index + 1
                )
            })?;
            match value.get("type").and_then(serde_json::Value::as_str) {
                Some("event_msg") => {
                    let Some(payload) = value.get("payload") else {
                        continue;
                    };
                    match payload.get("type").and_then(serde_json::Value::as_str) {
                        Some("user_message") => {
                            if let Some(message) =
                                payload.get("message").and_then(serde_json::Value::as_str)
                            {
                                let fingerprint = format!(
                                    "fnv1a64:{:016x}",
                                    stable_real_traffic_fingerprint64(message.as_bytes())
                                );
                                pending_prompt_text = Some(message.to_owned());
                                pending_request_fingerprint = Some(fingerprint);
                                pending_artifacts = PlanningArtifactTurnEvidence::default();
                            }
                        }
                        Some("agent_message")
                            if payload.get("phase").and_then(serde_json::Value::as_str)
                                == Some("final_answer") =>
                        {
                            let Some(response_text) =
                                payload.get("message").and_then(serde_json::Value::as_str)
                            else {
                                continue;
                            };
                            if finalize_planning_artifact_turn(
                                &mut by_request_fingerprint,
                                wanted_request_fingerprints,
                                pending_request_fingerprint.take(),
                                pending_prompt_text.take(),
                                Some(response_text),
                                std::mem::take(&mut pending_artifacts),
                            ) {
                                codex_turns_indexed += 1;
                            }
                        }
                        Some("task_complete") => {
                            let response_text = payload
                                .get("last_agent_message")
                                .and_then(serde_json::Value::as_str);
                            if finalize_planning_artifact_turn(
                                &mut by_request_fingerprint,
                                wanted_request_fingerprints,
                                pending_request_fingerprint.take(),
                                pending_prompt_text.take(),
                                response_text,
                                std::mem::take(&mut pending_artifacts),
                            ) {
                                codex_turns_indexed += 1;
                            }
                        }
                        _ => {}
                    }
                }
                Some("response_item") => {
                    let Some(payload) = value.get("payload") else {
                        continue;
                    };
                    if pending_request_fingerprint
                        .as_ref()
                        .is_some_and(|fingerprint| {
                            wanted_request_fingerprints.contains(fingerprint)
                        })
                    {
                        tool_events_indexed += usize::from(track_planning_artifact_tool_event(
                            payload,
                            &mut pending_artifacts,
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(CodexSessionPlanningArtifactProgressIndex {
        by_request_fingerprint,
        session_files_scanned: session_files.len(),
        codex_turns_indexed,
        tool_events_indexed,
    })
}

fn finalize_planning_artifact_turn(
    by_request_fingerprint: &mut BTreeMap<String, CodexSessionPlanningArtifactProgressEvidence>,
    wanted_request_fingerprints: &HashSet<String>,
    request_fingerprint: Option<String>,
    prompt_text: Option<String>,
    response_text: Option<&str>,
    artifact_evidence: PlanningArtifactTurnEvidence,
) -> bool {
    let Some(request_fingerprint) = request_fingerprint else {
        return false;
    };
    if !wanted_request_fingerprints.contains(&request_fingerprint) {
        return false;
    }
    let Some(prompt_text) = prompt_text else {
        return false;
    };
    let response_fingerprint = response_text.map(|text| {
        format!(
            "fnv1a64:{:016x}",
            stable_real_traffic_fingerprint64(text.as_bytes())
        )
    });
    let (verified_safe_accept, verifier_applicable, verifier_status) =
        deterministic_planning_artifact_progress_verification(
            &prompt_text,
            response_text.unwrap_or(""),
            &artifact_evidence,
        );
    let tool_call_fingerprints = artifact_evidence
        .progress_tool_fingerprints
        .into_iter()
        .chain(artifact_evidence.validation_tool_fingerprints)
        .collect::<Vec<_>>();
    by_request_fingerprint.entry(request_fingerprint).or_insert(
        CodexSessionPlanningArtifactProgressEvidence {
            response_fingerprint,
            tool_call_fingerprints,
            verified_safe_accept,
            verifier_applicable,
            verifier_status,
        },
    );
    true
}

fn track_planning_artifact_tool_event(
    payload: &serde_json::Value,
    artifact_evidence: &mut PlanningArtifactTurnEvidence,
) -> bool {
    match payload.get("type").and_then(serde_json::Value::as_str) {
        Some("function_call") | Some("custom_tool_call") => {
            let call_id = payload
                .get("call_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if call_id.is_empty() {
                return false;
            }
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let detail = payload
                .get("arguments")
                .or_else(|| payload.get("input"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if let Some(kind) = planning_artifact_tool_kind(name, detail) {
                artifact_evidence
                    .pending_tool_kinds
                    .insert(call_id.to_owned(), kind);
            }
            false
        }
        Some("function_call_output") | Some("custom_tool_call_output") => {
            let call_id = payload
                .get("call_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let Some(kind) = artifact_evidence.pending_tool_kinds.remove(call_id) else {
                return false;
            };
            let output = payload
                .get("output")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !planning_artifact_tool_output_success(output) {
                return false;
            }
            let fingerprint = format!(
                "toolfnv1a64:{:016x}",
                stable_real_traffic_fingerprint64(format!("{kind}:{call_id}:{output}").as_bytes())
            );
            if kind.starts_with("validation_") {
                artifact_evidence.successful_validation_kinds.insert(kind);
                artifact_evidence
                    .validation_tool_fingerprints
                    .push(fingerprint);
            } else {
                artifact_evidence.successful_progress_kinds.insert(kind);
                artifact_evidence
                    .progress_tool_fingerprints
                    .push(fingerprint);
            }
            true
        }
        _ => false,
    }
}

fn planning_artifact_tool_kind(name: &str, detail: &str) -> Option<String> {
    let name = name.trim();
    let lower = detail.to_lowercase();
    let touches_nando_wave = contains_any(
        &lower,
        &[
            "/home/ubu/projects/nando-wave",
            "projects/nando-wave",
            "target/nando-wave",
            "docs/executor_review_notes.md",
            "docs/structural_gates",
            "crates/nando-cli",
            "crates/nando-core",
        ],
    );

    if name == "apply_patch" && touches_nando_wave {
        return Some("progress_apply_patch_nando_wave".to_owned());
    }
    if name != "exec_command" {
        return None;
    }
    if !touches_nando_wave && !contains_any(&lower, &["cargo ", "git ", "nanda-gate"]) {
        return None;
    }
    if lower.contains("git commit") {
        return Some("progress_git_commit".to_owned());
    }
    if lower.contains("cargo run -p nando-cli") && lower.contains("role-binding-real-traffic") {
        return Some("progress_real_traffic_report_generation".to_owned());
    }
    if lower.contains("nanda-gate") || lower.contains("nanda-check") {
        return Some("progress_structural_gate_artifact".to_owned());
    }
    if lower.contains("> target/nando-wave") || lower.contains("write_json_file") {
        return Some("progress_generated_target_artifact".to_owned());
    }
    if lower.contains("cargo check") {
        return Some("validation_cargo_check".to_owned());
    }
    if lower.contains("cargo clippy") {
        return Some("validation_cargo_clippy".to_owned());
    }
    if lower.contains("cargo fmt") {
        return Some("validation_cargo_fmt".to_owned());
    }
    if lower.contains("git diff --check") {
        return Some("validation_git_diff_check".to_owned());
    }
    None
}

fn planning_artifact_tool_output_success(output: &str) -> bool {
    let lower = output.to_lowercase();
    contains_any(
        &lower,
        &[
            "process exited with code 0",
            "exit code: 0",
            "success. updated",
            "success. added",
        ],
    )
}

fn deterministic_planning_artifact_progress_verification(
    prompt_text: &str,
    response_text: &str,
    artifact_evidence: &PlanningArtifactTurnEvidence,
) -> (bool, bool, String) {
    let readiness =
        analyze_route_gap_payload_readiness(REAL_TRAFFIC_PLANNING_ROUTE_KEY, prompt_text);
    if !readiness.payload_ready {
        return (
            false,
            false,
            format!(
                "not_applicable_readiness_missing:{}",
                readiness.missing_reasons.join(",")
            ),
        );
    }
    if extract_planning_next_step_tokens(prompt_text).is_none() {
        return (
            false,
            false,
            "not_applicable_missing_planning_tokens".to_owned(),
        );
    }
    let response_lower = response_text.to_lowercase();
    if contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "not enough evidence",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    ) {
        return (false, true, "rejected_response_reports_failure".to_owned());
    }
    if !artifact_evidence.successful_progress_kinds.is_empty() {
        return (
            true,
            true,
            format!(
                "verified_tool_backed_project_progress:{}",
                artifact_evidence
                    .successful_progress_kinds
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("+")
            ),
        );
    }
    if !artifact_evidence.successful_validation_kinds.is_empty() {
        return (
            false,
            true,
            format!(
                "rejected_validation_only_without_project_progress:{}",
                artifact_evidence
                    .successful_validation_kinds
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("+")
            ),
        );
    }
    (
        false,
        true,
        "rejected_no_successful_project_progress_tool".to_owned(),
    )
}

fn collect_codex_session_jsonl_files(
    root: &Path,
    session_ids: &HashSet<String>,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if session_ids.is_empty() {
        return Ok(());
    }
    let entries = fs::read_dir(root)
        .map_err(|error| format!("failed to read sessions root {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read sessions root entry {}: {error}",
                root.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            format!(
                "failed to read sessions entry type {}: {error}",
                path.display()
            )
        })?;
        if file_type.is_dir() {
            collect_codex_session_jsonl_files(&path, session_ids, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    session_ids
                        .iter()
                        .any(|session_id| name.contains(session_id))
                })
        {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}

fn deterministic_edit_output_verification(
    prompt_text: &str,
    response_text: &str,
) -> (bool, bool, String) {
    let readiness = analyze_edit_payload_readiness(prompt_text);
    if !readiness.payload_ready {
        return (
            false,
            false,
            format!(
                "not_applicable_readiness_missing:{}",
                readiness.missing_reasons.join(",")
            ),
        );
    }
    let tokens = extract_request_side_edit_tokens(
        prompt_text,
        usize::from(REAL_TRAFFIC_EDIT_MARKER_ROLE_SLOT),
    );
    let Some(marker) = extract_request_side_marker_token(prompt_text, &tokens) else {
        return (false, false, "not_applicable_missing_marker".to_owned());
    };
    if marker.chars().count() < 3 {
        return (false, false, "not_applicable_marker_too_short".to_owned());
    }
    let response_lower = response_text.to_lowercase();
    let marker_lower = marker.to_lowercase();
    let marker_present = response_lower.contains(&marker_lower);
    let refusal_or_failure = contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    );
    let edit_confirmation = contains_any(
        &response_lower,
        &[
            "```",
            "diff",
            "patch",
            "updated",
            "changed",
            "fixed",
            "rewrote",
            "исправ",
            "обнов",
            "замен",
            "перепис",
            "готов",
        ],
    );
    let verified = marker_present && edit_confirmation && !refusal_or_failure;
    let status = if verified {
        "verified_marker_present_with_edit_confirmation"
    } else if !marker_present {
        "rejected_marker_absent_from_response"
    } else if refusal_or_failure {
        "rejected_response_reports_failure"
    } else {
        "rejected_no_edit_confirmation"
    };
    (verified, true, status.to_owned())
}

fn deterministic_conditional_output_verification(
    prompt_text: &str,
    response_text: &str,
) -> (bool, bool, String) {
    let readiness = analyze_conditional_payload_readiness(prompt_text);
    if !readiness.payload_ready {
        return (
            false,
            false,
            format!(
                "not_applicable_readiness_missing:{}",
                readiness.missing_reasons.join(",")
            ),
        );
    }
    let Some(branch) = extract_conditional_branch_tokens(prompt_text) else {
        return (
            false,
            false,
            "not_applicable_missing_branch_tokens".to_owned(),
        );
    };
    let response_lower = response_text.to_lowercase();
    let allowed_present = response_contains_branch_token(&response_lower, &branch.allowed_token);
    let refused_present = response_contains_branch_token(&response_lower, &branch.refused_token);
    let condition_present =
        response_contains_branch_token(&response_lower, &branch.condition_token)
            || contains_any(
                &response_lower,
                &["if", "when", "condition", "branch", "если", "услов"],
            );
    let evidence_present = response_contains_branch_token(&response_lower, &branch.evidence_token)
        || contains_any(
            &response_lower,
            &[
                "evidence",
                "report",
                "verdict",
                "result",
                "trace",
                "audit",
                "доказ",
                "отч",
                "вердикт",
                "результат",
            ],
        );
    let refusal_or_failure = contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "not enough evidence",
            "недостаточно",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    );
    let exactly_one_branch = allowed_present ^ refused_present;
    let verified =
        exactly_one_branch && (condition_present || evidence_present) && !refusal_or_failure;
    let status = if verified {
        "verified_single_branch_with_condition_or_evidence"
    } else if allowed_present && refused_present {
        "rejected_ambiguous_both_branches_present"
    } else if !allowed_present && !refused_present {
        "rejected_no_branch_token_in_response"
    } else if refusal_or_failure {
        "rejected_response_reports_failure"
    } else {
        "rejected_missing_condition_or_evidence_signal"
    };
    (verified, true, status.to_owned())
}

fn deterministic_mixed_output_verification(
    prompt_text: &str,
    response_text: &str,
) -> (bool, bool, String) {
    let readiness = analyze_mixed_payload_readiness(prompt_text);
    if !readiness.payload_ready {
        return (
            false,
            false,
            format!(
                "not_applicable_readiness_missing:{}",
                readiness.missing_reasons.join(",")
            ),
        );
    }
    let Some(tokens) = extract_mixed_map_tokens(prompt_text) else {
        return (
            false,
            false,
            "not_applicable_missing_mixed_tokens".to_owned(),
        );
    };
    let response_lower = response_text.to_lowercase();
    let action_present = response_contains_branch_token(&response_lower, &tokens.action_token)
        || contains_any(
            &response_lower,
            &[
                "updated",
                "mapped",
                "moved",
                "recorded",
                "wrote",
                "обнов",
                "запис",
                "перен",
                "собран",
                "готов",
            ],
        );
    let destination_present =
        response_contains_branch_token(&response_lower, &tokens.destination_token);
    let invariant_present =
        response_contains_branch_token(&response_lower, &tokens.invariant_token)
            || contains_any(
                &response_lower,
                &[
                    "operator",
                    "оператор",
                    "runtime",
                    "trace",
                    "route",
                    "map",
                    "mapping",
                ],
            );
    let refusal_or_failure = contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "not enough evidence",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    );
    let verified =
        action_present && (destination_present || invariant_present) && !refusal_or_failure;
    let status = if verified {
        "verified_mixed_action_with_destination_or_invariant"
    } else if refusal_or_failure {
        "rejected_response_reports_failure"
    } else if !action_present {
        "rejected_action_absent_from_response"
    } else {
        "rejected_destination_and_invariant_absent_from_response"
    };
    (verified, true, status.to_owned())
}

fn deterministic_planning_next_step_output_verification(
    prompt_text: &str,
    response_text: &str,
) -> (bool, bool, String) {
    let readiness =
        analyze_route_gap_payload_readiness(REAL_TRAFFIC_PLANNING_ROUTE_KEY, prompt_text);
    if !readiness.payload_ready {
        return (
            false,
            false,
            format!(
                "not_applicable_readiness_missing:{}",
                readiness.missing_reasons.join(",")
            ),
        );
    }
    if extract_planning_next_step_tokens(prompt_text).is_none() {
        return (
            false,
            false,
            "not_applicable_missing_planning_tokens".to_owned(),
        );
    }

    let response_lower = response_text.to_lowercase();
    if contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "not enough evidence",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    ) {
        return (false, true, "rejected_response_reports_failure".to_owned());
    }

    let mentions_artifact_or_check = contains_any(
        &response_lower,
        &[
            "target/nando-wave",
            "docs/executor_review_notes.md",
            "cargo check",
            "cargo clippy",
            "cargo fmt",
            "git diff",
            "commit",
            "report",
            "artifact",
            "verification",
            "audit",
            "отчет",
            "отчёт",
            "артефакт",
            "провер",
            "коммит",
        ],
    );
    if mentions_artifact_or_check {
        return (
            false,
            true,
            "rejected_final_answer_only_artifact_claim_requires_artifact_progress_verifier"
                .to_owned(),
        );
    }

    (
        false,
        true,
        "rejected_requires_artifact_progress_verifier".to_owned(),
    )
}

fn deterministic_agent_control_output_verification(
    prompt_text: &str,
    response_text: &str,
) -> (bool, bool, String) {
    let intent = agent_control_intent_kind(prompt_text);
    let response_lower = response_text.to_lowercase();
    let response_tokens = normalized_token_count(&response_lower);
    let refusal_or_failure = contains_any(
        &response_lower,
        &[
            "cannot",
            "can't",
            "unable",
            "failed",
            "failure",
            "не могу",
            "не смог",
            "не получилось",
            "ошибка",
            "провал",
        ],
    );
    let work_claim = contains_any(
        &response_lower,
        &[
            "commit",
            "diff",
            "patch",
            "cargo",
            "clippy",
            "pytest",
            "запуст",
            "измен",
            "добав",
            "удал",
            "исправ",
            "закоммит",
            "провер",
            "прочит",
            "собрал",
            "создал",
        ],
    ) || response_text.contains("```");

    match intent {
        "stop" => {
            let stop_ack = contains_any(
                &response_lower,
                &[
                    "останов",
                    "стоп",
                    "пауза",
                    "жду",
                    "ничего не дел",
                    "не трога",
                    "stopped",
                    "paused",
                ],
            );
            let verified = stop_ack && !work_claim && !refusal_or_failure;
            let status = if verified {
                "verified_stop_ack_without_work_claim"
            } else if !stop_ack {
                "rejected_stop_ack_absent"
            } else if work_claim {
                "rejected_stop_response_claims_work"
            } else {
                "rejected_stop_response_reports_failure"
            };
            (verified, true, status.to_owned())
        }
        "short_ack" => {
            let ack = contains_any(
                &response_lower,
                &[
                    "да",
                    "нет",
                    "ок",
                    "ага",
                    "понял",
                    "хорошо",
                    "ладно",
                    "можно",
                    "yes",
                    "no",
                    "ok",
                ],
            );
            let verified = ack && response_tokens <= 16 && !work_claim && !refusal_or_failure;
            let status = if verified {
                "verified_short_ack_without_work_claim"
            } else if !ack {
                "rejected_short_ack_absent"
            } else if response_tokens > 16 {
                "rejected_short_ack_response_too_long"
            } else if work_claim {
                "rejected_short_ack_response_claims_work"
            } else {
                "rejected_short_ack_response_reports_failure"
            };
            (verified, true, status.to_owned())
        }
        "continue" => (
            false,
            true,
            "rejected_continue_requires_live_task_execution_or_tool_state_verifier".to_owned(),
        ),
        _ => (
            false,
            false,
            "not_applicable_unknown_agent_control_intent".to_owned(),
        ),
    }
}

fn response_contains_branch_token(response_lower: &str, token: &str) -> bool {
    let token_lower = token.to_lowercase();
    let token_lower = token_lower.trim();
    if token_lower.is_empty() {
        return false;
    }
    if token_lower.chars().count() < 3 {
        response_lower
            .split(|ch: char| !ch.is_alphanumeric())
            .any(|part| part == token_lower)
    } else {
        response_lower.contains(token_lower)
    }
}

fn append_trace_note(previous: Option<&str>, addition: &str) -> String {
    match previous.map(str::trim).filter(|text| !text.is_empty()) {
        Some(previous) => format!("{previous}; {addition}"),
        None => addition.to_owned(),
    }
}

fn nonempty_option(value: &Option<String>) -> bool {
    value
        .as_deref()
        .map(str::trim)
        .is_some_and(|text| !text.is_empty())
}

fn read_real_traffic_event_jsonl(
    events_path: &Path,
) -> Result<Vec<RoleBindingRealTrafficEventRow>, String> {
    let text = fs::read_to_string(events_path).map_err(|error| {
        format!(
            "failed to read real-traffic event JSONL {}: {error}",
            events_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row =
            serde_json::from_str::<RoleBindingRealTrafficEventRow>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic event JSONL {} line {}: {error}",
                    events_path.display(),
                    line_index + 1
                )
            })?;
        validate_real_traffic_event_row(&row)?;
        rows.push(row);
    }
    Ok(rows)
}

fn read_codex_history_jsonl(
    history_path: &Path,
) -> Result<Vec<RoleBindingCodexHistoryRow>, String> {
    let text = fs::read_to_string(history_path).map_err(|error| {
        format!(
            "failed to read Codex history JSONL {}: {error}",
            history_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<RoleBindingCodexHistoryRow>(trimmed).map_err(|error| {
            format!(
                "failed to parse Codex history JSONL {} line {}: {error}",
                history_path.display(),
                line_index + 1
            )
        })?;
        rows.push(row);
    }
    Ok(rows)
}

fn read_real_traffic_trace_jsonl(
    trace_path: &Path,
) -> Result<Vec<RoleBindingRealTrafficTraceRow>, String> {
    let text = fs::read_to_string(trace_path).map_err(|error| {
        format!(
            "failed to read real-traffic trace JSONL {}: {error}",
            trace_path.display()
        )
    })?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row =
            serde_json::from_str::<RoleBindingRealTrafficTraceRow>(trimmed).map_err(|error| {
                format!(
                    "failed to parse real-traffic trace JSONL {} line {}: {error}",
                    trace_path.display(),
                    line_index + 1
                )
            })?;
        validate_real_traffic_trace_row(&row)?;
        rows.push(row);
    }
    Ok(rows)
}

fn write_real_traffic_event_jsonl(
    events_path: &Path,
    rows: &[RoleBindingRealTrafficEventRow],
) -> Result<(), String> {
    if let Some(parent) = events_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create event parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize event JSONL row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    fs::write(events_path, text).map_err(|error| {
        format!(
            "failed to write event JSONL {}: {error}",
            events_path.display()
        )
    })
}

fn write_real_traffic_trace_jsonl(
    trace_path: &Path,
    rows: &[RoleBindingRealTrafficTraceRow],
) -> Result<(), String> {
    if let Some(parent) = trace_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create trace parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut text = String::new();
    for row in rows {
        let line = serde_json::to_string(row)
            .map_err(|error| format!("failed to serialize trace JSONL row: {error}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    fs::write(trace_path, text).map_err(|error| {
        format!(
            "failed to write trace JSONL {}: {error}",
            trace_path.display()
        )
    })
}

fn append_jsonl_row<T>(path: &Path, row: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create JSONL parent directory {}: {error}",
                parent.display()
            )
        })?;
    }
    let mut line = serde_json::to_string(row)
        .map_err(|error| format!("failed to serialize JSONL row: {error}"))?;
    line.push('\n');
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open JSONL {}: {error}", path.display()))?;
    file.write_all(line.as_bytes())
        .map_err(|error| format!("failed to append JSONL {}: {error}", path.display()))
}

fn stable_real_traffic_fingerprint64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn common_traffic_source(rows: &[RoleBindingRealTrafficTraceRow]) -> String {
    let mut sources = HashSet::new();
    for row in rows {
        sources.insert(
            row.traffic_source
                .clone()
                .unwrap_or_else(|| "unknown".to_owned()),
        );
    }
    if sources.len() == 1 {
        sources
            .into_iter()
            .next()
            .unwrap_or_else(|| "unknown".to_owned())
    } else {
        format!("mixed:{}sources", sources.len())
    }
}

fn real_traffic_time_window(rows: &[RoleBindingRealTrafficTraceRow]) -> String {
    let mut min_time = u64::MAX;
    let mut max_time = 0u64;
    let mut seen = false;
    for row in rows {
        if let Some(time_ms) = row.time_ms {
            min_time = min_time.min(time_ms);
            max_time = max_time.max(time_ms);
            seen = true;
        }
    }
    if seen {
        format!("{min_time}..{max_time}")
    } else {
        "unknown".to_owned()
    }
}

fn percentile(values: &[u128], percentile: usize) -> u128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() * percentile) / 100).min(sorted.len() - 1);
    sorted[index]
}

fn percentile_i32_sorted(sorted_values: &[i32], percentile: usize) -> i32 {
    if sorted_values.is_empty() {
        return 0;
    }
    let index = ((sorted_values.len() * percentile) / 100).min(sorted_values.len() - 1);
    sorted_values[index]
}

fn reduction_milli(exact_cache_llm_calls: usize, exact_cache_plus_nando_llm_calls: usize) -> usize {
    exact_cache_llm_calls
        .saturating_sub(exact_cache_plus_nando_llm_calls)
        .saturating_mul(1000)
        .checked_div(exact_cache_llm_calls)
        .unwrap_or(0)
}

fn current_rss_bytes() -> usize {
    let Ok(statm) = fs::read_to_string("/proc/self/statm") else {
        return 0;
    };
    let Some(resident_pages) = statm
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return 0;
    };
    resident_pages.saturating_mul(4096)
}
