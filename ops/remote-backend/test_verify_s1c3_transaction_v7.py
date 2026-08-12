#!/usr/bin/env python3
"""Fault-injection tests for the S1C-3 receipt verifier."""

from __future__ import annotations

import copy
import importlib.util
import json
import os
import pwd
import tempfile
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("verify_s1c3_transaction_v7.py")
SPEC = importlib.util.spec_from_file_location("s1c3_verifier", MODULE_PATH)
assert SPEC and SPEC.loader
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)

EXECUTOR_PATH = Path(__file__).with_name("s1c3_remote_transaction_v7.py")
EXECUTOR_SPEC = importlib.util.spec_from_file_location("s1c3_executor", EXECUTOR_PATH)
assert EXECUTOR_SPEC and EXECUTOR_SPEC.loader
executor = importlib.util.module_from_spec(EXECUTOR_SPEC)
EXECUTOR_SPEC.loader.exec_module(executor)


def rooted(value: dict, field: str) -> dict:
    value = copy.deepcopy(value)
    value.pop(field, None)
    value[field] = verifier.digest(verifier.canonical_bytes(value, omit_field=field))
    return value


def process_snapshot(rows: list[dict] | None = None) -> dict:
    rows = copy.deepcopy(rows or [])
    results = [
        (row["pid"], executor.classify_process_observation(row)) for row in rows
    ]
    classified = [{**row, **result} for row, (_, result) in zip(rows, results)]
    reasons = sorted({result["reason"] for _, result in results})
    summary = {
        "total_rows": len(results),
        "observable_user_process_count": sum(
            result["classification"] == verifier.OBSERVABLE_USER_PROCESS
            for _, result in results
        ),
        "proven_vanished_count": sum(
            result["reason"] == "PID_VANISHED" for _, result in results
        ),
        "proven_zombie_count": sum(
            result["reason"] == "STABLE_ZOMBIE" for _, result in results
        ),
        "proven_kernel_thread_count": sum(
            result["reason"] == "STABLE_KERNEL_THREAD" for _, result in results
        ),
        "unresolved_process_count": sum(
            result["classification"] == verifier.UNRESOLVED_PROCESS_OBSERVATION
            for _, result in results
        ),
        "reason_counts": {
            reason: sum(result["reason"] == reason for _, result in results)
            for reason in reasons
        },
    }
    unresolved = [
        executor.process_row_projection(row) for row in classified
        if row["classification"] == verifier.UNRESOLVED_PROCESS_OBSERVATION
    ]
    forbidden = [
        {**executor.process_row_projection(row), "matched_names": row["forbidden_names"]}
        for row in classified if row["forbidden_names"]
    ]
    return rooted({
        "schema": verifier.PROCESS_SNAPSHOT_SCHEMA,
        "detector_schema": verifier.PROCESS_DETECTOR_SCHEMA,
        "rows": classified,
        "summary": summary,
        "unresolved_rows": unresolved,
        "forbidden_process_matches": forbidden,
    }, "process_snapshot_root_sha256")


def stat_value(pid: int, comm: str, state: str, starttime: int) -> dict:
    parsed = {"pid": pid, "comm": comm, "state": state, "starttime": starttime}
    tail = [state, *(["0"] * 18), str(starttime)]
    raw = f"{pid} ({comm}) {' '.join(tail)}"
    return {
        "status": "VALUE",
        "parsed": parsed,
        "raw": raw,
    }


def process_observation(
    *,
    pid: int = 42,
    opening_comm: str = "worker",
    closing_comm: str | None = None,
    opening_state: str = "S",
    closing_state: str | None = None,
    opening_starttime: int = 100,
    closing_starttime: int | None = None,
    kthread: int = 0,
    cmdline: bytes = b"worker\0",
    exe: str | None = "/usr/bin/worker",
) -> dict:
    closing_comm = opening_comm if closing_comm is None else closing_comm
    closing_state = opening_state if closing_state is None else closing_state
    closing_starttime = (
        opening_starttime if closing_starttime is None else closing_starttime
    )
    return {
        "pid": pid,
        "opening_stat": stat_value(
            pid, opening_comm, opening_state, opening_starttime
        ),
        "status": {"status": "VALUE", "kthread_line": f"Kthread:\t{kthread}"},
        "cmdline": {
            "status": "VALUE",
            "byte_count": len(cmdline),
            "sha256": verifier.digest(cmdline),
        },
        "exe": (
            {"status": "VALUE", "target": exe, "basename": os.path.basename(exe)}
            if exe is not None else {"status": "ENOENT"}
        ),
        "closing_stat": stat_value(
            pid, closing_comm, closing_state, closing_starttime
        ),
    }


def service_snapshot(transition_pid: int, untouched_pid_offset: int = 0) -> dict:
    result = {}
    for index, unit in enumerate(verifier.ALL_UNITS):
        result[unit] = {
            "active_state": "active",
            "sub_state": "running",
            "main_pid": transition_pid if unit == verifier.TRANSITION_UNIT else 2000 + index + untouched_pid_offset,
            "nrestarts": 0,
            "fragment_sha256": verifier.digest(unit.encode()),
        }
    return result


def connector() -> dict:
    return {
        "schema": "nando.s1c3-connector-snapshot.v1",
        "label": "before",
        "observed_at": "2026-08-12T00:00:00Z",
        "active_state": "active",
        "main_pid": 2919,
        "nrestarts": 0,
        "route_receipt_failures": 0,
        "command_sha256": "a" * 64,
    }


def executable_identities() -> dict:
    result = {}
    for index, name in enumerate(sorted(verifier.EXECUTABLE_KEYS)):
        source = verifier.BASELINE_COMMIT if name == "parity-baseline" else verifier.CANDIDATE_COMMIT
        result[name] = {
            "path": f"/tmp/{name}",
            "sha256": f"{index + 1:x}" * 64,
            "size_bytes": 100 + index,
            "mode_octal": "0755",
            "source_identity": source,
        }
    for label in ("baseline", "candidate"):
        result[f"parity-{label}"]["path"] = verifier.expected_oracle_paths(
            "fixture", label
        )["executable"]
    return result


def ownership_receipt() -> dict:
    rows = {}
    base = "/home/e/.cache/nando-s1c3-fixture"
    probe = {
        "writer_uid": verifier.ORACLE_UID,
        "writer_gid": verifier.ORACLE_GID,
        "probe_uid": verifier.ORACLE_UID,
        "probe_gid": verifier.ORACLE_GID,
        "probe_mode_octal": "0600",
        "create_fsync_unlink_pass": True,
        "directory_fsync_pass": True,
    }
    for label in ("baseline", "candidate"):
        paths = verifier.expected_oracle_paths("fixture", label)
        workspace = paths["workspace"]
        command = [*verifier.ORACLE_CARGO_COMMAND_PREFIX, paths["manifest"]]
        rows[label] = rooted({
            "schema": verifier.OWNERSHIP_ROW_SCHEMA,
            "label": label,
            "workspace": {"path": workspace, "uid": verifier.ORACLE_UID,
                          "gid": verifier.ORACLE_GID, "mode_octal": "0750"},
            "src": {"path": f"{workspace}/src", "uid": verifier.ORACLE_UID,
                    "gid": verifier.ORACLE_GID, "mode_octal": "0750"},
            "cargo_toml": {"path": f"{workspace}/Cargo.toml", "uid": verifier.ORACLE_UID,
                           "gid": verifier.ORACLE_GID, "mode_octal": "0640"},
            "main_rs": {"path": f"{workspace}/src/main.rs", "uid": verifier.ORACLE_UID,
                        "gid": verifier.ORACLE_GID, "mode_octal": "0640"},
            "main_rs_sha256": verifier.ORACLE_SOURCE_SHA256,
            "cargo_lock": {
                "path": paths["lock"], "uid": verifier.ORACLE_UID,
                "gid": verifier.ORACLE_GID, "mode_octal": "0640",
                "sha256_before_build": verifier.ORACLE_LOCK_SHA256,
                "sha256_after_build": verifier.ORACLE_LOCK_SHA256,
            },
            "manifest_sha256": verifier.digest(verifier.oracle_manifest(paths["source"])),
            "build_command": command,
            "build_environment": {
                **verifier.ORACLE_CARGO_ENVIRONMENT,
                "CARGO_TARGET_DIR": paths["target"],
            },
            "target_directory": paths["target"],
            "executable_path": paths["executable"],
            "probe": copy.deepcopy(probe),
            "probe_retained": False,
        }, "ownership_row_root_sha256")
    return rooted({
        "schema": verifier.OWNERSHIP_SCHEMA,
        "transaction_id": "fixture",
        "build_user": {"name": "e", "uid": verifier.ORACLE_UID,
                       "gid": verifier.ORACLE_GID},
        "rows": rows,
        "rows_root_sha256": verifier.digest(verifier.canonical_bytes(rows)),
        "oracle_build_contract": {
            "schema": verifier.ORACLE_BUILD_CONTRACT_SCHEMA,
            "package": {
                "name": verifier.ORACLE_PACKAGE_NAME,
                "version": verifier.ORACLE_PACKAGE_VERSION,
                "edition": verifier.ORACLE_EDITION,
            },
            "oracle_source_sha256": verifier.ORACLE_SOURCE_SHA256,
            "cargo_lock_sha256": verifier.ORACLE_LOCK_SHA256,
            "offline": True,
            "locked": True,
            "cargo_net_offline": "true",
            "manifest_sha256": {
                label: rows[label]["manifest_sha256"]
                for label in ("baseline", "candidate")
            },
        },
    }, "ownership_root_sha256")


