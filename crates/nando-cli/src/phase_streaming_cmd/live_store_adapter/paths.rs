use std::path::{Path, PathBuf};

pub(super) fn live_store_route_key_from_bucket_key(bucket_key: &str) -> &str {
    bucket_key
        .split_once("::")
        .map_or(bucket_key, |(route_key, _)| route_key)
}

pub(super) fn live_store_resolve_registry_relative_path(
    registry_path: &Path,
    package_path: &Path,
) -> PathBuf {
    if package_path.is_absolute() || package_path.exists() {
        return package_path.to_path_buf();
    }
    for ancestor in registry_path.ancestors() {
        let candidate = ancestor.join(package_path);
        if candidate.exists() {
            return candidate;
        }
    }
    package_path.to_path_buf()
}

pub(super) fn live_store_hot_path_promotion_review_path(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-benchmark-v1");
    report_path.with_file_name(format!("{stem}-promotion-review.json"))
}

pub(super) fn live_store_hot_path_daemon_admission_policy_path(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-benchmark-v1");
    report_path.with_file_name(format!("{stem}-daemon-admission-policy.json"))
}

pub(super) fn live_store_numeric_candidate_package_dir(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-live-loop-numeric-benchmark-v1");
    report_path.with_file_name(format!("{stem}-candidates"))
}

pub(super) fn live_store_append_tail_clean_promotion_manifest_path(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-append-live-tail-v1");
    report_path.with_file_name(format!("{stem}-clean-promotion-manifest.json"))
}

pub(super) fn live_store_append_tail_clean_promotion_package_dir(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-append-live-tail-v1");
    report_path.with_file_name(format!("{stem}-clean-promotion"))
}

pub(super) fn live_store_append_tail_call_token_promotion_manifest_path(
    report_path: &Path,
) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-append-live-tail-v1");
    report_path.with_file_name(format!("{stem}-call-token-promotion-manifest.json"))
}

pub(super) fn live_store_append_tail_call_token_active_manifest_path(
    report_path: &Path,
) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-append-live-tail-v1");
    report_path.with_file_name(format!("{stem}-call-token-promotion-active-manifest.json"))
}

pub(super) fn live_store_append_tail_call_token_promotion_package_dir(
    report_path: &Path,
) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-append-live-tail-v1");
    report_path.with_file_name(format!("{stem}-call-token-promotion"))
}

pub(super) fn live_store_numeric_future_candidate_package_dir(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-numeric-future-package-audit-v1");
    report_path.with_file_name(format!("{stem}-candidates"))
}

pub(super) fn live_store_numeric_future_policy_smoke_path(report_path: &Path) -> PathBuf {
    let file_name = report_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-numeric-future-package-audit-v1.report.json");
    if let Some(prefix) = file_name.strip_suffix(".report.json") {
        report_path.with_file_name(format!("{prefix}.policy-smoke.report.json"))
    } else {
        let stem = report_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("phase-stream-hot-path-daemon-numeric-future-package-audit-v1");
        report_path.with_file_name(format!("{stem}.policy-smoke.report.json"))
    }
}

pub(super) fn live_store_numeric_future_portfolio_child_report_path(
    report_path: &Path,
    index: usize,
) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1");
    report_path.with_file_name(format!("{stem}-child-{index:03}.report.json"))
}

pub(super) fn live_store_numeric_future_portfolio_gate_report_path(report_path: &Path) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1");
    report_path.with_file_name(format!("{stem}-admission-portfolio-gate.report.json"))
}

pub(super) fn live_store_numeric_future_portfolio_runtime_replay_report_path(
    report_path: &Path,
) -> PathBuf {
    let stem = report_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("phase-stream-hot-path-daemon-numeric-future-portfolio-audit-v1");
    report_path.with_file_name(format!("{stem}-runtime-replay.report.json"))
}
