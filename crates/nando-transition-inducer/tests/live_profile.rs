use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use nando_transition_inducer::a2_lab::{
    build_a2_live_smoke_fixture, build_raw_phase_frontier_fixture,
    build_raw_phase_live_smoke_fixture,
};
use nando_transition_inducer::{
    LivePackageOrigin, LiveProfileRegistry, LiveProfileState, read_package,
    read_package_artifact_bytes,
};
use serde_json::{Value, json};

#[test]
fn stored_package_accounting_uses_the_persisted_schema() {
    let fixture = build_a2_live_smoke_fixture().expect("fixture");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/package-accounting-schema-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("root");
    let path = root.join("package.json");
    let mut persisted = serde_json::to_value(&fixture.package).expect("package value");
    persisted
        .as_object_mut()
        .expect("package object")
        .insert("legacy_extension".to_owned(), json!({"version": 1}));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&persisted).expect("persisted package"),
    )
    .expect("package write");

    let stored_bytes = read_package_artifact_bytes(&path).expect("stored artifact bytes");
    let decoded_bytes = read_package(&path)
        .expect("current package")
        .artifact_bytes()
        .expect("current artifact bytes")
        .len();
    assert_eq!(
        stored_bytes,
        serde_json::to_vec(&persisted)
            .expect("canonical persisted bytes")
            .len()
    );
    assert_ne!(stored_bytes, decoded_bytes);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn imported_smoke_profile_never_promotes_or_executes() {
    let fixture = build_a2_live_smoke_fixture().expect("fixture");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/live-profile-integration-test");
    let _ = fs::remove_dir_all(&root);
    let inbox = root.join("package-inbox");
    fs::create_dir_all(&inbox).expect("inbox");
    fs::write(
        inbox.join("package.json"),
        serde_json::to_vec_pretty(&fixture.package).expect("package json"),
    )
    .expect("package write");
    let trace_path = root.join("live-transitions.jsonl");
    write_traces(&trace_path, &fixture.traces[..16]);
    fs::write(
        root.join("execution-events.jsonl"),
        concat!(
            "{\"event\":\"bridge_request\",\"request_sha256\":\"request-a\",\"tokens\":8}\n",
            "{\"event\":\"transition_request\",\"request_sha256\":\"request-a\",\"tokens\":8}\n",
            "{\"event\":\"local_accept\",\"request_sha256\":\"request-a\",\"tokens\":8}\n",
            "{\"event\":\"bridge_request\",\"request_sha256\":\"request-b\",\"tokens\":3}\n"
        ),
    )
    .expect("events write");

    run_daemon(&root);
    let registry_path = root.join("registry.json");
    let registry = LiveProfileRegistry::load(&registry_path).expect("registry");
    assert_eq!(registry.active_profile_count(), 0, "{registry:#?}");
    assert_eq!(registry.non_raw_active_profile_count(), 0, "{registry:#?}");
    assert_eq!(registry.telemetry.traces_seen, 16);
    assert_eq!(registry.telemetry.total_bridge_requests, 2);
    assert_eq!(registry.telemetry.active_local_accepts, 0);
    assert_eq!(registry.telemetry.tokens_saved, 0);
    assert!(registry.trace_watermark_bytes > 0);
    assert!(registry.execution_event_watermark_bytes > 0);
    assert!(
        registry
            .packages
            .values()
            .flat_map(|package| package.profiles.iter())
            .all(|profile| profile.state == LiveProfileState::Quarantine)
    );
    let admission = inspect(&registry_path);
    assert_eq!(
        admission.get("verdict"),
        Some(&Value::String("VETO".to_owned()))
    );
    assert_eq!(
        admission.get("local_accept_eligible"),
        Some(&Value::Bool(false))
    );

    let accepted = execute(&registry_path, &fixture.traces[16]);
    assert_eq!(accepted.get("local_accept"), Some(&Value::Bool(false)));

    let mut legacy_active = registry;
    legacy_active
        .packages
        .values_mut()
        .next()
        .expect("package")
        .profiles[0]
        .state = LiveProfileState::Active;
    legacy_active
        .save(&registry_path)
        .expect("legacy active save");
    run_daemon(&root);
    let migrated = LiveProfileRegistry::load(&registry_path).expect("migrated registry");
    assert_eq!(migrated.active_profile_count(), 0, "{migrated:#?}");
    assert_eq!(migrated.non_raw_active_profile_count(), 0, "{migrated:#?}");
    assert_eq!(migrated.revoked_profile_count(), 1, "{migrated:#?}");
    assert!(migrated.packages.values().any(|record| {
        record.origin == LivePackageOrigin::Imported
            && record.profiles.iter().any(|profile| {
                profile.last_reason == "automatic_revoke_non_raw_production_authority"
            })
    }));
}