def quiescence_receipt(ownership: dict) -> dict:
    executables = executable_identities()
    samples = []
    process_snapshots = [process_snapshot() for _ in range(31)]
    start = 1_000_000_000
    for index in range(30):
        end = start + 1_000_000_000
        start_counters = {
            str(cpu): {"total": index * 100, "idle": index * 99}
            for cpu in verifier.LOGICAL_CPU_POOL
        }
        end_counters = {
            str(cpu): {"total": (index + 1) * 100, "idle": (index + 1) * 99}
            for cpu in verifier.LOGICAL_CPU_POOL
        }
        samples.append({
            "started_at": f"2026-08-12T00:00:{index:02d}Z",
            "ended_at": f"2026-08-12T00:00:{index + 1:02d}Z",
            "start_monotonic_ns": start,
            "end_monotonic_ns": end,
            "interval_seconds": 1.0,
            "start_cpu_counters": start_counters,
            "end_cpu_counters": end_counters,
            "start_cpu_snapshot_root_sha256": verifier.digest(
                verifier.canonical_bytes(start_counters)
            ),
            "end_cpu_snapshot_root_sha256": verifier.digest(
                verifier.canonical_bytes(end_counters)
            ),
            "cpu_busy_percent": {
                str(cpu): 1.0 for cpu in verifier.LOGICAL_CPU_POOL
            },
            "io_some_avg10": 0.0,
            "io_full_avg10": 0.0,
            "build_processes_start": [],
            "build_processes_end": [],
            "start_process_snapshot_root_sha256": process_snapshots[index][
                "process_snapshot_root_sha256"
            ],
            "end_process_snapshot_root_sha256": process_snapshots[index + 1][
                "process_snapshot_root_sha256"
            ],
            "loadavg_end": "1.0 1.0 1.0 1/1 1",
            "blockers": [],
            "eligible_physical_cores": list(verifier.MEASUREMENT_CPU_POOL),
            "window_mean_blockers": [],
        })
        start = end
    topology = [
        {
            "representative_cpu": representative,
            "siblings": verifier.PHYSICAL_CORE_SIBLINGS[representative],
            "core_id": verifier.EXPECTED_CORE_IDS[representative],
            "max_frequency_khz": verifier.EXPECTED_MAX_FREQUENCY_KHZ,
            "logical_cpus": [
                {
                    "cpu": cpu,
                    "online": True,
                    "core_id": verifier.EXPECTED_CORE_IDS[representative],
                    "max_frequency_khz": verifier.EXPECTED_MAX_FREQUENCY_KHZ,
                }
                for cpu in verifier.PHYSICAL_CORE_SIBLINGS[representative]
            ],
        }
        for representative in verifier.MEASUREMENT_CPU_POOL
    ]
    executable_root = verifier.digest(verifier.canonical_bytes(executables))
    census = {
        "forbidden_build_process": 0,
        "unresolved_process_observation": 0,
        "interval_duration": 0,
        **{f"cpu_busy_per_interval:{cpu}": 0 for cpu in verifier.LOGICAL_CPU_POOL},
        **{f"cpu_busy_window_mean:{cpu}": 0 for cpu in verifier.LOGICAL_CPU_POOL},
        "io_some": 0,
        "io_full": 0,
    }
    return rooted({
        "schema": verifier.QUIESCENCE_SCHEMA,
        "transaction_id": "fixture",
        "candidate_commit": verifier.CANDIDATE_COMMIT,
        "candidate_tree": verifier.CANDIDATE_TREE,
        "verdict": "PASS",
        "measurement_cpu_pool": list(verifier.MEASUREMENT_CPU_POOL),
        "selected_cpu": 4,
        "topology": topology,
        "topology_root_sha256": verifier.digest(verifier.canonical_bytes(topology)),
        "detector_schema": verifier.PROCESS_DETECTOR_SCHEMA,
        "snapshot_schema": "adjacent-cpu-and-process-snapshots-v1",
        "forbidden_build_names": list(verifier.FORBIDDEN_BUILD_NAMES),
        "maximum_wait_seconds": 1800,
        "required_intervals": 30,
        "thresholds": {
            "interval_min_seconds": 0.90,
            "interval_max_seconds": 1.50,
            "cpu_per_interval_max_percent": 20.0,
            "cpu_window_mean_max_percent": 5.0,
            "io_some_avg10_max": 0.20,
            "io_full_avg10_max": 0.05,
        },
        "eligibility_started_at": "2026-08-12T00:00:00Z",
        "eligibility_reached_at": "2026-08-12T00:00:30Z",
        "eligibility_started_monotonic_ns": 1_000_000_000,
        "eligibility_finished_monotonic_ns": 31_000_000_000,
        "attempted_interval_count": len(samples),
        "process_snapshot_count": len(process_snapshots),
        "process_snapshots": process_snapshots,
        "process_snapshots_root_sha256": verifier.digest(
            verifier.canonical_bytes(process_snapshots)
        ),
        "attempted_samples": copy.deepcopy(samples),
        "attempted_samples_root_sha256": verifier.digest(
            verifier.canonical_bytes(samples)
        ),
        "eligible_window": copy.deepcopy(samples),
        "eligible_window_cpu_mean_percent": {"4": 1.0, "5": 1.0},
        "eligible_window_root_sha256": verifier.digest(verifier.canonical_bytes(samples)),
        "longest_eligible_streaks": {"4": 30, "6": 30},
        "minimum_completed_window_mean_percent": {
            "4": {"4": 1.0, "5": 1.0},
            "6": {"6": 1.0, "7": 1.0},
        },
        "blocker_census": census,
        "executables": executables,
        "executable_set_root_sha256": executable_root,
        "oracle_ownership_root_sha256": ownership["ownership_root_sha256"],
    }, "quiescence_root_sha256")


