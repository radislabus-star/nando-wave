use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nando_response_actor::{
    CaptureCommitmentArchiveReader, CaptureCommitmentIndex, CaptureTransitionBindingArchiveReader,
    CompositeResponseAdmissionV2, OnlineAdmissionCandidateBundle,
    OnlineAdmissionCandidateRejection, OnlineAdmissionSnapshot, ResponseExecutor, ResponseRegistry,
    build_crystallized_admission_snapshot, build_online_admission_evaluation,
    build_online_collection_admission_snapshot, merge_with_active_online_admission,
    response_runtime_contract_sha256, sha256_bytes, verify_crystallized_capture_provenance_durable,
};
use serde::Serialize;

#[derive(Serialize)]
struct AdmissionControllerReport {
    schema: &'static str,
    generated_at_unix: u64,
    verdict: &'static str,
    blocker: Option<String>,
    blocker_stage: Option<String>,
    candidate_rejections: Vec<OnlineAdmissionCandidateRejection>,
    candidate_revision: u64,
    relation_candidates: usize,
    collection_candidates: usize,
    crystallized_candidates: usize,
    relation_max_future_rows: usize,
    relation_max_runtime_parity_cases: usize,
    collection_max_future_rows: usize,
    collection_max_runtime_parity_cases: usize,
    active_packages: usize,
    last_known_good_preserved: bool,
    elapsed_micros: u64,
}