#[test]
fn raw_phase_daemon_discovers_shadows_and_promotes_compact_profiles() {
    let fixture = build_raw_phase_live_smoke_fixture();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/raw-phase-live-integration-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("package-inbox")).expect("inbox");
    let trace_path = root.join("live-transitions.jsonl");
    write_traces(&trace_path, &fixture.support);

    run_daemon(&root);
    let registry_path = root.join("registry.json");
    let registry = LiveProfileRegistry::load(&registry_path).expect("registry");
    assert!(!registry.raw_phase_families.is_empty(), "{registry:#?}");
    assert!(
        registry
            .raw_phase_families
            .values()
            .all(|family| family.stage == "cleanup"),
        "{registry:#?}"
    );
    assert!(registry.telemetry.raw_phase_training_attempts > 0);
    assert!(registry.telemetry.raw_phase_packages_induced > 0);
    assert_eq!(
        registry
            .raw_phase_families
            .values()
            .map(|family| family.surface_frontier.len())
            .sum::<usize>(),
        4,
        "{registry:#?}"
    );
    assert!(registry.raw_phase_families.values().all(|family| {
        family.leave_one_surface_out_pass
            && family.new_session_split_pass
            && family.forward_time_split_pass
            && family.transfer_tested_surfaces == 4
            && family.transfer_passed_surfaces == 4
            && family.transfer_query_rows > 0
            && family.transfer_correct_executions == family.transfer_query_rows
            && family.transfer_wrong_accepts == 0
            && family.transfer_abstains == 0
            && family.session_transfer_query_rows > 0
            && family.session_transfer_correct_executions == family.session_transfer_query_rows
            && family.session_transfer_wrong_accepts == 0
            && family.session_transfer_abstains == 0
            && family.time_transfer_query_rows > 0
            && family.time_transfer_correct_executions == family.time_transfer_query_rows
            && family.time_transfer_wrong_accepts == 0
            && family.time_transfer_abstains == 0
            && family.surface_frontier.values().all(|surface| {
                surface.circuit_covered && surface.session_count > 1 && surface.package_id.is_some()
            })
    }));
    assert!(
        registry
            .packages
            .values()
            .all(|record| record.origin == LivePackageOrigin::RawPhaseInduction),
        "{registry:#?}"
    );
    assert_eq!(registry.active_profile_count(), 0, "{registry:#?}");
    assert!(registry.quarantined_profile_count() > 0, "{registry:#?}");
    for record in registry.packages.values() {
        let package = read_package(Path::new(&record.package_path)).expect("package");
        assert!(package.transitions.iter().all(|transition| {
            transition.routing_atoms.is_empty() && !transition.routing_atom_ids.is_empty()
        }));
    }

    let attempts = registry.telemetry.raw_phase_training_attempts;
    run_daemon(&root);
    let unchanged = LiveProfileRegistry::load(&registry_path).expect("unchanged registry");
    assert_eq!(
        unchanged.telemetry.raw_phase_training_attempts, attempts,
        "no new traces must not retrain"
    );

    let mut stale_backfill = fixture.future_promotion.clone();
    for (index, trace) in stale_backfill.iter_mut().enumerate() {
        trace.trace_id = format!("raw-phase-live-stale-backfill-{index:04}");
        trace.timestamp = "2026-07-07T23:59:59Z".to_owned();
    }
    append_traces(&trace_path, &stale_backfill);
    run_daemon(&root);
    let stale_rejected = LiveProfileRegistry::load(&registry_path).expect("stale registry");
    assert!(
        stale_rejected
            .packages
            .values()
            .filter(|record| record.origin == LivePackageOrigin::RawPhaseInduction)
            .all(|record| {
                record.future_evidence_not_before_unix_nanos > 0
                    && record
                        .profiles
                        .iter()
                        .all(|profile| profile.future_rows == 0)
            }),
        "delayed pre-training rows cannot become future evidence: {stale_rejected:#?}"
    );

    append_traces(&trace_path, &fixture.future_shadow);
    run_daemon(&root);
    let shadowed = LiveProfileRegistry::load(&registry_path).expect("shadowed registry");
    assert_eq!(shadowed.active_profile_count(), 0, "{shadowed:#?}");
    assert_eq!(shadowed.telemetry.false_accepts, 0, "{shadowed:#?}");
    assert!(shadowed.telemetry.shadow_executions > 0, "{shadowed:#?}");

    append_traces(&trace_path, &fixture.future_promotion);
    run_daemon(&root);
    let promoted = LiveProfileRegistry::load(&registry_path).expect("promoted registry");
    assert_eq!(promoted.active_profile_count(), 16, "{promoted:#?}");
    assert_eq!(promoted.telemetry.profiles_promoted, 16, "{promoted:#?}");
    assert_eq!(promoted.telemetry.false_accepts, 0, "{promoted:#?}");
    let metrics: Value =
        serde_json::from_slice(&fs::read(root.join("metrics.json")).expect("metrics read"))
            .expect("metrics json");
    assert_eq!(metrics.get("raw_phase_enabled"), Some(&Value::Bool(true)));
    assert_eq!(
        metrics.get("legacy_named_inducer_enabled"),
        Some(&Value::Bool(false))
    );
    assert_eq!(
        metrics.get("raw_phase_transfer_pass_families"),
        Some(&Value::from(1))
    );
    assert_eq!(
        metrics.get("raw_phase_transfer_wrong_accepts"),
        Some(&Value::from(0))
    );
    assert_eq!(
        metrics.get("raw_phase_session_transfer_pass_families"),
        Some(&Value::from(1))
    );
    assert_eq!(
        metrics.get("raw_phase_time_transfer_pass_families"),
        Some(&Value::from(1))
    );
    assert_eq!(
        metrics.get("verdict"),
        Some(&Value::String("ACTIVE_GUARDED_CPU_EXECUTION".to_owned()))
    );
    let accepted = execute(&registry_path, &fixture.future_promotion[0]);
    assert_eq!(accepted.get("local_accept"), Some(&Value::Bool(true)));
    assert_eq!(accepted.get("verifier_ok"), Some(&Value::Bool(true)));
    assert_eq!(
        accepted
            .get("verification_receipt_id")
            .and_then(Value::as_str)
            .map(str::len),
        Some(64)
    );
    assert_eq!(
        accepted
            .get("verified_after_digest")
            .and_then(Value::as_str)
            .map(str::len),
        Some(64)
    );
    assert_eq!(
        accepted.get("verifier_schema"),
        Some(&Value::String(
            "typed_actor_independent_verifier.v1".to_owned()
        ))
    );

    let mut drift = fixture.future_promotion[0].clone();
    drift.trace_id = "raw-phase-live-drift".to_owned();
    drift.after = drift.before.clone();
    append_traces(&trace_path, &[drift]);
    run_daemon(&root);
    let revoked = LiveProfileRegistry::load(&registry_path).expect("revoked registry");
    assert!(revoked.revoked_profile_count() > 0, "{revoked:#?}");
    assert!(revoked.active_profile_count() < 16, "{revoked:#?}");
}

