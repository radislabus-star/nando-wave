#!/usr/bin/env python3
"""Fail-closed verifier for the frozen S1C-3 V4 transaction receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any


SCHEMA = "nando.s1c3-transaction-receipt.v4"
PREPARATION_SCHEMA = "nando.s1c3-transaction-preparation.v4"
RESOURCE_SCHEMA = "nando.s1c3-resource-receipt.v4"
PARITY_SCHEMA = "nando.s1c3-parity-receipt.v4"
QUIESCENCE_SCHEMA = "nando.s1c3-quiescence-receipt.v4"
CONTAMINATION_SCHEMA = "nando.s1c3-measurement-contamination-receipt.v4"
OWNERSHIP_SCHEMA = "nando.s1c3-oracle-ownership-receipt.v4"
OWNERSHIP_ROW_SCHEMA = "nando.s1c3-oracle-ownership-row.v4"

PAPER_COMMIT = "1def4272a46641f2c72a9c0efbd5818f93caa655"
PAPER_MANIFEST_ROOT = "805a93477295172dfae83d5dd91f68659d0c19fb28bb5c40aa00fb59beab48e0"
PAPER_VERIFICATION_SHA256 = "ea704c59d55865a93f890913f8f45c8898b96d3fff9066e103d2eff2fc271ec4"
CANDIDATE_COMMIT = "03e3dd00c90206e2f705371318c50dd50537d6d8"
CANDIDATE_TREE = "06a9df51797dffc127fec41672bddae29c38bb92"
PRODUCTION_PROJECTION_PATH = (
    "crates/nando-transition-serving/src/grounded_decision_capture.rs"
)
PRODUCTION_PROJECTION_SCHEMA = "prefix-through-first-cfg-test-line-v1"
PRODUCTION_PROJECTION_SHA256 = (
    "10b2856687c0e22c47e43754d2a05ffa82641002b11d70d42edca1e4c797c316"
)
CARGO_LOCK_SHA256 = "0c4afa1a2b78cb6c4723d955ad56df5638de7a277f5f954970ae75c455b0aec1"
CANDIDATE_CONFIG_SHA256 = "1e6e6726d3d8df34dfcac6cb6644664cf3b066d0c31fe40652fa17cd524708d6"
BASELINE_COMMIT = "663959064a37caf7eb917fc99dfedb6386355fa6"
BASELINE_TREE = "05460ccbc9c44ac8b7174318903c0211de709e2e"
BASELINE_RECEIPT_ROOT = "785450d76037410d96baade19c2b6bb7f0fb24c6be034e2166be5533c7dd985b"
BASELINE_BINARY_SHA256 = "6ad63428f0cbbe96b539db2d63844403c697dec5041a91652b37857bb653ea58"
BASELINE_CONFIG_SHA256 = "cb2e33bdd2c9959b2c975e9585eb60927f9827327f6a74af6ade92b9b19486f5"
UNIT_SHA256 = "6e9d2fe41b1db95f94768d1ab41dffce1f15be92e2f774832c7fe392bb77b135"
PHASE_CONFIG_SHA256 = "5c019cebbde083f963c03619ff1d938786f5b4ec58730dddd5b34adeb33cce31"
AUTHORITY_CONFIG_SHA256 = "d40b7262ff6d744a393b0fc03a5d06610d01728aa2f4603199ca8567189ec88f"
ORACLE_UID = 1000
ORACLE_GID = 1000

TRANSITION_UNIT = "nando-transition-serving.service"
UNTOUCHED_UNITS = (
    "nando-transport-gateway.service",
    "nando-response-learning.service",
    "nando-gateway-control.service",
    "nando-operator-certification-authority.service",
)
ALL_UNITS = (TRANSITION_UNIT, *UNTOUCHED_UNITS)

FORBIDDEN_BUILD_NAMES = (
    "cargo", "rustc", "sccache", "cc", "cc1", "cc1plus", "gcc", "g++",
    "clang", "clang++", "ld", "ld.lld", "lld", "mold", "ninja", "make",
    "cmake", "meson",
)
EXECUTABLE_KEYS = {
    "candidate-binary", "test-response-actor", "test-transition-serving",
    "parity-baseline", "parity-candidate",
}
MEASUREMENT_LABELS = [
    "hot-1", "hot-2", "hot-3",
    "single-sync-1", "single-sync-2", "single-sync-3",
    "three-sync-1", "three-sync-2", "three-sync-3",
    "idle", "rss-capture_off", "rss-capture_on",
    "parity-baseline", "parity-candidate",
]


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, *, omit_field: str | None = None) -> bytes:
    if omit_field is not None:
        value = dict(value)
        value.pop(omit_field, None)
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def require(condition: bool, error: str) -> None:
    if not condition:
        raise InvalidReceipt(error)


def require_exact(value: Any, expected: Any, label: str) -> None:
    require(value == expected, f"{label}_mismatch")


def require_hash(value: Any, label: str) -> str:
    require(
        isinstance(value, str)
        and len(value) == 64
        and all(char in "0123456789abcdef" for char in value),
        f"{label}_invalid",
    )
    return value


def require_nonnegative_int(value: Any, label: str) -> int:
    require(type(value) is int and value >= 0, f"{label}_invalid")
    return value


def require_number(value: Any, label: str) -> float:
    require(type(value) in (int, float) and value >= 0, f"{label}_invalid")
    return float(value)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}:{error}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_embedded_root(value: dict[str, Any], field: str, label: str) -> str:
    expected = require_hash(value.get(field), field)
    actual = digest(canonical_bytes(value, omit_field=field))
    require_exact(actual, expected, f"{label}_root")
    return actual


def verify_identity(preparation: dict[str, Any]) -> None:
    require_exact(preparation.get("schema"), PREPARATION_SCHEMA, "preparation_schema")
    verify_embedded_root(preparation, "preparation_root_sha256", "preparation")
    paper = preparation.get("paper", {})
    candidate = preparation.get("candidate", {})
    baseline = preparation.get("baseline", {})
    immutable = preparation.get("immutable", {})
    require_exact(paper.get("commit"), PAPER_COMMIT, "paper_commit")
    require_exact(paper.get("manifest_root_sha256"), PAPER_MANIFEST_ROOT, "paper_manifest")
    require_exact(paper.get("verification_sha256"), PAPER_VERIFICATION_SHA256,
                  "paper_verification")
    require_exact(candidate.get("source_commit"), CANDIDATE_COMMIT, "candidate_commit")
    require_exact(candidate.get("source_tree"), CANDIDATE_TREE, "candidate_tree")
    require_exact(candidate.get("cargo_lock_sha256"), CARGO_LOCK_SHA256, "cargo_lock")
    require_exact(candidate.get("config_sha256"), CANDIDATE_CONFIG_SHA256, "candidate_config")
    require_exact(candidate.get("production_projection_path"), PRODUCTION_PROJECTION_PATH,
                  "candidate_production_projection_path")
    require_exact(candidate.get("production_projection_schema"), PRODUCTION_PROJECTION_SCHEMA,
                  "candidate_production_projection_schema")
    require_exact(candidate.get("production_projection_sha256"),
                  PRODUCTION_PROJECTION_SHA256,
                  "candidate_production_projection")
    require_hash(candidate.get("binary_sha256"), "candidate_binary")
    require(require_nonnegative_int(candidate.get("binary_size_bytes"), "candidate_binary_size") > 0,
            "candidate_binary_size_zero")
    require_exact(baseline.get("source_commit"), BASELINE_COMMIT, "baseline_commit")
    require_exact(baseline.get("source_tree"), BASELINE_TREE, "baseline_tree")
    require_exact(baseline.get("deployment_receipt_root_sha256"), BASELINE_RECEIPT_ROOT,
                  "baseline_receipt")
    require_exact(baseline.get("binary_sha256"), BASELINE_BINARY_SHA256, "baseline_binary")
    require_exact(baseline.get("config_sha256"), BASELINE_CONFIG_SHA256, "baseline_config")
    require_exact(immutable.get("unit_sha256"), UNIT_SHA256, "unit")
    require_exact(immutable.get("phase_config_sha256"), PHASE_CONFIG_SHA256, "phase_config")
    require_exact(immutable.get("authority_config_sha256"), AUTHORITY_CONFIG_SHA256,
                  "authority_config")
    require_hash(preparation.get("quiescence_root_sha256"), "preparation_quiescence_root")
    require_hash(
        preparation.get("measurement_contamination_root_sha256"),
        "preparation_contamination_root",
    )
    require_hash(preparation.get("executable_set_root_sha256"), "preparation_executable_root")
    require_hash(
        preparation.get("oracle_ownership_root_sha256"),
        "preparation_oracle_ownership_root",
    )
    rollback = preparation.get("rollback", {})
    require_hash(rollback.get("manifest_root_sha256"), "rollback_manifest")
    entries = rollback.get("entries")
    require(isinstance(entries, list), "rollback_entries_invalid")
    require_exact(
        {entry.get("path") for entry in entries if isinstance(entry, dict)},
        {
            "nando-transition-serving",
            "transition-serving.env",
            "nando-transition-serving.service",
            "previous-deployment-receipt.json",
        },
        "rollback_entry_set",
    )
    entry_by_path = {entry["path"]: entry for entry in entries}
    manifest = "".join(
        f'{entry["sha256"]} {entry["size_bytes"]} {entry["path"]}\n'
        for entry in sorted(entries, key=lambda item: item["path"])
    ).encode("ascii")
    require_exact(digest(manifest), rollback.get("manifest_root_sha256"),
                  "rollback_manifest")
    require_exact(entry_by_path["nando-transition-serving"].get("sha256"),
                  BASELINE_BINARY_SHA256, "rollback_binary")
    require_exact(entry_by_path["transition-serving.env"].get("sha256"),
                  BASELINE_CONFIG_SHA256, "rollback_config")
    require_exact(entry_by_path["nando-transition-serving.service"].get("sha256"),
                  UNIT_SHA256, "rollback_unit")


def verify_executables(executables: Any) -> tuple[dict[str, Any], str]:
    require(isinstance(executables, dict), "executables_not_object")
    require_exact(set(executables), EXECUTABLE_KEYS, "executable_key_set")
    expected_sources = {
        "candidate-binary": CANDIDATE_COMMIT,
        "test-response-actor": CANDIDATE_COMMIT,
        "test-transition-serving": CANDIDATE_COMMIT,
        "parity-baseline": BASELINE_COMMIT,
        "parity-candidate": CANDIDATE_COMMIT,
    }
    for name, identity in executables.items():
        require(isinstance(identity, dict), f"executable_{name}_not_object")
        require(isinstance(identity.get("path"), str) and identity["path"].startswith("/"),
                f"executable_{name}_path_invalid")
        require_hash(identity.get("sha256"), f"executable_{name}_sha256")
        require(require_nonnegative_int(identity.get("size_bytes"), f"executable_{name}_size") > 0,
                f"executable_{name}_size_zero")
        mode = identity.get("mode_octal")
        require(isinstance(mode, str) and len(mode) == 4 and all(char in "01234567" for char in mode),
                f"executable_{name}_mode_invalid")
        require(int(mode, 8) & 0o111 != 0, f"executable_{name}_not_executable")
        require_exact(identity.get("source_identity"), expected_sources[name],
                      f"executable_{name}_source")
    return executables, digest(canonical_bytes(executables))


def verify_oracle_file_identity(
    identity: Any,
    expected_path: str,
    expected_mode: str,
    label: str,
) -> None:
    require(isinstance(identity, dict), f"ownership_{label}_not_object")
    require_exact(
        set(identity),
        {"path", "uid", "gid", "mode_octal"},
        f"ownership_{label}_field_set",
    )
    require_exact(identity.get("path"), expected_path, f"ownership_{label}_path")
    require_exact(identity.get("uid"), ORACLE_UID, f"ownership_{label}_uid")
    require_exact(identity.get("gid"), ORACLE_GID, f"ownership_{label}_gid")
    require_exact(identity.get("mode_octal"), expected_mode, f"ownership_{label}_mode")


def verify_ownership(path: Path) -> tuple[dict[str, Any], str]:
    ownership = load_json(path)
    require_exact(ownership.get("schema"), OWNERSHIP_SCHEMA, "ownership_schema")
    ownership_root = verify_embedded_root(
        ownership, "ownership_root_sha256", "ownership"
    )
    require((path.stat().st_mode & 0o7777) == 0o400, "ownership_file_mode")
    transaction_id = ownership.get("transaction_id")
    require(isinstance(transaction_id, str) and transaction_id, "ownership_transaction_id")
    require_exact(
        ownership.get("build_user"),
        {"name": "e", "uid": ORACLE_UID, "gid": ORACLE_GID},
        "ownership_build_user",
    )
    rows = ownership.get("rows")
    require(isinstance(rows, dict), "ownership_rows_not_object")
    require_exact(set(rows), {"baseline", "candidate"}, "ownership_row_labels")
    require_exact(
        ownership.get("rows_root_sha256"),
        digest(canonical_bytes(rows)),
        "ownership_rows_root",
    )
    base = f"/home/e/.cache/nando-s1c3-{transaction_id}"
    expected_probe = {
        "writer_uid": ORACLE_UID,
        "writer_gid": ORACLE_GID,
        "probe_uid": ORACLE_UID,
        "probe_gid": ORACLE_GID,
        "probe_mode_octal": "0600",
        "create_fsync_unlink_pass": True,
        "directory_fsync_pass": True,
    }
    expected_fields = {
        "schema", "label", "workspace", "src", "cargo_toml", "main_rs",
        "probe", "probe_retained", "ownership_row_root_sha256",
    }
    for row_label in ("baseline", "candidate"):
        row = rows[row_label]
        require(isinstance(row, dict), f"ownership_row_{row_label}_not_object")
        require_exact(set(row), expected_fields, f"ownership_row_{row_label}_field_set")
        require_exact(row.get("schema"), OWNERSHIP_ROW_SCHEMA,
                      f"ownership_row_{row_label}_schema")
        verify_embedded_root(
            row, "ownership_row_root_sha256", f"ownership_row_{row_label}"
        )
        require_exact(row.get("label"), row_label, f"ownership_row_{row_label}_label")
        workspace = f"{base}/oracle-{row_label}"
        verify_oracle_file_identity(
            row.get("workspace"), workspace, "0750", f"{row_label}_workspace"
        )
        verify_oracle_file_identity(
            row.get("src"), f"{workspace}/src", "0750", f"{row_label}_src"
        )
        verify_oracle_file_identity(
            row.get("cargo_toml"), f"{workspace}/Cargo.toml", "0640",
            f"{row_label}_cargo_toml",
        )
        verify_oracle_file_identity(
            row.get("main_rs"), f"{workspace}/src/main.rs", "0640",
            f"{row_label}_main_rs",
        )
        require_exact(row.get("probe"), expected_probe, f"ownership_row_{row_label}_probe")
        require_exact(row.get("probe_retained"), False,
                      f"ownership_row_{row_label}_probe_retained")
    return ownership, ownership_root


def verify_quiescence(
    path: Path,
    ownership_root: str,
) -> tuple[dict[str, Any], dict[str, Any], str]:
    quiescence = load_json(path)
    require_exact(quiescence.get("schema"), QUIESCENCE_SCHEMA, "quiescence_schema")
    verify_embedded_root(quiescence, "quiescence_root_sha256", "quiescence")
    require_exact(quiescence.get("candidate_commit"), CANDIDATE_COMMIT, "quiescence_candidate")
    require_exact(quiescence.get("candidate_tree"), CANDIDATE_TREE, "quiescence_tree")
    require_exact(
        quiescence.get("oracle_ownership_root_sha256"),
        ownership_root,
        "quiescence_ownership_root",
    )
    require_exact(quiescence.get("detector_schema"), "proc-comm-exe-basename-v1",
                  "quiescence_detector")
    require_exact(quiescence.get("forbidden_build_names"), list(FORBIDDEN_BUILD_NAMES),
                  "quiescence_forbidden_names")
    require_exact(quiescence.get("maximum_wait_seconds"), 1800, "quiescence_max_wait")
    require_exact(quiescence.get("required_intervals"), 30, "quiescence_intervals")
    require_exact(
        quiescence.get("thresholds"),
        {
            "interval_min_seconds": 0.90,
            "interval_max_seconds": 1.50,
            "cpu4_max_percent": 20.0,
            "cpu4_mean_max_percent": 5.0,
            "io_some_avg10_max": 0.20,
            "io_full_avg10_max": 0.05,
        },
        "quiescence_thresholds",
    )
    attempted = quiescence.get("attempted_samples")
    window = quiescence.get("eligible_window")
    require(isinstance(attempted, list) and len(attempted) >= 30, "quiescence_attempted_samples")
    require(isinstance(window, list) and len(window) == 30, "quiescence_window_count")
    require_exact(attempted[-30:], window, "quiescence_window_tail")
    busy_values = []
    previous_end = None
    for index, sample in enumerate(window):
        require(isinstance(sample, dict), f"quiescence_sample_{index}_not_object")
        require_exact(sample.get("eligible_base"), True, f"quiescence_sample_{index}_eligible")
        require_exact(sample.get("build_processes_start"), [], f"quiescence_sample_{index}_build_start")
        require_exact(sample.get("build_processes_end"), [], f"quiescence_sample_{index}_build_end")
        interval = require_number(sample.get("interval_seconds"), f"quiescence_sample_{index}_interval")
        require(0.90 <= interval <= 1.50, f"quiescence_sample_{index}_interval_budget")
        busy = require_number(sample.get("cpu4_busy_percent"), f"quiescence_sample_{index}_cpu")
        require(busy <= 20.0, f"quiescence_sample_{index}_cpu_budget")
        busy_values.append(busy)
        require(require_number(sample.get("io_some_avg10"), f"quiescence_sample_{index}_some") <= 0.20,
                f"quiescence_sample_{index}_some_budget")
        require(require_number(sample.get("io_full_avg10"), f"quiescence_sample_{index}_full") <= 0.05,
                f"quiescence_sample_{index}_full_budget")
        start = require_nonnegative_int(sample.get("start_monotonic_ns"), f"quiescence_sample_{index}_start")
        end = require_nonnegative_int(sample.get("end_monotonic_ns"), f"quiescence_sample_{index}_end")
        require(end > start, f"quiescence_sample_{index}_order")
        if previous_end is not None:
            require_exact(start, previous_end, f"quiescence_sample_{index}_continuity")
        previous_end = end
    mean_busy = sum(busy_values) / len(busy_values)
    require(mean_busy <= 5.0, "quiescence_cpu_mean_budget")
    require(abs(require_number(quiescence.get("eligible_cpu4_mean_percent"),
                               "quiescence_recorded_cpu_mean") - mean_busy) <= 1e-9,
            "quiescence_cpu_mean_mismatch")
    require_exact(quiescence.get("eligible_window_root_sha256"), digest(canonical_bytes(window)),
                  "quiescence_window_root")
    executables, executable_root = verify_executables(quiescence.get("executables"))
    require((path.stat().st_mode & 0o7777) == 0o400, "quiescence_file_mode")
    return quiescence, executables, executable_root


def verify_contamination(
    contamination: dict[str, Any],
    quiescence_root: str,
    executable_root: str,
) -> None:
    require_exact(contamination.get("schema"), CONTAMINATION_SCHEMA, "contamination_schema")
    verify_embedded_root(
        contamination, "measurement_contamination_root_sha256", "contamination"
    )
    require_exact(contamination.get("quiescence_root_sha256"), quiescence_root,
                  "contamination_quiescence_root")
    require_exact(contamination.get("executable_set_root_sha256"), executable_root,
                  "contamination_executable_root")
    require_exact(contamination.get("monitor_interval_seconds"), 0.5,
                  "contamination_monitor_interval")
    require_exact(contamination.get("maximum_sample_gap_seconds"), 2.0,
                  "contamination_maximum_gap")
    require(require_number(contamination.get("observed_max_sample_gap_seconds"),
                           "contamination_observed_gap") <= 2.0,
            "contamination_observed_gap_budget")
    require_exact(contamination.get("metric_labels"), MEASUREMENT_LABELS,
                  "contamination_metric_labels")
    expected_boundaries = [
        {"label": label, "phase": phase}
        for label in MEASUREMENT_LABELS
        for phase in ("before", "after")
    ]
    boundaries = contamination.get("boundaries")
    require(isinstance(boundaries, list), "contamination_boundaries_invalid")
    require_exact(
        [{"label": row.get("label"), "phase": row.get("phase")} for row in boundaries],
        expected_boundaries,
        "contamination_boundary_order",
    )
    samples = contamination.get("samples")
    require(isinstance(samples, list) and samples, "contamination_samples_empty")
    previous = None
    observed_gaps = []
    for index, sample in enumerate(samples):
        require(isinstance(sample, dict), f"contamination_sample_{index}_not_object")
        require_exact(sample.get("build_processes"), [], f"contamination_sample_{index}_build")
        current = require_nonnegative_int(sample.get("monotonic_ns"), f"contamination_sample_{index}_time")
        if previous is not None:
            require(current >= previous, f"contamination_sample_{index}_order")
            observed_gaps.append((current - previous) / 1_000_000_000)
        previous = current
    actual_max_gap = max(observed_gaps, default=0.0)
    require(actual_max_gap <= 2.0, "contamination_actual_gap_budget")
    require(abs(require_number(contamination.get("observed_max_sample_gap_seconds"),
                               "contamination_recorded_gap") - actual_max_gap) <= 1e-9,
            "contamination_gap_mismatch")
    require_exact(contamination.get("forbidden_process_matches"), [], "contamination_matches")
    require_exact(contamination.get("monitor_errors"), [], "contamination_errors")
    require_exact(contamination.get("contaminated"), False, "contamination_verdict")


def verify_service_snapshot(snapshot: Any, label: str) -> dict[str, Any]:
    require(isinstance(snapshot, dict), f"{label}_not_object")
    require_exact(set(snapshot), set(ALL_UNITS), f"{label}_unit_set")
    for unit, state in snapshot.items():
        require(isinstance(state, dict), f"{label}_{unit}_not_object")
        require_exact(state.get("active_state"), "active", f"{label}_{unit}_active")
        require(require_nonnegative_int(state.get("main_pid"), f"{label}_{unit}_pid") > 0,
                f"{label}_{unit}_pid_zero")
        require_nonnegative_int(state.get("nrestarts"), f"{label}_{unit}_nrestarts")
        require_hash(state.get("fragment_sha256"), f"{label}_{unit}_fragment")
    return snapshot


def verify_resource(
    resource: dict[str, Any],
    quiescence_root: str,
    contamination_root: str,
    executable_root: str,
    ownership_root: str,
) -> None:
    require_exact(resource.get("schema"), RESOURCE_SCHEMA, "resource_schema")
    verify_embedded_root(resource, "resource_root_sha256", "resource")
    require_exact(resource.get("all_pass"), True, "resource_all_pass")
    require_exact(resource.get("quiescence_root_sha256"), quiescence_root,
                  "resource_quiescence_root")
    require_exact(resource.get("measurement_contamination_root_sha256"), contamination_root,
                  "resource_contamination_root")
    require_exact(resource.get("executable_set_root_sha256"), executable_root,
                  "resource_executable_root")
    require_exact(resource.get("oracle_ownership_root_sha256"), ownership_root,
                  "resource_ownership_root")
    require_exact(resource.get("direct_exec_only"), True, "resource_direct_exec")
    require_exact(resource.get("compiler_invocations_after_quiescence"), 0,
                  "resource_compiler_invocations")
    metrics = resource.get("metrics", {})
    for name, limit_p99, limit_max, expected_samples in (
        ("hot_latency", 1_000_000, 2_000_000, 4096),
        ("single_ledger_sync", 5_000_000, 20_000_000, 1024),
    ):
        runs = metrics.get(name)
        require(isinstance(runs, list) and len(runs) == 3, f"{name}_run_count")
        for index, run in enumerate(runs):
            require(isinstance(run, dict), f"{name}_{index}_not_object")
            require(require_nonnegative_int(run.get("p99_ns"), f"{name}_{index}_p99") <= limit_p99,
                    f"{name}_{index}_p99_budget")
            require(require_nonnegative_int(run.get("hard_max_ns"), f"{name}_{index}_max") <= limit_max,
                    f"{name}_{index}_max_budget")
            require_exact(run.get("samples"), expected_samples, f"{name}_{index}_samples")
            if name == "hot_latency":
                require(require_nonnegative_int(run.get("no_goal_p99_ns"),
                                                f"{name}_{index}_no_goal") <= 250_000,
                        f"{name}_{index}_no_goal_budget")
    three_runs = metrics.get("three_ledger_sync")
    require(isinstance(three_runs, list) and len(three_runs) == 3,
            "three_ledger_sync_run_count")
    expected_stage_fields = {
        "precommit_p99_ns",
        "precommit_hard_max_ns",
        "settlement_p99_ns",
        "settlement_hard_max_ns",
        "episode_p99_ns",
        "episode_hard_max_ns",
        "samples",
    }
    for index, run in enumerate(three_runs):
        require(isinstance(run, dict), f"three_ledger_sync_{index}_not_object")
        require_exact(set(run), expected_stage_fields,
                      f"three_ledger_sync_{index}_field_set")
        for field in ("precommit_p99_ns", "settlement_p99_ns"):
            require(require_nonnegative_int(run.get(field),
                                            f"three_ledger_sync_{index}_{field}") <= 5_000_000,
                    f"three_ledger_sync_{index}_{field}_budget")
        for field in (
            "precommit_hard_max_ns",
            "settlement_hard_max_ns",
            "episode_hard_max_ns",
        ):
            require(require_nonnegative_int(run.get(field),
                                            f"three_ledger_sync_{index}_{field}") <= 20_000_000,
                    f"three_ledger_sync_{index}_{field}_budget")
        require_nonnegative_int(run.get("episode_p99_ns"),
                                f"three_ledger_sync_{index}_episode_p99_ns")
        require_exact(run.get("samples"), 256, f"three_ledger_sync_{index}_samples")
    idle = metrics.get("idle_cpu", {})
    require(type(idle.get("percent_of_one_core")) in (int, float), "idle_cpu_percent_invalid")
    require(0 <= idle["percent_of_one_core"] <= 0.25, "idle_cpu_budget")
    rss = metrics.get("rss", {})
    require(require_nonnegative_int(rss.get("delta_bytes"), "rss_delta") <= 16 * 1024 * 1024,
            "rss_delta_budget")
    require_exact(
        resource.get("frozen_bounds"),
        {
            "max_precommit_bytes": 32 * 1024,
            "max_typed_goal_bytes": 4 * 1024,
            "max_k1_actions": 256,
            "segment_bytes": 64 * 1024 * 1024,
            "journal_quota_bytes": 2 * 1024 * 1024 * 1024,
            "persisted_raw_payload_bytes": 0,
        },
        "frozen_bounds",
    )


def verify_parity(parity: dict[str, Any], executables: dict[str, Any]) -> None:
    require_exact(parity.get("schema"), PARITY_SCHEMA, "parity_schema")
    verify_embedded_root(parity, "parity_root_sha256", "parity")
    require_exact(parity.get("byte_identical"), True, "parity_identical")
    require_exact(parity.get("baseline_output_sha256"), parity.get("candidate_output_sha256"),
                  "parity_output")
    require(require_nonnegative_int(parity.get("rows"), "parity_rows") > 0, "parity_rows_zero")
    require_exact(parity.get("direct_exec_only"), True, "parity_direct_exec")
    require_exact(parity.get("baseline_oracle_sha256"),
                  executables["parity-baseline"]["sha256"], "parity_baseline_oracle")
    require_exact(parity.get("candidate_oracle_sha256"),
                  executables["parity-candidate"]["sha256"], "parity_candidate_oracle")


def verify_connector(before: Any, after: Any) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "connector_not_object")
    for field in ("main_pid", "nrestarts", "route_receipt_failures", "command_sha256"):
        require_exact(after.get(field), before.get(field), f"connector_{field}")
    require_exact(before.get("active_state"), "active", "connector_before_active")
    require_exact(after.get("active_state"), "active", "connector_after_active")


def verify_journal(before: Any, after: Any, *, rollback: bool) -> None:
    require(isinstance(before, dict) and isinstance(after, dict), "journal_not_object")
    require_nonnegative_int(before.get("total_bytes"), "journal_before_bytes")
    require_nonnegative_int(after.get("total_bytes"), "journal_after_bytes")
    require(after["total_bytes"] <= 2 * 1024 * 1024 * 1024, "journal_quota")
    require_exact(after.get("raw_payload_bytes"), 0, "journal_raw_payload")
    require_hash(before.get("manifest_root_sha256"), "journal_before_root")
    require_hash(after.get("manifest_root_sha256"), "journal_after_root")
    if rollback:
        require_exact(after.get("preserved_prefixes"), True, "rollback_journal_prefixes")


def verify_receipt(directory: Path) -> dict[str, Any]:
    ownership, ownership_root = verify_ownership(
        directory / "oracle-ownership-receipt.json"
    )
    quiescence, executables, executable_root = verify_quiescence(
        directory / "quiescence-receipt.json", ownership_root
    )
    contamination = load_json(directory / "measurement-contamination-receipt.json")
    preparation = load_json(directory / "preparation.json")
    resource = load_json(directory / "resource-receipt.json")
    parity = load_json(directory / "parity-receipt.json")
    receipt = load_json(directory / "deployment-receipt.json")
    verify_contamination(
        contamination, quiescence["quiescence_root_sha256"], executable_root
    )
    verify_identity(preparation)
    require_exact(ownership.get("transaction_id"), preparation.get("transaction_id"),
                  "ownership_transaction_binding")
    require_exact(quiescence.get("transaction_id"), preparation.get("transaction_id"),
                  "quiescence_transaction_id")
    require_exact(contamination.get("transaction_id"), preparation.get("transaction_id"),
                  "contamination_transaction_id")
    verify_resource(
        resource,
        quiescence["quiescence_root_sha256"],
        contamination["measurement_contamination_root_sha256"],
        executable_root,
        ownership_root,
    )
    verify_parity(parity, executables)
    require_exact(preparation.get("quiescence_root_sha256"),
                  quiescence["quiescence_root_sha256"], "preparation_quiescence_binding")
    require_exact(preparation.get("measurement_contamination_root_sha256"),
                  contamination["measurement_contamination_root_sha256"],
                  "preparation_contamination_binding")
    require_exact(preparation.get("executable_set_root_sha256"), executable_root,
                  "preparation_executable_binding")
    require_exact(preparation.get("oracle_ownership_root_sha256"), ownership_root,
                  "preparation_ownership_binding")
    require_exact(preparation.get("candidate", {}).get("binary_sha256"),
                  executables["candidate-binary"]["sha256"],
                  "preparation_candidate_executable_hash")
    require_exact(preparation.get("candidate", {}).get("binary_size_bytes"),
                  executables["candidate-binary"]["size_bytes"],
                  "preparation_candidate_executable_size")
    require_exact(receipt.get("schema"), SCHEMA, "receipt_schema")
    verify_embedded_root(receipt, "receipt_root_sha256", "receipt")
    require_exact(receipt.get("preparation_root_sha256"), preparation["preparation_root_sha256"],
                  "receipt_preparation_root")
    require_exact(receipt.get("quiescence_root_sha256"),
                  quiescence["quiescence_root_sha256"], "receipt_quiescence_root")
    require_exact(receipt.get("measurement_contamination_root_sha256"),
                  contamination["measurement_contamination_root_sha256"],
                  "receipt_contamination_root")
    require_exact(receipt.get("executable_set_root_sha256"), executable_root,
                  "receipt_executable_root")
    require_exact(receipt.get("oracle_ownership_root_sha256"), ownership_root,
                  "receipt_ownership_root")
    require_exact(receipt.get("transaction_id"), preparation.get("transaction_id"),
                  "receipt_transaction_id")
    require_exact(receipt.get("resource_root_sha256"), resource["resource_root_sha256"],
                  "receipt_resource_root")
    require_exact(receipt.get("parity_root_sha256"), parity["parity_root_sha256"],
                  "receipt_parity_root")
    verdict = receipt.get("verdict")
    require(verdict in {"S1C3_DEPLOYMENT_PASS", "S1C3_ROLLBACK_PASS", "S1C3_VETO"},
            "verdict_invalid")

    before = verify_service_snapshot(receipt.get("services_before"), "services_before")
    after = verify_service_snapshot(receipt.get("services_after"), "services_after")
    survival = verify_service_snapshot(receipt.get("services_survival"), "services_survival")
    rollback = verdict == "S1C3_ROLLBACK_PASS"
    expected_binary = BASELINE_BINARY_SHA256 if rollback else preparation["candidate"]["binary_sha256"]
    expected_config = BASELINE_CONFIG_SHA256 if rollback else CANDIDATE_CONFIG_SHA256
    require_exact(receipt.get("installed_binary_sha256"), expected_binary, "installed_binary")
    require_exact(receipt.get("installed_config_sha256"), expected_config, "installed_config")

    old_pid = before[TRANSITION_UNIT]["main_pid"]
    new_pid = after[TRANSITION_UNIT]["main_pid"]
    require(new_pid != old_pid, "transition_pid_did_not_change")
    require_exact(survival[TRANSITION_UNIT]["main_pid"], new_pid, "transition_survival_pid")
    require_exact(after[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"],
                  "transition_nrestarts")
    require_exact(survival[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"],
                  "transition_survival_nrestarts")
    for unit in UNTOUCHED_UNITS:
        require_exact(after[unit], before[unit], f"untouched_after_{unit}")
        require_exact(survival[unit], before[unit], f"untouched_survival_{unit}")

    verify_connector(receipt.get("connector_before"), receipt.get("connector_after"))
    verify_journal(receipt.get("journal_before"), receipt.get("journal_after"), rollback=rollback)
    require_exact(receipt.get("survival_seconds"), 15, "survival_seconds")
    require_exact(receipt.get("startup_log_clean"), True, "startup_log")
    require_exact(receipt.get("health_semantics_preserved"), True, "health_semantics")
    require_exact(receipt.get("route_probe_equivalent"), True, "route_probe")
    require_exact(receipt.get("active_packages_preserved"), True, "active_packages")
    require_exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    require_exact(receipt.get("runtime_parity_failures_after"), 0, "runtime_parity_failures")
    immutable_after = receipt.get("immutable_after", {})
    require_exact(immutable_after.get("unit_sha256"), UNIT_SHA256, "final_unit")
    require_exact(immutable_after.get("phase_config_sha256"), PHASE_CONFIG_SHA256, "final_phase")
    require_exact(immutable_after.get("authority_config_sha256"), AUTHORITY_CONFIG_SHA256,
                  "final_authority")
    if verdict == "S1C3_DEPLOYMENT_PASS":
        environment = receipt.get("capture_environment", {})
        require_exact(environment.get("NANDO_GROUNDED_DECISION_SHADOW_ENABLED"), "1",
                      "capture_enabled")
        require_exact(environment.get("NANDO_GROUNDED_DECISION_JOURNAL"),
                      "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1",
                      "capture_journal")
        require_exact(receipt.get("capture_available"), True, "capture_available")
    require(verdict != "S1C3_VETO", "terminal_veto")
    return {
        "schema": "nando.s1c3-verification.v4",
        "valid": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt["receipt_root_sha256"],
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": ownership_root,
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("transaction_directory", type=Path)
    parser.add_argument("--allow-rollback", action="store_true")
    args = parser.parse_args()
    try:
        result = verify_receipt(args.transaction_directory)
        if result["verdict"] == "S1C3_ROLLBACK_PASS" and not args.allow_rollback:
            raise InvalidReceipt("rollback_is_not_deployment_pass")
    except InvalidReceipt as error:
        print(json.dumps({"schema": "nando.s1c3-verification.v4", "valid": False,
                          "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