pub(super) fn main() {
    match env::args().nth(1).as_deref() {
        Some("--print-runtime-contract-sha256") => {
            println!("{}", response_runtime_contract_sha256());
            return;
        }
        Some("--inspect-candidate-routes") => {
            if let Err(error) = inspect_candidate_routes() {
                eprintln!("nando-response-admission: {error}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }
    let started = Instant::now();
    if let Err(error) = run(started) {
        eprintln!("nando-response-admission: {error}");
        std::process::exit(1);
    }
}

fn inspect_candidate_routes() -> Result<(), String> {
    let state_dir = env_path(
        "NANDO_TRANSITION_STATE_DIR",
        "/var/lib/nando-wave/transition",
    );
    let candidate_path = env_path_join(
        "NANDO_RESPONSE_ADMISSION_CANDIDATES",
        &state_dir,
        "response-admission-candidates.cbor",
    );
    let bytes = fs::read(&candidate_path)
        .map_err(|error| format!("candidate_bundle_read:{}:{error}", candidate_path.display()))?;
    let bundle: OnlineAdmissionCandidateBundle = serde_cbor::from_slice(&bytes)
        .map_err(|error| format!("candidate_bundle_decode:{error}"))?;
    bundle.validate().map_err(str::to_owned)?;
    verify_capture_provenance(&bundle, &state_dir)?;
    let candidates = bundle
        .relation_candidates
        .iter()
        .map(|candidate| {
            let support = candidate
                .support
                .iter()
                .map(nando_response_actor::relation_frame_online_routing_atom_ids)
                .collect::<Vec<_>>();
            let negatives = candidate
                .negatives
                .iter()
                .map(nando_response_actor::relation_frame_online_routing_atom_ids)
                .collect::<Vec<_>>();
            let mut frequencies = BTreeMap::<u64, (usize, usize)>::new();
            for atoms in &support {
                for atom in atoms {
                    frequencies.entry(*atom).or_default().0 += 1;
                }
            }
            for atoms in &negatives {
                for atom in atoms {
                    frequencies.entry(*atom).or_default().1 += 1;
                }
            }
            let required_coverage = candidate
                .required_routing_atom_ids
                .iter()
                .map(|required| {
                    serde_json::json!({
                        "atom_id": required,
                        "support": support.iter().filter(|atoms| atoms.binary_search(required).is_ok()).count(),
                        "negatives": negatives.iter().filter(|atoms| atoms.binary_search(required).is_ok()).count(),
                    })
                })
                .collect::<Vec<_>>();
            let mut clean_atoms = frequencies
                .iter()
                .filter(|(_, (_, negative))| *negative == 0)
                .map(|(atom, (positive, _))| (*atom, *positive))
                .collect::<Vec<_>>();
            clean_atoms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            clean_atoms.truncate(32);
            let guard_like_negatives = candidate
                .negatives
                .iter()
                .zip(&negatives)
                .filter(|(_, atoms)| {
                    candidate
                        .required_routing_atom_ids
                        .iter()
                        .all(|required| atoms.binary_search(required).is_ok())
                })
                .map(|(frame, atoms)| {
                    let identical_support = candidate
                        .support
                        .iter()
                        .zip(&support)
                        .filter(|(_, positive)| *positive == atoms)
                        .map(|(support_frame, _)| {
                            serde_json::json!({
                                "frame_id_sha256": support_frame.frame_id_sha256,
                                "session_id_sha256": support_frame.session_id_sha256,
                                "client_intent_id_sha256": support_frame.client_intent_id_sha256,
                                "event_id_sha256": support_frame.event_id_sha256,
                                "evidence_ref_sha256": support_frame.evidence_ref_sha256,
                                "observed_at_unix_nanos": support_frame.observed_at_unix_nanos,
                                "atoms": support_frame.atoms,
                            })
                        })
                        .collect::<Vec<_>>();
                    let minimum_atom_difference = support
                        .iter()
                        .map(|positive| sorted_symmetric_difference_len(positive, atoms))
                        .min()
                        .unwrap_or(0);
                    serde_json::json!({
                        "frame_id_sha256": frame.frame_id_sha256,
                        "session_id_sha256": frame.session_id_sha256,
                        "client_intent_id_sha256": frame.client_intent_id_sha256,
                        "event_id_sha256": frame.event_id_sha256,
                        "evidence_ref_sha256": frame.evidence_ref_sha256,
                        "observed_at_unix_nanos": frame.observed_at_unix_nanos,
                        "identical_support_rows": identical_support.len(),
                        "identical_support": identical_support,
                        "minimum_atom_difference": minimum_atom_difference,
                        "atom_count": atoms.len(),
                        "atoms": frame.atoms,
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "bucket_id": candidate.candidate.bucket_id,
                "program": format!("{:?}", candidate.candidate.program.operation),
                "support_rows": support.len(),
                "future_rows": candidate.future.len(),
                "negative_rows": negatives.len(),
                "negative_event_times": candidate.negatives.iter().map(|frame| frame.observed_at_unix_nanos).collect::<Vec<_>>(),
                "required_atom_coverage": required_coverage,
                "clean_atoms_top32": clean_atoms,
                "guard_like_negatives": guard_like_negatives,
            })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "nando.response-candidate-route-inspection.v1",
            "revision": bundle.revision,
            "candidates": candidates,
        }))
        .map_err(|error| format!("candidate_inspection_encode:{error}"))?
    );
    Ok(())
}

fn sorted_symmetric_difference_len(left: &[u64], right: &[u64]) -> usize {
    let mut left_index = 0;
    let mut right_index = 0;
    let mut difference = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            std::cmp::Ordering::Less => {
                difference += 1;
                left_index += 1;
            }
            std::cmp::Ordering::Greater => {
                difference += 1;
                right_index += 1;
            }
            std::cmp::Ordering::Equal => {
                left_index += 1;
                right_index += 1;
            }
        }
    }
    difference + left.len().saturating_sub(left_index) + right.len().saturating_sub(right_index)
}

fn run(started: Instant) -> Result<(), String> {
    let state_dir = env_path(
        "NANDO_TRANSITION_STATE_DIR",
        "/var/lib/nando-wave/transition",
    );
    let candidate_path = env_path_join(
        "NANDO_RESPONSE_ADMISSION_CANDIDATES",
        &state_dir,
        "response-admission-candidates.cbor",
    );
    let registry_path = env_path_join(
        "NANDO_RESPONSE_REGISTRY",
        &state_dir,
        "response-registry.json",
    );
    // The controller proof is an input to the composite gate. Only the gate may
    // write the final admission.json consumed by serving.
    let controller_admission_path = env_path_join(
        "NANDO_RESPONSE_CONTROLLER_ADMISSION_JSON",
        &state_dir,
        "response-admission-controller.json",
    );
    let active_admission_path =
        env_path_join("NANDO_TRANSITION_ADMISSION", &state_dir, "admission.json");
    let authority_candidate_path = env_path_join(
        "NANDO_RESPONSE_AUTHORITY_CANDIDATE",
        &state_dir,
        "response-authority-candidate.json",
    );
    let report_path = env_path_join(
        "NANDO_RESPONSE_ADMISSION_REPORT",
        &state_dir,
        "response-admission-controller-report.json",
    );
    let marker_path = state_dir.join("response-admission-controller.marker.json");
    let gate_path = env_path(
        "NANDO_LIVE_TRANSITION_GATE_BUILD",
        "/opt/nando-wave/bin/nando-live-transition-gate",
    );
    let max_age_seconds = env::var("NANDO_TRANSITION_ADMISSION_MAX_AGE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30)
        .clamp(5, 3_600);
    let bytes = fs::read(&candidate_path)
        .map_err(|error| format!("candidate_bundle_read:{}:{error}", candidate_path.display()))?;
    let bundle: OnlineAdmissionCandidateBundle = serde_cbor::from_slice(&bytes)
        .map_err(|error| format!("candidate_bundle_decode:{error}"))?;
    bundle.validate().map_err(str::to_owned)?;
    let relation_max_future_rows = bundle
        .relation_candidates
        .iter()
        .map(|candidate| candidate.future.len())
        .max()
        .unwrap_or(0);
    let relation_max_runtime_parity_cases = bundle
        .relation_candidates
        .iter()
        .map(|candidate| candidate.runtime_parity_cases.len())
        .max()
        .unwrap_or(0);
    let collection_max_future_rows = bundle
        .collection_candidates
        .iter()
        .map(|candidate| candidate.future_receipts.len())
        .max()
        .unwrap_or(0);
    let collection_max_runtime_parity_cases = bundle
        .collection_candidates
        .iter()
        .map(|candidate| candidate.runtime_parity_cases.len())
        .max()
        .unwrap_or(0);
    if let Err(blocker) = verify_capture_provenance(&bundle, &state_dir) {
        let preserved_active_packages = last_known_good_package_count(
            &registry_path,
            &controller_admission_path,
            &authority_candidate_path,
            &marker_path,
        );
        return write_report(
            &report_path,
            AdmissionControllerReport {
                schema: "nando.response-admission-controller-report.v2",
                generated_at_unix: unix_now(),
                verdict: "BLOCK",
                blocker: Some(blocker),
                blocker_stage: Some("capture_provenance".to_owned()),
                candidate_rejections: Vec::new(),
                candidate_revision: bundle.revision,
                relation_candidates: bundle.relation_candidates.len(),
                collection_candidates: bundle.collection_candidates.len(),
                crystallized_candidates: bundle.crystallized_candidates.len(),
                relation_max_future_rows,
                relation_max_runtime_parity_cases,
                collection_max_future_rows,
                collection_max_runtime_parity_cases,
                active_packages: preserved_active_packages,
                last_known_good_preserved: preserved_active_packages > 0,
                elapsed_micros: elapsed_micros(started),
            },
        );
    }
    let gate_sha256 = sha256_file(&gate_path, "gate_build")?;
    let runtime_sha256 = response_runtime_contract_sha256();
    let now_unix = unix_now();
    let relation_evaluation = build_online_admission_evaluation(
        &bundle.relation_candidates,
        &bundle.project_id,
        bundle.revision,
        now_unix,
        max_age_seconds,
        &gate_sha256,
        &runtime_sha256,
    )
    .map_err(str::to_owned)?;
    let relation_rejections = relation_evaluation.candidate_rejections;
    let relation_shadow_ready = relation_evaluation.snapshot.is_some();
    let collection_shadow_ready = build_online_collection_admission_snapshot(
        &bundle.collection_candidates,
        &bundle.project_id,
        bundle.revision,
        now_unix,
        max_age_seconds,
        &gate_sha256,
        &runtime_sha256,
    )
    .map_err(str::to_owned)?
    .is_some();
    let crystallized = build_crystallized_admission_snapshot(
        &bundle.crystallized_candidates,
        &bundle.project_id,
        bundle.revision,
        now_unix,
        max_age_seconds,
        &gate_sha256,
        &runtime_sha256,
    )
    .map_err(str::to_owned)?;
    // Legacy relation and collection routes remain observable controls. New
    // authority has one owner: a provenance-bound crystallized operator.
    let snapshot = crystallized
        .map(|candidate| {
            merge_with_active_generation_files(
                candidate,
                &registry_path,
                &active_admission_path,
                &bundle.project_id,
                &gate_sha256,
                &runtime_sha256,
                now_unix,
                max_age_seconds,
            )
        })
        .transpose()?;
    let Some(snapshot) = snapshot else {
        let preserved_active_packages = last_known_good_package_count(
            &registry_path,
            &controller_admission_path,
            &authority_candidate_path,
            &marker_path,
        );
        return write_report(
            &report_path,
            AdmissionControllerReport {
                schema: "nando.response-admission-controller-report.v2",
                generated_at_unix: unix_now(),
                verdict: "BLOCK",
                blocker: Some(if relation_shadow_ready || collection_shadow_ready {
                    "legacy_candidate_routes_shadow_only".to_owned()
                } else {
                    "no_crystallized_authority_candidate".to_owned()
                }),
                blocker_stage: Some("crystallized_authority_boundary".to_owned()),
                candidate_rejections: relation_rejections,
                candidate_revision: bundle.revision,
                relation_candidates: bundle.relation_candidates.len(),
                collection_candidates: bundle.collection_candidates.len(),
                crystallized_candidates: bundle.crystallized_candidates.len(),
                relation_max_future_rows,
                relation_max_runtime_parity_cases,
                collection_max_future_rows,
                collection_max_runtime_parity_cases,
                active_packages: preserved_active_packages,
                last_known_good_preserved: preserved_active_packages > 0,
                elapsed_micros: elapsed_micros(started),
            },
        );
    };
    if active_generation_is_immutable(
        &registry_path,
        &controller_admission_path,
        &authority_candidate_path,
        &marker_path,
        &snapshot.registry,
    ) {
        let active_packages = snapshot.registry.packages.len();
        return write_report(
            &report_path,
            AdmissionControllerReport {
                schema: "nando.response-admission-controller-report.v2",
                generated_at_unix: unix_now(),
                verdict: "PASS",
                blocker: Some("active_generation_immutable".to_owned()),
                blocker_stage: Some("authority_registry".to_owned()),
                candidate_rejections: relation_rejections,
                candidate_revision: bundle.revision,
                relation_candidates: bundle.relation_candidates.len(),
                collection_candidates: bundle.collection_candidates.len(),
                crystallized_candidates: bundle.crystallized_candidates.len(),
                relation_max_future_rows,
                relation_max_runtime_parity_cases,
                collection_max_future_rows,
                collection_max_runtime_parity_cases,
                active_packages,
                last_known_good_preserved: true,
                elapsed_micros: elapsed_micros(started),
            },
        );
    }
    let active_packages = snapshot.registry.packages.len();
    ResponseExecutor::from_registry_with_admission(
        snapshot.registry.clone(),
        snapshot.admission.clone(),
        &bundle.project_id,
        &gate_sha256,
        &runtime_sha256,
        now_unix,
        max_age_seconds,
    )
    .map_err(|error| format!("admission_self_check:{error}"))?;
    let response_authority = &snapshot.admission.response_authority;
    let authority_candidate = serde_json::json!({
        "schema": "nando.response-authority-candidate.v1",
        "authority_schema": response_authority.schema,
        "registry_schema": response_authority.registry_schema,
        "registry_revision": response_authority.registry_revision,
        "registry_sha256": response_authority.registry_sha256,
        "execution_authority": false,
        "packages": response_authority.packages,
        "required_gate_fields": [
            "gate_build_sha256",
            "runtime_build_sha256",
            "generated_at_unix",
            "expires_at_unix"
        ]
    });
    write_json_atomic(&registry_path, &snapshot.registry, "response-registry")?;
    write_json_atomic(
        &controller_admission_path,
        &snapshot.admission,
        "response-controller-admission",
    )?;
    write_json_atomic(
        &marker_path,
        &serde_json::json!({
            "schema": "nando.response-admission-controller-marker.v1",
            "candidate_revision": bundle.revision,
            "registry_revision": snapshot.registry.revision,
            "runtime_build_sha256": runtime_sha256,
            "written_at_unix": now_unix,
        }),
        "response-admission-marker",
    )?;
    write_json_atomic(
        &authority_candidate_path,
        &authority_candidate,
        "response-authority-candidate",
    )?;
    write_report(
        &report_path,
        AdmissionControllerReport {
            schema: "nando.response-admission-controller-report.v2",
            generated_at_unix: unix_now(),
            verdict: "PASS",
            blocker: None,
            blocker_stage: None,
            candidate_rejections: relation_rejections,
            candidate_revision: bundle.revision,
            relation_candidates: bundle.relation_candidates.len(),
            collection_candidates: bundle.collection_candidates.len(),
            crystallized_candidates: bundle.crystallized_candidates.len(),
            relation_max_future_rows,
            relation_max_runtime_parity_cases,
            collection_max_future_rows,
            collection_max_runtime_parity_cases,
            active_packages,
            last_known_good_preserved: false,
            elapsed_micros: elapsed_micros(started),
        },
    )
}

fn verify_capture_provenance(
    bundle: &OnlineAdmissionCandidateBundle,
    state_dir: &Path,
) -> Result<(), String> {
    if bundle.crystallized_candidates.is_empty() {
        return Ok(());
    }
    let capture_index_path = env_path_join(
        "NANDO_STREAMING_EVIDENCE_DIR",
        state_dir,
        "streaming-evidence-v2",
    )
    .join("capture-commitment-index.cbor");
    let bytes = fs::read(&capture_index_path).map_err(|error| {
        format!(
            "capture_commitment_index_read:{}:{error}",
            capture_index_path.display()
        )
    })?;
    let index: CaptureCommitmentIndex = serde_cbor::from_slice(&bytes)
        .map_err(|error| format!("capture_commitment_index_decode:{error}"))?;
    let mut archive = CaptureCommitmentArchiveReader::open(
        capture_index_path
            .parent()
            .ok_or_else(|| "capture_archive_parent_missing".to_owned())?,
    )?;
    let mut binding_archive = CaptureTransitionBindingArchiveReader::open(
        capture_index_path
            .parent()
            .ok_or_else(|| "capture_binding_archive_parent_missing".to_owned())?,
    )?;
    verify_crystallized_capture_provenance_durable(
        &bundle.crystallized_candidates,
        &index,
        &mut archive,
        &mut binding_archive,
    )
}

fn write_report(path: &Path, report: AdmissionControllerReport) -> Result<(), String> {
    write_json_atomic(path, &report, "response-admission-report")
}

fn write_json_atomic(path: &Path, value: &impl Serialize, stem: &str) -> Result<(), String> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|error| format!("{stem}_encode:{error}"))?;
    write_atomic(path, &bytes, stem)
}

fn write_atomic(path: &Path, bytes: &[u8], stem: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{stem}_parent_missing"))?;
    fs::create_dir_all(parent).map_err(|error| format!("{stem}_parent_create:{error}"))?;
    let temporary = parent.join(format!(
        ".{stem}.{}.{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("{stem}_temp_create:{error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("{stem}_temp_write:{error}"))?;
        file.sync_all()
            .map_err(|error| format!("{stem}_temp_sync:{error}"))?;
        fs::rename(&temporary, path).map_err(|error| format!("{stem}_rename:{error}"))?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("{stem}_directory_sync:{error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn last_known_good_package_count(
    registry_path: &Path,
    admission_path: &Path,
    authority_candidate_path: &Path,
    marker_path: &Path,
) -> usize {
    if !admission_path.is_file() || !authority_candidate_path.is_file() || !marker_path.is_file() {
        return 0;
    }
    fs::read(registry_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<ResponseRegistry>(&bytes).ok())
        .filter(|registry| registry.validate().is_ok())
        .map_or(0, |registry| registry.packages.len())
}

#[allow(clippy::too_many_arguments)]
fn merge_with_active_generation_files(
    candidate: nando_response_actor::OnlineAdmissionSnapshot,
    registry_path: &Path,
    active_admission_path: &Path,
    project_id: &str,
    gate_sha256: &str,
    runtime_sha256: &str,
    now_unix: u64,
    max_age_seconds: u64,
) -> Result<OnlineAdmissionSnapshot, String> {
    if !registry_path.is_file() || !active_admission_path.is_file() {
        return Ok(candidate);
    }
    let existing: ResponseRegistry = serde_json::from_slice(
        &fs::read(registry_path).map_err(|error| format!("active_registry_read:{error}"))?,
    )
    .map_err(|error| format!("active_registry_decode:{error}"))?;
    let active_admission: CompositeResponseAdmissionV2 = serde_json::from_slice(
        &fs::read(active_admission_path)
            .map_err(|error| format!("active_admission_read:{error}"))?,
    )
    .map_err(|error| format!("active_admission_decode:{error}"))?;
    merge_with_active_online_admission(
        candidate,
        existing,
        active_admission,
        project_id,
        gate_sha256,
        runtime_sha256,
        now_unix,
        max_age_seconds,
    )
    .map_err(str::to_owned)
}

fn active_generation_is_immutable(
    registry_path: &Path,
    admission_path: &Path,
    authority_candidate_path: &Path,
    marker_path: &Path,
    candidate: &ResponseRegistry,
) -> bool {
    if !admission_path.is_file() || !authority_candidate_path.is_file() || !marker_path.is_file() {
        return false;
    }
    let Ok(bytes) = fs::read(registry_path) else {
        return false;
    };
    let Ok(existing) = serde_json::from_slice::<ResponseRegistry>(&bytes) else {
        return false;
    };
    if existing.validate().is_err() || candidate.validate().is_err() {
        return false;
    }
    let existing_ids = existing
        .packages
        .iter()
        .map(|package| package.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let candidate_ids = candidate
        .packages
        .iter()
        .map(|package| package.package_id.as_str())
        .collect::<BTreeSet<_>>();
    !existing_ids.is_empty() && existing_ids == candidate_ids
}

fn sha256_file(path: &Path, label: &str) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| format!("{label}_read:{}:{error}", path.display()))
}

fn env_path(name: &str, default: &str) -> PathBuf {
    env::var_os(name).map_or_else(|| PathBuf::from(default), PathBuf::from)
}

fn env_path_join(name: &str, parent: &Path, default: &str) -> PathBuf {
    env::var_os(name).map_or_else(|| parent.join(default), PathBuf::from)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn elapsed_micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}
