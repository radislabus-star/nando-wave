use std::path::Path;

use nando_core::{PhaseCenterOnlineCandidatePackage, PhaseCenterVerifierBinding};

use super::reports::PhaseStreamLiveStoreCandidatePackageReport;
use super::source_events::live_store_hash_id;

pub(super) fn live_store_verifier_binding() -> PhaseCenterVerifierBinding {
    PhaseCenterVerifierBinding {
        verifier_id: live_store_hash_id(["verifier", "phase_stream_live_store_adapter_smoke_v1"]),
        verifier_version: 1,
        verifier_input_kind_id: live_store_hash_id(["verifier_input", "shadow_verifier_label"]),
        verifier_evidence_source_id: live_store_hash_id([
            "verifier_evidence_source",
            "real_traffic_phase_atom_trace_jsonl",
        ]),
        false_accept_threshold: 0,
    }
}

pub(super) fn write_live_store_candidate_packages(
    candidate_package_dir: &Path,
    candidate_packages: Vec<PhaseCenterOnlineCandidatePackage>,
) -> Result<Vec<PhaseStreamLiveStoreCandidatePackageReport>, String> {
    write_live_store_candidate_packages_with_route_lookup(
        candidate_package_dir,
        candidate_packages,
        |_| None,
    )
}

pub(super) fn write_live_store_candidate_packages_with_route_lookup<F>(
    candidate_package_dir: &Path,
    candidate_packages: Vec<PhaseCenterOnlineCandidatePackage>,
    route_id_for_bucket: F,
) -> Result<Vec<PhaseStreamLiveStoreCandidatePackageReport>, String>
where
    F: Fn(u32) -> Option<u32>,
{
    std::fs::create_dir_all(candidate_package_dir).map_err(|error| {
        format!(
            "failed to create candidate package dir '{}': {error}",
            candidate_package_dir.display()
        )
    })?;
    let mut reports = Vec::with_capacity(candidate_packages.len());
    for package in candidate_packages {
        let package_path = candidate_package_dir.join(format!(
            "bucket-{:08x}-{:016x}.nwpc",
            package.bucket_id, package.package_info.fingerprint64
        ));
        std::fs::write(&package_path, &package.package_bytes).map_err(|error| {
            format!(
                "failed to write candidate package '{}': {error}",
                package_path.display()
            )
        })?;
        reports.push(PhaseStreamLiveStoreCandidatePackageReport {
            route_id: route_id_for_bucket(package.bucket_id).unwrap_or_default(),
            bucket_id: package.bucket_id,
            threshold_micro: package.threshold_micro,
            package_path: package_path.display().to_string(),
            package_bytes: package.package_bytes.len(),
            package_fingerprint64: package.package_info.fingerprint64,
            record_count: package.package_info.record_count,
            verifier_bound: package.verifier_binding.is_bound(),
            promotion_allowed: false,
        });
    }
    reports.sort_by(|a, b| a.bucket_id.cmp(&b.bucket_id));
    Ok(reports)
}
