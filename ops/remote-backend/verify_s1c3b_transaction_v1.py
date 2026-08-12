#!/usr/bin/env python3
"""Independent verifier for the frozen S1C-3B transaction evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

import s1c3b_remote_transaction_v1 as executor

try:
    import verify_s1c3_transaction_v7 as legacy_verifier
except ModuleNotFoundError:
    import importlib.util

    _legacy_path = Path(__file__).with_name("verify_s1c3_transaction_v7.py")
    _legacy_spec = importlib.util.spec_from_file_location(
        "verify_s1c3_transaction_v7", _legacy_path
    )
    if _legacy_spec is None or _legacy_spec.loader is None:
        raise
    legacy_verifier = importlib.util.module_from_spec(_legacy_spec)
    _legacy_spec.loader.exec_module(legacy_verifier)


SCHEMA = executor.RECEIPT_SCHEMA
PREPARATION_SCHEMA = executor.PREPARATION_SCHEMA
RESOURCE_SCHEMA = executor.RESOURCE_SCHEMA
PARITY_SCHEMA = executor.PARITY_SCHEMA
MONITOR_SCHEMA = executor.MONITOR_SCHEMA
PREDEPLOYMENT_SCHEMA = executor.PREDEPLOYMENT_SCHEMA
STATE_SCHEMA = executor.STATE_SCHEMA
OWNERSHIP_SCHEMA = executor.OWNERSHIP_SCHEMA
OWNERSHIP_ROW_SCHEMA = executor.OWNERSHIP_ROW_SCHEMA
PROCESS_SNAPSHOT_SCHEMA = executor.PROCESS_SNAPSHOT_SCHEMA
MEASUREMENT_CPU = executor.MEASUREMENT_CPU
MEASUREMENT_LABELS = list(executor.MEASUREMENT_LABELS)
ROUND_COUNT = executor.ROUND_COUNT
FLOOR_RECORDS = executor.FLOOR_RECORDS

PAPER_COMMIT = executor.PAPER_COMMIT
PAPER_TREE = executor.PAPER_TREE
PAPER_MANIFEST_ROOT = executor.PAPER_MANIFEST_ROOT
PAPER_VERIFICATION_SHA256 = executor.PAPER_VERIFICATION_SHA256
PAPER_CRITIQUE_SHA256 = executor.PAPER_CRITIQUE_SHA256
CANDIDATE_COMMIT = executor.CANDIDATE_COMMIT
CANDIDATE_TREE = executor.CANDIDATE_TREE
CARGO_LOCK_SHA256 = executor.CARGO_LOCK_SHA256
CANDIDATE_CONFIG_SHA256 = executor.CANDIDATE_CONFIG_SHA256
PRODUCTION_PROJECTION_PATH = str(executor.PRODUCTION_PROJECTION_PATH)
PRODUCTION_PROJECTION_SCHEMA = executor.PRODUCTION_PROJECTION_SCHEMA
PRODUCTION_PROJECTION_SHA256 = executor.PRODUCTION_PROJECTION_SHA256
BASELINE_BINARY_SHA256 = executor.BASELINE_BINARY_SHA256
BASELINE_CONFIG_SHA256 = executor.BASELINE_CONFIG_SHA256
CURRENT_RECEIPT_ROOT = executor.CURRENT_RECEIPT_ROOT
CURRENT_RECEIPT_FILE_SHA256 = executor.CURRENT_RECEIPT_FILE_SHA256
CURRENT_RECEIPT_COMMIT = executor.CURRENT_RECEIPT_COMMIT
CURRENT_RECEIPT_TREE = executor.CURRENT_RECEIPT_TREE

UNIT_SHA256 = executor.legacy.UNIT_SHA256
PHASE_CONFIG_SHA256 = executor.legacy.PHASE_CONFIG_SHA256
AUTHORITY_CONFIG_SHA256 = executor.legacy.AUTHORITY_CONFIG_SHA256
TRANSITION_UNIT = executor.TRANSITION_UNIT
UNTOUCHED_UNITS = executor.UNTOUCHED_UNITS
ALL_UNITS = (TRANSITION_UNIT, *UNTOUCHED_UNITS)

HOT_RE = executor.HOT_RE
SYNC_RE = executor.SYNC_RE
STAGE_SYNC_RE = executor.STAGE_SYNC_RE
IDLE_RE = executor.IDLE_RE
AFFINITY_RE = executor.AFFINITY_RE


class InvalidReceipt(ValueError):
    pass


def canonical_bytes(value: Any, omit_field: str | None = None) -> bytes:
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


def require_int(value: Any, label: str) -> int:
    require(type(value) is int and value >= 0, f"{label}_invalid")
    return value


def require_number(value: Any, label: str) -> float:
    require(type(value) in (int, float) and value >= 0, f"{label}_invalid")
    return float(value)


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InvalidReceipt(f"json_invalid:{path.name}") from error
    require(isinstance(value, dict), f"json_not_object:{path.name}")
    return value


def verify_root(value: dict[str, Any], field: str, label: str) -> str:
    root = require_hash(value.get(field), f"{label}_root")
    require_exact(digest(canonical_bytes(value, field)), root, f"{label}_root")
    return root


def percentile(samples: list[int], percent: int) -> int:
    ordered = sorted(samples)
    index = max(0, (len(ordered) * percent + 99) // 100 - 1)
    return ordered[index]


def verify_ownership(path: Path) -> tuple[dict[str, Any], str]:
    ownership = load_json(path)
    require_exact(ownership.get("schema"), OWNERSHIP_SCHEMA, "ownership_schema")
    root = verify_root(ownership, "ownership_root_sha256", "ownership")
    require((path.stat().st_mode & 0o7777) == 0o400, "ownership_file_mode")
    transaction_id = ownership.get("transaction_id")
    require(isinstance(transaction_id, str) and transaction_id, "ownership_transaction_id")
    require_exact(
        ownership.get("build_user"),
        {"name": "e", "uid": 1000, "gid": 1000},
        "ownership_build_user",
    )
    expected_manifests = {
        label: legacy_verifier.digest(
            legacy_verifier.oracle_manifest(
                legacy_verifier.expected_oracle_paths(transaction_id, label)["source"]
            )
        )
        for label in ("baseline", "candidate")
    }
    require_exact(
        ownership.get("oracle_build_contract"),
        {
            "schema": executor.legacy.ORACLE_BUILD_CONTRACT_SCHEMA,
            "package": {
                "name": executor.legacy.ORACLE_PACKAGE_NAME,
                "version": executor.legacy.ORACLE_PACKAGE_VERSION,
                "edition": executor.legacy.ORACLE_EDITION,
            },
            "oracle_source_sha256": executor.legacy.ORACLE_SOURCE_SHA256,
            "cargo_lock_sha256": executor.legacy.ORACLE_LOCK_SHA256,
            "offline": True,
            "locked": True,
            "cargo_net_offline": "true",
            "manifest_sha256": expected_manifests,
        },
        "ownership_build_contract",
    )
    rows = ownership.get("rows")
    require(isinstance(rows, dict) and set(rows) == {"baseline", "candidate"}, "ownership_rows")
    require_exact(digest(canonical_bytes(rows)), ownership.get("rows_root_sha256"), "ownership_rows_root")
    for label, row in rows.items():
        require(isinstance(row, dict), f"ownership_{label}_not_object")
        require_exact(row.get("schema"), OWNERSHIP_ROW_SCHEMA, f"ownership_{label}_schema")
        verify_root(row, "ownership_row_root_sha256", f"ownership_{label}")
        require_exact(row.get("main_rs_sha256"), executor.legacy.ORACLE_SOURCE_SHA256, f"ownership_{label}_source")
        paths = legacy_verifier.expected_oracle_paths(transaction_id, label)
        expected_files = {
            "workspace": (paths["workspace"], "0750"),
            "src": (f'{paths["workspace"]}/src', "0750"),
            "cargo_toml": (paths["manifest"], "0640"),
            "main_rs": (paths["main"], "0640"),
        }
        for field, (expected_path, expected_mode) in expected_files.items():
            identity = row.get(field)
            require(isinstance(identity, dict), f"ownership_{label}_{field}_shape")
            require_exact(identity.get("path"), expected_path, f"ownership_{label}_{field}_path")
            require_exact(identity.get("uid"), 1000, f"ownership_{label}_{field}_uid")
            require_exact(identity.get("gid"), 1000, f"ownership_{label}_{field}_gid")
            require_exact(identity.get("mode_octal"), expected_mode, f"ownership_{label}_{field}_mode")
        lock = row.get("cargo_lock", {})
        require_exact(lock.get("path"), paths["lock"], f"ownership_{label}_lock_path")
        require_exact(lock.get("uid"), 1000, f"ownership_{label}_lock_uid")
        require_exact(lock.get("gid"), 1000, f"ownership_{label}_lock_gid")
        require_exact(lock.get("mode_octal"), "0640", f"ownership_{label}_lock_mode")
        require_exact(lock.get("sha256_before_build"), executor.legacy.ORACLE_LOCK_SHA256, f"ownership_{label}_lock_before")
        require_exact(lock.get("sha256_after_build"), executor.legacy.ORACLE_LOCK_SHA256, f"ownership_{label}_lock_after")
        require_exact(row.get("manifest_sha256"), expected_manifests[label], f"ownership_{label}_manifest")
        command = row.get("build_command")
        require_exact(
            command,
            [*executor.legacy.ORACLE_CARGO_COMMAND_PREFIX, paths["manifest"]],
            f"ownership_{label}_build_command",
        )
        require_exact(
            row.get("build_environment"),
            {**executor.legacy.ORACLE_CARGO_ENVIRONMENT, "CARGO_TARGET_DIR": paths["target"]},
            f"ownership_{label}_build_environment",
        )
        require_exact(row.get("target_directory"), paths["target"], f"ownership_{label}_target")
        require_exact(row.get("executable_path"), paths["executable"], f"ownership_{label}_executable")
        require_exact(
            row.get("probe"),
            {
                "writer_uid": 1000,
                "writer_gid": 1000,
                "probe_uid": 1000,
                "probe_gid": 1000,
                "probe_mode_octal": "0600",
                "create_fsync_unlink_pass": True,
                "directory_fsync_pass": True,
            },
            f"ownership_{label}_probe",
        )
        require_exact(row.get("probe_retained"), False, f"ownership_{label}_probe_retained")
    return ownership, root


def verify_executable_set(
    value: Any, label: str, expected_keys: set[str]
) -> dict[str, Any]:
    require(isinstance(value, dict) and set(value) == expected_keys, f"{label}_key_set")
    for name, row in value.items():
        require(isinstance(row, dict), f"{label}_{name}_not_object")
        require(
            isinstance(row.get("path"), str) and row["path"].startswith("/"),
            f"{label}_{name}_path",
        )
        require_hash(row.get("sha256"), f"{label}_{name}_sha256")
        require(
            require_int(row.get("size_bytes"), f"{label}_{name}_size") > 0,
            f"{label}_{name}_empty",
        )
        mode = row.get("mode_octal")
        require(
            isinstance(mode, str) and len(mode) == 4 and int(mode, 8) & 0o111,
            f"{label}_{name}_mode",
        )
        require(
            isinstance(row.get("source_identity"), str) and row["source_identity"],
            f"{label}_{name}_source",
        )
    return value


def verify_executables(
    path: Path, expected_root: str
) -> tuple[dict[str, Any], bool]:
    receipt = load_json(path)
    require_exact(receipt.get("schema"), "nando.s1c3b-executable-identities.v1", "executables_schema")
    verify_root(receipt, "executable_identities_root_sha256", "executables_receipt")
    before = receipt.get("before")
    after = receipt.get("after")
    expected_keys = {
        "candidate-binary",
        "test-response-actor",
        "test-transition-serving",
        "parity-baseline",
        "parity-candidate",
        "python-runtime",
        "affinity-wrapper",
        "filesystem-floor-probe",
    }
    before = verify_executable_set(before, "executable_before", expected_keys)
    after = verify_executable_set(after, "executable_after", expected_keys)
    require_exact(digest(canonical_bytes(before)), expected_root, "executable_set_root")
    return before, after != before


def independently_parse_parent_pid(stat_raw: str) -> int:
    closing = stat_raw.rfind(")")
    tail = stat_raw[closing + 2 :].split()
    require(closing >= 0 and len(tail) >= 2, "process_parent_stat_shape")
    try:
        return int(tail[1])
    except ValueError as error:
        raise InvalidReceipt("process_parent_stat_value") from error


def independently_transaction_owned_builds(
    snapshot: dict[str, Any], executor_pid: int
) -> list[dict[str, Any]]:
    parents: dict[int, int] = {}
    rows: dict[int, dict[str, Any]] = {}
    for row in snapshot["rows"]:
        opening = row.get("opening_stat", {})
        if opening.get("status") != "VALUE":
            continue
        pid = require_int(row.get("pid"), "process_owned_pid")
        parents[pid] = independently_parse_parent_pid(opening.get("raw", ""))
        rows[pid] = row
    owned = []
    for pid, row in rows.items():
        cursor = pid
        seen: set[int] = set()
        while cursor in parents and cursor not in seen and cursor != 1:
            seen.add(cursor)
            cursor = parents[cursor]
            if cursor == executor_pid:
                if row.get("forbidden_names"):
                    owned.append({
                        "pid": pid,
                        "parent_chain": sorted(seen),
                        "matched_names": row["forbidden_names"],
                    })
                break
    return owned


def verify_command(row: Any, label: str, executables: dict[str, Any]) -> dict[str, Any]:
    require(isinstance(row, dict), f"command_{label}_not_object")
    require_exact(row.get("label"), label, f"command_{label}_label")
    require_exact(row.get("requested_affinity"), [MEASUREMENT_CPU], f"command_{label}_requested_affinity")
    require_exact(row.get("observed_affinity"), [MEASUREMENT_CPU], f"command_{label}_observed_affinity")
    path = row.get("executable")
    require(isinstance(path, str) and path.startswith("/"), f"command_{label}_executable")
    sha = require_hash(row.get("executable_sha256"), f"command_{label}_sha")
    require_exact(row.get("wrapper_reported_executable_sha256"), sha, f"command_{label}_wrapper_sha")
    known = {identity["path"]: identity["sha256"] for identity in executables.values()}
    require_exact(known.get(path), sha, f"command_{label}_known_executable")
    require_exact(row.get("error"), None, f"command_{label}_error")
    require(type(row.get("returncode")) is int, f"command_{label}_returncode")
    return row


def expected_metric_from_log(directory: Path, label: str) -> tuple[dict[str, Any] | None, int | None]:
    path = directory / "evidence" / f"{label}.log"
    require(path.is_file(), f"metric_log_missing:{label}")
    text = path.read_text(encoding="utf-8", errors="replace")
    if label.startswith("hot-"):
        match = HOT_RE.search(text)
        fields = ("p99_ns", "no_goal_p99_ns", "hard_max_ns", "samples")
    elif label.startswith("single-sync-"):
        match = SYNC_RE.search(text)
        fields = ("p99_ns", "hard_max_ns", "samples", "segments")
    elif label.startswith("three-sync-"):
        match = STAGE_SYNC_RE.search(text)
        fields = (
            "precommit_p99_ns",
            "precommit_hard_max_ns",
            "settlement_p99_ns",
            "settlement_hard_max_ns",
            "episode_p99_ns",
            "episode_hard_max_ns",
            "samples",
        )
    elif label == "idle":
        match = IDLE_RE.search(text)
        fields = ("elapsed_ticks", "ticks_per_second", "percent_of_one_core")
    else:
        raise InvalidReceipt(f"unknown_metric_label:{label}")
    if match is None:
        return None, None
    values: list[Any] = list(match.groups())
    if label == "idle":
        values = [int(values[0]), int(values[1]), float(values[2])]
    else:
        values = [int(value) for value in values]
    return dict(zip(fields, values, strict=True)), 0 if "test result: ok. 1 passed; 0 failed;" in text else 101


def verify_floor(
    directory: Path, row: Any, index: int, commands: dict[str, Any]
) -> None:
    require(isinstance(row, dict), f"floor_{index}_not_object")
    round_index = index // 2 + 1
    position = "before" if index % 2 == 0 else "after"
    label = f"floor-{position}-{round_index}"
    require_exact(row.get("label"), label, f"floor_{index}_label")
    require_exact(row.get("round"), round_index, f"floor_{index}_round")
    require_exact(row.get("position"), position, f"floor_{index}_position")
    require_exact(row.get("diagnostic_only"), True, f"floor_{index}_authority")
    samples = row.get("samples_ns")
    require(isinstance(samples, list) and len(samples) == FLOOR_RECORDS, f"floor_{index}_denominator")
    require(all(type(value) is int and value >= 0 for value in samples), f"floor_{index}_samples")
    require_exact(row.get("records"), FLOOR_RECORDS, f"floor_{index}_records")
    require_exact(row.get("samples_root_sha256"), digest(canonical_bytes(samples)), f"floor_{index}_root")
    log_path = directory / "evidence" / f"{label}.log"
    require(log_path.is_file(), f"floor_{index}_log_missing")
    try:
        logged_samples = json.loads(
            log_path.read_text(encoding="utf-8").splitlines()[-1]
        )["samples_ns"]
    except (OSError, json.JSONDecodeError, KeyError, IndexError) as error:
        raise InvalidReceipt(f"floor_{index}_log_invalid") from error
    require_exact(samples, logged_samples, f"floor_{index}_log_samples")
    require_exact(row.get("p50_ns"), percentile(samples, 50), f"floor_{index}_p50")
    require_exact(row.get("p99_ns"), percentile(samples, 99), f"floor_{index}_p99")
    require_exact(row.get("hard_max_ns"), max(samples), f"floor_{index}_max")
    require_exact(row.get("returncode"), 0, f"floor_{index}_returncode")
    require_exact(row.get("error"), None, f"floor_{index}_error")
    require_exact(row.get("command"), commands[label], f"floor_{index}_command")
    filesystem = row.get("filesystem")
    require(isinstance(filesystem, dict) and filesystem.get("findmnt_returncode") == 0, f"floor_{index}_filesystem")


def verify_metric_row(
    directory: Path,
    row: Any,
    label: str,
    expected_test: str,
    commands: dict[str, Any],
) -> dict[str, Any]:
    require(isinstance(row, dict), f"metric_{label}_not_object")
    require_exact(row.get("label"), label, f"metric_{label}_label")
    require_exact(row.get("test"), expected_test, f"metric_{label}_test")
    require_exact(row.get("command"), commands[label], f"metric_{label}_command")
    expected, expected_return = expected_metric_from_log(directory, label)
    require_exact(row.get("metric_present"), expected is not None, f"metric_{label}_present")
    require_exact(row.get("metrics"), expected, f"metric_{label}_values")
    require_exact(row.get("returncode"), expected_return, f"metric_{label}_returncode")
    require_exact(row.get("test_assertion_pass"), expected_return == 0, f"metric_{label}_assertion")
    require_exact(
        row.get("output_sha256"),
        digest((directory / "evidence" / f"{label}.log").read_bytes()),
        f"metric_{label}_output",
    )
    return expected or {}


def verify_monitor(monitor: dict[str, Any], executable_root: str) -> tuple[str, bool]:
    require_exact(monitor.get("schema"), MONITOR_SCHEMA, "monitor_schema")
    root = verify_root(monitor, "monitor_root_sha256", "monitor")
    require_exact(monitor.get("measurement_cpu"), MEASUREMENT_CPU, "monitor_cpu")
    require_exact(monitor.get("measurement_sibling"), executor.MEASUREMENT_SIBLING, "monitor_sibling")
    executor_pid = require_int(monitor.get("executor_pid"), "monitor_executor_pid")
    require(executor_pid > 1, "monitor_executor_pid_zero")
    require_exact(monitor.get("monitor_interval_seconds"), 0.5, "monitor_interval")
    require_exact(monitor.get("maximum_sample_gap_seconds"), 2.0, "monitor_gap_limit")
    require_number(monitor.get("observed_max_sample_gap_seconds"), "monitor_observed_gap")
    require_exact(monitor.get("metric_labels"), MEASUREMENT_LABELS, "monitor_labels")
    require_exact(monitor.get("executable_set_root_sha256"), executable_root, "monitor_executable_root")
    boundaries = monitor.get("boundaries")
    require(isinstance(boundaries, list) and len(boundaries) == len(MEASUREMENT_LABELS) * 2, "monitor_boundary_count")
    expected_boundaries = [
        (label, phase) for label in MEASUREMENT_LABELS for phase in ("before", "after")
    ]
    require_exact([(row.get("label"), row.get("phase")) for row in boundaries], expected_boundaries, "monitor_boundary_order")
    samples = monitor.get("samples")
    require(isinstance(samples, list) and len(samples) >= len(boundaries) + 2, "monitor_samples")
    monotonic = [require_int(row.get("monotonic_ns"), f"monitor_sample_{index}_time") for index, row in enumerate(samples)]
    require(monotonic == sorted(monotonic), "monitor_sample_order")
    gaps = [(right - left) / 1_000_000_000 for left, right in zip(monotonic, monotonic[1:])]
    require_exact(monitor.get("observed_max_sample_gap_seconds"), max(gaps, default=0.0), "monitor_gap_recompute")
    aggregate_owned = []
    for index, row in enumerate(samples):
        require(isinstance(row.get("cpu_counters"), dict) and set(row["cpu_counters"]) == {"4", "5"}, f"monitor_sample_{index}_cpus")
        require(isinstance(row.get("cpu_pressure"), dict), f"monitor_sample_{index}_cpu_pressure")
        require(isinstance(row.get("io_pressure"), dict), f"monitor_sample_{index}_io_pressure")
        require(require_int(row.get("memory_available_bytes"), f"monitor_sample_{index}_memory") > 0, f"monitor_sample_{index}_memory_zero")
        require(isinstance(row.get("block_device_counters"), list), f"monitor_sample_{index}_disk")
        require(isinstance(row.get("process_snapshot"), dict), f"monitor_sample_{index}_process")
        require_exact(row["process_snapshot"].get("process_snapshot_root_sha256"), row.get("process_snapshot_root_sha256"), f"monitor_sample_{index}_process_root")
        try:
            legacy_verifier.verify_process_snapshot(
                row["process_snapshot"], f"monitor_sample_{index}_process"
            )
        except legacy_verifier.InvalidReceipt as error:
            raise InvalidReceipt(str(error)) from error
        require_exact(
            row.get("foreign_build_processes"),
            row["process_snapshot"].get("forbidden_process_matches"),
            f"monitor_sample_{index}_foreign_builds",
        )
        require_exact(
            row.get("unresolved_processes"),
            row["process_snapshot"].get("unresolved_rows"),
            f"monitor_sample_{index}_unresolved",
        )
        recomputed_owned = independently_transaction_owned_builds(
            row["process_snapshot"], executor_pid
        )
        require_exact(
            row.get("transaction_owned_build_processes"),
            recomputed_owned,
            f"monitor_sample_{index}_owned_compiler",
        )
        aggregate_owned.extend(
            {"sample_index": index, "process": process}
            for process in recomputed_owned
        )
        require(MEASUREMENT_CPU not in row.get("observer_affinity", []), f"monitor_sample_{index}_observer_affinity")
    for boundary in boundaries:
        index = require_int(boundary.get("sample_index"), "monitor_boundary_index")
        require(index < len(samples), "monitor_boundary_index_range")
        require_exact(samples[index].get("kind"), f'boundary-{boundary["phase"]}', "monitor_boundary_kind")
        require_exact(samples[index].get("label"), boundary["label"], "monitor_boundary_label")
        require_exact(samples[index].get("monotonic_ns"), boundary.get("monotonic_ns"), "monitor_boundary_time")
    require_exact(monitor.get("transaction_owned_build_processes"), aggregate_owned, "monitor_owned_compilers")
    errors = monitor.get("monitor_errors")
    require(isinstance(errors, list) and all(isinstance(error, str) for error in errors), "monitor_errors_shape")
    instrument_pass = not errors and not aggregate_owned and max(gaps, default=0.0) <= 2.0
    require_exact(monitor.get("instrument_pass"), instrument_pass, "monitor_instrument_pass")
    return root, instrument_pass


def verify_resource(
    directory: Path,
    resource: dict[str, Any],
    monitor: dict[str, Any],
    executables: dict[str, Any],
    executable_root: str,
    ownership_root: str,
    executable_drift: bool,
) -> str:
    require_exact(resource.get("schema"), RESOURCE_SCHEMA, "resource_schema")
    root = verify_root(resource, "resource_root_sha256", "resource")
    require_exact(resource.get("candidate_commit"), CANDIDATE_COMMIT, "resource_candidate")
    require_exact(resource.get("measurement_cpu"), MEASUREMENT_CPU, "resource_cpu")
    require_exact(resource.get("round_count"), ROUND_COUNT, "resource_round_count")
    require_exact(resource.get("executable_set_root_sha256"), executable_root, "resource_executable_root")
    require_exact(resource.get("oracle_ownership_root_sha256"), ownership_root, "resource_ownership_root")
    require_exact(resource.get("monitor_root_sha256"), monitor["monitor_root_sha256"], "resource_monitor_root")
    commands_list = monitor.get("commands")
    require(isinstance(commands_list, list) and len(commands_list) == len(MEASUREMENT_LABELS), "monitor_command_count")
    require_exact([row.get("label") for row in commands_list], MEASUREMENT_LABELS, "monitor_command_order")
    commands = {row["label"]: verify_command(row, row["label"], executables) for row in commands_list}

    floors = resource.get("floor_probes")
    require(isinstance(floors, list) and len(floors) == ROUND_COUNT * 2, "floor_probe_count")
    for index, row in enumerate(floors):
        verify_floor(directory, row, index, commands)
    require_exact(
        [row.get("filesystem") for row in floors],
        [floors[0].get("filesystem")] * len(floors),
        "floor_filesystem_identity",
    )

    metrics = resource.get("metrics")
    require(isinstance(metrics, dict), "resource_metrics")
    failures: list[str] = []
    instruments: list[str] = []
    if executable_drift:
        instruments.append("executable_drift_after_measurement")
    if not monitor.get("instrument_pass"):
        instruments.append("monitor_instrument_failure")
    sets = (
        ("hot_latency", "hot", executor.HOT_TEST),
        ("single_ledger_sync", "single-sync", executor.SINGLE_SYNC_TEST),
        ("three_ledger_sync", "three-sync", executor.THREE_SYNC_TEST),
    )
    for name, prefix, test in sets:
        rows = metrics.get(name)
        require(isinstance(rows, list) and len(rows) == ROUND_COUNT, f"{name}_count")
        for index, row in enumerate(rows, start=1):
            label = f"{prefix}-{index}"
            metric = verify_metric_row(directory, row, label, test, commands)
            if not row.get("test_assertion_pass"):
                instruments.append(f"{label}:test_assertion_failed")
            if not metric:
                instruments.append(f"{label}:metric_missing")
                continue
            if name == "hot_latency":
                if metric["samples"] != 4096:
                    instruments.append(f"{label}:denominator")
                if metric["p99_ns"] > 1_000_000:
                    failures.append(f"{label}:matched_p99")
                if metric["no_goal_p99_ns"] > 250_000:
                    failures.append(f"{label}:no_goal_p99")
                if metric["hard_max_ns"] > 2_000_000:
                    failures.append(f"{label}:hard_max")
            elif name == "single_ledger_sync":
                if metric["samples"] != 1024:
                    instruments.append(f"{label}:denominator")
                if metric["p99_ns"] > 5_000_000:
                    failures.append(f"{label}:p99")
                if metric["hard_max_ns"] > 20_000_000:
                    failures.append(f"{label}:hard_max")
            else:
                if metric["samples"] != 256:
                    instruments.append(f"{label}:denominator")
                for field in ("precommit_p99_ns", "settlement_p99_ns"):
                    if metric[field] > 5_000_000:
                        failures.append(f"{label}:{field}")
                for field in ("precommit_hard_max_ns", "settlement_hard_max_ns", "episode_hard_max_ns"):
                    if metric[field] > 20_000_000:
                        failures.append(f"{label}:{field}")
    idle = metrics.get("idle_cpu")
    idle_metric = verify_metric_row(directory, idle, "idle", executor.IDLE_TEST, commands)
    if not idle_metric:
        instruments.append("idle:metric_missing")
    elif idle_metric["percent_of_one_core"] > 0.25:
        failures.append("idle:percent_of_one_core")
    if not idle.get("test_assertion_pass"):
        instruments.append("idle:test_assertion_failed")
    rss = metrics.get("rss")
    require(isinstance(rss, dict), "rss_not_object")
    rss_rows = rss.get("rows")
    require(isinstance(rss_rows, list) and len(rss_rows) == 2, "rss_rows")
    for index, expected_label in enumerate(("capture_off", "capture_on")):
        row = rss_rows[index]
        require_exact(row.get("label"), expected_label, f"rss_{index}_label")
        verify_command(row.get("command"), f"rss-{expected_label}", executables)
        if row.get("error") is not None or row.get("rss_bytes") is None or row.get("sample_count") != 20:
            instruments.append("rss:incomplete")
    if "rss:incomplete" not in instruments:
        delta = max(0, rss_rows[1]["rss_bytes"] - rss_rows[0]["rss_bytes"])
        require_exact(rss.get("delta_bytes"), delta, "rss_delta_recompute")
        if delta > 16 * 1024 * 1024:
            failures.append("rss:delta")
    parity = load_json(directory / "parity-receipt.json")
    require_exact(parity.get("schema"), PARITY_SCHEMA, "parity_schema")
    parity_root = verify_root(parity, "parity_root_sha256", "parity")
    require_exact(resource.get("parity_root_sha256"), parity_root, "resource_parity_root")
    rows = parity.get("rows")
    require(isinstance(rows, list) and len(rows) == 2, "parity_rows")
    for row, label in zip(rows, ("baseline", "candidate"), strict=True):
        require_exact(row.get("label"), label, f"parity_{label}_label")
        require_exact(row.get("command"), commands[f"parity-{label}"], f"parity_{label}_command")
        log_path = directory / "evidence" / f"parity-{label}.log"
        require(log_path.is_file(), f"parity_{label}_log_missing")
        raw = log_path.read_bytes()
        lines = raw.splitlines()
        require(len(lines) >= 2, f"parity_{label}_log_shape")
        payload = b"\n".join(lines[1:])
        require_exact(row.get("output_sha256"), digest(raw), f"parity_{label}_raw_root")
        require_exact(row.get("row_count"), len(lines) - 1, f"parity_{label}_row_count")
        require_exact(
            parity.get(f"{label}_output_sha256"),
            digest(payload),
            f"parity_{label}_payload_root",
        )
    if not parity.get("byte_identical") or parity.get("row_count") != 16:
        failures.append("parity:byte_identity")
    require_exact(parity.get("baseline_output_sha256"), parity.get("candidate_output_sha256"), "parity_output")

    require_exact(resource.get("resource_failures"), sorted(set(failures)), "resource_failures")
    require_exact(resource.get("instrument_failures"), sorted(set(instruments)), "instrument_failures")
    before_monitor = [
        failure
        for failure in instruments
        if failure
        not in {"monitor_instrument_failure", "executable_drift_after_measurement"}
    ]
    require_exact(
        resource.get("all_pass_before_monitor"),
        not failures and not before_monitor,
        "resource_all_pass_before_monitor",
    )
    require_exact(resource.get("all_pass"), not failures and not instruments, "resource_all_pass")
    return root


def verify_service_snapshot(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict) and set(value) == set(ALL_UNITS), f"{label}_unit_set")
    for unit, row in value.items():
        require_exact(row.get("active_state"), "active", f"{label}_{unit}_active")
        require(require_int(row.get("main_pid"), f"{label}_{unit}_pid") > 0, f"{label}_{unit}_pid_zero")
        require_int(row.get("nrestarts"), f"{label}_{unit}_restarts")
        require_hash(row.get("fragment_sha256"), f"{label}_{unit}_fragment")
    return value


def verify_identity(preparation: dict[str, Any]) -> None:
    require_exact(preparation.get("schema"), PREPARATION_SCHEMA, "preparation_schema")
    verify_root(preparation, "preparation_root_sha256", "preparation")
    paper = preparation.get("paper", {})
    require_exact(paper, {
        "commit": PAPER_COMMIT,
        "tree": PAPER_TREE,
        "manifest_root_sha256": PAPER_MANIFEST_ROOT,
        "verification_sha256": PAPER_VERIFICATION_SHA256,
        "critique_sha256": PAPER_CRITIQUE_SHA256,
    }, "paper")
    candidate = preparation.get("candidate", {})
    for actual, expected, label in (
        (candidate.get("source_commit"), CANDIDATE_COMMIT, "candidate_commit"),
        (candidate.get("source_tree"), CANDIDATE_TREE, "candidate_tree"),
        (candidate.get("cargo_lock_sha256"), CARGO_LOCK_SHA256, "candidate_lock"),
        (candidate.get("config_sha256"), CANDIDATE_CONFIG_SHA256, "candidate_config"),
        (candidate.get("production_projection_path"), PRODUCTION_PROJECTION_PATH, "candidate_projection_path"),
        (candidate.get("production_projection_schema"), PRODUCTION_PROJECTION_SCHEMA, "candidate_projection_schema"),
        (candidate.get("production_projection_sha256"), PRODUCTION_PROJECTION_SHA256, "candidate_projection_root"),
    ):
        require_exact(actual, expected, label)
    require_hash(candidate.get("binary_sha256"), "candidate_binary")
    require(require_int(candidate.get("binary_size_bytes"), "candidate_binary_size") > 0, "candidate_binary_empty")
    production = preparation.get("production", {})
    require_exact(production.get("receipt_root_sha256"), CURRENT_RECEIPT_ROOT, "production_receipt")
    require_exact(production.get("source_commit"), CURRENT_RECEIPT_COMMIT, "production_commit")
    require_exact(production.get("source_tree"), CURRENT_RECEIPT_TREE, "production_tree")
    require_exact(production.get("binary_sha256"), BASELINE_BINARY_SHA256, "production_binary")
    require_exact(production.get("config_sha256"), BASELINE_CONFIG_SHA256, "production_config")
    require_exact(preparation.get("measurement_cpu"), MEASUREMENT_CPU, "preparation_cpu")
    verify_service_snapshot(preparation.get("services_before"), "services_before")
    require_exact(preparation.get("economics_before"), {"false_accepts": 0, "runtime_parity_mismatches": 0}, "economics_before")
    rollback = preparation.get("rollback", {})
    entries = rollback.get("entries")
    require(isinstance(entries, list), "rollback_entries")
    require_exact({row.get("path") for row in entries}, {
        "nando-transition-serving",
        "transition-serving.env",
        "nando-transition-serving.service",
        "previous-deployment-receipt.json",
    }, "rollback_entry_set")
    manifest = "".join(
        f'{row["sha256"]} {row["size_bytes"]} {row["path"]}\n'
        for row in sorted(entries, key=lambda item: item["path"])
    ).encode()
    require_exact(rollback.get("manifest_root_sha256"), digest(manifest), "rollback_manifest")
    by_name = {row["path"]: row for row in entries}
    require_exact(by_name["nando-transition-serving"]["sha256"], BASELINE_BINARY_SHA256, "rollback_binary")
    require_exact(by_name["transition-serving.env"]["sha256"], BASELINE_CONFIG_SHA256, "rollback_config")
    require_exact(by_name["nando-transition-serving.service"]["sha256"], UNIT_SHA256, "rollback_unit")
    require_exact(
        by_name["previous-deployment-receipt.json"]["sha256"],
        CURRENT_RECEIPT_FILE_SHA256,
        "rollback_receipt_file",
    )


def verify_preparation(directory: Path) -> dict[str, Any]:
    ownership, ownership_root = verify_ownership(directory / "oracle-ownership-receipt.json")
    resource = load_json(directory / "resource-receipt.json")
    require_exact(resource.get("all_pass"), True, "predeployment_resource_pass")
    executable_root = require_hash(resource.get("executable_set_root_sha256"), "resource_executable_root")
    executables, executable_drift = verify_executables(
        directory / "executable-identities.json", executable_root
    )
    monitor = load_json(directory / "measurement-monitor-receipt.json")
    monitor_root, monitor_pass = verify_monitor(monitor, executable_root)
    require(monitor_pass, "predeployment_monitor_veto")
    resource_root = verify_resource(
        directory,
        resource,
        monitor,
        executables,
        executable_root,
        ownership_root,
        executable_drift,
    )
    preparation = load_json(directory / "preparation.json")
    verify_identity(preparation)
    require_exact(preparation.get("oracle_ownership_root_sha256"), ownership_root, "preparation_ownership_root")
    require_exact(preparation.get("monitor_root_sha256"), monitor_root, "preparation_monitor_root")
    require_exact(preparation.get("resource_root_sha256"), resource_root, "preparation_resource_root")
    parity = load_json(directory / "parity-receipt.json")
    require_exact(preparation.get("parity_root_sha256"), parity.get("parity_root_sha256"), "preparation_parity_root")
    result = {
        "schema": PREDEPLOYMENT_SCHEMA,
        "valid": True,
        "authority": True,
        "verdict": "S1C3B_PREPARATION_PASS",
        "preparation_root_sha256": preparation["preparation_root_sha256"],
        "oracle_ownership_root_sha256": ownership_root,
        "monitor_root_sha256": monitor_root,
        "resource_root_sha256": resource_root,
        "parity_root_sha256": parity["parity_root_sha256"],
    }
    result["predeployment_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def verify_resource_veto(directory: Path) -> dict[str, Any]:
    state = load_json(directory / "transaction-state.json")
    require_exact(state.get("schema"), STATE_SCHEMA, "resource_veto_state_schema")
    require_exact(state.get("state"), "RESOURCE_VETO", "resource_veto_state")
    require_exact(state.get("verdict"), "S1C3B_RESOURCE_VETO", "resource_veto_verdict")
    require_exact(state.get("production_mutation"), False, "resource_veto_mutation")
    ownership, ownership_root = verify_ownership(directory / "oracle-ownership-receipt.json")
    resource = load_json(directory / "resource-receipt.json")
    require_exact(resource.get("all_pass"), False, "resource_veto_all_pass")
    executable_root = require_hash(resource.get("executable_set_root_sha256"), "resource_executable_root")
    executables, executable_drift = verify_executables(
        directory / "executable-identities.json", executable_root
    )
    monitor = load_json(directory / "measurement-monitor-receipt.json")
    verify_monitor(monitor, executable_root)
    resource_root = verify_resource(
        directory,
        resource,
        monitor,
        executables,
        executable_root,
        ownership_root,
        executable_drift,
    )
    require_exact(state.get("resource_root_sha256"), resource_root, "resource_veto_root")
    require(not (directory / "preparation.json").exists(), "resource_veto_preparation_present")
    require(not (directory / "rollback").exists(), "resource_veto_rollback_present")
    require(not (directory / "deployment-receipt.json").exists(), "resource_veto_deployment_present")
    return {
        "schema": "nando.s1c3b-verification.v1",
        "valid": True,
        "authority": False,
        "verdict": "S1C3B_RESOURCE_VETO",
        "resource_root_sha256": resource_root,
    }


def connector_failure_reasons(before: Any, after: Any) -> list[str]:
    require(isinstance(before, dict) and isinstance(after, dict), "connector_shape")
    require_exact(before.get("active_state"), "active", "connector_before_active")
    fields = (
        "main_pid",
        "nrestarts",
        "route_receipt_failures",
        "command_sha256",
        "active_state",
    )
    return [
        f"connector_{field}"
        for field in fields
        if after.get(field) != before.get(field)
    ]


def semantic_health_projection(value: Any, label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label}_not_object")
    projected = {}
    for key, row in value.items():
        require(isinstance(row, dict) and isinstance(row.get("semantic"), dict), f"{label}_{key}_shape")
        projected[key] = row["semantic"]
    return projected


def verify_final(directory: Path) -> dict[str, Any]:
    expected_predeployment = verify_preparation(directory)
    recorded_predeployment = load_json(directory / "predeployment-verification.json")
    require_exact(recorded_predeployment, expected_predeployment, "predeployment_receipt")
    receipt = load_json(directory / "deployment-receipt.json")
    require_exact(receipt.get("schema"), SCHEMA, "receipt_schema")
    receipt_root = verify_root(receipt, "receipt_root_sha256", "receipt")
    verdict = receipt.get("verdict")
    require(
        verdict
        in {"S1C3B_DEPLOYMENT_PASS", "S1C3B_ROLLBACK_PASS", "S1C3B_VETO"},
        "receipt_verdict",
    )
    preparation = load_json(directory / "preparation.json")
    for field in (
        "preparation_root_sha256",
        "oracle_ownership_root_sha256",
        "monitor_root_sha256",
        "executable_set_root_sha256",
        "resource_root_sha256",
        "parity_root_sha256",
    ):
        require_exact(receipt.get(field), preparation.get(field), f"receipt_{field}")
    require_exact(receipt.get("predeployment_verification_root_sha256"), expected_predeployment["predeployment_verification_root_sha256"], "receipt_predeployment_root")
    before = verify_service_snapshot(receipt.get("services_before"), "final_before")
    after = verify_service_snapshot(receipt.get("services_after"), "final_after")
    survival = verify_service_snapshot(receipt.get("services_survival"), "final_survival")
    for unit in UNTOUCHED_UNITS:
        require_exact(after[unit], before[unit], f"untouched_after_{unit}")
        require_exact(survival[unit], before[unit], f"untouched_survival_{unit}")
    require(after[TRANSITION_UNIT]["main_pid"] != before[TRANSITION_UNIT]["main_pid"], "transition_pid_unchanged")
    require_exact(survival[TRANSITION_UNIT]["main_pid"], after[TRANSITION_UNIT]["main_pid"], "transition_survival_pid")
    require_exact(after[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"], "transition_after_restarts")
    require_exact(survival[TRANSITION_UNIT]["nrestarts"], before[TRANSITION_UNIT]["nrestarts"], "transition_survival_restarts")
    connector_failures = connector_failure_reasons(
        receipt.get("connector_before"), receipt.get("connector_after")
    )
    if verdict == "S1C3B_VETO":
        require(bool(connector_failures), "veto_without_connector_failure")
        require_exact(
            receipt.get("veto_reasons"), connector_failures, "veto_reasons"
        )
    else:
        require_exact(connector_failures, [], "connector_failures")
        require_exact(receipt.get("veto_reasons"), [], "veto_reasons")
    require_exact(receipt.get("survival_seconds"), 15, "survival_seconds")
    require_exact(receipt.get("startup_log_clean"), True, "startup_log_clean")
    health_before = semantic_health_projection(receipt.get("health_before"), "health_before")
    health_after = semantic_health_projection(receipt.get("health_after"), "health_after")
    health_survival = semantic_health_projection(receipt.get("health_survival"), "health_survival")
    require_exact(health_after, health_before, "health_after_semantics")
    require_exact(health_survival, health_before, "health_survival_semantics")
    require_exact(receipt.get("health_semantics_preserved"), True, "health_semantics")
    require_exact(receipt.get("route_probe_after"), receipt.get("route_probe_before"), "route_probe_after")
    require_exact(receipt.get("route_probe_survival"), receipt.get("route_probe_before"), "route_probe_survival")
    require_exact(receipt.get("route_probe_equivalent"), True, "route_probe")
    require_exact(receipt.get("active_packages_preserved"), True, "active_packages")
    require_exact(receipt.get("false_accepts_after"), 0, "false_accepts")
    require_exact(receipt.get("runtime_parity_failures_after"), 0, "runtime_parity")
    require(require_int(receipt.get("transition_rss_after"), "transition_rss_after")
            - require_int(receipt.get("transition_rss_before"), "transition_rss_before") <= 16 * 1024 * 1024,
            "transition_rss_delta")
    immutable = receipt.get("immutable_after", {})
    require_exact(immutable.get("unit_sha256"), UNIT_SHA256, "final_unit")
    require_exact(immutable.get("phase_config_sha256"), PHASE_CONFIG_SHA256, "final_phase")
    require_exact(immutable.get("authority_config_sha256"), AUTHORITY_CONFIG_SHA256, "final_authority")
    if verdict == "S1C3B_DEPLOYMENT_PASS":
        require_exact(receipt.get("installed_binary_sha256"), preparation["candidate"]["binary_sha256"], "installed_candidate")
        require_exact(receipt.get("installed_config_sha256"), CANDIDATE_CONFIG_SHA256, "installed_config")
        require_exact(receipt.get("capture_environment"), {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": str(executor.JOURNAL),
        }, "capture_environment")
        require_exact(receipt.get("capture_available"), True, "capture_available")
    else:
        require_exact(receipt.get("installed_binary_sha256"), BASELINE_BINARY_SHA256, "rollback_binary")
        require_exact(receipt.get("installed_config_sha256"), BASELINE_CONFIG_SHA256, "rollback_config")
        require_exact(receipt.get("capture_available"), False, "rollback_capture")
    result = {
        "schema": "nando.s1c3b-verification.v1",
        "valid": True,
        "authority": True,
        "verdict": verdict,
        "receipt_root_sha256": receipt_root,
        "preparation_root_sha256": preparation["preparation_root_sha256"],
    }
    result["final_verification_root_sha256"] = digest(canonical_bytes(result))
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("transaction_directory", type=Path)
    parser.add_argument("--pre-deployment", action="store_true")
    parser.add_argument("--allow-rollback", action="store_true")
    args = parser.parse_args()
    try:
        state_path = args.transaction_directory / "transaction-state.json"
        state = load_json(state_path)
        if state.get("state") == "RESOURCE_VETO":
            result = verify_resource_veto(args.transaction_directory)
        elif args.pre_deployment:
            result = verify_preparation(args.transaction_directory)
        else:
            result = verify_final(args.transaction_directory)
        if result["verdict"] != "S1C3B_DEPLOYMENT_PASS" and not args.allow_rollback:
            raise InvalidReceipt("terminal_non_deployment_verdict")
        if args.pre_deployment and result.get("authority") is not True:
            raise InvalidReceipt("predeployment_authority_false")
    except InvalidReceipt as error:
        print(json.dumps({"schema": "nando.s1c3b-verification.v1", "valid": False, "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