def timeout_quiescence_receipt(ownership: dict) -> dict:
    receipt = quiescence_receipt(ownership)
    samples = []
    process_snapshots = [process_snapshot() for _ in range(1801)]
    for index in range(1800):
        start = 1_000_000_000 + index * 1_000_000_000
        end = start + 1_000_000_000
        start_counters = {
            str(cpu): {"total": index * 100, "idle": index * 70}
            for cpu in verifier.LOGICAL_CPU_POOL
        }
        end_counters = {
            str(cpu): {"total": (index + 1) * 100, "idle": (index + 1) * 70}
            for cpu in verifier.LOGICAL_CPU_POOL
        }
        blockers = [
            f"cpu_busy_per_interval:{cpu}" for cpu in verifier.LOGICAL_CPU_POOL
        ]
        samples.append({
            "started_at": "2026-08-12T00:00:00Z",
            "ended_at": "2026-08-12T00:00:01Z",
            "start_monotonic_ns": start,
            "end_monotonic_ns": end,
            "interval_seconds": 1.0,
            "start_cpu_counters": start_counters,
            "end_cpu_counters": end_counters,
            "start_cpu_snapshot_root_sha256": verifier.digest(
                verifier.canonical_bytes(start_counters)
            ),
            "end_cpu_snapshot_root_sha256": verifier.digest(
                verifier.canonical_bytes(end_counters)
            ),
            "cpu_busy_percent": {
                str(cpu): 30.0 for cpu in verifier.LOGICAL_CPU_POOL
            },
            "io_some_avg10": 0.0,
            "io_full_avg10": 0.0,
            "build_processes_start": [],
            "build_processes_end": [],
            "start_process_snapshot_root_sha256": process_snapshots[index][
                "process_snapshot_root_sha256"
            ],
            "end_process_snapshot_root_sha256": process_snapshots[index + 1][
                "process_snapshot_root_sha256"
            ],
            "loadavg_end": "1.0 1.0 1.0 1/1 1",
            "blockers": blockers,
            "eligible_physical_cores": [],
            "window_mean_blockers": [],
        })
    receipt.update({
        "verdict": "TIMEOUT",
        "selected_cpu": None,
        "eligibility_reached_at": None,
        "eligibility_started_monotonic_ns": 1_000_000_000,
        "eligibility_finished_monotonic_ns": 1_801_000_000_000,
        "attempted_interval_count": len(samples),
        "process_snapshot_count": len(process_snapshots),
        "process_snapshots": process_snapshots,
        "process_snapshots_root_sha256": verifier.digest(
            verifier.canonical_bytes(process_snapshots)
        ),
        "attempted_samples": samples,
        "attempted_samples_root_sha256": verifier.digest(
            verifier.canonical_bytes(samples)
        ),
        "eligible_window": None,
        "eligible_window_cpu_mean_percent": None,
        "eligible_window_root_sha256": None,
        "longest_eligible_streaks": {"4": 0, "6": 0},
        "minimum_completed_window_mean_percent": {
            "4": {"4": None, "5": None},
            "6": {"6": None, "7": None},
        },
        "blocker_census": {
            "forbidden_build_process": 0,
            "unresolved_process_observation": 0,
            "interval_duration": 0,
            **{
                f"cpu_busy_per_interval:{cpu}": len(samples)
                for cpu in verifier.LOGICAL_CPU_POOL
            },
            **{
                f"cpu_busy_window_mean:{cpu}": 0
                for cpu in verifier.LOGICAL_CPU_POOL
            },
            "io_some": 0,
            "io_full": 0,
        },
    })
    return rooted(receipt, "quiescence_root_sha256")


def contamination_receipt(quiescence: dict) -> dict:
    samples = []
    boundaries = []
    now = 10_000_000_000
    for label in verifier.MEASUREMENT_LABELS:
        for phase in ("before", "after"):
            boundaries.append({"label": label, "phase": phase, "observed_at": "2026-08-12T00:01:00Z"})
            processes = process_snapshot()
            samples.append({
                "label": label,
                "observed_at": "2026-08-12T00:01:00Z",
                "monotonic_ns": now,
                "cpu_counters": {
                    str(cpu): {"total": 100, "idle": 99}
                    for cpu in verifier.LOGICAL_CPU_POOL
                },
                "cpu_snapshot_root_sha256": "f" * 64,
                "io_pressure": {"some": {"avg10": 0.0, "total": 0}, "full": {"avg10": 0.0, "total": 0}},
                "loadavg": "1.0 1.0 1.0 1/1 1",
                "process_snapshot": processes,
                "process_snapshot_root_sha256": processes[
                    "process_snapshot_root_sha256"
                ],
                "build_processes": [],
                "unresolved_processes": [],
                "kind": f"boundary-{phase}",
            })
            now += 100_000_000
    executable_root = verifier.digest(verifier.canonical_bytes(quiescence["executables"]))
    return rooted({
        "schema": verifier.CONTAMINATION_SCHEMA,
        "transaction_id": "fixture",
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_cpu": quiescence["selected_cpu"],
        "executable_set_root_sha256": executable_root,
        "monitor_interval_seconds": 0.5,
        "maximum_sample_gap_seconds": 2.0,
        "observed_max_sample_gap_seconds": 0.1,
        "metric_labels": list(verifier.MEASUREMENT_LABELS),
        "boundaries": boundaries,
        "samples": samples,
        "forbidden_process_matches": [],
        "unresolved_process_observations": [],
        "monitor_errors": [],
        "contaminated": False,
    }, "measurement_contamination_root_sha256")


def resource_receipt(quiescence: dict, contamination: dict) -> dict:
    hot = {"p99_ns": 10_000, "no_goal_p99_ns": 1_000, "hard_max_ns": 20_000, "samples": 4096}
    single = {"p99_ns": 1_000_000, "hard_max_ns": 2_000_000, "samples": 1024, "segments": 2}
    three = {
        "precommit_p99_ns": 1_000_000,
        "precommit_hard_max_ns": 2_000_000,
        "settlement_p99_ns": 2_000_000,
        "settlement_hard_max_ns": 3_000_000,
        "episode_p99_ns": 3_000_000,
        "episode_hard_max_ns": 4_000_000,
        "samples": 256,
    }
    return rooted({
        "schema": verifier.RESOURCE_SCHEMA,
        "candidate_commit": verifier.CANDIDATE_COMMIT,
        "observed_at": "2026-08-12T00:00:00Z",
        "metrics": {
            "hot_latency": [copy.deepcopy(hot) for _ in range(3)],
            "single_ledger_sync": [copy.deepcopy(single) for _ in range(3)],
            "three_ledger_sync": [copy.deepcopy(three) for _ in range(3)],
            "idle_cpu": {"elapsed_ticks": 0, "ticks_per_second": 100, "percent_of_one_core": 0.0},
            "rss": {"capture_off_bytes": 10, "capture_on_bytes": 20, "delta_bytes": 10},
        },
        "frozen_bounds": {
            "max_precommit_bytes": 32 * 1024,
            "max_typed_goal_bytes": 4 * 1024,
            "max_k1_actions": 256,
            "segment_bytes": 64 * 1024 * 1024,
            "journal_quota_bytes": 2 * 1024 * 1024 * 1024,
            "persisted_raw_payload_bytes": 0,
        },
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_cpu": quiescence["selected_cpu"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "direct_exec_only": True,
        "compiler_invocations_after_quiescence": 0,
        "all_pass": True,
    }, "resource_root_sha256")


def parity_receipt(quiescence: dict) -> dict:
    return rooted({
        "schema": verifier.PARITY_SCHEMA,
        "baseline_output_sha256": "b" * 64,
        "candidate_output_sha256": "b" * 64,
        "byte_identical": True,
        "rows": 16,
        "direct_exec_only": True,
        "baseline_oracle_sha256": quiescence["executables"]["parity-baseline"]["sha256"],
        "candidate_oracle_sha256": quiescence["executables"]["parity-candidate"]["sha256"],
    }, "parity_root_sha256")