#[test]
fn raw_phase_frontier_tracks_all_surfaces_without_a_fixed_cap() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/raw-phase-frontier-integration-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("package-inbox")).expect("inbox");
    write_traces(
        &root.join("live-transitions.jsonl"),
        &build_raw_phase_frontier_fixture(20),
    );

    run_daemon(&root);
    let registry =
        LiveProfileRegistry::load(&root.join("registry.json")).expect("frontier registry");
    assert_eq!(registry.raw_phase_families.len(), 1, "{registry:#?}");
    let family = registry.raw_phase_families.values().next().expect("family");
    assert_eq!(family.observed_surfaces, 20, "{family:#?}");
    assert_eq!(family.surface_frontier.len(), 20, "{family:#?}");
    assert_eq!(family.eligible_surfaces, 0, "{family:#?}");
    assert!(
        family
            .surface_frontier
            .values()
            .all(|surface| !surface.circuit_covered),
        "{family:#?}"
    );
    let metrics: Value = serde_json::from_slice(
        &fs::read(root.join("metrics.json")).expect("frontier metrics read"),
    )
    .expect("frontier metrics json");
    assert_eq!(
        metrics.get("raw_phase_total_observed_surfaces"),
        Some(&Value::from(20))
    );
    assert_eq!(
        metrics.get("raw_phase_total_unsupported_surfaces"),
        Some(&Value::from(20))
    );
}

