#!/usr/bin/env python3
"""Fault-injection tests for S1C-3B executor and independent verifier."""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

import s1c3b_remote_transaction_v1 as executor
import verify_s1c3b_transaction_v1 as verifier


def rooted(value: dict, field: str) -> dict:
    value = dict(value)
    value[field] = verifier.digest(verifier.canonical_bytes(value, field))
    return value


def executable(path: str, sha: str = "a" * 64, source: str = "fixture") -> dict:
    return {
        "path": path,
        "sha256": sha,
        "size_bytes": 100,
        "mode_octal": "0500",
        "source_identity": source,
    }


def process_snapshot(rows: list[dict] | None = None) -> dict:
    rows = copy.deepcopy(rows or [])
    results = [
        (row["pid"], executor.legacy.classify_process_observation(row))
        for row in rows
    ]
    rows = [{**row, **result} for row, (_, result) in zip(rows, results)]
    reasons = sorted({result["reason"] for _, result in results})
    unresolved = [
        executor.legacy.process_row_projection(row)
        for row in rows
        if row["classification"] == verifier.legacy_verifier.UNRESOLVED_PROCESS_OBSERVATION
    ]
    forbidden = [
        {
            **executor.legacy.process_row_projection(row),
            "matched_names": row["forbidden_names"],
        }
        for row in rows
        if row["forbidden_names"]
    ]
    return rooted({
        "schema": verifier.legacy_verifier.PROCESS_SNAPSHOT_SCHEMA,
        "detector_schema": verifier.legacy_verifier.PROCESS_DETECTOR_SCHEMA,
        "rows": rows,
        "summary": {
            "total_rows": len(results),
            "observable_user_process_count": sum(
                result["classification"] == verifier.legacy_verifier.OBSERVABLE_USER_PROCESS
                for _, result in results
            ),
            "proven_vanished_count": sum(result["reason"] == "PID_VANISHED" for _, result in results),
            "proven_zombie_count": sum(result["reason"] == "STABLE_ZOMBIE" for _, result in results),
            "proven_kernel_thread_count": sum(
                result["reason"] == "STABLE_KERNEL_THREAD" for _, result in results
            ),
            "unresolved_process_count": sum(
                result["classification"] == verifier.legacy_verifier.UNRESOLVED_PROCESS_OBSERVATION
                for _, result in results
            ),
            "reason_counts": {
                reason: sum(result["reason"] == reason for _, result in results)
                for reason in reasons
            },
        },
        "unresolved_rows": unresolved,
        "forbidden_process_matches": forbidden,
    }, "process_snapshot_root_sha256")


def process_stat(pid: int, comm: str, parent: int, starttime: int = 10) -> dict:
    tail = ["S", str(parent), *(["0"] * 17), str(starttime), *(["0"] * 30)]
    raw = f"{pid} ({comm}) " + " ".join(tail)
    return {
        "status": "VALUE",
        "raw": raw,
        "parsed": {"pid": pid, "comm": comm, "state": "S", "starttime": starttime},
    }


def user_process(pid: int, comm: str, parent: int, basename: str) -> dict:
    opening = process_stat(pid, comm, parent)
    return {
        "pid": pid,
        "opening_stat": opening,
        "status": {"status": "VALUE", "kthread_line": "Kthread:\t0"},
        "cmdline": {"status": "VALUE", "byte_count": 4, "sha256": verifier.digest(b"cmd\0")},
        "exe": {"status": "VALUE", "target": f"/usr/bin/{basename}", "basename": basename},
        "closing_stat": copy.deepcopy(opening),
    }


def service_snapshot(transition_pid: int) -> dict:
    return {
        unit: {
            "active_state": "active",
            "sub_state": "running",
            "main_pid": transition_pid if unit == verifier.TRANSITION_UNIT else 2000 + index,
            "nrestarts": 0,
            "fragment_sha256": verifier.digest(unit.encode()),
        }
        for index, unit in enumerate(verifier.ALL_UNITS)
    }


def connector(label: str = "before") -> dict:
    return {
        "schema": "nando.s1c3b-connector-snapshot.v1",
        "label": label,
        "observed_at": "2026-08-12T00:00:00Z",
        "active_state": "active",
        "main_pid": 2919,
        "nrestarts": 0,
        "route_receipt_failures": 0,
        "command_sha256": "a" * 64,
    }


def health() -> dict:
    semantic = {
        "ok": True,
        "service": "fixture",
        "mode": "CPU",
        "admission_verdict": "PASS",
        "transition_active_profiles": 2,
        "response_active_profiles": 2,
        "response_executor_cache_ready": True,
    }
    return {
        label: {
            "url": f"http://fixture/{label}",
            "raw_sha256": verifier.digest(label.encode()),
            "semantic": copy.deepcopy(semantic),
            "semantic_root_sha256": verifier.digest(verifier.canonical_bytes(semantic)),
        }
        for label in ("hot", "control", "gateway", "cpu")
    }


def journal() -> dict:
    return {
        "present": True,
        "entries": [],
        "total_bytes": 0,
        "manifest_root_sha256": verifier.digest(b""),
        "raw_payload_bytes": 0,
        "preserved_prefixes": True,
    }


def ownership_receipt(transaction_id: str = "fixture") -> dict:
    rows = {}
    probe = {
        "writer_uid": 1000,
        "writer_gid": 1000,
        "probe_uid": 1000,
        "probe_gid": 1000,
        "probe_mode_octal": "0600",
        "create_fsync_unlink_pass": True,
        "directory_fsync_pass": True,
    }
    for label in ("baseline", "candidate"):
        paths = verifier.legacy_verifier.expected_oracle_paths(transaction_id, label)
        identity = lambda path, mode: {
            "path": path, "uid": 1000, "gid": 1000, "mode_octal": mode,
        }
        rows[label] = rooted({
            "schema": verifier.OWNERSHIP_ROW_SCHEMA,
            "label": label,
            "workspace": identity(paths["workspace"], "0750"),
            "src": identity(f'{paths["workspace"]}/src', "0750"),
            "cargo_toml": identity(paths["manifest"], "0640"),
            "main_rs": identity(paths["main"], "0640"),
            "main_rs_sha256": executor.legacy.ORACLE_SOURCE_SHA256,
            "cargo_lock": {
                **identity(paths["lock"], "0640"),
                "sha256_before_build": executor.legacy.ORACLE_LOCK_SHA256,
                "sha256_after_build": executor.legacy.ORACLE_LOCK_SHA256,
            },
            "manifest_sha256": verifier.digest(
                verifier.legacy_verifier.oracle_manifest(paths["source"])
            ),
            "build_command": [*executor.legacy.ORACLE_CARGO_COMMAND_PREFIX, paths["manifest"]],
            "build_environment": {
                **executor.legacy.ORACLE_CARGO_ENVIRONMENT,
                "CARGO_TARGET_DIR": paths["target"],
            },
            "target_directory": paths["target"],
            "executable_path": paths["executable"],
            "probe": copy.deepcopy(probe),
            "probe_retained": False,
        }, "ownership_row_root_sha256")
    return rooted({
        "schema": verifier.OWNERSHIP_SCHEMA,
        "transaction_id": transaction_id,
        "build_user": {"name": "e", "uid": 1000, "gid": 1000},
        "rows": rows,
        "rows_root_sha256": verifier.digest(verifier.canonical_bytes(rows)),
        "oracle_build_contract": {
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
            "manifest_sha256": {
                label: rows[label]["manifest_sha256"]
                for label in ("baseline", "candidate")
            },
        },
    }, "ownership_root_sha256")