def preparation(quiescence: dict, contamination: dict, resource: dict, parity: dict) -> dict:
    rollback_entries = [
        {"path": "nando-transition-serving", "sha256": verifier.BASELINE_BINARY_SHA256,
         "size_bytes": 90},
        {"path": "transition-serving.env", "sha256": verifier.BASELINE_CONFIG_SHA256,
         "size_bytes": 10},
        {"path": "nando-transition-serving.service", "sha256": verifier.UNIT_SHA256,
         "size_bytes": 10},
        {"path": "previous-deployment-receipt.json", "sha256": "f" * 64,
         "size_bytes": 10},
    ]
    rollback_manifest = "".join(
        f'{entry["sha256"]} {entry["size_bytes"]} {entry["path"]}\n'
        for entry in sorted(rollback_entries, key=lambda item: item["path"])
    ).encode("ascii")
    return rooted({
        "schema": verifier.PREPARATION_SCHEMA,
        "transaction_id": "fixture",
        "state": "PREPARED",
        "created_at": "2026-08-12T00:00:00Z",
        "paper": {"commit": verifier.PAPER_COMMIT,
                  "manifest_root_sha256": verifier.PAPER_MANIFEST_ROOT,
                  "verification_sha256": verifier.PAPER_VERIFICATION_SHA256},
        "candidate": {"source_commit": verifier.CANDIDATE_COMMIT,
                      "source_tree": verifier.CANDIDATE_TREE,
                      "cargo_lock_sha256": verifier.CARGO_LOCK_SHA256,
                      "binary_sha256": quiescence["executables"]["candidate-binary"]["sha256"],
                      "binary_size_bytes": quiescence["executables"]["candidate-binary"]["size_bytes"],
                      "config_sha256": verifier.CANDIDATE_CONFIG_SHA256,
                      "production_projection_path": verifier.PRODUCTION_PROJECTION_PATH,
                      "production_projection_schema": verifier.PRODUCTION_PROJECTION_SCHEMA,
                      "production_projection_sha256": verifier.PRODUCTION_PROJECTION_SHA256},
        "baseline": {"source_commit": verifier.BASELINE_COMMIT,
                     "source_tree": verifier.BASELINE_TREE,
                     "deployment_receipt_root_sha256": verifier.BASELINE_RECEIPT_ROOT,
                     "binary_sha256": verifier.BASELINE_BINARY_SHA256,
                     "binary_size_bytes": 90,
                     "config_sha256": verifier.BASELINE_CONFIG_SHA256},
        "toolchain": {},
        "immutable": {"unit_sha256": verifier.UNIT_SHA256,
                      "phase_config_sha256": verifier.PHASE_CONFIG_SHA256,
                      "authority_config_sha256": verifier.AUTHORITY_CONFIG_SHA256},
        "services_before": service_snapshot(1000),
        "health_before": {},
        "economics_before": {"false_accepts": 0, "runtime_parity_mismatches": 0},
        "route_probe_before": {},
        "connector_before": connector(),
        "journal_before": {},
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_cpu": quiescence["selected_cpu"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
        "rollback": {
            "manifest_root_sha256": verifier.digest(rollback_manifest),
            "entries": rollback_entries,
        },
        "intent": [],
    }, "preparation_root_sha256")


def predeployment_receipt(
    prep: dict,
    quiescence: dict,
    contamination: dict,
    resource: dict,
    parity: dict,
) -> dict:
    return rooted({
        "schema": "nando.s1c3-predeployment-verification.v7",
        "valid": True,
        "authority": True,
        "verdict": "S1C3_PREPARATION_PASS",
        "selected_cpu": quiescence["selected_cpu"],
        "preparation_root_sha256": prep["preparation_root_sha256"],
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
    }, "predeployment_verification_root_sha256")


def journal() -> dict:
    return {"present": True, "entries": [], "total_bytes": 0,
            "manifest_root_sha256": verifier.digest(b""), "raw_payload_bytes": 0,
            "preserved_prefixes": True}


def deployment_receipt(
    prep: dict,
    quiescence: dict,
    contamination: dict,
    resource: dict,
    parity: dict,
    predeployment_root: str,
) -> dict:
    before = service_snapshot(1000)
    after = service_snapshot(1001)
    survival = copy.deepcopy(after)
    before_connector = connector()
    after_connector = copy.deepcopy(before_connector)
    after_connector["label"] = "after"
    return rooted({
        "schema": verifier.SCHEMA,
        "transaction_id": "fixture",
        "verdict": "S1C3_DEPLOYMENT_PASS",
        "finalized_at": "2026-08-12T00:00:16Z",
        "preparation_root_sha256": prep["preparation_root_sha256"],
        "oracle_ownership_root_sha256": quiescence["oracle_ownership_root_sha256"],
        "quiescence_root_sha256": quiescence["quiescence_root_sha256"],
        "measurement_cpu": quiescence["selected_cpu"],
        "measurement_contamination_root_sha256": contamination[
            "measurement_contamination_root_sha256"
        ],
        "executable_set_root_sha256": verifier.digest(
            verifier.canonical_bytes(quiescence["executables"])
        ),
        "resource_root_sha256": resource["resource_root_sha256"],
        "parity_root_sha256": parity["parity_root_sha256"],
        "predeployment_verification_root_sha256": predeployment_root,
        "services_before": before,
        "services_after": after,
        "services_survival": survival,
        "connector_before": before_connector,
        "connector_after": after_connector,
        "installed_binary_sha256": prep["candidate"]["binary_sha256"],
        "installed_config_sha256": verifier.CANDIDATE_CONFIG_SHA256,
        "immutable_after": {"unit_sha256": verifier.UNIT_SHA256,
                            "phase_config_sha256": verifier.PHASE_CONFIG_SHA256,
                            "authority_config_sha256": verifier.AUTHORITY_CONFIG_SHA256},
        "capture_environment": {
            "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
            "NANDO_GROUNDED_DECISION_JOURNAL": "/var/lib/nando-wave/transition/grounded-meaning-v1/decision-contract-precommits-v1",
        },
        "capture_available": True,
        "startup_log_clean": True,
        "health_semantics_preserved": True,
        "route_probe_equivalent": True,
        "active_packages_preserved": True,
        "false_accepts_after": 0,
        "runtime_parity_failures_after": 0,
        "journal_before": journal(),
        "journal_after": journal(),
        "survival_seconds": 15,
    }, "receipt_root_sha256")


class TransactionVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.ownership = ownership_receipt()
        self.quiescence = quiescence_receipt(self.ownership)
        self.contamination = contamination_receipt(self.quiescence)
        self.resource = resource_receipt(self.quiescence, self.contamination)
        self.parity = parity_receipt(self.quiescence)
        self.prep = preparation(
            self.quiescence, self.contamination, self.resource, self.parity
        )
        self.predeployment = predeployment_receipt(
            self.prep,
            self.quiescence,
            self.contamination,
            self.resource,
            self.parity,
        )
        self.receipt = deployment_receipt(
            self.prep,
            self.quiescence,
            self.contamination,
            self.resource,
            self.parity,
            self.predeployment["predeployment_verification_root_sha256"],
        )
        self.write_all()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, name: str, value: dict) -> None:
        path = self.directory / name
        if path.exists():
            path.chmod(0o600)
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

    def write_all(self) -> None:
        self.write("oracle-ownership-receipt.json", self.ownership)
        (self.directory / "oracle-ownership-receipt.json").chmod(0o400)
        self.write("quiescence-receipt.json", self.quiescence)
        (self.directory / "quiescence-receipt.json").chmod(0o400)
        self.write("measurement-contamination-receipt.json", self.contamination)
        self.write("resource-receipt.json", self.resource)
        self.write("parity-receipt.json", self.parity)
        self.write("preparation.json", self.prep)
        self.write("predeployment-verification.json", self.predeployment)
        self.write("deployment-receipt.json", self.receipt)

    def rebind_environment(self) -> None:
        executable_root = verifier.digest(
            verifier.canonical_bytes(self.quiescence["executables"])
        )
        self.contamination["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.contamination["measurement_cpu"] = self.quiescence["selected_cpu"]
        self.contamination["executable_set_root_sha256"] = executable_root
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.resource["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.resource["measurement_cpu"] = self.quiescence["selected_cpu"]
        self.resource["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.resource["executable_set_root_sha256"] = executable_root
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.parity["baseline_oracle_sha256"] = self.quiescence["executables"][
            "parity-baseline"
        ]["sha256"]
        self.parity["candidate_oracle_sha256"] = self.quiescence["executables"][
            "parity-candidate"
        ]["sha256"]
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.prep["measurement_cpu"] = self.quiescence["selected_cpu"]
        self.prep["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.prep["executable_set_root_sha256"] = executable_root
        self.prep["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.predeployment = predeployment_receipt(
            self.prep,
            self.quiescence,
            self.contamination,
            self.resource,
            self.parity,
        )
        self.receipt["quiescence_root_sha256"] = self.quiescence[
            "quiescence_root_sha256"
        ]
        self.receipt["measurement_cpu"] = self.quiescence["selected_cpu"]
        self.receipt["measurement_contamination_root_sha256"] = self.contamination[
            "measurement_contamination_root_sha256"
        ]
        self.receipt["executable_set_root_sha256"] = executable_root
        self.receipt["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt["predeployment_verification_root_sha256"] = self.predeployment[
            "predeployment_verification_root_sha256"
        ]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()

    def assert_invalid(self, expected: str) -> None:
        with self.assertRaisesRegex(verifier.InvalidReceipt, expected):
            verifier.verify_receipt(self.directory)

    def rehash_ownership(self, label: str) -> None:
        self.ownership["rows"][label] = rooted(
            self.ownership["rows"][label], "ownership_row_root_sha256"
        )
        self.ownership["rows_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.ownership["rows"])
        )
        self.ownership = rooted(self.ownership, "ownership_root_sha256")
        self.write_all()

    def rehash_quiescence(self) -> None:
        attempted = self.quiescence["attempted_samples"]
        self.quiescence["attempted_interval_count"] = len(attempted)
        self.quiescence["attempted_samples_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(attempted)
        )
        window = self.quiescence["eligible_window"]
        self.quiescence["eligible_window_root_sha256"] = (
            verifier.digest(verifier.canonical_bytes(window))
            if window is not None else None
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()

    def install_process_snapshot(self, index: int, rows: list[dict]) -> None:
        snapshot = process_snapshot(rows)
        self.quiescence["process_snapshots"][index] = snapshot
        self.quiescence["process_snapshots_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["process_snapshots"])
        )
        if index > 0:
            self.quiescence["attempted_samples"][index - 1][
                "end_process_snapshot_root_sha256"
            ] = snapshot["process_snapshot_root_sha256"]
        if index < len(self.quiescence["attempted_samples"]):
            self.quiescence["attempted_samples"][index][
                "start_process_snapshot_root_sha256"
            ] = snapshot["process_snapshot_root_sha256"]
        self.rehash_quiescence()

    def write_timeout_fixture(self) -> None:
        self.quiescence = timeout_quiescence_receipt(self.ownership)
        for name in (
            "measurement-contamination-receipt.json", "resource-receipt.json",
            "parity-receipt.json", "preparation.json", "predeployment-verification.json",
            "deployment-receipt.json",
        ):
            path = self.directory / name
            if path.exists():
                path.unlink()
        self.write("quiescence-receipt.json", self.quiescence)
        (self.directory / "quiescence-receipt.json").chmod(0o400)
        self.write("transaction-state.json", {
            "schema": "nando.s1c3-state.v7",
            "state": "QUIESCENCE_TIMEOUT",
            "transaction_id": self.quiescence["transaction_id"],
            "quiescence_root_sha256": self.quiescence["quiescence_root_sha256"],
        })

    def test_valid_deployment_pass(self) -> None:
        result = verifier.verify_receipt(self.directory)
        self.assertEqual(result["verdict"], "S1C3_DEPLOYMENT_PASS")

    def test_valid_predeployment_authority(self) -> None:
        (self.directory / "deployment-receipt.json").unlink()
        result = verifier.verify_preparation(self.directory)
        self.assertEqual(result["verdict"], "S1C3_PREPARATION_PASS")
        self.assertTrue(result["authority"])

    def test_proc_stat_parser_handles_parentheses_and_field_22(self) -> None:
        row = stat_value(42, "worker) helper", "S", 987654)
        self.assertEqual(executor.parse_proc_stat(row["raw"]), row["parsed"])
        self.assertEqual(verifier.independently_parse_proc_stat(row["raw"]), row["parsed"])

    def test_stable_kernel_thread_is_non_executing(self) -> None:
        snapshot = process_snapshot([
            process_observation(kthread=1, cmdline=b"", exe=None)
        ])
        self.assertEqual(snapshot["summary"]["proven_kernel_thread_count"], 1)
        self.assertEqual(snapshot["summary"]["unresolved_process_count"], 0)
        verifier.verify_process_snapshot(snapshot, "kernel_thread")

    def test_vanished_pid_is_non_executing(self) -> None:
        row = process_observation()
        row["status"] = {"status": "PERMISSION_DENIED"}
        row["cmdline"] = {"status": "PERMISSION_DENIED"}
        row["exe"] = {"status": "PERMISSION_DENIED"}
        row["closing_stat"] = {"status": "ENOENT"}
        snapshot = process_snapshot([row])
        self.assertEqual(snapshot["summary"]["proven_vanished_count"], 1)
        self.assertEqual(snapshot["summary"]["unresolved_process_count"], 0)
        verifier.verify_process_snapshot(snapshot, "vanished")

    def test_pid_vanished_before_opening_stat_is_non_executing(self) -> None:
        row = process_observation()
        row["opening_stat"] = {"status": "ENOENT"}
        row["status"] = {"status": "ENOENT"}
        row["cmdline"] = {"status": "ENOENT"}
        row["exe"] = {"status": "ENOENT"}
        row["closing_stat"] = {"status": "ENOENT"}
        snapshot = process_snapshot([row])
        self.assertEqual(snapshot["summary"]["proven_vanished_count"], 1)
        self.assertEqual(snapshot["summary"]["unresolved_process_count"], 0)
        verifier.verify_process_snapshot(snapshot, "vanished_before_opening")

    def test_opening_missing_but_closing_present_is_unresolved(self) -> None:
        row = process_observation()
        row["opening_stat"] = {"status": "ENOENT"}
        self.install_process_snapshot(0, [row])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_stable_zombie_is_non_executing(self) -> None:
        row = process_observation(opening_state="Z", closing_state="Z")
        row["status"] = {"status": "PERMISSION_DENIED"}
        row["cmdline"] = {"status": "PERMISSION_DENIED"}
        row["exe"] = {"status": "PERMISSION_DENIED"}
        snapshot = process_snapshot([row])
        self.assertEqual(snapshot["summary"]["proven_zombie_count"], 1)
        self.assertEqual(snapshot["summary"]["unresolved_process_count"], 0)
        verifier.verify_process_snapshot(snapshot, "zombie")

    def test_missing_exe_alone_does_not_prove_kernel_thread(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(kthread=0, cmdline=b"worker\0", exe=None)
        ])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_nonempty_kernel_thread_cmdline_is_unresolved(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(kthread=1, cmdline=b"worker\0", exe=None)
        ])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_kthread_zero_with_empty_cmdline_is_unresolved(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(kthread=0, cmdline=b"", exe=None)
        ])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_unstable_zombie_is_unresolved(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(
                opening_state="Z", closing_state="S", cmdline=b"", exe=None
            )
        ])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_pid_reuse_is_unresolved(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(closing_starttime=101)
        ])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_permission_denial_is_unresolved(self) -> None:
        row = process_observation()
        row["closing_stat"] = {"status": "PERMISSION_DENIED"}
        self.install_process_snapshot(0, [row])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_malformed_stat_is_unresolved(self) -> None:
        row = process_observation()
        row["opening_stat"] = {
            "status": "MALFORMED", "error": "stat_tail_invalid", "raw": "42 bad"
        }
        self.install_process_snapshot(0, [row])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_malformed_status_is_unresolved(self) -> None:
        row = process_observation(exe=None, cmdline=b"")
        row["status"] = {"status": "MALFORMED", "error": "kthread_field_invalid"}
        self.install_process_snapshot(0, [row])
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_forbidden_comm_is_recomputed(self) -> None:
        self.install_process_snapshot(0, [
            process_observation(opening_comm="cargo", closing_comm="cargo")
        ])
        self.assert_invalid("quiescence_sample_0_build_start_mismatch")

    def test_forbidden_executable_is_recomputed(self) -> None:
        self.install_process_snapshot(0, [process_observation(exe="/usr/bin/rustc")])
        self.assert_invalid("quiescence_sample_0_build_start_mismatch")

    def test_forged_process_summary_is_rejected(self) -> None:
        snapshot = self.quiescence["process_snapshots"][0]
        snapshot["summary"]["unresolved_process_count"] = 1
        snapshot = rooted(snapshot, "process_snapshot_root_sha256")
        self.quiescence["process_snapshots"][0] = snapshot
        self.quiescence["process_snapshots_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["process_snapshots"])
        )
        self.quiescence["attempted_samples"][0][
            "start_process_snapshot_root_sha256"
        ] = snapshot["process_snapshot_root_sha256"]
        self.rehash_quiescence()
        self.assert_invalid("quiescence_process_snapshot_0_summary_mismatch")

    def test_v6_process_schema_is_rejected(self) -> None:
        self.quiescence["detector_schema"] = "proc-comm-exe-basename-v1"
        self.quiescence["snapshot_schema"] = "single-proc-stat-snapshot-four-cpus-v1"
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_detector_mismatch")

    def test_oracle_workspace_owner_mismatch_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["workspace"]["uid"] = 0
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_baseline_workspace_uid_mismatch")

    def test_oracle_workspace_unwritable_mode_is_rejected(self) -> None:
        self.ownership["rows"]["candidate"]["workspace"]["mode_octal"] = "0550"
        self.rehash_ownership("candidate")
        self.assert_invalid("ownership_candidate_workspace_mode_mismatch")

    def test_oracle_probe_retained_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["probe_retained"] = True
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_row_baseline_probe_retained_mismatch")

    def test_missing_offline_flag_is_rejected(self) -> None:
        command = self.ownership["rows"]["baseline"]["build_command"]
        command.remove("--offline")
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_baseline_build_command_mismatch")

    def test_missing_locked_flag_is_rejected(self) -> None:
        command = self.ownership["rows"]["candidate"]["build_command"]
        command.remove("--locked")
        self.rehash_ownership("candidate")
        self.assert_invalid("ownership_candidate_build_command_mismatch")

    def test_cargo_net_offline_drift_is_rejected(self) -> None:
        self.ownership["rows"]["candidate"]["build_environment"][
            "CARGO_NET_OFFLINE"
        ] = "false"
        self.rehash_ownership("candidate")
        self.assert_invalid("ownership_candidate_build_environment_mismatch")

    def test_lock_hash_drift_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["cargo_lock"][
            "sha256_after_build"
        ] = "f" * 64
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_baseline_cargo_lock_after_mismatch")

    def test_package_name_divergence_is_rejected(self) -> None:
        self.ownership["oracle_build_contract"]["package"]["name"] = (
            "s1c3-parity-baseline"
        )
        self.ownership = rooted(self.ownership, "ownership_root_sha256")
        self.write_all()
        self.assert_invalid("ownership_build_contract_mismatch")

    def test_manifest_substitution_is_rejected(self) -> None:
        self.ownership["rows"]["baseline"]["manifest_sha256"] = "e" * 64
        self.rehash_ownership("baseline")
        self.assert_invalid("ownership_baseline_manifest_sha256_mismatch")

    def test_oracle_source_substitution_is_rejected(self) -> None:
        self.ownership["rows"]["candidate"]["main_rs_sha256"] = "d" * 64
        self.rehash_ownership("candidate")
        self.assert_invalid("ownership_candidate_main_rs_sha256_mismatch")

    def test_reused_v5_executable_path_is_rejected(self) -> None:
        self.quiescence["executables"]["parity-baseline"]["path"] = (
            "/home/e/.cache/nando-s1c3-v5/oracle-target-baseline/release/"
            "s1c3-parity-baseline"
        )
        self.rehash_quiescence()
        self.assert_invalid("executable_parity-baseline_fresh_path_mismatch")

    def test_reused_diagnostic_executable_path_is_rejected(self) -> None:
        self.quiescence["executables"]["parity-candidate"]["path"] = (
            "/tmp/s1c3-v7-offline-diagnostic/candidate/release/"
            "s1c3-parity-oracle"
        )
        self.rehash_quiescence()
        self.assert_invalid("executable_parity-candidate_fresh_path_mismatch")

    def test_offline_closure_failure_is_terminal_before_quiescence(self) -> None:
        offline_error = (
            b"error: failed to download `serde_json`; attempting to make an HTTP "
            b"request, but --offline was specified"
        )
        self.assertTrue(executor.offline_dependency_closure_missing(offline_error))
        oracle_root = self.directory / "offline-fault"
        oracle_root.mkdir()
        manifest = oracle_root / "Cargo.toml"
        manifest.write_text("[package]\n", encoding="utf-8")
        copied_lock = oracle_root / "Cargo.lock"
        frozen_lock = (
            Path(__file__).parents[2]
            / "plans/effect-law-unification-v1/evidence/"
            "S1C_TRANSACTIONAL_DEPLOYMENT_PREREGISTRATION_V7/oracle.Cargo.lock"
        )
        copied_lock.write_bytes(frozen_lock.read_bytes())
        completed = mock.Mock(returncode=101, stdout=offline_error)
        with (
            mock.patch.object(
                executor,
                "make_oracle",
                return_value=(manifest, copied_lock, {"label": "candidate"}),
            ),
            mock.patch.object(executor, "run", return_value=completed) as cargo_run,
        ):
            with self.assertRaisesRegex(
                executor.GateFailure,
                "OFFLINE_DEPENDENCY_CLOSURE_MISSING:candidate",
            ):
                executor.prebuild_oracle(
                    self.directory / "main.rs",
                    frozen_lock,
                    self.directory / "candidate",
                    oracle_root,
                    "candidate",
                    self.directory,
                )
        cargo_run.assert_called_once()
        executor_source = Path(__file__).with_name(
            "s1c3_remote_transaction_v7.py"
        ).read_text(encoding="utf-8")
        prepare = executor_source.split("def prepare(", 1)[1].split(
            "def exact_untouched", 1
        )[0]
        oracle_builds = prepare.index("baseline_oracle, baseline_ownership")
        quiescence = prepare.index("quiescence = wait_for_quiescence")
        self.assertLess(oracle_builds, quiescence)
        self.assertNotIn("retry", executor_source.split("def prebuild_oracle(", 1)[1].split(
            "def build_ownership_receipt", 1
        )[0].lower())
        error = executor.GateFailure(
            "OFFLINE_DEPENDENCY_CLOSURE_MISSING:candidate"
        )
        self.assertEqual(
            executor.preflight_terminal_state(error),
            "OFFLINE_DEPENDENCY_CLOSURE_MISSING",
        )
        self.assertEqual(
            executor.preflight_terminal_state(RuntimeError("other")),
            "PREFLIGHT_FAILURE",
        )

    def test_oracle_ownership_root_mismatch_is_rejected(self) -> None:
        self.ownership["ownership_root_sha256"] = "0" * 64
        self.write_all()
        self.assert_invalid("ownership_root_mismatch")

    def test_oracle_ownership_file_mode_is_rejected(self) -> None:
        (self.directory / "oracle-ownership-receipt.json").chmod(0o600)
        self.assert_invalid("ownership_file_mode")

    def test_quiescence_ownership_cross_binding_is_rejected(self) -> None:
        self.quiescence["oracle_ownership_root_sha256"] = "f" * 64
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_ownership_root_mismatch")

    def test_non_root_probe_smoke_uses_production_helper(self) -> None:
        account = pwd.getpwuid(os.geteuid())
        self.assertNotEqual(account.pw_uid, 0)
        workspace = self.directory / "ownership-smoke"
        workspace.mkdir(mode=0o750)
        result = executor.run_oracle_ownership_probe(
            workspace,
            account.pw_name,
            self.directory / "ownership-smoke.log",
        )
        self.assertEqual(result["writer_uid"], account.pw_uid)
        self.assertFalse((workspace / ".s1c3-v7-ownership-probe").exists())

    def test_candidate_config_drift_is_rejected(self) -> None:
        self.prep["candidate"]["config_sha256"] = "0" * 64
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("candidate_config_mismatch")

    def test_candidate_production_projection_drift_is_rejected(self) -> None:
        self.prep["candidate"]["production_projection_sha256"] = "0" * 64
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("candidate_production_projection_mismatch")

    def test_executor_production_projection_excludes_test_module(self) -> None:
        source = self.directory / "projection.rs"
        production = b"fn runtime() {}\n#[cfg(test)]\n"
        source.write_bytes(production + b"mod tests { changed(); }\n")
        self.assertEqual(
            executor.production_projection_sha256(source),
            verifier.digest(production),
        )

    def test_untouched_pid_change_is_rejected(self) -> None:
        unit = verifier.UNTOUCHED_UNITS[0]
        self.receipt["services_after"][unit]["main_pid"] += 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("untouched_after")

    def test_extra_restart_is_rejected(self) -> None:
        self.receipt["services_survival"][verifier.TRANSITION_UNIT]["nrestarts"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("transition_survival_nrestarts")

    def test_connector_failure_change_is_rejected(self) -> None:
        self.receipt["connector_after"]["route_receipt_failures"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("connector_route_receipt_failures")

    def test_raw_payload_is_rejected(self) -> None:
        self.receipt["journal_after"]["raw_payload_bytes"] = 1
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("journal_raw_payload_mismatch")

    def test_parity_divergence_is_rejected(self) -> None:
        self.parity["byte_identical"] = False
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("parity_identical_mismatch")

    def test_resource_budget_breach_is_rejected(self) -> None:
        self.resource["metrics"]["hot_latency"][0]["p99_ns"] = 1_000_001
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.prep["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("hot_latency_0_p99_budget")

    def test_precommit_stage_p99_breach_is_rejected(self) -> None:
        self.resource["metrics"]["three_ledger_sync"][0]["precommit_p99_ns"] = 5_000_001
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("three_ledger_sync_0_precommit_p99_ns_budget")

    def test_settlement_stage_p99_breach_is_rejected(self) -> None:
        self.resource["metrics"]["three_ledger_sync"][0]["settlement_p99_ns"] = 5_000_001
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("three_ledger_sync_0_settlement_p99_ns_budget")

    def test_episode_hard_max_breach_is_rejected(self) -> None:
        self.resource["metrics"]["three_ledger_sync"][0]["episode_hard_max_ns"] = 20_000_001
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("three_ledger_sync_0_episode_hard_max_ns_budget")

    def test_old_aggregate_resource_shape_is_rejected(self) -> None:
        self.resource["metrics"]["three_ledger_sync"][0] = {
            "p99_ns": 1_000_000,
            "hard_max_ns": 2_000_000,
            "samples": 256,
        }
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("three_ledger_sync_0_field_set")

    def test_capture_unavailable_is_rejected(self) -> None:
        self.receipt["capture_available"] = False
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("capture_available_mismatch")

    def test_quiescence_build_process_is_rejected(self) -> None:
        process = {"pid": 7, "comm": "rustc", "executable_basename": "rustc"}
        self.quiescence["eligible_window"][0]["build_processes_start"] = [process]
        self.quiescence["attempted_samples"][-30]["build_processes_start"] = [process]
        self.rehash_quiescence()
        self.assert_invalid("quiescence_sample_0_build_start_mismatch")

    def test_quiescence_cpu_mean_is_rejected(self) -> None:
        self.quiescence["eligible_window_cpu_mean_percent"]["4"] = 6.0
        self.rehash_quiescence()
        self.assert_invalid("quiescence_window_means_mismatch")

    def test_quiescence_io_pressure_is_rejected(self) -> None:
        self.quiescence["eligible_window"][0]["io_some_avg10"] = 0.21
        self.quiescence["attempted_samples"][-30]["io_some_avg10"] = 0.21
        self.rehash_quiescence()
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_topology_mismatch_is_rejected(self) -> None:
        self.quiescence["topology"][1]["logical_cpus"][0]["core_id"] = 8
        self.quiescence["topology_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["topology"])
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        self.assert_invalid("quiescence_topology_mismatch")

    def test_sibling_contention_is_recomputed(self) -> None:
        sample = self.quiescence["attempted_samples"][0]
        sample["end_cpu_counters"]["5"]["idle"] -= 30
        sample["end_cpu_snapshot_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(sample["end_cpu_counters"])
        )
        sample["cpu_busy_percent"]["5"] = 31.0
        self.rehash_quiescence()
        self.assert_invalid("quiescence_sample_0_blockers_mismatch")

    def test_simultaneous_snapshot_binding_is_rejected(self) -> None:
        self.quiescence["attempted_samples"][1]["start_cpu_snapshot_root_sha256"] = "0" * 64
        self.rehash_quiescence()
        self.assert_invalid("quiescence_sample_1_snapshot_continuity_mismatch")

    def test_lowest_index_tie_break_is_rejected(self) -> None:
        self.quiescence["selected_cpu"] = 6
        self.rehash_quiescence()
        self.assert_invalid("quiescence_selected_cpu_mismatch")

    def test_sliding_mean_summary_is_recomputed(self) -> None:
        self.quiescence["minimum_completed_window_mean_percent"]["4"]["4"] = 0.0
        self.rehash_quiescence()
        self.assert_invalid("quiescence_minimum_mean_4_mismatch")

    def test_sliding_mean_advances_to_next_window(self) -> None:
        samples = []
        process_snapshots = [process_snapshot() for _ in range(32)]
        counters = {str(cpu): {"total": 0, "idle": 0} for cpu in verifier.LOGICAL_CPU_POOL}
        for index, busy in enumerate([20.0] + [5.0] * 30):
            start_counters = copy.deepcopy(counters)
            for cpu in verifier.LOGICAL_CPU_POOL:
                counters[str(cpu)]["total"] += 100
                counters[str(cpu)]["idle"] += int(100 - busy)
            end_counters = copy.deepcopy(counters)
            start_ns = 1_000_000_000 + index * 1_000_000_000
            end_ns = start_ns + 1_000_000_000
            samples.append({
                "started_at": "2026-08-12T00:00:00Z",
                "ended_at": "2026-08-12T00:00:01Z",
                "start_monotonic_ns": start_ns,
                "end_monotonic_ns": end_ns,
                "interval_seconds": 1.0,
                "start_cpu_counters": start_counters,
                "end_cpu_counters": end_counters,
                "start_cpu_snapshot_root_sha256": verifier.digest(
                    verifier.canonical_bytes(start_counters)
                ),
                "end_cpu_snapshot_root_sha256": verifier.digest(
                    verifier.canonical_bytes(end_counters)
                ),
                "cpu_busy_percent": {
                    str(cpu): busy for cpu in verifier.LOGICAL_CPU_POOL
                },
                "io_some_avg10": 0.0,
                "io_full_avg10": 0.0,
                "build_processes_start": [],
                "build_processes_end": [],
                "start_process_snapshot_root_sha256": process_snapshots[index][
                    "process_snapshot_root_sha256"
                ],
                "end_process_snapshot_root_sha256": process_snapshots[index + 1][
                    "process_snapshot_root_sha256"
                ],
                "loadavg_end": "1.0 1.0 1.0 1/1 1",
                "blockers": [],
                "eligible_physical_cores": list(verifier.MEASUREMENT_CPU_POOL),
                "window_mean_blockers": (
                    list(verifier.LOGICAL_CPU_POOL) if index == 29 else []
                ),
            })
        self.quiescence.update({
            "eligibility_finished_monotonic_ns": 32_000_000_000,
            "attempted_interval_count": 31,
            "process_snapshot_count": len(process_snapshots),
            "process_snapshots": process_snapshots,
            "process_snapshots_root_sha256": verifier.digest(
                verifier.canonical_bytes(process_snapshots)
            ),
            "attempted_samples": samples,
            "attempted_samples_root_sha256": verifier.digest(
                verifier.canonical_bytes(samples)
            ),
            "eligible_window": copy.deepcopy(samples[-30:]),
            "eligible_window_cpu_mean_percent": {"4": 5.0, "5": 5.0},
            "eligible_window_root_sha256": verifier.digest(
                verifier.canonical_bytes(samples[-30:])
            ),
            "longest_eligible_streaks": {"4": 31, "6": 31},
            "minimum_completed_window_mean_percent": {
                "4": {"4": 5.0, "5": 5.0},
                "6": {"6": 5.0, "7": 5.0},
            },
        })
        for cpu in verifier.LOGICAL_CPU_POOL:
            self.quiescence["blocker_census"][f"cpu_busy_window_mean:{cpu}"] = 1
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.rebind_environment()
        result = verifier.verify_receipt(self.directory)
        self.assertEqual(result["verdict"], "S1C3_DEPLOYMENT_PASS")

    def test_forged_pass_from_timeout_rows_is_rejected(self) -> None:
        self.write_timeout_fixture()
        self.quiescence["verdict"] = "PASS"
        self.quiescence["selected_cpu"] = 4
        self.quiescence["eligibility_reached_at"] = "2026-08-12T00:30:00Z"
        self.quiescence["eligible_window"] = self.quiescence["attempted_samples"][-30:]
        self.quiescence["eligible_window_cpu_mean_percent"] = {"4": 30.0, "5": 30.0}
        self.quiescence["eligible_window_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.quiescence["eligible_window"])
        )
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.write("quiescence-receipt.json", self.quiescence)
        (self.directory / "quiescence-receipt.json").chmod(0o400)
        self.assert_invalid("quiescence_pass_without_window")

    def test_forged_blocker_census_is_rejected(self) -> None:
        self.quiescence["blocker_census"]["io_full"] = 1
        self.rehash_quiescence()
        self.assert_invalid("quiescence_blocker_census_mismatch")

    def test_forged_timeout_from_passing_rows_is_rejected(self) -> None:
        self.quiescence["verdict"] = "TIMEOUT"
        self.quiescence["selected_cpu"] = None
        self.quiescence["eligible_window"] = None
        self.quiescence["eligible_window_cpu_mean_percent"] = None
        self.quiescence["eligible_window_root_sha256"] = None
        self.quiescence["eligibility_reached_at"] = None
        self.quiescence = rooted(self.quiescence, "quiescence_root_sha256")
        self.write("quiescence-receipt.json", self.quiescence)
        (self.directory / "quiescence-receipt.json").chmod(0o400)
        self.assert_invalid("quiescence_timeout_with_passing_window")

    def test_valid_timeout_is_terminal_non_authority(self) -> None:
        self.write_timeout_fixture()
        result = verifier.verify_receipt(self.directory)
        self.assertEqual(result["verdict"], "INVALID_ENVIRONMENT_QUIESCENCE_TIMEOUT")
        self.assertFalse(result["authority"])

    def test_timeout_cannot_reach_resource(self) -> None:
        self.write_timeout_fixture()
        self.write("resource-receipt.json", self.resource)
        self.assert_invalid("timeout_post_quiescence_artifacts_mismatch")

    def test_resource_cpu_substitution_is_rejected(self) -> None:
        self.resource["measurement_cpu"] = 6
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.prep["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["resource_root_sha256"] = self.resource["resource_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("resource_measurement_cpu_mismatch")

    def test_quiescence_file_mode_is_rejected(self) -> None:
        (self.directory / "quiescence-receipt.json").chmod(0o600)
        self.assert_invalid("quiescence_file_mode")

    def test_measurement_build_process_is_rejected(self) -> None:
        process = {"pid": 8, "comm": "cargo", "executable_basename": "cargo"}
        self.contamination["samples"][0]["build_processes"] = [process]
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.rebind_environment()
        self.assert_invalid("contamination_sample_0_build_mismatch")

    def test_measurement_gap_is_rejected(self) -> None:
        self.contamination["samples"][1]["monotonic_ns"] = (
            self.contamination["samples"][0]["monotonic_ns"] + 2_100_000_000
        )
        for index in range(2, len(self.contamination["samples"])):
            self.contamination["samples"][index]["monotonic_ns"] = (
                self.contamination["samples"][index - 1]["monotonic_ns"] + 100_000_000
            )
        self.contamination["observed_max_sample_gap_seconds"] = 2.1
        self.contamination = rooted(
            self.contamination, "measurement_contamination_root_sha256"
        )
        self.rebind_environment()
        self.assert_invalid("contamination_observed_gap_budget")

    def test_resource_direct_exec_false_is_rejected(self) -> None:
        self.resource["direct_exec_only"] = False
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("resource_direct_exec_mismatch")

    def test_compiler_after_quiescence_is_rejected(self) -> None:
        self.resource["compiler_invocations_after_quiescence"] = 1
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.rebind_environment()
        self.assert_invalid("resource_compiler_invocations_mismatch")

    def test_parity_oracle_substitution_is_rejected(self) -> None:
        self.parity["candidate_oracle_sha256"] = "f" * 64
        self.parity = rooted(self.parity, "parity_root_sha256")
        self.prep["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.prep = rooted(self.prep, "preparation_root_sha256")
        self.receipt["parity_root_sha256"] = self.parity["parity_root_sha256"]
        self.receipt["preparation_root_sha256"] = self.prep["preparation_root_sha256"]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write_all()
        self.assert_invalid("parity_candidate_oracle_mismatch")

    def test_remote_executor_has_no_build_command_after_quiescence(self) -> None:
        executor = Path(__file__).with_name("s1c3_remote_transaction_v7.py").read_text(
            encoding="utf-8"
        )
        measured_stage = executor.split(
            "quiescence = wait_for_quiescence", 1
        )[1].split("def exact_untouched", 1)[0]
        self.assertNotIn('"/home/e/.cargo/bin/cargo"', measured_stage)
        self.assertNotIn('"/home/e/.cargo/bin/rustc"', measured_stage)

    def test_timeout_returns_before_resource_measurement(self) -> None:
        executor_source = Path(__file__).with_name(
            "s1c3_remote_transaction_v7.py"
        ).read_text(encoding="utf-8")
        prepare = executor_source.split("def prepare(", 1)[1].split(
            "def exact_untouched", 1
        )[0]
        timeout_return = prepare.index('if quiescence["verdict"] == "TIMEOUT"')
        resource_call = prepare.index("resource = run_resources(")
        self.assertLess(timeout_return, resource_call)

    def test_launcher_verifies_before_arming_rollback(self) -> None:
        launcher = Path(__file__).with_name(
            "run_s1c3_transaction_v7.sh"
        ).read_text(encoding="utf-8")
        verification = launcher.index("--pre-deployment")
        rollback_arm = launcher.index("rollback_armed=true")
        execute = launcher.index("s1c3_remote_transaction_v7.py' execute")
        self.assertLess(verification, rollback_arm)
        self.assertLess(rollback_arm, execute)

    def test_predeployment_receipt_substitution_is_rejected(self) -> None:
        receipt = json.loads(
            (self.directory / "predeployment-verification.json").read_text(
                encoding="utf-8"
            )
        )
        receipt["selected_cpu"] = 6
        receipt = rooted(receipt, "predeployment_verification_root_sha256")
        self.write("predeployment-verification.json", receipt)
        self.receipt["predeployment_verification_root_sha256"] = receipt[
            "predeployment_verification_root_sha256"
        ]
        self.receipt = rooted(self.receipt, "receipt_root_sha256")
        self.write("deployment-receipt.json", self.receipt)
        self.assert_invalid("predeployment_verification_receipt_mismatch")

    def test_launcher_enforces_single_remote_attempt(self) -> None:
        launcher = Path(__file__).with_name(
            "run_s1c3_transaction_v7.sh"
        ).read_text(encoding="utf-8")
        census = launcher.index("prior_attempts=")
        remote_upload = launcher.index('ssh "$remote" "set -e; test ! -e')
        self.assertLess(census, remote_upload)
        self.assertIn("v7_attempt_already_exists", launcher)

    def test_executor_never_changes_production_affinity(self) -> None:
        executor_source = Path(__file__).with_name(
            "s1c3_remote_transaction_v7.py"
        ).read_text(encoding="utf-8")
        for forbidden in (
            "systemctl set-property",
            "CPUAffinity=",
            "sched_setaffinity(old_pid",
            "sched_setaffinity(new_pid",
        ):
            self.assertNotIn(forbidden, executor_source)

    def test_receipt_tamper_without_rehash_is_rejected(self) -> None:
        self.receipt["survival_seconds"] = 14
        self.write_all()
        self.assert_invalid("receipt_root_mismatch")


if __name__ == "__main__":
    unittest.main()