#[test]
fn three_surface_circuit_cannot_export_without_leave_one_surface_out_proof() {
    let mut support = build_raw_phase_live_smoke_fixture().support;
    support.retain(|trace| !trace.trace_id.starts_with("raw-live-support-3-"));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/raw-phase-three-surface-gate-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("package-inbox")).expect("inbox");
    write_traces(&root.join("live-transitions.jsonl"), &support);

    run_daemon(&root);
    let registry =
        LiveProfileRegistry::load(&root.join("registry.json")).expect("three surface registry");
    let family = registry.raw_phase_families.values().next().expect("family");
    assert_eq!(family.observed_surfaces, 3, "{family:#?}");
    assert!(family.phase_circuit_ready, "{family:#?}");
    assert!(!family.leave_one_surface_out_pass, "{family:#?}");
    assert_eq!(
        family.last_reason,
        "awaiting_leave_one_surface_out_transfer"
    );
    assert!(registry.packages.is_empty(), "{registry:#?}");
    assert_eq!(registry.telemetry.raw_phase_packages_induced, 0);
}

#[test]
fn package_export_requires_independent_session_evidence() {
    let mut support = build_raw_phase_live_smoke_fixture().support;
    for trace in &mut support {
        trace.source_session_id_sha256.clear();
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/raw-phase-session-gate-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("package-inbox")).expect("inbox");
    write_traces(&root.join("live-transitions.jsonl"), &support);

    run_daemon(&root);
    let registry =
        LiveProfileRegistry::load(&root.join("registry.json")).expect("session gate registry");
    let family = registry.raw_phase_families.values().next().expect("family");
    assert!(family.phase_circuit_ready, "{family:#?}");
    assert!(family.leave_one_surface_out_pass, "{family:#?}");
    assert!(!family.new_session_split_pass, "{family:#?}");
    assert_eq!(family.last_reason, "awaiting_session_time_transfer");
    assert!(registry.packages.is_empty(), "{registry:#?}");
}

#[test]
fn package_export_requires_forward_time_evidence() {
    let mut support = build_raw_phase_live_smoke_fixture().support;
    for trace in &mut support {
        trace.timestamp.clear();
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/nando-wave/raw-phase-time-gate-test");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("package-inbox")).expect("inbox");
    write_traces(&root.join("live-transitions.jsonl"), &support);

    run_daemon(&root);
    let registry =
        LiveProfileRegistry::load(&root.join("registry.json")).expect("time gate registry");
    let family = registry.raw_phase_families.values().next().expect("family");
    assert!(family.phase_circuit_ready, "{family:#?}");
    assert!(family.leave_one_surface_out_pass, "{family:#?}");
    assert!(family.new_session_split_pass, "{family:#?}");
    assert!(!family.forward_time_split_pass, "{family:#?}");
    assert_eq!(family.last_reason, "awaiting_session_time_transfer");
    assert!(registry.packages.is_empty(), "{registry:#?}");
}

fn run_daemon(root: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_nando-transition-profile-daemon"))
        .arg("--once")
        .env("NANDO_TRANSITION_STATE_DIR", root)
        .env("NANDO_TRANSITION_PACKAGE_INBOX", root.join("package-inbox"))
        .env("NANDO_TRANSITION_REGISTRY", root.join("registry.json"))
        .env("NANDO_TRANSITION_METRICS", root.join("metrics.json"))
        .env(
            "NANDO_TRANSITION_TRACE_JSONL",
            root.join("live-transitions.jsonl"),
        )
        .env(
            "NANDO_TRANSITION_EXECUTION_EVENTS_JSONL",
            root.join("execution-events.jsonl"),
        )
        .status()
        .expect("daemon status");
    assert!(status.success());
}

fn execute(
    registry_path: &Path,
    trace: &nando_transition_inducer::LiveObservedTransition,
) -> Value {
    let request = json!({
        "schema": "nando.live-transition-request.v1",
        "before": trace.before,
        "action": trace.action,
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_nando-transition-live-exec"))
        .env("NANDO_TRANSITION_REGISTRY", registry_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("executor spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(
            serde_json::to_string(&request)
                .expect("request json")
                .as_bytes(),
        )
        .expect("request write");
    let output = child.wait_with_output().expect("executor output");
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("executor json")
}

fn inspect(registry_path: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_nando-transition-admission-inspect"))
        .env("NANDO_TRANSITION_REGISTRY", registry_path)
        .output()
        .expect("inspector output");
    serde_json::from_slice(&output.stdout).expect("inspector json")
}

fn write_traces(path: &Path, traces: &[nando_transition_inducer::LiveObservedTransition]) {
    let rows = traces
        .iter()
        .map(|trace| serde_json::to_string(trace).expect("trace json"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{rows}\n")).expect("trace write");
}

fn append_traces(path: &Path, traces: &[nando_transition_inducer::LiveObservedTransition]) {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("append traces");
    for trace in traces {
        writeln!(
            file,
            "{}",
            serde_json::to_string(trace).expect("trace json")
        )
        .expect("trace append");
    }
}