class SyntheticTransaction:
    def __init__(self, directory: Path) -> None:
        self.directory = directory
        self.evidence = directory / "evidence"
        self.evidence.mkdir()
        self.transaction_id = "fixture"
        self.ownership = ownership_receipt(self.transaction_id)
        self.executables = self.make_executables()
        self.executable_root = verifier.digest(verifier.canonical_bytes(self.executables))
        self.commands = [self.make_command(label) for label in verifier.MEASUREMENT_LABELS]
        self.monitor = self.make_monitor()
        self.parity = self.make_parity()
        self.resource = self.make_resource()
        self.preparation = self.make_preparation()
        self.predeployment = self.make_predeployment()
        self.receipt = self.make_receipt()
        self.write_pass()

    def write_json(self, name: str, value: dict, mode: int = 0o600) -> None:
        path = self.directory / name
        if path.exists():
            path.chmod(0o600)
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
        path.chmod(mode)

    def make_executables(self) -> dict:
        paths = {
            "candidate-binary": "/tmp/s1c3b-candidate",
            "test-response-actor": "/tmp/s1c3b-response-test",
            "test-transition-serving": "/tmp/s1c3b-transition-test",
            "parity-baseline": verifier.legacy_verifier.expected_oracle_paths(
                self.transaction_id, "baseline"
            )["executable"],
            "parity-candidate": verifier.legacy_verifier.expected_oracle_paths(
                self.transaction_id, "candidate"
            )["executable"],
            "python-runtime": "/usr/bin/python3",
            "affinity-wrapper": "/tmp/s1c3b-affinity-exec.py",
            "filesystem-floor-probe": "/tmp/s1c3b-filesystem-floor.py",
        }
        sources = {
            "candidate-binary": verifier.CANDIDATE_COMMIT,
            "test-response-actor": verifier.CANDIDATE_COMMIT,
            "test-transition-serving": verifier.CANDIDATE_COMMIT,
            "parity-baseline": executor.BASELINE_COMMIT,
            "parity-candidate": verifier.CANDIDATE_COMMIT,
            "python-runtime": "host-runtime",
            "affinity-wrapper": verifier.PAPER_COMMIT,
            "filesystem-floor-probe": verifier.PAPER_COMMIT,
        }
        return {
            name: executable(path, f"{index + 1:x}" * 64, sources[name])
            for index, (name, path) in enumerate(paths.items())
        }

    def executable_name_for_label(self, label: str) -> str:
        if label.startswith("floor-"):
            return "python-runtime"
        if label.startswith(("hot-", "idle")):
            return "test-response-actor"
        if label.startswith(("single-sync-", "three-sync-")):
            return "test-transition-serving"
        if label.startswith("rss-"):
            return "candidate-binary"
        return label

    def make_command(self, label: str) -> dict:
        identity = self.executables[self.executable_name_for_label(label)]
        return {
            "label": label,
            "executable": identity["path"],
            "executable_sha256": identity["sha256"],
            "requested_affinity": [verifier.MEASUREMENT_CPU],
            "observed_affinity": [verifier.MEASUREMENT_CPU],
            "wrapper_reported_executable_sha256": identity["sha256"],
            "returncode": 0,
            "error": None,
        }

    def make_monitor(self, process_rows: list[dict] | None = None) -> dict:
        snapshot = process_snapshot(process_rows)
        samples = []
        boundaries = []
        monotonic = 1_000_000_000

        def sample(kind: str, label: str) -> int:
            nonlocal monotonic
            row = {
                "label": label,
                "observed_at": "2026-08-12T00:00:00Z",
                "monotonic_ns": monotonic,
                "cpu_counters": {
                    "4": {"total": 100, "idle": 99},
                    "5": {"total": 100, "idle": 99},
                },
                "cpu_snapshot_root_sha256": "f" * 64,
                "cpu_pressure": {
                    "some": {"avg10": 0.0, "avg60": 0.0, "avg300": 0.0, "total": 0},
                    "full": {"avg10": 0.0, "avg60": 0.0, "avg300": 0.0, "total": 0},
                },
                "io_pressure": {
                    "some": {"avg10": 0.0, "avg60": 0.0, "avg300": 0.0, "total": 0},
                    "full": {"avg10": 0.0, "avg60": 0.0, "avg300": 0.0, "total": 0},
                },
                "loadavg": "1.0 1.0 1.0 1/1 1",
                "memory_available_bytes": 1024 * 1024 * 1024,
                "block_device_counters": [],
                "process_snapshot": copy.deepcopy(snapshot),
                "process_snapshot_root_sha256": snapshot["process_snapshot_root_sha256"],
                "foreign_build_processes": copy.deepcopy(snapshot["forbidden_process_matches"]),
                "unresolved_processes": copy.deepcopy(snapshot["unresolved_rows"]),
                "transaction_owned_build_processes": executor.transaction_owned_builds(snapshot, 4242),
                "observer_affinity": [0],
                "kind": kind,
            }
            samples.append(row)
            monotonic += 100_000_000
            return len(samples) - 1

        sample("monitor-start", "before-first-metric")
        for label in verifier.MEASUREMENT_LABELS:
            for phase in ("before", "after"):
                index = sample(f"boundary-{phase}", label)
                boundaries.append({
                    "label": label,
                    "phase": phase,
                    "observed_at": samples[index]["observed_at"],
                    "monotonic_ns": samples[index]["monotonic_ns"],
                    "sample_index": index,
                })
        sample("monitor-stop", verifier.MEASUREMENT_LABELS[-1])
        gaps = [
            (right["monotonic_ns"] - left["monotonic_ns"]) / 1_000_000_000
            for left, right in zip(samples, samples[1:])
        ]
        owned = [
            {"sample_index": index, "process": process}
            for index, row in enumerate(samples)
            for process in row["transaction_owned_build_processes"]
        ]
        return rooted({
            "schema": verifier.MONITOR_SCHEMA,
            "transaction_id": self.transaction_id,
            "executor_pid": 4242,
            "measurement_cpu": verifier.MEASUREMENT_CPU,
            "measurement_sibling": executor.MEASUREMENT_SIBLING,
            "measurement_started_at": "2026-08-12T00:00:00Z",
            "measurement_started_monotonic_ns": 1_000_000_000,
            "measurement_finished_at": "2026-08-12T00:00:05Z",
            "measurement_finished_monotonic_ns": samples[-1]["monotonic_ns"],
            "monitor_interval_seconds": 0.5,
            "maximum_sample_gap_seconds": 2.0,
            "observed_max_sample_gap_seconds": max(gaps),
            "metric_labels": list(verifier.MEASUREMENT_LABELS),
            "boundaries": boundaries,
            "commands": copy.deepcopy(self.commands),
            "samples": samples,
            "transaction_owned_build_processes": owned,
            "monitor_errors": [],
            "instrument_pass": not owned and max(gaps) <= 2.0,
            "executable_set_root_sha256": self.executable_root,
        }, "monitor_root_sha256")

    def affinity_line(self, label: str) -> str:
        command = next(row for row in self.commands if row["label"] == label)
        return (
            f'S1C3B_AFFINITY pid=1 cpus=4 '
            f'executable_sha256={command["executable_sha256"]}\n'
        )

    def metric_log(self, label: str, override: dict | None = None) -> tuple[bytes, dict]:
        override = override or {}
        if label.startswith("hot-"):
            metric = {"p99_ns": 10_000, "no_goal_p99_ns": 1_000,
                      "hard_max_ns": 20_000, "samples": 4096, **override}
            line = (
                "S1C_HOT_LATENCY "
                f'matched_p99_ns={metric["p99_ns"]} '
                f'no_goal_p99_ns={metric["no_goal_p99_ns"]} '
                f'hard_max_ns={metric["hard_max_ns"]} samples={metric["samples"]}'
            )
        elif label.startswith("single-sync-"):
            metric = {"p99_ns": 1_000_000, "hard_max_ns": 2_000_000,
                      "samples": 1024, "segments": 2, **override}
            line = (
                "S1C_SYNC_LATENCY "
                f'p99_ns={metric["p99_ns"]} hard_max_ns={metric["hard_max_ns"]} '
                f'records={metric["samples"]} segments={metric["segments"]}'
            )
        elif label.startswith("three-sync-"):
            metric = {
                "precommit_p99_ns": 1_000_000,
                "precommit_hard_max_ns": 2_000_000,
                "settlement_p99_ns": 2_000_000,
                "settlement_hard_max_ns": 3_000_000,
                "episode_p99_ns": 3_000_000,
                "episode_hard_max_ns": 4_000_000,
                "samples": 256,
                **override,
            }
            line = (
                "S1C3_STAGE_SYNC_LATENCY "
                f'precommit_p99_ns={metric["precommit_p99_ns"]} '
                f'precommit_hard_max_ns={metric["precommit_hard_max_ns"]} '
                f'settlement_p99_ns={metric["settlement_p99_ns"]} '
                f'settlement_hard_max_ns={metric["settlement_hard_max_ns"]} '
                f'episode_p99_ns={metric["episode_p99_ns"]} '
                f'episode_hard_max_ns={metric["episode_hard_max_ns"]} '
                f'records={metric["samples"]}'
            )
        else:
            metric = {"elapsed_ticks": 0, "ticks_per_second": 100,
                      "percent_of_one_core": 0.0, **override}
            line = (
                "S1C_IDLE_CPU "
                f'elapsed_ticks={metric["elapsed_ticks"]} '
                f'ticks_per_second={metric["ticks_per_second"]} '
                f'percent_of_one_core={metric["percent_of_one_core"]}'
            )
        raw = (
            self.affinity_line(label)
            + line
            + "\ntest result: ok. 1 passed; 0 failed;\n"
        ).encode()
        return raw, metric

    def make_floor(self, round_index: int, position: str) -> dict:
        label = f"floor-{position}-{round_index}"
        samples = [1000] * verifier.FLOOR_RECORDS
        raw = (self.affinity_line(label) + json.dumps({"samples_ns": samples}) + "\n").encode()
        (self.evidence / f"{label}.log").write_bytes(raw)
        command = next(row for row in self.commands if row["label"] == label)
        return {
            "label": label,
            "round": round_index,
            "position": position,
            "records": verifier.FLOOR_RECORDS,
            "samples_ns": samples,
            "samples_root_sha256": verifier.digest(verifier.canonical_bytes(samples)),
            "p50_ns": 1000,
            "p99_ns": 1000,
            "hard_max_ns": 1000,
            "filesystem": {
                "device": 1,
                "filesystem_id": 2,
                "block_size": 4096,
                "findmnt": "/dev/test ext4 rw",
                "findmnt_returncode": 0,
            },
            "diagnostic_only": True,
            "returncode": 0,
            "error": None,
            "command": copy.deepcopy(command),
        }

    def make_metric(self, label: str) -> dict:
        raw, metric = self.metric_log(label)
        (self.evidence / f"{label}.log").write_bytes(raw)
        test = executor.IDLE_TEST
        if label.startswith("hot-"):
            test = executor.HOT_TEST
        elif label.startswith("single-sync-"):
            test = executor.SINGLE_SYNC_TEST
        elif label.startswith("three-sync-"):
            test = executor.THREE_SYNC_TEST
        command = next(row for row in self.commands if row["label"] == label)
        return {
            "label": label,
            "test": test,
            "returncode": 0,
            "test_assertion_pass": True,
            "metric_present": True,
            "metrics": metric,
            "output_sha256": verifier.digest(raw),
            "command": copy.deepcopy(command),
        }

    def make_parity(self) -> dict:
        rows = []
        payload = b"\n".join(
            json.dumps({"row": index}, sort_keys=True).encode() for index in range(16)
        )
        payload_root = verifier.digest(payload)
        for label in ("baseline", "candidate"):
            metric_label = f"parity-{label}"
            raw = self.affinity_line(metric_label).encode() + payload + b"\n"
            (self.evidence / f"{metric_label}.log").write_bytes(raw)
            command = next(row for row in self.commands if row["label"] == metric_label)
            rows.append({
                "label": label,
                "returncode": 0,
                "output_sha256": verifier.digest(raw),
                "row_count": 16,
                "command": copy.deepcopy(command),
            })
        return rooted({
            "schema": verifier.PARITY_SCHEMA,
            "rows": rows,
            "byte_identical": True,
            "row_count": 16,
            "baseline_output_sha256": payload_root,
            "candidate_output_sha256": payload_root,
        }, "parity_root_sha256")

    def make_resource(self) -> dict:
        floors = []
        hot = []
        single = []
        three = []
        for round_index in range(1, 4):
            floors.append(self.make_floor(round_index, "before"))
            hot.append(self.make_metric(f"hot-{round_index}"))
            single.append(self.make_metric(f"single-sync-{round_index}"))
            three.append(self.make_metric(f"three-sync-{round_index}"))
            floors.append(self.make_floor(round_index, "after"))
        idle = self.make_metric("idle")
        rss_rows = []
        for index, label in enumerate(("capture_off", "capture_on")):
            metric_label = f"rss-{label}"
            command = next(row for row in self.commands if row["label"] == metric_label)
            (self.evidence / f"{metric_label}.log").write_bytes(
                self.affinity_line(metric_label).encode()
            )
            rss_rows.append({
                "label": label,
                "rss_bytes": 100_000 + index * 1000,
                "sample_count": 20,
                "error": None,
                "command": copy.deepcopy(command),
            })
        return rooted({
            "schema": verifier.RESOURCE_SCHEMA,
            "candidate_commit": verifier.CANDIDATE_COMMIT,
            "measurement_cpu": verifier.MEASUREMENT_CPU,
            "round_count": verifier.ROUND_COUNT,
            "floor_probes": floors,
            "metrics": {
                "hot_latency": hot,
                "single_ledger_sync": single,
                "three_ledger_sync": three,
                "idle_cpu": idle,
                "rss": {"rows": rss_rows, "delta_bytes": 1000},
            },
            "resource_failures": [],
            "instrument_failures": [],
            "all_pass_before_monitor": True,
            "all_pass": True,
            "executable_set_root_sha256": self.executable_root,
            "oracle_ownership_root_sha256": self.ownership["ownership_root_sha256"],
            "monitor_root_sha256": self.monitor["monitor_root_sha256"],
            "parity_root_sha256": self.parity["parity_root_sha256"],
        }, "resource_root_sha256")

    def make_preparation(self) -> dict:
        rollback_entries = [
            {"path": "nando-transition-serving", "sha256": verifier.BASELINE_BINARY_SHA256,
             "size_bytes": 100},
            {"path": "transition-serving.env", "sha256": verifier.BASELINE_CONFIG_SHA256,
             "size_bytes": 100},
            {"path": "nando-transition-serving.service", "sha256": verifier.UNIT_SHA256,
             "size_bytes": 100},
            {"path": "previous-deployment-receipt.json",
             "sha256": verifier.CURRENT_RECEIPT_FILE_SHA256, "size_bytes": 100},
        ]
        manifest = "".join(
            f'{row["sha256"]} {row["size_bytes"]} {row["path"]}\n'
            for row in sorted(rollback_entries, key=lambda item: item["path"])
        ).encode()
        before = service_snapshot(1000)
        return rooted({
            "schema": verifier.PREPARATION_SCHEMA,
            "transaction_id": self.transaction_id,
            "state": "PREPARED",
            "created_at": "2026-08-12T00:00:00Z",
            "paper": {
                "commit": verifier.PAPER_COMMIT,
                "tree": verifier.PAPER_TREE,
                "manifest_root_sha256": verifier.PAPER_MANIFEST_ROOT,
                "verification_sha256": verifier.PAPER_VERIFICATION_SHA256,
                "critique_sha256": verifier.PAPER_CRITIQUE_SHA256,
            },
            "candidate": {
                "source_commit": verifier.CANDIDATE_COMMIT,
                "source_tree": verifier.CANDIDATE_TREE,
                "cargo_lock_sha256": verifier.CARGO_LOCK_SHA256,
                "binary_sha256": self.executables["candidate-binary"]["sha256"],
                "binary_size_bytes": self.executables["candidate-binary"]["size_bytes"],
                "config_sha256": verifier.CANDIDATE_CONFIG_SHA256,
                "production_projection_path": verifier.PRODUCTION_PROJECTION_PATH,
                "production_projection_schema": verifier.PRODUCTION_PROJECTION_SCHEMA,
                "production_projection_sha256": verifier.PRODUCTION_PROJECTION_SHA256,
            },
            "production": {
                "receipt_path": str(executor.CURRENT_RECEIPT),
                "receipt_root_sha256": verifier.CURRENT_RECEIPT_ROOT,
                "source_commit": verifier.CURRENT_RECEIPT_COMMIT,
                "source_tree": verifier.CURRENT_RECEIPT_TREE,
                "binary_sha256": verifier.BASELINE_BINARY_SHA256,
                "config_sha256": verifier.BASELINE_CONFIG_SHA256,
            },
            "services_before": before,
            "health_before": health(),
            "economics_before": {"false_accepts": 0, "runtime_parity_mismatches": 0},
            "route_probe_before": {"status": 418, "body_sha256": "f" * 64, "body_size": 10},
            "connector_before": connector(),
            "journal_before": journal(),
            "transition_rss_before": 100_000,
            "measurement_cpu": verifier.MEASUREMENT_CPU,
            "executable_set_root_sha256": self.executable_root,
            "oracle_ownership_root_sha256": self.ownership["ownership_root_sha256"],
            "monitor_root_sha256": self.monitor["monitor_root_sha256"],
            "resource_root_sha256": self.resource["resource_root_sha256"],
            "parity_root_sha256": self.parity["parity_root_sha256"],
            "rollback": {
                "manifest_root_sha256": verifier.digest(manifest),
                "entries": rollback_entries,
            },
        }, "preparation_root_sha256")

    def make_predeployment(self) -> dict:
        result = {
            "schema": verifier.PREDEPLOYMENT_SCHEMA,
            "valid": True,
            "authority": True,
            "verdict": "S1C3B_PREPARATION_PASS",
            "preparation_root_sha256": self.preparation["preparation_root_sha256"],
            "oracle_ownership_root_sha256": self.ownership["ownership_root_sha256"],
            "monitor_root_sha256": self.monitor["monitor_root_sha256"],
            "resource_root_sha256": self.resource["resource_root_sha256"],
            "parity_root_sha256": self.parity["parity_root_sha256"],
        }
        result["predeployment_verification_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(result)
        )
        return result

    def make_receipt(self) -> dict:
        before = self.preparation["services_before"]
        after = service_snapshot(1001)
        after_connector = connector("after")
        return rooted({
            "schema": verifier.SCHEMA,
            "transaction_id": self.transaction_id,
            "verdict": "S1C3B_DEPLOYMENT_PASS",
            "finalized_at": "2026-08-12T00:00:16Z",
            "preparation_root_sha256": self.preparation["preparation_root_sha256"],
            "oracle_ownership_root_sha256": self.ownership["ownership_root_sha256"],
            "monitor_root_sha256": self.monitor["monitor_root_sha256"],
            "executable_set_root_sha256": self.executable_root,
            "resource_root_sha256": self.resource["resource_root_sha256"],
            "parity_root_sha256": self.parity["parity_root_sha256"],
            "predeployment_verification_root_sha256": self.predeployment[
                "predeployment_verification_root_sha256"
            ],
            "services_before": before,
            "services_after": after,
            "services_survival": copy.deepcopy(after),
            "health_before": self.preparation["health_before"],
            "health_after": copy.deepcopy(self.preparation["health_before"]),
            "health_survival": copy.deepcopy(self.preparation["health_before"]),
            "route_probe_before": self.preparation["route_probe_before"],
            "route_probe_after": copy.deepcopy(self.preparation["route_probe_before"]),
            "route_probe_survival": copy.deepcopy(self.preparation["route_probe_before"]),
            "connector_before": self.preparation["connector_before"],
            "connector_after": after_connector,
            "installed_binary_sha256": self.preparation["candidate"]["binary_sha256"],
            "installed_config_sha256": verifier.CANDIDATE_CONFIG_SHA256,
            "immutable_after": {
                "unit_sha256": verifier.UNIT_SHA256,
                "phase_config_sha256": verifier.PHASE_CONFIG_SHA256,
                "authority_config_sha256": verifier.AUTHORITY_CONFIG_SHA256,
            },
            "capture_environment": {
                "NANDO_GROUNDED_DECISION_SHADOW_ENABLED": "1",
                "NANDO_GROUNDED_DECISION_JOURNAL": str(executor.JOURNAL),
            },
            "capture_available": True,
            "startup_log_clean": True,
            "health_semantics_preserved": True,
            "route_probe_equivalent": True,
            "active_packages_preserved": True,
            "false_accepts_after": 0,
            "runtime_parity_failures_after": 0,
            "journal_before": self.preparation["journal_before"],
            "journal_after": journal(),
            "transition_rss_before": self.preparation["transition_rss_before"],
            "transition_rss_after": self.preparation["transition_rss_before"] + 1000,
            "survival_seconds": 15,
            "veto_reasons": [],
        }, "receipt_root_sha256")

    def write_common(self) -> None:
        self.write_json("oracle-ownership-receipt.json", self.ownership, 0o400)
        executable_receipt = rooted({
            "schema": "nando.s1c3b-executable-identities.v1",
            "transaction_id": self.transaction_id,
            "before": self.executables,
            "after": copy.deepcopy(self.executables),
        }, "executable_identities_root_sha256")
        self.write_json("executable-identities.json", executable_receipt)
        self.write_json("measurement-monitor-receipt.json", self.monitor)
        self.write_json("resource-receipt.json", self.resource)
        self.write_json("parity-receipt.json", self.parity)

    def write_pass(self) -> None:
        self.write_common()
        self.write_json("preparation.json", self.preparation)
        self.write_json("predeployment-verification.json", self.predeployment)
        self.write_json("deployment-receipt.json", self.receipt)
        self.write_json("transaction-state.json", {
            "schema": verifier.STATE_SCHEMA,
            "state": "COMPLETE",
            "transaction_id": self.transaction_id,
            "verdict": self.receipt["verdict"],
        })

    def rehash_monitor(self) -> None:
        monotonic = [row["monotonic_ns"] for row in self.monitor["samples"]]
        gaps = [(right - left) / 1_000_000_000 for left, right in zip(monotonic, monotonic[1:])]
        owned = [
            {"sample_index": index, "process": process}
            for index, row in enumerate(self.monitor["samples"])
            for process in row["transaction_owned_build_processes"]
        ]
        self.monitor["observed_max_sample_gap_seconds"] = max(gaps)
        self.monitor["transaction_owned_build_processes"] = owned
        self.monitor["instrument_pass"] = (
            not self.monitor["monitor_errors"] and not owned and max(gaps) <= 2.0
        )
        self.monitor = rooted(self.monitor, "monitor_root_sha256")
        self.resource["monitor_root_sha256"] = self.monitor["monitor_root_sha256"]

    def write_veto(self) -> None:
        self.resource = rooted(self.resource, "resource_root_sha256")
        self.write_common()
        for name in ("preparation.json", "predeployment-verification.json", "deployment-receipt.json"):
            path = self.directory / name
            if path.exists():
                path.unlink()
        self.write_json("transaction-state.json", {
            "schema": verifier.STATE_SCHEMA,
            "state": "RESOURCE_VETO",
            "verdict": "S1C3B_RESOURCE_VETO",
            "transaction_id": self.transaction_id,
            "production_mutation": False,
            "resource_root_sha256": self.resource["resource_root_sha256"],
        })


class ExecutorTests(unittest.TestCase):
    def test_measurement_labels_freeze_round_order(self) -> None:
        self.assertEqual(
            executor.MEASUREMENT_LABELS[:5],
            ("floor-before-1", "hot-1", "single-sync-1", "three-sync-1", "floor-after-1"),
        )
        self.assertEqual(len(executor.MEASUREMENT_LABELS), 20)

    def test_resource_evaluator_continues_after_first_assertion_failure(self) -> None:
        calls = []

        def fake_floor(round_index, position, *args):
            calls.append(f"floor-{position}-{round_index}")
            return {
                "label": calls[-1], "round": round_index, "position": position,
                "records": 256, "samples_ns": [1] * 256,
                "samples_root_sha256": verifier.digest(verifier.canonical_bytes([1] * 256)),
                "p50_ns": 1, "p99_ns": 1, "hard_max_ns": 1,
                "filesystem": {}, "diagnostic_only": True, "returncode": 0,
                "error": None, "command": {},
            }

        def fake_metric(*args):
            label = args[3]
            calls.append(label)
            if label.startswith("hot-"):
                metrics = {"p99_ns": 1_000_001 if label == "hot-1" else 1,
                           "no_goal_p99_ns": 1, "hard_max_ns": 1, "samples": 4096}
            elif label.startswith("single-"):
                metrics = {"p99_ns": 1, "hard_max_ns": 1, "samples": 1024, "segments": 2}
            elif label.startswith("three-"):
                metrics = {"precommit_p99_ns": 1, "precommit_hard_max_ns": 1,
                           "settlement_p99_ns": 1, "settlement_hard_max_ns": 1,
                           "episode_p99_ns": 1, "episode_hard_max_ns": 1, "samples": 256}
            else:
                metrics = {"elapsed_ticks": 0, "ticks_per_second": 100,
                           "percent_of_one_core": 0.0}
            return {"label": label, "test": args[2], "returncode": 101 if label == "hot-1" else 0,
                    "test_assertion_pass": label != "hot-1", "metric_present": True,
                    "metrics": metrics, "output_sha256": "a" * 64,
                    "command": {"observed_affinity": [4]}}

        def fake_rss(_, __, capture, *args):
            label = "capture_on" if capture else "capture_off"
            calls.append(f"rss-{label}")
            return {"label": label, "rss_bytes": 100, "sample_count": 20,
                    "error": None, "command": {"observed_affinity": [4]}}

        with mock.patch.object(executor, "floor_probe", side_effect=fake_floor), \
             mock.patch.object(executor, "test_metric", side_effect=fake_metric), \
             mock.patch.object(executor, "measure_rss_mode", side_effect=fake_rss), \
             mock.patch.object(executor, "run_parity", return_value={
                 "parity_root_sha256": "f" * 64, "byte_identical": True, "row_count": 16,
             }):
            resource, _ = executor.evaluate_measurement(
                Path("/bin/true"), Path("/tmp"), {"response-actor": Path("/bin/true"),
                "transition-serving": Path("/bin/true")}, {}, Path("/tmp"), Path("/tmp"),
                object(), Path("/bin/true"), Path("/bin/true"), {}, "e" * 64,
                "o" * 64, {"baseline": Path("/bin/true"), "candidate": Path("/bin/true")},
            )
        self.assertIn("three-sync-3", calls)
        self.assertIn("floor-after-3", calls)
        self.assertIn("hot-1:matched_p99", resource["resource_failures"])

    def test_near_miss_is_failure_without_threshold_adaptation(self) -> None:
        self.assertGreater(5_010_709, 5_000_000)
        source = Path(__file__).with_name("s1c3b_remote_transaction_v1.py").read_text()
        self.assertNotIn("5_010_709", source)
        self.assertNotIn("fourth", source.lower())

    def test_foreign_build_is_diagnostic_but_owned_build_is_blocker(self) -> None:
        foreign = process_snapshot([user_process(200, "cargo", 1, "cargo")])
        self.assertEqual(executor.transaction_owned_builds(foreign, 100), [])
        owned = process_snapshot([user_process(200, "cargo", 100, "cargo")])
        self.assertEqual(len(executor.transaction_owned_builds(owned, 100)), 1)

    def test_executor_has_no_quiet_window_or_cpu_selection(self) -> None:
        source = Path(__file__).with_name("s1c3b_remote_transaction_v1.py").read_text()
        for forbidden in ("wait_for_quiescence(", "MEASUREMENT_CPU_POOL", "selected_cpu"):
            self.assertNotIn(forbidden, source)
        self.assertIn("MEASUREMENT_CPU = 4", source)


class MinimalVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        (self.directory / "evidence").mkdir()
        self.executables = {
            "candidate-binary": executable("/tmp/candidate"),
            "test-response-actor": executable("/tmp/response"),
            "test-transition-serving": executable("/tmp/transition"),
            "parity-baseline": executable("/tmp/parity-base"),
            "parity-candidate": executable("/tmp/parity-candidate"),
            "python-runtime": executable("/usr/bin/python3"),
            "affinity-wrapper": executable("/tmp/wrapper"),
            "filesystem-floor-probe": executable("/tmp/floor"),
        }
        self.executable_root = verifier.digest(verifier.canonical_bytes(self.executables))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def command(self, label: str, path: str) -> dict:
        sha = {row["path"]: row["sha256"] for row in self.executables.values()}[path]
        return {"label": label, "executable": path, "executable_sha256": sha,
                "requested_affinity": [4], "observed_affinity": [4],
                "wrapper_reported_executable_sha256": sha, "returncode": 0, "error": None}

    def test_wrong_affinity_is_rejected(self) -> None:
        row = self.command("hot-1", "/tmp/response")
        row["observed_affinity"] = [5]
        with self.assertRaisesRegex(verifier.InvalidReceipt, "observed_affinity"):
            verifier.verify_command(row, "hot-1", self.executables)

    def test_wrapper_executable_drift_is_rejected(self) -> None:
        row = self.command("hot-1", "/tmp/response")
        row["wrapper_reported_executable_sha256"] = "f" * 64
        with self.assertRaisesRegex(verifier.InvalidReceipt, "wrapper_sha"):
            verifier.verify_command(row, "hot-1", self.executables)

    def test_floor_quantiles_are_recomputed_but_never_gate(self) -> None:
        samples = [1] * 255 + [10**12]
        label = "floor-before-1"
        (self.directory / "evidence" / f"{label}.log").write_text(
            "S1C3B_AFFINITY pid=1 cpus=4 executable_sha256=" + "a" * 64 + "\n"
            + json.dumps({"samples_ns": samples}) + "\n"
        )
        command = self.command(label, "/usr/bin/python3")
        row = {"label": label, "round": 1, "position": "before", "records": 256,
               "samples_ns": samples, "samples_root_sha256": verifier.digest(verifier.canonical_bytes(samples)),
               "p50_ns": 1, "p99_ns": 1, "hard_max_ns": 10**12, "filesystem": {"findmnt_returncode": 0},
               "diagnostic_only": True, "returncode": 0, "error": None, "command": command}
        verifier.verify_floor(self.directory, row, 0, {label: command})

    def test_metric_log_tamper_is_rejected(self) -> None:
        label = "hot-1"
        log = "S1C3B_AFFINITY pid=1 cpus=4 executable_sha256=" + "a" * 64 + "\n"
        log += "S1C_HOT_LATENCY matched_p99_ns=1 no_goal_p99_ns=1 hard_max_ns=1 samples=4096\n"
        log += "test result: ok. 1 passed; 0 failed;\n"
        (self.directory / "evidence" / f"{label}.log").write_text(log)
        command = self.command(label, "/tmp/response")
        row = {"label": label, "test": executor.HOT_TEST, "returncode": 0,
               "test_assertion_pass": True, "metric_present": True,
               "metrics": {"p99_ns": 2, "no_goal_p99_ns": 1, "hard_max_ns": 1, "samples": 4096},
               "output_sha256": verifier.digest(log.encode()), "command": command}
        with self.assertRaisesRegex(verifier.InvalidReceipt, "values_mismatch"):
            verifier.verify_metric_row(self.directory, row, label, executor.HOT_TEST, {label: command})

    def test_launcher_verifies_before_rollback_arm(self) -> None:
        source = Path(__file__).with_name("run_s1c3b_transaction_v1.sh").read_text()
        self.assertLess(source.index("--pre-deployment"), source.index("rollback_armed=true"))
        self.assertLess(source.index("rollback_armed=true"), source.index("s1c3b_remote_transaction_v1.py' execute"))
        self.assertIn("trap emergency_rollback EXIT INT TERM HUP", source)
        self.assertIn("state == FINAL_VERIFICATION_PENDING", source)
        self.assertIn("state == ROLLBACK_PENDING", source)
        self.assertLess(source.index("rollback_armed=true"), source.rindex("local-verification.json"))
        self.assertLess(source.index("remote-verification.json"), source.rindex("rollback_armed=false"))
        self.assertIn("implementation_file_not_committed", source)

    def test_launcher_disarms_rollback_only_in_trap_or_after_complete(self) -> None:
        source = Path(__file__).with_name("run_s1c3b_transaction_v1.sh").read_text()
        trap_body = source.split("emergency_rollback() {", 1)[1].split(
            "trap emergency_rollback EXIT INT TERM HUP", 1
        )[0]
        unexpected_state = source.split(
            "if [[ $state != FINALIZE_PENDING && $state != ROLLBACK_PENDING ]]", 1
        )[1].split("fi", 1)[0]

        self.assertEqual(source.count("rollback_armed=false"), 2)
        self.assertEqual(trap_body.count("rollback_armed=false"), 1)
        self.assertNotIn("rollback_armed=false", unexpected_state)
        self.assertLess(source.rindex("== COMPLETE ]]"), source.rindex("rollback_armed=false"))

    def test_rollback_rejects_terminal_state(self) -> None:
        source = Path(__file__).with_name("s1c3b_remote_transaction_v1.py").read_text()
        rollback = source.split("def rollback(", 1)[1].split("def execute(", 1)[0]
        self.assertIn('"FINAL_VERIFICATION_PENDING"', rollback)
        self.assertIn("rollback_state_invalid", rollback)

    def test_all_mutation_commands_share_one_remote_lock(self) -> None:
        source = Path(__file__).with_name("s1c3b_remote_transaction_v1.py").read_text()
        locked = source.split("def locked_command(", 1)[1].split("def main(", 1)[0]
        main = source.split("def main(", 1)[1]

        self.assertIn('lock_path = root / ".mutation.lock"', locked)
        self.assertIn("fcntl.flock(descriptor, fcntl.LOCK_EX)", locked)
        for command in ("execute", "finalize", "seal"):
            self.assertIn(f'args.command == "{command}"', locked)
        self.assertIn("return rollback_command(args)", locked)
        self.assertIn("return locked_command(args)", main)

    def test_remote_mutation_lock_serializes_independent_processes(self) -> None:
        lock_holder = subprocess.Popen(
            [
                sys.executable,
                "-c",
                (
                    "import fcntl,os,sys,time;"
                    "p=os.path.join(sys.argv[1],'.mutation.lock');"
                    "f=os.open(p,os.O_RDWR|os.O_CREAT,0o600);"
                    "fcntl.flock(f,fcntl.LOCK_EX);"
                    "print('locked',flush=True);"
                    "time.sleep(0.2);"
                    "os.close(f)"
                ),
                str(self.directory),
            ],
            stdout=subprocess.PIPE,
            text=True,
        )
        self.assertIsNotNone(lock_holder.stdout)
        self.assertEqual(lock_holder.stdout.readline().strip(), "locked")
        with mock.patch.object(executor, "seal", return_value=0) as seal:
            result = executor.locked_command(
                SimpleNamespace(
                    command="seal",
                    transaction_directory=str(self.directory),
                )
            )
        self.assertEqual(result, 0)
        self.assertEqual(lock_holder.wait(timeout=1), 0)
        lock_holder.stdout.close()
        seal.assert_called_once()

    def test_launcher_enforces_one_attempt(self) -> None:
        launcher = Path(__file__).with_name("run_s1c3b_transaction_v1.sh")
        source = launcher.read_text()
        self.assertIn("s1c3b_attempt_already_exists", source)
        self.assertIn("set -o pipefail; sudo -n find /var/lib/nando-wave/deployments", source)
        self.assertNotIn("retry", source.lower())
        self.assertNotEqual(launcher.stat().st_mode & 0o111, 0)

    def test_launcher_reports_success_only_for_deployment_pass(self) -> None:
        source = Path(__file__).with_name("run_s1c3b_transaction_v1.sh").read_text()
        self.assertIn('final_verdict=$(jq -er .verdict "$local_dir/local-verification.json")', source)
        self.assertIn("$final_verdict != S1C3B_DEPLOYMENT_PASS", source)


class SyntheticEndToEndVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.fixture = SyntheticTransaction(self.directory)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_valid_preparation_pass(self) -> None:
        result = verifier.verify_preparation(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_PREPARATION_PASS")
        self.assertTrue(result["authority"])

    def test_valid_deployment_pass(self) -> None:
        result = verifier.verify_final(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_DEPLOYMENT_PASS")

    def test_valid_connector_veto_requires_baseline_rollback(self) -> None:
        self.fixture.receipt["verdict"] = "S1C3B_VETO"
        self.fixture.receipt["connector_after"]["main_pid"] += 1
        self.fixture.receipt["installed_binary_sha256"] = verifier.BASELINE_BINARY_SHA256
        self.fixture.receipt["installed_config_sha256"] = verifier.BASELINE_CONFIG_SHA256
        self.fixture.receipt["capture_environment"] = {}
        self.fixture.receipt["capture_available"] = False
        self.fixture.receipt["veto_reasons"] = ["connector_main_pid"]
        self.fixture.receipt = rooted(
            self.fixture.receipt, "receipt_root_sha256"
        )
        self.fixture.write_json("deployment-receipt.json", self.fixture.receipt)

        result = verifier.verify_final(self.directory)

        self.assertEqual(result["verdict"], "S1C3B_VETO")

    def test_connector_veto_cannot_leave_candidate_installed(self) -> None:
        self.fixture.receipt["verdict"] = "S1C3B_VETO"
        self.fixture.receipt["connector_after"]["main_pid"] += 1
        self.fixture.receipt["veto_reasons"] = ["connector_main_pid"]
        self.fixture.receipt = rooted(
            self.fixture.receipt, "receipt_root_sha256"
        )
        self.fixture.write_json("deployment-receipt.json", self.fixture.receipt)

        with self.assertRaisesRegex(verifier.InvalidReceipt, "rollback_binary"):
            verifier.verify_final(self.directory)

    def test_final_seal_requires_verified_receipt(self) -> None:
        verification = verifier.verify_final(self.directory)
        verification_path = self.directory / "final-verification-input.json"
        verification_path.write_text(json.dumps(verification, sort_keys=True))
        self.fixture.write_json("transaction-state.json", {
            "schema": verifier.STATE_SCHEMA,
            "state": "FINAL_VERIFICATION_PENDING",
            "transaction_id": self.fixture.transaction_id,
            "verdict": "S1C3B_DEPLOYMENT_PASS",
        })
        code = executor.seal(SimpleNamespace(
            transaction_directory=str(self.directory),
            final_verification=str(verification_path),
        ))
        self.assertEqual(code, 0)
        state = json.loads((self.directory / "transaction-state.json").read_text())
        self.assertEqual(state["state"], "COMPLETE")
        self.assertEqual(
            state["final_verification_root_sha256"],
            verification["final_verification_root_sha256"],
        )

    def test_valid_resource_veto(self) -> None:
        resource = self.fixture.resource
        raw, metric = self.fixture.metric_log("single-sync-1", {"p99_ns": 5_010_709})
        (self.fixture.evidence / "single-sync-1.log").write_bytes(raw)
        resource["metrics"]["single_ledger_sync"][0].update({
            "returncode": 0,
            "test_assertion_pass": True,
            "metrics": metric,
            "output_sha256": verifier.digest(raw),
        })
        resource["resource_failures"] = ["single-sync-1:p99"]
        resource["all_pass_before_monitor"] = False
        resource["all_pass"] = False
        self.fixture.write_veto()
        result = verifier.verify_resource_veto(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_RESOURCE_VETO")
        self.assertFalse(result["authority"])

    def test_assertion_failure_with_metric_is_valid_instrument_veto(self) -> None:
        raw, metric = self.fixture.metric_log("hot-1")
        raw = raw.replace(
            b"test result: ok. 1 passed; 0 failed;",
            b"test result: FAILED. 0 passed; 1 failed;",
        )
        (self.fixture.evidence / "hot-1.log").write_bytes(raw)
        row = self.fixture.resource["metrics"]["hot_latency"][0]
        row.update({
            "returncode": 101,
            "test_assertion_pass": False,
            "metrics": metric,
            "output_sha256": verifier.digest(raw),
        })
        command = next(
            command for command in self.fixture.monitor["commands"]
            if command["label"] == "hot-1"
        )
        command["returncode"] = 101
        row["command"]["returncode"] = 101
        self.fixture.rehash_monitor()
        self.fixture.resource["monitor_root_sha256"] = self.fixture.monitor[
            "monitor_root_sha256"
        ]
        self.fixture.resource["instrument_failures"] = [
            "hot-1:test_assertion_failed"
        ]
        self.fixture.resource["all_pass_before_monitor"] = False
        self.fixture.resource["all_pass"] = False
        self.fixture.write_veto()
        result = verifier.verify_resource_veto(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_RESOURCE_VETO")

    def test_monitor_gap_is_valid_instrument_veto(self) -> None:
        samples = self.fixture.monitor["samples"]
        delta = 2_100_000_000 - (
            samples[1]["monotonic_ns"] - samples[0]["monotonic_ns"]
        )
        for row in samples[1:]:
            row["monotonic_ns"] += delta
        for boundary in self.fixture.monitor["boundaries"]:
            index = boundary["sample_index"]
            boundary["monotonic_ns"] = samples[index]["monotonic_ns"]
        self.fixture.rehash_monitor()
        self.fixture.resource["instrument_failures"] = ["monitor_instrument_failure"]
        self.fixture.resource["all_pass"] = False
        self.fixture.write_veto()
        result = verifier.verify_resource_veto(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_RESOURCE_VETO")

    def install_monitor_process(self, process: dict) -> None:
        snapshot = process_snapshot([process])
        for row in self.fixture.monitor["samples"]:
            row["process_snapshot"] = copy.deepcopy(snapshot)
            row["process_snapshot_root_sha256"] = snapshot["process_snapshot_root_sha256"]
            row["foreign_build_processes"] = copy.deepcopy(snapshot["forbidden_process_matches"])
            row["unresolved_processes"] = copy.deepcopy(snapshot["unresolved_rows"])
            row["transaction_owned_build_processes"] = executor.transaction_owned_builds(
                snapshot, self.fixture.monitor["executor_pid"]
            )
        self.fixture.rehash_monitor()

    def test_owned_compiler_is_valid_instrument_veto(self) -> None:
        self.install_monitor_process(user_process(5000, "cargo", 4242, "cargo"))
        self.fixture.resource["instrument_failures"] = ["monitor_instrument_failure"]
        self.fixture.resource["all_pass"] = False
        self.fixture.write_veto()
        result = verifier.verify_resource_veto(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_RESOURCE_VETO")

    def test_foreign_compiler_is_diagnostic_only(self) -> None:
        self.install_monitor_process(user_process(5000, "cargo", 1, "cargo"))
        self.fixture.resource["monitor_root_sha256"] = self.fixture.monitor[
            "monitor_root_sha256"
        ]
        self.fixture.resource = rooted(self.fixture.resource, "resource_root_sha256")
        self.fixture.preparation["monitor_root_sha256"] = self.fixture.monitor[
            "monitor_root_sha256"
        ]
        self.fixture.preparation["resource_root_sha256"] = self.fixture.resource[
            "resource_root_sha256"
        ]
        self.fixture.preparation = rooted(
            self.fixture.preparation, "preparation_root_sha256"
        )
        self.fixture.write_common()
        self.fixture.write_json("preparation.json", self.fixture.preparation)
        result = verifier.verify_preparation(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_PREPARATION_PASS")

    def test_executable_before_after_drift_is_valid_instrument_veto(self) -> None:
        self.fixture.resource["instrument_failures"] = [
            "executable_drift_after_measurement"
        ]
        self.fixture.resource["all_pass"] = False
        self.fixture.write_veto()
        path = self.directory / "executable-identities.json"
        receipt = json.loads(path.read_text())
        receipt["after"]["candidate-binary"]["sha256"] = "f" * 64
        receipt = rooted(receipt, "executable_identities_root_sha256")
        self.fixture.write_json("executable-identities.json", receipt)
        result = verifier.verify_resource_veto(self.directory)
        self.assertEqual(result["verdict"], "S1C3B_RESOURCE_VETO")

    def test_receipt_root_tamper_is_rejected(self) -> None:
        self.fixture.receipt["survival_seconds"] = 14
        self.fixture.write_json("deployment-receipt.json", self.fixture.receipt)
        with self.assertRaisesRegex(verifier.InvalidReceipt, "receipt_root"):
            verifier.verify_final(self.directory)

    def test_ownership_lock_drift_is_rejected(self) -> None:
        row = self.fixture.ownership["rows"]["candidate"]
        row["cargo_lock"]["sha256_after_build"] = "f" * 64
        row = rooted(row, "ownership_row_root_sha256")
        self.fixture.ownership["rows"]["candidate"] = row
        self.fixture.ownership["rows_root_sha256"] = verifier.digest(
            verifier.canonical_bytes(self.fixture.ownership["rows"])
        )
        self.fixture.ownership = rooted(
            self.fixture.ownership, "ownership_root_sha256"
        )
        self.fixture.write_json(
            "oracle-ownership-receipt.json", self.fixture.ownership, 0o400
        )
        with self.assertRaisesRegex(verifier.InvalidReceipt, "ownership_candidate_lock_after"):
            verifier.verify_preparation(self.directory)


if __name__ == "__main__":
    unittest.main()
